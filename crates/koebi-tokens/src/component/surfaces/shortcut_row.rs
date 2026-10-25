//! Rows of the shortcut editor.

use super::*;
use crate::semantic;

koebi_design::tokens! {
    group SHORTCUT_ROW: Component, start = SECTION_HEADER::END;

    /// Shortcut row: background color, hovered.
    SHORTCUT_ROW_BACKGROUND_COLOR_HOVER = "shortcut-row.background.color.hover": Color ->
        semantic::BACKGROUND_RAISED_DEFAULT;
    /// Shortcut row: background color, selected.
    SHORTCUT_ROW_BACKGROUND_COLOR_SELECTED = "shortcut-row.background.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
}
