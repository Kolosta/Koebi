//! Menus: panels of items opened at the pointer or by a control, and their submenus.

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
    MENU_PADDING_HORIZONTAL_DEFAULT = "menu.padding.horizontal.default": Dimension = px(8.0), range(0.0, 32.0);
    /// Menu: vertical padding.
    MENU_PADDING_VERTICAL_DEFAULT = "menu.padding.vertical.default": Dimension = px(6.0), range(0.0, 32.0);
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

    /// Menu: height of an item.
    MENU_ITEM_HEIGHT_DEFAULT = "menu.item.height.default": Dimension ->
        semantic::SIZE_CONTROL_HEIGHT;
    /// Menu: label color of an item.
    MENU_ITEM_LABEL_COLOR_DEFAULT = "menu.item.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Menu: label color of the highlighted item.
    MENU_ITEM_LABEL_COLOR_HOVER = "menu.item.label.color.hover": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Menu: label color of the current choice.
    MENU_ITEM_LABEL_COLOR_SELECTED = "menu.item.label.color.selected": Color ->
        semantic::STATIC_TEXT_COLOR_WHITE;
    /// Menu: label color of a disabled item.
    MENU_ITEM_LABEL_COLOR_DISABLED = "menu.item.label.color.disabled": Color ->
        semantic::TEXT_COLOR_DISABLED_DEFAULT;
    /// Menu: color of the shortcut shown at the end of an item.
    MENU_ITEM_SHORTCUT_COLOR_DEFAULT = "menu.item.shortcut.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Menu: color of an item's icon and of the arrow of a submenu.
    MENU_ITEM_ICON_COLOR_DEFAULT = "menu.item.icon.color.default": Color ->
        semantic::ICON_COLOR_PRIMARY_DEFAULT;
    /// Menu: color of the check and radio marks.
    MENU_ITEM_MARK_COLOR_DEFAULT = "menu.item.mark.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Menu: space between an item's mark, icon, label and submenu arrow.
    MENU_ITEM_ICON_GAP_DEFAULT = "menu.item.icon-gap.default": Dimension =
        px(6.0), range(0.0, 24.0);
    /// Menu: side of the dot of a radio mark.
    MENU_ITEM_RADIO_SIZE_DEFAULT = "menu.item.radio.size.default": Dimension =
        px(6.0), range(2.0, 16.0);
    /// Menu: how far a notch of the wheel scrolls a menu taller than the window.
    MENU_WHEEL_STEP_DEFAULT = "menu.wheel-step.default": Dimension =
        px(48.0), range(1.0, 400.0);
}
