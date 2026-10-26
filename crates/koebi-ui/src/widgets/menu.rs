//! Menus: floating lists of choices, opened at the pointer (context menus) or below a control
//! (dropdowns), drawn above everything and taking the keyboard while open.

use koebi_core::input::Key;
use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_input::{ActionDef, ActionGroup, Context, Shortcut};
use koebi_render::{LinePrimitive, RectPrimitive};
use koebi_tokens::{component, semantic};

use crate::id::WidgetId;
use crate::routing::Layer;
use crate::text::Align;
use crate::ui::Ui;
use crate::widgets::{repeating, snap};

/// An open menu: its keys apply wherever the pointer is.
pub const MENU: Context = Context::new("menu");

/// Closes the open menu without choosing.
pub static MENU_CLOSE: ActionDef = ActionDef {
    id: "menu.close",
    label: "Close menu",
    description: "Closes the open menu without choosing anything.",
    context: MENU,
    defaults: &[Shortcut::key(Key::Escape)],
};

/// Highlights the next item.
pub static MENU_NEXT: ActionDef = ActionDef {
    id: "menu.next",
    label: "Next item",
    description: "Highlights the next item of the open menu.",
    context: MENU,
    defaults: &[repeating(Key::Down)],
};

/// Highlights the previous item.
pub static MENU_PREVIOUS: ActionDef = ActionDef {
    id: "menu.previous",
    label: "Previous item",
    description: "Highlights the previous item of the open menu.",
    context: MENU,
    defaults: &[repeating(Key::Up)],
};

/// Chooses the highlighted item.
pub static MENU_CHOOSE: ActionDef = ActionDef {
    id: "menu.choose",
    label: "Choose item",
    description: "Chooses the highlighted item of the open menu.",
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
    actions: &[&MENU_CLOSE, &MENU_NEXT, &MENU_PREVIOUS, &MENU_CHOOSE],
};

/// A choice of a menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuAction<'a> {
    /// What it does.
    pub label: &'a str,
    /// Its shortcut, shown right-aligned.
    pub shortcut: Option<&'a str>,
    /// Disabled items show but cannot be chosen.
    pub enabled: bool,
    /// The current choice of a dropdown.
    pub selected: bool,
}

