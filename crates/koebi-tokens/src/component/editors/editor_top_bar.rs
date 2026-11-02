//! The top bar of editors.

use super::*;
use koebi_design::define::*;

koebi_design::tokens! {
    group EDITOR_TOP_BAR: Component, start = EDITOR::END;

    /// Editor top bar: space between its sides and its items, one interface unit high.
    EDITOR_TOP_BAR_PADDING_DEFAULT = "editor-top-bar.padding.default": Dimension =
        px(2.0), range(0.0, 24.0);
    /// Editor top bar: space between its items.
    EDITOR_TOP_BAR_GAP_DEFAULT = "editor-top-bar.gap.default": Dimension =
        px(4.0), range(0.0, 24.0);
}
