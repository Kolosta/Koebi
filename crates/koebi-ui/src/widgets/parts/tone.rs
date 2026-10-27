//! [`Tone`]: the role a control plays ([`Variant`]) and an optional color family ([`Hue`]),
//! which pick its colors in every state ([`ToneColors`]).

use koebi_core::Color;

use super::states::States;

/// The role of a control, which picks its colors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Variant {
    /// A neutral control: most buttons.
    #[default]
    Default,
    /// The main action of a place, filled with the accent color.
    Accent,
    /// An action that confirms or succeeds.
    Positive,
    /// An action that deletes or cannot be undone.
    Negative,
    /// An action about information.
    Info,
    /// A strong neutral fill.
    Neutral,
    /// An action that needs attention.
    Notice,
}

impl Variant {
    /// Every variant, in declaration order.
    pub const ALL: [Self; 7] = [
        Self::Default,
        Self::Accent,
        Self::Positive,
        Self::Negative,
        Self::Info,
        Self::Neutral,
        Self::Notice,
    ];

    /// The position of the variant in [`Variant::ALL`].
    pub const fn index(self) -> usize {
        self as usize
    }

    /// `true` for variants filled with their color (every variant but the default one).
    pub const fn is_filled(self) -> bool {
        !matches!(self, Self::Default)
    }
}

/// A color family replacing the color of a control's variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hue {
    /// Blue.
    Blue,
    /// Cyan.
    Cyan,
    /// Green.
    Green,
    /// Yellow.
    Yellow,
    /// Orange.
    Orange,
    /// Red.
    Red,
    /// Magenta.
    Magenta,
    /// Purple.
    Purple,
}

impl Hue {
    /// Every family, in declaration order.
    pub const ALL: [Self; 8] = [
        Self::Blue,
        Self::Cyan,
        Self::Green,
        Self::Yellow,
        Self::Orange,
        Self::Red,
        Self::Magenta,
        Self::Purple,
    ];

    /// The position of the family in [`Hue::ALL`].
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// A variant, colored by its own intent or by a family: `Tone::ACCENT`,
/// `Tone::new(Variant::Positive).with_hue(Hue::Orange)`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Tone {
    /// The role.
    pub variant: Variant,
    /// The family replacing the role's color, if any.
    pub hue: Option<Hue>,
}

impl Tone {
    /// A neutral control.
    pub const DEFAULT: Self = Self::new(Variant::Default);
    /// The main action.
    pub const ACCENT: Self = Self::new(Variant::Accent);
    /// A confirming action.
    pub const POSITIVE: Self = Self::new(Variant::Positive);
    /// A destructive action.
    pub const NEGATIVE: Self = Self::new(Variant::Negative);
    /// An informative action.
    pub const INFO: Self = Self::new(Variant::Info);
    /// A strong neutral fill.
    pub const NEUTRAL: Self = Self::new(Variant::Neutral);
    /// An action needing attention.
    pub const NOTICE: Self = Self::new(Variant::Notice);

    /// `variant` in its own colors.
    pub const fn new(variant: Variant) -> Self {
        Self { variant, hue: None }
    }

    /// The same variant in the colors of `hue`.
    #[must_use]
    pub const fn with_hue(mut self, hue: Hue) -> Self {
        self.hue = Some(hue);
        self
    }
}

/// The colors of a control in every state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ToneColors {
    /// Background.
    pub background: States<Color>,
    /// Border.
    pub border: States<Color>,
    /// Label (and status text).
    pub content: States<Color>,
    /// Icons.
    pub icon: States<Color>,
}

/// The colors of a family, as tokens give them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HueColors {
    /// Fill of colored variants.
    pub fill: Color,
    /// Background of the default variant.
    pub subtle: Color,
    /// Label and border of the default variant.
    pub text: Color,
}

/// How hovered, pressed and disabled colors derive from one base color: a color family's, or
/// the selected color of a filled variant.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Shades {
    /// OKLCH lightness added when hovered.
    pub hover: f32,
    /// OKLCH lightness added when pressed.
    pub pressed: f32,
    /// Share of the family's color kept by disabled controls (the rest is the disabled
    /// color).
    pub disabled_tint: f32,
}

/// `color` with `delta` added to its OKLCH lightness.
pub(crate) fn shade(color: Color, delta: f32) -> Color {
    let oklch = color.to_oklch();
    oklch
        .with_lightness((oklch.l + delta).clamp(0.0, 1.0))
        .to_color()
        .with_alpha(color.a)
}

