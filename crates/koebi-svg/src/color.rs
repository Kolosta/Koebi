//! Colors as SVG writes them: `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()`, the
//! basic color keywords, `transparent`, `none` and `currentColor`.

/// A color, gamma-encoded sRGB with straight alpha, each channel from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba(pub [f32; 4]);

impl Rgba {
    /// Opaque black, SVG's default fill.
    pub const BLACK: Self = Self([0.0, 0.0, 0.0, 1.0]);

    /// The color with its alpha multiplied by `opacity`.
    #[must_use]
    pub fn faded(self, opacity: f32) -> Self {
        let [r, g, b, a] = self.0;
        Self([r, g, b, a * opacity.clamp(0.0, 1.0)])
    }

    /// The color as 8-bit channels, for comparisons and zone keys.
    pub fn to_rgba8(self) -> [u8; 4] {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "channels clamped to 0..=1 before scaling to a byte"
        )]
        self.0
            .map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
    }
}

/// What a `fill` or a `stroke` paints with.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PaintValue {
    /// Nothing.
    None,
    /// A color.
    Color(Rgba),
    /// The `color` property, inherited.
    Current,
    /// A gradient, by its id.
    Url(String),
}

/// Reads a paint: `none`, a color, `currentColor` or `url(#id)`.
pub(crate) fn parse_paint(text: &str) -> Option<PaintValue> {
    let text = text.trim();
    if text == "none" {
        return Some(PaintValue::None);
    }
    if text == "currentColor" {
        return Some(PaintValue::Current);
    }
    if let Some(rest) = text.strip_prefix("url(") {
        let end = rest.find(')')?;
        let id = rest[..end].trim().trim_matches(|c| c == '"' || c == '\'');
        return id
            .strip_prefix('#')
            .map(|id| PaintValue::Url(id.to_owned()));
    }
    parse_color(text).map(PaintValue::Color)
}

/// Reads a color.
pub(crate) fn parse_color(text: &str) -> Option<Rgba> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix('#') {
        return parse_hex(hex);
    }
    let lower = text.to_ascii_lowercase();
    if let Some(arguments) = lower
        .strip_prefix("rgba(")
        .or_else(|| lower.strip_prefix("rgb("))
    {
        return parse_function(arguments.strip_suffix(')')?);
    }
    keyword(&lower)
}

fn parse_hex(hex: &str) -> Option<Rgba> {
    let digit = |index: usize| {
        hex.get(index..=index)
            .and_then(|digit| u8::from_str_radix(digit, 16).ok())
    };
    let pair = |index: usize| {
        hex.get(index..index + 2)
            .and_then(|pair| u8::from_str_radix(pair, 16).ok())
    };
    let channel = |value: u8| f32::from(value) / 255.0;
    let short = |index: usize| digit(index).map(|value| channel(value * 17));
    match hex.len() {
        3 => Some(Rgba([short(0)?, short(1)?, short(2)?, 1.0])),
        4 => Some(Rgba([short(0)?, short(1)?, short(2)?, short(3)?])),
        6 => Some(Rgba([
            channel(pair(0)?),
            channel(pair(2)?),
            channel(pair(4)?),
            1.0,
        ])),
        8 => Some(Rgba([
            channel(pair(0)?),
            channel(pair(2)?),
            channel(pair(4)?),
            channel(pair(6)?),
        ])),
        _ => None,
    }
}

/// `r, g, b[, a]`: numbers from 0 to 255 or percentages, the alpha from 0 to 1 or a percentage.
fn parse_function(arguments: &str) -> Option<Rgba> {
    let parts: Vec<&str> = arguments
        .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() != 3 && parts.len() != 4 {
        return None;
    }
    let component = |text: &str, scale: f32| {
        let value = match text.strip_suffix('%') {
            Some(percent) => percent.parse::<f32>().ok()? / 100.0,
            None => text.parse::<f32>().ok()? / scale,
        };
        Some(value.clamp(0.0, 1.0))
    };
    Some(Rgba([
        component(parts[0], 255.0)?,
        component(parts[1], 255.0)?,
        component(parts[2], 255.0)?,
        parts
            .get(3)
            .map_or(Some(1.0), |alpha| component(alpha, 1.0))?,
    ]))
}

fn keyword(name: &str) -> Option<Rgba> {
    let rgb = |value: u32| {
        let channel = |shift: u32| ((value >> shift) & 0xFF) as f32 / 255.0;
        Some(Rgba([channel(16), channel(8), channel(0), 1.0]))
    };
    match name {
        "transparent" => Some(Rgba([0.0, 0.0, 0.0, 0.0])),
        "black" => rgb(0x00_00_00),
        "white" => rgb(0xFF_FF_FF),
        "red" => rgb(0xFF_00_00),
        "lime" => rgb(0x00_FF_00),
        "green" => rgb(0x00_80_00),
        "blue" => rgb(0x00_00_FF),
        "yellow" => rgb(0xFF_FF_00),
        "cyan" | "aqua" => rgb(0x00_FF_FF),
        "magenta" | "fuchsia" => rgb(0xFF_00_FF),
        "gray" | "grey" => rgb(0x80_80_80),
        "silver" => rgb(0xC0_C0_C0),
        "maroon" => rgb(0x80_00_00),
        "olive" => rgb(0x80_80_00),
        "navy" => rgb(0x00_00_80),
        "purple" => rgb(0x80_00_80),
        "teal" => rgb(0x00_80_80),
        "orange" => rgb(0xFF_A5_00),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_read_in_every_form() {
        assert_eq!(parse_color("#fff"), Some(Rgba([1.0, 1.0, 1.0, 1.0])));
        assert_eq!(
            parse_color("#E3E3E3").unwrap().to_rgba8(),
            [227, 227, 227, 255]
        );
        assert_eq!(
            parse_color("#ff000080").unwrap().to_rgba8(),
            [255, 0, 0, 128]
        );
        assert_eq!(
            parse_color("rgb(255, 0, 0)").unwrap().to_rgba8(),
            [255, 0, 0, 255]
        );
        assert_eq!(
            parse_color("rgba(0 0 255 / 50%)").unwrap().to_rgba8(),
            [0, 0, 255, 128]
        );
        assert_eq!(parse_color("Navy").unwrap().to_rgba8(), [0, 0, 128, 255]);
        assert_eq!(parse_color("#12"), None);
        assert_eq!(parse_color("salmon"), None);
        assert_eq!(parse_paint("none"), Some(PaintValue::None));
        assert_eq!(
            parse_paint("url(#shade)"),
            Some(PaintValue::Url("shade".into()))
        );
        assert_eq!(parse_paint("currentColor"), Some(PaintValue::Current));
    }
}
