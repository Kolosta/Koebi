//! Menus: floating panels of items, opened at the pointer (context menus) or beside a control
//! (dropdowns, menu buttons), drawn above everything, with submenus.
//!
//! - **Items** ([`MenuItem`]): actions with an icon, a label, a shortcut hint, a check or
//!   radio mark, disabled or marked as the current choice; submenus; separators and titles.
//! - **Opening** ([`MenuState`]): at a point, or beside a control with a [`Placement`] that
//!   flips when the window has no room; a menu opening below or above its control is at least
//!   as wide, and where the two meet, the menu squares its corners.
//! - **Submenus** open beside their item when the pointer rests on it
//!   (`semantic.input.submenu.delay`) or at a click. While the pointer heads from the item
//!   towards the open submenu, the items it crosses do not take over
//!   (`semantic.input.submenu.grace`).
//! - **Closing**: choosing an item closes every menu, and so do a press outside them (which
//!   nothing else receives) and the pointer moving farther than
//!   `semantic.input.menu.close-distance` from the menus and their control. Leaving a menu
//!   closes nothing by itself.
//! - **Keyboard**, wherever the pointer is: [`MENU_NEXT`], [`MENU_PREVIOUS`],
//!   [`MENU_OPEN_SUBMENU`], [`MENU_CLOSE_SUBMENU`], [`MENU_CHOOSE`] and [`MENU_CLOSE`] (which
//!   closes the innermost menu).
//! - **Long menus** scroll with the wheel and follow the keyboard's highlight.
//! - **Custom content**: [`Ui::popup`] opens and closes like a menu and draws what the caller
//!   gives it.

mod layout;
mod state;

#[cfg(test)]
mod tests;

pub use state::{MenuChoice, MenuState};

use core::time::Duration;

use koebi_core::input::Key;
use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_input::{ActionDef, ActionGroup, Context, Shortcut};
use koebi_render::{Corners, RectPrimitive};
use koebi_tokens::component;

use self::layout::{Columns, RowLook};
use self::state::{Aim, Anchor, MAX_DEPTH, Rest};
use crate::icon::Icon;
use crate::id::WidgetId;
use crate::routing::Layer;
use crate::ui::Ui;
use crate::widgets::parts::{Alignment, Placed, Placement, Side};
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

/// The mark shown before the label of an action.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Mark {
    /// No mark, and no room kept for one.
    #[default]
    None,
    /// A check mark while on: an option that is set or not.
    Check(bool),
    /// A dot while on: one choice among the items around it.
    Radio(bool),
}

/// A choice of a menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuAction<'a> {
    /// What it does.
    pub label: &'a str,
    /// An icon before the label.
    pub icon: Option<Icon>,
    /// Its shortcut, shown at the end.
    pub shortcut: Option<&'a str>,
    /// A check or radio mark.
    pub mark: Mark,
    /// Disabled items show but cannot be chosen.
    pub enabled: bool,
    /// The current choice of a dropdown.
    pub selected: bool,
}

/// An item opening a submenu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Submenu<'a> {
    /// What the submenu holds.
    pub label: &'a str,
    /// An icon before the label.
    pub icon: Option<Icon>,
    /// The items of the submenu.
    pub items: &'a [MenuItem<'a>],
    /// A disabled submenu shows but does not open.
    pub enabled: bool,
}

