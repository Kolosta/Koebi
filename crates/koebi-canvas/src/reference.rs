//! The CPU reference of the canvas's coverage: what `shaders/canvas.frag` computes for a pixel,
//! in the same `f32` arithmetic in the same order, from the same records (`cells.rs`).
//! Tests render a view on the GPU and here, and compare (ADR 0029).
//!
//! Each pixel of a cell casts two rays from its center, one going right through its horizontal
//! band, one going down through its vertical band. A curve a ray crosses adds its weight,
//! blurred over a pixel by the distance from the center to the crossing; the two windings shade
//! under the fill rule and mix by how near each ray passes to a curve, as glyphs do.

use koebi_geom::FillRule;

/// What the shader receives about one cell: where its origin lies on the target and how large
/// it shows, and its record. A cell's own coordinates span 0 to 1 (`cells.rs`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellView {
    /// The cell's origin on the target, in physical pixels.
    pub origin: [f32; 2],
    /// Physical pixels per side of the cell: the zoom times the cell's size.
    pub scale: f32,
    /// Its inverse, computed in f64: sides of the cell per pixel.
    pub inverse: f32,
    /// Its margin, over its size.
    pub margin: f32,
    /// Where its record starts in the data.
    pub record: usize,
}

/// The value of a quadratic at `t`, as `quad_at` in curves.glsl.
fn quad_at(p0: f32, p1: f32, p2: f32, t: f32) -> f32 {
    let u = 1.0 - t;
    u * u * p0 + 2.0 * u * t * p1 + t * t * p2
}

/// Where a curve monotonic in one coordinate reaches `y`, as `monotonic_root` in curves.glsl.
#[allow(
    clippy::many_single_char_names,
    reason = "the names of curves.glsl, which this follows line by line"
)]
fn monotonic_root(y0: f32, y1: f32, y2: f32, y: f32) -> f32 {
    let a = y0 - 2.0 * y1 + y2;
    let b = 2.0 * (y1 - y0);
    let c = y0 - y;
    let t = if a.abs() < 1e-9 {
        if b.abs() < 1e-12 { 0.0 } else { -c / b }
    } else {
        let discriminant = (b * b - 4.0 * a * c).max(0.0).sqrt();
        let q = -0.5
            * (b + if b >= 0.0 {
                discriminant
            } else {
                -discriminant
            });
        let first = q / a;
        let second = if q.abs() > 1e-30 { c / q } else { first };
        if (-1e-4..=1.0001).contains(&first) {
            first
        } else {
            second
        }
    };
    t.clamp(0.0, 1.0)
}

fn spans(from: f32, to: f32, value: f32) -> bool {
    value >= from.min(to) && value < from.max(to)
}

/// The coverage a blurred winding number gives under `rule`, as `shade` in curves.glsl.
fn shade(winding: f32, rule: FillRule) -> f32 {
    match rule {
        FillRule::EvenOdd => 1.0 - (1.0 - winding.abs().rem_euclid(2.0)).abs(),
        FillRule::NonZero => winding.abs().min(1.0),
    }
}

/// The texel `index` of `data`, zero past it: the shader reads nothing past the store.
fn fetch(data: &[[f32; 4]], index: i64) -> [f32; 4] {
    usize::try_from(index)
        .ok()
        .and_then(|index| data.get(index))
        .copied()
        .unwrap_or([0.0; 4])
}

