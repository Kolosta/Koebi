//! Tests of lists: layout, folding and its animation, selection, keyboard, drop rules and
//! dragging.

use core::time::Duration;

use koebi_core::input::{CursorShape, Modifiers};
use koebi_core::{Rect, Vec2};
use koebi_render::Scene;
use koebi_text::Fonts;

use super::layout::{Row, decimal};
use super::*;
use crate::{Input, PointerButton, Styles, UiState};

const LIST_ID: WidgetId = WidgetId::new("outliner");

/// The rows' height: one interface unit.
const ROW: f32 = 20.0;

/// A scene tree: a parent with a folding child, and a second top-level row.
fn items() -> Vec<ListItem<'static>> {
    vec![
        ListItem::new(1, "Scene"),
        ListItem::new(2, "Camera").with_depth(1),
        ListItem::new(3, "Lights").with_depth(1),
        ListItem::new(4, "Sun").with_depth(2),
        ListItem::new(5, "Fill").with_depth(2),
        ListItem::new(6, "Cube").with_depth(1),
        ListItem::new(7, "Notes"),
    ]
}

/// The primary modifier of the platform the tests run on.
fn primary() -> Modifiers {
    if cfg!(any(target_os = "macos", target_os = "ios")) {
        Modifiers::LEFT_GUI
    } else {
        Modifiers::LEFT_CTRL
    }
}

fn style() -> ListStyle {
    crate::test_styles().list
}

