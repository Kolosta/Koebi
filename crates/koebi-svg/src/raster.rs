//! Coverage of an icon's layers, exactly as the renderer's shader computes it, on the CPU: for
//! cursors, the application's icon and its platform files, and to check the shader against.
//!
//! A pixel's coverage comes from two rays leaving its center, one going right and one going
//! down. Every curve is monotonic in x and in y (the compiler splits them at their turning
//! points), so it crosses each ray at most once, counted over the half-open span of its
//! coordinates so that the end two curves share counts once. A crossing counts as the share of
//! the pixel on the near side of it (a box filter one pixel wide, signed by the curve's
//! direction): the sum along a ray is the winding number blurred across the pixel, which the fill
//! rule turns into a coverage (nonzero: clamped to 1, so that overlapping pieces show no seam;
//! even-odd: folded). The two rays' coverages are mixed by how close their crossings are, the
//! ray crossing an edge inside the pixel deciding (after the Slug algorithm, Lengyel 2017). Layers
//! are painted in order, over each other.

use crate::color::Rgba;
use crate::geometry::Point;
use crate::model::{FillRule, Icon, Layer, Paint, Stop};

/// How each zone is colored: its original color, or the colors given.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Colors<'a> {
    /// As the SVG gives them.
    Original,
    /// Zone by zone: a zone without a color keeps its own.
    Zones(&'a [Rgba]),
}

/// The winding number of `curves` around `p`.
pub fn winding(curves: &[[Point; 3]], p: Point) -> i32 {
    let mut winding = 0;
    for &[a, b, c] in curves {
        if !spans(a.y, c.y, p.y) {
            continue;
        }
        let Some(t) = monotonic_root(a.y, b.y, c.y, p.y) else {
            continue;
        };
        let x = quad_at(a.x, b.x, c.x, t);
        if x > p.x {
            winding += if c.y > a.y { 1 } else { -1 };
        }
    }
    winding
}

/// Whether `value` lies in the half-open span from `from` to `to`, whichever is lower.
fn spans(from: f32, to: f32, value: f32) -> bool {
    let (low, high) = if from <= to { (from, to) } else { (to, from) };
    value >= low && value < high
}

/// The parameter where a curve monotonic in one coordinate, from `y0` to `y2` through the
/// control `y1`, reaches `y`.
pub(crate) fn monotonic_root(y0: f32, y1: f32, y2: f32, y: f32) -> Option<f32> {
    let a = y0 - 2.0 * y1 + y2;
    let b = 2.0 * (y1 - y0);
    let c = y0 - y;
    let t = if a.abs() < 1e-9 {
        if b.abs() < 1e-12 {
            return None;
        }
        -c / b
    } else {
        // The two roots in the form that cancels nothing (Numerical Recipes 5.6): a nearly
        // straight curve has a tiny `a`.
        let discriminant = (b * b - 4.0 * a * c).max(0.0).sqrt();
        let q = -0.5 * (b + discriminant.copysign(b));
        let first = q / a;
        let second = if q.abs() > 1e-30 { c / q } else { first };
        // The root inside the curve: of the two, the one in 0..=1.
        if (-1e-4..=1.0001).contains(&first) {
            first
        } else {
            second
        }
    };
    Some(t.clamp(0.0, 1.0))
}

fn quad_at(p0: f32, p1: f32, p2: f32, t: f32) -> f32 {
    let u = 1.0 - t;
    u * u * p0 + 2.0 * u * t * p1 + t * t * p2
}

/// The coverage of `layer` for the pixel centered on `p` (in the icon's unit square), its curves
/// `scale` pixels per unit.
pub fn coverage(layer: &Layer, p: Point, scale: f32) -> f32 {
    // The blurred winding along each ray, and how close to the center its nearest crossing is
    // (1 on it, 0 half a pixel away or more).
    let (mut across, mut across_weight) = (0.0_f32, 0.0_f32);
    let (mut down, mut down_weight) = (0.0_f32, 0.0_f32);
    for &[a, b, c] in &layer.curves {
        if spans(a.y, c.y, p.y)
            && let Some(t) = monotonic_root(a.y, b.y, c.y, p.y)
        {
            let offset = (quad_at(a.x, b.x, c.x, t) - p.x) * scale;
            let sign = if c.y > a.y { 1.0 } else { -1.0 };
            across += sign * (offset + 0.5).clamp(0.0, 1.0);
            across_weight = across_weight.max((1.0 - offset.abs() * 2.0).clamp(0.0, 1.0));
        }
        if spans(a.x, c.x, p.x)
            && let Some(t) = monotonic_root(a.x, b.x, c.x, p.x)
        {
            let offset = (quad_at(a.y, b.y, c.y, t) - p.y) * scale;
            // A curve going left crosses the downward ray as one going down crosses the
            // rightward one: both count a shape drawn clockwise on screen positively.
            let sign = if c.x < a.x { 1.0 } else { -1.0 };
            down += sign * (offset + 0.5).clamp(0.0, 1.0);
            down_weight = down_weight.max((1.0 - offset.abs() * 2.0).clamp(0.0, 1.0));
        }
    }
    let (across, down) = (shade(across, layer.rule), shade(down, layer.rule));
    let weights = across_weight + down_weight;
    let mixed = if weights > 1e-5 {
        (across * across_weight + down * down_weight) / weights
    } else {
        0.0
    };
    mixed.max(across.min(down)).clamp(0.0, 1.0)
}

