//! Glyphs drawn from their curves (`docs/adr/0022-text-on-the-gpu.md`).
//!
//! The first time a frame draws a glyph, its outline (`koebi_text::Outline`, quadratic
//! TrueType contours) is turned into curves monotonic in x and y, mapped to the unit square of
//! the glyph's bounds, and filed by bands: the curves crossing each horizontal band (for the
//! ray going right) and each vertical band (for the ray going down). The glyph's record is
//! appended to a data texture of `vec4` texels that keeps every glyph drawn so far; the
//! fragment shader covers each pixel from the curves of its two bands, as the icon shader does
//! from an icon's curves. Nothing depends on the size: a glyph is stored once and drawn at any
//! size, rotated, from any font.
//!
//! Appended texels go up through a staging buffer per frame in flight, so the CPU never waits
//! for the GPU. The texture starts with 64 rows of 4096 texels and doubles up to 1024 rows; full
//! at 1024, it is cleared and refilled with the glyphs of the current frame.

use std::collections::HashMap;

use koebi_core::{Extent, Vec2};
use koebi_gpu::{
    AddressMode, BindGroup, BindGroupDesc, BindGroupLayout, BindGroupLayoutDesc, BindingResource,
    BindingType, Buffer, BufferDesc, BufferTexels, BufferUsage, CommandEncoder, Device, Error,
    FRAMES_IN_FLIGHT, FilterMode, MemoryLocation, Pod, Result, Sampler, SamplerDesc, Texture,
    TextureDesc, TextureFormat, TextureRegion, TextureState, TextureUsage,
};
use koebi_text::{FontId, Fonts, GlyphId, MonotonicCurve, Outline};

/// Texels per row of the data texture (`WIDTH_SHIFT` in `glyph.frag`).
pub(crate) const DATA_WIDTH: u32 = 4096;
/// Rows of a new data texture.
const INITIAL_ROWS: u32 = 64;
/// Most rows: 4M texels, 64 MiB.
const MAX_ROWS: u32 = 1024;
/// Most bands along each axis.
const MAX_BANDS: usize = 16;
/// About this many curves per band.
const CURVES_PER_BAND: usize = 4;
/// Smallest staging buffer, in bytes.
const MIN_STAGING: u64 = 64 * 1024;
/// Header texels before the first glyph: `[glyph count, length in texels, 0, 0]`.
const HEADER: usize = 1;

/// A glyph's record in the data texture, and its bounds in ems (y down, the pen at the origin).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StoredGlyph {
    /// The first texel of its record.
    pub(crate) offset: u32,
    /// `x min, y min, x max, y max`, in ems.
    pub(crate) bounds: [f32; 4],
}

/// What a lookup found.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Lookup {
    /// The glyph, or `None` for one that draws nothing (a space, a broken outline).
    Found(Option<StoredGlyph>),
    /// No room left: [`GlyphStore::make_room`], then look every glyph of the frame up again.
    Full,
}

type Curve = MonotonicCurve;

