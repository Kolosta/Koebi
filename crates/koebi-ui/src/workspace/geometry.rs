//! Pixels of the workspace: where areas, gutters and lines land in the window.

use koebi_core::{Rect, Vec2};
use koebi_workspace::{AreaId, GRID, GridRect, Layout, Line, Orientation, to_grid, to_pixel};

/// The grid laid over the window, with a gap around and between areas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Frame {
    origin: Vec2,
    size: Vec2,
    pub(crate) gap: f32,
}

impl Frame {
    /// The grid over `area` (the window), with `gap` pixels around and between areas.
    pub(crate) fn new(area: Rect, gap: f32) -> Self {
        let inner = area.inset(gap * 0.5);
        Self {
            origin: inner.min,
            size: inner.size().max(Vec2::ZERO),
            gap,
        }
    }

    fn x(&self, grid: u32) -> f32 {
        self.origin.x + to_pixel(grid, self.size.x)
    }

    fn y(&self, grid: u32) -> f32 {
        self.origin.y + to_pixel(grid, self.size.y)
    }

    /// The share of the window of a grid rectangle, before the gap.
    pub(crate) fn cell(&self, rect: GridRect) -> Rect {
        Rect::from_min_max(
            Vec2::new(self.x(rect.x0), self.y(rect.y0)),
            Vec2::new(self.x(rect.x1), self.y(rect.y1)),
        )
    }

    /// The panel of an area: its cell inset by half the gap on every side.
    pub(crate) fn panel(&self, rect: GridRect) -> Rect {
        let cell = self.cell(rect);
        let panel = cell.inset(self.gap * 0.5);
        if panel.is_empty() {
            Rect::from_center_size(cell.center(), Vec2::ZERO)
        } else {
            panel
        }
    }

    /// A window point on the grid, if it lies over the workspace.
    pub(crate) fn grid_point(&self, point: Vec2) -> Option<(u32, u32)> {
        let local = point - self.origin;
        let inside = (0.0..self.size.x).contains(&local.x) && (0.0..self.size.y).contains(&local.y);
        inside.then(|| (to_grid(local.x, self.size.x), to_grid(local.y, self.size.y)))
    }

    /// The window length across a line of `orientation`: the width for vertical lines.
    fn across(&self, orientation: Orientation) -> f32 {
        match orientation {
            Orientation::Vertical => self.size.x,
            Orientation::Horizontal => self.size.y,
        }
    }

    /// `pixels` across a line of `orientation`, in grid units.
    pub(crate) fn grid_delta(&self, pixels: f32, orientation: Orientation) -> f64 {
        let across = self.across(orientation);
        if across > 0.0 {
            f64::from(pixels) / f64::from(across) * f64::from(GRID)
        } else {
            0.0
        }
    }

    /// Grid units covering at least `pixels` across a line of `orientation`.
    pub(crate) fn grid_length(&self, pixels: f32, orientation: Orientation) -> u32 {
        koebi_workspace::grid_length(pixels, self.across(orientation))
    }

    /// A strip `thickness` pixels wide along `line`, from `start` to `end` (grid units along
    /// the line), its ends inset by half the gap so that it stays between the panels.
    pub(crate) fn strip(&self, line: &Line, start: u32, end: u32, thickness: f32) -> Rect {
        let half = thickness * 0.5;
        let inset = self.gap * 0.5;
        match line.orientation {
            Orientation::Vertical => {
                let x = self.x(line.position);
                Rect::from_min_max(
                    Vec2::new(x - half, self.y(start) + inset),
                    Vec2::new(x + half, self.y(end) - inset),
                )
            }
            Orientation::Horizontal => {
                let y = self.y(line.position);
                Rect::from_min_max(
                    Vec2::new(self.x(start) + inset, y - half),
                    Vec2::new(self.x(end) - inset, y + half),
                )
            }
        }
    }
}

/// A gutter: the stretch of a line where two areas meet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Segment {
    /// The whole line the gutter lies on.
    pub(crate) line: Line,
    /// The area before it (left or above).
    pub(crate) before: AreaId,
    /// The area after it.
    pub(crate) after: AreaId,
    /// Where the two areas meet along the line, in grid units.
    pub(crate) start: u32,
    pub(crate) end: u32,
}

