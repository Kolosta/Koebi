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
//!   [`Ui::pixel_columns`] gives the whole pixels a line covers and [`Ui::centered_on_line`]
//!   centers a shape on the middle of such a line, so that a fold arrow's tip and the tree line
//!   below it share their middle at every scale factor.
//! - **Spaces**: in a zoomable space ([`Ui::space`]), what is drawn snaps to the pixels the
//!   space's local units land on: the helpers that place drawing ([`Ui::pixel_columns`],
//!   [`Ui::centered_on_line`]) snap through it ([`PixelGrid`]), so that what they place lines up
//!   with what the renderer snaps there. Layout lengths stay those of the window, so that a
//!   component lays itself out the same at every zoom.
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

/// How positions along one axis land on physical pixels: `physical = position * scale +
/// offset`. A zoomable space moves its origin by `offset` physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PixelAxis {
    /// Physical pixels per unit.
    pub(crate) scale: f32,
    /// Where the origin lies, in physical pixels.
    pub(crate) offset: f32,
}

impl PixelAxis {
    /// `scale` physical pixels per unit, the origin on a pixel edge.
    pub(crate) const fn uniform(scale: f32) -> Self {
        Self { scale, offset: 0.0 }
    }

    /// `false` when there is nothing to round against.
    fn is_valid(self) -> bool {
        self.scale > 0.0 && self.scale.is_finite() && self.offset.is_finite()
    }

    /// The position at `physical` pixels.
    fn position(self, physical: f32) -> f32 {
        (physical - self.offset) / self.scale
    }

    /// `position` moved to the physical pixel edge where the renderer moves an edge lying
    /// there ([`koebi_render::pixel_edge`]).
    pub(crate) fn edge(self, position: f32) -> f32 {
        if !self.is_valid() {
            return position;
        }
        self.position(koebi_render::pixel_edge(
            position * self.scale + self.offset,
        ))
    }

    /// The span a line `width` wide covers around `center`: whole physical pixels, at least
    /// one, as near `center` as they can be, ties rounding up as the renderer moves edges
    /// (`floor(x + 0.5)`). Returns its start and end.
    fn span(self, center: f32, width: f32) -> (f32, f32) {
        if !self.is_valid() {
            return (center - width * 0.5, center + width * 0.5);
        }
        let pixels = koebi_render::pixel_edge(width * self.scale).max(1.0);
        let start = koebi_render::pixel_edge(center * self.scale + self.offset - pixels * 0.5);
        (self.position(start), self.position(start + pixels))
    }

    /// The middle of the span of a line `width` wide around `center`.
    fn axis(self, center: f32, width: f32) -> f32 {
        let (start, end) = self.span(center, width);
        f32::midpoint(start, end)
    }
}

/// How positions land on physical pixels, axis by axis: the window's scale factor, through the
/// space in force ([`Ui::pixel_grid`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PixelGrid {
    pub(crate) x: PixelAxis,
    pub(crate) y: PixelAxis,
}

impl PixelGrid {
    /// `scale` physical pixels per unit, the origin on a pixel corner.
    pub(crate) const fn uniform(scale: f32) -> Self {
        Self {
            x: PixelAxis::uniform(scale),
            y: PixelAxis::uniform(scale),
        }
    }

    /// `rect` with each edge on the physical pixel edge where the renderer draws it: what lies
    /// inside lines up with what shows, and rectangles sharing an edge stay joined. A rectangle
    /// whose size is a whole number of pixels keeps it wherever it lands
    /// (`floor(a + n + 0.5) = floor(a + 0.5) + n`), which styles see to at the window's scale.
    pub(crate) fn snap_rect(self, rect: Rect) -> Rect {
        Rect::from_min_max(
            Vec2::new(self.x.edge(rect.min.x), self.y.edge(rect.min.y)),
            Vec2::new(self.x.edge(rect.max.x), self.y.edge(rect.max.y)),
        )
    }
}

/// `position` moved to the physical pixel edge at `scale` where the renderer moves an edge
/// lying there ([`koebi_render::pixel_edge`]).
#[cfg(test)]
fn pixel_edge(position: f32, scale: f32) -> f32 {
    PixelAxis::uniform(scale).edge(position)
}

/// [`PixelGrid::snap_rect`] at `scale` physical pixels per unit.
#[cfg(test)]
fn snap_rect(rect: Rect, scale: f32) -> Rect {
    PixelGrid::uniform(scale).snap_rect(rect)
}

