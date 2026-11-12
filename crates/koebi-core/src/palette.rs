//! Color palettes ordered by contrast with the background they are made for.
//!
//! A palette step is not a fixed color but a contrast: step 900 of a hue is the color of that
//! hue with 4.81:1 contrast against the theme's background, whatever the theme. On a light
//! background it is a dark color, on a dark background a light one, and every token aliasing
//! step 900 keeps its legibility from one theme to the next. This is how Adobe Spectrum's
//! palettes are built (with its Leonardo tool); [`HUE_CONTRASTS`] are the ratios Spectrum's light
//! palettes have against white, measured.
//!
//! A [`ColorScale`] is the path a hue takes from black to white through key colors, in OKLCH:
//! lightness from 0 to 1, chroma and hue interpolated between the keys, out-of-gamut colors
//! brought into sRGB by reducing chroma. [`ColorScale::with_contrast`] finds the color of the
//! scale at a contrast ratio by bisection on lightness, on the side away from the background.
//!
//! ```
//! use koebi_core::Color;
//! use koebi_core::palette::{ColorScale, HUE_CONTRASTS};
//!
//! let blue = ColorScale::from_seed(Color::from_hex(0x3B_63_FB));
//! let dark = Color::from_hex(0x13_13_13);
//! let steps = blue.steps(dark, &HUE_CONTRASTS);
//! // Step 900 reaches its 4.81:1 on the dark background too.
//! let ratio = steps[8].contrast_ratio(dark);
//! assert!((ratio - 4.81).abs() < 0.05, "{ratio}");
//! ```

use crate::color::{Color, Oklch};

/// The steps of a hue palette, from the closest to the background to the farthest.
pub const HUE_STEPS: [u16; 16] = [
    100, 200, 300, 400, 500, 600, 700, 800, 900, 1000, 1100, 1200, 1300, 1400, 1500, 1600,
];

/// The contrast of each step of [`HUE_STEPS`] with the background: Spectrum's light palettes
/// against white, every hue within 1 % of these. Step 700 is the first above 3:1 (large text,
/// boundaries of controls), 900 the first above 4.5:1 (text), 1100 the first above 7:1.
pub const HUE_CONTRASTS: [f32; 16] = [
    1.06, 1.15, 1.33, 1.61, 2.0, 2.61, 3.23, 3.96, 4.81, 6.31, 7.94, 10.04, 12.36, 14.85, 17.04,
    19.53,
];

/// Bisection steps: lightness to about 1e-12, far below a step of 8-bit color.
const ITERATIONS: u32 = 40;

/// A hue's path from black to white through key colors, in OKLCH.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorScale {
    /// The keys by increasing lightness, black and white at the ends, hues unwrapped so that
    /// interpolation takes the short way round.
    keys: Vec<Oklch>,
}

impl ColorScale {
    /// The scale through `keys`, in any order. Below the darkest key the chroma falls to black,
    /// above the lightest to white, keeping the hue of the key. `None` without keys.
    pub fn new(keys: &[Color]) -> Option<Self> {
        let mut points: Vec<Oklch> = keys
            .iter()
            .map(|key| {
                let lch = key.with_alpha(1.0).to_oklch();
                Oklch::new(lch.l.clamp(0.0, 1.0), lch.c, lch.h)
            })
            .collect();
        if points.is_empty() {
            return None;
        }
        points.sort_by(|a, b| a.l.total_cmp(&b.l));
        // Hues of grays are meaningless: take their neighbour's, so that a gray key does not
        // turn the scale around the hue circle.
        let hue_of_color = points.iter().find(|p| p.c > 0.02).map(|p| p.h);
        for point in &mut points {
            if point.c <= 0.02 {
                point.h = hue_of_color.unwrap_or(point.h);
            }
        }
        for index in 1..points.len() {
            let previous = points[index - 1].h;
            let mut hue = points[index].h;
            while hue - previous > 180.0 {
                hue -= 360.0;
            }
            while previous - hue > 180.0 {
                hue += 360.0;
            }
            points[index].h = hue;
        }
        let (first, last) = (points[0], points[points.len() - 1]);
        let mut keys = Vec::with_capacity(points.len() + 2);
        if first.l > 0.0 {
            keys.push(Oklch::new(0.0, 0.0, first.h));
        }
        keys.extend(points);
        if last.l < 1.0 {
            keys.push(Oklch::new(1.0, 0.0, last.h));
        }
        Some(Self { keys })
    }

