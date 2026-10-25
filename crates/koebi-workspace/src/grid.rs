//! The grid areas live on, and its mapping to pixels.

use koebi_core::{Rect, Vec2};

/// Grid units per window side.
pub const GRID: u32 = 1 << 16;

/// The direction of a line: a vertical line has a constant x and separates areas side by side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Orientation {
    /// Constant x: areas left and right of it.
    Vertical,
    /// Constant y: areas above and below it.
    Horizontal,
}

impl Orientation {
    /// The other orientation.
    #[must_use]
    pub fn other(self) -> Self {
        match self {
            Self::Vertical => Self::Horizontal,
            Self::Horizontal => Self::Vertical,
        }
    }
}

/// A side of an area.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// Smaller x.
    Left,
    /// Larger x.
    Right,
    /// Smaller y.
    Top,
    /// Larger y.
    Bottom,
}

impl Side {
    /// Every side, in the order used to break ties.
    pub const ALL: [Self; 4] = [Self::Left, Self::Top, Self::Right, Self::Bottom];

    /// The orientation of the edge on this side: vertical for left and right.
    pub fn orientation(self) -> Orientation {
        match self {
            Self::Left | Self::Right => Orientation::Vertical,
            Self::Top | Self::Bottom => Orientation::Horizontal,
        }
    }

    /// The side facing this one.
    #[must_use]
    pub fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
        }
    }

    /// `true` for the sides with the larger coordinate (right and bottom).
    pub(crate) fn is_end(self) -> bool {
        matches!(self, Self::Right | Self::Bottom)
    }
}

/// A rectangle on the grid, from (`x0`, `y0`) included to (`x1`, `y1`) excluded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridRect {
    /// Left edge.
    pub x0: u32,
    /// Top edge.
    pub y0: u32,
    /// Right edge.
    pub x1: u32,
    /// Bottom edge.
    pub y1: u32,
}

impl GridRect {
    /// The whole grid.
    pub const FULL: Self = Self::new(0, 0, GRID, GRID);

    /// The rectangle from (`x0`, `y0`) to (`x1`, `y1`).
    pub const fn new(x0: u32, y0: u32, x1: u32, y1: u32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    /// Width in grid units (0 when inverted).
    pub const fn width(self) -> u32 {
        self.x1.saturating_sub(self.x0)
    }

    /// Height in grid units (0 when inverted).
    pub const fn height(self) -> u32 {
        self.y1.saturating_sub(self.y0)
    }

    /// Surface in square grid units.
    pub const fn surface(self) -> u64 {
        self.width() as u64 * self.height() as u64
    }

    /// `true` without surface.
    pub const fn is_empty(self) -> bool {
        self.width() == 0 || self.height() == 0
    }

    /// `true` when (`x`, `y`) lies inside (edges at the start included, at the end excluded).
    pub const fn contains(self, x: u32, y: u32) -> bool {
        self.x0 <= x && x < self.x1 && self.y0 <= y && y < self.y1
    }

    /// `true` when the two rectangles share some surface.
    pub const fn overlaps(self, other: Self) -> bool {
        self.x0 < other.x1 && other.x0 < self.x1 && self.y0 < other.y1 && other.y0 < self.y1
    }

    /// The extent along a line of `orientation`: y for vertical lines, x for horizontal ones.
    pub const fn span(self, orientation: Orientation) -> (u32, u32) {
        match orientation {
            Orientation::Vertical => (self.y0, self.y1),
            Orientation::Horizontal => (self.x0, self.x1),
        }
    }

    /// The coordinate of the edge on `side`.
    pub const fn edge(self, side: Side) -> u32 {
        match side {
            Side::Left => self.x0,
            Side::Right => self.x1,
            Side::Top => self.y0,
            Side::Bottom => self.y1,
        }
    }

    /// The rectangle with its edge on `side` moved to `position`.
    #[must_use]
    pub const fn with_edge(mut self, side: Side, position: u32) -> Self {
        match side {
            Side::Left => self.x0 = position,
            Side::Right => self.x1 = position,
            Side::Top => self.y0 = position,
            Side::Bottom => self.y1 = position,
        }
        self
    }

    /// The smallest rectangle holding both.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self::new(
            self.x0.min(other.x0),
            self.y0.min(other.y0),
            self.x1.max(other.x1),
            self.y1.max(other.y1),
        )
    }

    /// The rectangle in pixels for a window of `size`: each edge rounded, so that edges shared
    /// by two areas land on the same pixel.
    pub fn to_pixels(self, size: Vec2) -> Rect {
        Rect::from_min_max(
            Vec2::new(to_pixel(self.x0, size.x), to_pixel(self.y0, size.y)),
            Vec2::new(to_pixel(self.x1, size.x), to_pixel(self.y1, size.y)),
        )
    }
}

