//! Nodes of a node graph.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group NODE_BOX: Component, start = NODE_GRAPH::END;

    /// Node box: header background color.
    NODE_BOX_HEADER_BACKGROUND_COLOR_DEFAULT = "node-box.header.background.color.default": Color ->
        semantic::BACKGROUND_SURFACE_RAISED;
    /// Node box: header background color input.
    NODE_BOX_HEADER_BACKGROUND_COLOR_INPUT = "node-box.header.background.color.input": Color ->
        semantic::ACCENT_COLOR_VISUAL_INFO;
    /// Node box: header background color merge.
    NODE_BOX_HEADER_BACKGROUND_COLOR_MERGE = "node-box.header.background.color.merge": Color ->
        semantic::ACCENT_COLOR_VISUAL_NOTICE;
    /// Node box: header background color output.
    NODE_BOX_HEADER_BACKGROUND_COLOR_OUTPUT = "node-box.header.background.color.output": Color ->
        semantic::ACCENT_COLOR_VISUAL_NEUTRAL;
    /// Node box: header background color blend.
    NODE_BOX_HEADER_BACKGROUND_COLOR_BLEND = "node-box.header.background.color.blend": Color ->
        semantic::ACCENT_COLOR_VISUAL_BRAND;
    /// Node box: header background color clip mask.
    NODE_BOX_HEADER_BACKGROUND_COLOR_CLIP_MASK =
        "node-box.header.background.color.clip-mask": Color ->
        semantic::ACCENT_COLOR_VISUAL_POSITIVE;
    /// Node box: body background color.
    NODE_BOX_BODY_BACKGROUND_COLOR_DEFAULT = "node-box.body.background.color.default": Color ->
        semantic::BACKGROUND_BASE_DEFAULT;
    /// Node box: border color.
    NODE_BOX_BORDER_COLOR_DEFAULT = "node-box.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Node box: border color, selected.
    NODE_BOX_BORDER_COLOR_SELECTED = "node-box.border.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Node box: text color.
    NODE_BOX_TEXT_COLOR_DEFAULT = "node-box.text.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Node box: corner radius.
    NODE_BOX_CORNER_RADIUS_DEFAULT = "node-box.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
    /// Node box: horizontal padding.
    NODE_BOX_PADDING_HORIZONTAL_DEFAULT = "node-box.padding.horizontal.default": Dimension =
        px(8.0), range(0.0, 32.0);
    /// Node box: vertical padding.
    NODE_BOX_PADDING_VERTICAL_DEFAULT = "node-box.padding.vertical.default": Dimension = px(6.0), range(0.0, 32.0);
    /// Node box: header height.
    NODE_BOX_HEADER_SIZE_HEIGHT = "node-box.header.size.height": Dimension ->
        semantic::SIZE_CONTROL_HEIGHT;
}
