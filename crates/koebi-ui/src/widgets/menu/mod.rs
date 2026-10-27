//! Menus: floating panels of items, opened at the pointer (context menus) or beside a control
//! (dropdowns, menu buttons), drawn above everything, with submenus.
//!
//! - **Items** ([`MenuItem`], [`item`]): actions with an icon, a label, a shortcut hint, a
//!   check or radio mark, disabled or marked as the current choice; submenus, eight levels deep
//!   at most; separators and titles.
//! - **Opening** ([`MenuState`]): at a point, or beside a control with a
//!   [`Placement`](crate::Placement) that
//!   flips when the window has no room; a menu opening below or above its control is at least
//!   as wide, and where the two meet, the menu squares its corners ([`layout`]).
//! - **Submenus** open beside their item when the pointer rests on it
//!   (`semantic.input.submenu.delay`) or at a click, cascading the way the previous level
//!   went. While the pointer heads from the item towards the open submenu, the items it crosses
//!   wait (`semantic.input.submenu.grace`) ([`interact`]).
//! - **Closing**: choosing an item closes every menu, and so do a press outside them (which
//!   nothing else receives) and the pointer moving farther than
//!   `semantic.input.menu.close-distance` from the menus and their control. Leaving a menu
//!   closes nothing by itself.
//! - **Keyboard**, wherever the pointer is: [`MENU_NEXT`], [`MENU_PREVIOUS`],
//!   [`MENU_OPEN_SUBMENU`], [`MENU_CLOSE_SUBMENU`], [`MENU_CHOOSE`] and [`MENU_CLOSE`] (which
//!   closes the innermost menu). The pointer takes the highlight back only when it moves.
//! - **Long menus** scroll with the wheel and follow the keyboard's highlight.
//! - **Custom content**: [`Ui::popup`] opens and closes like a menu and draws what the caller
//!   gives it ([`popup`]).

mod interact;
mod item;
mod layout;
mod popup;
mod state;
mod style;

#[cfg(test)]
mod tests;

pub use item::{MenuAction, MenuItem, MenuMark, Submenu};
pub use state::{MenuChoice, MenuState};
pub use style::MenuStyle;

use koebi_core::input::Key;
use koebi_core::{Rect, Vec2};
use koebi_input::{ActionDef, ActionGroup, Context, Shortcut};
use koebi_render::{Corners, RectPrimitive};

use self::layout::RowLook;
use self::state::MAX_DEPTH;
use crate::id::WidgetId;
use crate::routing::Layer;
use crate::ui::Ui;
use crate::widgets::parts::Side;
use crate::widgets::{repeating, snap};

/// An open menu: its keys apply wherever the pointer is.
pub const MENU: Context = Context::new("menu");

/// Closes the innermost open menu without choosing.
pub static MENU_CLOSE: ActionDef = ActionDef {
    id: "menu.close",
    label: "Close menu",
    description: "Closes the innermost open menu without choosing anything.",
    context: MENU,
    defaults: &[Shortcut::key(Key::Escape)],
};

/// Highlights the next item.
pub static MENU_NEXT: ActionDef = ActionDef {
    id: "menu.next",
    label: "Next item",
    description: "Highlights the next item of the innermost open menu.",
    context: MENU,
    defaults: &[repeating(Key::Down)],
};

/// Highlights the previous item.
pub static MENU_PREVIOUS: ActionDef = ActionDef {
    id: "menu.previous",
    label: "Previous item",
    description: "Highlights the previous item of the innermost open menu.",
    context: MENU,
    defaults: &[repeating(Key::Up)],
};

/// Opens the submenu of the highlighted item.
pub static MENU_OPEN_SUBMENU: ActionDef = ActionDef {
    id: "menu.open-submenu",
    label: "Open submenu",
    description: "Opens the submenu of the highlighted item and highlights its first item.",
    context: MENU,
    defaults: &[Shortcut::key(Key::Right)],
};