/// Lays `items` out at `now` (milliseconds).
fn lay_out(state: &mut ListState, items: &[ListItem<'_>], tree: bool, now: Duration) -> bool {
    state.lay_out(items, tree, ROW, now, &style())
}

fn ids(state: &ListState) -> Vec<u64> {
    state.rows.iter().map(|row| row.id).collect()
}

fn tops(state: &ListState) -> Vec<f32> {
    state.rows.iter().map(|row| row.top).collect()
}

fn row_of(state: &ListState, id: u64) -> Row {
    *state.rows.iter().find(|row| row.id == id).unwrap()
}

/// Accepts rows into `containers` only (every row when empty); draws a toggle one row wide at
/// the end of each row and records the rows whose toggle was clicked.
#[derive(Default)]
struct Rules {
    containers: Vec<u64>,
    toggled: Vec<u64>,
}

impl ListDelegate for Rules {
    fn can_drop(&self, _dragged: &[u64], place: DropPlace) -> bool {
        match place {
            DropPlace::Into(id) => self.containers.is_empty() || self.containers.contains(&id),
            DropPlace::Before(_) | DropPlace::After(_) => true,
        }
    }

    fn row_end(&mut self, ui: &mut Ui<'_>, item: &ListItem<'_>, rect: Rect) -> f32 {
        let toggle = Rect::from_min_max(Vec2::new(rect.max.x - ROW, rect.min.y), rect.max);
        if ui
            .interact(WidgetId::new("visible").with(item.id), toggle)
            .clicked
        {
            self.toggled.push(item.id);
        }
        ROW
    }
}

struct Harness {
    state: UiState,
    list: ListState,
    input: Input,
    styles: Styles,
    items: Vec<ListItem<'static>>,
    options: ListOptions,
    rules: Rules,
    rect: Rect,
    scene: Scene,
}

impl Harness {
    /// A list showing every row of [`items`] (seven rows, 140 high).
    fn new(options: ListOptions) -> Self {
        Self::with_height(options, 160.0)
    }

    fn with_height(options: ListOptions, height: f32) -> Self {
        let mut harness = Self {
            state: UiState::new(),
            list: ListState::new(),
            input: Input::new(),
            styles: crate::test_styles(),
            items: items(),
            options,
            rules: Rules::default(),
            rect: Rect::from_origin_size(Vec2::ZERO, Vec2::new(200.0, height)),
            scene: Scene::new(),
        };
        assert_eq!(harness.styles.list.row_height, ROW);
        harness.frame();
        harness
    }

    fn frame(&mut self) -> ListResponse {
        self.scene.clear();
        let response = {
            let mut ui = Ui::new(&mut self.scene, &self.styles, &self.input, &mut self.state);
            ui.list(
                LIST_ID,
                self.rect,
                &mut self.list,
                &self.items,
                &self.options,
                &mut self.rules,
            )
        };
        self.input.end_frame();
        response
    }

    /// A click at `at`; returns the response of the frame that saw it.
    fn click(&mut self, at: Vec2) -> ListResponse {
        self.input.pointer_moved(at);
        self.frame();
        self.input.primary_button(true, at);
        self.input.primary_button(false, at);
        self.frame()
    }

    /// Lets every fold animation finish: the next frame starts them, a later one ends them.
    fn settle(&mut self) {
        self.frame();
        self.input
            .set_time(self.input.time() + Duration::from_secs(10));
        self.frame();
    }

    /// The middle of the row at `position`, right of its label.
    fn row(position: u16) -> Vec2 {
        Vec2::new(150.0, ROW * f32::from(position) + ROW * 0.5)
    }

    /// The middle of the fold arrow of the row at `position`, `depth` deep.
    fn arrow(&self, position: u16, depth: u16) -> Vec2 {
        let style = self.styles.list;
        let x = style.padding + style.indent * (f32::from(depth) + 0.5);
        Vec2::new(x, ROW * f32::from(position) + ROW * 0.5)
    }

    fn selected(&self) -> Vec<u64> {
        let mut selected: Vec<u64> = self.list.selected().collect();
        selected.sort_unstable();
        selected
    }

    /// Presses the row at `from` and moves the pointer to `to`, past the drag threshold.
    fn drag_to(&mut self, from: Vec2, to: Vec2) {
        self.input.pointer_moved(from);
        self.frame();
        self.input.primary_button(true, from);
        self.frame();
        self.input.pointer_moved(to);
        self.frame();
    }
}

#[test]
fn rows_follow_the_items_one_row_apart() {
    let mut state = ListState::new();
    let items = items();
    assert!(!lay_out(&mut state, &items, true, Duration::ZERO));
    assert_eq!(ids(&state), [1, 2, 3, 4, 5, 6, 7]);
    assert_eq!(tops(&state), [0.0, 20.0, 40.0, 60.0, 80.0, 100.0, 120.0]);
    assert_eq!(state.height, 140.0);
    let parents: Vec<u64> = state
        .rows
        .iter()
        .filter(|row| row.parent)
        .map(|row| row.id)
        .collect();
    assert_eq!(parents, [1, 3]);
}

#[test]
fn folded_rows_hide_their_subtree_and_count_it() {
    let mut state = ListState::new();
    let items = items();
    state.set_folded(3, true);
    lay_out(&mut state, &items, true, Duration::ZERO);
    assert_eq!(ids(&state), [1, 2, 3, 6, 7]);
    assert_eq!(row_of(&state, 3).hidden, 2);
    assert_eq!(state.height, 100.0);
    // Folding the parent hides everything below it, the folded child included.
    state.set_folded(1, true);
    lay_out(&mut state, &items, true, Duration::ZERO);
    assert_eq!(ids(&state), [1, 7]);
    assert_eq!(row_of(&state, 1).hidden, 5);
    assert!(state.is_folded(3));
}

#[test]
fn flat_lists_ignore_depths() {
    let mut state = ListState::new();
    let items = items();
    state.set_folded(1, true);
    lay_out(&mut state, &items, false, Duration::ZERO);
    assert_eq!(ids(&state), [1, 2, 3, 4, 5, 6, 7]);
    assert!(state.rows.iter().all(|row| !row.parent && row.hidden == 0));
}

#[test]
fn opening_folds_slide_their_rows_in() {
    let mut state = ListState::new();
    let items = items();
    state.set_folded(3, true);
    lay_out(&mut state, &items, true, Duration::ZERO);
    state.fold_animated(3, false);
    // The animation starts with the first layout after the change.
    let start = Duration::from_secs(1);
    assert!(lay_out(&mut state, &items, true, start));
    assert_eq!(ids(&state), [1, 2, 3, 6, 7]);
    assert!(lay_out(
        &mut state,
        &items,
        true,
        start + style().expand / 2
    ));
    assert_eq!(ids(&state), [1, 2, 3, 4, 5, 6, 7]);
    // Half the time, most of the way: the children show down to that line, cut there.
    let openness = 1.0 - 0.5_f32.powi(3);
    let line = 60.0 + 2.0 * ROW * openness;
    let sun = row_of(&state, 4);
    assert_eq!(sun.top, 60.0);
    assert!((sun.clip - line).abs() < 0.01, "{sun:?}");
    assert!((row_of(&state, 6).top - line).abs() < 0.01);
    assert!(!sun.leaving);
    assert!((row_of(&state, 3).openness - openness).abs() < 0.01);
    // Done: back to the plain layout, and no more frames needed.
    assert!(!lay_out(&mut state, &items, true, start + style().expand));
    assert_eq!(tops(&state), [0.0, 20.0, 40.0, 60.0, 80.0, 100.0, 120.0]);
    assert!(state.rows.iter().all(|row| row.clip == f32::INFINITY));
}

#[test]
fn closing_folds_keep_their_rows_until_they_are_gone() {
    let mut state = ListState::new();
    let items = items();
    lay_out(&mut state, &items, true, Duration::ZERO);
    state.fold_animated(3, true);
    let start = Duration::from_secs(1);
    lay_out(&mut state, &items, true, start);
    // From the start, the closing rows no longer count as visible.
    let visible: Vec<u64> = state.visible().map(|row| row.id).collect();
    assert_eq!(visible, [1, 2, 3, 6, 7]);
    assert_eq!(row_of(&state, 3).hidden, 2);
    lay_out(&mut state, &items, true, start + style().collapse / 2);
    let line = 60.0 + 2.0 * ROW * 0.5_f32.powi(3);
    let fill = row_of(&state, 5);
    assert!(fill.leaving);
    assert!((fill.clip - line).abs() < 0.01);
    assert!((row_of(&state, 6).top - line).abs() < 0.01);
    lay_out(&mut state, &items, true, start + style().collapse);
    assert_eq!(ids(&state), [1, 2, 3, 6, 7]);
}

#[test]
fn folds_inside_animating_folds_are_cut_by_both() {
    let mut state = ListState::new();
    let items = items();
    state.set_folded(3, true);
    lay_out(&mut state, &items, true, Duration::ZERO);
    // Lights opens while its parent closes.
    state.fold_animated(3, false);
    state.fold_animated(1, true);
    let start = Duration::from_secs(1);
    lay_out(&mut state, &items, true, start);
    lay_out(&mut state, &items, true, start + Duration::from_millis(50));
    let openness = |id: u64| {
        state
            .folds
            .iter()
            .find(|fold| fold.id == id)
            .unwrap()
            .current
    };
    let (scene, lights) = (openness(1), openness(3));
    assert!(scene > 0.0 && scene < 1.0 && lights > 0.0 && lights < 1.0);
    // Lights' rows end where Lights' fold has come to, inside Scene's fold, which ends
    // after its rows (Cube included) at the share it has come to.
    let lights_bottom = 60.0 + 2.0 * ROW * lights;
    let scene_bottom = 20.0 + (lights_bottom + ROW - 20.0) * scene;
    let sun = row_of(&state, 4);
    assert!((sun.clip - lights_bottom.min(scene_bottom)).abs() < 0.01);
    assert!(sun.leaving);
    assert!((row_of(&state, 7).top - scene_bottom).abs() < 0.01);
}

#[test]
fn a_click_selects_a_row_and_makes_it_active() {
    let mut harness = Harness::new(ListOptions::TREE);
    let response = harness.click(Harness::row(2));
    assert!(response.selection_changed);
    assert_eq!(harness.selected(), [3]);
    assert_eq!(harness.list.active(), Some(3));
    harness.click(Harness::row(1));
    assert_eq!(harness.selected(), [2]);
    assert_eq!(harness.list.active(), Some(2));
}

#[test]
fn ctrl_adds_and_removes_rows_and_shift_selects_a_range() {
    let mut harness = Harness::new(ListOptions::TREE);
    harness.click(Harness::row(1));
    harness.input.set_modifiers(Modifiers::LEFT_SHIFT);
    harness.click(Harness::row(5));
    assert_eq!(harness.selected(), [2, 3, 4, 5, 6]);
    assert_eq!(harness.list.active(), Some(6));
    harness.input.set_modifiers(primary());
    harness.click(Harness::row(3));
    assert_eq!(harness.selected(), [2, 3, 5, 6]);
    harness.click(Harness::row(6));
    assert_eq!(harness.selected(), [2, 3, 5, 6, 7]);
    assert_eq!(harness.list.active(), Some(7));
    harness.input.set_modifiers(Modifiers::NONE);
    harness.click(Harness::row(0));
    assert_eq!(harness.selected(), [1]);
}

#[test]
fn a_click_on_a_row_of_the_selection_selects_it_alone() {
    let mut harness = Harness::new(ListOptions::TREE);
    harness.click(Harness::row(1));
    harness.input.set_modifiers(primary());
    harness.click(Harness::row(2));
    harness.input.set_modifiers(Modifiers::NONE);
    // The press keeps the selection (it could start a drag); the click selects the row.
    harness.input.pointer_moved(Harness::row(2));
    harness.frame();
    harness.input.primary_button(true, Harness::row(2));
    harness.frame();
    assert_eq!(harness.selected(), [2, 3]);
    harness.input.primary_button(false, Harness::row(2));
    harness.frame();
    assert_eq!(harness.selected(), [3]);
}

#[test]
fn a_click_on_the_empty_space_clears_the_selection() {
    let mut harness = Harness::new(ListOptions::TREE);
    harness.list.set_folded(1, true);
    harness.click(Harness::row(0));
    assert_eq!(harness.selected(), [1]);
    let response = harness.click(Vec2::new(150.0, 100.0));
    assert!(response.selection_changed);
    assert_eq!(harness.selected(), Vec::<u64>::new());
    assert_eq!(harness.list.active(), None);
}

#[test]
fn the_fold_arrow_folds_without_selecting() {
    let mut harness = Harness::new(ListOptions::TREE);
    harness.click(harness.arrow(2, 1));
    assert!(harness.list.is_folded(3));
    assert_eq!(harness.selected(), Vec::<u64>::new());
    // Folds animate: the next frames are asked for until they are done.
    assert!(harness.state.redraw_at().is_some());
    harness.settle();
    assert_eq!(ids(&harness.list), [1, 2, 3, 6, 7]);
    assert_eq!(harness.state.redraw_at(), None);
    // Shift folds and unfolds the whole subtree.
    harness.input.set_modifiers(Modifiers::LEFT_SHIFT);
    harness.click(harness.arrow(2, 1));
    harness.settle();
    assert!(!harness.list.is_folded(3));
    harness.click(harness.arrow(0, 0));
    harness.settle();
    assert_eq!(ids(&harness.list), [1, 7]);
    assert!(harness.list.is_folded(3));
    harness.click(harness.arrow(0, 0));
    harness.settle();
    assert_eq!(ids(&harness.list), [1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn items_at_the_end_of_rows_take_their_clicks() {
    let mut harness = Harness::new(ListOptions::TREE);
    let style = harness.styles.list;
    let end = 200.0 - harness.styles.scroll.margin - style.padding;
    harness.click(Vec2::new(end - ROW * 0.5, ROW * 2.5));
    assert_eq!(harness.rules.toggled, [3]);
    assert_eq!(harness.selected(), Vec::<u64>::new());
}

#[test]
fn two_quick_clicks_are_a_double_click() {
    let mut harness = Harness::new(ListOptions::TREE);
    let double = harness.styles.list.double_click;
    harness.input.set_time(Duration::from_secs(1));
    assert_eq!(harness.click(Harness::row(1)).double_clicked, None);
    harness.input.set_time(Duration::from_secs(1) + double / 2);
    assert_eq!(harness.click(Harness::row(1)).double_clicked, Some(2));
    // A third click starts over, and so does a slow second click.
    harness.input.set_time(Duration::from_secs(1) + double);
    assert_eq!(harness.click(Harness::row(1)).double_clicked, None);
    harness.input.set_time(Duration::from_secs(5));
    assert_eq!(harness.click(Harness::row(1)).double_clicked, None);
}

#[test]
fn a_secondary_press_selects_the_row_and_asks_for_a_menu() {
    let mut harness = Harness::new(ListOptions::TREE);
    let at = Harness::row(2);
    harness.input.pointer_moved(at);
    harness.frame();
    harness.input.button(PointerButton::Secondary, true, at);
    let response = harness.frame();
    assert_eq!(
        response.context_menu,
        Some(ListContextMenu { at, row: Some(3) })
    );
    assert_eq!(harness.selected(), [3]);
    harness.input.button(PointerButton::Secondary, false, at);
    harness.frame();
    // On the empty space: a menu for the list itself.
    harness.list.set_folded(1, true);
    let empty = Vec2::new(150.0, 120.0);
    harness.input.pointer_moved(empty);
    harness.frame();
    harness.input.button(PointerButton::Secondary, true, empty);
    let response = harness.frame();
    assert_eq!(
        response.context_menu,
        Some(ListContextMenu {
            at: empty,
            row: None
        })
    );
}

#[test]
fn the_keyboard_walks_rows_folds_and_selects_all() {
    let mut harness = Harness::new(ListOptions::TREE);
    harness.input.pointer_moved(Harness::row(0));
    harness.frame();
    let press = |harness: &mut Harness, action: &'static ActionDef| {
        harness.input.trigger(action);
        harness.frame();
    };
    // From no active row, the first one; then down to Lights.
    for _ in 0..3 {
        press(&mut harness, &LIST_NEXT);
    }
    assert_eq!(harness.list.active(), Some(3));
    assert_eq!(harness.selected(), [3]);
    press(&mut harness, &LIST_FOLD);
    harness.settle();
    assert!(harness.list.is_folded(3));
    // Folded: Left goes to the parent; Right goes into an unfolded row.
    press(&mut harness, &LIST_FOLD);
    assert_eq!(harness.list.active(), Some(1));
    press(&mut harness, &LIST_UNFOLD);
    assert_eq!(harness.list.active(), Some(2));
    press(&mut harness, &LIST_EXTEND_NEXT);
    assert_eq!(harness.selected(), [2, 3]);
    press(&mut harness, &LIST_NEXT);
    press(&mut harness, &LIST_PREVIOUS);
    assert_eq!(harness.list.active(), Some(3));
    press(&mut harness, &LIST_UNFOLD);
    harness.settle();
    assert!(!harness.list.is_folded(3));
    press(&mut harness, &LIST_SELECT_ALL);
    assert_eq!(harness.selected(), [1, 2, 3, 4, 5, 6, 7]);
    // Away from the list, its keys do nothing.
    harness.input.pointer_moved(Vec2::new(500.0, 50.0));
    harness.frame();
    press(&mut harness, &LIST_PREVIOUS);
    assert_eq!(harness.list.active(), Some(3));
}

#[test]
fn the_keyboard_scrolls_to_the_active_row() {
    let mut harness = Harness::with_height(ListOptions::TREE, 100.0);
    harness.input.pointer_moved(Harness::row(0));
    harness.frame();
    // From no active row, Up goes to the last one, below the view.
    harness.input.trigger(&LIST_PREVIOUS);
    harness.frame();
    assert_eq!(harness.list.active(), Some(7));
    assert_eq!(harness.list.scroll().offset().y, 40.0);
}

/// Where rows dragged in `state` would go for the pointer at `at` in the content.
fn target(state: &ListState, options: &ListOptions, rules: &Rules, at: Vec2) -> Option<DropPlace> {
    let metrics = Metrics {
        row_height: ROW,
        indent: 16.0,
        start: 0.0,
    };
    state
        .drop_target(&items(), options, rules, at, metrics)
        .map(|(place, _)| place)
}

/// A tree laid out with every row shown, dragging `dragged`.
fn dragging(dragged: u64) -> ListState {
    let mut state = ListState::new();
    lay_out(&mut state, &items(), true, Duration::ZERO);
    state.collect_dragged(&items(), true, dragged, false);
    state
}

#[test]
fn the_pointer_picks_before_into_or_after_a_row() {
    let (state, rules) = (dragging(7), Rules::default());
    let tree = ListOptions::TREE;
    // Camera spans 20 to 40: its top quarter, middle half and bottom quarter.
    let at = |y: f32| target(&state, &tree, &rules, Vec2::new(100.0, y));
    assert_eq!(at(22.0), Some(DropPlace::Before(2)));
    assert_eq!(at(30.0), Some(DropPlace::Into(2)));
    assert_eq!(at(37.0), Some(DropPlace::After(2)));
    // Above the first row, and below the last.
    assert_eq!(at(-5.0), Some(DropPlace::Before(1)));
    let state = dragging(2);
    let at = |y: f32| target(&state, &tree, &rules, Vec2::new(100.0, y));
    assert_eq!(at(500.0), Some(DropPlace::After(7)));
}

#[test]
fn below_an_unfolded_parent_rows_go_first_inside() {
    let (mut state, rules) = (dragging(7), Rules::default());
    let tree = ListOptions::TREE;
    let below_lights = Vec2::new(100.0, 57.0);
    assert_eq!(
        target(&state, &tree, &rules, below_lights),
        Some(DropPlace::Before(4))
    );
    state.set_folded(3, true);
    lay_out(&mut state, &items(), true, Duration::ZERO);
    assert_eq!(
        target(&state, &tree, &rules, below_lights),
        Some(DropPlace::After(3))
    );
}

#[test]
fn below_the_last_child_the_indentation_picks_the_level() {
    let (state, rules) = (dragging(7), Rules::default());
    let tree = ListOptions::TREE;
    // Below Fill (depth 2, the last child of Lights) and above Cube (depth 1).
    let below_fill = |x: f32| target(&state, &tree, &rules, Vec2::new(x, 97.0));
    assert_eq!(below_fill(40.0), Some(DropPlace::After(5)));
    assert_eq!(below_fill(20.0), Some(DropPlace::After(3)));
    assert_eq!(below_fill(5.0), Some(DropPlace::After(3)));
    // Below Cube, the last child of Scene, above the top-level Notes.
    let below_cube = |x: f32| target(&state, &tree, &rules, Vec2::new(x, 117.0));
    assert_eq!(below_cube(20.0), Some(DropPlace::After(6)));
    assert_eq!(below_cube(5.0), Some(DropPlace::After(1)));
}

#[test]
fn rows_never_go_inside_themselves() {
    let (state, rules) = (dragging(3), Rules::default());
    assert_eq!(state.dragged(), [3]);
    let tree = ListOptions::TREE;
    let at = |x: f32, y: f32| target(&state, &tree, &rules, Vec2::new(x, y));
    // Lights itself, its child Sun, and below its last child: nowhere.
    assert_eq!(at(100.0, 50.0), None);
    assert_eq!(at(100.0, 70.0), None);
    assert_eq!(at(5.0, 97.0), None);
    // Elsewhere, anywhere.
    assert_eq!(at(100.0, 30.0), Some(DropPlace::Into(2)));
    assert_eq!(at(100.0, 102.0), Some(DropPlace::Before(6)));
}

#[test]
fn the_delegate_decides_where_rows_may_go() {
    let state = dragging(7);
    let rules = Rules {
        containers: vec![1],
        ..Rules::default()
    };
    let tree = ListOptions::TREE;
    let at = |y: f32| target(&state, &tree, &rules, Vec2::new(100.0, y));
    // Camera takes no children: its halves are before and after it.
    assert_eq!(at(26.0), Some(DropPlace::Before(2)));
    assert_eq!(at(30.0), Some(DropPlace::After(2)));
    assert_eq!(at(10.0), Some(DropPlace::Into(1)));
    // A flat list never drops into rows.
    let flat = ListOptions {
        drag: true,
        ..ListOptions::LIST
    };
    assert_eq!(
        target(&state, &flat, &Rules::default(), Vec2::new(100.0, 30.0)),
        Some(DropPlace::After(2))
    );
}

#[test]
fn dragging_a_row_drops_it_where_the_preview_shows() {
    let mut harness = Harness::new(ListOptions::TREE);
    let into_lights = Vec2::new(150.0, 50.0);
    harness.drag_to(Harness::row(1), into_lights);
    assert!(harness.list.is_dragging());
    assert_eq!(harness.state.cursor(), CursorShape::Move);
    // Dragging an unselected row moves it alone and leaves the selection as it is.
    assert_eq!(harness.selected(), Vec::<u64>::new());
    // The drop line or frame, and the label near the pointer, are drawn.
    let mut chain = Vec::new();
    harness.state.chain_at(None, &mut chain);
    assert_eq!(chain, [LIST_DRAG]);
    harness.input.primary_button(false, into_lights);
    let response = harness.frame();
    assert_eq!(response.dropped, Some(DropPlace::Into(3)));
    assert_eq!(harness.list.dragged(), [2]);
    // A drop is no click: the selection stays.
    assert!(!response.selection_changed);
    assert!(!harness.list.is_dragging());
    assert_eq!(harness.list.drag, None);
}

#[test]
fn escape_cancels_a_drag_and_the_release_moves_nothing() {
    let mut harness = Harness::new(ListOptions::TREE);
    let into_lights = Vec2::new(150.0, 50.0);
    harness.drag_to(Harness::row(1), into_lights);
    harness.input.trigger(&LIST_DRAG_CANCEL);
    harness.frame();
    assert!(!harness.list.is_dragging());
    harness.input.primary_button(false, into_lights);
    let response = harness.frame();
    assert_eq!(response.dropped, None);
    // The release ended a drag: it is no click.
    assert_eq!(harness.selected(), Vec::<u64>::new());
}

#[test]
fn dragging_a_selection_carries_its_topmost_rows() {
    let mut harness = Harness::new(ListOptions::TREE);
    harness.click(Harness::row(1));
    harness.input.set_modifiers(primary());
    harness.click(Harness::row(2));
    harness.click(Harness::row(3));
    harness.input.set_modifiers(Modifiers::NONE);
    assert_eq!(harness.selected(), [2, 3, 4]);
    let into_notes = Vec2::new(150.0, 130.0);
    harness.drag_to(Harness::row(2), into_notes);
    // Sun moves with Lights: it is not dragged on its own.
    assert_eq!(harness.list.dragged(), [2, 3]);
    harness.input.primary_button(false, into_notes);
    assert_eq!(harness.frame().dropped, Some(DropPlace::Into(7)));
    assert_eq!(harness.selected(), [2, 3, 4]);
}

#[test]
fn dragging_against_the_bottom_edge_scrolls() {
    let mut harness = Harness::with_height(ListOptions::TREE, 100.0);
    let edge = Vec2::new(150.0, 99.0);
    harness.drag_to(Harness::row(1), edge);
    assert_eq!(harness.list.scroll().offset().y, 0.0);
    harness.input.set_time(Duration::from_millis(100));
    harness.frame();
    // 0.95 of the speed at the edge, for a tenth of a second.
    let expected = 0.95 * AUTO_SCROLL_ROWS_PER_SECOND * ROW * 0.1;
    assert!((harness.list.scroll().offset().y - expected).abs() < 0.01);
    assert!(harness.state.redraw_at().is_some());
    harness.input.set_time(Duration::from_secs(2));
    harness.frame();
    assert_eq!(harness.list.scroll().offset().y, 40.0);
}

#[test]
fn folded_rows_show_how_many_rows_they_hide() {
    let glyphs = |counts: bool| {
        let mut harness = Harness::new(ListOptions {
            counts,
            ..ListOptions::TREE
        });
        harness.state.set_fonts(Fonts::with_embedded());
        harness.list.set_folded(1, true);
        harness.frame();
        harness.scene.glyph_count()
    };
    // "5" after Scene: the five rows below it.
    assert_eq!(glyphs(true), glyphs(false) + 1);
}

#[test]
fn decimal_writes_counts() {
    let mut digits = [0; 20];
    assert_eq!(decimal(0, &mut digits), "0");
    assert_eq!(decimal(1234, &mut digits), "1234");
    assert_eq!(decimal(usize::MAX, &mut digits), usize::MAX.to_string());
}

#[test]
fn retain_forgets_rows_that_went_away() {
    let mut state = ListState::new();
    state.set_folded(3, true);
    state.set_active(Some(4));
    state.set_selected(5, true);
    state.retain(|id| id != 4 && id != 3);
    assert!(!state.is_folded(3));
    assert_eq!(state.active(), None);
    assert_eq!(state.selected().collect::<Vec<_>>(), [5]);
}
