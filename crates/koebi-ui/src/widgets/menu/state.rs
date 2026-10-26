//! [`MenuState`]: whether a menu is open, where it opens, its open submenus, their highlighted
//! items and the timers of the pointer; [`MenuChoice`]: the item chosen.

use core::time::Duration;

use koebi_core::{Rect, Vec2};

use super::MenuItem;
use crate::widgets::parts::{Placed, Placement};

/// The deepest a menu nests: the root and seven levels of submenus.
pub(super) const MAX_DEPTH: usize = 8;

/// What a menu opens from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Anchor {
    /// A point: its top-left corner there, or wherever it fits around it.
    Point(Vec2),
    /// A control, beside which it opens as the placement says.
    Control(Rect, Placement),
}

/// One open level: the root menu or a submenu.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Level {
    /// The item of the parent level this submenu opened from (unused for the root).
    pub(super) parent: usize,
    /// The highlighted item.
    pub(super) highlighted: Option<usize>,
    /// How far its items are scrolled.
    pub(super) scroll: f32,
    /// Where it was drawn on the last frame.
    pub(super) rect: Rect,
}

/// The item the pointer rests on, and since when.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Rest {
    pub(super) level: usize,
    pub(super) item: usize,
    pub(super) since: Duration,
}

/// Where the pointer was last seen over the item of the open submenu, and when: while it
/// heads from there towards the submenu, the items it crosses do not take over.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Aim {
    pub(super) origin: Vec2,
    pub(super) since: Duration,
}

/// Whether a menu is open, where, its open submenus and their highlighted items. Keep one per
/// menu, between frames.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MenuState {
    pub(super) anchor: Option<Anchor>,
    /// Open levels: the root, then its open submenus.
    pub(super) depth: usize,
    pub(super) levels: [Level; MAX_DEPTH],
    pub(super) rest: Option<Rest>,
    pub(super) aim: Option<Aim>,
    /// The item under the pointer on the last frame: the pointer takes the highlight over
    /// from the keyboard only when it moves to another item.
    pub(super) pointed: Option<(usize, usize)>,
    /// Where the root opened on the last frame.
    pub(super) placed: Option<Placed>,
    /// Whether the pointer came near the menus since they opened: only then does moving away
    /// close them (a menu opened far from the pointer waits for it).
    pub(super) armed: bool,
}

impl MenuState {
    /// A closed menu.
    pub const fn new() -> Self {
        Self {
            anchor: None,
            depth: 0,
            levels: [Level {
                parent: 0,
                highlighted: None,
                scroll: 0.0,
                rect: Rect::ZERO,
            }; MAX_DEPTH],
            rest: None,
            aim: None,
            pointed: None,
            placed: None,
            armed: false,
        }
    }

    /// Opens the menu at `point`: a context menu.
    pub fn open_at(&mut self, point: Vec2) {
        self.open(Anchor::Point(point));
    }

    /// Opens the menu beside `control`, as `placement` says.
    pub fn open_for(&mut self, control: Rect, placement: Placement) {
        self.open(Anchor::Control(control, placement));
    }

    /// Opens the menu below `control`, at least as wide: a dropdown.
    pub fn open_below(&mut self, control: Rect) {
        self.open_for(control, Placement::BELOW);
    }

    fn open(&mut self, anchor: Anchor) {
        *self = Self::new();
        self.anchor = Some(anchor);
        self.depth = 1;
    }

    /// Moves the control an open menu belongs to: the menu follows a layout that changes.
    pub fn follow(&mut self, control: Rect) {
        if let Some(Anchor::Control(rect, _)) = &mut self.anchor {
            *rect = control;
        }
    }

    /// Closes the menu and its submenus.
    pub fn close(&mut self) {
        *self = Self::new();
    }

    /// `true` while the menu is open.
    pub fn is_open(&self) -> bool {
        self.anchor.is_some() && self.depth > 0
    }

    /// How many menus are open: the root and its open submenus.
    pub fn depth(&self) -> usize {
        if self.is_open() { self.depth } else { 0 }
    }

