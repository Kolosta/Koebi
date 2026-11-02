//! Tests of docking: dragging tabs and top bars, the tab bar of a lone editor, and the preview
//! of where an editor would go.

use super::*;

/// On the right area's top bar.
const RIGHT_TOP_BAR: Vec2 = Vec2::new(600.0, 20.0);

impl Harness {
    /// Presses the left top bar and drags it to `to`, without releasing.
    fn hold_top_bar(&mut self, to: Vec2) {
        self.input.pointer_moved(LEFT_TOP_BAR);
        self.frame();
        self.input.primary_button(true, LEFT_TOP_BAR);
        self.frame();
        self.input.pointer_moved(to);
        self.frame();
    }

    /// Draws a frame at `millis` milliseconds.
    fn frame_at(&mut self, millis: u64) {
        self.input.set_time(Duration::from_millis(millis));
        self.frame();
    }

    /// Draws frames every 16 milliseconds from `from` to `to`.
    fn frames_between(&mut self, from: u64, to: u64) {
        let mut at = from;
        while at <= to {
            self.frame_at(at);
            at += 16;
        }
    }

    /// How far the tab bar of a lone editor has opened.
    fn opening(&self) -> f32 {
        self.view.reveal.map_or(0.0, |reveal| reveal.amount)
    }
}

#[test]
fn a_drag_from_a_top_bar_does_not_open_its_tab_bar_at_once() {
    let mut harness = Harness::halves();
    // Past the threshold, still over the bar it started from.
    harness.hold_top_bar(LEFT_TOP_BAR + Vec2::new(20.0, 0.0));
    assert!(matches!(
        harness.view.operation,
        Some(Operation::Dock { started: true, .. })
    ));
    assert_eq!(harness.opening(), 0.0);
    assert_eq!(harness.tab_rects(), []);
}

#[test]
fn a_lone_editors_tab_bar_opens_when_a_drag_waits_over_its_top() {
    let mut harness = Harness::halves();
    let style = harness.styles.workspace;
    harness.hold_top_bar(RIGHT_TOP_BAR);
    assert_eq!(harness.tab_rects(), []);
    // The frame asks to be drawn again when the delay ends: no input comes meanwhile.
    assert_eq!(harness.state.redraw_at(), Some(style.reveal_delay));
    harness.frame_at(200);
    assert_eq!(harness.tab_rects(), []);
    // After the delay it slides in, and each frame asks for the next.
    harness.frame_at(350);
    assert!(harness.opening() < 0.5);
    harness.frame_at(366);
    assert!(harness.opening() > 0.0 && harness.opening() < 1.0);
    assert!(harness.state.redraw_at().is_some());
    harness.frames_between(382, 900);
    assert_eq!(harness.opening(), 1.0);
    let right = harness.area_ids()[1];
    let content = harness
        .view
        .parts
        .iter()
        .find(|part| part.area == right)
        .unwrap()
        .content;
    assert_eq!(
        content.min.y,
        4.0 + style.tab_bar_height + style.top_bar_height
    );
    // Over the bar, the drop goes among its tabs.
    let target = harness.view.dock_target;
    harness.frame_at(916);
    assert!(
        matches!(
            harness.view.dock_target,
            Some(DockTarget::Tabs { in_bar: true, .. })
        ),
        "{target:?}"
    );
    assert_eq!(harness.state.redraw_at(), None);
}

#[test]
fn a_lone_editors_tab_bar_closes_after_the_drag_leaves() {
    let mut harness = Harness::halves();
    let style = harness.styles.workspace;
    harness.hold_top_bar(RIGHT_TOP_BAR);
    harness.frames_between(350, 900);
    assert_eq!(harness.opening(), 1.0);
    // Away from the top: it stays open for the grace, then closes.
    harness.input.pointer_moved(Vec2::new(600.0, 320.0));
    harness.frame_at(1000);
    assert_eq!(
        harness.state.redraw_at(),
        Some(Duration::from_millis(1000) + style.reveal_grace)
    );
    harness.frame_at(1100);
    assert_eq!(harness.opening(), 1.0);
    harness.frames_between(1250, 1800);
    assert_eq!(harness.opening(), 0.0);
    assert!(harness.view.reveal.is_none());
    assert_eq!(harness.tab_rects(), []);
    assert_eq!(harness.state.redraw_at(), None);
}

