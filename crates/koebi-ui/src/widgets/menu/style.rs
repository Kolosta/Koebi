//! [`MenuStyle`]: the colors and metrics of menus, from the `component.menu.*` tokens.

use koebi_core::Color;
use koebi_design::Resolve;
use koebi_tokens::component;

/// Colors and metrics of menus, from the `component.menu.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuStyle {
    /// Background of the panel.
    pub background: Color,
    /// Border color.
    pub border_color: Color,
    /// Border width.
    pub border_width: f32,
    /// Corner radius of the panel and of highlighted items.
    pub corner_radius: f32,
    /// Space above the first item and below the last one.
    pub padding_vertical: f32,
    /// Space beside the content of a popup.
    pub padding_horizontal: f32,
    /// Background of the highlighted item.
    pub item_hover: Color,
    /// Background of the current choice of a dropdown.
    pub item_selected: Color,
    /// Height of an item.
    pub item_height: f32,
    /// Space between an item and the panel's sides.
    pub item_gap: f32,
    /// Space between an item's edges and its content.
    pub item_padding: f32,
    /// Space between a label and its shortcut.
    pub column_gap: f32,
    /// Space between an item's mark, icon, label and arrow.
    pub icon_gap: f32,
    /// Tallest a menu grows before it scrolls.
    pub max_height: f32,
    /// Color of headings.
    pub title_color: Color,
    /// Label color at rest.
    pub label: Color,
    /// Label color of the highlighted item.
    pub label_hover: Color,
    /// Label color of the current choice.
    pub label_selected: Color,
    /// Label (and icon, mark, shortcut) color of disabled items.
    pub label_disabled: Color,
    /// Shortcut color.
    pub shortcut_color: Color,
    /// Icon and submenu arrow color.
    pub icon_color: Color,
    /// Check and radio mark color.
    pub mark_color: Color,
    /// Side of icons and marks.
    pub icon_size: f32,
    /// Stroke width of icons.
    pub stroke_width: f32,
    /// Side of the dot of a radio mark.
    pub radio_size: f32,
    /// Separator lines.
    pub separator_color: Color,
    /// Separator thickness.
    pub separator_width: f32,
    /// How far a notch of the wheel scrolls a long menu.
    pub wheel_step: f32,
}

impl MenuStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        Self {
            background: tokens.get(c::MENU_BACKGROUND_COLOR_DEFAULT),
            border_color: tokens.get(c::MENU_BORDER_COLOR_DEFAULT),
            border_width: tokens.get(c::MENU_BORDER_WIDTH_DEFAULT),
            corner_radius: tokens.get(c::MENU_CORNER_RADIUS_DEFAULT),
            padding_vertical: tokens.get(c::MENU_PADDING_VERTICAL_DEFAULT),
            padding_horizontal: tokens.get(c::MENU_PADDING_HORIZONTAL_DEFAULT),
            item_hover: tokens.get(c::MENU_ITEM_BACKGROUND_COLOR_HOVER),
            item_selected: tokens.get(c::MENU_ITEM_BACKGROUND_COLOR_SELECTED),
            item_height: tokens.get(c::MENU_ITEM_HEIGHT_DEFAULT),
            item_gap: tokens.get(c::MENU_ITEM_GAP_DEFAULT),
            item_padding: tokens.get(c::MENU_ITEM_PADDING_HORIZONTAL),
            column_gap: tokens.get(c::MENU_COLUMN_GAP_DEFAULT),
            icon_gap: tokens.get(c::MENU_ITEM_ICON_GAP_DEFAULT),
            max_height: tokens.get(c::MENU_MAX_HEIGHT_DEFAULT),
            title_color: tokens.get(c::MENU_TITLE_LABEL_COLOR_DEFAULT),
            label: tokens.get(c::MENU_ITEM_LABEL_COLOR_DEFAULT),
            label_hover: tokens.get(c::MENU_ITEM_LABEL_COLOR_HOVER),
            label_selected: tokens.get(c::MENU_ITEM_LABEL_COLOR_SELECTED),
            label_disabled: tokens.get(c::MENU_ITEM_LABEL_COLOR_DISABLED),
            shortcut_color: tokens.get(c::MENU_ITEM_SHORTCUT_COLOR_DEFAULT),
            icon_color: tokens.get(c::MENU_ITEM_ICON_COLOR_DEFAULT),
            mark_color: tokens.get(c::MENU_ITEM_MARK_COLOR_DEFAULT),
            icon_size: tokens.get(c::ICON_SIZE_DEFAULT),
            stroke_width: tokens.get(c::ICON_STROKE_WIDTH_DEFAULT),
            radio_size: tokens.get(c::MENU_ITEM_RADIO_SIZE_DEFAULT),
            separator_color: tokens.get(c::SEPARATOR_COLOR_DEFAULT),
            separator_width: tokens.get(c::SEPARATOR_SIZE_DEFAULT),
            wheel_step: tokens.get(c::MENU_WHEEL_STEP_DEFAULT),
        }
    }
}