    /// The highlighted item of the innermost open menu, by index.
    pub fn highlighted(&self) -> Option<usize> {
        self.depth
            .checked_sub(1)
            .and_then(|level| self.levels[level].highlighted)
    }

    /// Where the root menu opened on the last frame, beside its control: what the control
    /// reads to square the corners where the menu meets it.
    pub fn placed(&self) -> Option<Placed> {
        self.placed.filter(|_| self.is_open())
    }

    /// The control the menu belongs to, if it opened beside one.
    pub(super) fn control(&self) -> Option<Rect> {
        match self.anchor {
            Some(Anchor::Control(rect, _)) => Some(rect),
            _ => None,
        }
    }

    /// Keeps the open levels `0..=level` and closes the deeper ones.
    pub(super) fn close_below(&mut self, level: usize) {
        self.depth = self.depth.min(level + 1);
        self.aim = None;
    }

    /// Opens the submenu of `item` of `level` (closing what was open deeper).
    pub(super) fn open_submenu(&mut self, level: usize, item: usize) {
        if level + 1 >= MAX_DEPTH {
            return;
        }
        self.depth = level + 2;
        self.levels[level].highlighted = Some(item);
        self.levels[level + 1] = Level {
            parent: item,
            ..Level::default()
        };
        self.aim = None;
    }

    /// The items of `level`, following the open submenus from `root`; `None` when the items
    /// changed under an open submenu.
    pub(super) fn items_at<'a>(
        &self,
        root: &'a [MenuItem<'a>],
        level: usize,
    ) -> Option<&'a [MenuItem<'a>]> {
        let mut items = root;
        for open in &self.levels[1..=level] {
            match items.get(open.parent) {
                Some(MenuItem::Submenu(submenu)) => items = submenu.items,
                _ => return None,
            }
        }
        Some(items)
    }

    /// Closes the levels whose items vanished, after the application changed them.
    pub(super) fn prune(&mut self, root: &[MenuItem<'_>]) {
        let mut open = 1;
        while open < self.depth && self.items_at(root, open).is_some() {
            open += 1;
        }
        self.depth = self.depth.min(open);
    }

    /// Moves the highlight of `level` to the next item that can be highlighted after the
    /// current one (or before it), wrapping around.
    pub(super) fn step(&mut self, items: &[MenuItem<'_>], level: usize, forward: bool) {
        let count = items.len();
        if count == 0 {
            return;
        }
        let current = self.levels[level].highlighted;
        let start = match (current, forward) {
            (Some(index), true) => index + 1,
            (Some(index), false) => index + count - 1,
            (None, true) => 0,
            (None, false) => count - 1,
        };
        let next = (0..count)
            .map(|offset| {
                if forward {
                    (start + offset) % count
                } else {
                    (start + count - offset) % count
                }
            })
            .find(|index| items[*index].is_active());
        if next.is_some() {
            self.levels[level].highlighted = next;
        }
    }

    /// The choice of `item` of `level`.
    pub(super) fn choice(&self, level: usize, item: usize) -> MenuChoice {
        let mut path = [0; MAX_DEPTH];
        for (step, open) in path.iter_mut().zip(&self.levels[1..=level]) {
            *step = u16::try_from(open.parent).unwrap_or(u16::MAX);
        }
        path[level] = u16::try_from(item).unwrap_or(u16::MAX);
        MenuChoice {
            path,
            len: level + 1,
        }
    }
}

/// The item chosen in a menu: its index in its own list, and the indices of the submenus it
/// is in, from the root.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MenuChoice {
    path: [u16; MAX_DEPTH],
    len: usize,
}

impl MenuChoice {
    /// The index of the item in its own list (a submenu's, for a nested item).
    pub fn index(&self) -> usize {
        usize::from(self.path[self.len - 1])
    }

    /// The indices from the root menu down to the item: `[2, 0]` is the first item of the
    /// submenu of the root's third item.
    pub fn path(&self) -> &[u16] {
        &self.path[..self.len]
    }
}
