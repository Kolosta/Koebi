//! Color models a color picker edits in, besides gamma-encoded sRGB ([`Color`]) and OKLab and
//! OKLCH ([`crate::Oklch`]): HSV, HSL and CMYK, each from and to [`Color`].
//!
//! HSV and HSL are cylinders over gamma-encoded sRGB, as every editor defines them: their
//! saturation and value are not perceptual (a yellow and a blue of the same value differ
//! much in brightness). CMYK here is the naive conversion, without an ink profile: what a
//! picker shows for reference, not a print workflow.
//!
//! Hues are in `0.0..1.0` (a full turn), every other component in `0.0..=1.0`; alpha is kept
//! apart, as [`Color::a`].

use crate::color::Color;

/// Hue, saturation and value over gamma-encoded sRGB.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Hsv {
    /// Hue, in turns: 0 red, 1/3 green, 2/3 blue.
    pub h: f32,
    /// Saturation: 0 gray, 1 pure.
    pub s: f32,
    /// Value: 0 black, 1 the brightest of the hue.
    pub v: f32,
}

/// Hue, saturation and lightness over gamma-encoded sRGB.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Hsl {
    /// Hue, in turns.
    pub h: f32,
    /// Saturation: 0 gray, 1 pure.
    pub s: f32,
    /// Lightness: 0 black, 1/2 the pure hue, 1 white.
    pub l: f32,
}

/// Cyan, magenta, yellow and key (black), the naive conversion of gamma-encoded sRGB.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Cmyk {
    /// Cyan.
    pub c: f32,
    /// Magenta.
    pub m: f32,
    /// Yellow.
    pub y: f32,
    /// Black.
    pub k: f32,
}

/// The hue (in turns) of `color` and its largest and smallest components.
fn hue_of(color: Color) -> (f32, f32, f32) {
    let (r, g, b) = (color.r, color.g, color.b);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta <= f32::EPSILON {
        0.0
    } else if (max - r).abs() <= f32::EPSILON {
        ((g - b) / delta).rem_euclid(6.0)
    } else if (max - g).abs() <= f32::EPSILON {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    (hue / 6.0, max, min)
}

/// The pure color of `hue` (in turns), its components scaled from `min` to `max`.
fn from_hue(hue: f32, max: f32, min: f32, alpha: f32) -> Color {
    let sector = hue.rem_euclid(1.0) * 6.0;
    let chroma = max - min;
    let rising = |x: f32| min + chroma * x;
    let falling = |x: f32| max - chroma * x;
    let within = sector.fract();
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a sector of the hue circle, 0 to 5"
    )]
    let (r, g, b) = match sector as u32 {
        0 => (max, rising(within), min),
        1 => (falling(within), max, min),
        2 => (min, max, rising(within)),
        3 => (min, falling(within), max),
        4 => (rising(within), min, max),
        _ => (max, min, falling(within)),
    };
    Color::new(r, g, b, alpha)
}

impl Hsv {
    /// The HSV components of `color`, its alpha left out.
    pub fn from_color(color: Color) -> Self {
        let (h, max, min) = hue_of(color);
        let s = if max <= f32::EPSILON {
            0.0
        } else {
            (max - min) / max
        };
        Self { h, s, v: max }
    }

    /// The color, with `alpha`.
    pub fn to_color(self, alpha: f32) -> Color {
        let (s, v) = (self.s.clamp(0.0, 1.0), self.v.clamp(0.0, 1.0));
        from_hue(self.h, v, v * (1.0 - s), alpha)
    }
}

impl Hsl {
    /// The HSL components of `color`, its alpha left out.
    pub fn from_color(color: Color) -> Self {
        let (h, max, min) = hue_of(color);
        let l = f32::midpoint(max, min);
        let s = if max - min <= f32::EPSILON {
            0.0
        } else {
            (max - min) / (1.0 - (2.0 * l - 1.0).abs())
        };
        Self {
            h,
            s: s.clamp(0.0, 1.0),
            l,
        }
    }

