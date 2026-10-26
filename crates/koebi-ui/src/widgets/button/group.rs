//! Button groups: buttons fused side by side, stacked, or on a grid with spans (Ctrl above L
//! and R).
//!
//! - **Places**: a cell goes where [`GroupCell::at`] puts it; the others take, in order, the
//!   next free place along the group's direction, past the cells already placed.
//! - **Tracks**: a column (or row) fits the buttons it holds, keeps a fixed size, or takes a
//!   share of the room the others leave ([`Track`]); a button across several tracks widens
//!   those that are not fixed. In a rectangle smaller than its own size, the group shrinks its
//!   fitting tracks first, then all of them: it never leaves its rectangle.
//! - **One border between two buttons**: cells tile the group; each cell's outline covers its
//!   own left and top edges and the shared line just past its right and bottom edges, so two
//!   neighbours draw the same line at the same place, never two lines.
//! - **Colors of shared lines**: outlines are drawn in the order of their cell's state
//!   priority ([`ControlState::priority`]): disabled, rest, selected, hovered, pressed; the cell
//!   of highest priority draws last and owns all four sides of its outline, so a hovered or
//!   pressed button never shows a neighbour's color on one side. Equal priorities keep the
//!   cells' order. Outlines are drawn over one another: with translucent border colors, a
//!   shared line shows both.
//! - **Corners**: only the corners of the group's outline are rounded.

use koebi_core::{Color, Rect, Vec2};
use koebi_design::kind;
use koebi_render::{Corners, RectPrimitive};

use super::Button;
use crate::style::Prop;
use crate::ui::Ui;
use crate::widgets::parts::ControlState;

/// The most columns, and the most rows, a group lays out.
const MAX_TRACKS: usize = 16;

/// The most cells a group shows.
const MAX_CELLS: usize = 2 * MAX_TRACKS;

/// How wide a column of a group is, or how high a row.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Track {
    /// As wide (or high) as the buttons it holds ask for.
    #[default]
    Fit,
    /// This size.
    Fixed(f32),
    /// A share of the room the other tracks leave, in proportion to this weight; as wide as
    /// its buttons when the group measures itself.
    Share(f32),
}

/// A button of a group and its place on the grid.
///
/// The group draws the borders and the corners: the button's own border width and corner
/// radii are not used; its colors, content and behaviour are.
#[derive(Debug)]
pub struct GroupCell<'a> {
    button: Button<'a>,
    place: Option<(u16, u16)>,
    span: (u16, u16),
}

impl<'a> GroupCell<'a> {
    /// `button` in the next free place along the group's direction.
    pub fn new(button: Button<'a>) -> Self {
        Self {
            button,
            place: None,
            span: (1, 1),
        }
    }

    /// The cell at column `column` and row `row`.
    #[must_use]
    pub fn at(mut self, column: u16, row: u16) -> Self {
        self.place = Some((column, row));
        self
    }

    /// The cell across `columns` columns and `rows` rows.
    #[must_use]
    pub fn span(mut self, columns: u16, rows: u16) -> Self {
        self.span = (columns.max(1), rows.max(1));
        self
    }
}

/// What happened to a group this frame, by cell index.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GroupResponse {
    /// The cell whose action fired.
    pub clicked: Option<usize>,
    /// The cell double-clicked.
    pub double_clicked: Option<usize>,
    /// The cell a secondary press went down on.
    pub secondary_pressed: Option<usize>,
    /// The cell under the pointer.
    pub hovered: Option<usize>,
    /// A cell toggled its value.
    pub changed: bool,
}

/// A group of fused buttons: horizontal, vertical, or on a grid.
///
/// A group lays out at most 16 columns and 16 rows and shows at most 32 cells; cells past
/// those, or without a free place, are not shown.
#[derive(Debug)]
pub struct ButtonGroup<'t> {
    vertical: bool,
    columns: &'t [Track],
    rows: &'t [Track],
    corner_radius: Option<Prop<kind::Dimension>>,
    border_width: Option<Prop<kind::Dimension>>,
}

/// Where a cell lies on the grid: its first column and row, and how many it covers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Place {
    column: usize,
    row: usize,
    columns: usize,
    rows: usize,
}

/// The places of the cells, by index; `None` for a cell past the limits or without room.
type Places = [Option<Place>; MAX_CELLS];

/// The grid of a group laid out in a rectangle: where each track starts.
struct Grid {
    columns: [f32; MAX_TRACKS + 1],
    rows: [f32; MAX_TRACKS + 1],
    size: (usize, usize),
}

