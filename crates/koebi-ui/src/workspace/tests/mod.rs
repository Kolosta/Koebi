//! Interaction tests of the workspace view, driven by synthetic input.

use core::time::Duration;

use koebi_core::input::{CursorShape, Modifiers};
use koebi_core::{Color, Rect, Vec2};
use koebi_render::{Corners, Scene};
use koebi_workspace::{AreaId, EditorId, GRID, GridRect, Orientation, Side, Workspace};

use super::*;
use crate::{Input, PointerButton, Styles, UiState};

mod docking;

/// The window: with a gap of 4, the grid covers (2, 2) to (802, 602), 800 x 600 pixels.
fn window() -> Rect {
    Rect::from_origin_size(Vec2::ZERO, Vec2::new(804.0, 604.0))
}

/// The pointer over the gutter between the halves.
const GUTTER_AT: Vec2 = Vec2::new(402.0, 300.0);

#[derive(Default)]
struct Host {
    drawn: Vec<(EditorId, Rect)>,
    copied: Vec<(EditorId, EditorId)>,
    removed: Vec<EditorId>,
    /// Editor types offered, none by default, and the types set.
    kinds: Vec<&'static str>,
    set: Vec<(EditorId, usize)>,
}

impl EditorHost for Host {
    fn title(&self, _editor: EditorId) -> &'static str {
        "Editor"
    }

    fn kinds(&self) -> &[&str] {
        &self.kinds
    }

    fn set_kind(&mut self, editor: EditorId, kind: usize) {
        self.set.push((editor, kind));
    }

    fn content(&mut self, _ui: &mut Ui<'_>, editor: EditorId, rect: Rect) {
        self.drawn.push((editor, rect));
    }

    fn copied(&mut self, source: EditorId, editor: EditorId) {
        self.copied.push((source, editor));
    }

    fn removed(&mut self, editors: &[EditorId]) {
        self.removed.extend_from_slice(editors);
    }
}

struct Harness {
    state: UiState,
    view: WorkspaceView,
    workspace: Workspace,
    input: Input,
    host: Host,
    styles: Styles,
}

impl Harness {
    /// Two halves side by side, laid out once.
    fn halves() -> Self {
        let mut workspace = Workspace::new();
        let first = workspace.layout().areas().next().unwrap().0;
        workspace
            .split(first, Orientation::Vertical, GRID / 2, 1)
            .unwrap();
        Self::with(workspace)
    }

    /// The areas of `areas`, one editor each, laid out once.
    fn tiled(areas: &[(u32, GridRect)]) -> Self {
        let layout = Layout::from_areas(areas).unwrap();
        let stacks: Vec<(u32, Vec<u32>, usize)> =
            areas.iter().map(|(id, _)| (*id, vec![*id], 0)).collect();
        Self::with(Workspace::from_saved(layout, &stacks).unwrap())
    }

    fn with(workspace: Workspace) -> Self {
        let mut state = UiState::new();
        state.set_fonts(koebi_text::Fonts::with_embedded());
        let mut harness = Self {
            state,
            view: WorkspaceView::new(),
            workspace,
            input: Input::new(),
            host: Host::default(),
            styles: crate::test_styles(),
        };
        harness.frame();
        harness
    }

    fn frame(&mut self) -> Scene {
        let mut scene = Scene::new();
        {
            let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
            ui.workspace(
                &mut self.view,
                &mut self.workspace,
                window(),
                &mut self.host,
            );
        }
        self.input.end_frame();
        scene
    }

    fn areas(&self) -> Vec<GridRect> {
        self.workspace
            .layout()
            .areas()
            .map(|(_, rect)| rect)
            .collect()
    }

    fn left(&self) -> GridRect {
        self.areas()[0]
    }

    fn area_ids(&self) -> Vec<AreaId> {
        self.workspace.layout().areas().map(|(id, _)| id).collect()
    }

