//! Buttons that show only an icon.

use super::*;
use crate::semantic;

koebi_design::tokens! {
    group ICON_BUTTON: Component, start = BUTTON::END;

    /// Icon button: background color.
    ICON_BUTTON_BACKGROUND_COLOR_DEFAULT = "icon-button.background.color.default": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Icon button: background color, hovered.
    ICON_BUTTON_BACKGROUND_COLOR_HOVER = "icon-button.background.color.hover": Color ->
        semantic::BACKGROUND_SURFACE_CANVAS;
    /// Icon button: background color, pressed.
    ICON_BUTTON_BACKGROUND_COLOR_PRESSED = "icon-button.background.color.pressed": Color ->
        semantic::BACKGROUND_SURFACE_RAISED;
    /// Icon button: border color.
    ICON_BUTTON_BORDER_COLOR_DEFAULT = "icon-button.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Icon button: icon color.
    ICON_BUTTON_ICON_COLOR_DEFAULT = "icon-button.icon.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Icon button: icon color negative.
    ICON_BUTTON_ICON_COLOR_NEGATIVE = "icon-button.icon.color.negative": Color ->
        semantic::BACKGROUND_NEGATIVE_DEFAULT;
    /// Icon button: corner radius.
    ICON_BUTTON_CORNER_RADIUS_DEFAULT = "icon-button.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
}
