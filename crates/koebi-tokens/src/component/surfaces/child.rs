//! Child areas nested in a window.

use super::*;
use crate::semantic;

koebi_design::tokens! {
    group CHILD: Component, start = POPUP::END;

    /// Child: background color.
    CHILD_BACKGROUND_COLOR_DEFAULT = "child.background.color.default": Color ->
        semantic::BACKGROUND_APP_CHILD;
    /// Child: background opacity.
    CHILD_BACKGROUND_OPACITY_DEFAULT = "child.background.opacity.default": Number ->
        semantic::OPACITY_TRANSPARENT;
    /// Child: corner radius.
    CHILD_CORNER_RADIUS_DEFAULT = "child.corner-radius.default": Dimension -> semantic::RADIUS_M;
    /// Child: border width.
    CHILD_BORDER_WIDTH_DEFAULT = "child.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
}
