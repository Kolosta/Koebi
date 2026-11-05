//! Vector icons: the icons compiled by `koebi-svg`, drawn from their curves in the fragment
//! shader at any size, recolored per instance (`docs/adr/0020-vector-icons.md`).
//!
//! The renderer holds one icon set ([`Renderer2D::set_icons`](crate::Renderer2D::set_icons)):
//! its packed curves, gradients and zones live in a data texture of `vec4` texels the shader
//! reads one by one (`texelFetch`), uploaded once. A frame's icons are instances like
//! rectangles': a square, the icon's number in the set, the color of each zone or the original
//! colors, an opacity. Nothing is rasterized ahead of time, so an icon costs the same at every
//! size and stays sharp when the interface scales.

use koebi_core::{Color, Extent, Rect};
use koebi_gpu::{
    AddressMode, BindGroup, BindGroupDesc, BindGroupLayout, BindGroupLayoutDesc, BindingResource,
    BindingType, BufferDesc, BufferTexels, BufferUsage, CommandEncoder, Device, Error, FilterMode,
    MemoryLocation, Pod, Result, Sampler, SamplerDesc, Texture, TextureDesc, TextureFormat,
    TextureRegion, TextureState, TextureUsage,
};
use koebi_svg::pack::Packed;

/// Zones an icon instance can recolor: as many as an icon may have.
pub const ICON_ZONES: usize = koebi_svg::MAX_ZONES;

/// Texels per row of the data texture; the shader computes the same coordinates.
const DATA_WIDTH: u32 = 4096;

/// How an icon's color zones are painted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IconColors {
    /// The colors of its SVG file.
    Original,
    /// Zone `i` in `colors[i]`, the zone's own opacity (a translucent fill, a gradient's
    /// stops) multiplying the color's alpha; a zone without a color keeps its original one.
    Zones([Option<Color>; ICON_ZONES]),
}

impl IconColors {
    /// Every zone in `color`: a monochrome icon tinted by the interface.
    pub fn uniform(color: Color) -> Self {
        Self::Zones([Some(color); ICON_ZONES])
    }

    /// Zone `i` in `colors[i]`; zones past the end keep their original colors.
    pub fn zones(colors: &[Color]) -> Self {
        let mut zones = [None; ICON_ZONES];
        for (zone, color) in zones.iter_mut().zip(colors) {
            *zone = Some(*color);
        }
        Self::Zones(zones)
    }
}

/// An icon of the renderer's set drawn in a square.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconPrimitive {
    /// Where to draw it, in logical pixels: the icon's square is the largest centered in it,
    /// its corner and side rounded to whole physical pixels.
    pub rect: Rect,
    /// Its number in the set given to [`Renderer2D::set_icons`](crate::Renderer2D::set_icons);
    /// a number past the set draws nothing.
    pub icon: u32,
    /// How its zones are painted.
    pub colors: IconColors,
    /// Multiplies the whole icon's alpha, 0 to 1.
    pub opacity: f32,
    /// Turns the icon clockwise about its center, in radians: a fold arrow opening. Its square
    /// leaves the pixel grid unless the angle is a multiple of a quarter turn.
    pub rotation: f32,
}

impl IconPrimitive {
    /// Icon `icon` in `rect`, every zone in `color`.
    pub fn tinted(rect: Rect, icon: u32, color: Color) -> Self {
        Self {
            rect,
            icon,
            colors: IconColors::uniform(color),
            opacity: 1.0,
            rotation: 0.0,
        }
    }
}

/// GPU instance of an icon. Must match `shaders/icon.vert`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub(crate) struct IconInstance {
    /// x, y, width, height (logical px).
    rect: [f32; 4],
    /// The icon's number, the mask of the zones recolored, the opacity, the rotation.
    params: [f32; 4],
    /// Each zone's color, gamma-encoded with straight alpha.
    colors: [[f32; 4]; ICON_ZONES],
}

// SAFETY: `repr(C)` struct of ten `[f32; 4]` arrays: 160 bytes with no padding, and every bit
// pattern is a valid `f32`.
unsafe impl Pod for IconInstance {}
const _: () = assert!(size_of::<IconInstance>() == 160);

impl IconInstance {
    pub(crate) const STRIDE: u32 = 160;

    pub(crate) fn from_primitive(primitive: &IconPrimitive) -> Self {
        let mut mask = 0_u32;
        let mut colors = [[0.0; 4]; ICON_ZONES];
        if let IconColors::Zones(zones) = primitive.colors {
            for (index, (zone, slot)) in zones.iter().zip(&mut colors).enumerate() {
                if let Some(color) = zone {
                    mask |= 1 << index;
                    *slot = [color.r, color.g, color.b, color.a];
                }
            }
        }
        let rect = primitive.rect;
        Self {
            rect: [rect.min.x, rect.min.y, rect.width(), rect.height()],
            params: [
                // Exact: icon numbers past 2^24 are past any set (its texture holds 2^24
                // texels), and the shader draws nothing for them.
                primitive.icon.min(1 << 24) as f32,
                mask as f32,
                primitive.opacity.clamp(0.0, 1.0),
                if primitive.rotation.is_finite() {
                    primitive.rotation
                } else {
                    0.0
                },
            ],
            colors,
        }
    }
}

/// The icon set in its data texture.
pub(crate) struct IconStore {
    layout: BindGroupLayout,
    sampler: Sampler,
    texture: Option<Texture>,
    bind_group: Option<BindGroup>,
    /// The set waiting to be uploaded, padded to whole rows.
    pending: Vec<[f32; 4]>,
    icons: usize,
}

