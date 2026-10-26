//! Tooltips.

use super::*;
use crate::component::EDITOR_TOP_BAR_BACKGROUND_COLOR_DEFAULT;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group TOOLTIP: Component, start = MENU::END;

    /// Tooltip: background color.
    TOOLTIP_BACKGROUND_COLOR_DEFAULT = "tooltip.background.color.default": Color ->
        EDITOR_TOP_BAR_BACKGROUND_COLOR_DEFAULT;
    /// Tooltip: label color.
    TOOLTIP_LABEL_COLOR_DEFAULT = "tooltip.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Tooltip: border color.
    TOOLTIP_BORDER_COLOR_DEFAULT = "tooltip.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Tooltip: border width.
    TOOLTIP_BORDER_WIDTH_DEFAULT = "tooltip.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Tooltip: corner radius.
    TOOLTIP_CORNER_RADIUS_DEFAULT = "tooltip.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
    /// Tooltip: horizontal padding.
    TOOLTIP_PADDING_HORIZONTAL_DEFAULT = "tooltip.padding.horizontal.default": Dimension = px(8.0), range(0.0, 32.0);
    /// Tooltip: vertical padding.
    TOOLTIP_PADDING_VERTICAL_DEFAULT = "tooltip.padding.vertical.default": Dimension = px(6.0), range(0.0, 32.0);
}
