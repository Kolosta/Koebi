//! Sliders.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group SLIDER: Component, start = CHECKBOX::END;

    /// Slider: grab color.
    SLIDER_GRAB_COLOR_DEFAULT = "slider.grab.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Slider: grab color, pressed.
    SLIDER_GRAB_COLOR_PRESSED = "slider.grab.color.pressed": Color ->
        semantic::BACKGROUND_ACCENT_PRESSED;
    /// Slider: corner radius.
    SLIDER_CORNER_RADIUS_DEFAULT = "slider.corner-radius.default": Dimension -> semantic::RADIUS_M;
    /// Slider: grab color, hovered.
    SLIDER_GRAB_COLOR_HOVER = "slider.grab.color.hover": Color -> semantic::BACKGROUND_ACCENT_HOVER;
    /// Slider: grab width.
    SLIDER_GRAB_WIDTH_DEFAULT = "slider.grab.width.default": Dimension =
        px(10.0), range(2.0, 40.0);
    /// Slider: space between the grab and the track edges.
    SLIDER_GRAB_PADDING_DEFAULT = "slider.grab.padding.default": Dimension =
        px(2.0), range(0.0, 10.0);
    /// Slider: track color.
    SLIDER_TRACK_COLOR_DEFAULT = "slider.track.color.default": Color ->
        semantic::BACKGROUND_APP_FRAME;
    /// Slider: track color, hovered.
    SLIDER_TRACK_COLOR_HOVER = "slider.track.color.hover": Color ->
        semantic::BACKGROUND_APP_FRAME_HOVER;
    /// Slider: track border color.
    SLIDER_TRACK_BORDER_COLOR_DEFAULT = "slider.track.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Slider: track border width.
    SLIDER_TRACK_BORDER_WIDTH_DEFAULT = "slider.track.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
}
