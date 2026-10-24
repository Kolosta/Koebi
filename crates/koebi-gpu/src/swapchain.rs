//! Swapchain creation and the policies that pick its parameters.

use ash::vk;
use koebi_core::Extent;

use crate::debug::set_object_name;
use crate::device::DeviceShared;
use crate::error::VkResultExt as _;
use crate::format::TextureFormat;
use crate::surface::Surface;
use crate::texture::COLOR_RANGE;
use crate::{Error, Result};

/// Presentation pacing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PresentMode {
    /// Vertical sync, no tearing; always available. Best for battery and UI.
    #[default]
    Fifo,
    /// Low latency without tearing (falls back to `Fifo`).
    Mailbox,
    /// No vsync, may tear (benchmarks; falls back to `Mailbox`, then `Fifo`).
    Immediate,
}

pub(crate) struct Swapchain {
    pub(crate) raw: vk::SwapchainKHR,
    pub(crate) images: Vec<vk::Image>,
    pub(crate) views: Vec<vk::ImageView>,
    pub(crate) format: TextureFormat,
    pub(crate) extent: Extent,
}

impl Swapchain {
    /// Creates a swapchain, or returns `None` while the surface has a zero size (minimized).
    pub(crate) fn new(
        device: &DeviceShared,
        surface: &Surface,
        desired: Extent,
        mode: PresentMode,
        old: vk::SwapchainKHR,
    ) -> Result<Option<Self>> {
        let loader = device
            .swapchain
            .as_ref()
            .ok_or_else(|| Error::MissingExtension("VK_KHR_swapchain".into()))?;
        let surface_loader = surface.loader();
        let physical = device.adapter.raw;
        let raw_surface = surface.handle();
        // SAFETY: the surface and the physical device belong to the same instance.
        let caps = unsafe {
            surface_loader.get_physical_device_surface_capabilities(physical, raw_surface)
        }
        .ctx("vkGetPhysicalDeviceSurfaceCapabilitiesKHR")?;
        // SAFETY: see above.
        let formats =
            unsafe { surface_loader.get_physical_device_surface_formats(physical, raw_surface) }
                .ctx("vkGetPhysicalDeviceSurfaceFormatsKHR")?;
        // SAFETY: see above.
        let modes = unsafe {
            surface_loader.get_physical_device_surface_present_modes(physical, raw_surface)
        }
        .ctx("vkGetPhysicalDeviceSurfacePresentModesKHR")?;

        let extent = choose_extent(&caps, desired);
        if extent.is_empty() {
            return Ok(None);
        }
        let (surface_format, format) = choose_format(&formats)
            .ok_or_else(|| Error::InvalidArgument("surface offers no 8-bit RGBA format".into()))?;
        let present_mode = choose_present_mode(&modes, mode);
        let mut usage = vk::ImageUsageFlags::COLOR_ATTACHMENT;
        if caps
            .supported_usage_flags
            .contains(vk::ImageUsageFlags::TRANSFER_SRC)
        {
            usage |= vk::ImageUsageFlags::TRANSFER_SRC;
        }
        let transform = if caps
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            caps.current_transform
        };

        let info = vk::SwapchainCreateInfoKHR::default()
            .surface(raw_surface)
            .min_image_count(choose_image_count(&caps))
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(vk::Extent2D {
                width: extent.width,
                height: extent.height,
            })
            .image_array_layers(1)
            .image_usage(usage)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(transform)
            .composite_alpha(choose_composite_alpha(caps.supported_composite_alpha))
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(old);
        // SAFETY: parameters come from the surface's reported capabilities.
        let raw = unsafe { loader.create_swapchain(&info, None) }.ctx("vkCreateSwapchainKHR")?;
        let mut swapchain = Self {
            raw,
            images: Vec::new(),
            views: Vec::new(),
            format,
            extent,
        };

        // SAFETY: `raw` is a live swapchain of this device.
        match unsafe { loader.get_swapchain_images(raw) } {
            Ok(images) => swapchain.images = images,
            Err(result) => {
                // SAFETY: partially built swapchain, destroyed exactly once.
                unsafe { swapchain.destroy(device) };
                return Err(Error::from_vk("vkGetSwapchainImagesKHR", result));
            }
        }
        for (index, &image) in swapchain.images.iter().enumerate() {
            let view_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(surface_format.format)
                .subresource_range(COLOR_RANGE);
            // SAFETY: `image` belongs to the swapchain of this device.
            match unsafe { device.raw.create_image_view(&view_info, None) } {
                Ok(view) => {
                    set_object_name(device, view, &format!("swapchain view {index}"));
                    swapchain.views.push(view);
                }
                Err(result) => {
                    // SAFETY: partially built swapchain, destroyed exactly once.
                    unsafe { swapchain.destroy(device) };
                    return Err(Error::from_vk("vkCreateImageView", result));
                }
            }
        }
        koebi_core::debug!(
            "swapchain {}x{} {format:?} {present_mode:?}, {} images",
            extent.width,
            extent.height,
            swapchain.images.len()
        );
        Ok(Some(swapchain))
    }

    /// Destroys the views and the swapchain.
    ///
    /// # Safety
    ///
    /// No pending GPU work may use the swapchain images (wait for idle first).
    pub(crate) unsafe fn destroy(&mut self, device: &DeviceShared) {
        // SAFETY: guaranteed by the caller; each object is destroyed once (vectors drained).
        unsafe {
            for view in self.views.drain(..) {
                device.raw.destroy_image_view(view, None);
            }
            if let Some(loader) = device.swapchain.as_ref() {
                loader.destroy_swapchain(self.raw, None);
            }
        }
        self.images.clear();
        self.raw = vk::SwapchainKHR::null();
    }
}

