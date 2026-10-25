//! Value constructors for token tables, all `const`.
//!
//! Import them with `use koebi_design::define::*;` next to a [`tokens!`](crate::tokens)
//! invocation. Every constructor checks its arguments: an invalid value in a token table is a
//! compile error.
//!
//! Colors can be written in the notation that fits the source of the value:
//!
//! | Notation | Example |
//! |---|---|
//! | Hexadecimal, 3, 4, 6 or 8 digits | `hex("#F00")`, `hex("#F008")`, `hex("#5D89FF")`, `hex("#5D89FF80")` |
//! | 8-bit channels, opacity from 0 to 1 | `rgb(93, 137, 255)`, `rgba(93, 137, 255, 0.5)` |
//! | 10-bit channels (0 to 1023) | `rgb10(372, 549, 1023)`, `rgba10(372, 549, 1023, 0.5)` |
//! | Channels from 0 to 1 | `rgb_f32(0.36, 0.54, 1.0)`, `rgba_f32(0.36, 0.54, 1.0, 0.5)` |
//! | HSL: hue in degrees, saturation and lightness from 0 to 1 | `hsl(225.0, 1.0, 0.68)`, `hsla(225.0, 1.0, 0.68, 0.5)` |
//! | OKLCH: lightness, chroma, hue in degrees | `oklch(0.68, 0.17, 266.0)`, `oklcha(0.68, 0.17, 266.0, 0.5)` |
//!
//! Every notation except OKLCH is converted to gamma-encoded sRGB while compiling. OKLCH needs
//! trigonometry, which constant evaluation cannot run: the table keeps the OKLCH value and the
//! design system converts it once when it loads, reducing chroma if the color is outside the
//! sRGB gamut.
//!
//! Other kinds: `px(4.0)` for dimensions, `ms(150)` for durations, `bezier(0.4, 0.0, 0.2, 1.0)`
//! for easing curves, `weight(600)` for font weights, `family("Inter")` for font families;
//! numbers and integers are plain literals.

use core::time::Duration;

use koebi_core::{Color, Oklch};

use crate::value::{ColorLiteral, CubicBezier, FontWeight};

/// A length in logical pixels, as written in token tables.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Px(pub f32);

/// A color from hexadecimal digits: `"#RGB"`, `"#RGBA"`, `"#RRGGBB"` or `"#RRGGBBAA"`, in
/// either case. A short form repeats each digit: `"#F80"` is `"#FF8800"`.
///
/// # Panics
///
/// At compile time in token tables, for any other form.
#[allow(
    clippy::panic,
    reason = "token tables are constants: the panic is a compile-time error"
)]
pub const fn hex(text: &str) -> ColorLiteral {
    let bytes = text.as_bytes();
    let digits = bytes.len().saturating_sub(1);
    assert!(
        !bytes.is_empty() && bytes[0] == b'#' && matches!(digits, 3 | 4 | 6 | 8),
        "hexadecimal colors are written #RGB, #RGBA, #RRGGBB or #RRGGBBAA"
    );
    let mut values = [0_u8; 8];
    let mut i = 0;
    while i < digits {
        values[i] = match bytes[i + 1] {
            b'0'..=b'9' => bytes[i + 1] - b'0',
            b'a'..=b'f' => bytes[i + 1] - b'a' + 10,
            b'A'..=b'F' => bytes[i + 1] - b'A' + 10,
            _ => panic!("hexadecimal colors only use the digits 0-9 and A-F"),
        };
        i += 1;
    }
    let alpha = if digits == 4 || digits == 8 {
        hex_channel(values, digits, 3)
    } else {
        u8::MAX
    };
    ColorLiteral::Srgb(Color::from_rgba8(
        hex_channel(values, digits, 0),
        hex_channel(values, digits, 1),
        hex_channel(values, digits, 2),
        alpha,
    ))
}

/// Channel `index` of parsed hexadecimal digits: short forms repeat each digit.
const fn hex_channel(values: [u8; 8], digits: usize, index: usize) -> u8 {
    if digits <= 4 {
        values[index] * 17
    } else {
        values[2 * index] * 16 + values[2 * index + 1]
    }
}

/// An opaque color from 8-bit channels.
pub const fn rgb(r: u8, g: u8, b: u8) -> ColorLiteral {
    ColorLiteral::Srgb(Color::from_rgba8(r, g, b, u8::MAX))
}

/// A color from 8-bit channels and an opacity from 0 to 1.
///
/// # Panics
///
/// At compile time in token tables, when `a` is outside `0.0..=1.0`.
#[allow(
    clippy::cast_lossless,
    reason = "`From` conversions are not callable in a const fn; u8 to f32 is lossless"
)]
pub const fn rgba(r: u8, g: u8, b: u8, a: f32) -> ColorLiteral {
    assert!(unit(a), "opacity lies in 0..=1");
    ColorLiteral::Srgb(Color::new(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a,
    ))
}