/// The record of a glyph, its curves in the unit square of its bounds:
///
/// ```text
/// [horizontal bands, vertical bands, curves, 0]
/// [x min, y min, x max, y max]                   bounds in ems
/// [first texel, curve count, 0, 0] × bands       horizontal, then vertical
/// [a.x, a.y, b.x, b.y] [c.x, c.y, 0, 0] × curves  each band's curves, one after the other
/// ```
pub(crate) fn record(curves: &[Curve], bounds: [f32; 4], offset: u32) -> Vec<[f32; 4]> {
    let [x0, y0, x1, y1] = bounds;
    let size = Vec2::new((x1 - x0).max(1e-6), (y1 - y0).max(1e-6));
    let unit: Vec<Curve> = curves
        .iter()
        .map(|curve| curve.map(|p| Vec2::new((p.x - x0) / size.x, (p.y - y0) / size.y)))
        .collect();
    let bands = unit.len().div_ceil(CURVES_PER_BAND).clamp(1, MAX_BANDS);
    // A band holds the curves whose span along its axis meets it.
    let band_curves = |axis: fn(Vec2) -> f32| -> Vec<Vec<usize>> {
        (0..bands)
            .map(|band| {
                let (low, high) = (band as f32 / bands as f32, (band + 1) as f32 / bands as f32);
                (0..unit.len())
                    .filter(|&index| {
                        let [a, _, c] = unit[index];
                        let (from, to) = (axis(a).min(axis(c)), axis(a).max(axis(c)));
                        from <= high + 1e-4 && to >= low - 1e-4
                    })
                    .collect()
            })
            .collect()
    };
    let rows = band_curves(|p| p.y);
    let columns = band_curves(|p| p.x);
    #[allow(clippy::cast_precision_loss, reason = "counts far below 2^24")]
    let mut out = vec![[bands as f32, bands as f32, unit.len() as f32, 0.0], bounds];
    let mut next = offset as usize + 2 + 2 * bands;
    let mut tables = Vec::with_capacity(2 * bands);
    let mut texels = Vec::new();
    for band in rows.iter().chain(&columns) {
        #[allow(
            clippy::cast_precision_loss,
            reason = "offsets below 2^24, counts below 2^14"
        )]
        tables.push([next as f32, band.len() as f32, 0.0, 0.0]);
        for &index in band {
            let [a, b, c] = unit[index];
            texels.push([a.x, a.y, b.x, b.y]);
            texels.push([c.x, c.y, 0.0, 0.0]);
        }
        next += 2 * band.len();
    }
    out.extend(tables);
    out.extend(texels);
    out
}

/// GPU layout of one glyph (vertex buffer, per-instance step). Must match the inputs of
/// `shaders/glyph.vert`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub(crate) struct GlyphInstance {
    /// The pen position in physical pixels, then the em's x axis on screen (physical pixels).
    pub(crate) origin_axis_x: [f32; 4],
    /// The em's y axis on screen, the record's first texel, a margin in ems.
    pub(crate) axis_y_record: [f32; 4],
    /// The glyph's bounds in ems: `x min, y min, x max, y max`.
    pub(crate) bounds: [f32; 4],
    /// Premultiplied color.
    pub(crate) color: [f32; 4],
}

// SAFETY: `repr(C)` struct of four `[f32; 4]` arrays: 64 bytes with no padding, and every bit
// pattern is a valid value.
unsafe impl Pod for GlyphInstance {}
const _: () = assert!(size_of::<GlyphInstance>() == 64);

impl GlyphInstance {
    pub(crate) const STRIDE: u32 = 64;

    /// The quad of `glyph` with its pen at `origin` (physical pixels), `size` physical pixels
    /// per em, turned clockwise by `rotation` radians.
    pub(crate) fn new(
        glyph: StoredGlyph,
        origin: Vec2,
        size: f32,
        rotation: f32,
        color: [f32; 4],
    ) -> Self {
        let (sin, cos) = rotation.sin_cos();
        let axis_x = Vec2::new(cos, sin) * size;
        let axis_y = Vec2::new(-sin, cos) * size;
        // A pixel of margin around the bounds: the coverage of the edge pixels.
        let margin = 1.0 / size.max(1e-3);
        #[allow(clippy::cast_precision_loss, reason = "offsets below 2^24")]
        Self {
            origin_axis_x: [origin.x, origin.y, axis_x.x, axis_x.y],
            axis_y_record: [axis_y.x, axis_y.y, glyph.offset as f32, margin],
            bounds: glyph.bounds,
            color,
        }
    }
}

