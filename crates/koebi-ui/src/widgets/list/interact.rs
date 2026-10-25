//! A list's interaction outside drawing: the keyboard, the drag in progress, and what a
//! press or a click on a row does to the selection.

use core::time::Duration;

use koebi_core::{Rect, Vec2};

use super::state::{Drag, ListState};
use super::{
    AUTO_SCROLL_ROWS_PER_SECOND, LIST_DRAG, LIST_DRAG_CANCEL, LIST_EXTEND_NEXT,
    LIST_EXTEND_PREVIOUS, LIST_FOLD, LIST_NEXT, LIST_PREVIOUS, LIST_SELECT_ALL, LIST_UNFOLD,
    ListContextMenu, ListDelegate, ListItem, ListOptions, ListResponse, ListSelection, Metrics,
    primary_held,
};
use crate::input::PointerButton;
use crate::ui::{Response, Ui};

impl Ui<'_> {
    /// The keyboard: moves the active row, folds, selects all. Returns whether the selection
    /// changed.
    pub(super) fn list_keys(
        &mut self,
        state: &mut ListState,
        items: &[ListItem<'_>],
        options: &ListOptions,
        view_height: f32,
        row_height: f32,
    ) -> bool {
        let mut changed = false;
        let moves = [
            (&LIST_NEXT, true, false),
            (&LIST_PREVIOUS, false, false),
            (&LIST_EXTEND_NEXT, true, true),
            (&LIST_EXTEND_PREVIOUS, false, true),
        ];
        for (action, forward, extend) in moves {
            if self.triggered(action) {
                changed |= state.step(forward, extend, options.selection);
            }
        }
        if options.tree && self.triggered(&LIST_UNFOLD) {
            changed |= state.unfold_or_enter(items, options.selection);
        }
        if options.tree && self.triggered(&LIST_FOLD) {
            changed |= state.fold_or_leave(items, options.selection);
        }
        if options.selection == ListSelection::Multiple && self.triggered(&LIST_SELECT_ALL) {
            changed |= state.select_all(items);
        }
        if changed {
            state.reveal_active(row_height, view_height);
        }
        changed
    }

    /// The drag in progress, before the rows draw: its start past the threshold, its
    /// cancellation, scrolling at the edges, where the rows would go, and the drop.
    #[allow(clippy::too_many_arguments, reason = "the parts of one list")]
    pub(super) fn list_drag(
        &mut self,
        state: &mut ListState,
        items: &[ListItem<'_>],
        options: &ListOptions,
        delegate: &impl ListDelegate,
        viewport: Rect,
        metrics: Metrics,
        response: &mut ListResponse,
    ) {
        let Some(mut drag) = state.drag else {
            return;
        };
        let style = self.styles().list;
        let now = self.input().time();
        let pointer = self.input().pointer();
        if !drag.started
            && pointer.is_some_and(|point| (point - drag.origin).length() >= style.drag_threshold)
        {
            drag.started = true;
            drag.scrolled_at = now;
        }
        if drag.started && !drag.cancelled {
            self.grab(LIST_DRAG);
            if self.triggered(&LIST_DRAG_CANCEL) {
                drag.cancelled = true;
            }
        }
        drag.target = None;
        if drag.started
            && !drag.cancelled
            && let Some(point) = pointer
        {
            // Pressing against the top or bottom edge scrolls, faster past it.
            let edge = metrics.row_height;
            let push = if point.y < viewport.min.y + edge {
                -((viewport.min.y + edge - point.y) / edge).min(2.0)
            } else if point.y > viewport.max.y - edge {
                ((point.y - viewport.max.y + edge) / edge).min(2.0)
            } else {
                0.0
            };
            let elapsed = now.saturating_sub(drag.scrolled_at).as_secs_f32();
            drag.scrolled_at = now;
            let offset = state.scroll.offset();
            let overflow = (state.height - viewport.height()).max(0.0);
            let scrolled = (offset.y
                + push * AUTO_SCROLL_ROWS_PER_SECOND * metrics.row_height * elapsed)
                .clamp(0.0, overflow);
            state.scroll.set_offset(Vec2::new(offset.x, scrolled));
            if push < 0.0 && scrolled > 0.0 || push > 0.0 && scrolled < overflow {
                self.request_redraw_after(Duration::from_millis(16));
            }
            state.collect_dragged(items, options.tree, drag.row, drag.selection);
            if point.x >= viewport.min.x && point.x < viewport.max.x {
                let y = point.y.clamp(viewport.min.y, viewport.max.y - 1.0);
                let at = Vec2::new(point.x - metrics.start, y - viewport.min.y + scrolled);
                drag.target = state.drop_target(items, options, delegate, at, metrics);
            }
        }
        if self.input().released() && drag.started && !drag.cancelled {
            response.dropped = drag.target.map(|(place, _)| place);
        }
        state.drag = Some(drag);
    }

    /// What a press, a click or a secondary press on `item`'s row does: prepare a drag,
    /// select, report double clicks and context menus.
    pub(super) fn list_row_input(
        &mut self,
        state: &mut ListState,
        options: &ListOptions,
        item: &ListItem<'_>,
        row: Response,
        response: &mut ListResponse,
    ) {
        let modifiers = self.input().modifiers();
        let selection = options.selection;
        // The same button drags rows: a press only prepares a drag, the click selects (and a
        // drag leaves the selection alone).
        if row.pressed
            && options.drag
            && let Some(origin) = self.input().pressed_at()
        {
            state.drag = Some(Drag::new(
                item.id,
                origin,
                state.is_selected(item.id),
                self.input().time(),
            ));
        }
        let dragged = state.drag.is_some_and(|drag| drag.started);
        if row.clicked && !dragged {
            response.selection_changed |= match selection {
                ListSelection::None => false,
                ListSelection::Multiple if modifiers.shift() => {
                    state.select_range(item.id, primary_held(modifiers))
                }
                ListSelection::Multiple if primary_held(modifiers) => state.toggle(item.id),
                ListSelection::Single | ListSelection::Multiple => state.select_only(item.id),
            };
            let style = self.styles().list;
            let now = self.input().time();
            if state.last_click.is_some_and(|(id, at)| {
                id == item.id && now.saturating_sub(at) <= style.double_click
            }) {
                response.double_clicked = Some(item.id);
                state.last_click = None;
            } else {
                state.last_click = Some((item.id, now));
            }
        }
        if row.secondary_pressed && state.drag.is_none() {
            if selection != ListSelection::None && !state.is_selected(item.id) {
                response.selection_changed |= state.select_only(item.id);
            }
            response.context_menu = Some(ListContextMenu {
                at: self
                    .input()
                    .button_pressed_at(PointerButton::Secondary)
                    .unwrap_or_default(),
                row: Some(item.id),
            });
        }
    }
}