/// An entry of a menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuItem<'a> {
    /// A choice.
    Action(MenuAction<'a>),
    /// An item opening a submenu.
    Submenu(Submenu<'a>),
    /// A line between groups of items.
    Separator,
    /// A heading above a group of items.
    Title(&'a str),
}

impl<'a> MenuItem<'a> {
    /// An enabled choice.
    pub const fn action(label: &'a str) -> Self {
        Self::Action(MenuAction {
            label,
            icon: None,
            shortcut: None,
            mark: Mark::None,
            enabled: true,
            selected: false,
        })
    }

    /// An enabled item opening a submenu of `items`.
    pub const fn submenu(label: &'a str, items: &'a [MenuItem<'a>]) -> Self {
        Self::Submenu(Submenu {
            label,
            icon: None,
            items,
            enabled: true,
        })
    }

    /// The same choice or submenu, showing `icon` before its label.
    #[must_use]
    pub const fn icon(self, icon: Icon) -> Self {
        match self {
            Self::Action(mut action) => {
                action.icon = Some(icon);
                Self::Action(action)
            }
            Self::Submenu(mut submenu) => {
                submenu.icon = Some(icon);
                Self::Submenu(submenu)
            }
            other => other,
        }
    }

    /// The same choice, showing `shortcut`.
    #[must_use]
    pub const fn shortcut(self, shortcut: &'a str) -> Self {
        match self {
            Self::Action(mut action) => {
                action.shortcut = Some(shortcut);
                Self::Action(action)
            }
            other => other,
        }
    }

    /// The same choice or submenu, enabled or not.
    #[must_use]
    pub const fn enabled(self, enabled: bool) -> Self {
        match self {
            Self::Action(mut action) => {
                action.enabled = enabled;
                Self::Action(action)
            }
            Self::Submenu(mut submenu) => {
                submenu.enabled = enabled;
                Self::Submenu(submenu)
            }
            other => other,
        }
    }

    /// The same choice, marked as the current one or not.
    #[must_use]
    pub const fn selected(self, selected: bool) -> Self {
        match self {
            Self::Action(mut action) => {
                action.selected = selected;
                Self::Action(action)
            }
            other => other,
        }
    }

    /// The same choice with a check mark, shown while `checked`.
    #[must_use]
    pub const fn checked(self, checked: bool) -> Self {
        self.mark(Mark::Check(checked))
    }

    /// The same choice with a radio mark, shown while `on`.
    #[must_use]
    pub const fn radio(self, on: bool) -> Self {
        self.mark(Mark::Radio(on))
    }

    const fn mark(self, mark: Mark) -> Self {
        match self {
            Self::Action(mut action) => {
                action.mark = mark;
                Self::Action(action)
            }
            other => other,
        }
    }

    /// `true` for items that can be highlighted: enabled choices and submenus.
    pub(crate) fn is_active(&self) -> bool {
        match self {
            Self::Action(action) => action.enabled,
            Self::Submenu(submenu) => submenu.enabled,
            Self::Separator | Self::Title(_) => false,
        }
    }
}

/// Colors and metrics of menus, from the `component.menu.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuStyle {
    /// Background of the panel.
    pub background: Color,
    /// Border color.
    pub border_color: Color,
    /// Border width.
    pub border_width: f32,
    /// Corner radius of the panel and of highlighted items.
    pub corner_radius: f32,
    /// Space above the first item and below the last one.
    pub padding_vertical: f32,
    /// Space around the content of a popup.
    pub padding_horizontal: f32,
    /// Background of the highlighted item.
    pub item_hover: Color,
    /// Background of the current choice of a dropdown.
    pub item_selected: Color,
    /// Height of an item.
    pub item_height: f32,
    /// Space between an item and the panel's sides.
    pub item_gap: f32,
    /// Space between an item's edges and its content.
    pub item_padding: f32,
    /// Space between a label and its shortcut.
    pub column_gap: f32,
    /// Space between an item's mark, icon, label and arrow.
    pub icon_gap: f32,
    /// Tallest a menu grows before it scrolls.
    pub max_height: f32,
    /// Color of headings.
    pub title_color: Color,
    /// Label color at rest.
    pub label: Color,
    /// Label color of the highlighted item.
    pub label_hover: Color,
    /// Label color of the current choice.
    pub label_selected: Color,
    /// Label (and icon, mark, shortcut) color of disabled items.
    pub label_disabled: Color,
    /// Shortcut color.
    pub shortcut_color: Color,
    /// Icon and submenu arrow color.
    pub icon_color: Color,
    /// Check and radio mark color.
    pub mark_color: Color,
    /// Side of icons and marks.
    pub icon_size: f32,
    /// Stroke width of icons.
    pub stroke_width: f32,
    /// Side of the dot of a radio mark.
    pub radio_size: f32,
    /// Separator lines.
    pub separator_color: Color,
    /// Separator thickness.
    pub separator_width: f32,
    /// How far a notch of the wheel scrolls a long menu.
    pub wheel_step: f32,
}

impl MenuStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        Self {
            background: tokens.get(c::MENU_BACKGROUND_COLOR_DEFAULT),
            border_color: tokens.get(c::MENU_BORDER_COLOR_DEFAULT),
            border_width: tokens.get(c::MENU_BORDER_WIDTH_DEFAULT),
            corner_radius: tokens.get(c::MENU_CORNER_RADIUS_DEFAULT),
            padding_vertical: tokens.get(c::MENU_PADDING_VERTICAL_DEFAULT),
            padding_horizontal: tokens.get(c::MENU_PADDING_HORIZONTAL_DEFAULT),
            item_hover: tokens.get(c::MENU_ITEM_BACKGROUND_COLOR_HOVER),
            item_selected: tokens.get(c::MENU_ITEM_BACKGROUND_COLOR_SELECTED),
            item_height: tokens.get(c::MENU_ITEM_HEIGHT_DEFAULT),
            item_gap: tokens.get(c::MENU_ITEM_GAP_DEFAULT),
            item_padding: tokens.get(c::MENU_ITEM_PADDING_HORIZONTAL),
            column_gap: tokens.get(c::MENU_COLUMN_GAP_DEFAULT),
            icon_gap: tokens.get(c::MENU_ITEM_ICON_GAP_DEFAULT),
            max_height: tokens.get(c::MENU_MAX_HEIGHT_DEFAULT),
            title_color: tokens.get(c::MENU_TITLE_LABEL_COLOR_DEFAULT),
            label: tokens.get(c::MENU_ITEM_LABEL_COLOR_DEFAULT),
            label_hover: tokens.get(c::MENU_ITEM_LABEL_COLOR_HOVER),
            label_selected: tokens.get(c::MENU_ITEM_LABEL_COLOR_SELECTED),
            label_disabled: tokens.get(c::MENU_ITEM_LABEL_COLOR_DISABLED),
            shortcut_color: tokens.get(c::MENU_ITEM_SHORTCUT_COLOR_DEFAULT),
            icon_color: tokens.get(c::MENU_ITEM_ICON_COLOR_DEFAULT),
            mark_color: tokens.get(c::MENU_ITEM_MARK_COLOR_DEFAULT),
            icon_size: tokens.get(c::ICON_SIZE_DEFAULT),
            stroke_width: tokens.get(c::ICON_STROKE_WIDTH_DEFAULT),
            radio_size: tokens.get(c::MENU_ITEM_RADIO_SIZE_DEFAULT),
            separator_color: tokens.get(c::SEPARATOR_COLOR_DEFAULT),
            separator_width: tokens.get(c::SEPARATOR_SIZE_DEFAULT),
            wheel_step: tokens.get(c::MENU_WHEEL_STEP_DEFAULT),
        }
    }
}

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