    /// Opens the area menu on the gutter and clicks its item `index`.
    fn area_menu(&mut self, index: usize) {
        self.input.pointer_moved(GUTTER_AT);
        self.frame();
        self.input.button(PointerButton::Secondary, true, GUTTER_AT);
        self.input
            .button(PointerButton::Secondary, false, GUTTER_AT);
        self.frame();
        assert!(self.view.menu.is_open());
        // Lay the menu out, then click the item.
        self.frame();
        let menu = self.styles.menu;
        let rows = [
            menu.item_height,
            menu.item_height,
            menu.padding_vertical + menu.separator_width,
            menu.item_height,
            menu.item_height,
        ];
        // It opens with the pointer over its first item.
        let y = GUTTER_AT.y - rows[0] * 0.5 + rows[..index].iter().sum::<f32>() + rows[index] * 0.5;
        let item = Vec2::new(GUTTER_AT.x, y);
        self.input.primary_button(true, item);
        self.input.primary_button(false, item);
        self.frame();
    }
}

fn close(a: u32, b: u32, tolerance: u32) -> bool {
    a.abs_diff(b) <= tolerance
}

#[test]
fn areas_draw_their_panels_and_editors() {
    let harness = Harness::halves();
    assert_eq!(harness.host.drawn.len(), 2);
    let (_, left) = harness.host.drawn[0];
    // The content lies inside the left panel, below its top bar.
    let style = harness.styles.workspace;
    assert_eq!(left.min.x, 4.0);
    assert_eq!(left.min.y, 4.0 + style.top_bar_height);
    assert_eq!(left.max.x, 400.0);
}

#[test]
fn dragging_a_gutter_moves_its_line_and_escape_restores_it() {
    let mut harness = Harness::halves();
    let before = harness.areas();
    harness.input.pointer_moved(GUTTER_AT);
    harness.frame();
    harness.input.primary_button(true, GUTTER_AT);
    harness.frame();
    assert!(harness.view.is_operating());
    // 80 pixels right is a tenth of the grid.
    harness
        .input
        .pointer_moved(GUTTER_AT + Vec2::new(80.0, 0.0));
    harness.frame();
    assert!(
        close(harness.left().x1, GRID / 2 + GRID / 10, 2),
        "{:?}",
        harness.left()
    );
    // The resize cursor stays during the drag.
    assert_eq!(harness.state.cursor(), CursorShape::ResizeHorizontal);
    harness.input.trigger(&OPERATION_CANCEL);
    harness.frame();
    assert_eq!(harness.areas(), before);
    assert!(!harness.view.is_operating());
    // Moving on after the cancel changes nothing.
    harness
        .input
        .pointer_moved(GUTTER_AT + Vec2::new(160.0, 0.0));
    harness.frame();
    harness
        .input
        .primary_button(false, GUTTER_AT + Vec2::new(160.0, 0.0));
    harness.frame();
    assert_eq!(harness.areas(), before);
}

#[test]
fn releasing_keeps_the_resize() {
    let mut harness = Harness::halves();
    harness.input.pointer_moved(GUTTER_AT);
    harness.frame();
    harness.input.primary_button(true, GUTTER_AT);
    harness.frame();
    harness
        .input
        .primary_button(false, GUTTER_AT - Vec2::new(80.0, 0.0));
    harness.frame();
    assert!(close(harness.left().x1, GRID / 2 - GRID / 10, 2));
    assert!(!harness.view.is_operating());
    harness.workspace.check().unwrap();
}

#[test]
fn shift_resizes_ten_times_finer_and_ctrl_snaps() {
    let mut harness = Harness::halves();
    harness.input.pointer_moved(GUTTER_AT);
    harness.frame();
    harness.input.primary_button(true, GUTTER_AT);
    harness.frame();
    harness.input.set_modifiers(Modifiers::LEFT_SHIFT);
    harness
        .input
        .pointer_moved(GUTTER_AT + Vec2::new(80.0, 0.0));
    harness.frame();
    assert!(
        close(harness.left().x1, GRID / 2 + GRID / 100, 2),
        "{:?}",
        harness.left()
    );
    // Releasing Shift mid-drag: later moves are full speed, earlier ones stay fine.
    harness.input.set_modifiers(Modifiers::default());
    harness
        .input
        .pointer_moved(GUTTER_AT + Vec2::new(160.0, 0.0));
    harness.frame();
    assert!(close(
        harness.left().x1,
        GRID / 2 + GRID / 100 + GRID / 10,
        3
    ));
    // Ctrl snaps to a 48th of the window.
    harness.input.set_modifiers(Modifiers::LEFT_CTRL);
    harness
        .input
        .pointer_moved(GUTTER_AT + Vec2::new(163.0, 0.0));
    harness.frame();
    let steps = f64::from(harness.left().x1) * 48.0 / f64::from(GRID);
    assert!(
        (steps - steps.round()).abs() * f64::from(GRID) / 48.0 < 1.0,
        "{steps}"
    );
}

