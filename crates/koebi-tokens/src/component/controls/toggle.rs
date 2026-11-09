//! Toggle buttons: buttons of the default tone that switch a value on and off
//! (`Button::toggle`).

use super::*;
use crate::semantic;

koebi_design::tokens! {
    group TOGGLE: Component, start = ICON::END;

    /// Toggle: background color.
    TOGGLE_BACKGROUND_COLOR_DEFAULT = "toggle.background.color.default": Color ->
        semantic::BACKGROUND_APP_FRAME;
    /// Toggle: background color, hovered.
    TOGGLE_BACKGROUND_COLOR_HOVER = "toggle.background.color.hover": Color ->
        semantic::BACKGROUND_APP_FRAME_HOVER;
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
    /// Toggle: border width, the theme's thin border like the button groups beside toggles.
    TOGGLE_BORDER_WIDTH_DEFAULT = "toggle.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;

    /// Toggle: background color, pressed.
    TOGGLE_BACKGROUND_COLOR_PRESSED = "toggle.background.color.pressed": Color ->
        semantic::BACKGROUND_APP_CHILD;
    /// Toggle: background color, selected and hovered.
    TOGGLE_BACKGROUND_COLOR_SELECTED_HOVER = "toggle.background.color.selected-hover": Color ->
        semantic::BACKGROUND_ACCENT_HOVER;
    /// Toggle: background color, selected and pressed.
    TOGGLE_BACKGROUND_COLOR_SELECTED_PRESSED = "toggle.background.color.selected-pressed":
        Color -> semantic::ACCENT_COLOR_800;
    /// Toggle: background color, disabled.
    TOGGLE_BACKGROUND_COLOR_DISABLED = "toggle.background.color.disabled": Color ->
        semantic::BACKGROUND_APP_DISABLED;
    /// Toggle: border color, hovered or pressed.
    TOGGLE_BORDER_COLOR_HOVER = "toggle.border.color.hover": Color ->
        semantic::BORDER_COLOR_STRONG_DEFAULT;
    /// Toggle: border color, disabled.
    TOGGLE_BORDER_COLOR_DISABLED = "toggle.border.color.disabled": Color ->
        semantic::BORDER_COLOR_DISABLED_DEFAULT;
    /// Toggle: label color, hovered or pressed.
    TOGGLE_LABEL_COLOR_HOVER = "toggle.label.color.hover": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Toggle: label color, disabled.
    TOGGLE_LABEL_COLOR_DISABLED = "toggle.label.color.disabled": Color ->
        semantic::TEXT_COLOR_DISABLED_DEFAULT;
}