#[test]
fn a_tab_bar_opening_closes_when_the_drag_is_released_elsewhere() {
    let mut harness = Harness::halves();
    harness.hold_top_bar(RIGHT_TOP_BAR);
    harness.frames_between(350, 900);
    let at = Vec2::new(790.0, 300.0);
    harness.input.pointer_moved(at);
    harness.frame_at(916);
    harness.input.primary_button(false, at);
    harness.frame_at(932);
    assert!(!harness.view.is_operating());
    // It keeps sliding out after the drop, without a grace.
    assert!(harness.opening() < 1.0);
    harness.frames_between(948, 1500);
    assert!(harness.view.reveal.is_none());
    assert_eq!(harness.state.redraw_at(), None);
}

#[test]
fn moving_between_two_lone_areas_closes_the_first_bar_before_opening_the_second() {
    // Regression test: the first bar shut in one frame, and its content jumped.
    let half = GRID / 2;
    let mut harness = Harness::tiled(&[
        (0, GridRect::new(0, 0, half, GRID)),
        (1, GridRect::new(half, 0, GRID, half)),
        (2, GridRect::new(half, half, GRID, GRID)),
    ]);
    let ids = harness.area_ids();
    harness.hold_top_bar(Vec2::new(600.0, 10.0));
    harness.frames_between(350, 900);
    assert_eq!(harness.opening(), 1.0);
    // Over the top of the other area, about 300 pixels down: the first bar slides out.
    harness.input.pointer_moved(Vec2::new(600.0, 310.0));
    harness.frame_at(916);
    let first = harness.view.reveal.unwrap();
    assert_eq!(first.area, ids[1]);
    assert!(first.amount > 0.0 && first.amount < 1.0, "{first:?}");
    assert!(harness.state.redraw_at().is_some());
    harness.frames_between(932, 1500);
    // Once it has closed, the second one waits its delay, asking for the frames it needs.
    let second = harness.view.reveal.unwrap();
    assert_eq!(second.area, ids[2]);
    harness.frames_between(1516, 2200);
    assert_eq!(harness.opening(), 1.0);
}

#[test]
fn a_tab_bar_opening_in_one_frame_still_updates_the_drop_target() {
    // Regression test: without a time constant, the last frame of the slide asked for no other,
    // and the target stayed the one from before the bar opened until the next input.
    let mut harness = Harness::halves();
    harness.styles.workspace.drop_glide = Duration::ZERO;
    harness.hold_top_bar(RIGHT_TOP_BAR);
    harness.frame_at(350);
    assert_eq!(harness.opening(), 1.0);
    assert!(harness.state.redraw_at().is_some());
    harness.frame_at(366);
    assert!(matches!(
        harness.view.dock_target,
        Some(DockTarget::Tabs { in_bar: true, .. })
    ));
    assert_eq!(harness.state.redraw_at(), None);
}

#[test]
fn a_drop_over_the_top_of_a_lone_editor_goes_beside_it_until_its_bar_opens() {
    let mut harness = Harness::halves();
    let right = harness.area_ids()[1];
    harness.hold_top_bar(RIGHT_TOP_BAR);
    harness.frame_at(16);
    assert_eq!(
        harness.view.dock_target,
        Some(DockTarget::Side {
            area: right,
            side: Side::Top
        })
    );
}

