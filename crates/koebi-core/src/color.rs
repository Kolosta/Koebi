//! Color science for the design system.
//!
//! # Color spaces
//!
//! - [`Color`]: **gamma-encoded sRGB with straight alpha**. This is what designers pick in
//!   Figma or CSS (`#3366FF`) and what every public API accepts. Koebi blends UI in this space
//!   (UNORM render targets) so that on-screen results match design tools; see
//!   `docs/adr/0004-color-pipeline.md`.
//! - [`LinearRgba`]: linear-light sRGB primaries, for physically meaningful math (lighting,
//!   blurs, future HDR compositing).
//! - [`Oklab`] / [`Oklch`]: perceptually uniform spaces (Björn Ottosson, 2020). Use them to
//!   build palettes with even lightness steps, to interpolate gradients without muddy
//!   midpoints, and to derive hover/pressed states by changing lightness only.
//!
//! Conversions run in `f64` and are rounded once to `f32`.

#![allow(
    clippy::many_single_char_names,
    reason = "color math uses the standard notation of the published formulas (r, g, b, l, m, s)"
)]

use core::fmt;
use core::str::FromStr;

/// A color in gamma-encoded sRGB with straight (non-premultiplied) alpha.
///
/// Components are nominally in `0.0..=1.0`. The GPU receives premultiplied values, see
/// [`Color::premultiplied`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Color {
    /// Red, gamma-encoded.
    pub r: f32,
    /// Green, gamma-encoded.
    pub g: f32,
    /// Blue, gamma-encoded.
    pub b: f32,
    /// Opacity, linear (`0.0` transparent, `1.0` opaque).
    pub a: f32,
}