/// Every glyph drawn so far, in a data texture of `vec4` texels.
///
/// The texture holds the header, then the records in the order glyphs were first drawn; only
/// the records stored since the last upload stay on the CPU. A texture that grows copies what
/// it held to the larger one on the GPU. Every upload leaves the texture ready to be sampled,
/// even when a step fails: the store changes only once what it needs exists.
pub(crate) struct GlyphStore {
    layout: BindGroupLayout,
    sampler: Sampler,
    texture: Option<Texture>,
    bind_group: Option<BindGroup>,
    rows: u32,
    /// Texels the store holds, the header included: where the next record starts.
    length: usize,
    /// Texels on the GPU already; past them, `pending` holds the records to upload.
    uploaded: usize,
    /// The records stored since the last upload.
    pending: Vec<[f32; 4]>,
    /// Glyphs stored with curves.
    stored: usize,
    entries: HashMap<(FontId, GlyphId), Option<StoredGlyph>>,
    fonts: Fonts,
    outline: Outline,
    curves: Vec<Curve>,
    staging: Vec<Option<Buffer>>,
    /// The copies of an upload and where their texels start in the staging buffer: reused.
    regions: Vec<(TextureRegion, u64)>,
}

/// Bytes of a texel.
const TEXEL: u64 = 16;

impl GlyphStore {
    pub(crate) fn new(device: &Device) -> Result<Self> {
        Ok(Self {
            layout: device.create_bind_group_layout(&BindGroupLayoutDesc {
                label: "glyph curves",
                entries: &[BindingType::SampledTexture],
            })?,
            // Read with texelFetch: the sampler is never used to filter.
            sampler: device.create_sampler(&SamplerDesc {
                label: "glyph curves",
                filter: FilterMode::Nearest,
                address_mode: AddressMode::ClampToEdge,
            })?,
            texture: None,
            bind_group: None,
            rows: INITIAL_ROWS,
            length: HEADER,
            uploaded: HEADER,
            pending: Vec::new(),
            stored: 0,
            entries: HashMap::new(),
            fonts: Fonts::new(),
            outline: Outline::new(),
            curves: Vec::new(),
            staging: (0..FRAMES_IN_FLIGHT).map(|_| None).collect(),
            regions: Vec::with_capacity(4),
        })
    }

    /// The layout of the store's bind group, for the glyph pipeline.
    pub(crate) fn layout(&self) -> &BindGroupLayout {
        &self.layout
    }

    /// The bind group of the data texture, once a frame has drawn glyphs.
    pub(crate) fn bind_group(&self) -> Option<&BindGroup> {
        self.bind_group.as_ref()
    }

    /// Replaces the fonts glyphs come from; stored glyphs are forgotten.
    pub(crate) fn set_fonts(&mut self, fonts: Fonts) {
        self.fonts = fonts;
        self.forget();
    }

    /// The record of `glyph` of `font`, stored on first use.
    pub(crate) fn glyph(&mut self, font: FontId, glyph: GlyphId) -> Lookup {
        if let Some(entry) = self.entries.get(&(font, glyph)) {
            return Lookup::Found(*entry);
        }
        let curves = &mut self.curves;
        let bounds = self.fonts.get(font).and_then(|face| {
            self.outline.load(face, glyph).ok()?;
            self.outline
                .monotonic_curves(f32::from(face.units_per_em()), curves)
        });
        let Some(bounds) = bounds else {
            self.entries.insert((font, glyph), None);
            return Lookup::Found(None);
        };
        let capacity = (MAX_ROWS * DATA_WIDTH) as usize;
        #[allow(clippy::cast_possible_truncation, reason = "below the capacity, 2^22")]
        let offset = self.length as u32;
        let texels = record(&self.curves, bounds, offset);
        if self.length + texels.len() > capacity {
            return Lookup::Full;
        }
        self.length += texels.len();
        self.pending.extend(texels);
        self.stored += 1;
        let stored = StoredGlyph { offset, bounds };
        self.entries.insert((font, glyph), Some(stored));
        Lookup::Found(Some(stored))
    }

    /// Makes room after [`Lookup::Full`]: everything is forgotten and the frame's glyphs are
    /// stored again.
    pub(crate) fn make_room(&mut self) {
        koebi_core::debug!("glyph curves full: cleared");
        self.forget();
    }

