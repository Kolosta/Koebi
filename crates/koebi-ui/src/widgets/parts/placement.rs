//! [`Placement`]: where a floating panel opens beside the control that opens it (menus,
//! submenus, dropdowns), how it flips when the window has no room there, and which corners
//! the panel and its control square where they meet.

use koebi_core::{Rect, Vec2};
use koebi_render::Corners;

use crate::widgets::snap;

/// The side of its control a panel opens on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Side {
    /// Below the control.
    #[default]
    Below,
    /// Above it.
    Above,
    /// On its right.
    Right,
    /// On its left.
    Left,
}

impl Side {
    /// The other side of the control.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Below => Self::Above,
            Self::Above => Self::Below,
            Self::Right => Self::Left,
            Self::Left => Self::Right,
        }
    }

    /// `true` for the sides above and below the control.
    pub const fn is_vertical(self) -> bool {
        matches!(self, Self::Below | Self::Above)
    }
}

/// Where a panel lines up along the side of its control.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Alignment {
    /// With the control's left edge (its top edge on the left and right sides).
    #[default]
    Start,
    /// With the control's right edge (its bottom edge on the left and right sides).
    End,
}

impl Alignment {
    /// The other alignment.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Start => Self::End,
            Self::End => Self::Start,
        }
    }
}

/// Where a panel opens beside its control: a side, an alignment along it, and whether the
/// panel may flip to the other side and alignment when the window has no room for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Placement {
    /// The side of the control.
    pub side: Side,
    /// The alignment along that side.
    pub align: Alignment,
    /// Whether the panel moves to the other side or alignment when it does not fit.
    pub flip: bool,
}

impl Default for Placement {
    fn default() -> Self {
        Self::BELOW
    }
}

/// A panel placed beside its control: its rectangle, and the side and alignment it took.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    /// Where the panel lies, on whole pixels.
    pub rect: Rect,
    /// The side it opened on, after flipping.
    pub side: Side,
    /// Its alignment, after flipping.
    pub align: Alignment,
}

impl Placement {
    /// Below the control, aligned with its left edge, flipping when needed: dropdowns.
    pub const BELOW: Self = Self::new(Side::Below);
    /// On the control's right, aligned with its top, flipping when needed: submenus and
    /// vertical dropdowns.
    pub const RIGHT: Self = Self::new(Side::Right);

    /// On `side`, aligned at the start, flipping when needed.
    pub const fn new(side: Side) -> Self {
        Self {
            side,
            align: Alignment::Start,
            flip: true,
        }
    }

    /// The same placement, aligned `align`.
    #[must_use]
    pub const fn aligned(mut self, align: Alignment) -> Self {
        self.align = align;
        self
    }

    /// The same placement, flipping or not.
    #[must_use]
    pub const fn flipping(mut self, flip: bool) -> Self {
        self.flip = flip;
        self
    }

    /// Places a panel of `size` beside `control`, inside `bounds`: on the placement's side when
    /// it fits there, else on the opposite side when that one fits or has more room (and
    /// flipping is allowed); aligned the same way; kept inside `bounds` in the end.
    pub fn place(self, control: Rect, size: Vec2, bounds: Rect) -> Placed {
        // Along the side: the main axis; along the edge: the cross axis.
        let vertical = self.side.is_vertical();
        let (main, cross) = if vertical {
            (size.y, size.x)
        } else {
            (size.x, size.y)
        };
        let (low, high) = if vertical {
            (bounds.min.y, bounds.max.y)
        } else {
            (bounds.min.x, bounds.max.x)
        };
        let (start, end) = if vertical {
            (control.min.y, control.max.y)
        } else {
            (control.min.x, control.max.x)
        };
        // Room after the control (below or right) and before it (above or left).
        let after = high - end;
        let before = start - low;
        let wants_after = matches!(self.side, Side::Below | Side::Right);
        let (room, other) = if wants_after {
            (after, before)
        } else {
            (before, after)
        };
        let flip_side = self.flip && main > room && (main <= other || other > room);
        let side = if flip_side {
            self.side.opposite()
        } else {
            self.side
        };
        let main_origin = if matches!(side, Side::Below | Side::Right) {
            end
        } else {
            start - main
        };

        let (cross_low, cross_high) = if vertical {
            (bounds.min.x, bounds.max.x)
        } else {
            (bounds.min.y, bounds.max.y)
        };
        let (edge_start, edge_end) = if vertical {
            (control.min.x, control.max.x)
        } else {
            (control.min.y, control.max.y)
        };
        let origin_of = |align: Alignment| match align {
            Alignment::Start => edge_start,
            Alignment::End => edge_end - cross,
        };
        let fits = |origin: f32| origin >= cross_low && origin + cross <= cross_high;
        let room_of = |align: Alignment| match align {
            Alignment::Start => cross_high - edge_start,
            Alignment::End => edge_end - cross_low,
        };
        let flip_align = self.flip
            && !fits(origin_of(self.align))
            && (fits(origin_of(self.align.opposite()))
                || room_of(self.align.opposite()) > room_of(self.align));
        let align = if flip_align {
            self.align.opposite()
        } else {
            self.align
        };
        let cross_origin = origin_of(align);

        let keep =
            |origin: f32, length: f32, low: f32, high: f32| origin.min(high - length).max(low);
        let origin = if vertical {
            Vec2::new(
                keep(cross_origin, cross, cross_low, cross_high),
                keep(main_origin, main, low, high),
            )
        } else {
            Vec2::new(
                keep(main_origin, main, low, high),
                keep(cross_origin, cross, cross_low, cross_high),
            )
        };
        Placed {
            rect: snap(Rect::from_origin_size(origin, size)),
            side,
            align,
        }
    }
}

