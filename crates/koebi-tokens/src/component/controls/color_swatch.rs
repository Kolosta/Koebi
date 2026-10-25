//! Color swatches.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group COLOR_SWATCH: Component, start = RESIZE_GRIP::END;

    /// Color swatch: corner radius.
    COLOR_SWATCH_CORNER_RADIUS_DEFAULT = "color-swatch.corner-radius.default": Dimension ->
        semantic::RADIUS_XS;
    /// Color swatch: border color.
    COLOR_SWATCH_BORDER_COLOR_DEFAULT = "color-swatch.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Color swatch: border color, hovered.
    COLOR_SWATCH_BORDER_COLOR_HOVER = "color-swatch.border.color.hover": Color ->
        semantic::BORDER_COLOR_STRONG_DEFAULT;
    /// Color swatch: border color, selected.
    COLOR_SWATCH_BORDER_COLOR_SELECTED = "color-swatch.border.color.selected": Color ->
        semantic::BORDER_COLOR_FOCUS_DEFAULT;
    /// Color swatch: border width.
    COLOR_SWATCH_BORDER_WIDTH_DEFAULT = "color-swatch.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Color swatch: border width, selected.
    COLOR_SWATCH_BORDER_WIDTH_SELECTED = "color-swatch.border.width.selected": Dimension ->
        semantic::BORDER_WIDTH_MEDIUM;
    /// Color swatch: light squares of the checkerboard behind translucent colors.
    COLOR_SWATCH_CHECKER_COLOR_LIGHT = "color-swatch.checker.color.light": Color ->
        semantic::NEUTRAL_COLOR_500;
    /// Color swatch: dark squares of the checkerboard behind translucent colors.
    COLOR_SWATCH_CHECKER_COLOR_DARK = "color-swatch.checker.color.dark": Color ->
        semantic::NEUTRAL_COLOR_700;
    /// Color swatch: size of the checkerboard squares.
    COLOR_SWATCH_CHECKER_SIZE_DEFAULT = "color-swatch.checker.size.default": Dimension =
        px(6.0), range(2.0, 32.0);
}
