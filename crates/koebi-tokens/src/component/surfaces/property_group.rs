//! Property groups.

use super::*;
use koebi_design::define::*;

koebi_design::tokens! {
    group PROPERTY_GROUP: Component, start = SIDE_PANEL::END;

    /// Property group: gap size.
    PROPERTY_GROUP_GAP_SIZE_DEFAULT = "property-group.gap.size.default": Dimension =
        px(10.0), range(0.0, 40.0);
}
