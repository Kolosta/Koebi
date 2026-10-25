//! Viewports: the canvas, its guides and the cursor.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group VIEWPORT: Component, start = TOOL_PALETTE::END;

    /// Viewport: canvas background.
    VIEWPORT_CANVAS_BACKGROUND = "viewport.canvas.background": Color ->
        semantic::BACKGROUND_SURFACE_CANVAS;
    /// Viewport: guide color.
    VIEWPORT_GUIDE_COLOR = "viewport.guide.color": Color -> semantic::BORDER_COLOR_BLUE_DEFAULT;
    /// Viewport: page border color.
    VIEWPORT_PAGE_BORDER = "viewport.page.border": Color = hex("#00000070");
    /// Viewport: page name background, hovered.
    VIEWPORT_PAGE_NAME_BACKGROUND_HOVER = "viewport.page-name.background.hover": Color =
        hex("#FFFFFF2B");
    /// Viewport: origin outline color.
    VIEWPORT_ORIGIN_OUTLINE = "viewport.origin.outline": Color = hex("#000000B0");
    /// Viewport: cursor ring color.
    VIEWPORT_CURSOR_RING = "viewport.cursor.ring": Color -> semantic::STATIC_TEXT_COLOR_WHITE;
    /// Viewport: cursor ring accent color.
    VIEWPORT_CURSOR_RING_ACCENT = "viewport.cursor.ring-accent": Color = hex("#D73220");
    /// Viewport: cursor tick color.
    VIEWPORT_CURSOR_TICK = "viewport.cursor.tick": Color -> semantic::STATIC_TEXT_COLOR_BLACK;
    /// Viewport: cursor axis x color.
    VIEWPORT_CURSOR_AXIS_X = "viewport.cursor.axis-x": Color = hex("#F03823");
    /// Viewport: cursor axis y color.
    VIEWPORT_CURSOR_AXIS_Y = "viewport.cursor.axis-y": Color = hex("#12B867");
    /// Viewport: thumbnail background.
    VIEWPORT_THUMBNAIL_BACKGROUND = "viewport.thumbnail.background": Color ->
        semantic::STATIC_TEXT_COLOR_WHITE;
    /// Viewport: thumbnail border color.
    VIEWPORT_THUMBNAIL_BORDER = "viewport.thumbnail.border": Color = hex("#00000070");
    /// Viewport: crosshair color.
    VIEWPORT_CROSSHAIR = "viewport.crosshair": Color -> semantic::STATIC_TEXT_COLOR_WHITE;
}