/// The coverage of the pixel whose center is `pixel` (physical pixels of the target) by the
/// cell `cell` of the records `data`, under `rule`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    reason = "band indices and counts held in f32 texels, and the shader's names"
)]
pub fn coverage(data: &[[f32; 4]], cell: CellView, pixel: [f32; 2], rule: FillRule) -> f32 {
    // As cells::MAX_BANDS and the shader's.
    const MAX_BANDS: i64 = 32;
    const MAX_BAND_CURVES: i64 = 1 << 16;
    let record = i64::try_from(cell.record).unwrap_or(i64::MAX);
    let head = fetch(data, record);
    let rows = (head[0] as i64).clamp(1, MAX_BANDS);
    let columns = (head[1] as i64).clamp(1, MAX_BANDS);
    let most = (head[2] as i64).clamp(0, MAX_BAND_CURVES);
    // The pixel's center in the cell's own coordinates, by a product as the shader's.
    let p = [
        (pixel[0] - cell.origin[0]) * cell.inverse,
        (pixel[1] - cell.origin[1]) * cell.inverse,
    ];
    let extent = 1.0 + 2.0 * cell.margin;
    let band = |coordinate: f32, count: i64| {
        (((coordinate + cell.margin) / extent * count as f32).floor() as i64).clamp(0, count - 1)
    };
    let (row, column) = (band(p[1], rows), band(p[0], columns));
    let ray = |table: i64, along: usize| {
        let other = 1 - along;
        let entry = fetch(data, table);
        // Counted from the record's start.
        let first = record + entry[0] as i64;
        let count = (entry[1] as i64).min(most);
        let (mut winding, mut weight) = (0.0_f32, 0.0_f32);
        for k in 0..count {
            let ab = fetch(data, first + 2 * k);
            let cw = fetch(data, first + 2 * k + 1);
            let (a, b, c) = ([ab[0], ab[1]], [ab[2], ab[3]], [cw[0], cw[1]]);
            if spans(a[other], c[other], p[other]) {
                let t = monotonic_root(a[other], b[other], c[other], p[other]);
                let offset = (quad_at(a[along], b[along], c[along], t) - p[along]) * cell.scale;
                // Going right, a curve going down counts; going down, one going left.
                let sign = if along == 0 {
                    if c[1] > a[1] { 1.0 } else { -1.0 }
                } else if c[0] < a[0] {
                    1.0
                } else {
                    -1.0
                };
                winding += cw[2] * sign * (offset + 0.5).clamp(0.0, 1.0);
                weight = weight.max((1.0 - offset.abs() * 2.0).clamp(0.0, 1.0));
            }
        }
        (shade(winding, rule), weight)
    };
    let (across, across_weight) = ray(record + 1 + row, 0);
    let (down, down_weight) = ray(record + 1 + rows + column, 1);
    let weights = across_weight + down_weight;
    let mixed = if weights > 1e-5 {
        (across * across_weight + down * down_weight) / weights
    } else {
        0.0
    };
    mixed.max(across.min(down)).clamp(0.0, 1.0)
}

/// `value` rounded to the nearest half-precision float, ties to even, as a store into an
/// `Rgba16Float` target rounds it.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "bit fields of an f32"
)]
fn half(value: f32) -> f32 {
    if !value.is_finite() || value == 0.0 {
        return value;
    }
    let bits = value.to_bits();
    let exponent = ((bits >> 23) & 0xff) as i32 - 127;
    // Halves keep 10 bits of mantissa from 2^-14 up, and fewer below, down to 2^-24.
    let kept = if exponent >= -14 {
        10
    } else {
        10 - (-14 - exponent)
    };
    if kept < 0 {
        return 0.0_f32.copysign(value);
    }
    let step = f64::from(exponent - kept).exp2();
    let scaled = f64::from(value) / step;
    let rounded = scaled.round_ties_even() * step;
    (rounded as f32).clamp(-65504.0, 65504.0)
}

/// Draws `instances` of a frame over `image`, its pixels row by row from the top left: every
/// pixel takes, instance after instance, those whose rectangle holds its center (as the GPU
/// rasterizes them), each covering it by [`coverage`] and drawn over what lies below.
fn draw(
    image: &mut [[f32; 4]],
    width: u32,
    instances: &[crate::frame::CellInstance],
    data: &[[f32; 4]],
) {
    for (index, pixel) in image.iter_mut().enumerate() {
        let index = u32::try_from(index).unwrap_or(u32::MAX);
        #[allow(clippy::cast_precision_loss, reason = "pixel coordinates of a view")]
        let center = [(index % width) as f32 + 0.5, (index / width) as f32 + 0.5];
        for instance in instances {
            let [x0, y0, x1, y1] = instance.rect;
            if center[0] < x0 || center[0] >= x1 || center[1] < y0 || center[1] >= y1 {
                continue;
            }
            let [ox, oy, scale, record] = instance.map;
            let rule = if instance.params[0] > 0.5 {
                FillRule::EvenOdd
            } else {
                FillRule::NonZero
            };
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a record index held in an f32"
            )]
            let cell = CellView {
                origin: [ox, oy],
                scale,
                inverse: instance.params[2],
                margin: instance.params[1],
                record: (record + 0.5) as usize,
            };
            let cover = coverage(data, cell, center, rule);
            let source = instance.color.map(|channel| channel * cover);
            *pixel =
                crate::blend::composite(koebi_doc::BlendMode::Normal, *pixel, source).map(half);
        }
    }
}