#[test]
fn resizing_stops_at_the_minimum_size() {
    let mut harness = Harness::halves();
    harness.input.pointer_moved(GUTTER_AT);
    harness.frame();
    harness.input.primary_button(true, GUTTER_AT);
    harness.frame();
    harness.input.pointer_moved(Vec2::new(-500.0, 300.0));
    harness.frame();
    // 40 pixels of 800, rounded up.
    let min = koebi_workspace::grid_length(harness.styles.workspace.min_size, 800.0);
    assert_eq!(harness.left().x1, min);
    // Coming back moves the line again at once.
    harness.input.pointer_moved(Vec2::new(-420.0, 300.0));
    harness.frame();
    assert!(harness.left().x1 > min);
}

#[test]
fn the_band_shows_after_the_hover_delay() {
    let mut harness = Harness::halves();
    let resting = harness.frame().rect_count();
    harness.input.pointer_moved(GUTTER_AT);
    let hovering = harness.frame().rect_count();
    assert_eq!(hovering, resting);
    let delay = harness.styles.workspace.hover_delay;
    assert_eq!(harness.state.redraw_at(), Some(delay));
    harness.input.set_time(delay);
    // The whole line, then the stretch between the two hovered areas.
    assert_eq!(harness.frame().rect_count(), resting + 2);
    harness.input.pointer_moved(Vec2::new(200.0, 300.0));
    harness.input.set_time(delay * 2);
    assert_eq!(harness.frame().rect_count(), resting);
}

#[test]
fn the_area_menu_swaps_the_areas() {
    let mut harness = Harness::halves();
    let ids = harness.area_ids();
    let stacks: Vec<_> = ids
        .iter()
        .map(|id| harness.workspace.stack(*id).unwrap().active())
        .collect();
    harness.area_menu(0);
    assert!(!harness.view.menu.is_open());
    assert_eq!(harness.workspace.stack(ids[0]).unwrap().active(), stacks[1]);
    assert_eq!(harness.workspace.stack(ids[1]).unwrap().active(), stacks[0]);
}

#[test]
fn the_split_preview_follows_the_pointer_and_a_press_splits() {
    let mut harness = Harness::halves();
    harness.area_menu(3);
    assert!(harness.view.is_operating());
    let target = Vec2::new(202.0, 300.0);
    harness.input.pointer_moved(target);
    let preview = harness.frame();
    // Pointing into an area previews its two halves.
    harness.frame();
    assert_eq!(harness.state.cursor(), CursorShape::ResizeHorizontal);
    harness.input.primary_button(true, target);
    harness.frame();
    assert!(!harness.view.is_operating());
    assert_eq!(harness.areas().len(), 3);
    assert!(
        close(harness.left().x1, GRID / 4, 2),
        "{:?}",
        harness.left()
    );
    assert_eq!(harness.host.copied.len(), 1);
    assert!(preview.rect_count() > 0);
    harness.workspace.check().unwrap();
}

#[test]
fn the_join_preview_removes_the_area_under_the_pointer() {
    let mut harness = Harness::halves();
    let ids = harness.area_ids();
    let right_editor = harness.workspace.stack(ids[1]).unwrap().active();
    harness.area_menu(1);
    assert!(harness.view.is_operating());
    // Outside both areas nothing can join: the press is ignored.
    harness.input.pointer_moved(Vec2::new(1.0, 1.0));
    harness.frame();
    harness.frame();
    assert_eq!(harness.state.cursor(), CursorShape::NotAllowed);
    harness.input.primary_button(true, Vec2::new(1.0, 1.0));
    harness.frame();
    harness.input.primary_button(false, Vec2::new(1.0, 1.0));
    harness.frame();
    assert_eq!(harness.areas().len(), 2);
    // Over the right area: it goes, into the left one; the arrow points the way the left
    // area grows, towards the right one (#99).
    let target = Vec2::new(600.0, 300.0);
    harness.input.pointer_moved(target);
    harness.frame();
    harness.frame();
    assert_eq!(harness.state.cursor(), CursorShape::ArrowRight);
    harness.input.primary_button(true, target);
    harness.frame();
    assert_eq!(harness.areas(), [GridRect::FULL]);
    assert_eq!(harness.host.removed, [right_editor]);
}