/// Closes the innermost submenu.
pub static MENU_CLOSE_SUBMENU: ActionDef = ActionDef {
    id: "menu.close-submenu",
    label: "Close submenu",
    description: "Closes the innermost submenu and goes back to its item.",
    context: MENU,
    defaults: &[Shortcut::key(Key::Left)],
};

/// Chooses the highlighted item, or opens its submenu.
pub static MENU_CHOOSE: ActionDef = ActionDef {
    id: "menu.choose",
    label: "Choose item",
    description: "Chooses the highlighted item of the open menu, or opens its submenu.",
    context: MENU,
    defaults: &[
        Shortcut::key(Key::Enter),
        Shortcut::key(Key::KeypadEnter),
        Shortcut::key(Key::Space),
    ],
};

/// The actions of menus, for the keymap.
pub static MENU_ACTIONS: ActionGroup = ActionGroup {
    name: "Menus",
    actions: &[
        &MENU_CLOSE,
        &MENU_NEXT,
        &MENU_PREVIOUS,
        &MENU_OPEN_SUBMENU,
        &MENU_CLOSE_SUBMENU,
        &MENU_CHOOSE,
    ],
};

/// The widget key of the region behind the menus that catches presses outside them.
const OUTSIDE: u64 = u64::MAX;

/// The widget key of the panel of `level`.
fn panel_key(level: usize) -> u64 {
    (1 << 40) + level as u64
}

/// The widget key of `item` of `level`.
fn item_key(level: usize, item: usize) -> u64 {
    ((level as u64 + 1) << 32) | item as u64
}

/// The rectangle of a context menu of `size` opened at `point`: right and down from it when
/// there is room, else left or up, kept inside `bounds`.
fn at_point(point: Vec2, size: Vec2, bounds: Rect) -> Rect {
    let x = if point.x + size.x <= bounds.max.x {
        point.x
    } else {
        point.x - size.x
    };
    let y = if point.y + size.y <= bounds.max.y {
        point.y
    } else {
        point.y - size.y
    };
    let keep = |start: f32, length: f32, low: f32, high: f32| start.min(high - length).max(low);
    snap(Rect::from_origin_size(
        Vec2::new(
            keep(x, size.x, bounds.min.x, bounds.max.x),
            keep(y, size.y, bounds.min.y, bounds.max.y),
        ),
        size,
    ))
}

