//! Tests of menus: opening, placement, keyboard, choosing, closing, submenus and popups.

use core::time::Duration;

use koebi_core::{Rect, Vec2};
use koebi_render::Scene;
use koebi_text::Fonts;

use super::*;
use crate::icon::Icon;
use crate::widgets::parts::Placement;
use crate::{Input, PointerButton, ScrollAxes, ScrollState, Styles, UiState};

const MENU_ID: WidgetId = WidgetId::new("menu");

const ADD: [MenuItem<'static>; 3] = [
    MenuItem::action("Cube"),
    MenuItem::action("Sphere").checked(true),
    MenuItem::action("Plane"),
];

fn items() -> [MenuItem<'static>; 6] {
    [
        MenuItem::Title("Area"),
        MenuItem::action("Swap areas"),
        MenuItem::action("Join areas").enabled(false),
        MenuItem::Separator,
        MenuItem::action("Vertical split").shortcut("Ctrl+V"),
        MenuItem::submenu("Add", &ADD).icon(Icon::Plus),
    ]
}

fn bounds() -> Rect {
    Rect::from_origin_size(Vec2::ZERO, Vec2::new(400.0, 300.0))
}

/// A menu, its UI state and input, one frame at a time.
struct Harness {
    state: UiState,
    menu: MenuState,
    input: Input,
    styles: Styles,
}

impl Harness {
    fn new() -> Self {
        let mut state = UiState::new();
        state.set_fonts(Fonts::with_embedded());
        Self {
            state,
            menu: MenuState::new(),
            input: Input::new(),
            styles: crate::test_styles(),
        }
    }