impl<'t> ButtonGroup<'t> {
    /// Buttons side by side, in the order of the cells (each button has its own identifier).
    pub fn horizontal() -> Self {
        Self {
            vertical: false,
            columns: &[],
            rows: &[],
            corner_radius: None,
            border_width: None,
        }
    }

    /// Buttons stacked, in the order of the cells.
    pub fn vertical() -> Self {
        Self {
            vertical: true,
            ..Self::horizontal()
        }
    }

    /// The tracks of the columns; columns without a track fit their buttons.
    #[must_use]
    pub fn columns(mut self, columns: &'t [Track]) -> Self {
        self.columns = columns;
        self
    }

    /// The tracks of the rows; rows without a track fit their buttons.
    #[must_use]
    pub fn rows(mut self, rows: &'t [Track]) -> Self {
        self.rows = rows;
        self
    }

    /// The radius of the group's outer corners.
    #[must_use]
    pub fn corner_radius(mut self, radius: impl Into<Prop<kind::Dimension>>) -> Self {
        self.corner_radius = Some(radius.into());
        self
    }

    /// The width of the borders around and between the buttons.
    #[must_use]
    pub fn border_width(mut self, width: impl Into<Prop<kind::Dimension>>) -> Self {
        self.border_width = Some(width.into());
        self
    }

    /// The width of the borders: the call's, or the style's.
    fn border(&self, ui: &Ui<'_>) -> f32 {
        let styles = ui.styles();
        self.border_width
            .as_ref()
            .map_or(styles.button.group_border_width, |prop| {
                prop.resolve(&styles.tokens)
            })
    }

    /// The tracks of the columns and rows sized for `room` (each axis optional: the natural
    /// size when absent). Each cell is measured once.
    fn grid(
        &self,
        ui: &mut Ui<'_>,
        cells: &[GroupCell<'_>],
        places: &Places,
        room: Option<Vec2>,
    ) -> Grid {
        let border = self.border(ui);
        let mut widths = [0.0_f32; MAX_TRACKS];
        let mut heights = [0.0_f32; MAX_TRACKS];
        let mut across = [(0, 0, 0.0); MAX_CELLS];
        let mut down = [(0, 0, 0.0); MAX_CELLS];
        let (mut column_count, mut row_count, mut count) = (0, 0, 0);
        for (cell, place) in cells.iter().zip(places) {
            let Some(place) = *place else {
                continue;
            };
            let size = cell.button.measure(ui, border);
            column_count = column_count.max(place.column + place.columns);
            row_count = row_count.max(place.row + place.rows);
            if place.columns == 1 {
                widths[place.column] = widths[place.column].max(size.x);
            }
            if place.rows == 1 {
                heights[place.row] = heights[place.row].max(size.y);
            }
            across[count] = (place.column, place.columns, size.x);
            down[count] = (place.row, place.rows, size.y);
            count += 1;
        }
        size_tracks(
            &mut widths,
            column_count,
            &across[..count],
            self.columns,
            room.map(|room| room.x),
        );
        size_tracks(
            &mut heights,
            row_count,
            &down[..count],
            self.rows,
            room.map(|room| room.y),
        );
        let mut grid = Grid {
            columns: [0.0; MAX_TRACKS + 1],
            rows: [0.0; MAX_TRACKS + 1],
            size: (column_count, row_count),
        };
        for index in 0..MAX_TRACKS {
            grid.columns[index + 1] = grid.columns[index] + widths[index];
            grid.rows[index + 1] = grid.rows[index] + heights[index];
        }
        grid
    }

    /// The size the group asks for: its tracks at their natural sizes.
    pub fn size(&self, ui: &mut Ui<'_>, cells: &[GroupCell<'_>]) -> Vec2 {
        let places = places(self.vertical, cells);
        let grid = self.grid(ui, cells, &places, None);
        Vec2::new(grid.columns[grid.size.0], grid.rows[grid.size.1])
    }

