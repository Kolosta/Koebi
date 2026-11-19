//! Compositing a source onto a backdrop (ADR 0029): the W3C *Compositing and Blending Level 1*
//! formulas, the twelve separable modes and the four non-separable ones, and Erase, on linear,
//! premultiplied colors. `shaders/composite.frag` computes the same in the same order; the CPU
//! reference ([`crate::reference`]) calls these.
//!
//! A source of premultiplied color `cs` and alpha `as`, scaled by its opacity and its mask,
//! composites onto a backdrop `cb`, `ab` by source-over with blending:
//! `co = cs·(1 − ab) + cb·(1 − as) + as·ab·B(Cb, Cs)` and `ao = as + ab·(1 − as)`, where `Cs` and
//! `Cb` are the colors unpremultiplied and `B` the mode's blend function. Erase is
//! destination-out: `co = cb·(1 − as)`, `ao = ab·(1 − as)`.

#![allow(
    clippy::many_single_char_names,
    reason = "the W3C specification's names, which this follows formula by formula"
)]

use koebi_doc::BlendMode;

/// The luminosity of a color by the W3C's weights.
fn lum(c: [f32; 3]) -> f32 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

/// `c` brought back within 0 to 1 at its luminosity.
fn clip_color(c: [f32; 3]) -> [f32; 3] {
    let l = lum(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut c = c;
    if n < 0.0 {
        let d = l - n;
        c = c.map(|channel| {
            if d > 0.0 {
                l + (channel - l) * l / d
            } else {
                l
            }
        });
    }
    if x > 1.0 {
        let d = x - l;
        c = c.map(|channel| {
            if d > 0.0 {
                l + (channel - l) * (1.0 - l) / d
            } else {
                l
            }
        });
    }
    c
}

/// `c` at luminosity `l`.
fn set_lum(c: [f32; 3], l: f32) -> [f32; 3] {
    let d = l - lum(c);
    clip_color(c.map(|channel| channel + d))
}

/// The saturation of `c`: its largest channel less its smallest.
fn sat(c: [f32; 3]) -> f32 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

/// `c` at saturation `s`, its hue kept.
fn set_sat(c: [f32; 3], s: f32) -> [f32; 3] {
    let (x, n) = (c[0].max(c[1]).max(c[2]), c[0].min(c[1]).min(c[2]));
    if x > n {
        c.map(|channel| (channel - n) * s / (x - n))
    } else {
        [0.0; 3]
    }
}

/// The separable blend function of mode `mode` for one channel: backdrop `b`, source `s`.
fn separable(mode: BlendMode, b: f32, s: f32) -> f32 {
    let multiply = |b: f32, s: f32| b * s;
    let screen = |b: f32, s: f32| b + s - b * s;
    let hard_light = |b: f32, s: f32| {
        if s <= 0.5 {
            multiply(b, 2.0 * s)
        } else {
            screen(b, 2.0 * s - 1.0)
        }
    };
    match mode {
        BlendMode::Multiply => multiply(b, s),
        BlendMode::Screen => screen(b, s),
        BlendMode::Overlay => hard_light(s, b),
        BlendMode::Darken => b.min(s),
        BlendMode::Lighten => b.max(s),
        BlendMode::ColorDodge => {
            if b <= 0.0 {
                0.0
            } else if s >= 1.0 {
                1.0
            } else {
                (b / (1.0 - s)).min(1.0)
            }
        }
        BlendMode::ColorBurn => {
            if b >= 1.0 {
                1.0
            } else if s <= 0.0 {
                0.0
            } else {
                1.0 - ((1.0 - b) / s).min(1.0)
            }
        }
        BlendMode::HardLight => hard_light(b, s),
        BlendMode::SoftLight => {
            if s <= 0.5 {
                b - (1.0 - 2.0 * s) * b * (1.0 - b)
            } else {
                let d = if b <= 0.25 {
                    ((16.0 * b - 12.0) * b + 4.0) * b
                } else {
                    b.sqrt()
                };
                b + (2.0 * s - 1.0) * (d - b)
            }
        }
        BlendMode::Difference => (b - s).abs(),
        BlendMode::Exclusion => b + s - 2.0 * b * s,
        _ => s,
    }
}

/// The blend function of `mode`: what the source's color becomes over the backdrop's, both
/// unpremultiplied.
pub(crate) fn blended(mode: BlendMode, b: [f32; 3], s: [f32; 3]) -> [f32; 3] {
    match mode {
        BlendMode::Hue => set_lum(set_sat(s, sat(b)), lum(b)),
        BlendMode::Saturation => set_lum(set_sat(b, sat(s)), lum(b)),
        BlendMode::Color => set_lum(s, lum(b)),
        BlendMode::Luminosity => set_lum(b, lum(s)),
        _ => [0, 1, 2].map(|channel| separable(mode, b[channel], s[channel])),
    }
}

/// `source` (premultiplied, already scaled by its opacity and mask) composited onto `backdrop`
/// (premultiplied) in `mode`.
pub(crate) fn composite(mode: BlendMode, backdrop: [f32; 4], source: [f32; 4]) -> [f32; 4] {
    let (ab, a_s) = (backdrop[3], source[3]);
    match mode {
        BlendMode::Erase => backdrop.map(|channel| channel * (1.0 - a_s)),
        BlendMode::Normal => {
            let keep = 1.0 - a_s;
            [0, 1, 2, 3].map(|channel| source[channel] + backdrop[channel] * keep)
        }
        _ => {
            let unpremultiplied = |c: [f32; 4]| {
                if c[3] > 0.0 {
                    [c[0] / c[3], c[1] / c[3], c[2] / c[3]]
                } else {
                    [0.0; 3]
                }
            };
            let mix = blended(mode, unpremultiplied(backdrop), unpremultiplied(source));
            let both = a_s * ab;
            let mut out = [0.0; 4];
            for channel in 0..3 {
                out[channel] = source[channel] * (1.0 - ab)
                    + backdrop[channel] * (1.0 - a_s)
                    + both * mix[channel];
            }
            out[3] = a_s + ab * (1.0 - a_s);
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_follow_the_w3c_formulas_at_their_turning_points() {
        // Separable modes on single channels.
        assert!((separable(BlendMode::Multiply, 0.5, 0.4) - 0.2).abs() < 1e-6);
        assert!((separable(BlendMode::Screen, 0.5, 0.4) - 0.7).abs() < 1e-6);
        assert_eq!(separable(BlendMode::ColorDodge, 0.0, 0.9), 0.0);
        assert_eq!(separable(BlendMode::ColorDodge, 0.3, 1.0), 1.0);
        assert_eq!(separable(BlendMode::ColorBurn, 1.0, 0.0), 1.0);
        assert_eq!(separable(BlendMode::ColorBurn, 0.3, 0.0), 0.0);
        // Overlay is hard light with the layers swapped.
        assert!(
            (separable(BlendMode::Overlay, 0.3, 0.8) - separable(BlendMode::HardLight, 0.8, 0.3))
                .abs()
                < 1e-6
        );
        // Soft light's two pieces meet at a backdrop of a quarter.
        let (low, high) = (
            separable(BlendMode::SoftLight, 0.249_99, 0.9),
            separable(BlendMode::SoftLight, 0.250_01, 0.9),
        );
        assert!((low - high).abs() < 1e-3);
        // Non-separable: luminosity keeps the backdrop's hue and the source's lightness.
        let out = blended(BlendMode::Luminosity, [0.8, 0.2, 0.2], [0.5, 0.5, 0.5]);
        assert!((lum(out) - 0.5).abs() < 1e-5 && out[0] > out[1]);
        let gray = blended(BlendMode::Saturation, [0.4, 0.6, 0.2], [0.5, 0.5, 0.5]);
        assert!(
            (gray[0] - gray[1]).abs() < 1e-6 && (lum(gray) - lum([0.4, 0.6, 0.2])).abs() < 1e-5
        );
        // Over transparent, any mode is the source; Erase removes what the source covers.
        let source = [0.2, 0.1, 0.05, 0.5];
        assert_eq!(composite(BlendMode::Difference, [0.0; 4], source), source);
        let erased = composite(
            BlendMode::Erase,
            [0.4, 0.4, 0.4, 0.8],
            [1.0, 0.0, 0.0, 0.25],
        );
        assert!((erased[3] - 0.6).abs() < 1e-6 && (erased[0] - 0.3).abs() < 1e-6);
    }
}