/// An opaque color from 10-bit channels (0 to 1023).
///
/// # Panics
///
/// At compile time in token tables, when a channel exceeds 1023.
pub const fn rgb10(r: u16, g: u16, b: u16) -> ColorLiteral {
    rgba10(r, g, b, 1.0)
}

/// A color from 10-bit channels (0 to 1023) and an opacity from 0 to 1.
///
/// # Panics
///
/// At compile time in token tables, when a channel exceeds 1023 or `a` is outside `0.0..=1.0`.
#[allow(
    clippy::cast_lossless,
    reason = "`From` conversions are not callable in a const fn; u16 to f32 is lossless"
)]
pub const fn rgba10(r: u16, g: u16, b: u16, a: f32) -> ColorLiteral {
    assert!(
        r <= 1023 && g <= 1023 && b <= 1023,
        "10-bit channels lie in 0..=1023"
    );
    assert!(unit(a), "opacity lies in 0..=1");
    ColorLiteral::Srgb(Color::new(
        r as f32 / 1023.0,
        g as f32 / 1023.0,
        b as f32 / 1023.0,
        a,
    ))
}

/// An opaque color from gamma-encoded channels from 0 to 1.
///
/// # Panics
///
/// At compile time in token tables, when a channel is outside `0.0..=1.0`.
pub const fn rgb_f32(r: f32, g: f32, b: f32) -> ColorLiteral {
    rgba_f32(r, g, b, 1.0)
}

/// A color from gamma-encoded channels and an opacity, all from 0 to 1. Use it for values that
/// eight bits per channel cannot express (`rgba_f32(1.0, 1.0, 1.0, 0.06)`).
///
/// # Panics
///
/// At compile time in token tables, when a channel is outside `0.0..=1.0`.
pub const fn rgba_f32(r: f32, g: f32, b: f32, a: f32) -> ColorLiteral {
    assert!(
        unit(r) && unit(g) && unit(b) && unit(a),
        "channels lie in 0..=1"
    );
    ColorLiteral::Srgb(Color::new(r, g, b, a))
}

/// An opaque color from its hue (degrees, `0.0..360.0`), saturation and lightness (`0.0..=1.0`).
///
/// # Panics
///
/// At compile time in token tables, for an argument out of its range.
pub const fn hsl(hue: f32, saturation: f32, lightness: f32) -> ColorLiteral {
    hsla(hue, saturation, lightness, 1.0)
}

/// A color from its hue (degrees, `0.0..360.0`), saturation, lightness and opacity
/// (`0.0..=1.0`).
///
/// # Panics
///
/// At compile time in token tables, for an argument out of its range.
pub const fn hsla(hue: f32, saturation: f32, lightness: f32, alpha: f32) -> ColorLiteral {
    assert!(hue >= 0.0 && hue < 360.0, "hue lies in 0..360 degrees");
    assert!(
        unit(saturation) && unit(lightness) && unit(alpha),
        "saturation, lightness and opacity lie in 0..=1"
    );
    let chroma = (1.0 - abs(2.0 * lightness - 1.0)) * saturation;
    let sector = hue / 60.0;
    let mut even = sector;
    while even >= 2.0 {
        even -= 2.0;
    }
    // The second largest channel, then the channels by hue sector.
    let second = chroma * (1.0 - abs(even - 1.0));
    let (red, green, blue) = if sector < 1.0 {
        (chroma, second, 0.0)
    } else if sector < 2.0 {
        (second, chroma, 0.0)
    } else if sector < 3.0 {
        (0.0, chroma, second)
    } else if sector < 4.0 {
        (0.0, second, chroma)
    } else if sector < 5.0 {
        (second, 0.0, chroma)
    } else {
        (chroma, 0.0, second)
    };
    let offset = lightness - chroma / 2.0;
    ColorLiteral::Srgb(Color::new(
        red + offset,
        green + offset,
        blue + offset,
        alpha,
    ))
}

/// An opaque color from OKLCH: perceived lightness (`0.0..=1.0`), chroma (`0.0` and up, about
/// `0.37` at most in sRGB) and hue (degrees).
///
/// # Panics
///
/// At compile time in token tables, for an argument out of its range.
pub const fn oklch(lightness: f32, chroma: f32, hue: f32) -> ColorLiteral {
    oklcha(lightness, chroma, hue, 1.0)
}

/// An OKLCH color with an opacity from 0 to 1.
///
/// # Panics
///
/// At compile time in token tables, for an argument out of its range.
pub const fn oklcha(lightness: f32, chroma: f32, hue: f32, a: f32) -> ColorLiteral {
    let color = Oklch::new(lightness, chroma, hue).with_alpha(a);
    assert!(
        ColorLiteral::Oklch(color).is_valid(),
        "OKLCH: lightness and opacity lie in 0..=1, chroma is positive, hue is finite"
    );
    ColorLiteral::Oklch(color)
}

