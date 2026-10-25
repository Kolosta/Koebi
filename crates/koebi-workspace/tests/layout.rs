//! Layout operations on concrete layouts: what moves, what joins, what is refused.

use koebi_workspace::{AreaId, GRID, GridRect, Layout, LayoutError, Line, Orientation, Side};

const MIN: u32 = 1_000;
/// Thirds and quarters of the grid.
const THIRD: u32 = GRID / 3;
const QUARTER: u32 = GRID / 4;

fn first(layout: &Layout) -> AreaId {
    layout.areas().next().expect("a layout has an area").0
}

fn rect(layout: &Layout, area: AreaId) -> GridRect {
    layout.rect(area).expect("the area exists")
}

/// Builds a layout from `(id, rect)` pairs, checking the tiling.
fn layout(areas: &[(u32, GridRect)]) -> Layout {
    Layout::from_areas(areas).expect("a valid tiling")
}

/// Finds the area whose rectangle is `r`.
fn area_with(layout: &Layout, r: GridRect) -> AreaId {
    layout
        .areas()
        .find(|(_, rect)| *rect == r)
        .expect("an area with that rectangle")
        .0
}

#[test]
fn splits_tile_the_grid_and_respect_the_minimum() {
    let mut layout = Layout::new();
    let left = first(&layout);
    let right = layout
        .split(left, Orientation::Vertical, GRID / 2, MIN)
        .unwrap();
    assert_eq!(rect(&layout, left), GridRect::new(0, 0, GRID / 2, GRID));
    assert_eq!(rect(&layout, right), GridRect::new(GRID / 2, 0, GRID, GRID));
    let bottom = layout
        .split(right, Orientation::Horizontal, QUARTER, MIN)
        .unwrap();
    assert_eq!(
        rect(&layout, bottom),
        GridRect::new(GRID / 2, QUARTER, GRID, GRID)
    );
    assert_eq!(layout.len(), 3);
    layout.check().unwrap();
    assert_eq!(layout.area_at(GRID - 1, GRID - 1), Some(bottom));

    // Parts thinner than the minimum, or empty, are refused.
    for position in [0, MIN - 1, GRID / 2 - MIN + 1, GRID / 2] {
        assert_eq!(
            layout.split(left, Orientation::Vertical, position, MIN),
            Err(LayoutError::TooSmall),
            "{position}"
        );
    }
    assert_eq!(layout.len(), 3);
    assert!(layout.split(left, Orientation::Vertical, MIN, MIN).is_ok());
}

/// Two columns, each cut in two rows at different heights.
fn two_columns() -> Layout {
    layout(&[
        (0, GridRect::new(0, 0, GRID / 2, THIRD)),
        (1, GridRect::new(0, THIRD, GRID / 2, GRID)),
        (2, GridRect::new(GRID / 2, 0, GRID, GRID / 2)),
        (3, GridRect::new(GRID / 2, GRID / 2, GRID, GRID)),
    ])
}

#[test]
fn lines_run_through_every_aligned_edge() {
    let mut layout = two_columns();
    let top_left = area_with(&layout, GridRect::new(0, 0, GRID / 2, THIRD));
    let line = layout.line(top_left, Side::Right).unwrap();
    // The vertical line runs the whole height, across both rows of both columns.
    assert_eq!(
        line,
        Line {
            orientation: Orientation::Vertical,
            position: GRID / 2,
            start: 0,
            end: GRID,
        }
    );
    let (before, after) = layout.line_areas(&line);
    assert_eq!((before.len(), after.len()), (2, 2));
    assert_eq!(layout.move_line(&line, GRID / 3, MIN), GRID / 3);
    layout.check().unwrap();
    assert!(
        layout
            .areas()
            .all(|(_, r)| r.x0 == GRID / 3 || r.x1 == GRID / 3)
    );
    // The row lines stay within their column.
    let row = layout.line(top_left, Side::Bottom).unwrap();
    assert_eq!((row.start, row.end, row.position), (0, GRID / 3, THIRD));
    // Edges on the border of the grid have no line.
    assert_eq!(layout.line(top_left, Side::Left), None);
    assert_eq!(layout.line(top_left, Side::Top), None);
}

#[test]
fn lines_stop_where_an_area_crosses_them() {
    // A band across the top, two columns below: the columns' line stops at the band.
    let layout = layout(&[
        (0, GridRect::new(0, 0, GRID, QUARTER)),
        (1, GridRect::new(0, QUARTER, GRID / 2, GRID)),
        (2, GridRect::new(GRID / 2, QUARTER, GRID, GRID)),
    ]);
    let line = layout.line(AreaId::from_saved(1), Side::Right).unwrap();
    assert_eq!((line.start, line.end), (QUARTER, GRID));
    let band = layout.line(AreaId::from_saved(0), Side::Bottom).unwrap();
    assert_eq!((band.start, band.end), (0, GRID));
}