/// The coverage a blurred winding number gives under `rule`.
fn shade(winding: f32, rule: FillRule) -> f32 {
    match rule {
        FillRule::NonZero => winding.abs().min(1.0),
        // Inside at odd numbers, outside at even ones, linear between.
        FillRule::EvenOdd => 1.0 - (1.0 - winding.abs() % 2.0).abs(),
    }
}

/// The color of `paint` at `p`, straight alpha.
pub fn paint_at(paint: &Paint, p: Point, colors: Colors<'_>) -> Rgba {
    let color_of = |stop: &Stop| match colors {
        Colors::Original => stop.color,
        Colors::Zones(zones) => zones
            .get(usize::from(stop.zone))
            .map_or(stop.color, |zone| {
                let [r, g, b, a] = zone.0;
                Rgba([r, g, b, a * stop.color.0[3]])
            }),
    };
    match paint {
        Paint::Solid(stop) => color_of(stop),
        Paint::Gradient {
            radial,
            to_gradient,
            stops,
        } => {
            let [a, b, c, d, e, f] = *to_gradient;
            let q = Point::new(a * p.x + c * p.y + e, b * p.x + d * p.y + f);
            let t = if *radial { q.length() } else { q.x }.clamp(0.0, 1.0);
            let Some(first) = stops.first() else {
                return Rgba([0.0; 4]);
            };
            if t <= first.offset {
                return color_of(first);
            }
            for pair in stops.windows(2) {
                let (from, to) = (&pair[0], &pair[1]);
                if t <= to.offset {
                    let span = (to.offset - from.offset).max(1e-6);
                    let share = ((t - from.offset) / span).clamp(0.0, 1.0);
                    let (x, y) = (color_of(from).0, color_of(to).0);
                    return Rgba(core::array::from_fn(|i| x[i] + (y[i] - x[i]) * share));
                }
            }
            stops.last().map_or(Rgba([0.0; 4]), color_of)
        }
    }
}

/// Renders `icon` into a square of `size` pixels, straight-alpha RGBA bytes row by row from the
/// top left, each pixel sampled at its center.
pub fn render(icon: &Icon, size: u32, colors: Colors<'_>) -> Vec<u8> {
    let mut out = vec![0_u8; (size as usize) * (size as usize) * 4];
    let scale = size as f32;
    for row in 0..size {
        for column in 0..size {
            let p = Point::new((column as f32 + 0.5) / scale, (row as f32 + 0.5) / scale);
            // Premultiplied, painted over each other.
            let mut pixel = [0.0_f32; 4];
            for layer in &icon.layers {
                let cover = coverage(layer, p, scale);
                if cover <= 0.0 {
                    continue;
                }
                let [r, g, b, a] = paint_at(&layer.paint, p, colors).0;
                let alpha = a * cover;
                pixel = [
                    r * alpha + pixel[0] * (1.0 - alpha),
                    g * alpha + pixel[1] * (1.0 - alpha),
                    b * alpha + pixel[2] * (1.0 - alpha),
                    alpha + pixel[3] * (1.0 - alpha),
                ];
            }
            let at = ((row * size + column) * 4) as usize;
            let alpha = pixel[3];
            let straight = |channel: f32| {
                if alpha > 1e-6 { channel / alpha } else { 0.0 }
            };
            let bytes = Rgba([
                straight(pixel[0]),
                straight(pixel[1]),
                straight(pixel[2]),
                alpha,
            ])
            .to_rgba8();
            out[at..at + 4].copy_from_slice(&bytes);
        }
    }
    out
}
