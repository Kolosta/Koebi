//! Drop targets.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group DROP_TARGET: Component, start = ZONE_OVERLAY::END;

    /// Drop target: corner radius.
    DROP_TARGET_CORNER_RADIUS_DEFAULT = "drop-target.corner-radius.default": Dimension ->
        semantic::RADIUS_M;
    /// Drop target: border width.
    DROP_TARGET_BORDER_WIDTH_DEFAULT = "drop-target.border.width.default": Dimension =
        px(2.0), range(0.0, 4.0);
    /// Drop target: padding.
    DROP_TARGET_PADDING_DEFAULT = "drop-target.padding.default": Dimension =
        px(3.0), range(0.0, 16.0);
    /// Drop target: border color of the target dragged items would go into.
    DROP_TARGET_BORDER_COLOR_DEFAULT = "drop-target.border.color.default": Color ->
        semantic::BACKGROUND_DROP_TARGET_DEFAULT;
}
