//! Theme cards: previews of a theme.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group THEME_CARD: Component, start = COLOR_SWATCH::END;

    /// Theme card: corner radius.
    THEME_CARD_CORNER_RADIUS_DEFAULT = "theme-card.corner-radius.default": Dimension ->
        semantic::RADIUS_M;
    /// Theme card: border color.
    THEME_CARD_BORDER_COLOR_DEFAULT = "theme-card.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Theme card: border color, hovered.
    THEME_CARD_BORDER_COLOR_HOVER = "theme-card.border.color.hover": Color ->
        semantic::BORDER_COLOR_STRONG_DEFAULT;
    /// Theme card: border color of the active theme's card.
    THEME_CARD_BORDER_COLOR_SELECTED = "theme-card.border.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Theme card: border width.
    THEME_CARD_BORDER_WIDTH_DEFAULT = "theme-card.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Theme card: border width of the active theme's card.
    THEME_CARD_BORDER_WIDTH_SELECTED = "theme-card.border.width.selected": Dimension ->
        semantic::BORDER_WIDTH_MEDIUM;
    /// Theme card: space between the border and the preview.
    THEME_CARD_PADDING_DEFAULT = "theme-card.padding.default": Dimension =
        px(4.0), range(0.0, 16.0);
}