    /// The scale of one color: from black through `seed` to white, the seed's hue and chroma at
    /// its lightness. What a palette made from a single chosen color follows.
    pub fn from_seed(seed: Color) -> Self {
        Self::new(&[seed]).unwrap_or_else(|| Self {
            keys: vec![Oklch::new(0.0, 0.0, 0.0), Oklch::new(1.0, 0.0, 0.0)],
        })
    }

    /// The color of the scale at OKLCH lightness `l` (clamped to `0..=1`), opaque.
    pub fn at(&self, l: f32) -> Color {
        let l = l.clamp(0.0, 1.0);
        let keys = &self.keys;
        let upper = keys
            .iter()
            .position(|key| key.l >= l)
            .unwrap_or(keys.len() - 1);
        let lch = if upper == 0 {
            keys[0]
        } else {
            let (a, b) = (keys[upper - 1], keys[upper]);
            let span = b.l - a.l;
            let t = if span > f32::EPSILON {
                (l - a.l) / span
            } else {
                0.0
            };
            Oklch::new(l, a.c + (b.c - a.c) * t, a.h + (b.h - a.h) * t)
        };
        Oklch::new(l, lch.c, lch.h.rem_euclid(360.0)).to_color()
    }

    /// The color of the scale with contrast `ratio` against `background`, on the side away from
    /// it: lighter on a dark background, darker on a light one. A ratio out of reach on that
    /// side gives the end of the scale (white or black); a ratio of 1 or less gives the color at
    /// the background's lightness.
    pub fn with_contrast(&self, background: Color, ratio: f32) -> Color {
        let below = background.with_alpha(1.0).to_oklch().l;
        // The background's side of the contrast, once ([`Color::contrast_ratio`]).
        let luminance = background.relative_luminance();
        let contrast = |color: Color| {
            let other = color.relative_luminance();
            let (lighter, darker) = if other >= luminance {
                (other, luminance)
            } else {
                (luminance, other)
            };
            (lighter + 0.05) / (darker + 0.05)
        };
        // A background darker than middle gray takes lighter colors; the other side otherwise.
        let lighter = luminance < 0.18;
        let (mut near, mut far) = if lighter { (below, 1.0) } else { (below, 0.0) };
        if contrast(self.at(far)) <= ratio {
            return self.at(far);
        }
        for _ in 0..ITERATIONS {
            let middle = near.midpoint(far);
            // No other `f32` lies between the ends: past this step, nothing changes.
            #[allow(
                clippy::float_cmp,
                reason = "exact on purpose: the midpoint is one of the ends once no other f32                           lies between them"
            )]
            let last = middle == near || middle == far;
            if contrast(self.at(middle)) < ratio {
                near = middle;
            } else {
                far = middle;
            }
            if last {
                break;
            }
        }
        self.at(far)
    }

    /// One color per ratio of `ratios` (see [`Self::with_contrast`]).
    pub fn steps<const N: usize>(&self, background: Color, ratios: &[f32; N]) -> [Color; N] {
        ratios.map(|ratio| self.with_contrast(background, ratio))
    }
}