#[test]
fn the_drop_preview_glides_within_an_area_and_jumps_between_areas() {
    let (half, quarter) = (GRID / 2, GRID / 4);
    let mut harness = Harness::tiled(&[
        (0, GridRect::new(0, 0, half, GRID)),
        (1, GridRect::new(half, 0, GRID, quarter * 2)),
        (2, GridRect::new(half, quarter * 2, GRID, GRID)),
    ]);
    let glide = |harness: &Harness| harness.view.glide.unwrap();
    // The first preview shows at once: the right half of the area at the top right.
    harness.hold_top_bar(Vec2::new(795.0, 150.0));
    let first = glide(&harness).rect;
    assert!(first.min.x > 600.0, "{first:?}");
    // Into the middle of that area: it heads there without arriving, and asks for frames.
    harness.input.pointer_moved(Vec2::new(600.0, 150.0));
    harness.frame_at(16);
    harness.frame_at(32);
    let moving = glide(&harness).rect;
    let middle = harness.view.parts[1]
        .panel
        .inset(harness.styles.workspace.drop_center_inset);
    assert!(
        moving.min.x < first.min.x && moving.min.x > middle.min.x,
        "{moving:?}"
    );
    assert!(harness.state.redraw_at().is_some());
    harness.frames_between(48, 800);
    assert_eq!(glide(&harness).rect, middle);
    assert_eq!(harness.state.redraw_at(), None);
    // Into the other area: a jump.
    harness.input.pointer_moved(Vec2::new(600.0, 450.0));
    harness.frame_at(816);
    harness.frame_at(832);
    let other = harness.view.parts[2]
        .panel
        .inset(harness.styles.workspace.drop_center_inset);
    assert_eq!(glide(&harness).rect, other);
}

#[test]
fn stacked_editors_show_tabs_and_a_click_shows_one() {
    let mut harness = Harness::halves();
    let right = harness.area_ids()[1];
    let first = harness.workspace.stack(right).unwrap().active();
    let second = harness.workspace.add_tab(right).unwrap();
    harness.frame();
    let tabs = harness.tab_rects();
    assert_eq!(tabs.len(), 2);
    assert_eq!((tabs[0].0, tabs[1].0), (first, second));
    // The tab bar pushes the content down.
    let style = harness.styles.workspace;
    let content = harness.host.drawn.last().unwrap().1;
    assert_eq!(
        content.min.y,
        4.0 + style.tab_bar_height + style.top_bar_height
    );
    let at = tabs[0].1.center();
    harness.input.pointer_moved(at);
    harness.frame();
    harness.input.primary_button(true, at);
    harness.input.primary_button(false, at);
    harness.frame();
    assert_eq!(harness.workspace.stack(right).unwrap().active(), first);
    assert!(!harness.view.is_operating());
}

#[test]
fn a_right_click_cancels_docking_and_reaches_nothing_else() {
    // Regression test: it also selected the row of the outliner under the pointer.
    let mut harness = Harness::halves();
    harness.hold_top_bar(RIGHT_TOP_BAR);
    assert!(harness.view.operation.is_some());
    // Over the content of the other editor.
    let pointer = Vec2::new(200.0, 300.0);
    harness.input.pointer_moved(pointer);
    harness.frame();
    // The keymap cancels; here, the action it would trigger.
    harness
        .input
        .button(PointerButton::Secondary, true, pointer);
    harness.input.trigger(&OPERATION_CANCEL);
    harness.frame();
    assert!(harness.view.operation.is_none());
    assert!(!harness.host.secondary);
    harness
        .input
        .button(PointerButton::Secondary, false, pointer);
    harness.frame();
    assert!(!harness.host.secondary);
}

#[test]
fn a_drop_outside_the_window_takes_the_editor_out() {
    let mut harness = Harness::halves();
    harness.view.set_window(window());
    // The editor of the bar that is held.
    let editor = harness.shown()[0];
    harness.hold_top_bar(RIGHT_TOP_BAR);
    let outside = Vec2::new(1200.0, 300.0);
    harness.input.pointer_moved(outside);
    harness.frame();
    assert_eq!(harness.view.take_detached(), None);
    harness.input.primary_button(false, outside);
    harness.frame();
    assert_eq!(harness.view.take_detached(), Some(editor));
    assert_eq!(harness.view.take_detached(), None);
    // It is up to the application to move it.
    assert_eq!(harness.shown().len(), 2);
}

#[test]
fn a_drop_inside_the_window_takes_no_editor_out() {
    let mut harness = Harness::halves();
    harness.view.set_window(window());
    harness.hold_top_bar(RIGHT_TOP_BAR);
    let inside = Vec2::new(200.0, 300.0);
    harness.input.pointer_moved(inside);
    harness.frame();
    harness.input.primary_button(false, inside);
    harness.frame();
    assert_eq!(harness.view.take_detached(), None);
}

