//! Charts: lines and histograms.

use super::*;
use crate::semantic;

koebi_design::tokens! {
    group DATA_VIZ: Component, start = IMAGE::END;

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
}