/// Prefers 8-bit UNORM formats (gamma-space UI blending, see ADR 0004), then sRGB ones.
pub(crate) fn choose_format(
    formats: &[vk::SurfaceFormatKHR],
) -> Option<(vk::SurfaceFormatKHR, TextureFormat)> {
    const PREFERENCE: [vk::Format; 4] = [
        vk::Format::B8G8R8A8_UNORM,
        vk::Format::R8G8B8A8_UNORM,
        vk::Format::B8G8R8A8_SRGB,
        vk::Format::R8G8B8A8_SRGB,
    ];
    PREFERENCE.iter().find_map(|wanted| {
        formats
            .iter()
            .find(|f| f.format == *wanted && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .and_then(|f| TextureFormat::from_vk(f.format).map(|format| (*f, format)))
    })
}

pub(crate) fn choose_present_mode(
    available: &[vk::PresentModeKHR],
    requested: PresentMode,
) -> vk::PresentModeKHR {
    let chain: &[vk::PresentModeKHR] = match requested {
        PresentMode::Fifo => &[],
        PresentMode::Mailbox => &[vk::PresentModeKHR::MAILBOX],
        PresentMode::Immediate => &[vk::PresentModeKHR::IMMEDIATE, vk::PresentModeKHR::MAILBOX],
    };
    chain
        .iter()
        .copied()
        .find(|mode| available.contains(mode))
        .unwrap_or(vk::PresentModeKHR::FIFO)
}

pub(crate) fn choose_extent(caps: &vk::SurfaceCapabilitiesKHR, desired: Extent) -> Extent {
    if caps.current_extent.width != u32::MAX {
        return Extent::new(caps.current_extent.width, caps.current_extent.height);
    }
    Extent::new(
        desired
            .width
            .clamp(caps.min_image_extent.width, caps.max_image_extent.width),
        desired
            .height
            .clamp(caps.min_image_extent.height, caps.max_image_extent.height),
    )
}

/// One image more than the minimum lets the CPU record while the GPU presents.
pub(crate) fn choose_image_count(caps: &vk::SurfaceCapabilitiesKHR) -> u32 {
    let wanted = caps.min_image_count + 1;
    if caps.max_image_count == 0 {
        wanted
    } else {
        wanted.min(caps.max_image_count)
    }
}

fn choose_composite_alpha(supported: vk::CompositeAlphaFlagsKHR) -> vk::CompositeAlphaFlagsKHR {
    [
        vk::CompositeAlphaFlagsKHR::OPAQUE,
        vk::CompositeAlphaFlagsKHR::INHERIT,
        vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED,
        vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED,
    ]
    .into_iter()
    .find(|mode| supported.contains(*mode))
    .unwrap_or(vk::CompositeAlphaFlagsKHR::OPAQUE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surface_format(format: vk::Format) -> vk::SurfaceFormatKHR {
        vk::SurfaceFormatKHR {
            format,
            color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
        }
    }

    #[test]
    fn unorm_formats_win_over_srgb() {
        let formats = [
            surface_format(vk::Format::B8G8R8A8_SRGB),
            surface_format(vk::Format::R8G8B8A8_UNORM),
        ];
        assert_eq!(
            choose_format(&formats).map(|(_, f)| f),
            Some(TextureFormat::Rgba8Unorm)
        );
        let srgb_only = [surface_format(vk::Format::B8G8R8A8_SRGB)];
        assert_eq!(
            choose_format(&srgb_only).map(|(_, f)| f),
            Some(TextureFormat::Bgra8Srgb)
        );
        assert!(choose_format(&[surface_format(vk::Format::R5G6B5_UNORM_PACK16)]).is_none());
    }

    #[test]
    fn present_mode_falls_back_to_fifo() {
        let only_fifo = [vk::PresentModeKHR::FIFO];
        assert_eq!(
            choose_present_mode(&only_fifo, PresentMode::Mailbox),
            vk::PresentModeKHR::FIFO
        );
        let all = [vk::PresentModeKHR::FIFO, vk::PresentModeKHR::MAILBOX];
        assert_eq!(
            choose_present_mode(&all, PresentMode::Immediate),
            vk::PresentModeKHR::MAILBOX
        );
        assert_eq!(
            choose_present_mode(&all, PresentMode::Fifo),
            vk::PresentModeKHR::FIFO
        );
    }

    #[test]
    fn extent_uses_current_extent_or_clamps_desired() {
        let fixed = vk::SurfaceCapabilitiesKHR {
            current_extent: vk::Extent2D {
                width: 800,
                height: 600,
            },
            ..Default::default()
        };
        assert_eq!(
            choose_extent(&fixed, Extent::new(1, 1)),
            Extent::new(800, 600)
        );
        let free = vk::SurfaceCapabilitiesKHR {
            current_extent: vk::Extent2D {
                width: u32::MAX,
                height: u32::MAX,
            },
            min_image_extent: vk::Extent2D {
                width: 16,
                height: 16,
            },
            max_image_extent: vk::Extent2D {
                width: 4096,
                height: 4096,
            },
            ..Default::default()
        };
        assert_eq!(
            choose_extent(&free, Extent::new(8, 10_000)),
            Extent::new(16, 4096)
        );
    }

    #[test]
    fn image_count_is_min_plus_one_capped_by_max() {
        let caps = vk::SurfaceCapabilitiesKHR {
            min_image_count: 2,
            max_image_count: 0,
            ..Default::default()
        };
        assert_eq!(choose_image_count(&caps), 3);
        let capped = vk::SurfaceCapabilitiesKHR {
            min_image_count: 2,
            max_image_count: 2,
            ..Default::default()
        };
        assert_eq!(choose_image_count(&capped), 2);
    }
}
