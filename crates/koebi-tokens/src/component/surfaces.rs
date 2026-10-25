//! Surfaces: windows, title bars, popups, panels, menus, tooltips, bars, tables and images.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group SURFACES: Component, start = CONTROLS::END;

    /// Window: background color.
    WINDOW_BACKGROUND_COLOR_DEFAULT = "window.background.color.default": Color ->
        semantic::BACKGROUND_BASE_DEFAULT;
    /// Window: corner radius.
    WINDOW_CORNER_RADIUS_DEFAULT = "window.corner-radius.default": Dimension -> semantic::RADIUS_M;
    /// Window: border width.
    WINDOW_BORDER_WIDTH_DEFAULT = "window.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Window: border color.
    WINDOW_BORDER_COLOR_DEFAULT = "window.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Window: horizontal padding.
    WINDOW_PADDING_HORIZONTAL_DEFAULT = "window.padding.horizontal.default": Dimension = px(8.0);
    /// Window: vertical padding.
    WINDOW_PADDING_VERTICAL_DEFAULT = "window.padding.vertical.default": Dimension = px(4.0);
    /// Window: shadow color.
    WINDOW_SHADOW_COLOR_DEFAULT = "window.shadow.color.default": Color = hex("#00000040");
    /// Window: shadow size.
    WINDOW_SHADOW_SIZE_DEFAULT = "window.shadow.size.default": Dimension =
        px(18.0), range(0.0, 64.0);

    /// Title bar: background color.
    TITLE_BAR_BACKGROUND_COLOR_DEFAULT = "title-bar.background.color.default": Color ->
        semantic::BACKGROUND_BASE_DEFAULT;
    /// Title bar: icon color.
    TITLE_BAR_ICON_COLOR_DEFAULT = "title-bar.icon.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Title bar: label color.
    TITLE_BAR_LABEL_COLOR_DEFAULT = "title-bar.label.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Title bar: button background color, hovered.
    TITLE_BAR_BUTTON_BACKGROUND_COLOR_HOVER = "title-bar.button.background.color.hover": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Title bar: close background color, hovered.
    TITLE_BAR_CLOSE_BACKGROUND_COLOR_HOVER = "title-bar.close.background.color.hover": Color ->
        semantic::BACKGROUND_NEGATIVE_DEFAULT;

    /// Preferences title bar: background color.
    PREFERENCES_TITLE_BAR_BACKGROUND_COLOR_DEFAULT =
        "preferences-title-bar.background.color.default": Color ->
        semantic::BACKGROUND_BASE_DEFAULT;
    /// Preferences title bar: label color.
    PREFERENCES_TITLE_BAR_LABEL_COLOR_DEFAULT =
        "preferences-title-bar.label.color.default": Color -> semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Preferences title bar: icon color.
    PREFERENCES_TITLE_BAR_ICON_COLOR_DEFAULT = "preferences-title-bar.icon.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Preferences title bar: button background color, hovered.
    PREFERENCES_TITLE_BAR_BUTTON_BACKGROUND_COLOR_HOVER =
        "preferences-title-bar.button.background.color.hover": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Preferences title bar: close background color, hovered.
    PREFERENCES_TITLE_BAR_CLOSE_BACKGROUND_COLOR_HOVER =
        "preferences-title-bar.close.background.color.hover": Color ->
        semantic::BACKGROUND_NEGATIVE_DEFAULT;

    /// Splash: background color.
    SPLASH_BACKGROUND_COLOR_DEFAULT = "splash.background.color.default": Color ->
        semantic::BACKGROUND_BASE_DEFAULT;
    /// Splash: link color.
    SPLASH_LINK_COLOR_DEFAULT = "splash.link.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Splash: version text color.
    SPLASH_VERSION_TEXT_COLOR_DEFAULT = "splash.version-text.color.default": Color ->
        semantic::NEUTRAL_COLOR_900;

    /// Popup: background color.
    POPUP_BACKGROUND_COLOR_DEFAULT = "popup.background.color.default": Color ->
        semantic::BACKGROUND_FLOATING_DEFAULT;
    /// Popup: corner radius.
    POPUP_CORNER_RADIUS_DEFAULT = "popup.corner-radius.default": Dimension -> semantic::RADIUS_M;
    /// Popup: border width.
    POPUP_BORDER_WIDTH_DEFAULT = "popup.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Popup: menu bar background color.
    POPUP_MENU_BAR_BACKGROUND_COLOR_DEFAULT = "popup.menu-bar.background.color.default": Color ->
        semantic::BACKGROUND_SUNKEN_DEFAULT;

    /// Child: background color.
    CHILD_BACKGROUND_COLOR_DEFAULT = "child.background.color.default": Color ->
        semantic::BACKGROUND_APP_CHILD;
    /// Child: background opacity.
    CHILD_BACKGROUND_OPACITY_DEFAULT = "child.background.opacity.default": Number ->
        semantic::OPACITY_TRANSPARENT;
    /// Child: corner radius.
    CHILD_CORNER_RADIUS_DEFAULT = "child.corner-radius.default": Dimension -> semantic::RADIUS_M;
    /// Child: border width.
    CHILD_BORDER_WIDTH_DEFAULT = "child.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;

    /// Panel: header background color.
    PANEL_HEADER_BACKGROUND_COLOR_DEFAULT = "panel.header.background.color.default": Color ->
        semantic::BACKGROUND_SURFACE_RAISED;
    /// Panel: body background color level 1.
    PANEL_BODY_BACKGROUND_COLOR_LEVEL_1 = "panel.body.background.color.level-1": Color ->
        semantic::BACKGROUND_SURFACE_RAISED;
    /// Panel: body background color level 2.
    PANEL_BODY_BACKGROUND_COLOR_LEVEL_2 = "panel.body.background.color.level-2": Color ->
        semantic::BACKGROUND_SURFACE_CANVAS;
    /// Panel: body background color level 3.
    PANEL_BODY_BACKGROUND_COLOR_LEVEL_3 = "panel.body.background.color.level-3": Color ->
        semantic::BACKGROUND_BASE_DEFAULT;
    /// Panel: border color.
    PANEL_BORDER_COLOR_DEFAULT = "panel.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Panel: label color.
    PANEL_LABEL_COLOR_DEFAULT = "panel.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Panel: override badge color.
    PANEL_OVERRIDE_BADGE_COLOR_DEFAULT = "panel.override-badge.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Panel: gap size.
    PANEL_GAP_SIZE_DEFAULT = "panel.gap.size.default": Dimension = px(2.0), range(0.0, 20.0);
    /// Panel: corner radius.
    PANEL_CORNER_RADIUS_DEFAULT = "panel.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;

    /// Side panel: margin.
    SIDE_PANEL_MARGIN_DEFAULT = "side-panel.margin.default": Dimension = px(8.0), range(0.0, 32.0);

    /// Property group: gap size.
    PROPERTY_GROUP_GAP_SIZE_DEFAULT = "property-group.gap.size.default": Dimension =
        px(10.0), range(0.0, 40.0);

    /// Menu: background color.
    MENU_BACKGROUND_COLOR_DEFAULT = "menu.background.color.default": Color ->
        semantic::BACKGROUND_BASE_DEFAULT;
    /// Menu: corner radius.
    MENU_CORNER_RADIUS_DEFAULT = "menu.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
    /// Menu: item background color, hovered.
    MENU_ITEM_BACKGROUND_COLOR_HOVER = "menu.item.background.color.hover": Color ->
        semantic::BACKGROUND_RAISED_DEFAULT;
    /// Menu: item background color, selected.
    MENU_ITEM_BACKGROUND_COLOR_SELECTED = "menu.item.background.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Menu: column header label color.
    MENU_COLUMN_HEADER_LABEL_COLOR_DEFAULT = "menu.column-header.label.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Menu: horizontal padding.
    MENU_PADDING_HORIZONTAL_DEFAULT = "menu.padding.horizontal.default": Dimension = px(8.0);
    /// Menu: vertical padding.
    MENU_PADDING_VERTICAL_DEFAULT = "menu.padding.vertical.default": Dimension = px(6.0);
    /// Menu: column gap.
    MENU_COLUMN_GAP_DEFAULT = "menu.column-gap.default": Dimension = px(16.0), range(0.0, 64.0);
    /// Menu: item gap.
    MENU_ITEM_GAP_DEFAULT = "menu.item-gap.default": Dimension = px(2.0), range(0.0, 16.0);
    /// Menu: border color.
    MENU_BORDER_COLOR_DEFAULT = "menu.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Menu: border width.
    MENU_BORDER_WIDTH_DEFAULT = "menu.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Menu: item horizontal padding.
    MENU_ITEM_PADDING_HORIZONTAL = "menu.item.padding.horizontal": Dimension =
        px(6.0), range(0.0, 24.0);
    /// Menu: max height.
    MENU_MAX_HEIGHT_DEFAULT = "menu.max-height.default": Dimension =
        px(420.0), range(120.0, 2000.0);
    /// Menu: title label color.
    MENU_TITLE_LABEL_COLOR_DEFAULT = "menu.title.label.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;

    /// Tooltip: background color.
    TOOLTIP_BACKGROUND_COLOR_DEFAULT = "tooltip.background.color.default": Color ->
        EDITOR_TOP_BAR_BACKGROUND_COLOR_DEFAULT;
    /// Tooltip: label color.
    TOOLTIP_LABEL_COLOR_DEFAULT = "tooltip.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Tooltip: border color.
    TOOLTIP_BORDER_COLOR_DEFAULT = "tooltip.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Tooltip: border width.
    TOOLTIP_BORDER_WIDTH_DEFAULT = "tooltip.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Tooltip: corner radius.
    TOOLTIP_CORNER_RADIUS_DEFAULT = "tooltip.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
    /// Tooltip: horizontal padding.
    TOOLTIP_PADDING_HORIZONTAL_DEFAULT = "tooltip.padding.horizontal.default": Dimension = px(8.0);
    /// Tooltip: vertical padding.
    TOOLTIP_PADDING_VERTICAL_DEFAULT = "tooltip.padding.vertical.default": Dimension = px(6.0);

    /// Status bar: background color.
    STATUS_BAR_BACKGROUND_COLOR_DEFAULT = "status-bar.background.color.default": Color ->
        semantic::BACKGROUND_RAISED_DEFAULT;
    /// Status bar: label color.
    STATUS_BAR_LABEL_COLOR_DEFAULT = "status-bar.label.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Status bar: height.
    STATUS_BAR_SIZE_HEIGHT = "status-bar.size.height": Dimension = px(22.0), range(16.0, 40.0);
    /// Status bar: horizontal padding.
    STATUS_BAR_PADDING_HORIZONTAL_DEFAULT = "status-bar.padding.horizontal.default": Dimension =
        px(4.0);
    /// Status bar: vertical padding.
    STATUS_BAR_PADDING_VERTICAL_DEFAULT = "status-bar.padding.vertical.default": Dimension =
        px(2.0);
    /// Status bar: gap.
    STATUS_BAR_GAP_DEFAULT = "status-bar.gap.default": Dimension = px(12.0), range(0.0, 48.0);

    /// Section header: label color.
    SECTION_HEADER_LABEL_COLOR_DEFAULT = "section-header.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Section header: font scale.
    SECTION_HEADER_FONT_SCALE_DEFAULT = "section-header.font-scale.default": Number =
        1.1, range(0.5, 2.5);

    /// Shortcut row: background color, hovered.
    SHORTCUT_ROW_BACKGROUND_COLOR_HOVER = "shortcut-row.background.color.hover": Color ->
        semantic::BACKGROUND_RAISED_DEFAULT;
    /// Shortcut row: background color, selected.
    SHORTCUT_ROW_BACKGROUND_COLOR_SELECTED = "shortcut-row.background.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;

    /// Shortcut: conflict color soft.
    SHORTCUT_CONFLICT_COLOR_SOFT = "shortcut.conflict.color.soft": Color ->
        semantic::BACKGROUND_NOTICE_DEFAULT;
    /// Shortcut: conflict color hard.
    SHORTCUT_CONFLICT_COLOR_HARD = "shortcut.conflict.color.hard": Color ->
        semantic::BACKGROUND_NEGATIVE_DEFAULT;
    /// Shortcut: recording color.
    SHORTCUT_RECORDING_COLOR_DEFAULT = "shortcut.recording.color.default": Color ->
        semantic::BACKGROUND_NEGATIVE_DEFAULT;
    /// Shortcut: capture background color.
    SHORTCUT_CAPTURE_BACKGROUND_COLOR_DEFAULT =
        "shortcut.capture.background.color.default": Color -> semantic::BACKGROUND_RAISED_DEFAULT;

    /// Table: header background color.
    TABLE_HEADER_BACKGROUND_COLOR_DEFAULT = "table.header.background.color.default": Color ->
        semantic::BACKGROUND_RAISED_DEFAULT;
    /// Table: border color strong.
    TABLE_BORDER_COLOR_STRONG = "table.border.color.strong": Color ->
        semantic::BORDER_COLOR_STRONG_DEFAULT;
    /// Table: border color light.
    TABLE_BORDER_COLOR_LIGHT = "table.border.color.light": Color -> semantic::BORDER_COLOR_DEFAULT;
    /// Table: row background color.
    TABLE_ROW_BACKGROUND_COLOR_DEFAULT = "table.row.background.color.default": Color =
        rgba_f32(0.0, 0.0, 0.0, 0.0);
    /// Table: row background color alt.
    TABLE_ROW_BACKGROUND_COLOR_ALT = "table.row.background.color.alt": Color =
        rgba_f32(1.0, 1.0, 1.0, 0.06);

    /// Image: corner radius.
    IMAGE_CORNER_RADIUS_DEFAULT = "image.corner-radius.default": Dimension -> semantic::RADIUS_M;
    /// Image: border width.
    IMAGE_BORDER_WIDTH_DEFAULT = "image.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_NONE;

    /// Data viz: line color.
    DATA_VIZ_LINE_COLOR_DEFAULT = "data-viz.line.color.default": Color ->
        semantic::DATA_VIZ_CATEGORICAL_SHORT_1;
    /// Data viz: line color, hovered.
    DATA_VIZ_LINE_COLOR_HOVER = "data-viz.line.color.hover": Color ->
        semantic::DATA_VIZ_HIGHLIGHT_DEFAULT;
    /// Data viz: histogram color.
    DATA_VIZ_HISTOGRAM_COLOR_DEFAULT = "data-viz.histogram.color.default": Color ->
        semantic::DATA_VIZ_CATEGORICAL_SHORT_2;
    /// Data viz: histogram color, hovered.
    DATA_VIZ_HISTOGRAM_COLOR_HOVER = "data-viz.histogram.color.hover": Color ->
        semantic::DATA_VIZ_HIGHLIGHT_DEFAULT;

    /// Cursor: icon color.
    CURSOR_ICON_COLOR_DEFAULT = "cursor.icon.color.default": Color ->
        semantic::STATIC_TEXT_COLOR_WHITE;
}
