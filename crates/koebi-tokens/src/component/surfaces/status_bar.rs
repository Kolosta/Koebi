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
    /// Status bar: height of what it holds (shortcuts, the version), one interface unit; the
    /// bar adds its vertical padding above and below.
    STATUS_BAR_SIZE_HEIGHT = "status-bar.size.height": Dimension ->
        semantic::SIZE_CONTROL_HEIGHT;
    /// Status bar: horizontal padding.
    STATUS_BAR_PADDING_HORIZONTAL_DEFAULT = "status-bar.padding.horizontal.default": Dimension =
        px(4.0), range(0.0, 32.0);
    /// Status bar: vertical padding, the inset around what it holds.
    STATUS_BAR_PADDING_VERTICAL_DEFAULT = "status-bar.padding.vertical.default": Dimension ->
        semantic::SPACING_CONTROL_INSET;
    /// Status bar: gap.
    STATUS_BAR_GAP_DEFAULT = "status-bar.gap.default": Dimension = px(12.0), range(0.0, 48.0);
    /// Frame times, above the status bar: background color, the theme's deepest surface, made
    /// translucent by the opacity below.
    STATUS_BAR_FRAME_TIMES_BACKGROUND_COLOR_DEFAULT =
        "status-bar.frame-times.background.color.default": Color ->
        semantic::BACKGROUND_SUNKEN_DEFAULT;
    /// Frame times: opacity of the background, letting what lies below show through.
    STATUS_BAR_FRAME_TIMES_OPACITY_DEFAULT = "status-bar.frame-times.opacity.default": Number =
        0.72, range(0.1, 1.0);
    /// Frame times: text color.
    STATUS_BAR_FRAME_TIMES_LABEL_COLOR_DEFAULT =
        "status-bar.frame-times.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Frame times: corner radius of the background.
    STATUS_BAR_FRAME_TIMES_CORNER_RADIUS_DEFAULT =
        "status-bar.frame-times.corner-radius.default": Dimension -> semantic::RADIUS_CONTROL;
    /// Frame times: horizontal padding around the text.
    STATUS_BAR_FRAME_TIMES_PADDING_HORIZONTAL_DEFAULT =
        "status-bar.frame-times.padding.horizontal.default": Dimension = px(8.0),
        range(0.0, 32.0);
    /// Frame times: vertical padding around the text.
    STATUS_BAR_FRAME_TIMES_PADDING_VERTICAL_DEFAULT =
        "status-bar.frame-times.padding.vertical.default": Dimension = px(6.0),
        range(0.0, 32.0);
    /// Frame times: space from the left edge and from the status bar.
    STATUS_BAR_FRAME_TIMES_MARGIN_DEFAULT = "status-bar.frame-times.margin.default": Dimension ->
        semantic::SPACING_EDITOR_INSET;
}
