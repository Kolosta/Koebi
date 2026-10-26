//! The status bar.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group STATUS_BAR: Component, start = TOOLTIP::END;

    /// Status bar: background color.
    STATUS_BAR_BACKGROUND_COLOR_DEFAULT = "status-bar.background.color.default": Color ->
        semantic::BACKGROUND_RAISED_DEFAULT;
    /// Status bar: label color.
    STATUS_BAR_LABEL_COLOR_DEFAULT = "status-bar.label.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Status bar: height.
    STATUS_BAR_SIZE_HEIGHT = "status-bar.size.height": Dimension = px(22.0), range(16.0, 40.0);
    /// Status bar: horizontal padding.
    STATUS_BAR_PADDING_HORIZONTAL_DEFAULT = "status-bar.padding.horizontal.default": Dimension =
        px(4.0), range(0.0, 32.0);
    /// Status bar: vertical padding.
    STATUS_BAR_PADDING_VERTICAL_DEFAULT = "status-bar.padding.vertical.default": Dimension =
        px(2.0), range(0.0, 32.0);
    /// Status bar: gap.
    STATUS_BAR_GAP_DEFAULT = "status-bar.gap.default": Dimension = px(12.0), range(0.0, 48.0);
}
