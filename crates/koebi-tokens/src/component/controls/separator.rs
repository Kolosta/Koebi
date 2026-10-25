//! Separators.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group SEPARATOR: Component, start = KEY_CAP::END;

    /// Separator: color.
    SEPARATOR_COLOR_DEFAULT = "separator.color.default": Color -> semantic::BORDER_COLOR_DEFAULT;
    /// Separator: color, hovered.
    SEPARATOR_COLOR_HOVER = "separator.color.hover": Color -> semantic::BACKGROUND_ACCENT_PRESSED;
    /// Separator: color, pressed.
    SEPARATOR_COLOR_PRESSED = "separator.color.pressed": Color ->
        semantic::BACKGROUND_ACCENT_PRESSED;
    /// Separator: size.
    SEPARATOR_SIZE_DEFAULT = "separator.size.default": Dimension = px(1.0), range(1.0, 10.0);
    /// Separator: text border width.
    SEPARATOR_TEXT_BORDER_WIDTH_DEFAULT = "separator.text.border.width.default": Dimension =
        px(3.0), range(0.0, 10.0);
}