/// A grid coordinate in pixels, for a window `size` pixels long, rounded to a whole pixel.
pub fn to_pixel(grid: u32, size: f32) -> f32 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a pixel coordinate within the window, far below f32's range"
    )]
    let pixel = (f64::from(grid) * f64::from(size) / f64::from(GRID)).round() as f32;
    pixel
}

/// A pixel coordinate on the grid, for a window `size` pixels long, clamped to the grid.
pub fn to_grid(pixel: f32, size: f32) -> u32 {
    if size.is_nan() || size <= 0.0 || !pixel.is_finite() {
        return 0;
    }
    let grid = (f64::from(pixel) / f64::from(size) * f64::from(GRID)).round();
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=GRID first"
    )]
    let grid = grid.clamp(0.0, f64::from(GRID)) as u32;
    grid
}

/// Grid units covering at least `pixels` pixels of a window `size` pixels long.
pub fn grid_length(pixels: f32, size: f32) -> u32 {
    if size.is_nan() || size <= 0.0 || pixels.is_nan() || pixels <= 0.0 {
        return 0;
    }
    let grid = (f64::from(pixels) / f64::from(size) * f64::from(GRID)).ceil();
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=GRID first"
    )]
    let grid = grid.clamp(0.0, f64::from(GRID)) as u32;
    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_edges_map_to_the_same_pixel() {
        let size = Vec2::new(1366.0, 768.0);
        let left = GridRect::new(0, 0, 21_845, GRID);
        let right = GridRect::new(21_845, 0, GRID, GRID);
        assert_eq!(left.to_pixels(size).max.x, right.to_pixels(size).min.x);
        assert_eq!(right.to_pixels(size).max, size);
        assert_eq!(to_pixel(GRID / 2, 101.0), 51.0);
    }

    #[test]
    fn pixels_convert_to_the_grid_and_back() {
        assert_eq!(to_grid(683.0, 1366.0), GRID / 2);
        assert_eq!(to_grid(-5.0, 100.0), 0);
        assert_eq!(to_grid(500.0, 100.0), GRID);
        assert_eq!(to_grid(f32::NAN, 100.0), 0);
        assert_eq!(to_grid(10.0, 0.0), 0);
        // A minimum size in pixels never rounds down.
        assert!(to_pixel(grid_length(26.0, 1000.0), 1000.0) >= 26.0);
        assert_eq!(grid_length(0.0, 1000.0), 0);
    }

    #[test]
    fn rects_report_edges_spans_and_overlaps() {
        let rect = GridRect::new(10, 20, 30, 60);
        assert_eq!((rect.width(), rect.height(), rect.surface()), (20, 40, 800));
        assert_eq!(rect.span(Orientation::Vertical), (20, 60));
        assert_eq!(rect.span(Orientation::Horizontal), (10, 30));
        assert_eq!(rect.edge(Side::Bottom), 60);
        assert_eq!(rect.with_edge(Side::Left, 0).x0, 0);
        assert!(rect.contains(10, 20) && !rect.contains(30, 20));
        assert!(rect.overlaps(GridRect::new(29, 59, 40, 70)));
        assert!(!rect.overlaps(GridRect::new(30, 20, 40, 60)));
        assert!(GridRect::new(5, 5, 5, 9).is_empty());
        assert_eq!(Side::Top.opposite(), Side::Bottom);
        assert_eq!(Orientation::Vertical.other(), Orientation::Horizontal);
    }
}
