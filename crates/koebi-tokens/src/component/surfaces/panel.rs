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
    /// Collapsible panel: height of a header.
    PANEL_HEADER_HEIGHT_DEFAULT = "panel.header.height.default": Dimension ->
        semantic::SIZE_CONTROL_HEIGHT;
    /// Collapsible panel: space between a header's sides and its content.
    PANEL_HEADER_PADDING_DEFAULT = "panel.header.padding.default": Dimension = px(6.0),
        range(0.0, 32.0);
    /// Collapsible panel: space between a header's chevron, icon and title.
    PANEL_HEADER_GAP_DEFAULT = "panel.header.gap.default": Dimension = px(4.0),
        range(0.0, 24.0);
    /// Collapsible panel: lightness added to a header under the pointer (OKLCH).
    PANEL_HEADER_HOVER_LIGHTNESS = "panel.header.hover.lightness": Number = 0.04,
        range(-0.5, 0.5);
    /// Collapsible panel: side of a header's chevron.
    PANEL_CHEVRON_SIZE = "panel.chevron.size": Dimension = px(10.0), range(0.0, 32.0);
    /// Collapsible panel: chevron color.
    PANEL_CHEVRON_COLOR_DEFAULT = "panel.chevron.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Collapsible panel: space above and below its content.
    PANEL_CONTENT_PADDING_VERTICAL = "panel.content.padding.vertical": Dimension = px(6.0),
        range(0.0, 32.0);
    /// Collapsible panel: duration of a panel opening.
    PANEL_EXPAND_DURATION = "panel.expand.duration": Duration ->
        semantic::ANIMATION_DURATION_EXPAND;
    /// Collapsible panel: duration of a panel closing.
    PANEL_COLLAPSE_DURATION = "panel.collapse.duration": Duration ->
        semantic::ANIMATION_DURATION_COLLAPSE;
    /// Collapsible panel: curve of a panel opening; values past 0 and 1 hold at the ends.
    PANEL_EXPAND_CURVE = "panel.expand.curve": Curve -> semantic::ANIMATION_CURVE_EXPAND;
    /// Collapsible panel: curve of a panel closing; values past 0 and 1 hold at the ends.
    PANEL_COLLAPSE_CURVE = "panel.collapse.curve": Curve -> semantic::ANIMATION_CURVE_COLLAPSE;
}