    /// Forgets every record: the texture keeps its texels, which the next header puts out of
    /// reach until new records cover them.
    fn forget(&mut self) {
        self.entries.clear();
        self.pending.clear();
        self.length = HEADER;
        self.uploaded = HEADER;
        self.stored = 0;
    }

    /// Records the upload of the header and of the records stored since the last upload,
    /// creating or growing the texture first if needed. Call before the render pass, with the
    /// frame slot whose staging buffer may be rewritten.
    ///
    /// # Errors
    ///
    /// A texture, bind group or staging buffer that cannot be created, or a copy refused. The
    /// store keeps what it has not uploaded, and its texture is left ready to be sampled.
    pub(crate) fn record_uploads(
        &mut self,
        device: &Device,
        encoder: &mut CommandEncoder<'_>,
        frame_index: usize,
    ) -> Result<()> {
        let needed = self.length.div_ceil(DATA_WIDTH as usize);
        let mut rows = self.rows;
        while (rows as usize) < needed && rows < MAX_ROWS {
            rows *= 2;
        }
        let creates = self.texture.is_none() || rows != self.rows;
        if !creates && self.pending.is_empty() {
            return Ok(());
        }
        // What may fail comes first, so that a failure leaves the store as it was.
        let created = if creates {
            let texture = device.create_texture(&TextureDesc {
                label: "glyph curves",
                extent: Extent::new(DATA_WIDTH, rows),
                format: TextureFormat::Rgba32Float,
                usage: TextureUsage::SAMPLED
                    | TextureUsage::TRANSFER_DST
                    | TextureUsage::TRANSFER_SRC,
            })?;
            let bind_group = device.create_bind_group(&BindGroupDesc {
                label: "glyph curves",
                layout: &self.layout,
                entries: &[BindingResource::SampledTexture {
                    texture: &texture,
                    sampler: &self.sampler,
                }],
            })?;
            Some((texture, bind_group))
        } else {
            None
        };
        let bytes = TEXEL * (HEADER + self.pending.len()) as u64;
        let buffer = staging_buffer(&mut self.staging, device, frame_index, bytes)?;
        #[allow(clippy::cast_precision_loss, reason = "counts below 2^22")]
        let header = [self.stored as f32, self.length as f32, 0.0, 0.0];
        buffer.write(0, &[header])?;
        buffer.write(TEXEL * HEADER as u64, &self.pending)?;
        self.regions.clear();
        self.regions.push((TextureRegion::new(0, 0, 1, 1), 0));
        regions_of(
            self.uploaded,
            self.length,
            TEXEL * HEADER as u64,
            &mut self.regions,
        );

        // Past this point only a refused copy fails, and the texture still goes back to being
        // sampled.
        let replaced = created.map(|(texture, bind_group)| {
            // Frames in flight keep the old texture alive through their bind group.
            self.bind_group = Some(bind_group);
            self.rows = rows;
            self.texture.replace(texture)
        });
        let Some(texture) = self.texture.as_ref() else {
            return Ok(());
        };
        let mut copied = Ok(());
        match replaced {
            Some(old) => {
                encoder.texture_barrier(
                    texture,
                    TextureState::Undefined,
                    TextureState::TransferDst,
                );
                // What the old texture holds goes over on the GPU: the CPU no longer has it.
                if let Some(old) = old.filter(|_| self.uploaded > HEADER) {
                    #[allow(clippy::cast_possible_truncation, reason = "rows below MAX_ROWS")]
                    let held = self.uploaded.div_ceil(DATA_WIDTH as usize) as u32;
                    encoder.texture_barrier(
                        &old,
                        TextureState::ShaderRead,
                        TextureState::TransferSrc,
                    );
                    copied = encoder.copy_texture_to_texture(
                        &old,
                        texture,
                        TextureRegion::new(0, 0, DATA_WIDTH, held),
                    );
                    // The header and the end of the last row held are written again below.
                    encoder.texture_barrier(
                        texture,
                        TextureState::TransferDst,
                        TextureState::TransferDst,
                    );
                }
            }
            None => {
                encoder.texture_barrier(
                    texture,
                    TextureState::ShaderRead,
                    TextureState::TransferDst,
                );
            }
        }
        let Some(buffer) = self.staging.get(frame_index).and_then(Option::as_ref) else {
            return Err(Error::InvalidArgument(
                "glyph staging buffer missing".into(),
            ));
        };
        for &(region, offset) in &self.regions {
            if copied.is_ok() {
                let texels = BufferTexels {
                    offset,
                    bytes_per_row: DATA_WIDTH * 16,
                };
                copied = encoder.copy_buffer_to_texture(buffer, texels, texture, region);
            }
        }
        encoder.texture_barrier(texture, TextureState::TransferDst, TextureState::ShaderRead);
        copied?;
        self.uploaded = self.length;
        self.pending.clear();
        Ok(())
    }
}

