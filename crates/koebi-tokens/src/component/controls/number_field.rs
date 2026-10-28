//! Number fields: values changed by dragging, by step buttons, or typed as expressions.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group NUMBER_FIELD: Component, start = SLIDER::END;

    /// Number field: height.
    NUMBER_FIELD_SIZE_HEIGHT = "number-field.size.height": Dimension ->
        semantic::SIZE_CONTROL_HEIGHT;
    /// Number field: smallest width.
    NUMBER_FIELD_MIN_WIDTH_DEFAULT = "number-field.min-width.default": Dimension = px(64.0),
        range(0.0, 400.0);
    /// Number field: space between the step buttons (or the sides) and the text.
    NUMBER_FIELD_PADDING_HORIZONTAL_DEFAULT = "number-field.padding.horizontal.default":
        Dimension = px(4.0), range(0.0, 32.0);
    /// Number field: width of a step button (height in vertical fields).
    NUMBER_FIELD_STEP_BUTTON_WIDTH = "number-field.step-button.width": Dimension = px(16.0),
        range(0.0, 64.0);
    /// Number field: side of a step button's chevron.
    NUMBER_FIELD_STEP_BUTTON_ICON_SIZE = "number-field.step-button.icon.size": Dimension =
        px(10.0), range(0.0, 32.0);
    /// Number field: background color.
    NUMBER_FIELD_BACKGROUND_COLOR_DEFAULT = "number-field.background.color.default": Color ->
        semantic::BACKGROUND_APP_FRAME;
    /// Number field: background color, hovered.
    NUMBER_FIELD_BACKGROUND_COLOR_HOVER = "number-field.background.color.hover": Color ->
        semantic::BACKGROUND_APP_FRAME_HOVER;
    /// Number field: background color, pressed.
    NUMBER_FIELD_BACKGROUND_COLOR_PRESSED = "number-field.background.color.pressed": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Number field: background color, while dragging.
    NUMBER_FIELD_BACKGROUND_COLOR_DRAG = "number-field.background.color.drag": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Number field: background color, disabled.
    NUMBER_FIELD_BACKGROUND_COLOR_DISABLED = "number-field.background.color.disabled": Color ->
        semantic::BACKGROUND_APP_FRAME;
    /// Number field: value color.
    NUMBER_FIELD_TEXT_COLOR_DEFAULT = "number-field.text.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Number field: value color, disabled.
    NUMBER_FIELD_TEXT_COLOR_DISABLED = "number-field.text.color.disabled": Color ->
        semantic::TEXT_COLOR_DISABLED_DEFAULT;
    /// Number field: label color.
    NUMBER_FIELD_LABEL_COLOR_DEFAULT = "number-field.label.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Number field: unit color.
    NUMBER_FIELD_UNIT_COLOR_DEFAULT = "number-field.unit.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Number field: step button chevron color.
    NUMBER_FIELD_STEP_BUTTON_COLOR_DEFAULT = "number-field.step-button.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Number field: step button chevron color, hovered.
    NUMBER_FIELD_STEP_BUTTON_COLOR_HOVER = "number-field.step-button.color.hover": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Number field: step button background color, hovered.
    NUMBER_FIELD_STEP_BUTTON_BACKGROUND_COLOR_HOVER =
        "number-field.step-button.background.color.hover": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Number field: border color.
    NUMBER_FIELD_BORDER_COLOR_DEFAULT = "number-field.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Number field: border color, hovered.
    NUMBER_FIELD_BORDER_COLOR_HOVER = "number-field.border.color.hover": Color ->
        semantic::NEUTRAL_COLOR_600;
    /// Number field: border color, while dragging.
    NUMBER_FIELD_BORDER_COLOR_DRAG = "number-field.border.color.drag": Color ->
        semantic::BORDER_COLOR_FOCUS_DEFAULT;
    /// Number field: border color, disabled.
    NUMBER_FIELD_BORDER_COLOR_DISABLED = "number-field.border.color.disabled": Color ->
        semantic::BORDER_COLOR_DISABLED_DEFAULT;
    /// Number field: border color of a typed expression that is not valid.
    NUMBER_FIELD_BORDER_COLOR_ERROR = "number-field.border.color.error": Color ->
        semantic::BORDER_COLOR_NEGATIVE_FOCUS;
    /// Number field: border width.
    NUMBER_FIELD_BORDER_WIDTH_DEFAULT = "number-field.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Number field: corner radius.
    NUMBER_FIELD_CORNER_RADIUS_DEFAULT = "number-field.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
    /// Number field: bar showing where the value lies in its range.
    NUMBER_FIELD_FILL_COLOR_DEFAULT = "number-field.fill.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Number field: bar, hovered or dragged.
    NUMBER_FIELD_FILL_COLOR_HOVER = "number-field.fill.color.hover": Color ->
        semantic::BACKGROUND_ACCENT_HOVER;
    /// Number field: bar, disabled.
    NUMBER_FIELD_FILL_COLOR_DISABLED = "number-field.fill.color.disabled": Color ->
        semantic::NEUTRAL_COLOR_600;
    /// Number field: pointer travel that changes an unbounded value by one step.
    NUMBER_FIELD_DRAG_STEP_DISTANCE = "number-field.drag.step-distance": Dimension = px(8.0),
        range(1.0, 200.0);
    /// Number field: how much finer a drag or a step is with Shift.
    NUMBER_FIELD_DRAG_FINE_FACTOR = "number-field.drag.fine-factor": Number = 0.1,
        range(0.001, 1.0);
}
