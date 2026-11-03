//! Placing what is drawn on the physical pixel grid.
//!
//! - **Lengths** that lay things out (heights, insets, gaps, paddings) are whole physical
//!   pixels ([`whole_pixels`], applied to every style by [`Styles::resolve_at`]). The renderer
//!   moves each edge to the nearest pixel edge, so two edges a whole number of pixels apart
//!   stay that far apart wherever they lie, while a gap of 2.5 pixels comes out 2 or 3
//!   depending on where it falls. Positions may be anything: only lengths between edges count.
//! - **Lines and chevrons**: strokes stay where they are given, anti-aliased. A line one pixel
//!   wide covers the pixel it is snapped to, while a chevron centered on the same logical
//!   position, at the edge between two pixels, spreads over both: the two do not line up.
//!   [`Ui::pixel_span`] gives the whole pixels a line covers and [`Ui::centered_on_line`]
//!   centers a shape on the middle of such a line, so that a fold arrow's tip and the tree line
//!   below it share their middle at every scale factor.
//!
//! [`Styles::resolve_at`]: crate::Styles::resolve_at

use koebi_core::{Rect, Vec2};

use crate::Ui;

/// `length` logical pixels as a whole number of physical pixels at `scale` physical pixels per
/// logical pixel, in logical pixels: the nearest number, ties going down so that spacing never
/// grows past what was asked, and at least one pixel when `length` is not zero (a gap that
/// exists stays visible). At a scale of 1.25, an inset of 2 logical pixels (2.5 physical)
/// becomes 2 physical pixels on both sides of what it surrounds, wherever that lies.
pub fn whole_pixels(length: f32, scale: f32) -> f32 {
    if length.abs() <= f32::EPSILON || !length.is_finite() || !(scale > 0.0 && scale.is_finite()) {
        return length;
    }
    let pixels = (length.abs() * scale - 0.5 - koebi_render::TIE_TOLERANCE)
        .ceil()
        .max(1.0);
    pixels.copysign(length) / scale
}

/// `position` moved to the physical pixel edge at `scale` where the renderer moves an edge
/// lying there ([`koebi_render::pixel_edge`]).
pub(crate) fn pixel_edge(position: f32, scale: f32) -> f32 {
    if !(scale > 0.0 && scale.is_finite()) {
        return position;
    }
    koebi_render::pixel_edge(position * scale) / scale
}

/// `rect` with its origin on physical pixel edges and its size whole physical pixels at
/// `scale`: its edges lie where the renderer draws them, and its size does not depend on where
/// it lands.
pub(crate) fn snap_rect(rect: Rect, scale: f32) -> Rect {
    let size = |length: f32| {
        if scale > 0.0 && scale.is_finite() {
            (koebi_render::pixel_edge(length * scale) / scale).max(0.0)
        } else {
            length
        }
    };
    Rect::from_origin_size(
        Vec2::new(pixel_edge(rect.min.x, scale), pixel_edge(rect.min.y, scale)),
        Vec2::new(size(rect.width()), size(rect.height())),
    )
}

/// The span a line `width` logical pixels wide covers around `center` at `scale` physical
/// pixels per logical pixel: whole physical pixels, at least one, as near `center` as they
/// can be, ties rounding up as the renderer moves edges (`floor(x + 0.5)`). Returns its start
/// and end in logical pixels.
fn span(center: f32, width: f32, scale: f32) -> (f32, f32) {
    let pixels = koebi_render::pixel_edge(width * scale).max(1.0);
    let start = koebi_render::pixel_edge(center * scale - pixels * 0.5);
    (start / scale, (start + pixels) / scale)
}

/// The middle of the span of a line `width` logical pixels wide around `center`.
fn axis(center: f32, width: f32, scale: f32) -> f32 {
    let (start, end) = span(center, width, scale);
    f32::midpoint(start, end)
}

impl Ui<'_> {
    /// Physical pixels per logical pixel of the window the interface draws in.
    pub fn scale_factor(&self) -> f32 {
        self.state.scale_factor()
    }

    /// `length` as a whole number of physical pixels ([`whole_pixels`]): what a layout adds
    /// between things it places, so that the space is the same everywhere.
    pub fn whole_pixels(&self, length: f32) -> f32 {
        whole_pixels(length, self.state.scale_factor())
    }

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
    fn lengths_become_whole_physical_pixels_ties_going_down() {
        let physical = |length: f32, scale: f32| whole_pixels(length, scale) * scale;
        assert_eq!(physical(2.0, 1.0), 2.0);
        // 2.5 physical pixels: 2, on both sides of what an inset surrounds.
        assert!((physical(2.0, 1.25) - 2.0).abs() < 1e-5);
        assert!((physical(2.0, 1.5) - 3.0).abs() < 1e-5);
        assert!((physical(2.0, 1.75) - 3.0).abs() < 1e-5);
        assert!((physical(24.0, 1.25) - 30.0).abs() < 1e-5);
        assert!((physical(6.0, 1.25) - 7.0).abs() < 1e-5);
        // A gap that exists stays one pixel at least; none stays none.
        assert!((physical(0.25, 1.0) - 1.0).abs() < 1e-5);
        assert_eq!(whole_pixels(0.0, 1.25), 0.0);
        assert!((physical(-2.0, 1.25) + 2.0).abs() < 1e-5);
        // Nothing to round against: unchanged.
        assert_eq!(whole_pixels(3.3, 0.0), 3.3);
        assert!(whole_pixels(f32::INFINITY, 1.25).is_infinite());
    }

    #[test]
    fn edges_a_whole_number_of_pixels_apart_stay_that_far_wherever_they_lie() {
        // What the renderer does to an edge.
        let drawn = |x: f32, scale: f32| (x * scale + 0.5).floor();
        for scale in [1.0, 1.25, 1.5, 1.75, 2.25] {
            let inset = whole_pixels(2.0, scale);
            let height = whole_pixels(24.0, scale) + 2.0 * inset;
            for step in 0..40 {
                let top = 10.0 + step as f32 * 0.137;
                let control = (top + inset, top + height - inset);
                let above = drawn(control.0, scale) - drawn(top, scale);
                let below = drawn(top + height, scale) - drawn(control.1, scale);
                assert_eq!(above, below, "at {scale}, from {top}");
                assert_eq!(above, (inset * scale).round());
            }
        }
    }

    #[test]
    fn rects_snap_their_origin_and_round_their_size() {
        let rect = snap_rect(
            Rect::from_origin_size(Vec2::new(10.3, 7.9), Vec2::new(20.2, 9.6)),
            1.25,
        );
        let physical = |x: f32| x * 1.25;
        assert!((physical(rect.min.x) - 13.0).abs() < 1e-4);
        assert!((physical(rect.min.y) - 10.0).abs() < 1e-4);
        assert!((physical(rect.width()) - 25.0).abs() < 1e-4);
        assert!((physical(rect.height()) - 12.0).abs() < 1e-4);
        assert_eq!(pixel_edge(3.5, 1.0), 4.0);
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
