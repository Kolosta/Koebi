//! Checkboxes.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group CHECKBOX: Component, start = TOGGLE::END;

    /// Checkbox: mark color.
    CHECKBOX_MARK_COLOR_DEFAULT = "checkbox.mark.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Checkbox: background color, selected.
    CHECKBOX_BACKGROUND_COLOR_SELECTED = "checkbox.background.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Checkbox: background color.
    CHECKBOX_BACKGROUND_COLOR_DEFAULT = "checkbox.background.color.default": Color ->
        semantic::BACKGROUND_APP_FRAME;
    /// Checkbox: background color, hovered.
    CHECKBOX_BACKGROUND_COLOR_HOVER = "checkbox.background.color.hover": Color ->
        semantic::BACKGROUND_APP_FRAME_HOVER;
    /// Checkbox: background color, pressed.
    CHECKBOX_BACKGROUND_COLOR_PRESSED = "checkbox.background.color.pressed": Color ->
        semantic::BACKGROUND_APP_CHILD;
    /// Checkbox: background color, selected and hovered.
    CHECKBOX_BACKGROUND_COLOR_SELECTED_HOVER = "checkbox.background.color.selected-hover": Color ->
        semantic::BACKGROUND_ACCENT_HOVER;
    /// Checkbox: background color, selected and pressed.
    CHECKBOX_BACKGROUND_COLOR_SELECTED_PRESSED =
        "checkbox.background.color.selected-pressed": Color -> semantic::BACKGROUND_ACCENT_PRESSED;
    /// Checkbox: border color.
    CHECKBOX_BORDER_COLOR_DEFAULT = "checkbox.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Checkbox: border color, selected.
    CHECKBOX_BORDER_COLOR_SELECTED = "checkbox.border.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Checkbox: box size.
    CHECKBOX_BOX_SIZE_DEFAULT = "checkbox.box-size.default": Dimension = px(16.0), range(0.0, 40.0);
    /// Checkbox: corner radius.
    CHECKBOX_CORNER_RADIUS_DEFAULT = "checkbox.corner-radius.default": Dimension ->
        semantic::RADIUS_S;
    /// Checkbox: border width.
    CHECKBOX_BORDER_WIDTH_DEFAULT = "checkbox.border.width.default": Dimension =
        px(1.0), range(0.0, 4.0);
    /// Checkbox: stroke width of the check mark.
    CHECKBOX_MARK_WIDTH_DEFAULT = "checkbox.mark.width.default": Dimension =
        px(2.0), range(0.5, 6.0);
}