    /// One frame showing `items`.
    fn frame_with(&mut self, items: &[MenuItem<'_>]) -> (Option<MenuChoice>, Scene) {
        let mut scene = Scene::new();
        let chosen = {
            let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
            ui.menu(MENU_ID, &mut self.menu, items, bounds())
        };
        self.input.end_frame();
        (chosen, scene)
    }

    fn frame(&mut self) -> Option<MenuChoice> {
        self.frame_with(&items()).0
    }

    /// The middle of `item`'s row in the panel of `level`, as drawn on the last frame.
    fn row(&self, level: usize, item: usize, items: &[MenuItem<'_>]) -> Vec2 {
        let style = &self.styles.menu;
        let panel = self.menu.levels[level].rect;
        let above: f32 = items[..item]
            .iter()
            .map(|item| layout::row_height(style, item))
            .sum();
        let top = panel.min.y + style.border_width + style.padding_vertical
            - self.menu.levels[level].scroll
            + above;
        Vec2::new(
            panel.min.x + 30.0,
            top + layout::row_height(style, &items[item]) * 0.5,
        )
    }

    fn click(&mut self, at: Vec2) {
        self.input.primary_button(true, at);
        self.input.primary_button(false, at);
    }

    fn at(&mut self, time: Duration) {
        self.input.set_time(time);
    }
}

#[test]
fn context_menus_stay_inside_their_bounds() {
    let size = Vec2::new(100.0, 80.0);
    // Room on the right and below: the corner at the point.
    assert_eq!(
        at_point(Vec2::new(10.0, 10.0), size, bounds()).min,
        Vec2::new(10.0, 10.0)
    );
    // Near the bottom-right corner: opens left and up.
    assert_eq!(
        at_point(Vec2::new(390.0, 290.0), size, bounds()).min,
        Vec2::new(290.0, 210.0)
    );
    // Too large for any side: kept inside.
    assert_eq!(
        at_point(Vec2::new(50.0, 50.0), Vec2::new(500.0, 50.0), bounds())
            .min
            .x,
        0.0
    );
}

#[test]
fn the_keyboard_highlights_active_items_wrapping_around() {
    let mut menu = MenuState::new();
    menu.open_at(Vec2::ZERO);
    let items = items();
    menu.step(&items, 0, true);
    assert_eq!(menu.highlighted(), Some(1));
    // The disabled item, the separator and the title are skipped; submenus are not.
    menu.step(&items, 0, true);
    assert_eq!(menu.highlighted(), Some(4));
    menu.step(&items, 0, true);
    assert_eq!(menu.highlighted(), Some(5));
    menu.step(&items, 0, true);
    assert_eq!(menu.highlighted(), Some(1));
    menu.step(&items, 0, false);
    assert_eq!(menu.highlighted(), Some(5));
    let mut empty = MenuState::new();
    empty.open_at(Vec2::ZERO);
    empty.step(&[MenuItem::Separator], 0, true);
    assert_eq!(empty.highlighted(), None);
}

#[test]
fn clicking_an_item_chooses_it_and_closes_the_menu() {
    let mut harness = Harness::new();
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    let (chosen, scene) = harness.frame_with(&items());
    assert_eq!(chosen, None);
    assert!(scene.glyph_count() > 20, "{}", scene.glyph_count());
    // The disabled "Join areas" does nothing; "Vertical split" is chosen.
    let join = harness.row(0, 2, &items());
    harness.click(join);
    assert_eq!(harness.frame(), None);
    assert!(harness.menu.is_open());
    let split = harness.row(0, 4, &items());
    harness.click(split);
    let chosen = harness.frame().expect("a choice");
    assert_eq!((chosen.index(), chosen.path()), (4, &[4][..]));
    assert!(!harness.menu.is_open());
}

#[test]
fn a_choice_asks_for_the_next_frame_at_once() {
    // Regression test for #104: the frame that chooses applies the choice after the parts
    // drawn before it; without another frame, nothing shows it until the pointer moves.
    let mut harness = Harness::new();
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    harness.frame();
    let swap = harness.row(0, 1, &items());
    harness.click(swap);
    assert!(harness.frame().is_some());
    assert_eq!(harness.state.redraw_at(), Some(harness.input.time()));
}

#[test]
fn presses_outside_close_the_menus_and_reach_nothing() {
    let mut harness = Harness::new();
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    harness.frame();
    harness
        .input
        .button(PointerButton::Secondary, true, Vec2::new(350.0, 250.0));
    assert_eq!(harness.frame(), None);
    assert!(!harness.menu.is_open());
    // While it was open, the menu took the keyboard.
    let mut chain = Vec::new();
    harness.state.chain_at(None, &mut chain);
    assert_eq!(chain, [MENU]);
}

#[test]
fn the_keyboard_opens_submenus_chooses_and_closes_one_menu_at_a_time() {
    let mut harness = Harness::new();
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    harness.input.trigger(&MENU_NEXT);
    harness.frame();
    assert_eq!(harness.menu.highlighted(), Some(1));
    harness.input.trigger(&MENU_CHOOSE);
    assert_eq!(harness.frame().map(|choice| choice.index()), Some(1));

    // The last item opens a submenu whose first item is highlighted.
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    harness.input.trigger(&MENU_PREVIOUS);
    harness.frame();
    harness.input.trigger(&MENU_OPEN_SUBMENU);
    harness.frame();
    assert_eq!(
        (harness.menu.depth(), harness.menu.highlighted()),
        (2, Some(0))
    );
    harness.input.trigger(&MENU_NEXT);
    harness.input.trigger(&MENU_CHOOSE);
    let chosen = harness.frame().expect("a nested choice");
    assert_eq!(chosen.path(), &[5, 1]);

    // Escape and Left close the innermost menu only.
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    harness.input.trigger(&MENU_PREVIOUS);
    harness.frame();
    harness.input.trigger(&MENU_CHOOSE);
    harness.frame();
    assert_eq!(harness.menu.depth(), 2);
    harness.input.trigger(&MENU_CLOSE_SUBMENU);
    harness.frame();
    assert_eq!(harness.menu.depth(), 1);
    harness.input.trigger(&MENU_OPEN_SUBMENU);
    harness.frame();
    harness.input.trigger(&MENU_CLOSE);
    harness.frame();
    assert_eq!(harness.menu.depth(), 1);
    harness.input.trigger(&MENU_CLOSE);
    let (chosen, scene) = harness.frame_with(&items());
    assert!(chosen.is_none() && !harness.menu.is_open() && scene.is_empty());
}

#[test]
fn submenus_open_when_the_pointer_rests_on_their_item() {
    let mut harness = Harness::new();
    let delay = harness.styles.input.submenu_delay;
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    harness.frame();
    let add = harness.row(0, 5, &items());
    harness.input.pointer_moved(add);
    harness.frame();
    assert_eq!(harness.menu.depth(), 1);
    // The frame of the end of the rest is asked for.
    assert_eq!(harness.state.redraw_at(), Some(delay));
    harness.at(delay / 2);
    harness.frame();
    assert_eq!(harness.menu.depth(), 1);
    harness.at(delay);
    harness.frame();
    assert_eq!(harness.menu.depth(), 2);
    // The submenu opened beside its item, its first row level with it.
    harness.frame();
    let cube = harness.row(1, 0, &ADD);
    assert!((cube.y - add.y).abs() < 1.0, "{cube:?} {add:?}");
    assert!(harness.menu.levels[1].rect.min.x >= harness.menu.levels[0].rect.max.x - 1.0);
    harness.click(cube);
    assert_eq!(
        harness.frame().map(|choice| choice.path()[..].to_vec()),
        Some(vec![5, 0])
    );
}

#[test]
fn a_click_opens_a_submenu_at_once() {
    let mut harness = Harness::new();
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    harness.frame();
    let add = harness.row(0, 5, &items());
    harness.click(add);
    assert_eq!(harness.frame(), None);
    assert_eq!(harness.menu.depth(), 2);
}

#[test]
fn the_pointer_heading_for_a_submenu_does_not_close_it() {
    let mut harness = Harness::new();
    let input = harness.styles.input;
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    harness.frame();
    let add = harness.row(0, 5, &items());
    harness.click(add);
    harness.frame();
    harness.frame();
    assert_eq!(harness.menu.depth(), 2);
    // Over the submenu's item, near its left end: the aim starts there.
    let panel = harness.menu.levels[0].rect;
    let start = Vec2::new(panel.min.x + 40.0, add.y);
    harness.input.pointer_moved(start);
    harness.frame();
    // Up and right, into the item above but towards the submenu: it stays open.
    let crossing = Vec2::new(
        panel.max.x - 10.0,
        add.y - harness.styles.menu.item_height * 0.5 - 2.0,
    );
    harness.at(Duration::from_millis(10));
    harness.input.pointer_moved(crossing);
    harness.frame();
    assert_eq!(
        (harness.menu.depth(), harness.menu.levels[0].highlighted),
        (2, Some(5))
    );
    // Resting there past the grace: the item above takes over, and after the delay the
    // submenu closes.
    let after = Duration::from_millis(10) + input.submenu_grace;
    harness.at(after);
    harness.frame();
    assert_eq!(harness.menu.levels[0].highlighted, Some(4));
    harness.at(after + input.submenu_delay);
    harness.frame();
    assert_eq!(harness.menu.depth(), 1);
}

#[test]
fn moving_far_from_the_menus_closes_them() {
    let mut harness = Harness::new();
    let limit = harness.styles.input.menu_close_distance;
    // Opened far from the pointer: it waits for the pointer to come near.
    harness.input.pointer_moved(Vec2::new(390.0, 290.0));
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    harness.frame();
    harness.frame();
    assert!(harness.menu.is_open());
    // Near it, then away past the distance.
    let swap = harness.row(0, 1, &items());
    harness.input.pointer_moved(swap);
    harness.frame();
    let panel = harness.menu.levels[0].rect;
    harness
        .input
        .pointer_moved(Vec2::new(panel.max.x + limit * 0.5, swap.y));
    harness.frame();
    assert!(harness.menu.is_open(), "still within the distance");
    harness
        .input
        .pointer_moved(Vec2::new(panel.max.x + limit + 2.0, swap.y));
    harness.frame();
    assert!(!harness.menu.is_open());
}

#[test]
fn long_menus_scroll_with_the_wheel_and_follow_the_keyboard() {
    let labels: Vec<String> = (0..40).map(|index| format!("Item {index}")).collect();
    let items: Vec<MenuItem<'_>> = labels.iter().map(|label| MenuItem::action(label)).collect();
    let mut harness = Harness::new();
    harness.menu.open_at(Vec2::new(10.0, 0.0));
    harness.frame_with(&items);
    let panel = harness.menu.levels[0].rect;
    assert!(panel.height() <= bounds().height());
    let style = harness.styles.menu;
    let content = f32::from(u8::try_from(items.len()).unwrap()) * style.item_height
        + 2.0 * (style.border_width + style.padding_vertical);
    let end = content.ceil() - panel.height();
    // The last item, through the keyboard: the menu scrolls to its end.
    harness.input.trigger(&MENU_PREVIOUS);
    harness.frame_with(&items);
    assert_eq!(harness.menu.highlighted(), Some(39));
    assert!((harness.menu.levels[0].scroll - end).abs() < 1.0);
    // The wheel scrolls back, over the menu.
    harness.input.pointer_moved(panel.center());
    harness.frame_with(&items);
    harness.input.scrolled(Vec2::new(0.0, 1.0));
    harness.frame_with(&items);
    assert!((harness.menu.levels[0].scroll - (end - style.wheel_step)).abs() < 1.0);
}

#[test]
fn menus_beside_a_control_flip_and_are_at_least_as_wide() {
    let mut harness = Harness::new();
    let control = Rect::from_origin_size(Vec2::new(20.0, 270.0), Vec2::new(300.0, 20.0));
    harness.menu.open_for(control, Placement::BELOW);
    harness.frame();
    let placed = harness.menu.placed().expect("placed beside its control");
    assert_eq!(placed.side, Side::Above);
    assert_eq!(placed.rect.max.y, control.min.y);
    assert!(placed.rect.width() >= control.width());
}

#[test]
fn popups_draw_their_content_and_close_like_menus() {
    let mut harness = Harness::new();
    let frame = |harness: &mut Harness| {
        let mut scene = Scene::new();
        let content = {
            let mut ui = Ui::new(
                &mut scene,
                &harness.styles,
                &harness.input,
                &mut harness.state,
            );
            ui.popup(
                MENU_ID,
                &mut harness.menu,
                Vec2::new(100.0, 40.0),
                bounds(),
                |_, content| content,
            )
        };
        harness.input.end_frame();
        content
    };
    assert_eq!(frame(&mut harness), None);
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    let content = frame(&mut harness).expect("open");
    assert_eq!(content.size(), Vec2::new(100.0, 40.0));
    harness.input.trigger(&MENU_CLOSE);
    assert_eq!(frame(&mut harness), None);
    assert!(!harness.menu.is_open());
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    frame(&mut harness);
    harness.click(Vec2::new(390.0, 290.0));
    assert_eq!(frame(&mut harness), None);
    assert!(!harness.menu.is_open());
}

/// Nine nested levels, one more than menus open.
const DEEP_8: [MenuItem<'static>; 1] = [MenuItem::action("Leaf")];
const DEEP_7: [MenuItem<'static>; 1] = [MenuItem::submenu("8", &DEEP_8)];
const DEEP_6: [MenuItem<'static>; 1] = [MenuItem::submenu("7", &DEEP_7)];
const DEEP_5: [MenuItem<'static>; 1] = [MenuItem::submenu("6", &DEEP_6)];
const DEEP_4: [MenuItem<'static>; 1] = [MenuItem::submenu("5", &DEEP_5)];
const DEEP_3: [MenuItem<'static>; 1] = [MenuItem::submenu("4", &DEEP_4)];
const DEEP_2: [MenuItem<'static>; 1] = [MenuItem::submenu("3", &DEEP_3)];
const DEEP_1: [MenuItem<'static>; 1] = [MenuItem::submenu("2", &DEEP_2)];
const DEEP: [MenuItem<'static>; 1] = [MenuItem::submenu("1", &DEEP_1)];

#[test]
fn submenus_past_the_depth_limit_do_not_open() {
    // Regression test: the keyboard opened a ninth level, past the end of the levels.
    let mut harness = Harness::new();
    harness.menu.open_at(Vec2::new(4.0, 4.0));
    harness.input.trigger(&MENU_NEXT);
    harness.frame_with(&DEEP);
    for _ in 0..10 {
        harness.input.trigger(&MENU_OPEN_SUBMENU);
        harness.frame_with(&DEEP);
    }
    assert_eq!(harness.menu.depth(), 8);
    // The submenu of the eighth level is not active: nothing to highlight there.
    assert_eq!(harness.menu.highlighted(), None);
}

#[test]
fn items_changing_under_an_open_menu_forget_what_they_lost() {
    // Regression test: a highlight past the end of the new items made the keyboard panic.
    let labels: Vec<String> = (0..10).map(|index| format!("Recent {index}")).collect();
    let recent: Vec<MenuItem<'_>> = labels.iter().map(|label| MenuItem::action(label)).collect();
    let mut harness = Harness::new();
    harness.menu.open_at(Vec2::new(4.0, 4.0));
    harness.input.trigger(&MENU_PREVIOUS);
    harness.frame_with(&recent);
    assert_eq!(harness.menu.highlighted(), Some(9));
    let fewer = [MenuItem::action("Gone").enabled(false)];
    harness.input.trigger(&MENU_NEXT);
    harness.frame_with(&fewer);
    assert_eq!(harness.menu.highlighted(), None);
    // A submenu whose item is disabled while open closes.
    harness.menu.open_at(Vec2::new(4.0, 4.0));
    harness.input.trigger(&MENU_PREVIOUS);
    harness.frame();
    harness.input.trigger(&MENU_OPEN_SUBMENU);
    harness.frame();
    assert_eq!(harness.menu.depth(), 2);
    let mut disabled = items();
    disabled[5] = MenuItem::submenu("Add", &ADD).enabled(false);
    harness.frame_with(&disabled);
    assert_eq!(harness.menu.depth(), 1);
}

#[test]
fn the_frame_that_ends_a_rest_shows_the_submenu() {
    // Regression test: the submenu opened after the frame was drawn, and nothing asked for
    // the frame that would show it.
    let mut harness = Harness::new();
    let delay = harness.styles.input.submenu_delay;
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    harness.frame();
    let add = harness.row(0, 5, &items());
    harness.input.pointer_moved(add);
    let (_, entered) = harness.frame_with(&items());
    harness.at(delay);
    let (_, ended) = harness.frame_with(&items());
    assert_eq!(harness.menu.depth(), 2);
    // The submenu's panel and its items are in the frame that opened it.
    assert!(ended.rect_count() > entered.rect_count());
    assert!(ended.glyph_count() > entered.glyph_count());
}

#[test]
fn entering_an_item_highlights_it_in_the_same_frame() {
    // Regression test: the highlight was decided after the rows were drawn.
    let mut harness = Harness::new();
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    let (_, before) = harness.frame_with(&items());
    let swap = harness.row(0, 1, &items());
    harness.input.pointer_moved(swap);
    let (_, entered) = harness.frame_with(&items());
    assert_eq!(harness.menu.highlighted(), Some(1));
    assert_eq!(entered.rect_count(), before.rect_count() + 1);
}

#[test]
fn moving_over_plain_items_asks_for_no_frame() {
    let plain = [
        MenuItem::action("Swap areas"),
        MenuItem::action("Vertical split"),
    ];
    let mut harness = Harness::new();
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    harness.frame_with(&plain);
    let row = harness.row(0, 1, &plain);
    harness.input.pointer_moved(row);
    harness.frame_with(&plain);
    assert_eq!(harness.menu.highlighted(), Some(1));
    assert_eq!(harness.state.redraw_at(), None);
}

#[test]
fn the_keyboard_keeps_the_highlight_while_the_menu_scrolls_under_a_still_pointer() {
    // Regression test: rows scrolled under a still pointer took the highlight back.
    let labels: Vec<String> = (0..40).map(|index| format!("Item {index}")).collect();
    let items: Vec<MenuItem<'_>> = labels.iter().map(|label| MenuItem::action(label)).collect();
    let mut harness = Harness::new();
    harness.menu.open_at(Vec2::new(10.0, 0.0));
    harness.frame_with(&items);
    let third = harness.row(0, 3, &items);
    harness.input.pointer_moved(third);
    harness.frame_with(&items);
    assert_eq!(harness.menu.highlighted(), Some(3));
    for _ in 0..16 {
        harness.input.trigger(&MENU_NEXT);
        harness.frame_with(&items);
    }
    assert_eq!(harness.menu.highlighted(), Some(19));
    // Moving the pointer gives it back the highlight.
    harness.input.pointer_moved(third + Vec2::new(4.0, 0.0));
    harness.frame_with(&items);
    assert_ne!(harness.menu.highlighted(), Some(19));
}

#[test]
fn submenus_keep_cascading_the_way_the_previous_level_went() {
    // Regression test: once a level flipped left, the next one opened over the root.
    const THIRD: [MenuItem<'static>; 1] = [MenuItem::action("Deep")];
    const SECOND: [MenuItem<'static>; 1] = [MenuItem::submenu("Third", &THIRD)];
    const FIRST: [MenuItem<'static>; 1] = [MenuItem::submenu("Second", &SECOND)];
    let mut harness = Harness::new();
    let control = Rect::from_origin_size(Vec2::new(300.0, 10.0), Vec2::new(90.0, 20.0));
    harness.menu.open_for(control, Placement::BELOW);
    harness.input.trigger(&MENU_NEXT);
    harness.frame_with(&FIRST);
    for _ in 0..2 {
        harness.input.trigger(&MENU_OPEN_SUBMENU);
        harness.frame_with(&FIRST);
    }
    harness.frame_with(&FIRST);
    assert_eq!(harness.menu.depth(), 3);
    let [root, first, second] = [0, 1, 2].map(|level| harness.menu.levels[level].rect);
    assert!(first.max.x <= root.min.x + 1.0, "{root:?} {first:?}");
    assert!(second.max.x <= first.min.x + 1.0, "{first:?} {second:?}");
}

#[test]
fn the_wheel_over_an_open_menu_leaves_the_area_below_alone() {
    // Regression test: floating content inherited the zones of the area that opened it.
    let labels: Vec<String> = (0..40).map(|index| format!("Item {index}")).collect();
    let items: Vec<MenuItem<'_>> = labels.iter().map(|label| MenuItem::action(label)).collect();
    let mut harness = Harness::new();
    let mut scroll = ScrollState::new();
    let area = Rect::from_origin_size(Vec2::ZERO, Vec2::new(400.0, 300.0));
    let frame = |harness: &mut Harness, scroll: &mut ScrollState| {
        let mut scene = Scene::new();
        {
            let mut ui = Ui::new(
                &mut scene,
                &harness.styles,
                &harness.input,
                &mut harness.state,
            );
            ui.scroll_area(
                WidgetId::new("area"),
                area,
                scroll,
                Vec2::new(400.0, 2000.0),
                ScrollAxes::Vertical,
                |ui, _| ui.menu(MENU_ID, &mut harness.menu, &items, bounds()),
            );
        }
        harness.input.end_frame();
    };
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    frame(&mut harness, &mut scroll);
    // Beside the menu, then over it.
    for at in [Vec2::new(380.0, 280.0), Vec2::new(30.0, 40.0)] {
        harness.input.pointer_moved(at);
        frame(&mut harness, &mut scroll);
        harness.input.scrolled(Vec2::new(0.0, -1.0));
        frame(&mut harness, &mut scroll);
        assert_eq!(scroll.offset(), Vec2::ZERO, "at {at:?}");
    }
}

#[test]
fn escape_in_a_popup_closes_the_menu_inside_it_first() {
    let mut harness = Harness::new();
    let mut inner = MenuState::new();
    let frame = |harness: &mut Harness, inner: &mut MenuState| {
        let mut scene = Scene::new();
        {
            let mut ui = Ui::new(
                &mut scene,
                &harness.styles,
                &harness.input,
                &mut harness.state,
            );
            ui.popup(
                MENU_ID,
                &mut harness.menu,
                Vec2::new(120.0, 40.0),
                bounds(),
                |ui, _| {
                    ui.menu(WidgetId::new("inner"), inner, &items(), bounds());
                },
            );
        }
        harness.input.end_frame();
    };
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    inner.open_at(Vec2::new(40.0, 40.0));
    frame(&mut harness, &mut inner);
    harness.input.trigger(&MENU_CLOSE);
    frame(&mut harness, &mut inner);
    assert!(!inner.is_open() && harness.menu.is_open());
    harness.input.trigger(&MENU_CLOSE);
    frame(&mut harness, &mut inner);
    assert!(!harness.menu.is_open());
}

#[test]
fn popups_beside_a_control_square_where_they_meet() {
    let mut harness = Harness::new();
    let control = Rect::from_origin_size(Vec2::new(20.0, 20.0), Vec2::new(100.0, 20.0));
    harness.menu.open_for(control, Placement::BELOW);
    let mut scene = Scene::new();
    {
        let mut ui = Ui::new(
            &mut scene,
            &harness.styles,
            &harness.input,
            &mut harness.state,
        );
        ui.popup(
            MENU_ID,
            &mut harness.menu,
            Vec2::new(80.0, 30.0),
            bounds(),
            |_, _| (),
        );
    }
    let placed = harness.menu.placed().expect("placed beside its control");
    assert_eq!(placed.rect.min, Vec2::new(20.0, 40.0));
    let corners = placed.panel_corners(control, Corners::all(4.0));
    assert_eq!((corners.top_left, corners.bottom_left), (0.0, 4.0));
}

#[test]
fn closing_by_distance_asks_for_the_next_frame() {
    // Regression test: the control drawn open before the menu closed kept its open look.
    let mut harness = Harness::new();
    let limit = harness.styles.input.menu_close_distance;
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    harness.frame();
    let swap = harness.row(0, 1, &items());
    harness.input.pointer_moved(swap);
    harness.frame();
    let panel = harness.menu.levels[0].rect;
    harness
        .input
        .pointer_moved(Vec2::new(panel.max.x + limit + 2.0, swap.y));
    harness.frame();
    assert!(!harness.menu.is_open());
    assert_eq!(harness.state.redraw_at(), Some(harness.input.time()));
}
