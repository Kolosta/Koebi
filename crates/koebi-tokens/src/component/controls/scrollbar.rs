//! Scrollbars, classic and overlay.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group SCROLLBAR: Component, start = SEPARATOR::END;

    /// Scrollbar: background color.
    SCROLLBAR_BACKGROUND_COLOR_DEFAULT = "scrollbar.background.color.default": Color ->
        semantic::BACKGROUND_SUNKEN_DEFAULT;
    /// Scrollbar: grab color.
    SCROLLBAR_GRAB_COLOR_DEFAULT = "scrollbar.grab.color.default": Color ->
        semantic::BORDER_COLOR_SUBTLE_DEFAULT;
    /// Scrollbar: grab color, hovered.
    SCROLLBAR_GRAB_COLOR_HOVER = "scrollbar.grab.color.hover": Color ->
        semantic::BORDER_COLOR_STRONG_DEFAULT;
    /// Scrollbar: grab color, pressed.
    SCROLLBAR_GRAB_COLOR_PRESSED = "scrollbar.grab.color.pressed": Color ->
        semantic::TEXT_COLOR_TERTIARY_DEFAULT;
    /// Scrollbar: size.
    SCROLLBAR_SIZE_DEFAULT = "scrollbar.size.default": Dimension = px(14.0), range(1.0, 32.0);
    /// Scrollbar: corner radius.
    SCROLLBAR_CORNER_RADIUS_DEFAULT = "scrollbar.corner-radius.default": Dimension ->
        semantic::RADIUS_M;
    /// Scrollbar: padding.
    SCROLLBAR_PADDING_DEFAULT = "scrollbar.padding.default": Dimension = px(2.0), range(0.0, 10.0);
    /// Scrollbar: overlay margin.
    SCROLLBAR_OVERLAY_MARGIN_DEFAULT = "scrollbar.overlay.margin.default": Dimension =
        px(7.0), range(0.0, 20.0);
    /// Scrollbar: overlay width, at rest.
    SCROLLBAR_OVERLAY_WIDTH_REST = "scrollbar.overlay.width.rest": Dimension =
        px(3.0), range(0.0, 20.0);
    /// Scrollbar: overlay width, hovered.
    SCROLLBAR_OVERLAY_WIDTH_HOVER = "scrollbar.overlay.width.hover": Dimension =
        px(5.0), range(0.0, 20.0);
    /// Scrollbar: overlay proximity.
    SCROLLBAR_OVERLAY_PROXIMITY_DEFAULT = "scrollbar.overlay.proximity.default": Dimension =
        px(48.0), range(0.0, 200.0);
    /// Scrollbar: time the grab takes to grow from rest to its full width, or back.
    SCROLLBAR_OVERLAY_GROW_DURATION = "scrollbar.overlay.grow.duration": Duration ->
        semantic::ANIMATION_DURATION_GROW;
    /// Scrollbar: curve of the grab growing and shrinking.
    SCROLLBAR_OVERLAY_GROW_CURVE = "scrollbar.overlay.grow.curve": Curve ->
        semantic::ANIMATION_CURVE_GROW;
    /// Scrollbar: overlay padding.
    SCROLLBAR_OVERLAY_PADDING_DEFAULT = "scrollbar.overlay.padding.default": Dimension =
        px(4.0), range(0.0, 20.0);
    /// How far one notch of the wheel scrolls.
    SCROLLBAR_WHEEL_STEP_DEFAULT = "scrollbar.wheel-step.default": Dimension =
        px(48.0), range(1.0, 400.0);
}