/// The colors of `variant` in the family `hue`: filled variants take the family's fill, the
/// default variant its subtle background with its text color; `base` gives the colors of the
/// variant itself (content of filled variants, disabled colors), `shades` the hovered, pressed
/// and disabled shades.
pub(crate) fn hued(
    variant: Variant,
    base: &ToneColors,
    hue: HueColors,
    shades: Shades,
) -> ToneColors {
    let disabled = |base: Color, color: Color| base.mix_oklab(color, shades.disabled_tint);
    let ladder = |color: Color, selected: Color| States {
        normal: color,
        hovered: shade(color, shades.hover),
        pressed: shade(color, shades.pressed),
        selected,
        selected_hovered: shade(selected, shades.hover),
        disabled: color,
    };
    let (mut background, mut border, mut content) = if variant.is_filled() {
        let pressed = shade(hue.fill, shades.pressed);
        (
            ladder(hue.fill, pressed),
            ladder(hue.fill, pressed),
            base.content,
        )
    } else {
        (
            ladder(hue.subtle, hue.fill),
            ladder(hue.text, hue.fill),
            States {
                selected: base.content.selected,
                selected_hovered: base.content.selected_hovered,
                ..States::all(hue.text)
            },
        )
    };
    background.disabled = disabled(base.background.disabled, background.normal);
    border.disabled = disabled(base.border.disabled, border.normal);
    content.disabled = disabled(base.content.disabled, content.normal);
    ToneColors {
        background,
        border,
        content,
        icon: content,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> ToneColors {
        ToneColors {
            background: States::all(Color::from_hex(0x30_30_30)),
            border: States::all(Color::from_hex(0x50_50_50)),
            content: States::all(Color::WHITE),
            icon: States::all(Color::WHITE),
        }
    }

    fn blue() -> HueColors {
        HueColors {
            fill: Color::from_hex(0x2D_6A_FF),
            subtle: Color::from_hex(0x10_20_40),
            text: Color::from_hex(0x80_A8_FF),
        }
    }

    const SHADES: Shades = Shades {
        hover: 0.06,
        pressed: -0.05,
        disabled_tint: 0.35,
    };

    #[test]
    fn shades_move_the_lightness_only() {
        let color = Color::from_hex(0x2D_6A_FF);
        let lighter = shade(color, 0.1).to_oklch();
        let original = color.to_oklch();
        assert!((lighter.l - original.l - 0.1).abs() < 0.02);
        assert!((lighter.h - original.h).abs() < 2.0);
        assert_eq!(shade(color.with_alpha(0.5), 0.1).a, 0.5);
    }

    #[test]
    fn filled_variants_take_the_family_fill_and_keep_their_content() {
        let colors = hued(Variant::Accent, &base(), blue(), SHADES);
        assert_eq!(colors.background.normal, blue().fill);
        assert!(colors.background.hovered.to_oklch().l > blue().fill.to_oklch().l);
        assert!(colors.background.pressed.to_oklch().l < blue().fill.to_oklch().l);
        assert_eq!(colors.content.normal, Color::WHITE);
        // Disabled: mostly the disabled color, a little of the family.
        let disabled = colors.background.disabled;
        assert_ne!(disabled, blue().fill);
        assert_ne!(disabled, base().background.disabled);
    }

    #[test]
    fn the_default_variant_takes_the_subtle_background_and_text_color() {
        let colors = hued(Variant::Default, &base(), blue(), SHADES);
        assert_eq!(colors.background.normal, blue().subtle);
        assert_eq!(colors.content.normal, blue().text);
        assert_eq!(colors.border.normal, blue().text);
        // Selected, it fills with the family's color.
        assert_eq!(colors.background.selected, blue().fill);
        assert_eq!(colors.content.selected, Color::WHITE);
    }

    #[test]
    fn tones_combine_variants_and_families() {
        let tone = Tone::POSITIVE.with_hue(Hue::Orange);
        assert_eq!(
            (tone.variant, tone.hue),
            (Variant::Positive, Some(Hue::Orange))
        );
        assert!(Variant::ALL.iter().enumerate().all(|(i, v)| v.index() == i));
        assert!(Hue::ALL.iter().enumerate().all(|(i, h)| h.index() == i));
        assert!(!Variant::Default.is_filled() && Variant::Notice.is_filled());
    }
}
