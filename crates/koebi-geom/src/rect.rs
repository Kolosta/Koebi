//! [`Rect`]: axis-aligned bounds in the document plane, in f64.

use crate::{Affine, Point, Vector};

/// Axis-aligned bounds, from `min` to `max` inclusive. The empty bounds hold nothing and grow
/// from the first point included.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// The smallest x and y.
    pub min: Point,
    /// The largest x and y.
    pub max: Point,
}

impl Rect {
    /// Bounds holding nothing: including a point gives that point's bounds.
    pub const EMPTY: Self = Self {
        min: Point::new(f64::INFINITY, f64::INFINITY),
        max: Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    };

    /// The bounds from `min` to `max`.
    pub const fn new(min: Point, max: Point) -> Self {
        Self { min, max }
    }

    /// The bounds of `origin` and `origin + size`.
    pub fn from_origin_size(origin: Point, size: Vector) -> Self {
        Self::EMPTY.including(origin).including(origin + size)
    }

    /// Whether the bounds hold nothing.
    pub fn is_empty(self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y
    }

    /// The width, 0 when empty.
    pub fn width(self) -> f64 {
        (self.max.x - self.min.x).max(0.0)
    }

    /// The height, 0 when empty.
    pub fn height(self) -> f64 {
        (self.max.y - self.min.y).max(0.0)
    }

    /// The center.
    pub fn center(self) -> Point {
        self.min.midpoint(self.max)
    }

    /// These bounds grown to hold `point`.
    #[must_use]
    pub fn including(self, point: Point) -> Self {
        Self::new(
            Point::new(self.min.x.min(point.x), self.min.y.min(point.y)),
            Point::new(self.max.x.max(point.x), self.max.y.max(point.y)),
        )
    }

    /// The bounds of both.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        if other.is_empty() {
            return self;
        }
        self.including(other.min).including(other.max)
    }

    /// Whether `point` lies inside or on the edge.
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// Whether the two bounds share a point.
    pub fn intersects(self, other: Self) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && self.min.x <= other.max.x
            && other.min.x <= self.max.x
            && self.min.y <= other.max.y
            && other.min.y <= self.max.y
    }

    /// These bounds grown by `margin` on every side.
    #[must_use]
    pub fn inflated(self, margin: f64) -> Self {
        if self.is_empty() {
            return self;
        }
        Self::new(
            self.min - Vector::new(margin, margin),
            self.max + Vector::new(margin, margin),
        )
    }

    /// The bounds of these bounds' corners mapped by `transform`.
    #[must_use]
    pub fn transformed(self, transform: Affine) -> Self {
        if self.is_empty() {
            return self;
        }
        [
            self.min,
            Point::new(self.max.x, self.min.y),
            self.max,
            Point::new(self.min.x, self.max.y),
        ]
        .into_iter()
        .fold(Self::EMPTY, |bounds, corner| {
            bounds.including(transform.apply(corner))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_grow_from_empty() {
        assert!(Rect::EMPTY.is_empty());
        let r = Rect::EMPTY
            .including(Point::new(2.0, 3.0))
            .including(Point::new(-1.0, 5.0));
        assert_eq!(r, Rect::new(Point::new(-1.0, 3.0), Point::new(2.0, 5.0)));
        assert!(r.contains(Point::new(0.0, 4.0)));
        assert_eq!(r.union(Rect::EMPTY), r);
        assert!((r.width() - 3.0).abs() < 1e-15 && (r.height() - 2.0).abs() < 1e-15);
        let turned = r.transformed(Affine::rotate(core::f64::consts::FRAC_PI_2));
        assert!((turned.width() - 2.0).abs() < 1e-12 && (turned.height() - 3.0).abs() < 1e-12);
    }
}
