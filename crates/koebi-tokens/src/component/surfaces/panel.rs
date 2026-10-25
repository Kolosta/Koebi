//! Panels.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group PANEL: Component, start = CHILD::END;

    /// Panel: header background color.
    PANEL_HEADER_BACKGROUND_COLOR_DEFAULT = "panel.header.background.color.default": Color ->
        semantic::BACKGROUND_SURFACE_RAISED;
    /// Panel: body background color level 1.
    PANEL_BODY_BACKGROUND_COLOR_LEVEL_1 = "panel.body.background.color.level-1": Color ->
        semantic::BACKGROUND_SURFACE_RAISED;
    /// Panel: body background color level 2.
    PANEL_BODY_BACKGROUND_COLOR_LEVEL_2 = "panel.body.background.color.level-2": Color ->
        semantic::BACKGROUND_SURFACE_CANVAS;
    /// Panel: body background color level 3.
    PANEL_BODY_BACKGROUND_COLOR_LEVEL_3 = "panel.body.background.color.level-3": Color ->
        semantic::BACKGROUND_BASE_DEFAULT;
    /// Panel: border color.
    PANEL_BORDER_COLOR_DEFAULT = "panel.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Panel: label color.
    PANEL_LABEL_COLOR_DEFAULT = "panel.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Panel: override badge color.
    PANEL_OVERRIDE_BADGE_COLOR_DEFAULT = "panel.override-badge.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Panel: gap size.
    PANEL_GAP_SIZE_DEFAULT = "panel.gap.size.default": Dimension = px(2.0), range(0.0, 20.0);
    /// Panel: corner radius.
    PANEL_CORNER_RADIUS_DEFAULT = "panel.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
    /// Panel: border width.
    PANEL_BORDER_WIDTH_DEFAULT = "panel.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Panel: space between the border and the content.
    PANEL_PADDING_DEFAULT = "panel.padding.default": Dimension = px(8.0), range(0.0, 32.0);
}