impl Ui<'_> {
    /// Shows the menu of `state` and its open submenus when it is open, above everything,
    /// kept inside `bounds` (the window), and returns the item chosen this frame.
    ///
    /// Submenus open beside their item when the pointer rests on it
    /// (`semantic.input.submenu.delay`) or at a click; while the pointer heads towards an open
    /// submenu, the items it crosses wait (`semantic.input.submenu.grace`). Choosing closes
    /// every menu, and so do a press outside them (which nothing else receives) and the
    /// pointer moving farther than `semantic.input.menu.close-distance` from them; leaving a
    /// menu closes nothing by itself. The keyboard works wherever the pointer is
    /// ([`MENU_NEXT`], [`MENU_PREVIOUS`], [`MENU_OPEN_SUBMENU`], [`MENU_CLOSE_SUBMENU`],
    /// [`MENU_CHOOSE`], and [`MENU_CLOSE`] for the innermost menu); long menus scroll. Menus
    /// nest eight levels deep at most.
    pub fn menu(
        &mut self,
        id: WidgetId,
        state: &mut MenuState,
        items: &[MenuItem<'_>],
        bounds: Rect,
    ) -> Option<MenuChoice> {
        if !state.is_open() {
            return None;
        }
        state.prune(items);
        self.grab(MENU);
        let keys = self.menu_keys(state, items);
        if let Some(choice) = keys.choice {
            state.close();
            return Some(choice);
        }
        if !state.is_open() {
            return None;
        }
        // The pointer first, against the last frame: this frame shows what it decides.
        let hovered = self.menu_hovered(id, state, items);
        self.menu_pointer(state, items, hovered);
        let frames = self.menu_layout(state, items, bounds, keys.moved);
        let depth = state.depth;
        let mut panels = [Rect::ZERO; MAX_DEPTH];
        for (panel, frame) in panels.iter_mut().zip(&frames).take(depth) {
            *panel = frame.rect();
        }
        if self.menu_too_far(state, &panels[..depth]) {
            state.close();
            // Its control, drawn earlier in this frame, shows open until the next one.
            self.request_redraw_at(self.input().time());
            return None;
        }

        let style = &self.styles().menu;
        let (mut chosen, mut opened) = (None, None);
        self.on_layer(Layer::FLOATING, |ui| {
            // Presses outside the menus close them, and reach nothing else.
            let outside = ui.interact(id.with(OUTSIDE), bounds);
            if outside.pressed || outside.secondary_pressed {
                state.close();
                return;
            }
            for (level, frame) in frames.iter().enumerate().take(depth) {
                let Some(level_items) = state.items_at(items, level) else {
                    break;
                };
                let panel = frame.rect();
                let panel_id = id.with(panel_key(level));
                ui.identified_zone(panel, MENU, panel_id, |ui| {
                    if frame.overflow > 0.0 && ui.pointer_within(panel_id) {
                        let scroll = &mut state.levels[level].scroll;
                        *scroll = (*scroll - ui.input().wheel().y * style.wheel_step)
                            .clamp(0.0, frame.overflow);
                    }
                    let radii = match (level, frame.placed, state.control()) {
                        (0, Some(placed), Some(control)) => {
                            placed.panel_corners(control, Corners::all(style.corner_radius))
                        }
                        _ => Corners::all(style.corner_radius),
                    };
                    ui.scene().push_rect(
                        &RectPrimitive::new(panel, style.background)
                            .with_corner_radii(radii)
                            .with_border(style.border_width, style.border_color),
                    );
                    let content = panel.inset(style.border_width);
                    let child = (level + 1 < depth).then(|| state.levels[level + 1].parent);
                    let child_left = frames
                        .get(level + 1)
                        .and_then(|frame| frame.placed)
                        .is_some_and(|placed| placed.side == Side::Left);
                    let scroll = state.levels[level].scroll;
                    let highlighted = state.levels[level].highlighted;
                    ui.clipped(content, |ui| {
                        let mut y = content.min.y + style.padding_vertical - scroll;
                        for (index, item) in level_items.iter().enumerate() {
                            let height = layout::row_height(style, item);
                            let row = Rect::from_min_max(
                                Vec2::new(content.min.x + style.item_gap, y),
                                Vec2::new(content.max.x - style.item_gap, y + height),
                            );
                            y += height;
                            if row.max.y <= content.min.y || row.min.y >= content.max.y {
                                continue;
                            }
                            let active = item.is_active_at(level);
                            if matches!(item, MenuItem::Action(_) | MenuItem::Submenu(_)) {
                                let response = ui.interact(id.with(item_key(level, index)), row);
                                if response.clicked && active {
                                    match item {
                                        MenuItem::Submenu(_) => opened = Some((level, index)),
                                        _ => chosen = Some(state.choice(level, index)),
                                    }
                                }
                            }
                            let look = if active
                                && (highlighted == Some(index) || child == Some(index))
                            {
                                RowLook::Highlighted
                            } else if matches!(item, MenuItem::Action(action) if action.selected) {
                                RowLook::Selected
                            } else {
                                RowLook::Normal
                            };
                            let toward_left = child == Some(index) && child_left;
                            layout::draw_row(
                                ui,
                                item,
                                row,
                                &frame.columns,
                                look,
                                active,
                                toward_left,
                            );
                        }
                    });
                });
                state.levels[level].rect = panel;
            }
        });
        if !state.is_open() {
            return None;
        }
        if let Some(choice) = chosen {
            state.close();
            return Some(choice);
        }
        if let Some((level, item)) = opened {
            // A click is an event: the frame after it, asked for, shows the submenu.
            state.open_submenu(level, item);
            state.rest = None;
            state.pointed = Some((level, item));
        }
        None
    }
}
