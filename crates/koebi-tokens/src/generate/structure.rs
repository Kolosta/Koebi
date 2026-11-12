//! The structure of a theme as data: which step each surface, text and icon takes in a light
//! theme ([`LIGHT`], what Snow and Arctic share), and the fills of each role whose states and
//! content the rules choose ([`ROLE_FILLS`]). A dark theme's structure is the base theme's own
//! definitions of the same tokens. States are offsets from their fill (rule 8), not aliases of
//! their own here: one and two steps toward the background in a dark theme, away from it in a
//! light one.

use koebi_design::TokenId;

use crate::{component, primitive, semantic};

/// The aliases a light theme gives to surfaces, text, icons, border widths and the neutral
/// fill: Snow's and Arctic's (`tests/generate.rs` checks both). A dark theme's are the base
/// definitions of the same tokens.
pub const LIGHT: [(TokenId, TokenId); 27] = [
    // Surfaces.
    (
        semantic::BACKGROUND_SUNKEN_DEFAULT.id(),
        primitive::COLOR_GRAY_100.id(),
    ),
    (
        semantic::BACKGROUND_RAISED_DEFAULT.id(),
        primitive::COLOR_GRAY_25.id(),
    ),
    (
        semantic::BACKGROUND_FLOATING_DEFAULT.id(),
        primitive::COLOR_GRAY_25.id(),
    ),
    (
        semantic::BACKGROUND_FLOATING_HOVER.id(),
        primitive::COLOR_GRAY_100.id(),
    ),
    (
        semantic::BACKGROUND_SURFACE_CANVAS.id(),
        primitive::COLOR_GRAY_75.id(),
    ),
    (
        semantic::BACKGROUND_SURFACE_RAISED.id(),
        primitive::COLOR_GRAY_25.id(),
    ),
    (
        semantic::BACKGROUND_APP_CONTROL.id(),
        primitive::COLOR_GRAY_75.id(),
    ),
    (
        semantic::BACKGROUND_APP_CHILD.id(),
        primitive::COLOR_GRAY_50.id(),
    ),
    (
        semantic::BACKGROUND_APP_FRAME.id(),
        primitive::COLOR_GRAY_25.id(),
    ),
    (
        semantic::BACKGROUND_APP_FRAME_HOVER.id(),
        primitive::COLOR_GRAY_75.id(),
    ),
    (
        semantic::BACKGROUND_APP_DISABLED.id(),
        primitive::COLOR_GRAY_100.id(),
    ),
    (
        semantic::BACKGROUND_MENU_BAR_DEFAULT.id(),
        primitive::COLOR_GRAY_75.id(),
    ),
    (
        semantic::BACKGROUND_TITLE_DEFAULT.id(),
        primitive::COLOR_GRAY_75.id(),
    ),
    (
        semantic::BACKGROUND_TITLE_COLLAPSED.id(),
        primitive::COLOR_GRAY_75.id(),
    ),
    (
        semantic::BACKGROUND_SCROLLBAR_TRACK_DEFAULT.id(),
        primitive::COLOR_GRAY_75.id(),
    ),
    (
        semantic::BACKGROUND_DOCKING_EMPTY_DEFAULT.id(),
        primitive::COLOR_GRAY_100.id(),
    ),
    // Borders: light surfaces lie close together, outlines tell them apart.
    (
        semantic::BORDER_WIDTH_CONTROL.id(),
        semantic::BORDER_WIDTH_THIN.id(),
    ),
    (
        semantic::BORDER_WIDTH_FLOATING.id(),
        semantic::BORDER_WIDTH_THIN.id(),
    ),
    (
        semantic::BORDER_WIDTH_SURFACE.id(),
        semantic::BORDER_WIDTH_THIN.id(),
    ),
    // Text and icons.
    (
        semantic::TEXT_COLOR_SECONDARY_DEFAULT.id(),
        primitive::COLOR_GRAY_600.id(),
    ),
    (
        semantic::TEXT_COLOR_TERTIARY_DEFAULT.id(),
        primitive::COLOR_GRAY_500.id(),
    ),
    (
        semantic::TEXT_COLOR_PLACEHOLDER_DEFAULT.id(),
        primitive::COLOR_GRAY_500.id(),
    ),
    (
        semantic::TEXT_COLOR_DISABLED_DEFAULT.id(),
        primitive::COLOR_GRAY_400.id(),
    ),
    (
        semantic::ICON_COLOR_SECONDARY_DEFAULT.id(),
        primitive::COLOR_GRAY_600.id(),
    ),
    (
        semantic::ICON_COLOR_TERTIARY_DEFAULT.id(),
        primitive::COLOR_GRAY_500.id(),
    ),
    (
        semantic::ICON_COLOR_DISABLED_DEFAULT.id(),
        primitive::COLOR_GRAY_400.id(),
    ),
    // The neutral fill: a step further from a light background, whose gray holds content.
    (
        semantic::BACKGROUND_NEUTRAL_DEFAULT.id(),
        semantic::NEUTRAL_COLOR_500.id(),
    ),
];