/// The step of `ratios` whose contrast with `background` is closest to `color`'s, by the ratio
/// of ratios (a 10 % miss weighs the same at 1.1:1 and at 11:1).
pub fn nearest_step(color: Color, background: Color, ratios: &[f32]) -> Option<usize> {
    let ratio = color.contrast_ratio(background).ln();
    ratios
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (a.ln() - ratio).abs().total_cmp(&(b.ln() - ratio).abs()))
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spectrum's light blue palette, steps 100 to 1600: the values Koebi's primitives held.
    const SPECTRUM_BLUE: [u32; 16] = [
        0xF5_F9_FF, 0xE5_F0_FE, 0xCB_E2_FE, 0xAC_CF_FD, 0x8E_B9_FC, 0x72_9E_FD, 0x5D_89_FF,
        0x4B_75_FF, 0x3B_63_FB, 0x27_4D_EA, 0x1D_3E_CF, 0x15_32_AD, 0x10_28_8C, 0x0C_1F_69,
        0x0E_18_43, 0x07_0B_1E,
    ];

    fn close(a: Color, b: Color, tolerance: u8) -> bool {
        a.to_rgba8()
            .iter()
            .zip(b.to_rgba8())
            .all(|(x, y)| x.abs_diff(y) <= tolerance)
    }

    #[test]
    fn a_step_reaches_its_ratio_on_either_side() {
        let scale = ColorScale::from_seed(Color::from_hex(0x3B_63_FB));
        for background in [
            Color::WHITE,
            Color::from_hex(0x13_13_13),
            Color::from_hex(0xF8_F8_F8),
        ] {
            let max = Color::WHITE
                .contrast_ratio(background)
                .max(Color::BLACK.contrast_ratio(background));
            for ratio in HUE_CONTRASTS.into_iter().filter(|&r| r < max * 0.97) {
                let color = scale.with_contrast(background, ratio);
                let got = color.contrast_ratio(background);
                assert!(
                    (got - ratio).abs() / ratio < 0.01,
                    "{background}: {ratio} gave {got}"
                );
            }
        }
    }

    #[test]
    fn steps_move_away_from_the_background() {
        let scale = ColorScale::from_seed(Color::from_hex(0xD7_7B_6B));
        let light = scale.steps(Color::WHITE, &HUE_CONTRASTS);
        let dark = scale.steps(Color::from_hex(0x13_13_13), &HUE_CONTRASTS);
        for pair in light.windows(2) {
            assert!(pair[1].relative_luminance() <= pair[0].relative_luminance());
        }
        for pair in dark.windows(2) {
            assert!(pair[1].relative_luminance() >= pair[0].relative_luminance());
        }
        // Out of reach on a dark background: the last step is the white end of the scale.
        assert!(dark[15].relative_luminance() > 0.97, "{}", dark[15]);
    }

    #[test]
    fn spectrum_s_light_palette_comes_back_from_its_keys_and_ratios() {
        let keys = SPECTRUM_BLUE.map(Color::from_hex);
        let scale = ColorScale::new(&keys).unwrap();
        let steps = scale.steps(Color::WHITE, &HUE_CONTRASTS);
        for (index, (step, key)) in steps.iter().zip(keys).enumerate() {
            assert!(
                close(*step, key, 6),
                "step {}: {step} for {key}",
                HUE_STEPS[index]
            );
        }
    }

    #[test]
    fn a_seed_lies_on_its_own_scale() {
        for seed in [0x3B_63_FB, 0xD7_7B_6B, 0x64_78_8C, 0xDB_D4_AD, 0x13_9E_5C] {
            let seed = Color::from_hex(seed);
            let scale = ColorScale::from_seed(seed);
            assert!(close(scale.at(seed.to_oklch().l), seed, 2), "{seed}");
        }
    }

    #[test]
    fn a_gray_key_makes_a_gray_scale() {
        let scale = ColorScale::from_seed(Color::from_hex(0x80_80_80));
        for l in [0.1, 0.4, 0.7, 0.95] {
            let [r, g, b, _] = scale.at(l).to_rgba8();
            assert!(r.abs_diff(g) <= 1 && g.abs_diff(b) <= 1, "{r} {g} {b}");
        }
        assert!(ColorScale::new(&[]).is_none());
    }

    #[test]
    fn the_nearest_step_compares_ratios() {
        let white = Color::WHITE;
        assert_eq!(
            nearest_step(Color::from_hex(0x3B_63_FB), white, &HUE_CONTRASTS),
            Some(8)
        );
        assert_eq!(nearest_step(white, white, &HUE_CONTRASTS), Some(0));
        assert_eq!(nearest_step(white, white, &[]), None);
    }
}