/// One open level laid out for this frame.
#[derive(Clone, Copy, Debug, Default)]
struct Frame {
    placed: Option<Placed>,
    columns: Columns,
    /// How far the items reach past the panel: what the wheel can scroll.
    overflow: f32,
}

impl Frame {
    fn rect(&self) -> Rect {
        self.placed.map_or(Rect::ZERO, |placed| placed.rect)
    }
}

/// What the keyboard did this frame.
#[derive(Clone, Copy, Debug, Default)]
struct Keys {
    /// The item it chose.
    choice: Option<MenuChoice>,
    /// The level whose highlight it moved: that level scrolls to show it.
    moved: Option<usize>,
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
    /// [`MENU_CHOOSE`], and [`MENU_CLOSE`] for the innermost menu); long menus scroll.
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
        let frames = self.menu_layout(state, items, bounds, keys.moved);
        if self.menu_too_far(state, &frames) {
            state.close();
            return None;
        }

        let styles = self.styles();
        let style = &styles.menu;
        let depth = state.depth;
        let (mut chosen, mut hovered, mut opened) = (None, None, None);
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
                    ui.clipped(content, |ui| {
                        let mut y =
                            content.min.y + style.padding_vertical - state.levels[level].scroll;
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
                            if matches!(item, MenuItem::Action(_) | MenuItem::Submenu(_)) {
                                let response = ui.interact(id.with(item_key(level, index)), row);
                                if response.hovered && item.is_active() {
                                    hovered = Some((level, index));
                                }
                                if response.clicked && item.is_active() {
                                    match item {
                                        MenuItem::Submenu(_) => opened = Some((level, index)),
                                        _ => chosen = Some(state.choice(level, index)),
                                    }
                                }
                            }
                            let look = if state.levels[level].highlighted == Some(index)
                                || child == Some(index)
                            {
                                RowLook::Highlighted
                            } else if matches!(item, MenuItem::Action(action) if action.selected) {
                                RowLook::Selected
                            } else {
                                RowLook::Normal
                            };
                            let toward_left = child == Some(index) && child_left;
                            layout::draw_row(ui, item, row, &frame.columns, look, toward_left);
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
            state.open_submenu(level, item);
            state.rest = None;
            state.pointed = Some((level, item));
        } else {
            self.menu_pointer(state, items, hovered, &frames);
        }
        None
    }