impl IconStore {
    pub(crate) fn new(device: &Device) -> Result<Self> {
        Ok(Self {
            layout: device.create_bind_group_layout(&BindGroupLayoutDesc {
                label: "icons",
                entries: &[BindingType::SampledTexture],
            })?,
            // Read with texelFetch, never filtered.
            sampler: device.create_sampler(&SamplerDesc {
                label: "icons",
                filter: FilterMode::Nearest,
                address_mode: AddressMode::ClampToEdge,
            })?,
            texture: None,
            bind_group: None,
            pending: Vec::new(),
            icons: 0,
        })
    }

    /// The layout of the set's bind group, for the icon pipeline.
    pub(crate) fn layout(&self) -> &BindGroupLayout {
        &self.layout
    }

    /// The bind group of the uploaded set.
    pub(crate) fn bind_group(&self) -> Option<&BindGroup> {
        self.bind_group.as_ref()
    }

    /// How many icons the set holds.
    pub(crate) fn len(&self) -> usize {
        self.icons
    }

    /// Replaces the set, once checked; it is uploaded by the next
    /// [`IconStore::record_upload`]. The device destroys the previous texture and bind group
    /// once the frames in flight that use them complete.
    pub(crate) fn set(&mut self, packed: &Packed) -> Result<()> {
        packed
            .validate()
            .map_err(|problem| Error::InvalidArgument(format!("icon set: {problem}")))?;
        self.pending.clear();
        self.pending.extend_from_slice(&packed.data);
        let width = DATA_WIDTH as usize;
        self.pending
            .resize(self.pending.len().div_ceil(width).max(1) * width, [0.0; 4]);
        self.icons = packed.icons;
        self.texture = None;
        self.bind_group = None;
        Ok(())
    }

    /// Records the upload of a new set before the render pass. Does nothing once the set is
    /// uploaded. A set the device cannot hold is dropped with a warning, and icons draw nothing:
    /// the rest of the frame still draws.
    pub(crate) fn record_upload(&mut self, device: &Device, encoder: &mut CommandEncoder<'_>) {
        if self.pending.is_empty() {
            return;
        }
        if let Err(error) = self.upload(device, encoder) {
            koebi_core::warn!("the icon set cannot be uploaded, icons are not drawn: {error}");
        }
        // Uploaded or dropped: the copy is not kept.
        self.pending = Vec::new();
    }

    fn upload(&mut self, device: &Device, encoder: &mut CommandEncoder<'_>) -> Result<()> {
        // `set` checked the size: at most 2^24 texels, 4096 rows.
        let rows = u32::try_from(self.pending.len() / DATA_WIDTH as usize)
            .map_err(|_| Error::InvalidArgument("icon set too large".into()))?;
        let texture = device.create_texture(&TextureDesc {
            label: "icons",
            extent: Extent::new(DATA_WIDTH, rows),
            format: TextureFormat::Rgba32Float,
            usage: TextureUsage::SAMPLED | TextureUsage::TRANSFER_DST,
        })?;
        let mut staging = device.create_buffer(&BufferDesc {
            label: "icon staging",
            size: size_of_val(self.pending.as_slice()) as u64,
            usage: BufferUsage::TRANSFER_SRC,
            location: MemoryLocation::CpuToGpu,
        })?;
        staging.write(0, &self.pending)?;
        let bind_group = device.create_bind_group(&BindGroupDesc {
            label: "icons",
            layout: &self.layout,
            entries: &[BindingResource::SampledTexture {
                texture: &texture,
                sampler: &self.sampler,
            }],
        })?;
        encoder.texture_barrier(&texture, TextureState::Undefined, TextureState::TransferDst);
        encoder.copy_buffer_to_texture(
            &staging,
            BufferTexels {
                offset: 0,
                bytes_per_row: DATA_WIDTH * 16,
            },
            &texture,
            TextureRegion::new(0, 0, DATA_WIDTH, rows),
        )?;
        encoder.texture_barrier(
            &texture,
            TextureState::TransferDst,
            TextureState::ShaderRead,
        );
        self.texture = Some(texture);
        self.bind_group = Some(bind_group);
        // Dropped now, destroyed by the device once the submission copying it completes.
        drop(staging);
        Ok(())
    }
}

impl std::fmt::Debug for IconStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IconStore")
            .field("icons", &self.icons)
            .field("uploaded", &self.texture.is_some())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use koebi_core::Vec2;

    fn rect() -> Rect {
        Rect::from_origin_size(Vec2::new(4.0, 6.0), Vec2::new(16.0, 20.0))
    }

    #[test]
    fn instances_carry_the_zones_they_recolor() {
        let red = Color::new(1.0, 0.0, 0.0, 0.5);
        let instance = IconInstance::from_primitive(&IconPrimitive {
            rect: rect(),
            icon: 3,
            colors: IconColors::zones(&[red, Color::WHITE]),
            opacity: 0.25,
            rotation: 0.5,
        });
        assert_eq!(instance.rect, [4.0, 6.0, 16.0, 20.0]);
        assert_eq!(instance.params, [3.0, 3.0, 0.25, 0.5]);
        assert_eq!(instance.colors[0], [1.0, 0.0, 0.0, 0.5]);
        assert_eq!(instance.colors[1], [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(instance.colors[2], [0.0; 4]);
    }

    #[test]
    fn original_colors_recolor_nothing_and_uniform_colors_every_zone() {
        let original = IconInstance::from_primitive(&IconPrimitive {
            rect: rect(),
            icon: 0,
            colors: IconColors::Original,
            opacity: 2.0,
            rotation: f32::NAN,
        });
        assert_eq!(original.params, [0.0, 0.0, 1.0, 0.0]);
        let tinted = IconInstance::from_primitive(&IconPrimitive::tinted(rect(), 1, Color::BLACK));
        assert_eq!(tinted.params[1], 255.0);
    }
}
