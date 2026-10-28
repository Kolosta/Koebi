//! The layout of a list's rows, folds animating, and where dragged rows would go.
//!
//! Rows are laid out from the items in one pass. A folded row hides its subtree; a fold that
//! opens or closes lays its subtree out at its natural place but shows only the share of its
//! height it has reached: its rows are cut at that line and the rows after it move up by the
//! rest, so the subtree slides in or out like a drawer.

use core::time::Duration;

use koebi_core::Vec2;

use super::state::ListState;
use super::{DropPlace, ListDelegate, ListItem, ListOptions, ListStyle, Metrics};

/// A row as laid out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Row {
    /// The application's key of the row.
    pub(super) id: u64,
    /// Index of the row in the items.
    pub(super) item: usize,
    /// Top of the row, from the top of the content.
    pub(super) top: f32,
    /// The row is cut below this line, from the top of the content: a fold animating.
    pub(super) clip: f32,
    /// The innermost animating fold the row belongs to.
    pub(super) group: Option<usize>,
    /// How many rows the row hides while folded.
    pub(super) hidden: usize,
    /// Whether deeper rows follow the row: it folds.
    pub(super) parent: bool,
    /// The row belongs to a closing fold: it still shows, but no longer counts as visible.
    pub(super) leaving: bool,
    /// Openness of the row's fold, for its arrow: 0 folded, 1 unfolded.
    pub(super) openness: f32,
}

/// The subtree of a fold opening or closing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Group {
    /// Depth of the row that folds.
    depth: u16,
    /// Top of the subtree, from the top of the content.
    start: f32,
    /// Share of the subtree's height shown.
    openness: f32,
    /// Closing, or inside a closing fold.
    closing: bool,
    /// The line the subtree's rows are cut at, once known.
    bottom: f32,
    /// The animating fold this one is inside.
    parent: Option<usize>,
}

/// The preview of a drop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Preview {
    /// A line between rows, at `y` from the top of the content, indented for `depth`.
    Line { y: f32, depth: u16 },
    /// A frame around the row whose top is at `top`: the rows go into it.
    Frame { top: f32 },
}