/// Every gutter of `layout`, into `out`.
pub(crate) fn segments(layout: &Layout, out: &mut Vec<Segment>) {
    out.clear();
    for (before, a) in layout.areas() {
        for (after, b) in layout.areas() {
            let (orientation, position, (start, end)) = if a.x1 == b.x0 {
                (
                    Orientation::Vertical,
                    a.x1,
                    (a.y0.max(b.y0), a.y1.min(b.y1)),
                )
            } else if a.y1 == b.y0 {
                (
                    Orientation::Horizontal,
                    a.y1,
                    (a.x0.max(b.x0), a.x1.min(b.x1)),
                )
            } else {
                continue;
            };
            if start >= end {
                continue;
            }
            let side = match orientation {
                Orientation::Vertical => koebi_workspace::Side::Right,
                Orientation::Horizontal => koebi_workspace::Side::Bottom,
            };
            if let Some(line) = layout.line(before, side) {
                debug_assert_eq!(line.position, position);
                out.push(Segment {
                    line,
                    before,
                    after,
                    start,
                    end,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use koebi_workspace::Side;

    use super::*;

    fn window() -> Rect {
        Rect::from_origin_size(Vec2::ZERO, Vec2::new(804.0, 604.0))
    }

    #[test]
    fn panels_keep_a_gap_around_and_between_them() {
        let frame = Frame::new(window(), 4.0);
        let left = GridRect::new(0, 0, GRID / 2, GRID);
        let right = GridRect::new(GRID / 2, 0, GRID, GRID);
        // The grid covers the window minus half a gap; panels lose another half gap.
        assert_eq!(
            frame.panel(left),
            Rect::from_min_max(Vec2::new(4.0, 4.0), Vec2::new(400.0, 600.0))
        );
        assert_eq!(frame.panel(right).min.x, 404.0);
        assert_eq!(
            frame.grid_point(Vec2::new(2.0 + 400.0, 2.0 + 300.0)),
            Some((GRID / 2, GRID / 2))
        );
        assert_eq!(frame.grid_point(Vec2::new(1.0, 1.0)), None);
        assert!(
            (frame.grid_delta(80.0, Orientation::Vertical) - f64::from(GRID) / 10.0).abs() < 1e-6
        );
        assert!(frame.grid_length(40.0, Orientation::Horizontal) >= GRID / 15);
    }

    #[test]
    fn gutters_lie_where_areas_meet() {
        let layout = Layout::from_areas(&[
            (0, GridRect::new(0, 0, GRID / 2, GRID / 2)),
            (1, GridRect::new(0, GRID / 2, GRID / 2, GRID)),
            (2, GridRect::new(GRID / 2, 0, GRID, GRID)),
        ])
        .unwrap();
        let mut found = Vec::new();
        segments(&layout, &mut found);
        // Left column to the right area (twice), top-left to bottom-left.
        assert_eq!(found.len(), 3);
        let vertical: Vec<_> = found
            .iter()
            .filter(|segment| segment.line.orientation == Orientation::Vertical)
            .collect();
        assert_eq!(vertical.len(), 2);
        // Both vertical gutters lie on the same line, which runs the whole height.
        assert!(
            vertical
                .iter()
                .all(|segment| segment.line == vertical[0].line)
        );
        assert_eq!((vertical[0].line.start, vertical[0].line.end), (0, GRID));
        let row = found
            .iter()
            .find(|segment| segment.line.orientation == Orientation::Horizontal)
            .unwrap();
        assert_eq!((row.start, row.end), (0, GRID / 2));
        assert_eq!(
            layout.line(AreaId::from_saved(0), Side::Bottom),
            Some(row.line)
        );

        let frame = Frame::new(window(), 4.0);
        let strip = frame.strip(&row.line, row.start, row.end, 8.0);
        assert_eq!(strip.height(), 8.0);
        assert_eq!(strip.min.x, 4.0);
    }
}