#[test]
fn a_right_click_cancels_a_preview() {
    let mut harness = Harness::halves();
    harness.area_menu(4);
    assert!(harness.view.is_operating());
    harness.input.pointer_moved(Vec2::new(200.0, 300.0));
    harness.frame();
    harness.frame();
    harness
        .input
        .button(PointerButton::Secondary, true, Vec2::new(200.0, 300.0));
    harness.frame();
    assert!(!harness.view.is_operating());
    assert_eq!(harness.areas().len(), 2);
    // The cancel took the right click: no menu opened either.
    assert!(!harness.view.menu.is_open());
    assert_eq!(Duration::ZERO, harness.input.time());
}

impl Harness {
    /// Presses at `from`, moves in two steps to `to`, and releases there.
    fn drag(&mut self, from: Vec2, to: Vec2) {
        self.input.pointer_moved(from);
        self.frame();
        self.input.primary_button(true, from);
        self.frame();
        self.input.pointer_moved((from + to) * 0.5);
        self.frame();
        self.input.pointer_moved(to);
        self.frame();
        self.input.primary_button(false, to);
        self.frame();
    }
}

/// Inside the bottom-left corner of the left panel, (4, 4) to (400, 600).
const LEFT_BOTTOM_CORNER: Vec2 = Vec2::new(8.0, 596.0);
/// Inside the top-right corner of the left panel, at the top of the bar.
const LEFT_TOP_RIGHT_CORNER: Vec2 = Vec2::new(396.0, 6.0);

#[test]
fn dragging_a_corner_inwards_splits_along_the_drag() {
    let mut harness = Harness::halves();
    harness.input.pointer_moved(LEFT_BOTTOM_CORNER);
    harness.frame();
    harness.frame();
    assert_eq!(harness.state.cursor(), CursorShape::Crosshair);
    // A horizontal drag makes a vertical line where the pointer lets go.
    harness.drag(LEFT_BOTTOM_CORNER, Vec2::new(202.0, 560.0));
    assert_eq!(harness.areas().len(), 3);
    assert!(
        close(harness.left().x1, GRID / 4, 2),
        "{:?}",
        harness.left()
    );
    assert_eq!(harness.host.copied.len(), 1);
    assert!(!harness.view.is_operating());
    harness.workspace.check().unwrap();
    // A vertical drag makes a horizontal line.
    harness.drag(LEFT_BOTTOM_CORNER, Vec2::new(20.0, 302.0));
    assert_eq!(harness.areas().len(), 4);
    assert!(
        close(harness.left().y1, GRID / 2, 2),
        "{:?}",
        harness.left()
    );
}

#[test]
fn dragging_a_corner_into_a_neighbour_joins_it() {
    let mut harness = Harness::halves();
    let ids = harness.area_ids();
    let right_editor = harness.workspace.stack(ids[1]).unwrap().active();
    harness.drag(LEFT_TOP_RIGHT_CORNER, Vec2::new(600.0, 300.0));
    assert_eq!(harness.areas(), [GridRect::FULL]);
    assert_eq!(harness.host.removed, [right_editor]);
}

#[test]
fn escape_cancels_a_corner_drag() {
    let mut harness = Harness::halves();
    let before = harness.areas();
    harness.input.pointer_moved(LEFT_BOTTOM_CORNER);
    harness.frame();
    harness.input.primary_button(true, LEFT_BOTTOM_CORNER);
    harness.frame();
    harness.input.pointer_moved(Vec2::new(202.0, 560.0));
    harness.frame();
    assert!(harness.view.is_operating());
    assert_eq!(harness.state.cursor(), CursorShape::ResizeHorizontal);
    harness.input.trigger(&OPERATION_CANCEL);
    harness.frame();
    harness.input.primary_button(false, Vec2::new(202.0, 560.0));
    harness.frame();
    assert_eq!(harness.areas(), before);
    assert_eq!(harness.host.copied, []);
}

