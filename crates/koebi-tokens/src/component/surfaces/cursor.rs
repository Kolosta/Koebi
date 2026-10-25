//! Cursor icons.

use super::*;
use crate::semantic;

koebi_design::tokens! {
    group CURSOR: Component, start = DATA_VIZ::END;

    /// Cursor: icon color.
    CURSOR_ICON_COLOR_DEFAULT = "cursor.icon.color.default": Color ->
        semantic::STATIC_TEXT_COLOR_WHITE;
}
