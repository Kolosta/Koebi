//! Tests of how menus open and lay their rows out: highlights inside rows, and menus opened at
//! the pointer.

use super::*;

#[test]
fn items_lie_against_each_other_with_their_highlights_apart() {
    let plain = [
        MenuItem::action("One"),
        MenuItem::action("Two"),
        MenuItem::action("Three"),
    ];
    let mut harness = Harness::new();
    harness.menu.open_at(Vec2::new(10.0, 10.0));
    harness.frame_with(&plain);
    let style = harness.styles.menu;
    let panel = harness.menu.levels[0].rect;
    let first = harness.row(0, 0, &plain);
    let second = harness.row(0, 1, &plain);
    assert_eq!(second.y - first.y, style.item_height);
    // The padding counts to the highlight of the first item, half a gap inside its row.
    let highlight_top = first.y - style.item_height * 0.5 + style.item_gap * 0.5;
    assert_eq!(
        highlight_top - panel.min.y,
        style.border_width + style.padding_vertical
    );
    // The pointer selects an item up to the panel's sides, on the margin too.
    let at = Vec2::new(panel.min.x + style.border_width + 0.5, first.y);
    harness.input.pointer_moved(at);
    harness.frame_with(&plain);
    assert_eq!(harness.menu.levels[0].highlighted, Some(0));
    // And across the gap between two highlights.
    let gap = Vec2::new(panel.center().x, second.y - style.item_height * 0.5 + 0.5);
    harness.input.pointer_moved(gap);
    harness.frame_with(&plain);
    assert_eq!(harness.menu.levels[0].highlighted, Some(1));
}

#[test]
fn a_menu_opened_at_the_pointer_lies_over_its_first_item_towards_its_end() {
    let mut harness = Harness::new();
    let pointer = Vec2::new(200.0, 150.0);
    harness.input.pointer_moved(pointer);
    harness.frame();
    harness.menu.open_at_pointer(pointer);
    harness.frame();
    let style = harness.styles.menu;
    let panel = harness.menu.levels[0].rect;
    // Five sixths along the panel, in the middle of the first item that can be chosen (the
    // first row is a title), which is highlighted.
    let along = (pointer.x - panel.min.x) / panel.width();
    assert!((along - style.open_pointer_share).abs() < 0.02, "{along}");
    let row = harness.row(0, 1, &items());
    assert!((row.y - pointer.y).abs() <= 1.0, "{row:?}");
    assert_eq!(harness.menu.levels[0].highlighted, Some(1));
    // The pointer that has not moved does not take the highlight over.
    harness.frame();
    assert_eq!(harness.menu.levels[0].highlighted, Some(1));
    assert!(harness.menu.is_open());
}

#[test]
fn a_menu_opened_at_the_pointer_lies_over_its_last_choice() {
    let mut harness = Harness::new();
    let pointer = Vec2::new(200.0, 150.0);
    harness.input.pointer_moved(pointer);
    harness.frame();
    harness.menu.open_at_pointer(pointer);
    harness.frame();
    // Choose the item past the separator.
    let chosen = harness.row(0, 4, &items());
    harness.input.pointer_moved(chosen);
    harness.frame();
    harness.frame();
    harness.click(chosen);
    let chosen = harness.frame();
    assert_eq!(chosen.map(|choice| choice.index()), Some(4));
    assert!(!harness.menu.is_open());
    // Opened again elsewhere: the pointer lies over that item.
    let pointer = Vec2::new(250.0, 180.0);
    harness.input.pointer_moved(pointer);
    harness.frame();
    harness.menu.open_at_pointer(pointer);
    harness.frame();
    let row = harness.row(0, 4, &items());
    assert!((row.y - pointer.y).abs() <= 1.0, "{row:?}");
    assert_eq!(harness.menu.levels[0].highlighted, Some(4));
}

#[test]
fn a_menu_opened_at_the_pointer_stays_inside_its_bounds() {
    let mut harness = Harness::new();
    let bounds = bounds();
    for pointer in [
        Vec2::new(2.0, 2.0),
        Vec2::new(bounds.max.x - 1.0, 2.0),
        Vec2::new(2.0, bounds.max.y - 1.0),
        Vec2::new(bounds.max.x - 1.0, bounds.max.y - 1.0),
    ] {
        harness.menu.open_at_pointer(pointer);
        harness.frame();
        let panel = harness.menu.levels[0].rect;
        assert!(
            panel.min.x >= bounds.min.x
                && panel.min.y >= bounds.min.y
                && panel.max.x <= bounds.max.x
                && panel.max.y <= bounds.max.y,
            "{pointer:?}: {panel:?}"
        );
        harness.menu.close();
    }
}

#[test]
fn a_long_menu_opened_at_the_pointer_scrolls_under_a_still_panel() {
    // Regression test: the panel followed the scroll, so that the rows under the pointer did not
    // change until it reached the bounds.
    let labels: Vec<String> = (0..30).map(|index| format!("Item {index}")).collect();
    let long: Vec<MenuItem<'_>> = labels.iter().map(|label| MenuItem::action(label)).collect();
    let mut harness = Harness::new();
    harness.styles.menu.max_height = 120.0;
    let pointer = Vec2::new(200.0, 150.0);
    harness.input.pointer_moved(pointer);
    harness.frame_with(&long);
    harness.menu.open_at_pointer(pointer);
    harness.frame_with(&long);
    let panel = harness.menu.levels[0].rect;
    harness.input.scrolled(Vec2::new(0.0, -1.0));
    harness.frame_with(&long);
    harness.frame_with(&long);
    assert!(harness.menu.levels[0].scroll > 0.0);
    assert_eq!(harness.menu.levels[0].rect, panel);
}

#[test]
fn a_last_choice_that_cannot_be_chosen_any_more_gives_way_to_the_first_item() {
    let mut menu = MenuState::new();
    menu.last_choice = Some(1);
    let items = [
        MenuItem::Title("Title"),
        MenuItem::action("Off").enabled(false),
        MenuItem::action("On"),
    ];
    assert_eq!(menu.item_under_pointer(&items), 2);
    menu.last_choice = Some(9);
    assert_eq!(menu.item_under_pointer(&items), 2);
    menu.last_choice = Some(2);
    assert_eq!(menu.item_under_pointer(&items), 2);
}