/// What [`crate::Canvas::render`] draws for `program` seen by `view`, computed on the CPU from
/// cells prepared anew: the view's pixels, row by row from the top left, in linear light,
/// premultiplied, each store rounded to half precision as the GPU's targets are. The frame's
/// script runs on a stack of images: layers drawn over the top one, each pixel covered by
/// [`coverage`], groups composited onto the one below by the W3C formulas the shader uses
/// (`blend.rs`). A view showing more than a canvas holds
/// draws what fits.
pub fn render(program: &koebi_doc::eval::Program, view: &crate::View) -> Vec<[f32; 4]> {
    use crate::frame::Op;
    let mut store = crate::store::CellStore::default();
    store.begin();
    let mut strokes = crate::strokes::ViewStrokes::default();
    strokes.begin();
    let mut frame = crate::frame::Frame::default();
    // What fits is drawn: the canvas fails instead, which tests see.
    let _ = frame.build(program, view, &mut store, &mut strokes);
    let data = store.data();
    let (width, height) = (view.extent.width, view.extent.height);
    let pixels = (width as usize) * (height as usize);
    let mut stack = vec![vec![[0.0_f32; 4]; pixels]];
    for &op in &frame.script {
        match op {
            Op::Draw(layer) => {
                let range = frame.ranges.get(layer).cloned().unwrap_or(0..0);
                let instances = &frame.instances[range.start as usize..range.end as usize];
                if let Some(image) = stack.last_mut() {
                    draw(image, width, instances, data);
                }
            }
            Op::Push => stack.push(vec![[0.0; 4]; pixels]),
            Op::Pop { mode, opacity } => {
                let (Some(source), Some(below)) = (stack.pop(), stack.last_mut()) else {
                    break;
                };
                for (backdrop, source) in below.iter_mut().zip(source) {
                    let source = source.map(|channel| channel * opacity);
                    *backdrop = crate::blend::composite(mode, *backdrop, source).map(half);
                }
            }
            Op::PopMask { invert } => {
                let (Some(mask), Some(content)) = (stack.pop(), stack.pop()) else {
                    break;
                };
                let Some(below) = stack.last_mut() else {
                    break;
                };
                for ((backdrop, content), mask) in below.iter_mut().zip(content).zip(mask) {
                    let shown = if invert { 1.0 - mask[3] } else { mask[3] };
                    let source = content.map(|channel| channel * shown);
                    *backdrop =
                        crate::blend::composite(koebi_doc::BlendMode::Normal, *backdrop, source)
                            .map(half);
                }
            }
        }
    }
    stack.swap_remove(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_round_to_half_precision() {
        assert_eq!(half(1.0), 1.0);
        // 1 + 2^-11 lies halfway between two halves: to the even one.
        assert_eq!(half(1.0 + 2.0_f32.powi(-11)), 1.0);
        assert_eq!(
            half(1.0 + 3.0 * 2.0_f32.powi(-11)),
            1.0 + 2.0 * 2.0_f32.powi(-10)
        );
        assert!((half(0.1) - 0.099_975_586).abs() < 1e-9);
        // Below 2^-14 the steps stay 2^-24; far below, zero.
        assert_eq!(half(3.0 * 2.0_f32.powi(-24)), 3.0 * 2.0_f32.powi(-24));
        assert_eq!(half(2.0_f32.powi(-30)), 0.0);
        assert_eq!(half(1.0e6), 65504.0);
    }
}
