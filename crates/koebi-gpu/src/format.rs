//! Texture and vertex formats supported by Koebi.
//!
//! Only formats the renderer actually uses are listed: a small, documented set is easier to
//! validate on every driver than the full Vulkan format table.

use ash::vk;

/// Pixel format of a texture or swapchain image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextureFormat {
    /// 8-bit RGBA, values stored as-is (gamma-space UI rendering).
    Rgba8Unorm,
    /// 8-bit BGRA, values stored as-is. The most common swapchain format on desktop.
    Bgra8Unorm,
    /// 8-bit RGBA with hardware sRGB encoding on write.
    Rgba8Srgb,
    /// 8-bit BGRA with hardware sRGB encoding on write.
    Bgra8Srgb,
    /// 16-bit float RGBA (HDR intermediate targets).
    Rgba16Float,
    /// Single 8-bit channel (masks, glyph coverage).
    R8Unorm,
}

impl TextureFormat {
    /// The matching Vulkan format.
    pub const fn to_vk(self) -> vk::Format {
        match self {
            Self::Rgba8Unorm => vk::Format::R8G8B8A8_UNORM,
            Self::Bgra8Unorm => vk::Format::B8G8R8A8_UNORM,
            Self::Rgba8Srgb => vk::Format::R8G8B8A8_SRGB,
            Self::Bgra8Srgb => vk::Format::B8G8R8A8_SRGB,
            Self::Rgba16Float => vk::Format::R16G16B16A16_SFLOAT,
            Self::R8Unorm => vk::Format::R8_UNORM,
        }
    }

    /// The Koebi format for a Vulkan format, if supported.
    pub const fn from_vk(format: vk::Format) -> Option<Self> {
        Some(match format {
            vk::Format::R8G8B8A8_UNORM => Self::Rgba8Unorm,
            vk::Format::B8G8R8A8_UNORM => Self::Bgra8Unorm,
            vk::Format::R8G8B8A8_SRGB => Self::Rgba8Srgb,
            vk::Format::B8G8R8A8_SRGB => Self::Bgra8Srgb,
            vk::Format::R16G16B16A16_SFLOAT => Self::Rgba16Float,
            vk::Format::R8_UNORM => Self::R8Unorm,
            _ => return None,
        })
    }

    /// Size of one pixel in bytes.
    pub const fn bytes_per_pixel(self) -> u32 {
        match self {
            Self::Rgba8Unorm | Self::Bgra8Unorm | Self::Rgba8Srgb | Self::Bgra8Srgb => 4,
            Self::Rgba16Float => 8,
            Self::R8Unorm => 1,
        }
    }

    /// `true` when the hardware applies sRGB encoding on write (blending in linear space).
    pub const fn is_srgb(self) -> bool {
        matches!(self, Self::Rgba8Srgb | Self::Bgra8Srgb)
    }

    /// `true` when bytes are stored in B, G, R, A order.
    pub const fn is_bgra(self) -> bool {
        matches!(self, Self::Bgra8Unorm | Self::Bgra8Srgb)
    }
}

/// Format of one vertex (or instance) attribute.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VertexFormat {
    /// One `f32`.
    Float32,
    /// Two `f32` (`vec2`).
    Float32x2,
    /// Three `f32` (`vec3`).
    Float32x3,
    /// Four `f32` (`vec4`).
    Float32x4,
    /// One `u32`.
    Uint32,
    /// Four normalized `u8` (`vec4` in `0..=1`), e.g. packed colors.
    Unorm8x4,
}

impl VertexFormat {
    /// The matching Vulkan format.
    pub const fn to_vk(self) -> vk::Format {
        match self {
            Self::Float32 => vk::Format::R32_SFLOAT,
            Self::Float32x2 => vk::Format::R32G32_SFLOAT,
            Self::Float32x3 => vk::Format::R32G32B32_SFLOAT,
            Self::Float32x4 => vk::Format::R32G32B32A32_SFLOAT,
            Self::Uint32 => vk::Format::R32_UINT,
            Self::Unorm8x4 => vk::Format::R8G8B8A8_UNORM,
        }
    }

    /// Size in bytes.
    pub const fn size(self) -> u32 {
        match self {
            Self::Float32 | Self::Uint32 | Self::Unorm8x4 => 4,
            Self::Float32x2 => 8,
            Self::Float32x3 => 12,
            Self::Float32x4 => 16,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_formats_round_trip_through_vulkan() {
        for format in [
            TextureFormat::Rgba8Unorm,
            TextureFormat::Bgra8Unorm,
            TextureFormat::Rgba8Srgb,
            TextureFormat::Bgra8Srgb,
            TextureFormat::Rgba16Float,
            TextureFormat::R8Unorm,
        ] {
            assert_eq!(TextureFormat::from_vk(format.to_vk()), Some(format));
        }
        assert_eq!(TextureFormat::from_vk(vk::Format::D32_SFLOAT), None);
    }

    #[test]
    fn texture_format_properties() {
        assert!(TextureFormat::Bgra8Srgb.is_srgb() && TextureFormat::Bgra8Srgb.is_bgra());
        assert!(!TextureFormat::Rgba8Unorm.is_srgb() && !TextureFormat::Rgba8Unorm.is_bgra());
        assert_eq!(TextureFormat::Rgba16Float.bytes_per_pixel(), 8);
    }

    #[test]
    fn vertex_format_sizes_match_glsl_types() {
        assert_eq!(VertexFormat::Float32x2.size(), 8);
        assert_eq!(VertexFormat::Float32x4.size(), 16);
        assert_eq!(VertexFormat::Unorm8x4.size(), 4);
    }
}