#[test]
fn moving_a_line_stops_at_the_minimum_sizes() {
    let mut layout = two_columns();
    let top_left = area_with(&layout, GridRect::new(0, 0, GRID / 2, THIRD));
    let line = layout.line(top_left, Side::Right).unwrap();
    assert_eq!(layout.line_limits(&line, MIN), (MIN, GRID - MIN));
    assert_eq!(layout.move_line(&line, 0, MIN), MIN);
    let line = layout.line(top_left, Side::Right).unwrap();
    assert_eq!(layout.move_line(&line, GRID, MIN), GRID - MIN);
    layout.check().unwrap();
    // The row line of the left column is limited by both left areas.
    let row = layout.line(top_left, Side::Bottom).unwrap();
    assert_eq!(layout.line_limits(&row, MIN), (MIN, GRID - MIN));
}

#[test]
fn aligned_areas_join_into_one() {
    let mut layout = Layout::new();
    let left = first(&layout);
    let right = layout
        .split(left, Orientation::Vertical, GRID / 2, MIN)
        .unwrap();
    let plan = layout.join(right, left).unwrap();
    assert_eq!(plan.result, GridRect::FULL);
    assert_eq!(plan.fills, []);
    assert_eq!(layout.len(), 1);
    assert_eq!(rect(&layout, right), GridRect::FULL);
    layout.check().unwrap();
}

#[test]
fn unaligned_joins_hand_leftovers_to_neighbours() {
    // Left column cut in two rows, one area on the right.
    let mut layout = layout(&[
        (0, GridRect::new(0, 0, GRID / 2, GRID / 2)),
        (1, GridRect::new(0, GRID / 2, GRID / 2, GRID)),
        (2, GridRect::new(GRID / 2, 0, GRID, GRID)),
    ]);
    let (top, bottom, right) = (
        AreaId::from_saved(0),
        AreaId::from_saved(1),
        AreaId::from_saved(2),
    );
    // The right area is taller than the top one: it cannot extend across it.
    assert_eq!(layout.plan_join(right, top), Err(LayoutError::Unaligned));
    // The top area extends across the right one; the rest of it goes to the bottom area.
    let plan = layout.join(top, right).unwrap();
    assert_eq!(plan.result, GridRect::new(0, 0, GRID, GRID / 2));
    assert_eq!(plan.fills.len(), 1);
    assert_eq!(
        plan.fills[0].piece,
        GridRect::new(GRID / 2, GRID / 2, GRID, GRID)
    );
    assert_eq!(plan.fills[0].from, Side::Left);
    assert_eq!(
        rect(&layout, bottom),
        GridRect::new(0, GRID / 2, GRID, GRID)
    );
    layout.check().unwrap();
}

#[test]
fn leftovers_go_first_to_the_areas_opposite_the_kept_one() {
    let (c1, c2) = (THIRD, 2 * THIRD);
    let (h, h2) = (QUARTER, GRID / 2);
    let mut layout = layout(&[
        (0, GridRect::new(0, 0, c1, h)),
        (1, GridRect::new(0, h, c1, GRID)),
        (2, GridRect::new(c1, 0, c2, h2)),
        (3, GridRect::new(c1, h2, c2, GRID)),
        (4, GridRect::new(c2, 0, GRID, GRID)),
    ]);
    // The piece of area 2 below the kept area 0 could go to area 1 (towards the kept area) or
    // area 3 (opposite it): area 3, below, takes it.
    let plan = layout
        .join(AreaId::from_saved(0), AreaId::from_saved(2))
        .unwrap();
    assert_eq!(plan.fills[0].from, Side::Bottom);
    assert_eq!(
        rect(&layout, AreaId::from_saved(3)),
        GridRect::new(c1, h, c2, GRID)
    );
    assert_eq!(
        rect(&layout, AreaId::from_saved(1)),
        GridRect::new(0, h, c1, GRID)
    );
    layout.check().unwrap();
}

