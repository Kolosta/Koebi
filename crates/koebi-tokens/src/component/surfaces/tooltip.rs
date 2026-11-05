//! Tooltips: help shown below a control once the pointer rests on it, standing out on every
//! surface.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group TOOLTIP: Component, start = MENU::END;

    /// Tooltip: background color, inverted so that it stands out on every surface and on
    /// menus.
    TOOLTIP_BACKGROUND_COLOR_DEFAULT = "tooltip.background.color.default": Color ->
        semantic::BACKGROUND_INVERSE_DEFAULT;
    /// Tooltip: label color.
    TOOLTIP_LABEL_COLOR_DEFAULT = "tooltip.label.color.default": Color ->
        semantic::TEXT_COLOR_INVERSE_DEFAULT;
    /// Tooltip: border color, the shadow's, marking its edge on light surfaces too.
    TOOLTIP_BORDER_COLOR_DEFAULT = "tooltip.border.color.default": Color ->
        semantic::BORDER_COLOR_SHADOW_DEFAULT;
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
    /// Tooltip: space between the control and the tooltip below (or above) it.
    TOOLTIP_OFFSET_DEFAULT = "tooltip.offset.default": Dimension = px(6.0), range(0.0, 64.0);
    /// Tooltip: space kept from the window's edges.
    TOOLTIP_MARGIN_DEFAULT = "tooltip.margin.default": Dimension = px(6.0), range(0.0, 32.0);
    /// Tooltip: widest its text goes before it wraps.
    TOOLTIP_MAX_WIDTH_DEFAULT = "tooltip.max-width.default": Dimension = px(360.0),
        range(80.0, 1200.0);
}
