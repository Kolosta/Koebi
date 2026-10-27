//! How menus react: the keyboard, the pointer (highlights, the rest that opens submenus, the
//! aim towards an open submenu) and the distance that closes them.
//!
//! The pointer is resolved against the previous frame's layout before the menus are laid out
//! and drawn, so that a frame shows what it decided: the highlight of the item the pointer
//! entered, the submenu its rest opened.

use core::time::Duration;

use koebi_core::{Rect, Vec2};

use super::state::{Aim, Rest};
use super::{
    MENU_CHOOSE, MENU_CLOSE, MENU_CLOSE_SUBMENU, MENU_NEXT, MENU_OPEN_SUBMENU, MENU_PREVIOUS,
    MenuChoice, MenuItem, MenuState, item_key,
};
use crate::id::WidgetId;
use crate::ui::Ui;

/// What the keyboard did this frame.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Keys {
    /// The item it chose.
    pub(super) choice: Option<MenuChoice>,
    /// The level whose highlight it moved: that level scrolls to show it.
    pub(super) moved: Option<usize>,
}

/// The distance from `point` to `rect` (0 inside).
fn distance(rect: Rect, point: Vec2) -> f32 {
    let dx = (rect.min.x - point.x).max(point.x - rect.max.x).max(0.0);
    let dy = (rect.min.y - point.y).max(point.y - rect.max.y).max(0.0);
    dx.hypot(dy)
}

/// `true` when `point` lies inside the triangle `a`, `b`, `c` (or on its edges).
fn in_triangle(point: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let side = |p: Vec2, q: Vec2, r: Vec2| (p.x - r.x) * (q.y - r.y) - (q.x - r.x) * (p.y - r.y);
    let (d1, d2, d3) = (side(point, a, b), side(point, b, c), side(point, c, a));
    let negative = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let positive = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(negative && positive)
}

/// `true` while the pointer, from where `aim` last saw it over a submenu's item, heads towards
/// the submenu at `target` within the grace time.
fn heading(
    aim: Option<Aim>,
    pointer: Option<Vec2>,
    target: Rect,
    now: Duration,
    grace: Duration,
) -> bool {
    let (Some(aim), Some(pointer)) = (aim, pointer) else {
        return false;
    };
    if now >= aim.since.saturating_add(grace) {
        return false;
    }
    // The submenu's edge facing the pointer: the near side of the triangle.
    let x = if target.min.x >= aim.origin.x {
        target.min.x
    } else {
        target.max.x
    };
    in_triangle(
        pointer,
        aim.origin,
        Vec2::new(x, target.min.y),
        Vec2::new(x, target.max.y),
    )
}

