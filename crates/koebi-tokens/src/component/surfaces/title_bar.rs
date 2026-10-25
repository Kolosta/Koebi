//! Window title bars.

use super::*;
use crate::semantic;

koebi_design::tokens! {
    group TITLE_BAR: Component, start = WINDOW::END;

    /// Title bar: background color.
    TITLE_BAR_BACKGROUND_COLOR_DEFAULT = "title-bar.background.color.default": Color ->
        semantic::BACKGROUND_BASE_DEFAULT;
    /// Title bar: icon color.
    TITLE_BAR_ICON_COLOR_DEFAULT = "title-bar.icon.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Title bar: label color.
    TITLE_BAR_LABEL_COLOR_DEFAULT = "title-bar.label.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Title bar: button background color, hovered.
    TITLE_BAR_BUTTON_BACKGROUND_COLOR_HOVER = "title-bar.button.background.color.hover": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Title bar: close background color, hovered.
    TITLE_BAR_CLOSE_BACKGROUND_COLOR_HOVER = "title-bar.close.background.color.hover": Color ->
        semantic::BACKGROUND_NEGATIVE_DEFAULT;
}