/// A length in logical pixels.
pub const fn px(value: f32) -> Px {
    Px(value)
}

/// A duration in milliseconds.
pub const fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// An easing curve from `(0, 0)` to `(1, 1)` through the control points `(x1, y1)` and
/// `(x2, y2)`.
///
/// # Panics
///
/// At compile time in token tables, when `x1` or `x2` is outside `0.0..=1.0`.
pub const fn bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> CubicBezier {
    let curve = CubicBezier::new(x1, y1, x2, y2);
    assert!(
        curve.is_valid(),
        "cubic Bézier x coordinates must lie in 0..=1"
    );
    curve
}

/// A font weight.
///
/// # Panics
///
/// At compile time in token tables, when `weight` is outside `1..=1000`.
#[allow(
    clippy::panic,
    reason = "token tables are constants: the panic is a compile-time error"
)]
pub const fn weight(weight: u16) -> FontWeight {
    match FontWeight::new(weight) {
        Some(weight) => weight,
        None => panic!("font weights lie in 1..=1000"),
    }
}

/// A font family name.
pub const fn family(name: &'static str) -> &'static str {
    name
}

const fn unit(value: f32) -> bool {
    value >= 0.0 && value <= 1.0
}

const fn abs(value: f32) -> f32 {
    if value < 0.0 { -value } else { value }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn srgb(literal: ColorLiteral) -> Color {
        match literal {
            ColorLiteral::Srgb(color) => color,
            ColorLiteral::Oklch(_) => panic!("expected an sRGB literal"),
        }
    }

    #[test]
    fn hexadecimal_colors_accept_every_length_and_case() {
        assert_eq!(srgb(hex("#3366FF")), Color::from_hex(0x33_66_FF));
        assert_eq!(srgb(hex("#3366ff")), Color::from_hex(0x33_66_FF));
        assert_eq!(srgb(hex("#00000080")).to_rgba8(), [0, 0, 0, 0x80]);
        assert_eq!(srgb(hex("#F80")).to_rgba8(), [0xFF, 0x88, 0x00, 0xFF]);
        assert_eq!(srgb(hex("#F808")).to_rgba8(), [0xFF, 0x88, 0x00, 0x88]);
    }

    #[test]
    fn channel_notations_agree() {
        let expected = Color::from_hex(0x5D_89_FF);
        assert_eq!(srgb(rgb(0x5D, 0x89, 0xFF)), expected);
        assert_eq!(srgb(rgba(0x5D, 0x89, 0xFF, 1.0)), expected);
        assert_eq!(srgb(rgb10(1023, 0, 0)), Color::rgb(1.0, 0.0, 0.0));
        assert_eq!(srgb(rgba_f32(1.0, 1.0, 1.0, 0.25)).to_rgba8()[3], 64);
        assert_eq!(srgb(rgb_f32(0.0, 0.5, 1.0)), Color::rgb(0.0, 0.5, 1.0));
    }

    #[test]
    fn hsl_matches_reference_colors() {
        let cases = [
            (hsl(0.0, 1.0, 0.5), [255, 0, 0]),
            (hsl(120.0, 1.0, 0.25), [0, 128, 0]),
            (hsl(240.0, 1.0, 0.5), [0, 0, 255]),
            (hsl(60.0, 1.0, 0.5), [255, 255, 0]),
            (hsl(300.0, 0.5, 0.5), [191, 64, 191]),
            (hsl(0.0, 0.0, 0.5), [128, 128, 128]),
        ];
        for (literal, [r, g, b]) in cases {
            assert_eq!(srgb(literal).to_rgba8(), [r, g, b, 255], "{literal:?}");
        }
        assert_eq!(srgb(hsla(0.0, 1.0, 0.5, 0.5)).to_rgba8()[3], 128);
    }

    #[test]
    fn oklch_is_kept_and_converted_with_gamut_mapping() {
        let literal = oklch(0.62, 0.17, 265.0);
        assert!(matches!(literal, ColorLiteral::Oklch(_)));
        let color = literal.to_srgb();
        assert_eq!(color, Oklch::new(0.62, 0.17, 265.0).to_color());
        // More chroma than sRGB can show: reduced into the gamut, never clipped.
        let vivid = oklcha(0.7, 0.4, 150.0, 0.5).to_srgb();
        assert!(
            [vivid.r, vivid.g, vivid.b]
                .iter()
                .all(|c| (0.0..=1.0).contains(c))
        );
        assert!((vivid.a - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn other_constructors_build_the_expected_values() {
        assert_eq!(px(4.0), Px(4.0));
        assert_eq!(ms(150), Duration::from_millis(150));
        assert_eq!(weight(600).get(), 600);
        assert_eq!(
            bezier(0.4, 0.0, 1.0, 1.0),
            CubicBezier::new(0.4, 0.0, 1.0, 1.0)
        );
        assert_eq!(family("Inter"), "Inter");
    }
}
