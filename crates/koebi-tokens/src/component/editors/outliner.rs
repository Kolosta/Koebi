//! The outliner.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group OUTLINER: Component, start = EDIT_HANDLE::END;

    /// Outliner: row background, hovered.
    OUTLINER_ROW_BACKGROUND_HOVER = "outliner.row.background.hover": Color ->
        semantic::ACCENT_COLOR_HOVER_DEFAULT;
    /// Outliner: row background, selected.
    OUTLINER_ROW_BACKGROUND_SELECTED = "outliner.row.background.selected": Color ->
        semantic::ACCENT_COLOR_SELECTED_DEFAULT;
    /// Outliner: row background, selected and hovered.
    OUTLINER_ROW_BACKGROUND_SELECTED_HOVER = "outliner.row.background.selected-hover": Color ->
        semantic::ACCENT_COLOR_HOVER_SELECTED_DEFAULT;
    /// Outliner: row background, active.
    OUTLINER_ROW_BACKGROUND_ACTIVE = "outliner.row.background.active": Color ->
        semantic::ACCENT_COLOR_ACTIVE_DEFAULT;
    /// Outliner: row background, active and hovered.
    OUTLINER_ROW_BACKGROUND_ACTIVE_HOVER = "outliner.row.background.active-hover": Color ->
        semantic::ACCENT_COLOR_ACTIVE_DEFAULT;
    /// Outliner: row text color.
    OUTLINER_ROW_TEXT_DEFAULT = "outliner.row.text.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Outliner: tree line inset.
    OUTLINER_TREE_LINE_INSET_DEFAULT = "outliner.tree-line.inset.default": Dimension =
        px(4.0), range(0.0, 16.0);
    /// Outliner: color of the lines joining a row to its children.
    OUTLINER_TREE_LINE_COLOR_DEFAULT = "outliner.tree-line.color.default": Color ->
        semantic::BORDER_COLOR_TREE_LINE_DEFAULT;
    /// Outliner: width of those lines.
    OUTLINER_TREE_LINE_WIDTH_DEFAULT = "outliner.tree-line.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Outliner: row search background visual.
    OUTLINER_ROW_SEARCH_BACKGROUND_VISUAL = "outliner.row.search.background.visual": Color ->
        semantic::ACCENT_COLOR_VISUAL_POSITIVE;
    /// Outliner: row search background, hovered.
    OUTLINER_ROW_SEARCH_BACKGROUND_HOVER = "outliner.row.search.background.hover": Color ->
        semantic::ACCENT_COLOR_HOVER_POSITIVE;
    /// Outliner: row search background, selected.
    OUTLINER_ROW_SEARCH_BACKGROUND_SELECTED = "outliner.row.search.background.selected": Color ->
        semantic::ACCENT_COLOR_SELECTED_POSITIVE;
    /// Outliner: row search background, selected and hovered.
    OUTLINER_ROW_SEARCH_BACKGROUND_SELECTED_HOVER =
        "outliner.row.search.background.selected-hover": Color ->
        semantic::ACCENT_COLOR_HOVER_SELECTED_POSITIVE;
    /// Outliner: row search background, active.
    OUTLINER_ROW_SEARCH_BACKGROUND_ACTIVE = "outliner.row.search.background.active": Color ->
        semantic::ACCENT_COLOR_ACTIVE_POSITIVE;
    /// Outliner: row search background, active and hovered.
    OUTLINER_ROW_SEARCH_BACKGROUND_ACTIVE_HOVER =
        "outliner.row.search.background.active-hover": Color ->
        semantic::ACCENT_COLOR_ACTIVE_POSITIVE;
    /// Outliner: row search text color.
    OUTLINER_ROW_SEARCH_TEXT_DEFAULT = "outliner.row.search.text.default": Color ->
        semantic::ACCENT_COLOR_TEXT_POSITIVE;
}
