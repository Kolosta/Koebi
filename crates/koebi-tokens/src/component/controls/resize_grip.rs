//! Resize grips.

use super::*;
use crate::semantic;

koebi_design::tokens! {
    group RESIZE_GRIP: Component, start = HEADER::END;

    /// Resize grip: color.
    RESIZE_GRIP_COLOR_DEFAULT = "resize-grip.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Resize grip: color, hovered.
    RESIZE_GRIP_COLOR_HOVER = "resize-grip.color.hover": Color -> semantic::BACKGROUND_ACCENT_HOVER;
    /// Resize grip: color, pressed.
    RESIZE_GRIP_COLOR_PRESSED = "resize-grip.color.pressed": Color ->
        semantic::BACKGROUND_ACCENT_PRESSED;
}