/// [`PixelAxis::span`] at `scale` physical pixels per unit.
#[cfg(test)]
fn span(center: f32, width: f32, scale: f32) -> (f32, f32) {
    PixelAxis::uniform(scale).span(center, width)
}

/// [`PixelAxis::axis`] at `scale` physical pixels per unit.
#[cfg(test)]
fn axis(center: f32, width: f32, scale: f32) -> f32 {
    PixelAxis::uniform(scale).axis(center, width)
}

impl Ui<'_> {
    /// Physical pixels per logical pixel of the window the interface draws in.
    pub fn scale_factor(&self) -> f32 {
        self.state.scale_factor()
    }

    /// How local units land on physical pixels now: the window's scale factor through the
    /// space in force.
    pub(crate) fn pixel_grid(&self) -> PixelGrid {
        let space = self.window_space();
        let factor = self.state.scale_factor();
        let axis = |offset: f32| PixelAxis {
            scale: space.scale * factor,
            offset: offset * factor,
        };
        PixelGrid {
            x: axis(space.offset.x),
            y: axis(space.offset.y),
        }
    }

    /// `length` as a whole number of physical pixels of the window ([`whole_pixels`]): what a
    /// layout adds between things it places, so that the space is the same everywhere. In a
    /// zoomable space too, layouts round at the window's scale factor, as styles do: a
    /// component lays itself out the same at every zoom, and only its drawing snaps to the
    /// pixels the space lands on.
    pub fn whole_pixels(&self, length: f32) -> f32 {
        whole_pixels(length, self.state.scale_factor())
    }

    /// The columns of whole physical pixels a vertical line `width` wide around the x `center`
    /// covers (at least one), as near `center` as they can be, in local units. A rectangle
    /// over that span is drawn exactly there.
    pub fn pixel_columns(&self, center: f32, width: f32) -> (f32, f32) {
        self.pixel_grid().x.span(center, width)
    }

    /// A square of `side` (no larger than `area`) centered in `area`, moved to the center of a
    /// physical pixel: a shape symmetric around its center (a chevron's tip) draws its middle
    /// on whole pixels.
    pub fn pixel_centered(&self, area: Rect, side: f32) -> Rect {
        let scale = self.pixel_grid().x.scale;
        let pixel = if scale > 0.0 { 1.0 / scale } else { 0.0 };
        self.centered_on_line(area, side, pixel)
    }

    /// A square of `side` (no larger than `area`) centered in `area`, moved on both axes to the
    /// middle of the line `line_width` wide that [`Ui::pixel_columns`] draws through the
    /// center of `area`: a shape symmetric around its center lines up with that line to the
    /// pixel.
    pub fn centered_on_line(&self, area: Rect, side: f32, line_width: f32) -> Rect {
        let grid = self.pixel_grid();
        let side = side.min(area.width()).min(area.height()).max(0.0);
        let center = area.center();
        let center = Vec2::new(
            grid.x.axis(center.x, line_width),
            grid.y.axis(center.y, line_width),
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
    fn rects_snap_each_edge_where_the_renderer_draws_it() {
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
    fn positions_in_a_space_snap_where_their_units_land() {
        // A space at 0.3 whose origin lies 10.4 logical pixels in, in a window at 1.25.
        let axis = PixelAxis {
            scale: 0.3 * 1.25,
            offset: 10.4 * 1.25,
        };
        let physical = |position: f32| position * axis.scale + axis.offset;
        let whole = |value: f32| (value - value.round()).abs() < 1e-3;
        for position in [0.0, 3.3, 17.9, -4.2] {
            let edge = axis.edge(position);
            assert!(whole(physical(edge)), "{position}");
            assert!((physical(edge) - physical(position)).abs() <= 0.5 + 1e-3);
        }
        // A line one unit wide, under half a pixel: one whole pixel, its middle at its center.
        let (start, end) = axis.span(20.0, 1.0);
        assert!(whole(physical(start)) && whole(physical(end)));
        assert!((physical(end) - physical(start) - 1.0).abs() < 1e-3);
        let middle = physical(axis.axis(20.0, 1.0));
        assert!((middle - middle.floor() - 0.5).abs() < 1e-3);
        // A rectangle: its origin on an edge, its size whole pixels.
        let grid = PixelGrid { x: axis, y: axis };
        let rect = grid.snap_rect(Rect::from_origin_size(
            Vec2::new(3.3, 7.1),
            Vec2::new(40.0, 13.0),
        ));
        assert!(whole(physical(rect.min.x)) && whole(physical(rect.min.y)));
        assert!(whole(rect.width() * axis.scale) && whole(rect.height() * axis.scale));
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
