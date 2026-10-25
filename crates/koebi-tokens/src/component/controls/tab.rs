//! Tabs and tab bars.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group TAB: Component, start = SCROLLBAR::END;

    /// Tab: background color.
    TAB_BACKGROUND_COLOR_DEFAULT = "tab.background.color.default": Color ->
        semantic::BACKGROUND_BASE_DEFAULT;
    /// Tab: background color, hovered.
    TAB_BACKGROUND_COLOR_HOVER = "tab.background.color.hover": Color ->
        semantic::BACKGROUND_ACCENT_HOVER;
    /// Tab: background color, selected.
    TAB_BACKGROUND_COLOR_SELECTED = "tab.background.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Tab: background color, in an unfocused window.
    TAB_BACKGROUND_COLOR_DIMMED = "tab.background.color.dimmed": Color ->
        semantic::BACKGROUND_BASE_DEFAULT;
    /// Tab: background color, selected, in an unfocused window.
    TAB_BACKGROUND_COLOR_DIMMED_SELECTED = "tab.background.color.dimmed-selected": Color ->
        semantic::BACKGROUND_SUNKEN_DEFAULT;
    /// Tab: overline color, selected.
    TAB_OVERLINE_COLOR_SELECTED = "tab.overline.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Tab: overline color, in an unfocused window.
    TAB_OVERLINE_COLOR_DIMMED = "tab.overline.color.dimmed": Color = rgba_f32(0.5, 0.5, 0.5, 0.0);
    /// Tab: corner radius.
    TAB_CORNER_RADIUS_DEFAULT = "tab.corner-radius.default": Dimension -> semantic::RADIUS_S;
    /// Tab: border width.
    TAB_BORDER_WIDTH_DEFAULT = "tab.border.width.default": Dimension -> semantic::BORDER_WIDTH_NONE;
    /// Tab: bar border width.
    TAB_BAR_BORDER_WIDTH_DEFAULT = "tab.bar.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Tab: bar overline width.
    TAB_BAR_OVERLINE_WIDTH_DEFAULT = "tab.bar.overline.width.default": Dimension =
        px(1.0), range(0.0, 8.0);
    /// Tab: unsaved marker color.
    TAB_UNSAVED_MARKER_COLOR_DEFAULT = "tab.unsaved-marker.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
}