    /// The color, with `alpha`.
    pub fn to_color(self, alpha: f32) -> Color {
        let (s, l) = (self.s.clamp(0.0, 1.0), self.l.clamp(0.0, 1.0));
        let chroma = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let min = l - chroma * 0.5;
        from_hue(self.h, min + chroma, min, alpha)
    }
}

impl Cmyk {
    /// The CMYK components of `color`, its alpha left out.
    pub fn from_color(color: Color) -> Self {
        let max = color.r.max(color.g).max(color.b);
        let k = 1.0 - max;
        if max <= f32::EPSILON {
            return Self {
                c: 0.0,
                m: 0.0,
                y: 0.0,
                k: 1.0,
            };
        }
        Self {
            c: (max - color.r) / max,
            m: (max - color.g) / max,
            y: (max - color.b) / max,
            k,
        }
    }

    /// The color, with `alpha`.
    pub fn to_color(self, alpha: f32) -> Color {
        let ink = |part: f32| (1.0 - part.clamp(0.0, 1.0)) * (1.0 - self.k.clamp(0.0, 1.0));
        Color::new(ink(self.c), ink(self.m), ink(self.y), alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Color, b: Color) -> bool {
        [(a.r, b.r), (a.g, b.g), (a.b, b.b), (a.a, b.a)]
            .iter()
            .all(|(x, y)| (x - y).abs() < 1e-4)
    }

    #[test]
    fn primaries_have_their_hues() {
        let red = Hsv::from_color(Color::rgb(1.0, 0.0, 0.0));
        assert_eq!((red.h, red.s, red.v), (0.0, 1.0, 1.0));
        let green = Hsv::from_color(Color::rgb(0.0, 1.0, 0.0));
        assert!((green.h - 1.0 / 3.0).abs() < 1e-6);
        let blue = Hsl::from_color(Color::rgb(0.0, 0.0, 1.0));
        assert!((blue.h - 2.0 / 3.0).abs() < 1e-6 && (blue.l - 0.5).abs() < 1e-6);
        // Grays have no saturation, and black no value.
        assert_eq!(Hsv::from_color(Color::rgb(0.5, 0.5, 0.5)).s, 0.0);
        assert_eq!(Hsv::from_color(Color::BLACK).v, 0.0);
        assert_eq!(Cmyk::from_color(Color::BLACK).k, 1.0);
        let cyan = Cmyk::from_color(Color::rgb(0.0, 1.0, 1.0));
        assert_eq!((cyan.c, cyan.m, cyan.y, cyan.k), (1.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn every_model_goes_there_and_back() {
        let samples = [
            Color::new(0.2, 0.4, 0.9, 0.5),
            Color::rgb(0.95, 0.1, 0.3),
            Color::rgb(0.3, 0.8, 0.2),
            Color::rgb(0.7, 0.7, 0.1),
            Color::rgb(0.05, 0.6, 0.6),
            Color::rgb(0.6, 0.1, 0.8),
            Color::WHITE,
            Color::BLACK,
            Color::rgb(0.25, 0.25, 0.25),
        ];
        for color in samples {
            let alpha = color.a;
            assert!(
                close(Hsv::from_color(color).to_color(alpha), color),
                "HSV {color:?}"
            );
            assert!(
                close(Hsl::from_color(color).to_color(alpha), color),
                "HSL {color:?}"
            );
            assert!(
                close(Cmyk::from_color(color).to_color(alpha), color),
                "CMYK {color:?}"
            );
        }
        // Hues wrap a full turn.
        let wrapped = Hsv {
            h: 1.25,
            s: 1.0,
            v: 1.0,
        };
        let quarter = Hsv { h: 0.25, ..wrapped };
        assert!(close(wrapped.to_color(1.0), quarter.to_color(1.0)));
    }
}
