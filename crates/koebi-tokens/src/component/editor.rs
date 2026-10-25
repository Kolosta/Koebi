//! Editors: zones and their tabs, viewport, edit handles, outliner, lists and node graphs.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group EDITOR: Component, start = SURFACES::END;

    /// Editor: background color.
    EDITOR_BACKGROUND_COLOR_DEFAULT = "editor.background.color.default": Color ->
        semantic::BACKGROUND_SURFACE_CANVAS;
    /// Editor: top bar background color.
    EDITOR_TOP_BAR_BACKGROUND_COLOR_DEFAULT = "editor.top-bar.background.color.default": Color ->
        semantic::BACKGROUND_SURFACE_RAISED;
    /// Editor: content inset.
    EDITOR_CONTENT_INSET_DEFAULT = "editor.content.inset.default": Dimension ->
        semantic::SPACING_EDITOR_INSET;

    /// Editor top bar: padding.
    EDITOR_TOP_BAR_PADDING_DEFAULT = "editor-top-bar.padding.default": Dimension =
        px(6.0), range(0.0, 24.0);

    /// Zone: separator size.
    ZONE_SEPARATOR_SIZE_DEFAULT = "zone.separator.size.default": Dimension =
        px(4.0), range(2.0, 20.0);
    /// Zone: separator color.
    ZONE_SEPARATOR_COLOR_DEFAULT = "zone.separator.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Zone: separator color continuation.
    ZONE_SEPARATOR_COLOR_CONTINUATION = "zone.separator.color.continuation": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Zone: separator continuation opacity.
    ZONE_SEPARATOR_CONTINUATION_OPACITY_DEFAULT =
        "zone.separator.continuation.opacity.default": Number -> semantic::OPACITY_REDUCED;

    /// Zone tab: gap.
    ZONE_TAB_GAP_DEFAULT = "zone-tab.gap.default": Dimension = px(2.0), range(0.0, 24.0);
    /// Zone tab: horizontal padding.
    ZONE_TAB_PADDING_HORIZONTAL_DEFAULT = "zone-tab.padding.horizontal.default": Dimension =
        px(8.0);
    /// Zone tab: vertical padding.
    ZONE_TAB_PADDING_VERTICAL_DEFAULT = "zone-tab.padding.vertical.default": Dimension = px(2.0);
    /// Zone tab: background color.
    ZONE_TAB_BACKGROUND_COLOR_DEFAULT = "zone-tab.background.color.default": Color ->
        semantic::BACKGROUND_SURFACE_CANVAS;
    /// Zone tab: background color, active.
    ZONE_TAB_BACKGROUND_COLOR_ACTIVE = "zone-tab.background.color.active": Color ->
        EDITOR_TOP_BAR_BACKGROUND_COLOR_DEFAULT;
    /// Zone tab: background color, hovered.
    ZONE_TAB_BACKGROUND_COLOR_HOVER = "zone-tab.background.color.hover": Color ->
        semantic::BACKGROUND_SURFACE_RAISED;
    /// Zone tab: label color.
    ZONE_TAB_LABEL_COLOR_DEFAULT = "zone-tab.label.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Zone tab: label color, active.
    ZONE_TAB_LABEL_COLOR_ACTIVE = "zone-tab.label.color.active": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Zone tab: bar background color.
    ZONE_TAB_BAR_BACKGROUND_COLOR_DEFAULT = "zone-tab.bar.background.color.default": Color ->
        semantic::BACKGROUND_SURFACE_CANVAS;
    /// Zone tab: insert line color.
    ZONE_TAB_INSERT_LINE_COLOR_DEFAULT = "zone-tab.insert-line.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Zone tab: insert line width.
    ZONE_TAB_INSERT_LINE_WIDTH_DEFAULT = "zone-tab.insert-line.width.default": Dimension =
        px(2.0), range(1.0, 6.0);
    /// Zone tab: drop preview color.
    ZONE_TAB_DROP_PREVIEW_COLOR_DEFAULT = "zone-tab.drop-preview.color.default": Color =
        hex("#FFFFFF2E");
    /// Zone tab: drop preview center inset.
    ZONE_TAB_DROP_PREVIEW_CENTER_INSET = "zone-tab.drop-preview.center-inset": Dimension =
        px(6.0), range(0.0, 32.0);
    /// Zone tab: drop preview anim duration.
    ZONE_TAB_DROP_PREVIEW_ANIM_DURATION = "zone-tab.drop-preview.anim-duration": Duration =
        ms(60), range(0.0, 1000.0);
    /// Zone tab: drag threshold.
    ZONE_TAB_DRAG_THRESHOLD = "zone-tab.drag.threshold": Dimension = px(5.0), range(1.0, 20.0);
    /// Zone tab: show solo.
    ZONE_TAB_SHOW_SOLO = "zone-tab.show-solo": Integer = 0, range(0.0, 1.0);

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

    /// Drop target: corner radius.
    DROP_TARGET_CORNER_RADIUS_DEFAULT = "drop-target.corner-radius.default": Dimension ->
        semantic::RADIUS_M;
    /// Drop target: border width.
    DROP_TARGET_BORDER_WIDTH_DEFAULT = "drop-target.border.width.default": Dimension =
        px(2.0), range(0.0, 4.0);
    /// Drop target: padding.
    DROP_TARGET_PADDING_DEFAULT = "drop-target.padding.default": Dimension =
        px(3.0), range(0.0, 16.0);

    /// Tool palette: button size.
    TOOL_PALETTE_BUTTON_SIZE = "tool-palette.button.size": Dimension = px(32.0), range(16.0, 64.0);
    /// Tool palette: icon scale.
    TOOL_PALETTE_ICON_SCALE = "tool-palette.icon.scale": Number = 0.68, range(0.3, 1.0);

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

    /// List row: reorder duration.
    LIST_ROW_REORDER_DURATION = "list-row.reorder.duration": Duration ->
        semantic::ANIMATION_DURATION_ENTER;
    /// List row: drag background.
    LIST_ROW_DRAG_BACKGROUND = "list-row.drag.background": Color ->
        semantic::ACCENT_COLOR_HOVER_DEFAULT;
    /// List row: drag opacity.
    LIST_ROW_DRAG_OPACITY = "list-row.drag.opacity": Number = 0.55, range(0.0, 1.0);
    /// List row: drop slot background.
    LIST_ROW_DROP_SLOT_BACKGROUND = "list-row.drop-slot.background": Color ->
        semantic::BACKGROUND_NOTICE_DEFAULT;

    /// Node graph: background color.
    NODE_GRAPH_BACKGROUND_COLOR_DEFAULT = "node-graph.background.color.default": Color ->
        semantic::BACKGROUND_SURFACE_CANVAS;
    /// Node graph: grid line color.
    NODE_GRAPH_GRID_LINE_COLOR_DEFAULT = "node-graph.grid-line.color.default": Color ->
        semantic::BORDER_COLOR_TREE_LINE_DEFAULT;

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
        px(8.0);
    /// Node box: vertical padding.
    NODE_BOX_PADDING_VERTICAL_DEFAULT = "node-box.padding.vertical.default": Dimension = px(6.0);
    /// Node box: header height.
    NODE_BOX_HEADER_SIZE_HEIGHT = "node-box.header.size.height": Dimension ->
        semantic::SIZE_CONTROL_HEIGHT;

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
