//! Icons packed for the GPU: one array of `vec4` the renderer's shader reads texel by texel from
//! a data texture. Offsets and counts are floats, exact below 2^24.
//!
//! - `data[0]`: the header, `[icon count, length of the array, 0, 0]`: the shader draws nothing
//!   for an icon number past the count.
//! - `data[1..=icons]`: an icon each, `[first layer, layer count, first zone, zone count]`.
//! - A layer: four `vec4`: `[first curve, curve count, rule (0 nonzero, 1 even-odd), kind
//!   (0 solid, 1 linear, 2 radial)]`, `[a, c, e, first stop]`, `[b, d, f, stop count]` (the
//!   gradient's transform `x' = a x + c y + e`, `y' = b x + d y + f`), and its bounds
//!   `[min x, min y, max x, max y]` in the icon's square.
//! - A stop: `[r, g, b, a]` (straight alpha, as the SVG gives it) and `[offset, zone, 0, 0]`.
//! - A curve: `[x0, y0, x1, y1]`, `[x2, y2, 0, 0]`, monotonic in y.
//! - A zone: its original color `[r, g, b, a]`.

use crate::model::{FillRule, Icon, Paint, Stop};

/// What the packed icons hold.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Packed {
    /// The array the shader reads.
    pub data: Vec<[f32; 4]>,
    /// How many icons it holds, after the header.
    pub icons: usize,
}

/// Where icon records start, after the header.
pub const FIRST_ICON: usize = 1;

/// Packs `icons`, in order: icon `i` is the `i`-th.
pub fn pack(icons: &[&Icon]) -> Packed {
    let mut data: Vec<[f32; 4]> = vec![[0.0; 4]; FIRST_ICON + icons.len()];
    for (index, icon) in icons.iter().enumerate() {
        let first_zone = data.len();
        for zone in &icon.zones {
            data.push(zone.color.0);
        }
        // Layers first, their curves and stops after them.
        let first_layer = data.len();
        let layer_records = icon.layers.len() * 4;
        data.resize(first_layer + layer_records, [0.0; 4]);
        for (number, layer) in icon.layers.iter().enumerate() {
            let first_curve = data.len();
            let (mut low, mut high) = ([f32::INFINITY; 2], [f32::NEG_INFINITY; 2]);
            for &[a, b, c] in &layer.curves {
                data.push([a.x, a.y, b.x, b.y]);
                data.push([c.x, c.y, 0.0, 0.0]);
                for point in [a, b, c] {
                    low = [low[0].min(point.x), low[1].min(point.y)];
                    high = [high[0].max(point.x), high[1].max(point.y)];
                }
            }
            let first_stop = data.len();
            let (kind, transform, stops): (f32, [f32; 6], &[Stop]) = match &layer.paint {
                Paint::Solid(stop) => (
                    0.0,
                    [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                    core::slice::from_ref(stop),
                ),
                Paint::Gradient {
                    radial,
                    to_gradient,
                    stops,
                } => (if *radial { 2.0 } else { 1.0 }, *to_gradient, stops),
            };
            for stop in stops {
                data.push(stop.color.0);
                data.push([stop.offset, f32::from(stop.zone), 0.0, 0.0]);
            }
            let [a, b, c, d, e, f] = transform;
            let rule = match layer.rule {
                FillRule::NonZero => 0.0,
                FillRule::EvenOdd => 1.0,
            };
            let record = first_layer + number * 4;
            data[record] = [first_curve as f32, layer.curves.len() as f32, rule, kind];
            data[record + 1] = [a, c, e, first_stop as f32];
            data[record + 2] = [b, d, f, stops.len() as f32];
            data[record + 3] = [low[0], low[1], high[0], high[1]];
        }
        data[FIRST_ICON + index] = [
            first_layer as f32,
            icon.layers.len() as f32,
            first_zone as f32,
            icon.zones.len() as f32,
        ];
    }
    data[0] = [icons.len() as f32, data.len() as f32, 0.0, 0.0];
    Packed {
        data,
        icons: icons.len(),
    }
}