/// Which corners of a rectangle are square, clockwise from the top left.
type Square = [bool; 4];

impl Placed {
    /// The corners where the panel and `control` meet: `(control's, panel's)`. A corner meets
    /// the other rectangle when it lies on the edge they share; nothing meets when the panel
    /// was pushed away from the control to fit the window.
    fn junction(&self, control: Rect) -> (Square, Square) {
        const SLACK: f32 = 0.5;
        let panel = self.rect;
        let near = |a: f32, b: f32| (a - b).abs() <= SLACK;
        let within =
            |value: f32, low: f32, high: f32| value >= low - SLACK && value <= high + SLACK;
        // Along the shared edge: the panel's extent and the control's.
        let (touch, (panel_low, panel_high), (control_low, control_high)) = match self.side {
            Side::Below => (
                near(panel.min.y, control.max.y),
                (panel.min.x, panel.max.x),
                (control.min.x, control.max.x),
            ),
            Side::Above => (
                near(panel.max.y, control.min.y),
                (panel.min.x, panel.max.x),
                (control.min.x, control.max.x),
            ),
            Side::Right => (
                near(panel.min.x, control.max.x),
                (panel.min.y, panel.max.y),
                (control.min.y, control.max.y),
            ),
            Side::Left => (
                near(panel.max.x, control.min.x),
                (panel.min.y, panel.max.y),
                (control.min.y, control.max.y),
            ),
        };
        if !touch {
            return ([false; 4], [false; 4]);
        }
        // The control's corners at each end of the shared edge are covered when the panel
        // reaches past them; the panel's corners when they lie on the control's edge.
        let control_ends = (
            panel_low <= control_low + SLACK,
            panel_high >= control_high - SLACK,
        );
        let panel_ends = (
            within(panel_low, control_low, control_high),
            within(panel_high, control_low, control_high),
        );
        // Corners clockwise from the top left: [top left, top right, bottom right, bottom left].
        let corners = |side: Side, (first, last): (bool, bool)| match side {
            Side::Below => [false, false, last, first],
            Side::Above => [first, last, false, false],
            Side::Right => [false, first, last, false],
            Side::Left => [first, false, false, last],
        };
        (
            corners(self.side, control_ends),
            corners(self.side.opposite(), panel_ends),
        )
    }

    /// `radii` of `control` with the corners where the panel meets it squared.
    pub fn control_corners(&self, control: Rect, radii: Corners) -> Corners {
        square(radii, self.junction(control).0)
    }

    /// `radii` of the panel with the corners where it meets `control` squared.
    pub fn panel_corners(&self, control: Rect, radii: Corners) -> Corners {
        square(radii, self.junction(control).1)
    }
}

