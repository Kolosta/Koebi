//! Toggle buttons.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group TOGGLE: Component, start = ICON::END;

    /// Toggle: background color.
    TOGGLE_BACKGROUND_COLOR_DEFAULT = "toggle.background.color.default": Color ->
        semantic::BACKGROUND_APP_FRAME;
    /// Toggle: background color, hovered.
    TOGGLE_BACKGROUND_COLOR_HOVER = "toggle.background.color.hover": Color ->
        semantic::BACKGROUND_SUNKEN_DEFAULT;
    /// Toggle: background color, selected.
    TOGGLE_BACKGROUND_COLOR_SELECTED = "toggle.background.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Toggle: border color.
    TOGGLE_BORDER_COLOR_DEFAULT = "toggle.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Toggle: border color, selected.
    TOGGLE_BORDER_COLOR_SELECTED = "toggle.border.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Toggle: label color.
    TOGGLE_LABEL_COLOR_DEFAULT = "toggle.label.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Toggle: label color, selected.
    TOGGLE_LABEL_COLOR_SELECTED = "toggle.label.color.selected": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Toggle: corner radius.
    TOGGLE_CORNER_RADIUS_DEFAULT = "toggle.corner-radius.default": Dimension -> semantic::RADIUS_S;
    /// Toggle: border width.
    TOGGLE_BORDER_WIDTH_DEFAULT = "toggle.border.width.default": Dimension =
        px(1.0), range(0.0, 4.0);
}
