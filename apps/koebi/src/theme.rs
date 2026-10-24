//! Design tokens of the demo scene.
//!
//! Tokens are expressed in OKLCH so that variants (hover, border, disabled) are derived by
//! changing lightness only, keeping hue and colorfulness consistent. This module is the seed
//! of the future design-system crate.

use koebi_core::{Color, Oklch};

/// Colors and metrics used to draw the scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Theme {
    pub(crate) background: Color,
    pub(crate) accent: Color,
    pub(crate) accent_border: Color,
    pub(crate) corner_radius: f32,
    pub(crate) border_width: f32,
}

impl Theme {
    /// The default dark theme.
    pub(crate) fn dark() -> Self {
        let accent = Oklch::new(0.62, 0.17, 265.0);
        Self {
            background: Color::from_hex(0x11_13_18),
            accent: accent.to_color(),
            accent_border: accent.with_lightness(0.80).with_chroma(0.10).to_color(),
            corner_radius: 16.0,
            border_width: 2.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accent_meets_wcag_non_text_contrast_on_background() {
        // WCAG 2.x requires 3:1 for user-interface components.
        let theme = Theme::dark();
        assert!(theme.accent.contrast_ratio(theme.background) >= 3.0);
        assert!(theme.accent_border.contrast_ratio(theme.background) >= 3.0);
    }

    #[test]
    fn derived_border_keeps_the_accent_hue() {
        let theme = Theme::dark();
        let hue = |c: Color| c.to_oklch().h;
        assert!((hue(theme.accent) - hue(theme.accent_border)).abs() < 3.0);
        assert!(theme.accent_border.to_oklch().l > theme.accent.to_oklch().l);
    }
}
