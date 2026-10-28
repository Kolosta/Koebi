//! Property groups: labeled inputs in two columns.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group PROPERTY_GROUP: Component, start = SIDE_PANEL::END;

    /// Property group: gap between two sub-groups.
    PROPERTY_GROUP_GAP_SIZE_DEFAULT = "property-group.gap.size.default": Dimension =
        px(10.0), range(0.0, 40.0);
    /// Property group: gap between two rows of a sub-group.
    PROPERTY_GROUP_ROW_GAP_DEFAULT = "property-group.row.gap.default": Dimension = px(2.0),
        range(0.0, 16.0);
    /// Property group: height of a row.
    PROPERTY_GROUP_ROW_HEIGHT_DEFAULT = "property-group.row.height.default": Dimension ->
        semantic::SIZE_CONTROL_HEIGHT;
    /// Property group: share of the width the labels take, before the inputs.
    PROPERTY_GROUP_LABEL_WIDTH_RATIO = "property-group.label.width-ratio": Number = 0.35,
        range(0.0, 1.0);
    /// Property group: gap between a label and its input.
    PROPERTY_GROUP_COLUMN_GAP_DEFAULT = "property-group.column.gap.default": Dimension =
        px(8.0), range(0.0, 32.0);
    /// Property group: label color.
    PROPERTY_GROUP_LABEL_COLOR_DEFAULT = "property-group.label.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Property group: label color of disabled rows.
    PROPERTY_GROUP_LABEL_COLOR_DISABLED = "property-group.label.color.disabled": Color ->
        semantic::TEXT_COLOR_DISABLED_DEFAULT;
}