impl Color {
    /// Fully transparent black.
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);
    /// Opaque black.
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    /// Opaque white.
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);

    /// Creates a color from gamma-encoded components and straight alpha.
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Creates an opaque color from gamma-encoded components.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    /// Creates a color from 8-bit components, e.g. `Color::from_rgba8(51, 102, 255, 255)`.
    pub const fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }

    /// Creates an opaque color from a `0xRRGGBB` literal, e.g. `Color::from_hex(0x3366FF)`.
    pub const fn from_hex(rgb: u32) -> Self {
        Self::from_hex_rgba((rgb << 8) | 0xFF)
    }

    /// Creates a color from a `0xRRGGBBAA` literal.
    pub const fn from_hex_rgba(rgba: u32) -> Self {
        Self::new(
            ((rgba >> 24) & 0xFF) as f32 / 255.0,
            ((rgba >> 16) & 0xFF) as f32 / 255.0,
            ((rgba >> 8) & 0xFF) as f32 / 255.0,
            (rgba & 0xFF) as f32 / 255.0,
        )
    }

    /// Parses `#RGB`, `#RGBA`, `#RRGGBB` or `#RRGGBBAA` (case-insensitive).
    ///
    /// # Errors
    ///
    /// Returns a [`ParseColorError`] if the string does not start with `#`, has another length
    /// or contains a non-hexadecimal digit.
    pub fn parse_hex(text: &str) -> Result<Self, ParseColorError> {
        let digits = text.strip_prefix('#').ok_or(ParseColorError::MissingHash)?;
        let mut values = [0_u32; 8];
        for (slot, ch) in values.iter_mut().zip(digits.chars()) {
            *slot = ch.to_digit(16).ok_or(ParseColorError::InvalidDigit(ch))?;
        }
        let count = digits.chars().count();
        let rgba = match count {
            // Short forms duplicate each nibble: #3af == #33aaff.
            3 | 4 => {
                let alpha = if count == 4 { values[3] } else { 0xF };
                [values[0], values[1], values[2], alpha].map(|n| n * 0x11)
            }
            6 | 8 => {
                let byte = |i: usize| values[i] * 16 + values[i + 1];
                let alpha = if count == 8 { byte(6) } else { 0xFF };
                [byte(0), byte(2), byte(4), alpha]
            }
            other => return Err(ParseColorError::InvalidLength(other)),
        };
        Ok(Self::from_hex_rgba(
            (rgba[0] << 24) | (rgba[1] << 16) | (rgba[2] << 8) | rgba[3],
        ))
    }

    /// Returns the 8-bit components, rounded and clamped.
    #[must_use]
    pub fn to_rgba8(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a].map(unit_to_u8)
    }

    /// Returns the same color with another opacity.
    #[must_use]
    pub const fn with_alpha(self, a: f32) -> Self {
        Self::new(self.r, self.g, self.b, a)
    }

    /// Returns `[r·a, g·a, b·a, a]`, the premultiplied form uploaded to the GPU.
    #[must_use]
    pub fn premultiplied(self) -> [f32; 4] {
        [self.r * self.a, self.g * self.a, self.b * self.a, self.a]
    }

    /// Converts to linear light.
    #[must_use]
    pub fn to_linear(self) -> LinearRgba {
        LinearRgba::new(
            narrow(srgb_to_linear(f64::from(self.r))),
            narrow(srgb_to_linear(f64::from(self.g))),
            narrow(srgb_to_linear(f64::from(self.b))),
            self.a,
        )
    }

    /// Converts to OKLab.
    #[must_use]
    pub fn to_oklab(self) -> Oklab {
        Oklab::from_linear(self.to_linear())
    }

    /// Converts to OKLCH.
    #[must_use]
    pub fn to_oklch(self) -> Oklch {
        self.to_oklab().to_oklch()
    }

    /// Interpolates perceptually (in OKLab) between `self` (`t = 0`) and `other` (`t = 1`).
    #[must_use]
    pub fn mix_oklab(self, other: Self, t: f32) -> Self {
        let (a, b) = (self.to_oklab(), other.to_oklab());
        let lerp = |x: f32, y: f32| (y - x).mul_add(t, x);
        Oklab {
            l: lerp(a.l, b.l),
            a: lerp(a.a, b.a),
            b: lerp(a.b, b.b),
            alpha: lerp(a.alpha, b.alpha),
        }
        .to_color()
    }

    /// WCAG 2.x relative luminance (`0.0` black to `1.0` white). Opacity is ignored.
    #[must_use]
    pub fn relative_luminance(self) -> f32 {
        let lin = self.to_linear();
        let y = 0.2126 * f64::from(lin.r) + 0.7152 * f64::from(lin.g) + 0.0722 * f64::from(lin.b);
        narrow(y)
    }

    /// WCAG 2.x contrast ratio between two opaque colors, from `1.0` (none) to `21.0`.
    ///
    /// Body text needs at least 4.5:1 (AA) and large text 3:1.
    #[must_use]
    pub fn contrast_ratio(self, other: Self) -> f32 {
        let (l1, l2) = (self.relative_luminance(), other.relative_luminance());
        let (lighter, darker) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
        (lighter + 0.05) / (darker + 0.05)
    }
}

impl FromStr for Color {
    type Err = ParseColorError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse_hex(text)
    }
}

impl fmt::Display for Color {
    /// Formats as `#rrggbb`, or `#rrggbbaa` when not fully opaque.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [r, g, b, a] = self.to_rgba8();
        if a == u8::MAX {
            write!(f, "#{r:02x}{g:02x}{b:02x}")
        } else {
            write!(f, "#{r:02x}{g:02x}{b:02x}{a:02x}")
        }
    }
}

/// Error returned by [`Color::parse_hex`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseColorError {
    /// The string does not start with `#`.
    MissingHash,
    /// The number of hex digits is not 3, 4, 6 or 8.
    InvalidLength(usize),
    /// A character is not a hexadecimal digit.
    InvalidDigit(char),
}

impl fmt::Display for ParseColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHash => f.write_str("color must start with '#'"),
            Self::InvalidLength(n) => write!(f, "expected 3, 4, 6 or 8 hex digits, found {n}"),
            Self::InvalidDigit(c) => write!(f, "invalid hex digit {c:?}"),
        }
    }
}

impl std::error::Error for ParseColorError {}

