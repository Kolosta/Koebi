//! 2D textures and render targets.

use std::fmt;
use std::ops::{BitOr, BitOrAssign};
use std::sync::Arc;

use ash::vk;
use koebi_core::Extent;

use crate::command::RenderTarget;
use crate::debug::set_object_name;
use crate::device::DeviceShared;
use crate::error::VkResultExt as _;
use crate::format::TextureFormat;
use crate::garbage::Garbage;
use crate::memory::{self, MemoryLocation};
use crate::{Error, Result};

/// How a texture will be used. Combine flags with `|`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TextureUsage(u32);

impl TextureUsage {
    /// Rendered into by a render pass.
    pub const COLOR_ATTACHMENT: Self = Self(1 << 0);
    /// Sampled by shaders.
    pub const SAMPLED: Self = Self(1 << 1);
    /// Read/written as a storage image.
    pub const STORAGE: Self = Self(1 << 2);
    /// Source of copy commands (readback).
    pub const TRANSFER_SRC: Self = Self(1 << 3);
    /// Destination of copy commands (uploads).
    pub const TRANSFER_DST: Self = Self(1 << 4);

    /// `true` if every flag of `other` is set.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub(crate) fn to_vk(self) -> vk::ImageUsageFlags {
        use vk::ImageUsageFlags as F;
        [
            (Self::COLOR_ATTACHMENT, F::COLOR_ATTACHMENT),
            (Self::SAMPLED, F::SAMPLED),
            (Self::STORAGE, F::STORAGE),
            (Self::TRANSFER_SRC, F::TRANSFER_SRC),
            (Self::TRANSFER_DST, F::TRANSFER_DST),
        ]
        .into_iter()
        .filter(|(flag, _)| self.contains(*flag))
        .fold(F::empty(), |acc, (_, vk)| acc | vk)
    }
}

impl BitOr for TextureUsage {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for TextureUsage {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl fmt::Debug for TextureUsage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TextureUsage({:#07b})", self.0)
    }
}

/// Parameters of [`crate::Device::create_texture`].
#[derive(Clone, Copy, Debug)]
pub struct TextureDesc<'a> {
    /// Debug name.
    pub label: &'a str,
    /// Size in physical pixels (non-zero).
    pub extent: Extent,
    /// Pixel format.
    pub format: TextureFormat,
    /// Intended usage.
    pub usage: TextureUsage,
}

/// A 2D, single-mip, device-local texture with a default view.
///
/// Dropping is always safe: destruction is deferred until pending GPU work completes.
pub struct Texture {
    device: Arc<DeviceShared>,
    raw: vk::Image,
    view: vk::ImageView,
    memory: vk::DeviceMemory,
    extent: Extent,
    format: TextureFormat,
    usage: TextureUsage,
}

pub(crate) const COLOR_RANGE: vk::ImageSubresourceRange = vk::ImageSubresourceRange {
    aspect_mask: vk::ImageAspectFlags::COLOR,
    base_mip_level: 0,
    level_count: 1,
    base_array_layer: 0,
    layer_count: 1,
};

impl Texture {
    pub(crate) fn new(device: &Arc<DeviceShared>, desc: &TextureDesc<'_>) -> Result<Self> {
        if desc.extent.is_empty() {
            return Err(Error::InvalidArgument(format!(
                "texture {:?} is empty",
                desc.label
            )));
        }
        let info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(desc.format.to_vk())
            .extent(vk::Extent3D {
                width: desc.extent.width,
                height: desc.extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(desc.usage.to_vk())
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        // SAFETY: valid device and create info.
        let raw = unsafe { device.raw.create_image(&info, None) }.ctx("vkCreateImage")?;
        // SAFETY: `raw` is a live image of this device.
        let requirements = unsafe { device.raw.get_image_memory_requirements(raw) };

        let allocation = memory::allocate(device, requirements, MemoryLocation::GpuOnly)
            // SAFETY: the image is unused and destroyed exactly once on this error path.
            .inspect_err(|_| unsafe { device.raw.destroy_image(raw, None) })?;
        let destroy_both = || {
            // SAFETY: both objects are unused and destroyed exactly once on error paths.
            unsafe {
                device.raw.destroy_image(raw, None);
                device.raw.free_memory(allocation.memory, None);
            }
        };
        // SAFETY: the memory satisfies the image's requirements and is bound once.
        if let Err(result) = unsafe { device.raw.bind_image_memory(raw, allocation.memory, 0) } {
            destroy_both();
            return Err(Error::from_vk("vkBindImageMemory", result));
        }
        let view_info = vk::ImageViewCreateInfo::default()
            .image(raw)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(desc.format.to_vk())
            .subresource_range(COLOR_RANGE);
        // SAFETY: `raw` is a live, bound image of this device.
        let view = match unsafe { device.raw.create_image_view(&view_info, None) } {
            Ok(view) => view,
            Err(result) => {
                destroy_both();
                return Err(Error::from_vk("vkCreateImageView", result));
            }
        };
        set_object_name(device, raw, desc.label);
        set_object_name(device, view, desc.label);

        Ok(Self {
            device: Arc::clone(device),
            raw,
            view,
            memory: allocation.memory,
            extent: desc.extent,
            format: desc.format,
            usage: desc.usage,
        })
    }

    /// Size in physical pixels.
    pub fn extent(&self) -> Extent {
        self.extent
    }

    /// Pixel format.
    pub fn format(&self) -> TextureFormat {
        self.format
    }

    /// Usage flags.
    pub fn usage(&self) -> TextureUsage {
        self.usage
    }

    /// The raw image handle.
    pub fn handle(&self) -> vk::Image {
        self.raw
    }

    /// The default view's raw handle.
    pub fn view_handle(&self) -> vk::ImageView {
        self.view
    }

    /// Describes this texture as a render pass target.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`] without [`TextureUsage::COLOR_ATTACHMENT`].
    pub fn render_target(&self) -> Result<RenderTarget> {
        if !self.usage.contains(TextureUsage::COLOR_ATTACHMENT) {
            return Err(Error::InvalidArgument(
                "texture was not created with COLOR_ATTACHMENT usage".into(),
            ));
        }
        Ok(RenderTarget {
            view: self.view,
            extent: self.extent,
            format: self.format,
        })
    }
}

impl fmt::Debug for Texture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Texture")
            .field("handle", &self.raw)
            .field("extent", &self.extent)
            .field("format", &self.format)
            .field("usage", &self.usage)
            .finish_non_exhaustive()
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        self.device.defer_destroy(Garbage::Texture {
            image: self.raw,
            view: self.view,
            memory: self.memory,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_usage_maps_to_vulkan() {
        let usage = TextureUsage::COLOR_ATTACHMENT | TextureUsage::TRANSFER_SRC;
        assert_eq!(
            usage.to_vk(),
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC
        );
        assert!(!usage.contains(TextureUsage::SAMPLED));
    }
}
