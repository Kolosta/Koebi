//! Text fields: one line of text edited in place.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group TEXT_FIELD: Component, start = DRAG_VALUE::END;

    /// Text field: height.
    TEXT_FIELD_SIZE_HEIGHT = "text-field.size.height": Dimension -> semantic::SIZE_CONTROL_HEIGHT;
    /// Text field: smallest width.
    TEXT_FIELD_MIN_WIDTH_DEFAULT = "text-field.min-width.default": Dimension = px(48.0),
        range(0.0, 400.0);
    /// Text field: space between the sides and the text.
    TEXT_FIELD_PADDING_HORIZONTAL_DEFAULT = "text-field.padding.horizontal.default": Dimension =
        px(6.0), range(0.0, 32.0);
    /// Text field: space between the icon and the text.
    TEXT_FIELD_GAP_DEFAULT = "text-field.gap.default": Dimension = px(4.0), range(0.0, 24.0);
    /// Text field: icon size.
    TEXT_FIELD_ICON_SIZE = "text-field.icon.size": Dimension = px(14.0), range(0.0, 64.0);
    /// Text field: background color.
    TEXT_FIELD_BACKGROUND_COLOR_DEFAULT = "text-field.background.color.default": Color ->
        semantic::BACKGROUND_APP_FRAME;
    /// Text field: background color, hovered.
    TEXT_FIELD_BACKGROUND_COLOR_HOVER = "text-field.background.color.hover": Color ->
        semantic::BACKGROUND_APP_FRAME_HOVER;
    /// Text field: background color, while editing.
    TEXT_FIELD_BACKGROUND_COLOR_EDITING = "text-field.background.color.editing": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Text field: background color, disabled.
    TEXT_FIELD_BACKGROUND_COLOR_DISABLED = "text-field.background.color.disabled": Color ->
        semantic::BACKGROUND_APP_FRAME;
    /// Text field: border color.
    TEXT_FIELD_BORDER_COLOR_DEFAULT = "text-field.border.color.default": Color ->
        semantic::NEUTRAL_COLOR_700;
    /// Text field: border color, hovered.
    TEXT_FIELD_BORDER_COLOR_HOVER = "text-field.border.color.hover": Color ->
        semantic::NEUTRAL_COLOR_600;
    /// Text field: border color, while editing.
    TEXT_FIELD_BORDER_COLOR_EDITING = "text-field.border.color.editing": Color ->
        semantic::BORDER_COLOR_FOCUS_DEFAULT;
    /// Text field: border color, disabled.
    TEXT_FIELD_BORDER_COLOR_DISABLED = "text-field.border.color.disabled": Color ->
        semantic::BORDER_COLOR_DISABLED_DEFAULT;
    /// Text field: border width.
    TEXT_FIELD_BORDER_WIDTH_DEFAULT = "text-field.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Text field: corner radius.
    TEXT_FIELD_CORNER_RADIUS_DEFAULT = "text-field.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
    /// Text field: text color.
    TEXT_FIELD_TEXT_COLOR_DEFAULT = "text-field.text.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Text field: text color, disabled.
    TEXT_FIELD_TEXT_COLOR_DISABLED = "text-field.text.color.disabled": Color ->
        semantic::TEXT_COLOR_DISABLED_DEFAULT;
    /// Text field: color of the placeholder shown while the text is empty.
    TEXT_FIELD_PLACEHOLDER_COLOR_DEFAULT = "text-field.placeholder.color.default": Color ->
        semantic::TEXT_COLOR_PLACEHOLDER_DEFAULT;
    /// Text field: icon color.
    TEXT_FIELD_ICON_COLOR_DEFAULT = "text-field.icon.color.default": Color ->
        semantic::ICON_COLOR_SECONDARY_DEFAULT;
    /// Text field: icon color, disabled.
    TEXT_FIELD_ICON_COLOR_DISABLED = "text-field.icon.color.disabled": Color ->
        semantic::ICON_COLOR_DISABLED_DEFAULT;
    /// Text field: background of the selected text.
    TEXT_FIELD_SELECTION_COLOR_DEFAULT = "text-field.selection.color.default": Color ->
        semantic::BACKGROUND_TEXT_SELECTION_DEFAULT;
    /// Text field: caret color.
    TEXT_FIELD_CARET_COLOR_DEFAULT = "text-field.caret.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Text field: caret width.
    TEXT_FIELD_CARET_WIDTH = "text-field.caret.width": Dimension = px(1.0), range(0.0, 8.0);
    /// Text field: time the caret stays shown, then hidden, while blinking; 0 keeps it shown.
    TEXT_FIELD_CARET_BLINK_INTERVAL = "text-field.caret.blink-interval": Duration = ms(0),
        range(0.0, 2000.0);
}