/// A color in linear-light sRGB primaries with straight alpha.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct LinearRgba {
    /// Red, linear light.
    pub r: f32,
    /// Green, linear light.
    pub g: f32,
    /// Blue, linear light.
    pub b: f32,
    /// Opacity.
    pub a: f32,
}

impl LinearRgba {
    /// Creates a linear color.
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Converts back to gamma-encoded sRGB.
    #[must_use]
    pub fn to_srgb(self) -> Color {
        Color::new(
            narrow(linear_to_srgb(f64::from(self.r))),
            narrow(linear_to_srgb(f64::from(self.g))),
            narrow(linear_to_srgb(f64::from(self.b))),
            self.a,
        )
    }

    fn is_in_gamut(self) -> bool {
        const EPS: f32 = 1e-4;
        [self.r, self.g, self.b]
            .iter()
            .all(|c| (-EPS..=1.0 + EPS).contains(c))
    }
}

impl From<Color> for LinearRgba {
    fn from(color: Color) -> Self {
        color.to_linear()
    }
}

impl From<LinearRgba> for Color {
    fn from(color: LinearRgba) -> Self {
        color.to_srgb()
    }
}

/// A color in the OKLab perceptual space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Oklab {
    /// Perceived lightness, `0.0` (black) to `1.0` (white).
    pub l: f32,
    /// Green (negative) to red (positive) axis.
    pub a: f32,
    /// Blue (negative) to yellow (positive) axis.
    pub b: f32,
    /// Opacity.
    pub alpha: f32,
}

impl Oklab {
    /// Converts from linear sRGB.
    #[must_use]
    pub fn from_linear(color: LinearRgba) -> Self {
        let (r, g, b) = (f64::from(color.r), f64::from(color.g), f64::from(color.b));
        let l = 0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b;
        let m = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
        let s = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;
        let (l, m, s) = (l.cbrt(), m.cbrt(), s.cbrt());
        Self {
            l: narrow(0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s),
            a: narrow(1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s),
            b: narrow(0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s),
            alpha: color.a,
        }
    }

    /// Converts to linear sRGB (may be out of gamut).
    #[must_use]
    pub fn to_linear(self) -> LinearRgba {
        let (l, a, b) = (f64::from(self.l), f64::from(self.a), f64::from(self.b));
        let l_ = l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
        let m_ = l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
        let s_ = l - 0.089_484_177_5 * a - 1.291_485_548_0 * b;
        let (l, m, s) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
        LinearRgba::new(
            narrow(4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s),
            narrow(-1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s),
            narrow(-0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701_0 * s),
            self.alpha,
        )
    }

    /// Converts to gamma-encoded sRGB, clamping out-of-gamut channels.
    ///
    /// Prefer [`Oklch::to_color`] when building colors from perceptual parameters: it keeps
    /// lightness and hue by reducing chroma instead of clipping channels.
    #[must_use]
    pub fn to_color(self) -> Color {
        let srgb = self.to_linear().to_srgb();
        Color::new(
            srgb.r.clamp(0.0, 1.0),
            srgb.g.clamp(0.0, 1.0),
            srgb.b.clamp(0.0, 1.0),
            srgb.a.clamp(0.0, 1.0),
        )
    }

    /// Converts to the cylindrical OKLCH form.
    #[must_use]
    pub fn to_oklch(self) -> Oklch {
        let chroma = self.a.hypot(self.b);
        let hue = if chroma < 1e-6 {
            0.0
        } else {
            self.b.atan2(self.a).to_degrees().rem_euclid(360.0)
        };
        Oklch::new(self.l, chroma, hue).with_alpha(self.alpha)
    }
}

/// A color in OKLCH: OKLab expressed as lightness, chroma and hue.
///
/// This is the most convenient space for design tokens: `l` controls perceived lightness,
/// `c` colorfulness and `h` the hue angle, independently.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Oklch {
    /// Perceived lightness, `0.0` to `1.0`.
    pub l: f32,
    /// Chroma, `0.0` (gray) to about `0.37` for the most saturated sRGB colors.
    pub c: f32,
    /// Hue angle in degrees, `0.0..360.0`.
    pub h: f32,
    /// Opacity.
    pub alpha: f32,
}