    /// Shows the cells in `rect`, their tracks stretched or shrunk to fill it, and returns
    /// what happened, by cell index.
    pub fn show(self, ui: &mut Ui<'_>, rect: Rect, cells: &mut [GroupCell<'_>]) -> GroupResponse {
        debug_assert!(
            cells.len() <= MAX_CELLS,
            "a group shows at most {MAX_CELLS} cells"
        );
        let places = places(self.vertical, cells);
        let grid = self.grid(ui, cells, &places, Some(rect.size()));
        let styles = ui.styles();
        let radius = self
            .corner_radius
            .as_ref()
            .map_or(styles.button.corner_radius, |prop| {
                prop.resolve(&styles.tokens)
            });
        let width = self.border(ui);
        let snap = |value: f32| value.round();
        let (count_x, count_y) = grid.size;
        let mut response = GroupResponse::default();
        let mut frames = [(
            Rect::ZERO,
            Corners::ZERO,
            ControlState::Normal,
            Color::TRANSPARENT,
        ); MAX_CELLS];
        let mut count = 0;
        for (index, (cell, place)) in cells.iter_mut().zip(places).enumerate() {
            let Some(place) = place else {
                continue;
            };
            let bounds = Rect::from_min_max(
                Vec2::new(
                    snap(rect.min.x + grid.columns[place.column]),
                    snap(rect.min.y + grid.rows[place.row]),
                ),
                Vec2::new(
                    snap(rect.min.x + grid.columns[place.column + place.columns]),
                    snap(rect.min.y + grid.rows[place.row + place.rows]),
                ),
            );
            let button = &mut cell.button;
            let raw = ui.interact(button.id, bounds);
            let reacted = button.react(ui, raw);
            if reacted.clicked {
                response.clicked = Some(index);
            }
            if reacted.double_clicked {
                response.double_clicked = Some(index);
            }
            if reacted.secondary_pressed {
                response.secondary_pressed = Some(index);
            }
            if reacted.hovered {
                response.hovered = Some(index);
            }
            response.changed |= reacted.changed;
            let state = ControlState::of(&reacted, button.is_selected(), button.is_enabled());
            // The group's own edges, from the grid: exact wherever the rectangle lies.
            let (left, right) = (place.column == 0, place.column + place.columns == count_x);
            let (top, bottom) = (place.row == 0, place.row + place.rows == count_y);
            let corner = |rounded: bool| if rounded { radius } else { 0.0 };
            let radii = Corners {
                top_left: corner(left && top),
                top_right: corner(right && top),
                bottom_right: corner(right && bottom),
                bottom_left: corner(left && bottom),
            };
            let colors = button.colors(ui);
            ui.scene().push_rect(
                &RectPrimitive::new(bounds, colors.background.get(state)).with_corner_radii(radii),
            );
            // The content keeps clear of the borders on the cell's own sides.
            let sides = (width, if right { width } else { 0.0 });
            button.draw_content(ui, bounds, sides, colors.content.get(state));
            // The outline reaches over the shared lines past the right and bottom edges.
            let frame = Rect::from_min_max(
                bounds.min,
                Vec2::new(
                    if right {
                        bounds.max.x
                    } else {
                        bounds.max.x + width
                    },
                    if bottom {
                        bounds.max.y
                    } else {
                        bounds.max.y + width
                    },
                ),
            );
            frames[count] = (frame, radii, state, colors.border.get(state));
            count += 1;
        }
        // Outlines last, the highest priority on top: it owns every side of its cell.
        let mut order: [usize; MAX_CELLS] = core::array::from_fn(|index| index);
        order[..count].sort_unstable_by_key(|&index| (frames[index].2.priority(), index));
        for &index in &order[..count] {
            let (frame, radii, _, color) = frames[index];
            ui.scene().push_rect(
                &RectPrimitive::new(frame, Color::TRANSPARENT)
                    .with_corner_radii(radii)
                    .with_border(width, color),
            );
        }
        response
    }
}

/// A row of a grid's occupancy: one bit per column.
fn columns_mask(column: usize, columns: usize) -> u32 {
    ((1_u32 << columns) - 1) << column
}

/// `true` when `place` lies inside the grid on free cells of `taken`.
fn is_free(taken: &[u32; MAX_TRACKS], place: Place) -> bool {
    place.column + place.columns <= MAX_TRACKS
        && place.row + place.rows <= MAX_TRACKS
        && taken[place.row..place.row + place.rows]
            .iter()
            .all(|row| row & columns_mask(place.column, place.columns) == 0)
}

/// Marks `place` as taken.
fn take(taken: &mut [u32; MAX_TRACKS], place: Place) {
    for row in &mut taken[place.row..place.row + place.rows] {
        *row |= columns_mask(place.column, place.columns);
    }
}

