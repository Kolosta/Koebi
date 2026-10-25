//! The glyph atlas: rasterized glyphs cached in one GPU texture.
//!
//! The first time a frame draws a glyph at a given physical size and subpixel offset, the
//! atlas rasterizes it on the CPU ([`koebi_text::Rasterizer`]), packs it on a shelf of an R8
//! texture and stages its coverage; the uploads are recorded before the frame's render pass,
//! through a staging buffer per frame in flight, so the CPU never waits for the GPU. The atlas
//! starts at 512 × 512 texels and doubles up to 4096 when full; full at 4096, it is cleared and
//! refilled with the glyphs of the current frame.

use std::collections::HashMap;

use koebi_core::Extent;
use koebi_gpu::{
    AddressMode, BindGroup, BindGroupDesc, BindGroupLayout, BindGroupLayoutDesc, BindingResource,
    BindingType, Buffer, BufferDesc, BufferTexels, BufferUsage, CommandEncoder, Device, Error,
    FRAMES_IN_FLIGHT, FilterMode, MemoryLocation, Pod, Result, Sampler, SamplerDesc, Texture,
    TextureDesc, TextureFormat, TextureRegion, TextureState, TextureUsage,
};
use koebi_text::{FontId, Fonts, GlyphBitmap, GlyphId, Rasterizer};

use crate::scene::GlyphPrimitive;

/// Side of a new atlas, in texels.
const INITIAL_SIZE: u32 = 512;
/// Largest side: every Vulkan implementation supports 4096 × 4096 textures.
const MAX_SIZE: u32 = 4096;
/// Empty texels right of and below each glyph.
const PADDING: u32 = 1;
/// Horizontal pen positions are quantized to this many steps per pixel.
const SUBPIXEL_STEPS: f32 = 4.0;
/// Smallest staging buffer, in bytes.
const MIN_STAGING: u64 = 64 * 1024;

/// A glyph as rasterized: font, glyph, physical size in 1/64 pixel and subpixel step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct GlyphKey {
    font: FontId,
    glyph: GlyphId,
    size_64: u32,
    subpixel: u8,
}

impl GlyphKey {
    /// The key of `glyph` drawn at `scale_factor`, and the whole physical pixel where its
    /// bitmap's pen position lies: x rounded down to a subpixel step, baseline rounded.
    pub(crate) fn snap(glyph: &GlyphPrimitive, scale_factor: f32) -> Option<(Self, f32, f32)> {
        let size_64 = (glyph.size * scale_factor * 64.0).round();
        let pen_x = glyph.origin.x * scale_factor;
        let baseline = (glyph.origin.y * scale_factor).round();
        let mut whole = pen_x.floor();
        let mut steps = ((pen_x - whole) * SUBPIXEL_STEPS).round();
        if steps >= SUBPIXEL_STEPS {
            whole += 1.0;
            steps = 0.0;
        }
        let valid = (1.0..1e9).contains(&size_64)
            && [whole, baseline].iter().all(|value| value.is_finite());
        if !valid {
            return None;
        }
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "checked positive and below 1e9; steps are 0 to 3"
        )]
        let key = Self {
            font: glyph.font,
            glyph: glyph.glyph,
            size_64: size_64 as u32,
            subpixel: steps as u8,
        };
        Some((key, whole, baseline))
    }

    fn size(self) -> f32 {
        self.size_64 as f32 / 64.0
    }

    fn offset(self) -> f32 {
        f32::from(self.subpixel) / SUBPIXEL_STEPS
    }
}

/// A glyph's place in the atlas and its offset from the pen position, in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AtlasGlyph {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    left: i32,
    top: i32,
}

/// GPU layout of one glyph quad (vertex buffer, per-instance step).
///
/// Must match the inputs of `shaders/glyph.vert`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub(crate) struct GlyphInstance {
    /// `x, y, width, height` in physical pixels, on whole pixels.
    pub(crate) rect: [f32; 4],
    /// `x, y, width, height` in normalized atlas coordinates.
    pub(crate) uv: [f32; 4],
    /// Premultiplied color.
    pub(crate) color: [f32; 4],
}

// SAFETY: `repr(C)` struct of three `[f32; 4]` arrays: 48 bytes with no padding, and every bit
// pattern is a valid value.
unsafe impl Pod for GlyphInstance {}
const _: () = assert!(size_of::<GlyphInstance>() == 48);

impl GlyphInstance {
    pub(crate) const STRIDE: u32 = 48;

