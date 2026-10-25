//! The title bar of the preferences window.

use super::*;
use crate::semantic;

koebi_design::tokens! {
    group PREFERENCES_TITLE_BAR: Component, start = TITLE_BAR::END;

    /// Preferences title bar: background color.
    PREFERENCES_TITLE_BAR_BACKGROUND_COLOR_DEFAULT =
        "preferences-title-bar.background.color.default": Color ->
        semantic::BACKGROUND_BASE_DEFAULT;
    /// Preferences title bar: label color.
    PREFERENCES_TITLE_BAR_LABEL_COLOR_DEFAULT =
        "preferences-title-bar.label.color.default": Color -> semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Preferences title bar: icon color.
    PREFERENCES_TITLE_BAR_ICON_COLOR_DEFAULT = "preferences-title-bar.icon.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Preferences title bar: button background color, hovered.
    PREFERENCES_TITLE_BAR_BUTTON_BACKGROUND_COLOR_HOVER =
        "preferences-title-bar.button.background.color.hover": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Preferences title bar: close background color, hovered.
    PREFERENCES_TITLE_BAR_CLOSE_BACKGROUND_COLOR_HOVER =
        "preferences-title-bar.close.background.color.hover": Color ->
        semantic::BACKGROUND_NEGATIVE_DEFAULT;
}
