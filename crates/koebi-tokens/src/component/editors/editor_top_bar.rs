//! The top bar of editors.

use super::*;
use koebi_design::define::*;

koebi_design::tokens! {
    group EDITOR_TOP_BAR: Component, start = EDITOR::END;

    /// Editor top bar: padding.
    EDITOR_TOP_BAR_PADDING_DEFAULT = "editor-top-bar.padding.default": Dimension =
        px(6.0), range(0.0, 24.0);
}
