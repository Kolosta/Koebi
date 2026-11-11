//! Menu bars: titles along a bar, each opening its menu below it.
//!
//! A press on a title opens its menu; while one is open, moving over another title opens that one
//! instead, and the left and right arrows go from menu to menu. Only one menu is open at a time:
//! one [`MenuBarState`] holds it.

use koebi_core::{Rect, Vec2};
use koebi_render::RectPrimitive;

use crate::bar::BarLayout;
use crate::id::WidgetId;
use crate::text::Align;
use crate::ui::Ui;
use crate::widgets::menu::{
    MENU_CLOSE_SUBMENU, MENU_OPEN_SUBMENU, MenuChoice, MenuItem, MenuState,
};

/// A menu of a bar.
#[derive(Clone, Copy, Debug)]
pub struct MenuBarEntry<'a> {
    /// Its title on the bar.
    pub title: &'a str,
    /// Its items.
    pub items: &'a [MenuItem<'a>],
}

/// Which menu of a bar is open. Keep one per bar, between frames.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MenuBarState {
    menu: MenuState,
    open: Option<usize>,
    /// Where the pointer was on the last frame: only moving it takes over from the arrows.
    pointer: Option<Vec2>,
}

impl MenuBarState {
    /// The menu open, by its place on the bar.
    pub fn open(&self) -> Option<usize> {
        self.open
    }

    fn open_at(&mut self, index: usize, title: Rect) {
        self.menu.close();
        self.menu.open_below(title);
        self.open = Some(index);
    }
}

/// The widget key of the open menu.
const MENU_KEY: u64 = u64::MAX;

