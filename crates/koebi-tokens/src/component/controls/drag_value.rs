//! Drag values: number fields changed by dragging.

use super::*;
use crate::semantic;

koebi_design::tokens! {
    group DRAG_VALUE: Component, start = SLIDER::END;

    /// Drag value: background color.
    DRAG_VALUE_BACKGROUND_COLOR_DEFAULT = "drag-value.background.color.default": Color ->
        semantic::BACKGROUND_APP_FRAME;
    /// Drag value: background color, hovered.
    DRAG_VALUE_BACKGROUND_COLOR_HOVER = "drag-value.background.color.hover": Color ->
        semantic::BACKGROUND_APP_FRAME_HOVER;
    /// Drag value: background color, pressed.
    DRAG_VALUE_BACKGROUND_COLOR_PRESSED = "drag-value.background.color.pressed": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Drag value: background color, while dragging.
    DRAG_VALUE_BACKGROUND_COLOR_DRAG = "drag-value.background.color.drag": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Drag value: text color.
    DRAG_VALUE_TEXT_COLOR_DEFAULT = "drag-value.text.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Drag value: unit color.
    DRAG_VALUE_UNIT_COLOR_DEFAULT = "drag-value.unit.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Drag value: step button color.
    DRAG_VALUE_STEP_BUTTON_COLOR_DEFAULT = "drag-value.step-button.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Drag value: step button background color, hovered.
    DRAG_VALUE_STEP_BUTTON_BACKGROUND_COLOR_HOVER =
        "drag-value.step-button.background.color.hover": Color -> semantic::BACKGROUND_APP_CONTROL;
    /// Drag value: border color.
    DRAG_VALUE_BORDER_COLOR_DEFAULT = "drag-value.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Drag value: corner radius.
    DRAG_VALUE_CORNER_RADIUS_DEFAULT = "drag-value.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
    /// Drag value: border width.
    DRAG_VALUE_BORDER_WIDTH_DEFAULT = "drag-value.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Drag value: fill showing where the value lies in its range.
    DRAG_VALUE_FILL_COLOR_DEFAULT = "drag-value.fill.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
}
