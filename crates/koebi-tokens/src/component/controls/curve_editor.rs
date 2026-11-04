//! Curve editors: the graph of an easing curve, its grid, its line and its keys.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group CURVE_EDITOR: Component, start = COLOR_PICKER::END;

    /// Curve editor: height of its graph.
    CURVE_EDITOR_GRAPH_HEIGHT_DEFAULT = "curve-editor.graph.height.default": Dimension =
        px(160.0), range(64.0, 512.0);
    /// Curve editor: space between the graph's frame and the square the curve spans.
    CURVE_EDITOR_PADDING_DEFAULT = "curve-editor.padding.default": Dimension = px(10.0),
        range(0.0, 48.0);
    /// Curve editor: background of the graph.
    CURVE_EDITOR_BACKGROUND_COLOR_DEFAULT = "curve-editor.background.color.default": Color ->
        NUMBER_FIELD_BACKGROUND_COLOR_DEFAULT;
    /// Curve editor: border of the graph.
    CURVE_EDITOR_BORDER_COLOR_DEFAULT = "curve-editor.border.color.default": Color ->
        NUMBER_FIELD_BORDER_COLOR_DEFAULT;
    /// Curve editor: width of the graph's border.
    CURVE_EDITOR_BORDER_WIDTH_DEFAULT = "curve-editor.border.width.default": Dimension ->
        NUMBER_FIELD_BORDER_WIDTH_DEFAULT;
    /// Curve editor: corner radius of the graph.
    CURVE_EDITOR_CORNER_RADIUS_DEFAULT = "curve-editor.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
    /// Curve editor: lines of the grid (the start, the middle and the end in time, the levels
    /// 0 and 1).
    CURVE_EDITOR_GRID_COLOR_DEFAULT = "curve-editor.grid.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Curve editor: color of the curve.
    CURVE_EDITOR_CURVE_COLOR_DEFAULT = "curve-editor.curve.color.default": Color ->
        semantic::BORDER_COLOR_ACCENT_DEFAULT;
    /// Curve editor: width of the curve.
    CURVE_EDITOR_CURVE_WIDTH_DEFAULT = "curve-editor.curve.width.default": Dimension =
        px(2.0), range(0.5, 6.0);
    /// Curve editor: side of a key.
    CURVE_EDITOR_KEY_SIZE_DEFAULT = "curve-editor.key.size.default": Dimension = px(8.0),
        range(4.0, 24.0);
    /// Curve editor: side of the selected key.
    CURVE_EDITOR_KEY_SIZE_SELECTED = "curve-editor.key.size.selected": Dimension = px(10.0),
        range(4.0, 24.0);
    /// Curve editor: fill of a key.
    CURVE_EDITOR_KEY_COLOR_DEFAULT = "curve-editor.key.color.default": Color ->
        semantic::STATIC_TEXT_COLOR_WHITE;
    /// Curve editor: fill of the selected key.
    CURVE_EDITOR_KEY_COLOR_SELECTED = "curve-editor.key.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Curve editor: fill of a key that cannot move (the first and the last).
    CURVE_EDITOR_KEY_COLOR_FIXED = "curve-editor.key.color.fixed": Color ->
        semantic::NEUTRAL_COLOR_500;
    /// Curve editor: outline of the keys.
    CURVE_EDITOR_KEY_OUTLINE_COLOR_DEFAULT = "curve-editor.key.outline.color.default": Color ->
        semantic::STATIC_TEXT_COLOR_BLACK;
    /// Curve editor: width of the outline of the keys.
    CURVE_EDITOR_KEY_OUTLINE_WIDTH_DEFAULT = "curve-editor.key.outline.width.default":
        Dimension = px(1.0), range(0.5, 3.0);
    /// Curve editor: how far from a key a press takes it.
    CURVE_EDITOR_PICK_DISTANCE_DEFAULT = "curve-editor.pick-distance.default": Dimension =
        px(8.0), range(2.0, 32.0);
}