/// The index after the subtree of the item at `index`: the next item that is not deeper.
pub(super) fn subtree_end(items: &[ListItem<'_>], index: usize) -> usize {
    let depth = items[index].depth;
    items[index + 1..]
        .iter()
        .position(|item| item.depth <= depth)
        .map_or(items.len(), |offset| index + 1 + offset)
}

/// The index of the ancestor of the item at `index` at `depth` (the item itself at its own
/// depth).
fn ancestor(items: &[ListItem<'_>], index: usize, depth: u16) -> usize {
    (0..=index)
        .rev()
        .find(|&candidate| items[candidate].depth <= depth)
        .unwrap_or(index)
}

/// `count` in decimal, written into `digits`.
pub(super) fn decimal(mut count: usize, digits: &mut [u8; 20]) -> &str {
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = b"0123456789"[count % 10];
        count /= 10;
        if count == 0 {
            break;
        }
    }
    core::str::from_utf8(&digits[start..]).expect("decimal digits are ASCII")
}

impl Group {
    /// Ends the subtree at `y` (its natural bottom); returns where the rows after it start.
    fn close(&mut self, y: f32) -> f32 {
        self.bottom = self.start + (y - self.start) * self.openness;
        self.bottom
    }
}

impl ListState {
    /// Advances the folds animating to `now`. Returns whether some still animate.
    fn advance_folds(&mut self, now: Duration, style: &ListStyle) -> bool {
        let folded = &self.folded;
        self.folds.retain_mut(|fold| {
            let start = *fold.start.get_or_insert(now);
            let closing = folded.contains(&fold.id);
            let (to, duration, curve) = if closing {
                (0.0, style.collapse, &style.collapse_curve)
            } else {
                (1.0, style.expand, &style.expand_curve)
            };
            let length = duration.as_secs_f32() * (to - fold.from).abs();
            let t = if length > 0.0 {
                now.saturating_sub(start).as_secs_f32() / length
            } else {
                1.0
            };
            if t >= 1.0 {
                fold.current = to;
                return false;
            }
            // Overshooting curves hold at the ends: openness stays within 0 and 1.
            fold.current = curve
                .ease(t)
                .mul_add(to - fold.from, fold.from)
                .clamp(0.0, 1.0);
            true
        });
        !self.folds.is_empty()
    }

    /// The openness of `id`'s fold: animating, or 0 folded and 1 unfolded.
    fn openness(&self, id: u64) -> f32 {
        match self.folds.iter().find(|fold| fold.id == id) {
            Some(fold) => fold.current,
            None if self.folded.contains(&id) => 0.0,
            None => 1.0,
        }
    }

    /// Lays the rows of `items` out, `row_height` apart, the folds animated to `now`; without
    /// `tree`, every row is a top-level row. Returns whether a fold is animating.
    pub(super) fn lay_out(
        &mut self,
        items: &[ListItem<'_>],
        tree: bool,
        row_height: f32,
        now: Duration,
        style: &ListStyle,
    ) -> bool {
        let animating = self.advance_folds(now, style);
        self.rows.clear();
        self.groups.clear();
        self.stack.clear();
        let mut y = 0.0;
        let mut index = 0;
        while index < items.len() {
            let item = &items[index];
            let depth = if tree { item.depth } else { 0 };
            // Animating folds this row is not part of end above it.
            while let Some(&group) = self.stack.last()
                && self.groups[group].depth >= depth
            {
                y = self.groups[group].close(y);
                self.stack.pop();
            }
            let group = self.stack.last().copied();
            let leaving = group.is_some_and(|group| self.groups[group].closing);
            let parent = tree
                && items
                    .get(index + 1)
                    .is_some_and(|next| next.depth > item.depth);
            let folded = parent && self.folded.contains(&item.id);
            let openness = if parent { self.openness(item.id) } else { 1.0 };
            let end = if folded || openness <= 0.0 {
                subtree_end(items, index)
            } else {
                index + 1
            };
            self.rows.push(Row {
                id: item.id,
                item: index,
                top: y,
                clip: f32::INFINITY,
                group,
                hidden: if folded { end - index - 1 } else { 0 },
                parent,
                leaving,
                openness,
            });
            y += row_height;
            if parent && openness <= 0.0 {
                index = end;
                continue;
            }
            // An animating fold; a closing one takes its rows out of the visible ones at once.
            if parent && (openness < 1.0 || folded) {
                self.groups.push(Group {
                    depth,
                    start: y,
                    openness,
                    closing: folded || leaving,
                    bottom: f32::INFINITY,
                    parent: group,
                });
                self.stack.push(self.groups.len() - 1);
            }
            index += 1;
        }
        while let Some(group) = self.stack.pop() {
            y = self.groups[group].close(y);
        }
        // A fold inside another is cut by both; parents come first.
        for group in 0..self.groups.len() {
            if let Some(parent) = self.groups[group].parent {
                self.groups[group].bottom =
                    self.groups[group].bottom.min(self.groups[parent].bottom);
            }
        }
        for row in &mut self.rows {
            if let Some(group) = row.group {
                row.clip = self.groups[group].bottom;
            }
        }
        self.height = y;
        animating
    }

    /// Where the dragged rows would go for the pointer at `at`: `x` from the start of the
    /// rows' content, `y` from the top of the content. The preferred place comes from where
    /// the pointer is in its row; a place the list or the delegate refuses gives way to the
    /// next one. `None` when no place accepts the rows.
    pub(super) fn drop_target(
        &self,
        items: &[ListItem<'_>],
        options: &ListOptions,
        delegate: &impl ListDelegate,
        at: Vec2,
        metrics: Metrics,
    ) -> Option<(DropPlace, Preview)> {
        let height = metrics.row_height;
        let mut under = None;
        let (mut first, mut last) = (None, None);
        for (position, row) in self.rows.iter().enumerate() {
            if row.leaving {
                continue;
            }
            first = first.or(Some(position));
            last = Some(position);
            if at.y >= row.top && at.y < (row.top + height).min(row.clip) {
                under = Some(position);
            }
        }
        let (position, fraction) = match (under, first, last) {
            (Some(position), _, _) => (position, (at.y - self.rows[position].top) / height),
            (None, Some(first), _) if at.y < self.rows[first].top => (first, 0.0),
            (None, _, Some(last)) => (last, 1.0),
            _ => return None,
        };
        let row = self.rows[position];
        let item = &items[row.item];
        let before = (
            DropPlace::Before(item.id),
            Preview::Line {
                y: row.top,
                depth: item.depth,
            },
        );
        let into = options
            .tree
            .then_some((DropPlace::Into(item.id), Preview::Frame { top: row.top }));
        let after = self.after(items, position, at.x, metrics);
        // Into the row in its middle half when it accepts the rows, else before or after.
        let accepts_into = into.is_some_and(|candidate| self.accepts(items, delegate, candidate.0));
        let order = match (accepts_into, fraction) {
            (true, fraction) if fraction < 0.25 => [Some(before), into, Some(after)],
            (true, fraction) if fraction < 0.5 => [into, Some(before), Some(after)],
            (true, fraction) if fraction < 0.75 => [into, Some(after), Some(before)],
            (true, _) => [Some(after), into, Some(before)],
            (false, fraction) if fraction < 0.5 => [Some(before), Some(after), None],
            (false, _) => [Some(after), Some(before), None],
        };
        order
            .into_iter()
            .flatten()
            .find(|candidate| self.accepts(items, delegate, candidate.0))
    }

    /// The place after the row at `position`: the first child's slot below an unfolded
    /// parent; below the end of subtrees, the level the pointer's `x` points to, between the
    /// row's own and the next row's.
    fn after(
        &self,
        items: &[ListItem<'_>],
        position: usize,
        x: f32,
        metrics: Metrics,
    ) -> (DropPlace, Preview) {
        let row = self.rows[position];
        let item = &items[row.item];
        let y = row.top + metrics.row_height;
        let next = self.rows[position + 1..].iter().find(|next| !next.leaving);
        if row.parent
            && row.openness >= 1.0
            && let Some(child) = next.filter(|next| next.item == row.item + 1)
        {
            return (
                DropPlace::Before(child.id),
                Preview::Line {
                    y,
                    depth: items[child.item].depth,
                },
            );
        }
        let shallowest = next
            .map_or(0, |next| items[next.item].depth)
            .min(item.depth);
        let mut depth = item.depth;
        while depth > shallowest && x < f32::from(depth) * metrics.indent {
            depth -= 1;
        }
        let target = &items[ancestor(items, row.item, depth)];
        (DropPlace::After(target.id), Preview::Line { y, depth })
    }

    /// Whether the dragged rows may go at `place`: never inside themselves, then as the
    /// delegate decides.
    fn accepts(
        &self,
        items: &[ListItem<'_>],
        delegate: &impl ListDelegate,
        place: DropPlace,
    ) -> bool {
        let (DropPlace::Before(id) | DropPlace::After(id) | DropPlace::Into(id)) = place;
        let Some(index) = items.iter().position(|item| item.id == id) else {
            return false;
        };
        let inside = self
            .dragged_ranges
            .iter()
            .any(|&(start, end)| (start..end).contains(&index));
        !inside && delegate.can_drop(&self.dragged, place)
    }
}
