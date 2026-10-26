//! Zone tabs.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group ZONE_TAB: Component, start = ZONE::END;

    /// Zone tab: gap.
    ZONE_TAB_GAP_DEFAULT = "zone-tab.gap.default": Dimension = px(2.0), range(0.0, 24.0);
    /// Zone tab: horizontal padding.
    ZONE_TAB_PADDING_HORIZONTAL_DEFAULT = "zone-tab.padding.horizontal.default": Dimension =
        px(8.0), range(0.0, 32.0);
    /// Zone tab: vertical padding.
    ZONE_TAB_PADDING_VERTICAL_DEFAULT = "zone-tab.padding.vertical.default": Dimension = px(2.0), range(0.0, 32.0);
    /// Zone tab: background color.
    ZONE_TAB_BACKGROUND_COLOR_DEFAULT = "zone-tab.background.color.default": Color ->
        semantic::BACKGROUND_SURFACE_CANVAS;
    /// Zone tab: background color, active.
    ZONE_TAB_BACKGROUND_COLOR_ACTIVE = "zone-tab.background.color.active": Color ->
        EDITOR_TOP_BAR_BACKGROUND_COLOR_DEFAULT;
    /// Zone tab: background color, hovered.
    ZONE_TAB_BACKGROUND_COLOR_HOVER = "zone-tab.background.color.hover": Color ->
        semantic::BACKGROUND_SURFACE_RAISED;
    /// Zone tab: label color.
    ZONE_TAB_LABEL_COLOR_DEFAULT = "zone-tab.label.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Zone tab: label color, active.
    ZONE_TAB_LABEL_COLOR_ACTIVE = "zone-tab.label.color.active": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Zone tab: bar background color.
    ZONE_TAB_BAR_BACKGROUND_COLOR_DEFAULT = "zone-tab.bar.background.color.default": Color ->
        semantic::BACKGROUND_SURFACE_CANVAS;
    /// Zone tab: insert line color.
    ZONE_TAB_INSERT_LINE_COLOR_DEFAULT = "zone-tab.insert-line.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Zone tab: insert line width.
    ZONE_TAB_INSERT_LINE_WIDTH_DEFAULT = "zone-tab.insert-line.width.default": Dimension =
        px(2.0), range(1.0, 6.0);
    /// Zone tab: drop preview color.
    ZONE_TAB_DROP_PREVIEW_COLOR_DEFAULT = "zone-tab.drop-preview.color.default": Color =
        hex("#FFFFFF2E");
    /// Zone tab: drop preview center inset.
    ZONE_TAB_DROP_PREVIEW_CENTER_INSET = "zone-tab.drop-preview.center-inset": Dimension =
        px(6.0), range(0.0, 32.0);
    /// Zone tab: drop preview anim duration.
    ZONE_TAB_DROP_PREVIEW_ANIM_DURATION = "zone-tab.drop-preview.anim-duration": Duration =
        ms(60), range(0.0, 1000.0);
    /// Zone tab: drag threshold.
    ZONE_TAB_DRAG_THRESHOLD = "zone-tab.drag.threshold": Dimension = px(5.0), range(1.0, 20.0);
    /// Zone tab: show solo.
    ZONE_TAB_SHOW_SOLO = "zone-tab.show-solo": Integer = 0, range(0.0, 1.0);
}