    /// The keyboard: closing, moving the highlight, opening and closing submenus, choosing.
    fn menu_keys(&mut self, state: &mut MenuState, items: &[MenuItem<'_>]) -> Keys {
        let mut keys = Keys::default();
        let deepest = state.depth - 1;
        let Some(level_items) = state.items_at(items, deepest) else {
            return keys;
        };
        if self.triggered(&MENU_CLOSE) {
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
        match highlighted {
            Some((index, MenuItem::Submenu(submenu)))
                if submenu.enabled && (choose || self.triggered(&MENU_OPEN_SUBMENU)) =>
            {
                state.open_submenu(deepest, index);
                state.step(submenu.items, deepest + 1, true);
                keys.moved = Some(deepest + 1);
            }
            Some((index, MenuItem::Action(action))) if action.enabled && choose => {
                keys.choice = Some(state.choice(deepest, index));
            }
            _ => {}
        }
        keys
    }

    /// Lays every open level out, the root first and each submenu beside its item; scrolls
    /// the level the keyboard moved in to show its highlight.
    fn menu_layout(
        &mut self,
        state: &mut MenuState,
        items: &[MenuItem<'_>],
        bounds: Rect,
        moved: Option<usize>,
    ) -> [Frame; MAX_DEPTH] {
        let style = &self.styles().menu;
        let mut frames = [Frame::default(); MAX_DEPTH];
        // The open child's row and its panel, for the next level.
        let mut parent: Option<(Rect, Rect)> = None;
        let depth = state.depth;
        for (level, frame) in frames.iter_mut().enumerate().take(depth) {
            let Some(level_items) = state.items_at(items, level) else {
                state.close_below(level.saturating_sub(1));
                break;
            };
            let columns = layout::measure(self, level_items);
            let content = columns.size.y;
            let mut size = Vec2::new(
                columns.size.x,
                content.min(style.max_height).min(bounds.height()).max(0.0),
            );
            let placed = match (level, state.anchor, parent) {
                (0, Some(Anchor::Point(point)), _) => Placed {
                    rect: at_point(point, size, bounds),
                    side: Side::Below,
                    align: Alignment::Start,
                },
                (0, Some(Anchor::Control(control, placement)), _) => {
                    if placement.side.is_vertical() {
                        size.x = size.x.max(control.width());
                    }
                    let placed = placement.place(control, size, bounds);
                    state.placed = Some(placed);
                    placed
                }
                (_, _, Some((row, panel))) => {
                    let inset = style.border_width + style.padding_vertical;
                    let control = Rect::from_min_max(
                        Vec2::new(panel.min.x, row.min.y - inset),
                        Vec2::new(panel.max.x, row.max.y + inset),
                    );
                    Placement::RIGHT.place(control, size, bounds)
                }
                _ => break,
            };
            let overflow = (content - placed.rect.height()).max(0.0);
            let visible = placed.rect.height() - 2.0 * style.border_width;
            let current = &mut state.levels[level];
            if moved == Some(level)
                && let Some(highlighted) = current.highlighted
            {
                let top = style.padding_vertical
                    + level_items[..highlighted]
                        .iter()
                        .map(|item| layout::row_height(style, item))
                        .sum::<f32>();
                let bottom = top + layout::row_height(style, &level_items[highlighted]);
                // With the padding around it: the first and last items scroll to the ends.
                current.scroll = current
                    .scroll
                    .min(top - style.padding_vertical)
                    .max(bottom + style.padding_vertical - visible);
            }
            current.scroll = current.scroll.clamp(0.0, overflow);
            *frame = Frame {
                placed: Some(placed),
                columns,
                overflow,
            };
            parent = (level + 1 < state.depth).then(|| {
                let child = state.levels[level + 1].parent;
                let top = placed.rect.min.y + style.border_width + style.padding_vertical
                    - state.levels[level].scroll
                    + level_items[..child.min(level_items.len())]
                        .iter()
                        .map(|item| layout::row_height(style, item))
                        .sum::<f32>();
                let row = Rect::from_min_max(
                    Vec2::new(placed.rect.min.x, top),
                    Vec2::new(placed.rect.max.x, top + style.item_height),
                );
                (row, placed.rect)
            });
        }
        frames
    }

    /// `true` when the pointer went farther than the close distance from every open menu and
    /// from their control, after it came near them once.
    fn menu_too_far(&self, state: &mut MenuState, frames: &[Frame; MAX_DEPTH]) -> bool {
        let limit = self.styles().input.menu_close_distance;
        let Some(pointer) = self.input().pointer() else {
            return false;
        };
        if limit <= 0.0 {
            return false;
        }
        let near = |rect: Rect| distance(rect, pointer) <= limit;
        let close = frames[..state.depth].iter().any(|frame| near(frame.rect()))
            || state.control().is_some_and(near);
        if close {
            state.armed = true;
        }
        state.armed && !close
    }

    /// The pointer: highlighting, the rest that opens submenus, and the aim towards an open
    /// submenu.
    fn menu_pointer(
        &mut self,
        state: &mut MenuState,
        items: &[MenuItem<'_>],
        hovered: Option<(usize, usize)>,
        frames: &[Frame; MAX_DEPTH],
    ) {
        let input = self.styles().input;
        let now = self.input().time();
        let pointer = self.input().pointer();
        if let Some((level, item)) = hovered {
            let child = (level + 1 < state.depth).then(|| state.levels[level + 1].parent);
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
                return;
            }
            if child.is_some()
                && heading(
                    state.aim,
                    pointer,
                    frames[level + 1].rect(),
                    now,
                    input.submenu_grace,
                )
            {
                // Heading for the open submenu: the items crossed wait.
                if let Some(aim) = state.aim {
                    self.request_redraw_at(aim.since.saturating_add(input.submenu_grace));
                }
                return;
            }
            if state.pointed != hovered {
                state.levels[level].highlighted = Some(item);
                state.rest = Some(Rest {
                    level,
                    item,
                    since: now,
                });
            }
        } else {
            state.rest = None;
        }
        state.pointed = hovered;
        // Resting long enough on an item closes the deeper submenus, and opens its own.
        if let Some(rest) = state.rest {
            let due = rest.since.saturating_add(input.submenu_delay);
            if now >= due {
                state.rest = None;
                state.close_below(rest.level);
                let item = state
                    .items_at(items, rest.level)
                    .and_then(|items| items.get(rest.item));
                if matches!(item, Some(MenuItem::Submenu(submenu)) if submenu.enabled) {
                    state.open_submenu(rest.level, rest.item);
                }
            } else {
                self.request_redraw_at(due);
            }
        }
    }

    /// Shows a floating panel of `size` (content and its padding) when `state` is open,
    /// opened, placed and closed like a menu (a press outside, [`MENU_CLOSE`]; never by
    /// moving away, so that its content can be dragged), and returns what `add` returns. `add`
    /// draws the content in the rectangle it receives, inside the panel's border and padding.
    pub fn popup<R>(
        &mut self,
        id: WidgetId,
        state: &mut MenuState,
        size: Vec2,
        bounds: Rect,
        add: impl FnOnce(&mut Self, Rect) -> R,
    ) -> Option<R> {
        let anchor = state.anchor.filter(|_| state.is_open())?;
        self.grab(MENU);
        if self.triggered(&MENU_CLOSE) {
            state.close();
            return None;
        }
        let style = &self.styles().menu;
        let inset = Vec2::new(
            style.border_width + style.padding_horizontal,
            style.border_width + style.padding_vertical,
        );
        let total = size + inset * 2.0;
        let placed = match anchor {
            Anchor::Point(point) => Placed {
                rect: at_point(point, total, bounds),
                side: Side::Below,
                align: Alignment::Start,
            },
            Anchor::Control(control, placement) => {
                let placed = placement.place(control, total, bounds);
                state.placed = Some(placed);
                placed
            }
        };
        let mut result = None;
        self.on_layer(Layer::FLOATING, |ui| {
            let outside = ui.interact(id.with(OUTSIDE), bounds);
            if outside.pressed || outside.secondary_pressed {
                state.close();
                return;
            }
            ui.zone(placed.rect, MENU, |ui| {
                let radii = match state.control() {
                    Some(control) => {
                        placed.panel_corners(control, Corners::all(style.corner_radius))
                    }
                    None => Corners::all(style.corner_radius),
                };
                ui.scene().push_rect(
                    &RectPrimitive::new(placed.rect, style.background)
                        .with_corner_radii(radii)
                        .with_border(style.border_width, style.border_color),
                );
                let content = Rect::from_min_max(placed.rect.min + inset, placed.rect.max - inset);
                result = Some(ui.clipped(content, |ui| add(ui, content)));
            });
        });
        state.levels[0].rect = placed.rect;
        result
    }
}
