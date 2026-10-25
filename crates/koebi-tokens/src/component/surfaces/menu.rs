//! Menus.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group MENU: Component, start = PROPERTY_GROUP::END;

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
}