/// An entry of a menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuItem<'a> {
    /// A choice.
    Action(MenuAction<'a>),
    /// A line between groups of choices.
    Separator,
    /// A heading above a group of choices.
    Title(&'a str),
}

impl<'a> MenuItem<'a> {
    /// An enabled choice.
    pub const fn action(label: &'a str) -> Self {
        Self::Action(MenuAction {
            label,
            shortcut: None,
            enabled: true,
            selected: false,
        })
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

    /// The same choice, enabled or not.
    #[must_use]
    pub const fn enabled(self, enabled: bool) -> Self {
        match self {
            Self::Action(mut action) => {
                action.enabled = enabled;
                Self::Action(action)
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

    /// `true` for choices that can be chosen.
    fn is_choosable(&self) -> bool {
        matches!(self, Self::Action(action) if action.enabled)
    }
}

/// Where a menu opens.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Opening {
    /// Its top-left corner at the point, or wherever it fits around it.
    At(Vec2),
    /// Below the control, at least as wide, or above it when there is no room below.
    Below(Rect),
}

/// Whether a menu is open, where, and which item is highlighted. Keep one per menu, between
/// frames.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MenuState {
    opening: Option<Opening>,
    highlighted: Option<usize>,
}

impl MenuState {
    /// A closed menu.
    pub const fn new() -> Self {
        Self {
            opening: None,
            highlighted: None,
        }
    }

    /// Opens the menu at `point`: a context menu.
    pub fn open_at(&mut self, point: Vec2) {
        self.opening = Some(Opening::At(point));
        self.highlighted = None;
    }

    /// Opens the menu below `control`: a dropdown.
    pub fn open_below(&mut self, control: Rect) {
        self.opening = Some(Opening::Below(control));
        self.highlighted = None;
    }

    /// Closes the menu.
    pub fn close(&mut self) {
        self.opening = None;
        self.highlighted = None;
    }

    /// `true` while the menu is open.
    pub fn is_open(&self) -> bool {
        self.opening.is_some()
    }

    /// The highlighted item, by index.
    pub fn highlighted(&self) -> Option<usize> {
        self.highlighted
    }

    /// Moves the highlight to the next choosable item after the current one (`step` 1) or
    /// before it (`step` -1), wrapping around.
    fn step(&mut self, items: &[MenuItem<'_>], forward: bool) {
        let count = items.len();
        if count == 0 {
            return;
        }
        let start = match (self.highlighted, forward) {
            (Some(index), true) => index + 1,
            (Some(index), false) => index + count - 1,
            (None, true) => 0,
            (None, false) => count - 1,
        };
        self.highlighted = (0..count)
            .map(|offset| {
                if forward {
                    (start + offset) % count
                } else {
                    (start + count - offset) % count
                }
            })
            .find(|index| items[*index].is_choosable())
            .or(self.highlighted);
    }
}

/// Colors and metrics of [`Ui::menu`], from the `component.menu.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuStyle {
    /// Background of the panel.
    pub background: Color,
    /// Border color.
    pub border_color: Color,
    /// Border width.
    pub border_width: f32,
    /// Corner radius of the panel.
    pub corner_radius: f32,
    /// Space above the first item and below the last one.
    pub padding_vertical: f32,
    /// Background of the highlighted item.
    pub item_hover: Color,
    /// Background of the current choice of a dropdown.
    pub item_selected: Color,
    /// Height of a choice.
    pub item_height: f32,
    /// Space between an item and the panel's sides.
    pub item_gap: f32,
    /// Space between an item's edges and its text.
    pub item_padding: f32,
    /// Space between a label and its shortcut.
    pub column_gap: f32,
    /// Tallest a menu grows.
    pub max_height: f32,
    /// Color of headings.
    pub title_color: Color,
    /// Separator lines.
    pub separator_color: Color,
    /// Separator thickness.
    pub separator_width: f32,
}

impl MenuStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            background: tokens.get(component::MENU_BACKGROUND_COLOR_DEFAULT),
            border_color: tokens.get(component::MENU_BORDER_COLOR_DEFAULT),
            border_width: tokens.get(component::MENU_BORDER_WIDTH_DEFAULT),
            corner_radius: tokens.get(component::MENU_CORNER_RADIUS_DEFAULT),
            padding_vertical: tokens.get(component::MENU_PADDING_VERTICAL_DEFAULT),
            item_hover: tokens.get(component::MENU_ITEM_BACKGROUND_COLOR_HOVER),
            item_selected: tokens.get(component::MENU_ITEM_BACKGROUND_COLOR_SELECTED),
            item_height: tokens.get(semantic::SIZE_CONTROL_HEIGHT),
            item_gap: tokens.get(component::MENU_ITEM_GAP_DEFAULT),
            item_padding: tokens.get(component::MENU_ITEM_PADDING_HORIZONTAL),
            column_gap: tokens.get(component::MENU_COLUMN_GAP_DEFAULT),
            max_height: tokens.get(component::MENU_MAX_HEIGHT_DEFAULT),
            title_color: tokens.get(component::MENU_TITLE_LABEL_COLOR_DEFAULT),
            separator_color: tokens.get(component::SEPARATOR_COLOR_DEFAULT),
            separator_width: tokens.get(component::SEPARATOR_SIZE_DEFAULT),
        }
    }

    /// Height of `item`'s row.
    fn row_height(&self, item: &MenuItem<'_>) -> f32 {
        match item {
            MenuItem::Action(_) | MenuItem::Title(_) => self.item_height,
            MenuItem::Separator => self.padding_vertical + self.separator_width,
        }
    }
}

/// The rectangle of a menu of `size` opened at `opening`, kept inside `bounds`.
fn place(opening: Opening, size: Vec2, bounds: Rect) -> Rect {
    let fit = |start: f32, length: f32, low: f32, high: f32| start.min(high - length).max(low);
    let origin = match opening {
        Opening::At(point) => {
            // Open towards the room left: right and down first, else left or up.
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
            Vec2::new(x, y)
        }
        Opening::Below(control) => {
            let y = if control.max.y + size.y <= bounds.max.y {
                control.max.y
            } else {
                control.min.y - size.y
            };
            Vec2::new(control.min.x, y)
        }
    };
    let origin = Vec2::new(
        fit(origin.x, size.x, bounds.min.x, bounds.max.x),
        fit(origin.y, size.y, bounds.min.y, bounds.max.y),
    );
    snap(Rect::from_origin_size(origin, size))
}

