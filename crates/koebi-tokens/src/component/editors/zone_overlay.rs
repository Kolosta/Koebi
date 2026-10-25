//! Zone overlays: split and join previews.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group ZONE_OVERLAY: Component, start = ZONE_TAB::END;

    /// Zone overlay: corner top left color.
    ZONE_OVERLAY_CORNER_TOP_LEFT = "zone-overlay.corner.top-left": Color ->
        semantic::BORDER_COLOR_BLUE_DEFAULT;
    /// Zone overlay: corner top right color.
    ZONE_OVERLAY_CORNER_TOP_RIGHT = "zone-overlay.corner.top-right": Color ->
        semantic::BORDER_COLOR_GREEN_DEFAULT;
    /// Zone overlay: corner bottom left color.
    ZONE_OVERLAY_CORNER_BOTTOM_LEFT = "zone-overlay.corner.bottom-left": Color ->
        semantic::BORDER_COLOR_ORANGE_DEFAULT;
    /// Zone overlay: corner bottom right color.
    ZONE_OVERLAY_CORNER_BOTTOM_RIGHT = "zone-overlay.corner.bottom-right": Color ->
        semantic::BORDER_COLOR_MAGENTA_DEFAULT;
    /// Zone overlay: split line color.
    ZONE_OVERLAY_SPLIT_LINE = "zone-overlay.split-line": Color = hex("#FFFFFFF0");
    /// Zone overlay: transform dim color.
    ZONE_OVERLAY_TRANSFORM_DIM = "zone-overlay.transform-dim": Color = hex("#00000070");
    /// Zone overlay: join keep color.
    ZONE_OVERLAY_JOIN_KEEP = "zone-overlay.join.keep": Color = hex("#FFFFFF24");
    /// Zone overlay: join remove color.
    ZONE_OVERLAY_JOIN_REMOVE = "zone-overlay.join.remove": Color = hex("#00000070");
    /// Zone overlay: join residual color.
    ZONE_OVERLAY_JOIN_RESIDUAL = "zone-overlay.join.residual": Color = hex("#000000B0");
    /// Zone overlay: join frame color.
    ZONE_OVERLAY_JOIN_FRAME = "zone-overlay.join.frame": Color ->
        semantic::BORDER_COLOR_BLUE_DEFAULT;
}
