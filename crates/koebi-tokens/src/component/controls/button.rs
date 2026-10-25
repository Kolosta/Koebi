//! Push buttons.

use crate::semantic;

koebi_design::tokens! {
    group BUTTON: Component, start = semantic::TEXT::END;

    /// Button: background color.
    BUTTON_BACKGROUND_COLOR_DEFAULT = "button.background.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Button: background color, hovered.
    BUTTON_BACKGROUND_COLOR_HOVER = "button.background.color.hover": Color ->
        semantic::BACKGROUND_ACCENT_HOVER;
    /// Button: background color, pressed.
    BUTTON_BACKGROUND_COLOR_PRESSED = "button.background.color.pressed": Color ->
        semantic::BACKGROUND_ACCENT_PRESSED;
    /// Button: label color.
    BUTTON_LABEL_COLOR_DEFAULT = "button.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Button: border color.
    BUTTON_BORDER_COLOR_DEFAULT = "button.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Button: border width.
    BUTTON_BORDER_WIDTH_DEFAULT = "button.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_NONE;
    /// Button: corner radius.
    BUTTON_CORNER_RADIUS_DEFAULT = "button.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
}
