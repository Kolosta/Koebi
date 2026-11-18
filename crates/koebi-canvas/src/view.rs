//! [`View`]: what part of the document a target shows, and at what zoom, in f64.

use koebi_core::Extent;
use koebi_geom::{Octave, Point, Rect};

/// A camera over the document and the size of the target it fills.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    /// The document point at the target's center.
    pub center: Point,
    /// Physical pixels per document unit; positive and finite.
    pub zoom: f64,
    /// The target's size, in physical pixels.
    pub extent: Extent,
}

impl View {
    /// `true` when the view shows something: a zoom and a center that are finite, the zoom
    /// positive, a size that is not empty.
    pub fn is_valid(self) -> bool {
        self.zoom > 0.0
            && self.zoom.is_finite()
            && self.center.x.is_finite()
            && self.center.y.is_finite()
            && !self.extent.is_empty()
    }

    /// The zoom's octave: the cells and the tolerance its outlines are prepared at.
    pub fn octave(self) -> Octave {
        Octave::of(self.zoom)
    }

    /// Where the document point `point` lies on the target, in physical pixels.
    pub fn to_target(self, point: Point) -> [f64; 2] {
        [
            (point.x - self.center.x) * self.zoom + f64::from(self.extent.width) * 0.5,
            (point.y - self.center.y) * self.zoom + f64::from(self.extent.height) * 0.5,
        ]
    }

    /// The document point at `pixel` on the target (physical pixels).
    pub fn to_document(self, pixel: [f64; 2]) -> Point {
        Point::new(
            (pixel[0] - f64::from(self.extent.width) * 0.5) / self.zoom + self.center.x,
            (pixel[1] - f64::from(self.extent.height) * 0.5) / self.zoom + self.center.y,
        )
    }

    /// The part of the document the target shows.
    pub fn area(self) -> Rect {
        Rect::new(
            self.to_document([0.0, 0.0]),
            self.to_document([f64::from(self.extent.width), f64::from(self.extent.height)]),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn views_map_the_document_to_their_target_and_back() {
        let view = View {
            center: Point::new(1.0e6, -3.0),
            zoom: 1.0e9,
            extent: Extent::new(800, 600),
        };
        assert!(view.is_valid());
        let at = view.to_target(Point::new(1.0e6 + 1e-7, -3.0));
        assert!(
            (at[0] - 500.0).abs() < 1e-3 && (at[1] - 300.0).abs() < 1e-9,
            "{at:?}"
        );
        let back = view.to_document(at);
        // Within the spacing of f64 near a million.
        assert!((back.x - (1.0e6 + 1e-7)).abs() < 1e-9);
        let area = view.area();
        assert!((area.width() - 800.0e-9).abs() < 1e-9);
        assert_eq!(view.octave(), Octave::of(1.0e9));
        for invalid in [
            View { zoom: 0.0, ..view },
            View {
                zoom: f64::NAN,
                ..view
            },
            View {
                extent: Extent::new(0, 600),
                ..view
            },
        ] {
            assert!(!invalid.is_valid());
        }
    }
}
