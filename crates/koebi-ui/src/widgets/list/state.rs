//! [`ListState`]: what a list keeps between frames, and the selection rules.

use std::collections::HashSet;

use core::time::Duration;

use koebi_core::Vec2;

use super::layout::{Group, Preview, Row, subtree_end};
use super::{DropPlace, ListItem, ListSelection};
use crate::widgets::ScrollState;

/// An opening or closing fold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Fold {
    pub(super) id: u64,
    /// Openness when the animation started, from 0 (folded) to 1 (unfolded).
    pub(super) from: f32,
    /// Openness now.
    pub(super) current: f32,
    /// When the animation started: the first frame that lays the list out after the change.
    pub(super) start: Option<Duration>,
}

/// A press on a row that may become a drag.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Drag {
    /// The row pressed: it holds the pointer until the release.
    pub(super) row: u64,
    /// Where the press happened.
    pub(super) origin: Vec2,
    /// Whether the row was selected: the drag carries the selection, or the row alone.
    pub(super) selection: bool,
    /// Past the threshold: rows are being dragged.
    pub(super) started: bool,
    /// Cancelled: the release moves nothing.
    pub(super) cancelled: bool,
    /// Where the rows would go, and its preview.
    pub(super) target: Option<(DropPlace, Preview)>,
    /// When scrolling at the edges last advanced.
    pub(super) scrolled_at: Duration,
}

impl Drag {
    pub(super) fn new(row: u64, origin: Vec2, selection: bool, now: Duration) -> Self {
        Self {
            row,
            origin,
            selection,
            started: false,
            cancelled: false,
            target: None,
            scrolled_at: now,
        }
    }
}

/// What a list keeps between frames: folded rows and their animations, the selection and the
/// active row, the drag in progress, the scroll position, and the layout of the last frame.
/// Keep one per list.
///
/// Rows are referred to by the application's keys ([`ListItem::id`]); call
/// [`ListState::retain`] when rows go away for good.
#[derive(Clone, Debug, Default)]
pub struct ListState {
    pub(super) scroll: ScrollState,
    pub(super) folded: HashSet<u64>,
    pub(super) folds: Vec<Fold>,
    pub(super) selected: HashSet<u64>,
    pub(super) active: Option<u64>,
    /// Where Shift ranges start: the row clicked last without Shift.
    pub(super) anchor: Option<u64>,
    pub(super) drag: Option<Drag>,
    /// The rows of the current or last drag, and their subtrees as item index ranges.
    pub(super) dragged: Vec<u64>,
    pub(super) dragged_ranges: Vec<(usize, usize)>,
    /// The last click, for double clicks.
    pub(super) last_click: Option<(u64, Duration)>,
    /// The layout of this frame (see the `layout` module), its height, and scratch space.
    pub(super) rows: Vec<Row>,
    pub(super) height: f32,
    pub(super) groups: Vec<Group>,
    pub(super) stack: Vec<usize>,
}

impl ListState {
    /// Nothing folded, selected or scrolled.
    pub fn new() -> Self {
        Self::default()
    }

    /// `true` when `id` is folded: its children are hidden.
    pub fn is_folded(&self, id: u64) -> bool {
        self.folded.contains(&id)
    }

    /// Folds or unfolds `id` at once, without animation: restoring a saved state.
    pub fn set_folded(&mut self, id: u64, folded: bool) {
        self.folds.retain(|fold| fold.id != id);
        if folded {
            self.folded.insert(id);
        } else {
            self.folded.remove(&id);
        }
    }

    /// `true` when `id` is selected.
    pub fn is_selected(&self, id: u64) -> bool {
        self.selected.contains(&id)
    }