/// The staging buffer of `frame_index`, of at least `bytes`; one grown for a large upload
/// shrinks back once uploads are small again.
fn staging_buffer<'a>(
    staging: &'a mut [Option<Buffer>],
    device: &Device,
    frame_index: usize,
    bytes: u64,
) -> Result<&'a mut Buffer> {
    let size = bytes.next_power_of_two().max(MIN_STAGING);
    let slot = staging
        .get_mut(frame_index)
        .ok_or_else(|| Error::InvalidArgument(format!("frame index {frame_index}")))?;
    if slot
        .as_ref()
        .is_none_or(|buffer| buffer.size() < bytes || buffer.size() > 4 * size)
    {
        *slot = Some(device.create_buffer(&BufferDesc {
            label: "glyph curves staging",
            size,
            usage: BufferUsage::TRANSFER_SRC,
            location: MemoryLocation::CpuToGpu,
        })?);
    }
    slot.as_mut()
        .ok_or_else(|| Error::InvalidArgument("glyph staging buffer missing".into()))
}

/// The copies of texels `start..end` of the store, which lie one after the other in the staging
/// buffer from `offset`: the end of a first row, whole rows, the start of a last row.
fn regions_of(start: usize, end: usize, mut offset: u64, out: &mut Vec<(TextureRegion, u64)>) {
    let width = DATA_WIDTH as usize;
    let mut push = |at: usize, len: usize, rows: usize| {
        #[allow(clippy::cast_possible_truncation, reason = "texels below 2^22")]
        let (x, y, len32) = ((at % width) as u32, (at / width) as u32, len as u32);
        let region = if rows == 0 {
            TextureRegion::new(x, y, len32, 1)
        } else {
            #[allow(clippy::cast_possible_truncation, reason = "rows below 2^10")]
            TextureRegion::new(0, y, DATA_WIDTH, rows as u32)
        };
        out.push((region, offset));
        offset += TEXEL * len as u64;
    };
    let mut at = start;
    if !at.is_multiple_of(width) && at < end {
        let stop = end.min(at.next_multiple_of(width));
        push(at, stop - at, 0);
        at = stop;
    }
    let whole = end.saturating_sub(at) / width;
    if whole > 0 {
        push(at, whole * width, whole);
        at += whole * width;
    }
    if at < end {
        push(at, end - at, 0);
    }
}

