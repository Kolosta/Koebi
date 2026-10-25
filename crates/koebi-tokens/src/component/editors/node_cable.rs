//! Cables between nodes.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group NODE_CABLE: Component, start = NODE_PORT::END;

    /// Node cable: color.
    NODE_CABLE_COLOR_DEFAULT = "node-cable.color.default": Color ->
        semantic::BORDER_COLOR_TREE_LINE_DEFAULT;
    /// Node cable: color, active.
    NODE_CABLE_COLOR_ACTIVE = "node-cable.color.active": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Node cable: color incompatible.
    NODE_CABLE_COLOR_INCOMPATIBLE = "node-cable.color.incompatible": Color ->
        semantic::BACKGROUND_NEGATIVE_DEFAULT;
    /// Node cable: color type render output.
    NODE_CABLE_COLOR_TYPE_RENDER_OUTPUT = "node-cable.color.type-render-output": Color ->
        semantic::ACCENT_COLOR_VISUAL_INFO;
    /// Node cable: color type layer output.
    NODE_CABLE_COLOR_TYPE_LAYER_OUTPUT = "node-cable.color.type-layer-output": Color ->
        semantic::ACCENT_COLOR_VISUAL_BRAND;
    /// Node cable: width.
    NODE_CABLE_WIDTH_DEFAULT = "node-cable.width.default": Dimension = px(2.0), range(0.0, 12.0);
}
