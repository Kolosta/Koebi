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

/// The most `vec4` a packed set may hold: offsets stay exact as floats.
pub const MAX_DATA: usize = 1 << 24;

/// The most curves one icon may hold: a pixel's cost stays bounded.
pub const MAX_ICON_CURVES: usize = 65_536;

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

impl Packed {
    /// Checks that every offset and count of the set stays inside it: what the renderer's shader
    /// reads may then never leave the data, and no icon holds more than [`MAX_ICON_CURVES`].
    /// [`pack`] gives valid sets; a set built by hand may not be.
    ///
    /// # Errors
    ///
    /// Describes the first record that does not hold.
    pub fn validate(&self) -> Result<(), String> {
        let len = self.data.len();
        if len > MAX_DATA {
            return Err(format!("{len} texels, more than {MAX_DATA}"));
        }
        // A whole number from 0, an offset or a count.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a whole number from 0, below 2^24 where it matters"
        )]
        let whole = |value: f32| -> Option<usize> {
            (value.is_finite() && value >= 0.0 && value.fract() == 0.0).then_some(value as usize)
        };
        let header = self.data.first().ok_or("no header")?;
        if whole(header[0]) != Some(self.icons) || whole(header[1]) != Some(len) {
            return Err(format!(
                "the header gives {} icons and {} texels, the set {} and {len}",
                header[0], header[1], self.icons
            ));
        }
        // `count` records of `size` texels from `first`, inside the data.
        let span = |first: usize, count: usize, size: usize| -> bool {
            count
                .checked_mul(size)
                .and_then(|texels| first.checked_add(texels))
                .is_some_and(|end| end <= len)
        };
        for icon in 0..self.icons {
            let record = self
                .data
                .get(FIRST_ICON + icon)
                .ok_or_else(|| format!("icon {icon} has no record"))?;
            let fields = record.map(whole);
            let [
                Some(first_layer),
                Some(layers),
                Some(first_zone),
                Some(zones),
            ] = fields
            else {
                return Err(format!("icon {icon}: a record field is not a whole number"));
            };
            if !span(first_layer, layers, 4) || !span(first_zone, zones, 1) {
                return Err(format!("icon {icon}: its layers or zones leave the set"));
            }
            let mut curves = 0_usize;
            for layer in 0..layers {
                let at = first_layer + 4 * layer;
                let ([first_curve, count, rule, kind], [.., first_stop], [.., stops]) =
                    (self.data[at], self.data[at + 1], self.data[at + 2]);
                let (Some(first_curve), Some(count), Some(first_stop), Some(stops)) = (
                    whole(first_curve),
                    whole(count),
                    whole(first_stop),
                    whole(stops),
                ) else {
                    return Err(format!("icon {icon}, layer {layer}: not a whole number"));
                };
                let known =
                    matches!(whole(rule), Some(0 | 1)) && matches!(whole(kind), Some(0..=2));
                if !known
                    || stops == 0
                    || !span(first_curve, count, 2)
                    || !span(first_stop, stops, 2)
                {
                    return Err(format!("icon {icon}, layer {layer}: out of the set"));
                }
                curves += count;
            }
            if curves > MAX_ICON_CURVES {
                return Err(format!(
                    "icon {icon}: {curves} curves, more than {MAX_ICON_CURVES}"
                ));
            }
        }
        Ok(())
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile;

    /// A change that breaks a set, given the offset of its first layer.
    type Corruption = fn(&mut Packed, usize);

    fn set() -> Packed {
        let square =
            compile(r##"<svg viewBox="0 0 2 2"><rect width="1" height="1" fill="#f00"/></svg>"##)
                .unwrap();
        let shade = compile(
            r##"<svg viewBox="0 0 2 2"><defs><linearGradient id="g"><stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient></defs><circle cx="1" cy="1" r="1" fill="url(#g)"/></svg>"##,
        )
        .unwrap();
        pack(&[&square, &shade])
    }

    #[test]
    fn packed_sets_are_valid() {
        let set = set();
        assert_eq!(set.validate(), Ok(()));
        assert_eq!(set.data[0], [2.0, set.data.len() as f32, 0.0, 0.0]);
        assert_eq!(pack(&[]).validate(), Ok(()));
    }

    #[test]
    fn sets_whose_records_leave_them_are_refused() {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the first layer's offset, a small whole number"
        )]
        let layer = set().data[FIRST_ICON][0] as usize;
        let corruptions: [(&str, Corruption); 8] = [
            ("header", |set, _| set.icons = 3),
            ("length", |set, _| set.data[0][1] += 1.0),
            ("layers past the end", |set, _| {
                set.data[FIRST_ICON][1] = 1e6;
            }),
            ("negative offset", |set, _| set.data[FIRST_ICON][0] = -4.0),
            ("fraction", |set, layer| set.data[layer][1] = 1.5),
            ("curves past the end", |set, layer| set.data[layer][0] = 1e7),
            ("unknown paint", |set, layer| set.data[layer][3] = 7.0),
            ("no stop", |set, layer| set.data[layer + 2][3] = 0.0),
        ];
        for (what, corrupt) in corruptions {
            let mut set = set();
            corrupt(&mut set, layer);
            assert!(set.validate().is_err(), "{what}");
        }
        let mut nan = set();
        nan.data[layer][0] = f32::NAN;
        assert!(nan.validate().is_err());
    }
}
