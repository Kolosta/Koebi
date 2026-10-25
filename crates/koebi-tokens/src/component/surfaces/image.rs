//! Images.

use super::*;
use crate::semantic;

koebi_design::tokens! {
    group IMAGE: Component, start = TABLE::END;

    /// Image: corner radius.
    IMAGE_CORNER_RADIUS_DEFAULT = "image.corner-radius.default": Dimension -> semantic::RADIUS_M;
    /// Image: border width.
    IMAGE_BORDER_WIDTH_DEFAULT = "image.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_NONE;
}
