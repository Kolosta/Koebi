//! Zones: the areas the window is split into.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group ZONE: Component, start = EDITOR_TOP_BAR::END;

    /// Zone: separator size.
    ZONE_SEPARATOR_SIZE_DEFAULT = "zone.separator.size.default": Dimension =
        px(4.0), range(2.0, 20.0);
    /// Zone: separator color.
    ZONE_SEPARATOR_COLOR_DEFAULT = "zone.separator.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Zone: separator color continuation.
    ZONE_SEPARATOR_COLOR_CONTINUATION = "zone.separator.color.continuation": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Zone: separator continuation opacity.
    ZONE_SEPARATOR_CONTINUATION_OPACITY_DEFAULT =
        "zone.separator.continuation.opacity.default": Number -> semantic::OPACITY_REDUCED;
}