    /// The quad of `entry` with its pen position at (`x`, `baseline`), in an atlas of `size`.
    pub(crate) fn new(
        entry: AtlasGlyph,
        x: f32,
        baseline: f32,
        size: u32,
        color: [f32; 4],
    ) -> Self {
        let texel = 1.0 / size as f32;
        let (width, height) = (entry.width as f32, entry.height as f32);
        Self {
            rect: [
                x + entry.left as f32,
                baseline + entry.top as f32,
                width,
                height,
            ],
            uv: [
                entry.x as f32 * texel,
                entry.y as f32 * texel,
                width * texel,
                height * texel,
            ],
            color,
        }
    }
}

/// Shelf packing: rows of glyphs of similar heights, filled left to right.
#[derive(Debug)]
struct Shelves {
    size: u32,
    rows: Vec<Shelf>,
    next_y: u32,
}

#[derive(Debug)]
struct Shelf {
    y: u32,
    height: u32,
    next_x: u32,
}

impl Shelves {
    fn new(size: u32) -> Self {
        Self {
            size,
            rows: Vec::new(),
            next_y: 0,
        }
    }

    /// Room for `width × height` texels plus padding: its top-left corner.
    fn allocate(&mut self, width: u32, height: u32) -> Option<(u32, u32)> {
        let (width, height) = (width + PADDING, height + PADDING);
        if width > self.size {
            return None;
        }
        // A shelf not much taller than the glyph, else a new shelf, else any shelf with room.
        let index = self
            .shortest(width, height, true)
            .or_else(|| self.open(height))
            .or_else(|| self.shortest(width, height, false))?;
        let shelf = &mut self.rows[index];
        let corner = (shelf.next_x, shelf.y);
        shelf.next_x += width;
        Some(corner)
    }

    /// The shortest shelf with room, among those at most a quarter taller if `snug`.
    fn shortest(&self, width: u32, height: u32, snug: bool) -> Option<usize> {
        self.rows
            .iter()
            .enumerate()
            .filter(|(_, shelf)| shelf.height >= height && self.size - shelf.next_x >= width)
            .filter(|(_, shelf)| !snug || shelf.height <= height + height / 4 + 1)
            .min_by_key(|(_, shelf)| shelf.height)
            .map(|(index, _)| index)
    }

    /// Opens a shelf of `height` below the others, if there is room.
    fn open(&mut self, height: u32) -> Option<usize> {
        (self.size - self.next_y >= height).then(|| {
            self.rows.push(Shelf {
                y: self.next_y,
                height,
                next_x: 0,
            });
            self.next_y += height;
            self.rows.len() - 1
        })
    }
}

/// What looking a glyph up gave.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Lookup {
    /// In the atlas, or `None` for glyphs that draw nothing (spaces, unknown fonts).
    Found(Option<AtlasGlyph>),
    /// The atlas has no room left: grow or clear it, then prepare the frame again.
    Full,
}

/// Rasterized glyphs in a GPU texture, with the uploads of the frame being prepared.
pub(crate) struct GlyphAtlas {
    layout: BindGroupLayout,
    sampler: Sampler,
    texture: Option<Texture>,
    bind_group: Option<BindGroup>,
    /// The texture must be cleared before this frame's uploads.
    clear: bool,
    size: u32,
    shelves: Shelves,
    entries: HashMap<GlyphKey, Option<AtlasGlyph>>,
    fonts: Fonts,
    rasterizer: Rasterizer,
    bitmap: GlyphBitmap,
    /// This frame's coverage, and where each glyph goes.
    staged: Vec<u8>,
    uploads: Vec<(TextureRegion, u64)>,
    staging: Vec<Option<Buffer>>,
}

impl GlyphAtlas {
    pub(crate) fn new(device: &Device) -> Result<Self> {
        Ok(Self {
            layout: device.create_bind_group_layout(&BindGroupLayoutDesc {
                label: "glyph atlas",
                entries: &[BindingType::SampledTexture],
            })?,
            // Glyph quads cover whole pixels with one texel each: nearest reads them exactly.
            sampler: device.create_sampler(&SamplerDesc {
                label: "glyph atlas",
                filter: FilterMode::Nearest,
                address_mode: AddressMode::ClampToEdge,
            })?,
            texture: None,
            bind_group: None,
            clear: false,
            size: INITIAL_SIZE,
            shelves: Shelves::new(INITIAL_SIZE),
            entries: HashMap::new(),
            fonts: Fonts::new(),
            rasterizer: Rasterizer::new(),
            bitmap: GlyphBitmap::default(),
            staged: Vec::new(),
            uploads: Vec::new(),
            staging: (0..FRAMES_IN_FLIGHT).map(|_| None).collect(),
        })
    }

