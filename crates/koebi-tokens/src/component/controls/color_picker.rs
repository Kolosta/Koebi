//! Color pickers: the panel a color field opens, its area, strips, marks and swatches.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group COLOR_PICKER: Component, start = COLOR_FIELD::END;

    /// Color picker: side of its area of two components (a square, a wheel).
    COLOR_PICKER_AREA_SIZE_DEFAULT = "color-picker.area.size.default": Dimension =
        px(192.0), range(64.0, 512.0);
    /// Color picker: width of its strips (hue, opacity, value).
    COLOR_PICKER_STRIP_WIDTH_DEFAULT = "color-picker.strip.width.default": Dimension =
        px(16.0), range(8.0, 48.0);
    /// Color picker: width of its column of swatches (current, previous, original).
    COLOR_PICKER_SWATCH_WIDTH_DEFAULT = "color-picker.swatch.width.default": Dimension =
        px(40.0), range(16.0, 96.0);
    /// Color picker: space between its parts.
    COLOR_PICKER_GAP_DEFAULT = "color-picker.gap.default": Dimension = px(6.0),
        range(0.0, 24.0);
    /// Color picker: corner radius of its area, strips and swatches.
    COLOR_PICKER_CORNER_RADIUS_DEFAULT = "color-picker.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
    /// Color picker: side of the mark on its area.
    COLOR_PICKER_MARKER_SIZE_DEFAULT = "color-picker.marker.size.default": Dimension =
        px(12.0), range(6.0, 32.0);
    /// Color picker: color of the marks (the ring on the area, the triangles of the strips).
    COLOR_PICKER_MARKER_COLOR_DEFAULT = "color-picker.marker.color.default": Color ->
        semantic::STATIC_TEXT_COLOR_WHITE;
    /// Color picker: color of the thin outline around the marks.
    COLOR_PICKER_MARKER_OUTLINE_COLOR_DEFAULT = "color-picker.marker.outline.color.default":
        Color -> semantic::STATIC_TEXT_COLOR_BLACK;
    /// Color picker: width of the outline around the marks.
    COLOR_PICKER_MARKER_OUTLINE_WIDTH_DEFAULT = "color-picker.marker.outline.width.default":
        Dimension = px(1.0), range(0.5, 3.0);
    /// Color picker: depth of the triangles marking a strip, pointing inward.
    COLOR_PICKER_ARROW_SIZE_DEFAULT = "color-picker.arrow.size.default": Dimension = px(7.0),
        range(3.0, 16.0);
    /// Color picker: how far the triangles of a strip pass its sides, their outline included.
    COLOR_PICKER_ARROW_OVERHANG_DEFAULT = "color-picker.arrow.overhang.default": Dimension =
        px(2.0), range(0.0, 8.0);
    /// Color picker: share of the pointer's movement a drag takes while Shift is held.
    COLOR_PICKER_FINE_FACTOR_DEFAULT = "color-picker.fine-factor.default": Number = 0.1,
        range(0.01, 1.0);
}
