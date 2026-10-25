//! Section headers.

use super::*;
use crate::semantic;

koebi_design::tokens! {
    group SECTION_HEADER: Component, start = STATUS_BAR::END;

    /// Section header: label color.
    SECTION_HEADER_LABEL_COLOR_DEFAULT = "section-header.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Section header: font scale.
    SECTION_HEADER_FONT_SCALE_DEFAULT = "section-header.font-scale.default": Number =
        1.1, range(0.5, 2.5);
}
