//! Editors: the content of a zone.

use crate::semantic;

koebi_design::tokens! {
    group EDITOR: Component, start = crate::component::CURSOR::END;

    /// Editor: background color.
    EDITOR_BACKGROUND_COLOR_DEFAULT = "editor.background.color.default": Color ->
        semantic::BACKGROUND_SURFACE_CANVAS;
    /// Editor: top bar background color.
    EDITOR_TOP_BAR_BACKGROUND_COLOR_DEFAULT = "editor.top-bar.background.color.default": Color ->
        semantic::BACKGROUND_SURFACE_RAISED;
    /// Editor: content inset.
    EDITOR_CONTENT_INSET_DEFAULT = "editor.content.inset.default": Dimension ->
        semantic::SPACING_EDITOR_INSET;
    /// Corner radius of an editor's area.
    EDITOR_CORNER_RADIUS_DEFAULT = "editor.corner-radius.default": Dimension ->
        semantic::RADIUS_M;
}
