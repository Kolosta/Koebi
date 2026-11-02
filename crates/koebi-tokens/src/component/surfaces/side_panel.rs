//! Side panels.

use super::*;
use koebi_design::define::*;

koebi_design::tokens! {
    group SIDE_PANEL: Component, start = PANEL::END;

    /// Side panel: margin.
    SIDE_PANEL_MARGIN_DEFAULT = "side-panel.margin.default": Dimension = px(8.0), range(0.0, 32.0);
    /// Side panel: width of a column beside the content.
    SIDE_PANEL_WIDTH_DEFAULT = "side-panel.width.default": Dimension = px(160.0),
        range(80.0, 480.0);
}
