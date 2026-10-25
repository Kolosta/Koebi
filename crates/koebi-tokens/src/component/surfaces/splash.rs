//! The splash screen.

use super::*;
use crate::semantic;

koebi_design::tokens! {
    group SPLASH: Component, start = PREFERENCES_TITLE_BAR::END;

    /// Splash: background color.
    SPLASH_BACKGROUND_COLOR_DEFAULT = "splash.background.color.default": Color ->
        semantic::BACKGROUND_BASE_DEFAULT;
    /// Splash: link color.
    SPLASH_LINK_COLOR_DEFAULT = "splash.link.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Splash: version text color.
    SPLASH_VERSION_TEXT_COLOR_DEFAULT = "splash.version-text.color.default": Color ->
        semantic::NEUTRAL_COLOR_900;
}
