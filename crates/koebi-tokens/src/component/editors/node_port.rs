//! Ports of nodes.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group NODE_PORT: Component, start = NODE_BOX::END;

    /// Node port: background color.
    NODE_PORT_BACKGROUND_COLOR_DEFAULT = "node-port.background.color.default": Color ->
        semantic::BORDER_COLOR_STRONG_DEFAULT;
    /// Node port: background color, hovered.
    NODE_PORT_BACKGROUND_COLOR_HOVER = "node-port.background.color.hover": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Node port: background color connected.
    NODE_PORT_BACKGROUND_COLOR_CONNECTED = "node-port.background.color.connected": Color ->
        semantic::BACKGROUND_POSITIVE_DEFAULT;
    /// Node port: background color incompatible.
    NODE_PORT_BACKGROUND_COLOR_INCOMPATIBLE = "node-port.background.color.incompatible": Color ->
        semantic::BACKGROUND_NEGATIVE_DEFAULT;
    /// Node port: border color.
    NODE_PORT_BORDER_COLOR_DEFAULT = "node-port.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Node port: size.
    NODE_PORT_SIZE_DEFAULT = "node-port.size.default": Dimension = px(8.0), range(0.0, 32.0);
}
