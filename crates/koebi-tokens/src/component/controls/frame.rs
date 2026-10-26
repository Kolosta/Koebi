//! Frames: the backgrounds of input fields.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group FRAME: Component, start = COMBO::END;

    /// Frame: background color.
    FRAME_BACKGROUND_COLOR_DEFAULT = "frame.background.color.default": Color ->
        semantic::BACKGROUND_APP_FRAME;
    /// Frame: background color, hovered.
    FRAME_BACKGROUND_COLOR_HOVER = "frame.background.color.hover": Color ->
        semantic::BACKGROUND_APP_FRAME;
    /// Frame: background color, pressed.
    FRAME_BACKGROUND_COLOR_PRESSED = "frame.background.color.pressed": Color ->
        semantic::BACKGROUND_APP_CHILD;
    /// Frame: corner radius.
    FRAME_CORNER_RADIUS_DEFAULT = "frame.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
    /// Frame: border width.
    FRAME_BORDER_WIDTH_DEFAULT = "frame.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Frame: horizontal padding.
    FRAME_PADDING_HORIZONTAL_DEFAULT = "frame.padding.horizontal.default": Dimension = px(4.0), range(0.0, 32.0);
    /// Frame: vertical padding.
    FRAME_PADDING_VERTICAL_DEFAULT = "frame.padding.vertical.default": Dimension = px(4.0), range(0.0, 32.0);
    /// Frame: input text cursor color.
    FRAME_INPUT_TEXT_CURSOR_COLOR_DEFAULT = "frame.input-text.cursor.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
}
