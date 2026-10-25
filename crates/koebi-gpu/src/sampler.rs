//! Samplers: how shaders read textures.

use std::fmt;
use std::sync::Arc;

use ash::vk;

use crate::Result;
use crate::debug::set_object_name;
use crate::device::DeviceShared;
use crate::error::VkResultExt as _;
use crate::garbage::Garbage;

/// How texels are filtered when a texture is magnified or minified.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FilterMode {
    /// The nearest texel: exact pixels, for textures drawn one texel per pixel (glyph atlases).
    #[default]
    Nearest,
    /// Bilinear interpolation of the four nearest texels.
    Linear,
}

impl FilterMode {
    fn to_vk(self) -> vk::Filter {
        match self {
            Self::Nearest => vk::Filter::NEAREST,
            Self::Linear => vk::Filter::LINEAR,
        }
    }
}

/// What reads outside the texture return.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AddressMode {
    /// The nearest edge texel.
    #[default]
    ClampToEdge,
    /// The texture tiles.
    Repeat,
    /// The texture tiles, mirrored every other time.
    MirroredRepeat,
}

impl AddressMode {
    fn to_vk(self) -> vk::SamplerAddressMode {
        match self {
            Self::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            Self::Repeat => vk::SamplerAddressMode::REPEAT,
            Self::MirroredRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
        }
    }
}

/// Parameters of [`crate::Device::create_sampler`].
#[derive(Clone, Copy, Debug)]
pub struct SamplerDesc<'a> {
    /// Debug name.
    pub label: &'a str,
    /// Filtering, for magnification and minification alike.
    pub filter: FilterMode,
    /// Addressing outside the texture, on both axes.
    pub address_mode: AddressMode,
}

/// How shaders read a texture: filtering and addressing.
///
/// Textures have a single mip level, so samplers never filter between levels. Dropping is
/// always safe: destruction is deferred until pending GPU work completes, and until every
/// [`crate::BindGroup`] referencing the sampler is dropped.
pub struct Sampler {
    shared: Arc<SamplerShared>,
}

/// The Vulkan sampler, shared with the bind groups that reference it.
pub(crate) struct SamplerShared {
    pub(crate) device: Arc<DeviceShared>,
    pub(crate) raw: vk::Sampler,
    filter: FilterMode,
    address_mode: AddressMode,
}

impl Sampler {
    pub(crate) fn new(device: &Arc<DeviceShared>, desc: &SamplerDesc<'_>) -> Result<Self> {
        let filter = desc.filter.to_vk();
        let address_mode = desc.address_mode.to_vk();
        let info = vk::SamplerCreateInfo::default()
            .mag_filter(filter)
            .min_filter(filter)
            .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
            .address_mode_u(address_mode)
            .address_mode_v(address_mode)
            .address_mode_w(address_mode)
            .min_lod(0.0)
            // The specification's advice for single-level textures: minification still
            // selects `min_filter`, and no level beyond the first is read.
            .max_lod(0.25);
        // SAFETY: valid device and create info.
        let raw = unsafe { device.raw.create_sampler(&info, None) }.ctx("vkCreateSampler")?;
        set_object_name(device, raw, desc.label);
        Ok(Self {
            shared: Arc::new(SamplerShared {
                device: Arc::clone(device),
                raw,
                filter: desc.filter,
                address_mode: desc.address_mode,
            }),
        })
    }

    /// Filtering.
    pub fn filter(&self) -> FilterMode {
        self.shared.filter
    }

    /// Addressing outside the texture.
    pub fn address_mode(&self) -> AddressMode {
        self.shared.address_mode
    }

    /// The raw sampler handle.
    pub fn handle(&self) -> vk::Sampler {
        self.shared.raw
    }

    pub(crate) fn shared(&self) -> &Arc<SamplerShared> {
        &self.shared
    }
}

impl fmt::Debug for Sampler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sampler")
            .field("handle", &self.shared.raw)
            .field("filter", &self.shared.filter)
            .field("address_mode", &self.shared.address_mode)
            .finish_non_exhaustive()
    }
}

impl Drop for SamplerShared {
    fn drop(&mut self) {
        self.device
            .defer_destroy(Garbage::Sampler { sampler: self.raw });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sampler_modes_map_to_vulkan() {
        assert_eq!(FilterMode::Nearest.to_vk(), vk::Filter::NEAREST);
        assert_eq!(FilterMode::Linear.to_vk(), vk::Filter::LINEAR);
        assert_eq!(
            AddressMode::ClampToEdge.to_vk(),
            vk::SamplerAddressMode::CLAMP_TO_EDGE
        );
        assert_eq!(
            AddressMode::MirroredRepeat.to_vk(),
            vk::SamplerAddressMode::MIRRORED_REPEAT
        );
    }
}