#[test]
fn joins_that_cannot_stay_rectangular_are_refused() {
    let (c1, c2) = (THIRD, 2 * THIRD);
    let (h, h2) = (QUARTER, GRID / 2);
    // The leftover of area 2 below area 0 touches only areas longer than its sides.
    let mut layout = layout(&[
        (0, GridRect::new(0, 0, c1, h)),
        (1, GridRect::new(0, h, c1, GRID)),
        (2, GridRect::new(c1, 0, c2, h2)),
        (3, GridRect::new(c1, h2, GRID, GRID)),
        (4, GridRect::new(c2, 0, GRID, h2)),
    ]);
    let before = layout.clone();
    assert_eq!(
        layout.join(AreaId::from_saved(0), AreaId::from_saved(2)),
        Err(LayoutError::CannotFill)
    );
    assert_eq!(layout, before);
    // Areas that do not touch, or touch at a corner only, do not join.
    assert_eq!(
        layout.plan_join(AreaId::from_saved(0), AreaId::from_saved(3)),
        Err(LayoutError::NotAdjacent)
    );
    assert_eq!(
        layout.plan_join(AreaId::from_saved(0), AreaId::from_saved(0)),
        Err(LayoutError::SameArea)
    );
    assert_eq!(
        layout.plan_join(AreaId::from_saved(0), AreaId::from_saved(99)),
        Err(LayoutError::UnknownArea(AreaId::from_saved(99)))
    );
}

/// Four arms around a centre: no binary split tree can make it.
fn pinwheel() -> Layout {
    let (a, b) = (THIRD, 2 * THIRD);
    layout(&[
        (0, GridRect::new(0, 0, b, a)),
        (1, GridRect::new(b, 0, GRID, b)),
        (2, GridRect::new(a, b, GRID, GRID)),
        (3, GridRect::new(0, a, a, GRID)),
        (4, GridRect::new(a, a, b, b)),
    ])
}

#[test]
fn pinwheels_resize_by_their_arms_and_keep_their_centre() {
    let mut layout = pinwheel();
    let centre = AreaId::from_saved(4);
    // Every neighbour of the centre is longer than the side it touches.
    assert_eq!(layout.plan_close(centre), Err(LayoutError::CannotFill));
    // The centre's top edge runs along the north arm, and continues along the west arm's top.
    let line = layout.line(centre, Side::Top).unwrap();
    assert_eq!((line.start, line.end), (0, 2 * THIRD));
    layout.move_line(&line, THIRD + MIN, MIN);
    layout.check().unwrap();
    assert_eq!(rect(&layout, centre).y0, THIRD + MIN);
    // Arms close into their neighbours.
    let mut closed = pinwheel();
    let plan = closed.close(AreaId::from_saved(0)).unwrap();
    assert_eq!(plan.fill.areas.len(), 2);
    closed.check().unwrap();
}

#[test]
fn closing_prefers_a_single_neighbour() {
    // Three rows; the middle one closes into whichever single neighbour comes first.
    let mut layout = layout(&[
        (0, GridRect::new(0, 0, GRID, THIRD)),
        (1, GridRect::new(0, THIRD, GRID, 2 * THIRD)),
        (2, GridRect::new(0, 2 * THIRD, GRID / 2, GRID)),
        (3, GridRect::new(GRID / 2, 2 * THIRD, GRID, GRID)),
    ]);
    let plan = layout.close(AreaId::from_saved(1)).unwrap();
    // One area above against two below: the one above takes the space.
    assert_eq!(plan.fill.from, Side::Top);
    assert_eq!(
        rect(&layout, AreaId::from_saved(0)),
        GridRect::new(0, 0, GRID, 2 * THIRD)
    );
    layout.check().unwrap();
    let mut single = Layout::new();
    let only = first(&single);
    assert_eq!(single.close(only), Err(LayoutError::LastArea));
}

#[test]
fn saved_layouts_are_checked() {
    let gap = Layout::from_areas(&[(0, GridRect::new(0, 0, GRID / 2, GRID))]);
    assert!(matches!(gap, Err(LayoutError::Invalid(_))));
    let overlap = Layout::from_areas(&[
        (0, GridRect::new(0, 0, GRID, GRID)),
        (1, GridRect::new(0, 0, 1, 1)),
    ]);
    assert!(matches!(overlap, Err(LayoutError::Invalid(_))));
    let twice = Layout::from_areas(&[
        (0, GridRect::new(0, 0, GRID / 2, GRID)),
        (0, GridRect::new(GRID / 2, 0, GRID, GRID)),
    ]);
    assert!(matches!(twice, Err(LayoutError::Invalid(_))));
    // New areas never reuse a saved identifier.
    let mut saved = Layout::from_areas(&[
        (7, GridRect::new(0, 0, GRID / 2, GRID)),
        (3, GridRect::new(GRID / 2, 0, GRID, GRID)),
    ])
    .unwrap();
    let added = saved
        .split(
            AreaId::from_saved(3),
            Orientation::Horizontal,
            GRID / 2,
            MIN,
        )
        .unwrap();
    assert_eq!(added.get(), 8);
}