    /// The selected rows, in no particular order.
    pub fn selected(&self) -> impl Iterator<Item = u64> + '_ {
        self.selected.iter().copied()
    }

    /// Selects `id` or deselects it, leaving the others as they are.
    pub fn set_selected(&mut self, id: u64, selected: bool) {
        if selected {
            self.selected.insert(id);
        } else {
            self.selected.remove(&id);
        }
    }

    /// Deselects every row; no row is active anymore.
    pub fn clear_selection(&mut self) {
        self.selected.clear();
        self.active = None;
    }

    /// The active row: the one clicked last, the one the keyboard moves from.
    pub fn active(&self) -> Option<u64> {
        self.active
    }

    /// Makes `id` the active row (selecting it), or no row.
    pub fn set_active(&mut self, id: Option<u64>) {
        if let Some(id) = id {
            self.selected.insert(id);
            self.anchor = Some(id);
        }
        self.active = id;
    }

    /// The rows of the drag in progress, or of the last drop, in display order: a dragged
    /// selection lists its topmost rows (their children move with them).
    pub fn dragged(&self) -> &[u64] {
        &self.dragged
    }

    /// `true` while rows are being dragged.
    pub fn is_dragging(&self) -> bool {
        self.drag
            .is_some_and(|drag| drag.started && !drag.cancelled)
    }

    /// The scroll position: read it, or scroll the list.
    pub fn scroll(&mut self) -> &mut ScrollState {
        &mut self.scroll
    }

    /// Forgets the rows `keep` refuses: deleted rows leave the selection and the folds.
    pub fn retain(&mut self, mut keep: impl FnMut(u64) -> bool) {
        self.selected.retain(|id| keep(*id));
        self.folded.retain(|id| keep(*id));
        self.folds.retain(|fold| keep(fold.id));
        for row in [&mut self.active, &mut self.anchor] {
            if row.is_some_and(|id| !keep(id)) {
                *row = None;
            }
        }
    }

    /// Folds or unfolds `id` with an animation, from where a running one is.
    pub(super) fn fold_animated(&mut self, id: u64, folded: bool) {
        if self.folded.contains(&id) == folded {
            return;
        }
        let from = match self.folds.iter().position(|fold| fold.id == id) {
            Some(index) => self.folds.swap_remove(index).current,
            None if folded => 1.0,
            None => 0.0,
        };
        self.set_folded(id, folded);
        self.folds.push(Fold {
            id,
            from,
            current: from,
            start: None,
        });
    }

    /// Folds or unfolds the row at `index` with an animation, and every row of its subtree
    /// at once.
    pub(super) fn fold_subtree(&mut self, items: &[ListItem<'_>], index: usize, folded: bool) {
        let end = subtree_end(items, index);
        for inner in index + 1..end {
            let is_parent = items
                .get(inner + 1)
                .is_some_and(|next| next.depth > items[inner].depth);
            if is_parent {
                self.set_folded(items[inner].id, folded);
            }
        }
        self.fold_animated(items[index].id, folded);
    }

    /// The rows shown and not leaving with a closing fold, in display order: the rows the
    /// keyboard and ranges walk through.
    pub(super) fn visible(&self) -> impl Iterator<Item = &Row> + '_ {
        self.rows.iter().filter(|row| !row.leaving)
    }

    /// The position of `id` among [`ListState::visible`] rows.
    fn position(&self, id: u64) -> Option<usize> {
        self.visible().position(|row| row.id == id)
    }

    /// Selects `id` alone and makes it active. Returns whether anything changed.
    pub(super) fn select_only(&mut self, id: u64) -> bool {
        let unchanged =
            self.selected.len() == 1 && self.selected.contains(&id) && self.active == Some(id);
        self.selected.clear();
        self.selected.insert(id);
        self.active = Some(id);
        self.anchor = Some(id);
        !unchanged
    }

    /// Adds `id` to the selection and makes it active, or removes it.
    pub(super) fn toggle(&mut self, id: u64) -> bool {
        if self.selected.remove(&id) {
            if self.active == Some(id) {
                self.active = None;
            }
        } else {
            self.selected.insert(id);
            self.active = Some(id);
        }
        self.anchor = Some(id);
        true
    }

    /// Selects the visible rows from the anchor to `to` (added to the selection with `add`)
    /// and makes `to` active.
    pub(super) fn select_range(&mut self, to: u64, add: bool) -> bool {
        let (Some(start), Some(end)) = (
            self.anchor.and_then(|anchor| self.position(anchor)),
            self.position(to),
        ) else {
            return self.select_only(to);
        };
        if !add {
            self.selected.clear();
        }
        let (first, last) = (start.min(end), start.max(end));
        let rows = self.rows.iter().filter(|row| !row.leaving);
        self.selected
            .extend(rows.skip(first).take(last - first + 1).map(|row| row.id));
        self.active = Some(to);
        true
    }

    /// Selects every row. Returns whether a row was added.
    pub(super) fn select_all(&mut self, items: &[ListItem<'_>]) -> bool {
        let before = self.selected.len();
        self.selected.extend(items.iter().map(|item| item.id));
        if self.active.is_none() {
            self.active = items.first().map(|item| item.id);
        }
        self.selected.len() != before
    }

    /// Moves the active row to the next visible row (or the previous one), selecting it
    /// alone, or extending the selection from the anchor.
    pub(super) fn step(&mut self, forward: bool, extend: bool, selection: ListSelection) -> bool {
        if selection == ListSelection::None {
            return false;
        }
        let count = self.visible().count();
        if count == 0 {
            return false;
        }
        let target = match self.active.and_then(|id| self.position(id)) {
            Some(index) if forward => (index + 1).min(count - 1),
            Some(index) => index.saturating_sub(1),
            None if forward => 0,
            None => count - 1,
        };
        let Some(id) = self.visible().nth(target).map(|row| row.id) else {
            return false;
        };
        if extend && selection == ListSelection::Multiple {
            self.select_range(id, false)
        } else {
            self.select_only(id)
        }
    }

    /// Unfolds the active row, or goes to its first child when it is unfolded.
    pub(super) fn unfold_or_enter(
        &mut self,
        items: &[ListItem<'_>],
        selection: ListSelection,
    ) -> bool {
        let Some(row) = self.active_row() else {
            return false;
        };
        let item = &items[row.item];
        if !row.parent {
            return false;
        }
        if self.folded.contains(&item.id) {
            self.fold_animated(item.id, false);
            return false;
        }
        match items.get(row.item + 1) {
            Some(child) if selection != ListSelection::None => self.select_only(child.id),
            _ => false,
        }
    }

    /// Folds the active row, or goes to its parent when it is folded or has no children.
    pub(super) fn fold_or_leave(
        &mut self,
        items: &[ListItem<'_>],
        selection: ListSelection,
    ) -> bool {
        let Some(row) = self.active_row() else {
            return false;
        };
        let item = &items[row.item];
        if row.parent && !self.folded.contains(&item.id) {
            self.fold_animated(item.id, true);
            return false;
        }
        let parent = items[..row.item]
            .iter()
            .rev()
            .find(|other| other.depth < item.depth);
        match parent {
            Some(parent) if selection != ListSelection::None => self.select_only(parent.id),
            _ => false,
        }
    }

    /// The active row, when it is visible.
    fn active_row(&self) -> Option<Row> {
        let id = self.active?;
        self.visible().find(|row| row.id == id).copied()
    }

    /// Scrolls the least that shows the active row in a viewport `view_height` high.
    pub(super) fn reveal_active(&mut self, row_height: f32, view_height: f32) {
        let Some(top) = self.active_row().map(|row| row.top) else {
            return;
        };
        let offset = self.scroll.offset();
        let y = if top < offset.y {
            top
        } else if top + row_height > offset.y + view_height {
            top + row_height - view_height
        } else {
            return;
        };
        self.scroll.set_offset(Vec2::new(offset.x, y.max(0.0)));
    }

    /// Lists the dragged rows: the selection's topmost rows when the press was on a selected
    /// row, else the pressed row; and the item ranges of their subtrees (in a `tree`).
    pub(super) fn collect_dragged(
        &mut self,
        items: &[ListItem<'_>],
        tree: bool,
        pressed: u64,
        selection: bool,
    ) {
        self.dragged.clear();
        self.dragged_ranges.clear();
        let mut index = 0;
        while index < items.len() {
            let id = items[index].id;
            let taken = if selection {
                self.selected.contains(&id)
            } else {
                id == pressed
            };
            if taken {
                let end = if tree {
                    subtree_end(items, index)
                } else {
                    index + 1
                };
                self.dragged.push(id);
                self.dragged_ranges.push((index, end));
                index = end;
            } else {
                index += 1;
            }
        }
    }
}
