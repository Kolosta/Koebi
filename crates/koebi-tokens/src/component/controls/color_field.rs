//! Color fields: a color shown one interface unit high, opening a color picker.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group COLOR_FIELD: Component, start = THEME_CARD::END;

    /// Color field: outline color, a neutral gray marking the field against what lies behind
    /// when the colors are close, without taking the eye.
    COLOR_FIELD_OUTLINE_COLOR_DEFAULT = "color-field.outline.color.default": Color ->
        semantic::NEUTRAL_COLOR_500;
    /// Color field: outline color, hovered.
    COLOR_FIELD_OUTLINE_COLOR_HOVER = "color-field.outline.color.hover": Color ->
        semantic::NEUTRAL_COLOR_300;
    /// Color field: outline width, its own: the field keeps its outline when the borders of
    /// the interface go.
    COLOR_FIELD_OUTLINE_WIDTH_DEFAULT = "color-field.outline.width.default": Dimension =
        px(1.0), range(0.5, 4.0);
    /// Color field: corner radius.
    COLOR_FIELD_CORNER_RADIUS_DEFAULT = "color-field.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
    /// Color field: height, one interface unit.
    COLOR_FIELD_HEIGHT_DEFAULT = "color-field.height.default": Dimension ->
        semantic::SIZE_CONTROL_HEIGHT;
    /// Color field: side of the squares of the checkerboard behind transparency.
    COLOR_FIELD_CHECKER_SIZE_DEFAULT = "color-field.checker.size.default": Dimension =
        px(4.0), range(1.0, 32.0);
    /// Color field: light squares of the checkerboard.
    COLOR_FIELD_CHECKER_COLOR_LIGHT = "color-field.checker.color.light": Color ->
        COLOR_SWATCH_CHECKER_COLOR_LIGHT;
    /// Color field: dark squares of the checkerboard.
    COLOR_FIELD_CHECKER_COLOR_DARK = "color-field.checker.color.dark": Color ->
        COLOR_SWATCH_CHECKER_COLOR_DARK;
}
