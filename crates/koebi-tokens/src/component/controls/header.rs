//! Collapsible headers.

use super::*;
use crate::semantic;

koebi_design::tokens! {
    group HEADER: Component, start = TAB::END;

    /// Header: background color.
    HEADER_BACKGROUND_COLOR_DEFAULT = "header.background.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Header: background color, hovered.
    HEADER_BACKGROUND_COLOR_HOVER = "header.background.color.hover": Color ->
        semantic::BACKGROUND_ACCENT_HOVER;
    /// Header: background color, pressed.
    HEADER_BACKGROUND_COLOR_PRESSED = "header.background.color.pressed": Color ->
        semantic::BACKGROUND_ACCENT_PRESSED;
}