impl Harness {
    /// The editor shown in each area, area by area.
    fn shown(&self) -> Vec<EditorId> {
        self.area_ids()
            .iter()
            .map(|area| self.workspace.stack(*area).unwrap().active())
            .collect()
    }

    /// The tabs drawn last frame, as (editor, rectangle).
    fn tab_rects(&self) -> Vec<(EditorId, Rect)> {
        self.view
            .tabs
            .iter()
            .map(|tab| (tab.editor, tab.rect))
            .collect()
    }
}

/// On the left area's top bar, away from anything drawn on it.
const LEFT_TOP_BAR: Vec2 = Vec2::new(200.0, 20.0);

#[test]
fn the_join_preview_darkens_everything_around_the_future_area() {
    let half = GRID / 2;
    let harness = Harness::tiled(&[
        (0, GridRect::new(0, 0, half, GRID)),
        (1, GridRect::new(half, 0, GRID, half)),
        (2, GridRect::new(half, half, GRID, GRID)),
    ]);
    let ids = harness.area_ids();
    let layout = harness.workspace.layout();
    // The top right area takes the top of the left area; the bottom right area takes the rest.
    let plan = layout.plan_join(ids[1], ids[0]).unwrap();
    assert_eq!(plan.fills.len(), 1);
    let style = harness.styles.workspace;
    let frame = Frame::new(window(), style.gap);
    let mut drawn = Vec::new();
    preview::join_preview(layout, &plan, frame, &style, &mut |primitive| {
        drawn.push(primitive);
    });
    let radius = style.corner_radius;
    let stays = frame.panel(layout.rect(ids[1]).unwrap());
    let future = frame.panel(plan.result);
    let piece = frame.panel(plan.fills[0].piece);

    // What stays, lighter and rounded.
    assert_eq!(drawn[0].rect, stays);
    assert_eq!(drawn[0].fill, style.join_keep);
    assert_eq!(drawn[0].corner_radii, Corners::all(radius));
    // What the kept area takes reaches what stays, across the gutter, square towards it.
    assert_eq!(drawn[1].fill, style.join_remove);
    assert_eq!(drawn[1].rect.max.x, stays.min.x);
    assert_eq!(drawn[1].rect.min.y, future.min.y);
    assert_eq!(drawn[1].rect.max.y, future.max.y);
    assert_eq!(drawn[1].corner_radii.top_right, 0.0);
    assert_eq!(drawn[1].corner_radii.bottom_right, 0.0);
    assert_eq!(drawn[1].corner_radii.top_left, radius);
    // What the neighbour takes reaches the future area, across the gutter, with square corners.
    assert_eq!(drawn[2].fill, style.join_residual);
    assert_eq!(drawn[2].rect.min.y, future.max.y);
    assert_eq!(drawn[2].rect.max.y, piece.max.y);
    assert_eq!(drawn[2].corner_radii, Corners::ZERO);
    // The outsides of the rounded corners of what stays, on the side that goes.
    let outsides: Vec<_> = drawn
        .iter()
        .filter(|primitive| primitive.inverted)
        .collect();
    let around_kept: Vec<_> = outsides.iter().filter(|o| o.rect == stays).collect();
    assert_eq!(around_kept.len(), 2);
    assert!(around_kept.iter().all(|o| o.fill == style.join_remove
        && o.corner_radii.top_right <= 0.0
        && o.corner_radii.bottom_right <= 0.0));
    // And of the future area where it ends over what goes: darker next to what neighbours take.
    let around_future: Vec<_> = outsides.iter().filter(|o| o.rect == future).collect();
    assert_eq!(around_future.len(), 2);
    assert!(
        around_future
            .iter()
            .any(|o| o.corner_radii.top_left > 0.0 && o.fill == style.join_remove)
    );
    assert!(
        around_future
            .iter()
            .any(|o| o.corner_radii.bottom_left > 0.0 && o.fill == style.join_residual)
    );
    // The frame comes last, over the seams.
    let frame_drawn = drawn.last().unwrap();
    assert_eq!(frame_drawn.rect, future);
    assert_eq!(frame_drawn.fill, Color::TRANSPARENT);
}