    /// The layout of the atlas's bind group, for the glyph pipeline.
    pub(crate) fn layout(&self) -> &BindGroupLayout {
        &self.layout
    }

    /// The bind group of the atlas texture, once a frame has drawn glyphs.
    pub(crate) fn bind_group(&self) -> Option<&BindGroup> {
        self.bind_group.as_ref()
    }

    /// Side of the atlas texture, in texels.
    pub(crate) fn size(&self) -> u32 {
        self.size
    }

    /// Replaces the fonts glyphs are rasterized from; cached glyphs are forgotten.
    pub(crate) fn set_fonts(&mut self, fonts: Fonts) {
        self.fonts = fonts;
        self.forget();
        self.clear = self.texture.is_some();
    }

    /// Starts preparing a frame.
    pub(crate) fn begin_frame(&mut self) {
        self.staged.clear();
        self.uploads.clear();
    }

    /// The atlas entry of `key`, rasterized and staged on first use.
    pub(crate) fn glyph(&mut self, key: GlyphKey) -> Lookup {
        if let Some(entry) = self.entries.get(&key) {
            return Lookup::Found(*entry);
        }
        let rasterized = self.fonts.get(key.font).is_some_and(|font| {
            self.rasterizer
                .rasterize(font, key.glyph, key.size(), key.offset(), &mut self.bitmap)
                .is_ok()
        });
        if !rasterized || self.bitmap.is_empty() {
            self.entries.insert(key, None);
            return Lookup::Found(None);
        }
        let (width, height) = (self.bitmap.width, self.bitmap.height);
        let Some((x, y)) = self.shelves.allocate(width, height) else {
            return Lookup::Full;
        };
        // Copies start on 4-byte boundaries.
        self.staged.resize(self.staged.len().next_multiple_of(4), 0);
        let offset = self.staged.len() as u64;
        self.staged.extend_from_slice(&self.bitmap.coverage);
        self.uploads
            .push((TextureRegion::new(x, y, width, height), offset));
        let entry = AtlasGlyph {
            x,
            y,
            width,
            height,
            left: self.bitmap.left,
            top: self.bitmap.top,
        };
        self.entries.insert(key, Some(entry));
        Lookup::Found(Some(entry))
    }

    /// Makes room after [`Lookup::Full`]: doubles the atlas, or clears it at the largest size.
    /// Every glyph must be looked up again.
    pub(crate) fn make_room(&mut self) {
        if self.size < MAX_SIZE {
            self.size *= 2;
            koebi_core::debug!("glyph atlas grows to {0} × {0}", self.size);
            // Frames in flight keep the old texture alive through their bind group.
            self.texture = None;
            self.bind_group = None;
        } else {
            koebi_core::debug!("glyph atlas full: cleared");
            self.clear = self.texture.is_some();
        }
        self.forget();
    }

    fn forget(&mut self) {
        self.entries.clear();
        self.shelves = Shelves::new(self.size);
        self.staged.clear();
        self.uploads.clear();
    }

    /// Records this frame's uploads, creating the texture first if needed. Call before the
    /// render pass, with the frame slot the staging buffer may be rewritten for.
    pub(crate) fn record_uploads(
        &mut self,
        device: &Device,
        encoder: &mut CommandEncoder<'_>,
        frame_index: usize,
    ) -> Result<()> {
        let created = self.texture.is_none();
        if created {
            let texture = device.create_texture(&TextureDesc {
                label: "glyph atlas",
                extent: Extent::new(self.size, self.size),
                format: TextureFormat::R8Unorm,
                usage: TextureUsage::SAMPLED | TextureUsage::TRANSFER_DST,
            })?;
            self.bind_group = Some(device.create_bind_group(&BindGroupDesc {
                label: "glyph atlas",
                layout: &self.layout,
                entries: &[BindingResource::SampledTexture {
                    texture: &texture,
                    sampler: &self.sampler,
                }],
            })?);
            self.texture = Some(texture);
        }
        let Some(texture) = self.texture.as_ref() else {
            return Ok(());
        };
        if !created && !self.clear && self.uploads.is_empty() {
            return Ok(());
        }
        let staging = if self.staged.is_empty() {
            None
        } else {
            let slot = self
                .staging
                .get_mut(frame_index)
                .ok_or_else(|| Error::InvalidArgument(format!("frame index {frame_index}")))?;
            let needed = self.staged.len() as u64;
            if slot.as_ref().is_none_or(|buffer| buffer.size() < needed) {
                *slot = Some(device.create_buffer(&BufferDesc {
                    label: "glyph staging",
                    size: needed.next_power_of_two().max(MIN_STAGING),
                    usage: BufferUsage::TRANSFER_SRC,
                    location: MemoryLocation::CpuToGpu,
                })?);
            }
            let buffer = slot
                .as_mut()
                .ok_or_else(|| Error::InvalidArgument("glyph staging buffer missing".into()))?;
            buffer.write(0, &self.staged)?;
            Some(&*buffer)
        };
        let from = if created {
            TextureState::Undefined
        } else {
            TextureState::ShaderRead
        };
        encoder.texture_barrier(texture, from, TextureState::TransferDst);
        if created || self.clear {
            encoder.clear_texture(texture, [0.0; 4])?;
            self.clear = false;
            if staging.is_some() {
                // The copies write after the clear: order the two writes.
                encoder.texture_barrier(
                    texture,
                    TextureState::TransferDst,
                    TextureState::TransferDst,
                );
            }
        }
        if let Some(staging) = staging {
            for (region, offset) in &self.uploads {
                let texels = BufferTexels {
                    offset: *offset,
                    bytes_per_row: region.width,
                };
                encoder.copy_buffer_to_texture(staging, texels, texture, *region)?;
            }
        }
        encoder.texture_barrier(texture, TextureState::TransferDst, TextureState::ShaderRead);
        Ok(())
    }
}

