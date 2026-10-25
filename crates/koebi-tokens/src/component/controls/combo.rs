//! Combo boxes and their popups.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group COMBO: Component, start = DROPDOWN::END;

    /// Combo: background color.
    COMBO_BACKGROUND_COLOR_DEFAULT = "combo.background.color.default": Color ->
        semantic::BACKGROUND_SUNKEN_DEFAULT;
    /// Combo: background color, hovered.
    COMBO_BACKGROUND_COLOR_HOVER = "combo.background.color.hover": Color ->
        semantic::BACKGROUND_SUNKEN_DEFAULT;
    /// Combo: background color, pressed.
    COMBO_BACKGROUND_COLOR_PRESSED = "combo.background.color.pressed": Color ->
        semantic::BACKGROUND_RAISED_DEFAULT;
    /// Combo: preview label color.
    COMBO_PREVIEW_LABEL_COLOR_DEFAULT = "combo.preview.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Combo: arrow icon color.
    COMBO_ARROW_ICON_COLOR_DEFAULT = "combo.arrow.icon.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Combo: popup background color.
    COMBO_POPUP_BACKGROUND_COLOR_DEFAULT = "combo.popup.background.color.default": Color ->
        semantic::BACKGROUND_FLOATING_DEFAULT;
    /// Combo: item background color, hovered.
    COMBO_ITEM_BACKGROUND_COLOR_HOVER = "combo.item.background.color.hover": Color ->
        semantic::BACKGROUND_ACCENT_HOVER;
    /// Combo: item background color, selected.
    COMBO_ITEM_BACKGROUND_COLOR_SELECTED = "combo.item.background.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Combo: border color.
    COMBO_BORDER_COLOR_DEFAULT = "combo.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Combo: corner radius.
    COMBO_CORNER_RADIUS_DEFAULT = "combo.corner-radius.default": Dimension -> semantic::RADIUS_M;
    /// Combo: border width.
    COMBO_BORDER_WIDTH_DEFAULT = "combo.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Combo: horizontal padding.
    COMBO_PADDING_HORIZONTAL_DEFAULT = "combo.padding.horizontal.default": Dimension = px(4.0);
    /// Combo: vertical padding.
    COMBO_PADDING_VERTICAL_DEFAULT = "combo.padding.vertical.default": Dimension = px(4.0);
}