#[test]
fn the_join_preview_darkens_the_leftovers_of_the_kept_area_on_both_sides() {
    let (half, third) = (GRID / 2, GRID / 3);
    let harness = Harness::tiled(&[
        (0, GridRect::new(0, 0, half, GRID)),
        (1, GridRect::new(half, 0, GRID, third)),
        (2, GridRect::new(half, third, GRID, 2 * third)),
        (3, GridRect::new(half, 2 * third, GRID, GRID)),
    ]);
    let ids = harness.area_ids();
    let layout = harness.workspace.layout();
    // The tall left area keeps the middle third and takes the area beside it.
    let plan = layout.plan_join(ids[0], ids[2]).unwrap();
    assert_eq!(plan.fills.len(), 2);
    let style = harness.styles.workspace;
    let frame = Frame::new(window(), style.gap);
    let mut drawn = Vec::new();
    preview::join_preview(layout, &plan, frame, &style, &mut |primitive| {
        drawn.push(primitive);
    });
    let stays = drawn[0].rect;
    // Both leftovers reach what stays, across the gutter.
    let leftovers: Vec<_> = drawn
        .iter()
        .filter(|p| p.fill == style.join_residual && !p.inverted)
        .collect();
    assert_eq!(leftovers.len(), 2);
    assert!(leftovers.iter().any(|p| p.rect.max.y == stays.min.y));
    assert!(leftovers.iter().any(|p| p.rect.min.y == stays.max.y));
    // Every corner of what stays lies against something that goes, the darkest next to it.
    let around_kept: Vec<_> = drawn
        .iter()
        .filter(|p| p.inverted && p.rect == stays)
        .collect();
    assert_eq!(around_kept.len(), 4);
    assert!(around_kept.iter().all(|p| p.fill == style.join_residual));
}

#[test]
fn the_join_preview_of_aligned_areas_darkens_the_whole_area_that_goes() {
    let harness = Harness::halves();
    let ids = harness.area_ids();
    let layout = harness.workspace.layout();
    let plan = layout.plan_join(ids[0], ids[1]).unwrap();
    assert_eq!(plan.fills, []);
    let style = harness.styles.workspace;
    let frame = Frame::new(window(), style.gap);
    let mut drawn = Vec::new();
    preview::join_preview(layout, &plan, frame, &style, &mut |primitive| {
        drawn.push(primitive);
    });
    let stays = frame.panel(layout.rect(ids[0]).unwrap());
    let goes = frame.panel(layout.rect(ids[1]).unwrap());
    assert_eq!(drawn[1].rect.min.x, stays.max.x);
    assert_eq!(drawn[1].rect.max, goes.max);
    assert_eq!(
        drawn.iter().filter(|primitive| primitive.inverted).count(),
        4
    );
    assert!(
        drawn
            .iter()
            .filter(|p| p.inverted)
            .all(|p| p.fill == style.join_remove)
    );
}

#[test]
fn the_editor_type_dropdown_changes_the_kind() {
    let mut harness = Harness::halves();
    harness.host.kinds = vec!["Viewport", "Outliner"];
    harness.frame();
    let left_editor = harness.shown()[0];
    // The dropdown starts after the top bar's padding.
    let style = harness.styles.workspace;
    let control = Vec2::new(
        4.0 + style.top_bar_padding + 8.0,
        4.0 + style.top_bar_height * 0.5,
    );
    harness.input.pointer_moved(control);
    harness.frame();
    harness.input.primary_button(true, control);
    harness.input.primary_button(false, control);
    harness.frame();
    harness.frame();
    // The second item of the menu below the control.
    let menu = harness.styles.menu;
    let top = 4.0 + style.top_bar_height - style.top_bar_padding + menu.border_width;
    let second = Vec2::new(control.x, top + menu.rows_inset() + menu.item_height * 1.5);
    harness.input.pointer_moved(second);
    harness.frame();
    harness.input.primary_button(true, second);
    harness.input.primary_button(false, second);
    harness.frame();
    assert_eq!(harness.host.set, [(left_editor, 1)]);
}

