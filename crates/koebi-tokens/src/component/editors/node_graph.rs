//! Node graphs.

use super::*;
use crate::semantic;

koebi_design::tokens! {
    group NODE_GRAPH: Component, start = LIST_ROW::END;

    /// Node graph: background color.
    NODE_GRAPH_BACKGROUND_COLOR_DEFAULT = "node-graph.background.color.default": Color ->
        semantic::BACKGROUND_SURFACE_CANVAS;
    /// Node graph: grid line color.
    NODE_GRAPH_GRID_LINE_COLOR_DEFAULT = "node-graph.grid-line.color.default": Color ->
        semantic::BORDER_COLOR_TREE_LINE_DEFAULT;
}
