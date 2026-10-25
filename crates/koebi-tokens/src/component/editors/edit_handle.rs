//! Edit handles of vector paths.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group EDIT_HANDLE: Component, start = VIEWPORT::END;

    /// Edit handle: edge color.
    EDIT_HANDLE_EDGE = "edit-handle.edge": Color -> semantic::BORDER_COLOR_STRONG_DEFAULT;
    /// Edit handle: vertex color.
    EDIT_HANDLE_VERTEX = "edit-handle.vertex": Color = hex("#131313");
    /// Edit handle: vertex ring color.
    EDIT_HANDLE_VERTEX_RING = "edit-handle.vertex-ring": Color = hex("#FFFFFFA8");
    /// Edit handle: nurbs hull color.
    EDIT_HANDLE_NURBS_HULL = "edit-handle.nurbs-hull": Color -> semantic::BORDER_COLOR_BLUE_DEFAULT;
    /// Edit handle: free color.
    EDIT_HANDLE_FREE = "edit-handle.free": Color -> semantic::BORDER_COLOR_BLUE_DEFAULT;
    /// Edit handle: aligned color.
    EDIT_HANDLE_ALIGNED = "edit-handle.aligned": Color -> semantic::BORDER_COLOR_ORANGE_DEFAULT;
    /// Edit handle: mirrored color.
    EDIT_HANDLE_MIRRORED = "edit-handle.mirrored": Color -> semantic::BORDER_COLOR_GREEN_DEFAULT;
    /// Edit handle: vector color.
    EDIT_HANDLE_VECTOR = "edit-handle.vector": Color -> semantic::BORDER_COLOR_PURPLE_DEFAULT;
    /// Edit handle: color.
    EDIT_HANDLE_DEFAULT = "edit-handle.default": Color -> semantic::BORDER_COLOR_BLUE_DEFAULT;
}