impl std::fmt::Debug for GlyphStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlyphStore")
            .field("glyphs", &self.stored)
            .field("texels", &self.length)
            .field("rows", &self.rows)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regions(start: usize, end: usize) -> Vec<(u32, u32, u32, u32, u64)> {
        let mut out = Vec::new();
        regions_of(start, end, 16, &mut out);
        out.into_iter()
            .map(|(r, offset)| (r.x, r.y, r.width, r.height, offset))
            .collect()
    }

    #[test]
    fn uploads_copy_the_new_texels_only_in_three_rectangles_at_most() {
        let w = DATA_WIDTH;
        let width = DATA_WIDTH as usize;
        // Inside one row.
        assert_eq!(regions(10, 30), [(10, 0, 20, 1, 16)]);
        // The end of a row, whole rows, the start of another; offsets follow one another.
        let texels = |n: u32| 16 + 16 * u64::from(n);
        assert_eq!(
            regions(width - 6, 3 * width + 5),
            [
                (w - 6, 0, 6, 1, 16),
                (0, 1, w, 2, texels(6)),
                (0, 3, 5, 1, texels(6 + 2 * w)),
            ]
        );
        // Whole rows only, and nothing at all.
        assert_eq!(regions(width, 3 * width), [(0, 1, w, 2, 16)]);
        assert_eq!(regions(40, 40), []);
    }

    fn line(a: Vec2, c: Vec2) -> Curve {
        [a, (a + c) * 0.5, c]
    }

    /// A rectangle in ems, y down.
    fn rectangle(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<Curve> {
        let (a, b, c, d) = (
            Vec2::new(x0, y0),
            Vec2::new(x1, y0),
            Vec2::new(x1, y1),
            Vec2::new(x0, y1),
        );
        vec![line(a, b), line(b, c), line(c, d), line(d, a)]
    }

    #[test]
    fn a_record_files_each_curve_in_the_bands_it_crosses() {
        let curves = rectangle(0.0, -0.5, 1.0, 0.0);
        let bounds = [0.0, -0.5, 1.0, 0.0];
        let record = record(&curves, bounds, 10);
        let [rows, columns, count, _] = record[0];
        assert_eq!((rows, columns, count), (1.0, 1.0, 4.0));
        assert_eq!(record[1], bounds);
        // One band each way, every curve in it, the curves right after the two tables.
        assert_eq!(record[2], [14.0, 4.0, 0.0, 0.0]);
        assert_eq!(record[3], [22.0, 4.0, 0.0, 0.0]);
        assert_eq!(record.len(), 4 + 2 * 8);
        // Curves lie in the unit square of the bounds.
        assert!(
            record[4..]
                .iter()
                .all(|texel| texel.iter().all(|v| (0.0..=1.0).contains(v)))
        );
    }

    #[test]
    fn many_curves_split_into_bands() {
        // A comb: 20 thin vertical bars across the em.
        let curves: Vec<Curve> = (0..20)
            .flat_map(|bar| {
                let x = bar as f32 * 0.05;
                rectangle(x, -0.7, x + 0.02, 0.0)
            })
            .collect();
        let record = record(&curves, [0.0, -0.7, 0.97, 0.0], 0);
        assert_eq!(record[0][0], MAX_BANDS as f32);
        let bands = MAX_BANDS;
        // A vertical band holds the bars near it, not all eighty curves.
        let column = record[2 + bands];
        assert!(column[1] < 20.0, "{column:?}");
    }

    #[test]
    fn instances_turn_the_em_about_the_pen() {
        let glyph = StoredGlyph {
            offset: 7,
            bounds: [0.0, -0.7, 0.5, 0.0],
        };
        let upright = GlyphInstance::new(glyph, Vec2::new(10.0, 20.0), 16.0, 0.0, [1.0; 4]);
        assert_eq!(upright.origin_axis_x, [10.0, 20.0, 16.0, 0.0]);
        assert_eq!(&upright.axis_y_record[..3], &[0.0, 16.0, 7.0]);
        let quarter = GlyphInstance::new(
            glyph,
            Vec2::ZERO,
            16.0,
            core::f32::consts::FRAC_PI_2,
            [1.0; 4],
        );
        assert!(
            (quarter.origin_axis_x[3] - 16.0).abs() < 1e-4,
            "x axis points down"
        );
        assert!(
            (quarter.axis_y_record[0] + 16.0).abs() < 1e-4,
            "y axis points left"
        );
    }
}