impl Ui<'_> {
    /// The keyboard: closing the innermost menu, moving the highlight, opening and closing
    /// submenus, choosing.
    pub(super) fn menu_keys(&mut self, state: &mut MenuState, items: &[MenuItem<'_>]) -> Keys {
        let mut keys = Keys::default();
        let deepest = state.depth - 1;
        let Some(level_items) = state.items_at(items, deepest) else {
            return keys;
        };
        if self.triggered(&MENU_CLOSE) {
            // Used here: a popup holding this menu stays open.
            self.consume(&MENU_CLOSE);
            if deepest > 0 {
                state.close_below(deepest - 1);
            } else {
                state.close();
            }
            return keys;
        }
        if self.triggered(&MENU_CLOSE_SUBMENU) && deepest > 0 {
            state.close_below(deepest - 1);
            return keys;
        }
        if self.triggered(&MENU_NEXT) {
            state.step(level_items, deepest, true);
            keys.moved = Some(deepest);
        }
        if self.triggered(&MENU_PREVIOUS) {
            state.step(level_items, deepest, false);
            keys.moved = Some(deepest);
        }
        let highlighted = state.levels[deepest]
            .highlighted
            .and_then(|index| Some((index, level_items.get(index)?)));
        let choose = self.triggered(&MENU_CHOOSE);
        let open = choose || self.triggered(&MENU_OPEN_SUBMENU);
        match highlighted {
            Some((index, item)) if open && item.opens(deepest).is_some() => {
                let submenu = item.opens(deepest).unwrap_or_default();
                if state.open_submenu(deepest, index) {
                    state.step(submenu, deepest + 1, true);
                    keys.moved = Some(deepest + 1);
                }
            }
            Some((index, MenuItem::Action(action))) if action.enabled && choose => {
                keys.choice = Some(state.choice(deepest, index));
            }
            _ => {}
        }
        keys
    }

    /// The item of an open level under the pointer, from the previous frame's layout, unless
    /// another widget holds the pointer.
    pub(super) fn menu_hovered(
        &self,
        id: WidgetId,
        state: &MenuState,
        items: &[MenuItem<'_>],
    ) -> Option<(usize, usize)> {
        let hovered = self.state.hovered()?;
        if self.state.active().is_some_and(|active| active != hovered) {
            return None;
        }
        (0..state.depth).find_map(|level| {
            let level_items = state.items_at(items, level)?;
            let index = (0..level_items.len()).find(|index| {
                level_items[*index].is_active_at(level)
                    && hovered == id.with(item_key(level, *index))
            })?;
            Some((level, index))
        })
    }

    /// The pointer: the highlight follows it when it moves (never under a still pointer, so
    /// that the keyboard keeps the highlight while a menu scrolls under it), a rest opens the
    /// submenu of its item or closes a deeper one, and the aim towards an open submenu keeps
    /// it while the pointer crosses other items.
    pub(super) fn menu_pointer(
        &mut self,
        state: &mut MenuState,
        items: &[MenuItem<'_>],
        hovered: Option<(usize, usize)>,
    ) {
        let input = self.styles().input;
        let now = self.input().time();
        let pointer = self.input().pointer();
        let moved = pointer != state.pointer || self.input().wheel() != Vec2::ZERO;
        state.pointer = pointer;
        if moved || state.waiting {
            if let Some((level, item)) = hovered {
                self.menu_hover(state, items, level, item);
            } else {
                state.rest = None;
                state.pointed = None;
                state.waiting = false;
            }
        }
        // Resting long enough on an item closes the deeper submenus, and opens its own.
        if let Some(rest) = state.rest {
            let due = rest.since.saturating_add(input.submenu_delay);
            if now >= due {
                state.rest = None;
                state.close_below(rest.level);
                let opens = state
                    .items_at(items, rest.level)
                    .and_then(|items| items.get(rest.item))
                    .is_some_and(|item| item.opens(rest.level).is_some());
                if opens {
                    state.open_submenu(rest.level, rest.item);
                }
            } else {
                self.request_redraw_at(due);
            }
        }
    }

    /// The pointer moved over `item` of `level`: it highlights the item and starts a rest on
    /// it, unless it heads for the open submenu of another item of the level.
    fn menu_hover(
        &mut self,
        state: &mut MenuState,
        items: &[MenuItem<'_>],
        level: usize,
        item: usize,
    ) {
        let input = self.styles().input;
        let now = self.input().time();
        let pointer = self.input().pointer();
        let hovered = Some((level, item));
        let child = (level + 1 < state.depth).then(|| state.levels[level + 1].parent);
        let target = state.levels.get(level + 1).map(|next| next.rect);
        if child == Some(item) {
            // Over the item of the open submenu: the aim starts from here.
            if let Some(pointer) = pointer {
                state.aim = Some(Aim {
                    origin: pointer,
                    since: now,
                });
            }
            state.levels[level].highlighted = Some(item);
            state.rest = None;
            state.pointed = hovered;
            state.waiting = false;
        } else if child.is_some()
            && let Some(target) = target
            && heading(state.aim, pointer, target, now, input.submenu_grace)
        {
            // Heading for the open submenu: the items crossed wait for the end of the grace.
            state.waiting = true;
            if let Some(aim) = state.aim {
                self.request_redraw_at(aim.since.saturating_add(input.submenu_grace));
            }
        } else if state.pointed != hovered || state.waiting {
            state.waiting = false;
            state.levels[level].highlighted = Some(item);
            state.pointed = hovered;
            // A rest matters only when it opens a submenu or closes a deeper one.
            let opens = state
                .items_at(items, level)
                .and_then(|items| items.get(item))
                .is_some_and(|item| item.opens(level).is_some());
            state.rest = (opens || child.is_some()).then_some(Rest {
                level,
                item,
                since: now,
            });
        }
    }

    /// `true` when the pointer went farther than the close distance from every open panel
    /// and from the menus' control, after it came near them once.
    pub(super) fn menu_too_far(&self, state: &mut MenuState, panels: &[Rect]) -> bool {
        let limit = self.styles().input.menu_close_distance;
        let Some(pointer) = self.input().pointer() else {
            return false;
        };
        if limit <= 0.0 {
            return false;
        }
        let near = |rect: Rect| distance(rect, pointer) <= limit;
        let close = panels.iter().any(|panel| near(*panel)) || state.control().is_some_and(near);
        if close {
            state.armed = true;
        }
        state.armed && !close
    }
}