/// `radii` with the corners of `square` set to 0.
fn square(radii: Corners, square: Square) -> Corners {
    let pick = |radius: f32, square: bool| if square { 0.0 } else { radius };
    Corners {
        top_left: pick(radii.top_left, square[0]),
        top_right: pick(radii.top_right, square[1]),
        bottom_right: pick(radii.bottom_right, square[2]),
        bottom_left: pick(radii.bottom_left, square[3]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect::from_origin_size(Vec2::new(x, y), Vec2::new(width, height))
    }

    fn window() -> Rect {
        rect(0.0, 0.0, 400.0, 300.0)
    }

    const SIZE: Vec2 = Vec2::new(120.0, 80.0);

    #[test]
    fn panels_open_below_at_the_start_when_they_fit() {
        let placed = Placement::BELOW.place(rect(20.0, 20.0, 80.0, 20.0), SIZE, window());
        assert_eq!(placed.rect, rect(20.0, 40.0, 120.0, 80.0));
        assert_eq!((placed.side, placed.align), (Side::Below, Alignment::Start));
    }

    #[test]
    fn panels_flip_above_and_to_the_end_without_room() {
        // Near the bottom right corner: above, aligned with the control's right edge.
        let control = rect(300.0, 260.0, 80.0, 20.0);
        let placed = Placement::BELOW.place(control, SIZE, window());
        assert_eq!((placed.side, placed.align), (Side::Above, Alignment::End));
        assert_eq!(placed.rect, rect(260.0, 180.0, 120.0, 80.0));
        // Without flipping, the panel stays below and is pushed inside the window.
        let fixed = Placement::BELOW
            .flipping(false)
            .place(control, SIZE, window());
        assert_eq!((fixed.side, fixed.align), (Side::Below, Alignment::Start));
        assert_eq!(fixed.rect, rect(280.0, 220.0, 120.0, 80.0));
    }

    #[test]
    fn panels_on_the_side_flip_left_and_up() {
        let placed = Placement::RIGHT.place(rect(20.0, 20.0, 80.0, 20.0), SIZE, window());
        assert_eq!(placed.rect, rect(100.0, 20.0, 120.0, 80.0));
        let control = rect(320.0, 250.0, 60.0, 20.0);
        let placed = Placement::RIGHT.place(control, SIZE, window());
        assert_eq!((placed.side, placed.align), (Side::Left, Alignment::End));
        assert_eq!(placed.rect, rect(200.0, 190.0, 120.0, 80.0));
    }

    #[test]
    fn a_panel_too_large_for_both_sides_takes_the_larger_one() {
        // 100 pixels below, 180 above, 200 needed: above, the larger room, pushed inside.
        let control = rect(20.0, 180.0, 80.0, 20.0);
        let placed = Placement::BELOW.place(control, Vec2::new(120.0, 200.0), window());
        assert_eq!(placed.side, Side::Above);
        assert_eq!(placed.rect.min.y, 0.0);
    }

    #[test]
    fn junction_corners_are_square_where_panel_and_control_meet() {
        let control = rect(20.0, 20.0, 80.0, 20.0);
        let round = Corners::all(4.0);
        // A wider panel below, aligned at the start: the control's two bottom corners and the
        // panel's top left are square; the panel's top right, past the control, stays round.
        let wide = Placement::BELOW.place(control, SIZE, window());
        let corners = wide.control_corners(control, round);
        assert_eq!(
            (corners.bottom_left, corners.bottom_right, corners.top_left),
            (0.0, 0.0, 4.0)
        );
        let corners = wide.panel_corners(control, round);
        assert_eq!(
            (corners.top_left, corners.top_right, corners.bottom_left),
            (0.0, 4.0, 4.0)
        );
        // As wide as the control: both top corners of the panel are square too.
        let same = Placement::BELOW.place(control, Vec2::new(80.0, 80.0), window());
        let corners = same.panel_corners(control, round);
        assert_eq!((corners.top_left, corners.top_right), (0.0, 0.0));
        // Flipped above: the control's top corners and the panel's bottom ones.
        let low = rect(300.0, 260.0, 80.0, 20.0);
        let above = Placement::BELOW.place(low, SIZE, window());
        let corners = above.control_corners(low, round);
        assert_eq!(
            (corners.top_left, corners.top_right, corners.bottom_left),
            (0.0, 0.0, 4.0)
        );
        let corners = above.panel_corners(low, round);
        assert_eq!(
            (corners.bottom_left, corners.bottom_right, corners.top_left),
            (4.0, 0.0, 4.0)
        );
        // On the right: the control's right corners and the panel's left ones.
        let side = Placement::RIGHT.place(control, SIZE, window());
        let corners = side.control_corners(control, round);
        assert_eq!(
            (corners.top_right, corners.bottom_right, corners.top_left),
            (0.0, 0.0, 4.0)
        );
        assert_eq!(side.panel_corners(control, round).top_left, 0.0);
    }

    #[test]
    fn a_panel_pushed_away_from_its_control_squares_nothing() {
        // Taller than the window: it covers the control, it does not meet it.
        let control = rect(20.0, 140.0, 80.0, 20.0);
        let placed = Placement::BELOW.place(control, Vec2::new(120.0, 300.0), window());
        assert_eq!(
            placed.control_corners(control, Corners::all(4.0)),
            Corners::all(4.0)
        );
    }
}
