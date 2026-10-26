//! Capture fields, which record keyboard shortcuts.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group CAPTURE_FIELD: Component, start = FRAME::END;

    /// Capture field: background color.
    CAPTURE_FIELD_BACKGROUND_COLOR_DEFAULT = "capture-field.background.color.default": Color ->
        semantic::BACKGROUND_RAISED_DEFAULT;
    /// Capture field: background color, while recording a shortcut.
    CAPTURE_FIELD_BACKGROUND_COLOR_RECORDING = "capture-field.background.color.recording": Color ->
        semantic::BACKGROUND_SUNKEN_DEFAULT;
    /// Capture field: background color, hovered.
    CAPTURE_FIELD_BACKGROUND_COLOR_HOVER = "capture-field.background.color.hover": Color ->
        semantic::BACKGROUND_SUNKEN_DEFAULT;
    /// Capture field: background color, pressed.
    CAPTURE_FIELD_BACKGROUND_COLOR_PRESSED = "capture-field.background.color.pressed": Color ->
        semantic::BACKGROUND_RAISED_DEFAULT;
    /// Capture field: border color.
    CAPTURE_FIELD_BORDER_COLOR_DEFAULT = "capture-field.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Capture field: border color, while recording a shortcut.
    CAPTURE_FIELD_BORDER_COLOR_RECORDING = "capture-field.border.color.recording": Color ->
        semantic::BACKGROUND_NEGATIVE_DEFAULT;
    /// Capture field: label color.
    CAPTURE_FIELD_LABEL_COLOR_DEFAULT = "capture-field.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Capture field: label color, hint.
    CAPTURE_FIELD_LABEL_COLOR_HINT = "capture-field.label.color.hint": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Capture field: corner radius.
    CAPTURE_FIELD_CORNER_RADIUS_DEFAULT = "capture-field.corner-radius.default": Dimension ->
        semantic::RADIUS_XS;
    /// Capture field: horizontal padding.
    CAPTURE_FIELD_PADDING_HORIZONTAL_DEFAULT =
        "capture-field.padding.horizontal.default": Dimension = px(8.0), range(0.0, 32.0);
    /// Capture field: vertical padding.
    CAPTURE_FIELD_PADDING_VERTICAL_DEFAULT = "capture-field.padding.vertical.default": Dimension =
        px(4.0), range(0.0, 32.0);
    /// Capture field: minimum width.
    CAPTURE_FIELD_SIZE_MIN_WIDTH = "capture-field.size.min-width": Dimension =
        px(180.0), range(80.0, 600.0);
    /// Capture field: height.
    CAPTURE_FIELD_SIZE_HEIGHT = "capture-field.size.height": Dimension =
        px(28.0), range(20.0, 64.0);
}
