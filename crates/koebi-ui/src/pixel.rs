//! Placing what is drawn on the physical pixel grid.
//!
//! The renderer moves the edges of rectangles to whole physical pixels, but strokes stay
//! where they are given, anti-aliased. A line one pixel wide therefore covers the pixel it is
//! snapped to, while a chevron centered on the same logical position, at the edge between two
//! pixels, spreads over both: the two do not line up. [`Ui::pixel_span`] gives the whole
//! pixels a line covers and [`Ui::centered_on_line`] centers a shape on the middle of such a
//! line, so that a fold arrow's tip and the tree line below it share their middle at every
//! scale factor.

use koebi_core::{Rect, Vec2};

use crate::Ui;

/// The span a line `width` logical pixels wide covers around `center` at `scale` physical
/// pixels per logical pixel: whole physical pixels, at least one, as near `center` as they
/// can be, ties rounding up as the renderer moves edges (`floor(x + 0.5)`). Returns its start
/// and end in logical pixels.
fn span(center: f32, width: f32, scale: f32) -> (f32, f32) {
    let nearest = |x: f32| (x + 0.5).floor();
    let pixels = nearest(width * scale).max(1.0);
    let start = nearest(center * scale - pixels * 0.5);
    (start / scale, (start + pixels) / scale)
}

/// The middle of the span of a line `width` logical pixels wide around `center`.
fn axis(center: f32, width: f32, scale: f32) -> f32 {
    let (start, end) = span(center, width, scale);
    f32::midpoint(start, end)
}

impl Ui<'_> {
    /// The span of a line `width` logical pixels wide around `center`, in logical pixels: the
    /// whole physical pixels it covers (at least one), as near `center` as they can be. A
    /// rectangle over that span is drawn exactly there.
    pub fn pixel_span(&self, center: f32, width: f32) -> (f32, f32) {
        span(center, width, self.state.scale_factor())
    }

    /// A square of `side` (no larger than `area`) centered in `area`, moved to the center of a
    /// physical pixel: a shape symmetric around its center (a chevron's tip) draws its middle
    /// on whole pixels.
    pub fn pixel_centered(&self, area: Rect, side: f32) -> Rect {
        let pixel = 1.0 / self.state.scale_factor();
        self.centered_on_line(area, side, pixel)
    }

    /// A square of `side` (no larger than `area`) centered in `area`, moved on both axes to the
    /// middle of the line `line_width` wide that [`Ui::pixel_span`] draws through the center
    /// of `area`: a shape symmetric around its center lines up with that line to the pixel.
    pub fn centered_on_line(&self, area: Rect, side: f32, line_width: f32) -> Rect {
        let scale = self.state.scale_factor();
        let side = side.min(area.width()).min(area.height()).max(0.0);
        let center = area.center();
        let center = Vec2::new(
            axis(center.x, line_width, scale),
            axis(center.y, line_width, scale),
        );
        Rect::from_center_size(center, Vec2::splat(side))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_cover_whole_physical_pixels() {
        // One pixel wide around a pixel edge: the pixel after it.
        assert_eq!(span(10.0, 1.0, 1.0), (10.0, 11.0));
        assert_eq!(span(10.4, 1.0, 1.0), (10.0, 11.0));
        assert_eq!(span(10.6, 1.0, 1.0), (10.0, 11.0));
        // Two pixels wide: centered on the nearest edge.
        assert_eq!(span(10.4, 2.0, 1.0), (9.0, 11.0));
        // Thinner than a pixel, a line still covers one.
        assert_eq!(span(10.0, 0.25, 1.0), (10.0, 11.0));
        // Ties round up, on both sides of zero, as the renderer's edges do.
        assert_eq!(span(-2.0, 1.0, 1.0), (-2.0, -1.0));
        assert_eq!(span(-2.5, 2.0, 1.0), (-3.0, -1.0));
        // At 1.5, one logical pixel is two physical pixels wide, on whole physical pixels.
        let (start, end) = span(10.0, 1.0, 1.5);
        assert_eq!((start * 1.5, end * 1.5), (14.0, 16.0));
        // At 1.25, one logical pixel rounds to one physical pixel.
        let (start, end) = span(10.0, 1.0, 1.25);
        assert_eq!((start * 1.25, end * 1.25), (12.0, 13.0));
    }

    #[test]
    fn axes_lie_in_the_middle_of_their_span() {
        // A pixel's center for an odd number of pixels, an edge for an even one.
        assert_eq!(axis(10.0, 1.0, 1.0), 10.5);
        assert_eq!(axis(10.0, 2.0, 1.0), 10.0);
        assert!((axis(10.0, 1.0, 1.25) * 1.25 - 12.5).abs() < 1e-5);
        assert!((axis(10.0, 1.0, 1.5) * 1.5 - 15.0).abs() < 1e-5);
    }
}