impl Ui<'_> {
    /// Shows the menu of `state` when it is open, above everything, kept inside `bounds` (the
    /// window), and returns the item chosen this frame, by index. Choosing closes the menu,
    /// and so do a press outside it (which nothing else receives) and [`MENU_CLOSE`]; the
    /// keyboard moves the highlight ([`MENU_NEXT`], [`MENU_PREVIOUS`]) and chooses
    /// ([`MENU_CHOOSE`]).
    pub fn menu(
        &mut self,
        id: WidgetId,
        state: &mut MenuState,
        items: &[MenuItem<'_>],
        bounds: Rect,
    ) -> Option<usize> {
        let opening = state.opening?;
        self.grab(MENU);
        if self.triggered(&MENU_CLOSE) {
            state.close();
            return None;
        }
        if self.triggered(&MENU_NEXT) {
            state.step(items, true);
        }
        if self.triggered(&MENU_PREVIOUS) {
            state.step(items, false);
        }
        if self.triggered(&MENU_CHOOSE)
            && let Some(index) = state.highlighted
            && items.get(index).is_some_and(MenuItem::is_choosable)
        {
            state.close();
            return Some(index);
        }

        let styles = self.styles();
        let style = styles.menu;
        let (label_font, detail_font) = (styles.text.label.clone(), styles.text.detail.clone());
        let (mut labels, mut shortcuts) = (0.0_f32, 0.0_f32);
        for item in items {
            match item {
                MenuItem::Action(action) => {
                    labels = labels.max(self.text_width(action.label, &label_font));
                    if let Some(shortcut) = action.shortcut {
                        shortcuts = shortcuts.max(self.text_width(shortcut, &detail_font));
                    }
                }
                MenuItem::Title(title) => labels = labels.max(self.text_width(title, &detail_font)),
                MenuItem::Separator => {}
            }
        }
        let columns = if shortcuts > 0.0 {
            labels + style.column_gap + shortcuts
        } else {
            labels
        };
        let inset = style.border_width + style.item_gap;
        let mut width = (columns + 2.0 * (inset + style.item_padding)).ceil();
        if let Opening::Below(control) = opening {
            width = width.max(control.width());
        }
        let rows: f32 = items.iter().map(|item| style.row_height(item)).sum();
        let height = (rows + 2.0 * (style.border_width + style.padding_vertical))
            .min(style.max_height)
            .min(bounds.height());
        let panel = place(opening, Vec2::new(width, height), bounds);

        let mut chosen = None;
        self.on_layer(Layer::FLOATING, |ui| {
            // Presses outside the menu close it, and reach nothing else.
            let outside = ui.interact(id.with(u64::MAX), bounds);
            if outside.pressed || outside.secondary_pressed {
                state.close();
                return;
            }
            ui.zone(panel, MENU, |ui| {
                ui.scene.push_rect(
                    &RectPrimitive::new(panel, style.background)
                        .with_corner_radius(style.corner_radius)
                        .with_border(style.border_width, style.border_color),
                );
                let content = panel.inset(style.border_width);
                ui.clipped(content, |ui| {
                    let mut y = content.min.y + style.padding_vertical;
                    let text = &ui.styles().text;
                    let (primary, secondary, disabled) =
                        (text.primary, text.secondary, text.disabled);
                    for (index, item) in items.iter().enumerate() {
                        let row = Rect::from_min_max(
                            Vec2::new(content.min.x + style.item_gap, y),
                            Vec2::new(content.max.x - style.item_gap, y + style.row_height(item)),
                        );
                        y = row.max.y;
                        let text_rect = Rect::from_min_max(
                            Vec2::new(row.min.x + style.item_padding, row.min.y),
                            Vec2::new(row.max.x - style.item_padding, row.max.y),
                        );
                        match item {
                            MenuItem::Action(action) => {
                                let key = u64::try_from(index).unwrap_or(u64::MAX - 1);
                                let response = ui.interact(id.with(key), row);
                                if action.enabled && response.hovered {
                                    state.highlighted = Some(index);
                                }
                                if action.enabled && response.clicked {
                                    chosen = Some(index);
                                }
                                let background =
                                    if action.enabled && state.highlighted == Some(index) {
                                        Some(style.item_hover)
                                    } else if action.selected {
                                        Some(style.item_selected)
                                    } else {
                                        None
                                    };
                                if let Some(background) = background {
                                    ui.scene.push_rect(
                                        &RectPrimitive::new(row, background)
                                            .with_corner_radius(style.corner_radius),
                                    );
                                }
                                let (ink, hint) = if action.enabled {
                                    (primary, secondary)
                                } else {
                                    (disabled, disabled)
                                };
                                ui.label(action.label, text_rect, &label_font, ink, Align::Start);
                                if let Some(shortcut) = action.shortcut {
                                    ui.label(shortcut, text_rect, &detail_font, hint, Align::End);
                                }
                            }
                            MenuItem::Title(title) => {
                                ui.label(
                                    title,
                                    text_rect,
                                    &detail_font,
                                    style.title_color,
                                    Align::Start,
                                );
                            }
                            MenuItem::Separator => {
                                let middle = row.center().y;
                                ui.scene.push_line(&LinePrimitive::new(
                                    Vec2::new(text_rect.min.x, middle),
                                    Vec2::new(text_rect.max.x, middle),
                                    style.separator_width,
                                    style.separator_color,
                                ));
                            }
                        }
                    }
                });
            });
        });
        if chosen.is_some() {
            state.close();
        }
        chosen
    }
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;
    use koebi_text::Fonts;

    use super::*;
    use crate::{Input, PointerButton, Styles, UiState};

    const MENU_ID: WidgetId = WidgetId::new("menu");

    fn bounds() -> Rect {
        Rect::from_origin_size(Vec2::ZERO, Vec2::new(400.0, 300.0))
    }

    fn items() -> [MenuItem<'static>; 5] {
        [
            MenuItem::Title("Area"),
            MenuItem::action("Swap areas"),
            MenuItem::action("Join areas").enabled(false),
            MenuItem::Separator,
            MenuItem::action("Vertical split").shortcut("Ctrl+V"),
        ]
    }

    /// Runs one frame showing the menu; returns the choice and the scene.
    fn frame(
        state: &mut UiState,
        menu: &mut MenuState,
        input: &mut Input,
        styles: &Styles,
    ) -> (Option<usize>, Scene) {
        let mut scene = Scene::new();
        let chosen = {
            let mut ui = Ui::new(&mut scene, styles, input, state);
            ui.menu(MENU_ID, menu, &items(), bounds())
        };
        input.end_frame();
        (chosen, scene)
    }

    fn setup() -> (UiState, MenuState, Input, Styles) {
        let mut state = UiState::new();
        state.set_fonts(Fonts::with_embedded());
        (state, MenuState::new(), Input::new(), crate::test_styles())
    }

    #[test]
    fn menus_stay_inside_their_bounds() {
        let size = Vec2::new(100.0, 80.0);
        let bounds = bounds();
        // Room on the right and below: the corner at the point.
        assert_eq!(
            place(Opening::At(Vec2::new(10.0, 10.0)), size, bounds).min,
            Vec2::new(10.0, 10.0)
        );
        // Near the bottom-right corner: opens left and up.
        let rect = place(Opening::At(Vec2::new(390.0, 290.0)), size, bounds);
        assert_eq!(rect.min, Vec2::new(290.0, 210.0));
        // Below a control, or above it without room below.
        let control = Rect::from_origin_size(Vec2::new(20.0, 100.0), Vec2::new(80.0, 20.0));
        assert_eq!(
            place(Opening::Below(control), size, bounds).min,
            Vec2::new(20.0, 120.0)
        );
        let low = Rect::from_origin_size(Vec2::new(20.0, 250.0), Vec2::new(80.0, 20.0));
        assert_eq!(
            place(Opening::Below(low), size, bounds).min,
            Vec2::new(20.0, 170.0)
        );
        // Too large for any side: clamped to the bounds.
        let huge = place(
            Opening::At(Vec2::new(50.0, 50.0)),
            Vec2::new(500.0, 50.0),
            bounds,
        );
        assert_eq!(huge.min.x, 0.0);
    }

    #[test]
    fn the_keyboard_highlights_choosable_items_and_chooses() {
        let mut menu = MenuState::new();
        let items = items();
        menu.step(&items, true);
        assert_eq!(menu.highlighted(), Some(1));
        // The disabled item, the separator and the title are skipped; the end wraps around.
        menu.step(&items, true);
        assert_eq!(menu.highlighted(), Some(4));
        menu.step(&items, true);
        assert_eq!(menu.highlighted(), Some(1));
        menu.step(&items, false);
        assert_eq!(menu.highlighted(), Some(4));
        let mut empty = MenuState::new();
        empty.step(&[MenuItem::Separator], true);
        assert_eq!(empty.highlighted(), None);
    }

    #[test]
    fn clicking_an_item_chooses_it_and_closes_the_menu() {
        let (mut state, mut menu, mut input, styles) = setup();
        menu.open_at(Vec2::new(10.0, 10.0));
        let (chosen, scene) = frame(&mut state, &mut menu, &mut input, &styles);
        assert_eq!(chosen, None);
        assert!(scene.glyph_count() > 20, "{}", scene.glyph_count());
        // The third row (after the title and "Swap areas") is the disabled "Join areas":
        // clicking it does nothing; the fifth is "Vertical split".
        let style = styles.menu;
        let row = |index: usize| {
            let top = 10.0 + style.border_width + style.padding_vertical;
            let heights: f32 = items()[..index]
                .iter()
                .map(|item| style.row_height(item))
                .sum();
            Vec2::new(30.0, top + heights + 2.0)
        };
        for (index, expected) in [(2, None), (4, Some(4))] {
            input.primary_button(true, row(index));
            input.primary_button(false, row(index));
            let (chosen, _) = frame(&mut state, &mut menu, &mut input, &styles);
            assert_eq!(chosen, expected, "row {index}");
        }
        assert!(!menu.is_open());
    }

    #[test]
    fn a_choice_asks_for_the_next_frame_at_once() {
        // Regression test for #104: the frame that chooses applies the choice after the parts
        // drawn before it; without another frame, nothing shows it until the pointer moves.
        let (mut state, mut menu, mut input, styles) = setup();
        menu.open_at(Vec2::new(10.0, 10.0));
        frame(&mut state, &mut menu, &mut input, &styles);
        // Moving the pointer alone changes nothing that was drawn: no extra frame.
        input.pointer_moved(Vec2::new(30.0, 40.0));
        frame(&mut state, &mut menu, &mut input, &styles);
        assert_eq!(state.redraw_at(), None);
        let style = styles.menu;
        let second = Vec2::new(
            30.0,
            10.0 + style.border_width + style.padding_vertical + style.item_height * 1.5,
        );
        input.primary_button(true, second);
        input.primary_button(false, second);
        let (chosen, _) = frame(&mut state, &mut menu, &mut input, &styles);
        assert_eq!(chosen, Some(1));
        assert_eq!(state.redraw_at(), Some(input.time()));
    }

    #[test]
    fn presses_outside_close_the_menu_and_reach_nothing() {
        let (mut state, mut menu, mut input, styles) = setup();
        menu.open_at(Vec2::new(10.0, 10.0));
        frame(&mut state, &mut menu, &mut input, &styles);
        input.button(PointerButton::Secondary, true, Vec2::new(350.0, 250.0));
        let (chosen, _) = frame(&mut state, &mut menu, &mut input, &styles);
        assert_eq!(chosen, None);
        assert!(!menu.is_open());
        // While it was open, the menu took the keyboard.
        let mut chain = Vec::new();
        state.chain_at(None, &mut chain);
        assert_eq!(chain, [MENU]);
    }

    #[test]
    fn escape_closes_and_enter_chooses_the_highlighted_item() {
        let (mut state, mut menu, mut input, styles) = setup();
        menu.open_at(Vec2::new(10.0, 10.0));
        input.trigger(&MENU_NEXT);
        frame(&mut state, &mut menu, &mut input, &styles);
        assert_eq!(menu.highlighted(), Some(1));
        input.trigger(&MENU_CHOOSE);
        let (chosen, _) = frame(&mut state, &mut menu, &mut input, &styles);
        assert_eq!(chosen, Some(1));
        menu.open_below(Rect::from_origin_size(
            Vec2::new(5.0, 5.0),
            Vec2::new(200.0, 20.0),
        ));
        input.trigger(&MENU_CLOSE);
        let (chosen, scene) = frame(&mut state, &mut menu, &mut input, &styles);
        assert_eq!(chosen, None);
        assert!(!menu.is_open() && scene.is_empty());
    }
}
