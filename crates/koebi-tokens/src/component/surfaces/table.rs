//! Tables.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group TABLE: Component, start = SHORTCUT::END;

    /// Table: header background color.
    TABLE_HEADER_BACKGROUND_COLOR_DEFAULT = "table.header.background.color.default": Color ->
        semantic::BACKGROUND_RAISED_DEFAULT;
    /// Table: border color strong.
    TABLE_BORDER_COLOR_STRONG = "table.border.color.strong": Color ->
        semantic::BORDER_COLOR_STRONG_DEFAULT;
    /// Table: border color light.
    TABLE_BORDER_COLOR_LIGHT = "table.border.color.light": Color -> semantic::BORDER_COLOR_DEFAULT;
    /// Table: row background color.
    TABLE_ROW_BACKGROUND_COLOR_DEFAULT = "table.row.background.color.default": Color =
        rgba_f32(0.0, 0.0, 0.0, 0.0);
    /// Table: row background color alt.
    TABLE_ROW_BACKGROUND_COLOR_ALT = "table.row.background.color.alt": Color =
        rgba_f32(1.0, 1.0, 1.0, 0.06);
}