impl Ui<'_> {
    /// Shows the titles of `entries` from the start of `layout`, and the menu open, kept inside
    /// `bounds` (the window). Returns the menu and the item chosen this frame.
    pub fn menu_bar(
        &mut self,
        id: WidgetId,
        state: &mut MenuBarState,
        entries: &[MenuBarEntry<'_>],
        layout: &mut BarLayout,
        bounds: Rect,
    ) -> Option<(usize, MenuChoice)> {
        let style = self.styles().app_bar;
        let font = self.styles().text.label.clone();
        let pointer = self.input().pointer();
        let moved = pointer != state.pointer;
        state.pointer = pointer;
        let mut rects = [Rect::ZERO; 16];
        let mut hovered = [false; 16];
        let count = entries.len().min(rects.len());
        // What the frame shows is settled before anything is drawn: a press, the pointer or an
        // arrow that opens another menu shows it at once, and only its title lit.
        for (index, entry) in entries.iter().take(count).enumerate() {
            let width = self.text_width(entry.title, &font) + 2.0 * style.menu_padding;
            let rect = layout.left(width);
            rects[index] = rect;
            let response = self.interact(id.with(index as u64), rect);
            hovered[index] = response.hovered;
            if response.pressed {
                state.open_at(index, rect);
            }
            // One is open: the others open as the pointer comes over them.
            if moved
                && state.open.is_some_and(|open| open != index)
                && pointer.is_some_and(|point| rect.contains(point))
            {
                state.open_at(index, rect);
            }
        }
        // The arrows go from menu to menu, where they have nothing to do inside it.
        if let Some(open) = state.open.filter(|open| *open < count)
            && state.menu.depth() == 1
        {
            let on_submenu = state
                .menu
                .highlighted()
                .and_then(|item| entries[open].items.get(item))
                .is_some_and(|item| matches!(item, MenuItem::Submenu(_)));
            let step = if self.triggered(&MENU_CLOSE_SUBMENU) {
                self.consume(&MENU_CLOSE_SUBMENU);
                Some(count - 1)
            } else if !on_submenu && self.triggered(&MENU_OPEN_SUBMENU) {
                self.consume(&MENU_OPEN_SUBMENU);
                Some(1)
            } else {
                None
            };
            if let Some(step) = step {
                let next = (open + step) % count;
                state.open_at(next, rects[next]);
            }
        }
        for (index, entry) in entries.iter().take(count).enumerate() {
            let rect = rects[index];
            if state.open == Some(index) || hovered[index] {
                let background = RectPrimitive::new(rect, style.button_hover)
                    .with_corner_radius(style.menu_radius);
                self.scene().push_rect(&background);
            }
            let inner = Rect::from_min_max(
                Vec2::new(rect.min.x + style.menu_padding, rect.min.y),
                Vec2::new(rect.max.x - style.menu_padding, rect.max.y),
            );
            self.label(entry.title, inner, &font, style.menu_title, Align::Start);
        }
        let open = state.open.filter(|open| *open < count)?;
        let items = entries[open].items;
        let chosen = self.menu(id.with(MENU_KEY), &mut state.menu, items, bounds);
        if !state.menu.is_open() {
            state.open = None;
        }
        chosen.map(|choice| (open, choice))
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::input::Key;
    use koebi_render::Scene;
    use koebi_text::Fonts;

    use super::*;
    use crate::{Input, Styles, UiState};

    const ID: WidgetId = WidgetId::new("menus");

    fn window() -> Rect {
        Rect::from_origin_size(Vec2::ZERO, Vec2::new(600.0, 400.0))
    }

    struct Harness {
        state: UiState,
        menus: MenuBarState,
        input: Input,
        styles: Styles,
        /// What the last frame drew: rectangles and glyphs.
        drawn: (usize, usize),
    }

    impl Harness {
        fn new() -> Self {
            let mut state = UiState::new();
            state.set_fonts(Fonts::with_embedded());
            Self {
                state,
                menus: MenuBarState::default(),
                input: Input::new(),
                styles: crate::test_styles(),
                drawn: (0, 0),
            }
        }

        fn frame(&mut self) -> Option<(usize, usize)> {
            let file = [MenuItem::action("New"), MenuItem::action("Quit")];
            let edit = [MenuItem::action("Settings")];
            let entries = [
                MenuBarEntry {
                    title: "File",
                    items: &file,
                },
                MenuBarEntry {
                    title: "Edit",
                    items: &edit,
                },
            ];
            let mut scene = Scene::new();
            let chosen = {
                let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
                let bar = Rect::from_origin_size(Vec2::ZERO, Vec2::new(600.0, 20.0));
                let mut layout = BarLayout::new(bar, 0.0);
                ui.menu_bar(ID, &mut self.menus, &entries, &mut layout, window())
            };
            self.input.end_frame();
            self.drawn = (scene.rect_count(), scene.glyph_count());
            chosen.map(|(menu, choice)| (menu, choice.index()))
        }

        fn press(&mut self, at: Vec2) -> Option<(usize, usize)> {
            self.input.pointer_moved(at);
            self.frame();
            self.input.primary_button(true, at);
            let chosen = self.frame();
            self.input.primary_button(false, at);
            chosen.or_else(|| self.frame())
        }
    }

    /// Over the title of "File", then of "Edit".
    const FILE: Vec2 = Vec2::new(10.0, 10.0);
    const EDIT: Vec2 = Vec2::new(60.0, 10.0);

    #[test]
    fn a_press_opens_a_menu_and_another_closes_it() {
        let mut harness = Harness::new();
        harness.frame();
        assert_eq!(harness.press(FILE), None);
        assert_eq!(harness.menus.open(), Some(0));
        harness.frame();
        // On its title: the press belongs to the open menu, which closes.
        harness.press(FILE);
        assert_eq!(harness.menus.open(), None);
    }

    #[test]
    fn moving_over_another_title_opens_its_menu() {
        let mut harness = Harness::new();
        harness.frame();
        harness.press(FILE);
        harness.frame();
        harness.input.pointer_moved(EDIT);
        harness.frame();
        assert_eq!(harness.menus.open(), Some(1));
    }

    #[test]
    fn choosing_an_item_tells_its_menu_and_closes_it() {
        let mut harness = Harness::new();
        harness.frame();
        harness.press(FILE);
        harness.frame();
        let placed = harness.menus.menu.placed().expect("open").rect;
        // The second item.
        let row = harness.styles.menu.item_height;
        let at = Vec2::new(placed.min.x + 10.0, placed.min.y + row * 1.5 + 2.0);
        harness.input.pointer_moved(at);
        harness.frame();
        harness.frame();
        assert_eq!(harness.press(at), Some((0, 1)));
        assert_eq!(harness.menus.open(), None);
    }

    #[test]
    fn the_arrows_go_from_menu_to_menu() {
        let mut harness = Harness::new();
        harness.frame();
        harness.press(FILE);
        harness.frame();
        harness.input.trigger(&MENU_OPEN_SUBMENU);
        harness.frame();
        assert_eq!(harness.menus.open(), Some(1));
        harness.frame();
        harness.input.trigger(&MENU_CLOSE_SUBMENU);
        harness.frame();
        assert_eq!(harness.menus.open(), Some(0));
        // Around the ends.
        harness.frame();
        harness.input.trigger(&MENU_CLOSE_SUBMENU);
        harness.frame();
        assert_eq!(harness.menus.open(), Some(1));
        assert_eq!(
            MENU_OPEN_SUBMENU.defaults[0],
            koebi_input::Shortcut::key(Key::Right)
        );
    }

    #[test]
    fn the_menu_an_arrow_opens_shows_in_the_same_frame() {
        let mut harness = Harness::new();
        harness.frame();
        harness.press(FILE);
        harness.frame();
        harness.input.trigger(&MENU_OPEN_SUBMENU);
        harness.frame();
        let switched = harness.drawn;
        harness.frame();
        // No frame without a menu, nor with two titles lit.
        assert_eq!(switched, harness.drawn);
    }
}