/// The places of `cells`: placed cells where they ask, then the others, in order, in the next
/// free place along the direction (rows for a horizontal group, columns for a vertical one).
fn places(vertical: bool, cells: &[GroupCell<'_>]) -> Places {
    let mut places = [None; MAX_CELLS];
    let mut taken = [0_u32; MAX_TRACKS];
    let spans = |cell: &GroupCell<'_>, column: usize, row: usize| {
        (
            usize::from(cell.span.0).min(MAX_TRACKS - column),
            usize::from(cell.span.1).min(MAX_TRACKS - row),
        )
    };
    for (place, cell) in places.iter_mut().zip(cells) {
        if let Some((column, row)) = cell.place {
            let column = usize::from(column).min(MAX_TRACKS - 1);
            let row = usize::from(row).min(MAX_TRACKS - 1);
            let (columns, rows) = spans(cell, column, row);
            let at = Place {
                column,
                row,
                columns,
                rows,
            };
            take(&mut taken, at);
            *place = Some(at);
        }
    }
    let mut cursor = 0;
    for (place, cell) in places.iter_mut().zip(cells) {
        if cell.place.is_some() {
            continue;
        }
        let found = (cursor..MAX_TRACKS * MAX_TRACKS).find_map(|index| {
            let (along, across) = (index % MAX_TRACKS, index / MAX_TRACKS);
            let (column, row) = if vertical {
                (across, along)
            } else {
                (along, across)
            };
            let (columns, rows) = spans(cell, column, row);
            let at = Place {
                column,
                row,
                columns,
                rows,
            };
            is_free(&taken, at).then_some((index, at))
        });
        if let Some((index, at)) = found {
            take(&mut taken, at);
            *place = Some(at);
            cursor = index + 1;
        }
    }
    places
}

/// A count of tracks as a divisor.
fn share_of(count: usize) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "at most 16 tracks, exact in f32"
    )]
    let count = count as f32;
    count
}