/// A role's fills, as the rules see them: its steps, the default fill, the tokens taking the
/// hover step (hover, keyboard focus, recording) and the pressed step, and the content on them.
#[derive(Clone, Copy, Debug)]
pub struct RoleFills {
    /// The role's key: `accent`, `info`, `positive`, `notice`, `negative` or `neutral`.
    pub key: &'static str,
    /// The role's 16 steps, from the closest to the background.
    pub steps: [TokenId; 16],
    /// The default fill.
    pub default: TokenId,
    /// The tokens of the hover step.
    pub hover: &'static [TokenId],
    /// The tokens of the pressed step.
    pub pressed: &'static [TokenId],
    /// The content drawn on the fills.
    pub content: TokenId,
}

/// The steps of a hue role (`semantic.<role>.color.<step>`).
macro_rules! role_steps {
    ($($step:ident),+ $(,)?) => {
        [$(semantic::$step.id()),+]
    };
}

/// The fills of every role, the accent first.
pub const ROLE_FILLS: [RoleFills; 6] = [
    RoleFills {
        key: "accent",
        steps: role_steps!(
            ACCENT_COLOR_100,
            ACCENT_COLOR_200,
            ACCENT_COLOR_300,
            ACCENT_COLOR_400,
            ACCENT_COLOR_500,
            ACCENT_COLOR_600,
            ACCENT_COLOR_700,
            ACCENT_COLOR_800,
            ACCENT_COLOR_900,
            ACCENT_COLOR_1000,
            ACCENT_COLOR_1100,
            ACCENT_COLOR_1200,
            ACCENT_COLOR_1300,
            ACCENT_COLOR_1400,
            ACCENT_COLOR_1500,
            ACCENT_COLOR_1600,
        ),
        default: semantic::BACKGROUND_ACCENT_DEFAULT.id(),
        hover: &[
            semantic::BACKGROUND_ACCENT_HOVER.id(),
            semantic::BACKGROUND_ACCENT_KBD_FOCUS.id(),
        ],
        pressed: &[semantic::BACKGROUND_ACCENT_PRESSED.id()],
        content: semantic::TEXT_COLOR_ON_ACCENT_DEFAULT.id(),
    },
    RoleFills {
        key: "info",
        steps: role_steps!(
            INFO_COLOR_100,
            INFO_COLOR_200,
            INFO_COLOR_300,
            INFO_COLOR_400,
            INFO_COLOR_500,
            INFO_COLOR_600,
            INFO_COLOR_700,
            INFO_COLOR_800,
            INFO_COLOR_900,
            INFO_COLOR_1000,
            INFO_COLOR_1100,
            INFO_COLOR_1200,
            INFO_COLOR_1300,
            INFO_COLOR_1400,
            INFO_COLOR_1500,
            INFO_COLOR_1600,
        ),
        default: semantic::BACKGROUND_INFO_DEFAULT.id(),
        hover: &[
            semantic::BACKGROUND_INFO_HOVER.id(),
            semantic::BACKGROUND_INFO_KBD_FOCUS.id(),
        ],
        pressed: &[semantic::BACKGROUND_INFO_PRESSED.id()],
        content: semantic::TEXT_COLOR_ON_INFO_DEFAULT.id(),
    },
    RoleFills {
        key: "positive",
        steps: role_steps!(
            POSITIVE_COLOR_100,
            POSITIVE_COLOR_200,
            POSITIVE_COLOR_300,
            POSITIVE_COLOR_400,
            POSITIVE_COLOR_500,
            POSITIVE_COLOR_600,
            POSITIVE_COLOR_700,
            POSITIVE_COLOR_800,
            POSITIVE_COLOR_900,
            POSITIVE_COLOR_1000,
            POSITIVE_COLOR_1100,
            POSITIVE_COLOR_1200,
            POSITIVE_COLOR_1300,
            POSITIVE_COLOR_1400,
            POSITIVE_COLOR_1500,
            POSITIVE_COLOR_1600,
        ),
        default: semantic::BACKGROUND_POSITIVE_DEFAULT.id(),
        hover: &[
            semantic::BACKGROUND_POSITIVE_HOVER.id(),
            semantic::BACKGROUND_POSITIVE_KBD_FOCUS.id(),
        ],
        pressed: &[semantic::BACKGROUND_POSITIVE_PRESSED.id()],
        content: semantic::TEXT_COLOR_ON_POSITIVE_DEFAULT.id(),
    },
    RoleFills {
        key: "notice",
        steps: role_steps!(
            NOTICE_COLOR_100,
            NOTICE_COLOR_200,
            NOTICE_COLOR_300,
            NOTICE_COLOR_400,
            NOTICE_COLOR_500,
            NOTICE_COLOR_600,
            NOTICE_COLOR_700,
            NOTICE_COLOR_800,
            NOTICE_COLOR_900,
            NOTICE_COLOR_1000,
            NOTICE_COLOR_1100,
            NOTICE_COLOR_1200,
            NOTICE_COLOR_1300,
            NOTICE_COLOR_1400,
            NOTICE_COLOR_1500,
            NOTICE_COLOR_1600,
        ),
        default: semantic::BACKGROUND_NOTICE_DEFAULT.id(),
        hover: &[component::BUTTON_NOTICE_BACKGROUND_COLOR_HOVER.id()],
        pressed: &[component::BUTTON_NOTICE_BACKGROUND_COLOR_PRESSED.id()],
        content: semantic::TEXT_COLOR_ON_NOTICE_DEFAULT.id(),
    },
    RoleFills {
        key: "negative",
        steps: role_steps!(
            NEGATIVE_COLOR_100,
            NEGATIVE_COLOR_200,
            NEGATIVE_COLOR_300,
            NEGATIVE_COLOR_400,
            NEGATIVE_COLOR_500,
            NEGATIVE_COLOR_600,
            NEGATIVE_COLOR_700,
            NEGATIVE_COLOR_800,
            NEGATIVE_COLOR_900,
            NEGATIVE_COLOR_1000,
            NEGATIVE_COLOR_1100,
            NEGATIVE_COLOR_1200,
            NEGATIVE_COLOR_1300,
            NEGATIVE_COLOR_1400,
            NEGATIVE_COLOR_1500,
            NEGATIVE_COLOR_1600,
        ),
        default: semantic::BACKGROUND_NEGATIVE_DEFAULT.id(),
        hover: &[
            semantic::BACKGROUND_NEGATIVE_HOVER.id(),
            semantic::BACKGROUND_NEGATIVE_KBD_FOCUS.id(),
            semantic::BACKGROUND_NEGATIVE_RECORDING.id(),
        ],
        pressed: &[semantic::BACKGROUND_NEGATIVE_PRESSED.id()],
        content: semantic::TEXT_COLOR_ON_NEGATIVE_DEFAULT.id(),
    },
    RoleFills {
        key: "neutral",
        steps: role_steps!(
            NEUTRAL_COLOR_25,
            NEUTRAL_COLOR_50,
            NEUTRAL_COLOR_75,
            NEUTRAL_COLOR_100,
            NEUTRAL_COLOR_150,
            NEUTRAL_COLOR_200,
            NEUTRAL_COLOR_250,
            NEUTRAL_COLOR_300,
            NEUTRAL_COLOR_350,
            NEUTRAL_COLOR_400,
            NEUTRAL_COLOR_500,
            NEUTRAL_COLOR_600,
            NEUTRAL_COLOR_700,
            NEUTRAL_COLOR_800,
            NEUTRAL_COLOR_900,
            NEUTRAL_COLOR_1000,
        ),
        default: semantic::BACKGROUND_NEUTRAL_DEFAULT.id(),
        hover: &[
            semantic::BACKGROUND_NEUTRAL_HOVER.id(),
            semantic::BACKGROUND_NEUTRAL_KBD_FOCUS.id(),
        ],
        pressed: &[semantic::BACKGROUND_NEUTRAL_PRESSED.id()],
        content: semantic::TEXT_COLOR_ON_NEUTRAL_DEFAULT.id(),
    },
];
