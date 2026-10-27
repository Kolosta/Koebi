//! Dropdowns: a control showing the current choice that opens a menu of choices.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group DROPDOWN: Component, start = DRAG_VALUE::END;

    /// Dropdown: height.
    DROPDOWN_SIZE_HEIGHT = "dropdown.size.height": Dimension -> semantic::SIZE_CONTROL_HEIGHT;
    /// Dropdown: horizontal padding.
    DROPDOWN_PADDING_HORIZONTAL_DEFAULT = "dropdown.padding.horizontal.default": Dimension =
        px(8.0), range(0.0, 32.0);
    /// Dropdown: vertical padding.
    DROPDOWN_PADDING_VERTICAL_DEFAULT = "dropdown.padding.vertical.default": Dimension = px(2.0), range(0.0, 32.0);
    /// Dropdown: chevron size.
    DROPDOWN_CHEVRON_SIZE = "dropdown.chevron.size": Dimension = px(8.0), range(0.0, 32.0);
    /// Dropdown: icon size.
    DROPDOWN_ICON_SIZE = "dropdown.icon.size": Dimension = px(16.0), range(0.0, 64.0);
    /// Dropdown: background color.
    DROPDOWN_BACKGROUND_COLOR_DEFAULT = "dropdown.background.color.default": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Dropdown: background color, hovered.
    DROPDOWN_BACKGROUND_COLOR_HOVER = "dropdown.background.color.hover": Color ->
        semantic::BACKGROUND_SURFACE_CANVAS;
    /// Dropdown: background color, pressed.
    DROPDOWN_BACKGROUND_COLOR_PRESSED = "dropdown.background.color.pressed": Color ->
        semantic::BACKGROUND_SURFACE_RAISED;
    /// Dropdown: background color, open.
    DROPDOWN_BACKGROUND_COLOR_OPEN = "dropdown.background.color.open": Color ->
        semantic::BACKGROUND_APP_FRAME_HOVER;
    /// Dropdown: background color, hovered (minimal style).
    DROPDOWN_BACKGROUND_COLOR_HOVER_MINIMAL = "dropdown.background.color.hover-minimal": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Dropdown: label color.
    DROPDOWN_LABEL_COLOR_DEFAULT = "dropdown.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Dropdown: icon color.
    DROPDOWN_ICON_COLOR_DEFAULT = "dropdown.icon.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Dropdown: corner radius.
    DROPDOWN_CORNER_RADIUS_DEFAULT = "dropdown.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
    /// Dropdown: border color.
    DROPDOWN_BORDER_COLOR_DEFAULT = "dropdown.border.color.default": Color ->
        semantic::NEUTRAL_COLOR_700;
    /// Dropdown: border color, hovered.
    DROPDOWN_BORDER_COLOR_HOVER = "dropdown.border.color.hover": Color ->
        semantic::NEUTRAL_COLOR_600;
    /// Dropdown: border width.
    DROPDOWN_BORDER_WIDTH_DEFAULT = "dropdown.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;

    /// Dropdown: background color, disabled.
    DROPDOWN_BACKGROUND_COLOR_DISABLED = "dropdown.background.color.disabled": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Dropdown: border color, open.
    DROPDOWN_BORDER_COLOR_OPEN = "dropdown.border.color.open": Color ->
        semantic::NEUTRAL_COLOR_600;
    /// Dropdown: border color, disabled.
    DROPDOWN_BORDER_COLOR_DISABLED = "dropdown.border.color.disabled": Color ->
        semantic::BORDER_COLOR_DISABLED_DEFAULT;
    /// Dropdown: label color, disabled.
    DROPDOWN_LABEL_COLOR_DISABLED = "dropdown.label.color.disabled": Color ->
        semantic::TEXT_COLOR_DISABLED_DEFAULT;
    /// Dropdown: icon color, disabled.
    DROPDOWN_ICON_COLOR_DISABLED = "dropdown.icon.color.disabled": Color ->
        semantic::TEXT_COLOR_DISABLED_DEFAULT;
    /// Dropdown: space between the parts of its content (icon, label, chevron).
    DROPDOWN_GAP_DEFAULT = "dropdown.gap.default": Dimension = px(4.0), range(0.0, 24.0);
    /// Dropdown: smallest width of a dropdown fitting its content.
    DROPDOWN_MIN_WIDTH_DEFAULT = "dropdown.min-width.default": Dimension ->
        semantic::SIZE_CONTROL_HEIGHT;
}
