//! Window title bars.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group TITLE_BAR: Component, start = WINDOW::END;

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
    /// Title bar: lightness added to a window button held down; negative, it darkens.
    TITLE_BAR_BUTTON_PRESSED_LIGHTNESS = "title-bar.button.pressed-lightness": Number = -0.05,
        range(-0.5, 0.5);
    /// Title bar: space between its edges and its items, which are one interface unit high.
    TITLE_BAR_PADDING_DEFAULT = "title-bar.padding.default": Dimension = px(2.0), range(0.0, 16.0);
    /// Title bar: space between its items.
    TITLE_BAR_GAP_DEFAULT = "title-bar.gap.default": Dimension = px(0.0), range(0.0, 16.0);
    /// Title bar: width of a window button, which is as high as the bar.
    TITLE_BAR_BUTTON_WIDTH_DEFAULT = "title-bar.button.width.default": Dimension = px(36.0),
        range(16.0, 96.0);
    /// Title bar: side of a window button's glyph.
    TITLE_BAR_GLYPH_SIZE_DEFAULT = "title-bar.glyph.size.default": Dimension = px(10.0),
        range(4.0, 32.0);
    /// Title bar: width of the lines of a window button's glyph.
    TITLE_BAR_GLYPH_STROKE_DEFAULT = "title-bar.glyph.stroke.default": Dimension = px(1.0),
        range(1.0, 4.0);
    /// Title bar: how far the back square of the restore glyph lies from the front one.
    TITLE_BAR_GLYPH_RESTORE_OFFSET = "title-bar.glyph.restore-offset": Dimension = px(3.0),
        range(1.0, 16.0);
    /// Title bar: how far from the window's edges a press resizes it.
    TITLE_BAR_RESIZE_BORDER_DEFAULT = "title-bar.resize-border.default": Dimension = px(5.0),
        range(1.0, 32.0);
    /// Title bar: space between the sides of a menu's title and its label.
    TITLE_BAR_MENU_PADDING_DEFAULT = "title-bar.menu.padding.default": Dimension = px(8.0),
        range(0.0, 32.0);
    /// Title bar: corner radius of a menu's title.
    TITLE_BAR_MENU_CORNER_RADIUS_DEFAULT = "title-bar.menu.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
}