impl Oklch {
    /// Creates an opaque OKLCH color.
    pub const fn new(l: f32, c: f32, h: f32) -> Self {
        Self {
            l,
            c,
            h,
            alpha: 1.0,
        }
    }

    /// Returns the same color with another opacity.
    #[must_use]
    pub const fn with_alpha(self, alpha: f32) -> Self {
        Self { alpha, ..self }
    }

    /// Returns the same color with another lightness.
    #[must_use]
    pub const fn with_lightness(self, l: f32) -> Self {
        Self { l, ..self }
    }

    /// Returns the same color with another chroma.
    #[must_use]
    pub const fn with_chroma(self, c: f32) -> Self {
        Self { c, ..self }
    }

    /// Converts to OKLab.
    #[must_use]
    pub fn to_oklab(self) -> Oklab {
        let (sin, cos) = self.h.to_radians().sin_cos();
        Oklab {
            l: self.l,
            a: self.c * cos,
            b: self.c * sin,
            alpha: self.alpha,
        }
    }

    /// Converts to sRGB, mapping out-of-gamut colors into gamut by reducing chroma.
    ///
    /// Lightness and hue are preserved, which keeps palettes visually consistent even when a
    /// token asks for more colorfulness than sRGB can display.
    #[must_use]
    pub fn to_color(self) -> Color {
        let base = Self {
            l: self.l.clamp(0.0, 1.0),
            c: self.c.max(0.0),
            ..self
        };
        if base.to_oklab().to_linear().is_in_gamut() {
            return base.to_oklab().to_color();
        }
        let (mut low, mut high) = (0.0_f32, base.c);
        for _ in 0..24 {
            let mid = f32::midpoint(low, high);
            if base.with_chroma(mid).to_oklab().to_linear().is_in_gamut() {
                low = mid;
            } else {
                high = mid;
            }
        }
        base.with_chroma(low).to_oklab().to_color()
    }
}

