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
    /// Viewport: color of the canvas grid.
    VIEWPORT_GRID_COLOR = "viewport.grid.color": Color ->
        semantic::BORDER_COLOR_TREE_LINE_DEFAULT;
    /// Viewport: smallest space between grid lines on screen.
    VIEWPORT_GRID_SPACING = "viewport.grid.spacing": Dimension = px(24.0), range(4.0, 200.0);
    /// Viewport: background of the page.
    VIEWPORT_PAGE_BACKGROUND = "viewport.page.background": Color ->
        semantic::STATIC_TEXT_COLOR_WHITE;
    /// Viewport: thickness of a ruler.
    VIEWPORT_RULER_SIZE = "viewport.ruler.size": Dimension = px(20.0), range(8.0, 64.0);
    /// Viewport: background of the rulers.
    VIEWPORT_RULER_BACKGROUND = "viewport.ruler.background": Color ->
        semantic::BACKGROUND_RAISED_DEFAULT;
    /// Viewport: ticks of the rulers.
    VIEWPORT_RULER_TICK = "viewport.ruler.tick": Color -> semantic::BORDER_COLOR_DEFAULT;
    /// Viewport: numbers of the rulers.
    VIEWPORT_RULER_LABEL = "viewport.ruler.label": Color -> semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Viewport: zoom factor of one wheel notch or zoom step.
    VIEWPORT_ZOOM_STEP = "viewport.zoom-step": Number = 1.25, range(1.01, 4.0);
}
