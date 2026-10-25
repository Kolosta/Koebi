//! Rows of reorderable lists.

use super::*;
use crate::semantic;

koebi_design::tokens! {
    group LIST_ROW: Component, start = OUTLINER::END;

    /// List row: reorder duration.
    LIST_ROW_REORDER_DURATION = "list-row.reorder.duration": Duration ->
        semantic::ANIMATION_DURATION_ENTER;
    /// List row: drag background.
    LIST_ROW_DRAG_BACKGROUND = "list-row.drag.background": Color ->
        semantic::ACCENT_COLOR_HOVER_DEFAULT;
    /// List row: drag opacity.
    LIST_ROW_DRAG_OPACITY = "list-row.drag.opacity": Number = 0.55, range(0.0, 1.0);
    /// List row: drop slot background.
    LIST_ROW_DROP_SLOT_BACKGROUND = "list-row.drop-slot.background": Color ->
        semantic::BACKGROUND_NOTICE_DEFAULT;
}