impl From<Oklch> for Color {
    fn from(color: Oklch) -> Self {
        color.to_color()
    }
}

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f64) -> f64 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "colors are stored as f32; conversions run in f64 and round once here"
)]
const fn narrow(value: f64) -> f32 {
    value as f32
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to 0..=255 before the cast"
)]
fn unit_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f32, expected: f32, tolerance: f32) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {expected}, got {actual} (tolerance {tolerance})"
        );
    }

    #[test]
    fn hex_constructors_decode_channels() {
        let c = Color::from_hex(0xFF_80_00);
        assert_eq!(c.to_rgba8(), [255, 128, 0, 255]);
        assert_eq!(
            Color::from_hex_rgba(0x11_22_33_44).to_rgba8(),
            [0x11, 0x22, 0x33, 0x44]
        );
        assert_eq!(Color::from_rgba8(1, 2, 3, 4).to_rgba8(), [1, 2, 3, 4]);
    }

    #[test]
    fn parse_hex_accepts_all_css_forms() {
        assert_eq!(
            "#3af".parse::<Color>().unwrap().to_rgba8(),
            [0x33, 0xAA, 0xFF, 0xFF]
        );
        assert_eq!(
            "#3af8".parse::<Color>().unwrap().to_rgba8(),
            [0x33, 0xAA, 0xFF, 0x88]
        );
        assert_eq!(
            "#3366FF".parse::<Color>().unwrap().to_rgba8(),
            [0x33, 0x66, 0xFF, 0xFF]
        );
        assert_eq!(
            "#3366ff80".parse::<Color>().unwrap().to_rgba8(),
            [0x33, 0x66, 0xFF, 0x80]
        );
    }

    #[test]
    fn parse_hex_rejects_malformed_input() {
        assert_eq!(
            Color::parse_hex("3366ff"),
            Err(ParseColorError::MissingHash)
        );
        assert_eq!(
            Color::parse_hex("#12345"),
            Err(ParseColorError::InvalidLength(5))
        );
        assert_eq!(
            Color::parse_hex("#12g"),
            Err(ParseColorError::InvalidDigit('g'))
        );
    }

    #[test]
    fn display_round_trips_through_parse() {
        for text in ["#3366ff", "#00000080", "#ffffff"] {
            assert_eq!(text.parse::<Color>().unwrap().to_string(), text);
        }
    }

    #[test]
    fn premultiplied_scales_color_by_alpha() {
        let c = Color::new(1.0, 0.5, 0.25, 0.5).premultiplied();
        assert_eq!(c, [0.5, 0.25, 0.125, 0.5]);
    }

    #[test]
    fn srgb_transfer_matches_reference_values() {
        assert_close(Color::rgb(0.5, 0.5, 0.5).to_linear().r, 0.214_041, 1e-5);
        assert_close(Color::rgb(0.02, 0.0, 0.0).to_linear().r, 0.02 / 12.92, 1e-7);
        for i in 0..=255_u8 {
            let c = Color::from_rgba8(i, i, i, 255);
            assert_eq!(c.to_linear().to_srgb().to_rgba8(), c.to_rgba8());
        }
    }

    #[test]
    fn oklab_matches_published_reference_values() {
        // Reference values from Björn Ottosson's OKLab definition.
        let red = Color::from_hex(0xFF_00_00).to_oklab();
        assert_close(red.l, 0.627_955, 1e-4);
        assert_close(red.a, 0.224_863, 1e-4);
        assert_close(red.b, 0.125_846, 1e-4);

        let blue = Color::from_hex(0x00_00_FF).to_oklab();
        assert_close(blue.l, 0.452_014, 1e-4);
        assert_close(blue.a, -0.032_457, 1e-4);
        assert_close(blue.b, -0.311_528, 1e-4);

        let white = Color::WHITE.to_oklab();
        assert_close(white.l, 1.0, 1e-4);
        assert_close(white.a, 0.0, 1e-4);
        assert_close(white.b, 0.0, 1e-4);
    }

    #[test]
    fn oklch_round_trip_preserves_color() {
        for hex in [0x33_66_FF, 0xE5_48_4D, 0x1E_1E_24, 0xF5_F5_F5, 0x00_C8_53] {
            let original = Color::from_hex(hex);
            let back = original.to_oklch().to_color();
            assert_eq!(
                back.to_rgba8(),
                original.to_rgba8(),
                "round trip of #{hex:06x}"
            );
        }
        let red = Color::from_hex(0xFF_00_00).to_oklch();
        assert_close(red.c, 0.257_683, 1e-4);
        assert_close(red.h, 29.233_885, 1e-2);
    }

    #[test]
    fn oklch_gamut_mapping_keeps_lightness_and_hue() {
        // Far more chroma than sRGB can show at this lightness.
        let requested = Oklch::new(0.7, 0.4, 150.0);
        let mapped = requested.to_color();
        let result = mapped.to_oklch();
        assert!(mapped.to_linear().is_in_gamut());
        assert_close(result.l, 0.7, 2e-3);
        assert_close(result.h, 150.0, 0.5);
        assert!(result.c < 0.4);
    }

    #[test]
    fn mix_oklab_hits_endpoints() {
        let a = Color::from_hex(0x33_66_FF);
        let b = Color::from_hex(0xFF_CC_00);
        assert_eq!(a.mix_oklab(b, 0.0).to_rgba8(), a.to_rgba8());
        assert_eq!(a.mix_oklab(b, 1.0).to_rgba8(), b.to_rgba8());
    }

    #[test]
    fn contrast_ratio_follows_wcag() {
        assert_close(Color::BLACK.contrast_ratio(Color::WHITE), 21.0, 1e-4);
        assert_close(Color::WHITE.contrast_ratio(Color::WHITE), 1.0, 1e-6);
        assert_close(
            Color::from_hex(0x77_77_77).contrast_ratio(Color::WHITE),
            4.478,
            5e-3,
        );
    }
}
