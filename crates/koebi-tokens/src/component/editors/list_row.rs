//! Rows of lists and trees.

use super::*;
use crate::semantic;
use koebi_design::define::*;

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
    /// List row: height of a row, one interface unit; lists may ask for taller rows.
    LIST_ROW_HEIGHT_DEFAULT = "list-row.height.default": Dimension ->
        semantic::SIZE_CONTROL_HEIGHT;
    /// List row: indentation of each tree level, also the width of the fold arrow's slot.
    LIST_ROW_INDENT_DEFAULT = "list-row.indent.default": Dimension = px(16.0), range(0.0, 64.0);
    /// List row: space between the row's sides and its content.
    LIST_ROW_PADDING_HORIZONTAL_DEFAULT = "list-row.padding.horizontal.default": Dimension =
        px(4.0), range(0.0, 32.0);
    /// List row: space between the parts of a row (mark, label, count, end items).
    LIST_ROW_GAP_DEFAULT = "list-row.gap.default": Dimension = px(4.0), range(0.0, 32.0);
    /// List row: side of the fold arrow.
    LIST_ROW_FOLD_SIZE_DEFAULT = "list-row.fold.size.default": Dimension =
        px(10.0), range(4.0, 32.0);
    /// List row: color of the fold arrow.
    LIST_ROW_FOLD_COLOR_DEFAULT = "list-row.fold.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// List row: duration of a fold opening.
    LIST_ROW_EXPAND_DURATION = "list-row.expand.duration": Duration ->
        semantic::ANIMATION_DURATION_EXPAND;
    /// List row: duration of a fold closing.
    LIST_ROW_COLLAPSE_DURATION = "list-row.collapse.duration": Duration ->
        semantic::ANIMATION_DURATION_COLLAPSE;
    /// List row: side of a color mark.
    LIST_ROW_SWATCH_SIZE_DEFAULT = "list-row.swatch.size.default": Dimension =
        px(12.0), range(4.0, 32.0);
    /// List row: corner radius of a color mark.
    LIST_ROW_SWATCH_CORNER_RADIUS_DEFAULT = "list-row.swatch.corner-radius.default": Dimension ->
        semantic::RADIUS_XS;
    /// List row: color of the number of items a folded row hides.
    LIST_ROW_COUNT_COLOR_DEFAULT = "list-row.count.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// List row: color of the line where dragged rows would be inserted.
    LIST_ROW_DROP_LINE_COLOR_DEFAULT = "list-row.drop-line.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// List row: width of that line.
    LIST_ROW_DROP_LINE_WIDTH_DEFAULT = "list-row.drop-line.width.default": Dimension =
        px(2.0), range(1.0, 6.0);
}