impl std::fmt::Debug for GlyphAtlas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlyphAtlas")
            .field("size", &self.size)
            .field("glyphs", &self.entries.len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::{Color, Vec2};

    use super::*;

    fn primitive(x: f32, y: f32, size: f32) -> GlyphPrimitive {
        GlyphPrimitive {
            font: FontId::default(),
            glyph: GlyphId(36),
            size,
            origin: Vec2::new(x, y),
            color: Color::WHITE,
        }
    }

    #[test]
    fn pen_positions_snap_to_quarter_pixels_and_whole_baselines() {
        let (key, x, baseline) = GlyphKey::snap(&primitive(10.3, 20.4, 14.0), 1.0).unwrap();
        assert_eq!((x, baseline), (10.0, 20.0));
        assert_eq!((key.subpixel, key.size_64), (1, 14 * 64));
        // 0.9 rounds to the next whole pixel, not to a fourth step.
        let (key, x, _) = GlyphKey::snap(&primitive(10.9, 0.0, 14.0), 1.0).unwrap();
        assert_eq!((x, key.subpixel), (11.0, 0));
        // At 1.5: 10.3 logical pixels are 15.45 physical, size 21.
        let (key, x, baseline) = GlyphKey::snap(&primitive(10.3, 20.4, 14.0), 1.5).unwrap();
        assert_eq!(
            (x, baseline, key.subpixel, key.size_64),
            (15.0, 31.0, 2, 21 * 64)
        );
        assert!((key.size() - 21.0).abs() < f32::EPSILON && (key.offset() - 0.5).abs() < 1e-6);
        assert!(GlyphKey::snap(&primitive(0.0, 0.0, 0.0), 1.0).is_none());
        assert!(GlyphKey::snap(&primitive(f32::NAN, 0.0, 12.0), 1.0).is_none());
    }

    #[test]
    fn shelves_pack_rows_of_similar_heights() {
        let mut shelves = Shelves::new(32);
        // With one texel of padding, a 9 × 15 glyph takes 10 × 16.
        assert_eq!(shelves.allocate(9, 15), Some((0, 0)));
        assert_eq!(shelves.allocate(9, 15), Some((10, 0)));
        // Slightly shorter glyphs share the shelf, until it is full.
        assert_eq!(shelves.allocate(9, 13), Some((20, 0)));
        assert_eq!(shelves.allocate(1, 14), Some((30, 0)));
        assert_eq!(shelves.allocate(9, 15), Some((0, 16)));
        // Much shorter glyphs would open their own shelf; without room for one, they take
        // any shelf with room.
        assert_eq!(shelves.allocate(4, 4), Some((10, 16)));
        assert_eq!(shelves.allocate(30, 4), None);
        assert_eq!(shelves.allocate(40, 1), None);
    }

    #[test]
    fn instances_place_bitmaps_on_their_pen_position() {
        let entry = AtlasGlyph {
            x: 64,
            y: 128,
            width: 8,
            height: 10,
            left: 1,
            top: -9,
        };
        let instance = GlyphInstance::new(entry, 20.0, 30.0, 512, [1.0; 4]);
        assert_eq!(instance.rect, [21.0, 21.0, 8.0, 10.0]);
        assert_eq!(instance.uv, [0.125, 0.25, 8.0 / 512.0, 10.0 / 512.0]);
    }
}