/// Sizes the first `count` tracks of an axis. `sizes` holds what the cells within one track
/// ask for; `cells` lists `(first track, tracks, size)` for every cell; `tracks` the caller's
/// tracks; `room` the length to fill, if any.
fn size_tracks(
    sizes: &mut [f32; MAX_TRACKS],
    count: usize,
    cells: &[(usize, usize, f32)],
    tracks: &[Track],
    room: Option<f32>,
) {
    let track = |index: usize| tracks.get(index).copied().unwrap_or_default();
    let is_fixed = |index: usize| matches!(track(index), Track::Fixed(_));
    let is_fit = |index: &usize| matches!(track(*index), Track::Fit);
    for (index, size) in sizes.iter_mut().enumerate().take(count) {
        if let Track::Fixed(fixed) = track(index) {
            *size = fixed.max(0.0);
        }
    }
    // A cell across several tracks widens those that are not fixed.
    for &(first, span, need) in cells.iter().filter(|cell| cell.1 > 1) {
        let range = first..first + span;
        let have: f32 = sizes[range.clone()].iter().sum();
        let flexible = range.clone().filter(|index| !is_fixed(*index)).count();
        if need > have && flexible > 0 {
            let extra = (need - have) / share_of(flexible);
            for index in range.filter(|index| !is_fixed(*index)) {
                sizes[index] += extra;
            }
        }
    }
    let Some(room) = room else {
        return;
    };
    let room = room.max(0.0);
    let weight = |index: usize| match track(index) {
        Track::Share(weight) => Some(weight.max(0.0)),
        _ => None,
    };
    let sharing = (0..count).filter(|index| weight(*index).is_some()).count();
    if sharing > 0 {
        let weights: f32 = (0..count).filter_map(weight).sum();
        let others: f32 = (0..count)
            .filter(|index| weight(*index).is_none())
            .map(|index| sizes[index])
            .sum();
        let left = (room - others).max(0.0);
        for (index, size) in sizes.iter_mut().enumerate().take(count) {
            if let Some(weight) = weight(index) {
                *size = if weights > 0.0 {
                    left * weight / weights
                } else {
                    left / share_of(sharing)
                };
            }
        }
    } else {
        let total: f32 = sizes[..count].iter().sum();
        let fitting = (0..count).filter(is_fit).count();
        if room > total && fitting > 0 {
            let extra = (room - total) / share_of(fitting);
            for index in (0..count).filter(is_fit) {
                sizes[index] += extra;
            }
        }
    }
    // Too little room: the fitting tracks give way first, in proportion to their sizes, then
    // every track.
    let total: f32 = sizes[..count].iter().sum();
    if total > room {
        let mut excess = total - room;
        let fitting: f32 = (0..count).filter(is_fit).map(|index| sizes[index]).sum();
        if fitting > 0.0 {
            let cut = excess.min(fitting);
            for index in (0..count).filter(is_fit) {
                sizes[index] -= cut * sizes[index] / fitting;
            }
            excess -= cut;
        }
        let rest: f32 = sizes[..count].iter().sum();
        if excess > 0.0 && rest > 0.0 {
            let scale = room / rest;
            for size in &mut sizes[..count] {
                *size *= scale;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::WidgetId;

    fn cell(name: &'static str) -> GroupCell<'static> {
        GroupCell::new(Button::new(WidgetId::new(name)))
    }

    fn place(column: usize, row: usize, columns: usize, rows: usize) -> Option<Place> {
        let place = Place {
            column,
            row,
            columns,
            rows,
        };
        (columns > 0 && rows > 0).then_some(place)
    }

    #[test]
    fn unplaced_cells_take_the_next_free_place() {
        // A cell across two columns pushes the next one to the third column (it used to land
        // on the second, under the first cell).
        let cells = [cell("a").span(2, 1), cell("b")];
        assert_eq!(
            places(false, &cells)[..2],
            [place(0, 0, 2, 1), place(2, 0, 1, 1)]
        );
        // Stacked, a cell across two rows pushes the next one to the third row.
        let cells = [cell("a").span(1, 2), cell("b")];
        assert_eq!(
            places(true, &cells)[..2],
            [place(0, 0, 1, 2), place(0, 2, 1, 1)]
        );
        // Placed cells come first: the others go around them.
        let cells = [cell("a"), cell("b").at(0, 0), cell("c")];
        assert_eq!(
            places(false, &cells)[..3],
            [place(1, 0, 1, 1), place(0, 0, 1, 1), place(2, 0, 1, 1)]
        );
        // Spans stop at the grid's edge.
        let cells = [cell("a").at(15, 0).span(4, 1)];
        assert_eq!(places(false, &cells)[0], place(15, 0, 1, 1));
    }

    #[test]
    fn a_full_grid_leaves_the_last_cells_out() {
        let cells = [cell("a").span(16, 16), cell("b")];
        let found = places(false, &cells);
        assert_eq!(found[0], place(0, 0, 16, 16));
        assert_eq!(found[1], None);
    }

    fn sized(
        natural: &[f32],
        cells: &[(usize, usize, f32)],
        tracks: &[Track],
        room: Option<f32>,
    ) -> Vec<f32> {
        let mut sizes = [0.0; MAX_TRACKS];
        sizes[..natural.len()].copy_from_slice(natural);
        size_tracks(&mut sizes, natural.len(), cells, tracks, room);
        sizes[..natural.len()].to_vec()
    }

    #[test]
    fn shared_tracks_measure_their_buttons_and_split_the_room_left() {
        let tracks = [Track::Fixed(30.0), Track::Share(1.0), Track::Share(3.0)];
        // Measuring: shares keep what their buttons ask for (they used to measure 0).
        assert_eq!(
            sized(&[0.0, 40.0, 50.0], &[], &tracks, None),
            [30.0, 40.0, 50.0]
        );
        // Laid out: the 120 pixels left go 1 to 3.
        assert_eq!(
            sized(&[0.0, 40.0, 50.0], &[], &tracks, Some(150.0)),
            [30.0, 30.0, 90.0]
        );
    }

    #[test]
    fn fitting_tracks_share_the_extra_room_and_give_way_first() {
        let tracks = [Track::Fit, Track::Fixed(20.0), Track::Fit];
        assert_eq!(
            sized(&[10.0, 0.0, 30.0], &[], &tracks, Some(80.0)),
            [20.0, 20.0, 40.0]
        );
        // 40 pixels for 60: the fitting tracks lose 20, a quarter and three quarters (the
        // group used to overflow its rectangle).
        assert_eq!(
            sized(&[10.0, 0.0, 30.0], &[], &tracks, Some(40.0)),
            [5.0, 20.0, 15.0]
        );
        // Not even room for the fixed track: everything scales down.
        let shrunk = sized(&[10.0, 0.0, 30.0], &[], &tracks, Some(10.0));
        assert!(
            (shrunk.iter().sum::<f32>() - 10.0).abs() < 1e-4,
            "{shrunk:?}"
        );
    }

    #[test]
    fn a_cell_across_tracks_widens_only_those_not_fixed() {
        // A cell 60 wide across a fixed 20 and a fitting 10: the fitting one takes the 30
        // missing (the fixed track used to take half and lose it).
        let tracks = [Track::Fixed(20.0), Track::Fit];
        assert_eq!(
            sized(&[0.0, 10.0], &[(0, 2, 60.0)], &tracks, None),
            [20.0, 40.0]
        );
    }
}