#[test]
fn without_a_window_a_drop_outside_does_nothing() {
    let mut harness = Harness::halves();
    harness.hold_top_bar(RIGHT_TOP_BAR);
    let outside = Vec2::new(1200.0, 300.0);
    harness.input.pointer_moved(outside);
    harness.frame();
    harness.input.primary_button(false, outside);
    harness.frame();
    assert_eq!(harness.view.take_detached(), None);
}

#[test]
fn the_area_under_a_point_is_found_from_the_last_frame() {
    let mut harness = Harness::halves();
    harness.frame();
    let ids = harness.area_ids();
    assert_eq!(harness.view.area_at(Vec2::new(100.0, 300.0)), Some(ids[0]));
    assert_eq!(harness.view.area_at(Vec2::new(600.0, 300.0)), Some(ids[1]));
    assert_eq!(harness.view.area_at(Vec2::new(2000.0, 300.0)), None);
}

#[test]
fn dragging_a_tab_along_the_bar_reorders_it() {
    let mut harness = Harness::halves();
    let right = harness.area_ids()[1];
    let first = harness.workspace.stack(right).unwrap().active();
    let second = harness.workspace.add_tab(right).unwrap();
    harness.frame();
    let tabs = harness.tab_rects();
    let past_second = Vec2::new(tabs[1].1.max.x + 4.0, tabs[1].1.center().y);
    harness.drag(tabs[0].1.center(), past_second);
    assert_eq!(
        harness.workspace.stack(right).unwrap().editors(),
        [second, first]
    );
    harness.workspace.check().unwrap();
}

#[test]
fn dragging_a_top_bar_beside_an_area_docks_the_editor_there() {
    let mut harness = Harness::halves();
    let [left_editor, right_editor] = harness.shown()[..] else {
        panic!("two areas");
    };
    harness.drag(LEFT_TOP_BAR, Vec2::new(790.0, 300.0));
    // The right area split in half; the emptied left area closed into its neighbour.
    assert_eq!(harness.areas().len(), 2);
    assert_eq!(harness.shown(), [right_editor, left_editor]);
    assert!(
        close(harness.areas()[1].x0, 3 * GRID / 4, 2),
        "{:?}",
        harness.areas()
    );
    assert_eq!(harness.areas()[0].x0, 0);
    harness.workspace.check().unwrap();
}

#[test]
fn dragging_a_top_bar_into_the_middle_of_an_area_adds_a_tab() {
    let mut harness = Harness::halves();
    let [left_editor, right_editor] = harness.shown()[..] else {
        panic!("two areas");
    };
    harness.drag(LEFT_TOP_BAR, Vec2::new(600.0, 320.0));
    assert_eq!(harness.areas(), [GridRect::FULL]);
    let area = harness.area_ids()[0];
    let stack = harness.workspace.stack(area).unwrap();
    assert_eq!(stack.editors(), [right_editor, left_editor]);
    assert_eq!(stack.active(), left_editor);
}

#[test]
fn escape_cancels_docking_and_a_click_on_a_top_bar_docks_nothing() {
    let mut harness = Harness::halves();
    let before = harness.areas();
    harness.input.pointer_moved(LEFT_TOP_BAR);
    harness.frame();
    harness.input.primary_button(true, LEFT_TOP_BAR);
    harness.frame();
    harness.input.pointer_moved(Vec2::new(600.0, 320.0));
    harness.frame();
    assert_eq!(harness.state.cursor(), CursorShape::Move);
    harness.input.trigger(&OPERATION_CANCEL);
    harness.frame();
    harness.input.primary_button(false, Vec2::new(600.0, 320.0));
    harness.frame();
    assert_eq!(harness.areas(), before);
    assert!(!harness.view.is_operating());
    // A click without moving.
    harness.input.pointer_moved(LEFT_TOP_BAR);
    harness.frame();
    harness.input.primary_button(true, LEFT_TOP_BAR);
    harness.input.primary_button(false, LEFT_TOP_BAR);
    harness.frame();
    assert_eq!(harness.areas(), before);
    assert!(!harness.view.is_operating());
}