/// Moves the pointer to `to` with the button held since `from`, and lays out a frame.
fn hold_at(harness: &mut Harness, from: Vec2, to: Vec2) {
    harness.input.pointer_moved(from);
    harness.frame();
    harness.input.primary_button(true, from);
    harness.frame();
    harness.input.pointer_moved((from + to) * 0.5);
    harness.frame();
    harness.input.pointer_moved(to);
    harness.frame();
    harness.frame();
}

#[test]
fn corner_drags_point_where_the_area_grows_and_refuse_elsewhere() {
    // #99: three columns; a drag from the left column's corner.
    let (third, two_thirds) = (GRID / 3, 2 * (GRID / 3));
    let mut harness = Harness::tiled(&[
        (0, GridRect::new(0, 0, third, GRID)),
        (1, GridRect::new(third, 0, two_thirds, GRID)),
        (2, GridRect::new(two_thirds, 0, GRID, GRID)),
    ]);
    // The top-right corner of the left column, within the top bar's padding.
    let corner = Vec2::new(2.0 + 800.0 / 3.0 - 6.0, 6.0);
    // Into the middle column: the left one grows right.
    hold_at(&mut harness, corner, Vec2::new(400.0, 300.0));
    assert_eq!(harness.state.cursor(), CursorShape::ArrowRight);
    // Over the right column, not adjacent: nothing can happen there.
    harness.input.pointer_moved(Vec2::new(700.0, 300.0));
    harness.frame();
    harness.frame();
    assert_eq!(harness.state.cursor(), CursorShape::NotAllowed);
    // Outside every area as well.
    harness.input.pointer_moved(Vec2::new(1.0, 300.0));
    harness.frame();
    harness.frame();
    assert_eq!(harness.state.cursor(), CursorShape::NotAllowed);
    harness.input.primary_button(false, Vec2::new(1.0, 300.0));
    harness.frame();
    assert_eq!(harness.areas().len(), 3);
}

#[test]
fn a_corner_drag_joins_areas_that_only_partly_share_an_edge() {
    // #98: A (top left, two thirds high) and D (bottom right, two thirds high) share the
    // middle third of the line between the columns.
    let (third, two_thirds) = (GRID / 3, 2 * (GRID / 3));
    let mut harness = Harness::tiled(&[
        (0, GridRect::new(0, 0, GRID / 2, two_thirds)),
        (1, GridRect::new(0, two_thirds, GRID / 2, GRID)),
        (2, GridRect::new(GRID / 2, 0, GRID, third)),
        (3, GridRect::new(GRID / 2, third, GRID, GRID)),
    ]);
    // A's bottom-right corner, into D.
    let corner = Vec2::new(396.0, 2.0 + 400.0 - 6.0);
    hold_at(&mut harness, corner, Vec2::new(600.0, 350.0));
    assert_eq!(harness.state.cursor(), CursorShape::ArrowRight);
    // The preview darkens both leftovers: four translucent pieces and an outline at least.
    let preview = harness.frame();
    assert!(preview.rect_count() >= 5);
    harness.input.primary_button(false, Vec2::new(600.0, 350.0));
    harness.frame();
    assert_eq!(
        harness.areas(),
        [
            GridRect::new(0, third, GRID, two_thirds),
            GridRect::new(0, two_thirds, GRID, GRID),
            GridRect::new(0, 0, GRID, third),
        ]
    );
    assert_eq!(harness.host.removed, [EditorId::from_saved(3)]);
    harness.workspace.check().unwrap();
}

#[test]
fn the_area_menu_offers_to_join_partly_aligned_areas() {
    let (third, two_thirds) = (GRID / 3, 2 * (GRID / 3));
    let layout = Layout::from_areas(&[
        (0, GridRect::new(0, 0, GRID / 2, two_thirds)),
        (1, GridRect::new(0, two_thirds, GRID / 2, GRID)),
        (2, GridRect::new(GRID / 2, 0, GRID, third)),
        (3, GridRect::new(GRID / 2, third, GRID, GRID)),
    ])
    .unwrap();
    let (a, d) = (AreaId::from_saved(0), AreaId::from_saved(3));
    assert!(layout.plan_join(a, d).is_ok() && layout.plan_join(d, a).is_ok());
}
