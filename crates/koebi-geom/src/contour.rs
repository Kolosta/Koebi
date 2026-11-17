//! [`Contours`]: a path's segments, ready to be measured, hit-tested and drawn. Built once per
//! edit by [`crate::Path::contours`]; every query here is allocation-free.

use crate::{Affine, Point, Rect, Segment, Vector};

/// How the inside of overlapping contours is decided, as in SVG.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FillRule {
    /// Inside where the winding number is not zero.
    #[default]
    NonZero,
    /// Inside where the winding number is odd.
    EvenOdd,
}

/// The segments of one subpath, end to end.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Contour {
    segments: Vec<Segment>,
    closed: bool,
}

/// The contours of a path.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Contours {
    contours: Vec<Contour>,
}

/// The point of a contour nearest to a query, from [`Contours::nearest`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nearest {
    /// The contour's index.
    pub contour: usize,
    /// The segment's index in its contour.
    pub segment: usize,
    /// The parameter on that segment.
    pub t: f64,
    /// The point itself.
    pub point: Point,
    /// Its distance to the query.
    pub distance: f64,
}

impl Contour {
    /// A contour of `segments`, closed or not.
    pub const fn new(segments: Vec<Segment>, closed: bool) -> Self {
        Self { segments, closed }
    }

    /// Its segments.
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Whether its end joins its start.
    pub const fn is_closed(&self) -> bool {
        self.closed
    }

    /// Its length.
    pub fn length(&self) -> f64 {
        self.segments.iter().map(Segment::arc_length).sum()
    }

    /// The point and the unit tangent at arc length `length` from the start, clamped to the
    /// contour; `None` when it is empty.
    pub fn point_at_length(&self, length: f64) -> Option<(Point, Vector)> {
        let mut remaining = length.max(0.0);
        let last = self.segments.len().checked_sub(1)?;
        for (index, segment) in self.segments.iter().enumerate() {
            let own = segment.arc_length();
            if remaining <= own || index == last {
                let t = segment.param_at_length(remaining, own.max(1.0) * 1e-12);
                let tangent = segment.derivative(t).normalized().unwrap_or_else(|| {
                    (segment.end() - segment.start())
                        .normalized()
                        .unwrap_or(Vector::new(1.0, 0.0))
                });
                return Some((segment.eval(t), tangent));
            }
            remaining -= own;
        }
        None
    }

    /// The winding of the ray from `point` toward +x, the contour closed by a line if it is
    /// open (as fills treat open subpaths).
    fn winding(&self, point: Point) -> i32 {
        let mut winding: i32 = self.segments.iter().map(|s| s.winding(point)).sum();
        if !self.closed
            && let (Some(first), Some(last)) = (self.segments.first(), self.segments.last())
        {
            winding += Segment::line(last.end(), first.start()).winding(point);
        }
        winding
    }
}

impl Contours {
    /// Removes every contour, keeping the storage.
    pub fn clear(&mut self) {
        self.contours.clear();
    }

    /// Adds `contour`.
    pub fn push(&mut self, contour: Contour) {
        self.contours.push(contour);
    }

    /// The contours.
    pub fn contours(&self) -> &[Contour] {
        &self.contours
    }

    /// Whether there is no segment at all.
    pub fn is_empty(&self) -> bool {
        self.contours.iter().all(|c| c.segments.is_empty())
    }

    /// The tight bounds of every segment.
    pub fn bounds(&self) -> Rect {
        self.contours
            .iter()
            .flat_map(|c| &c.segments)
            .fold(Rect::EMPTY, |bounds, segment| {
                bounds.union(segment.bounds())
            })
    }

    /// The contours mapped by `transform`.
    #[must_use]
    pub fn transformed(&self, transform: Affine) -> Self {
        Self {
            contours: self
                .contours
                .iter()
                .map(|c| Contour {
                    segments: c
                        .segments
                        .iter()
                        .map(|s| s.transformed(transform))
                        .collect(),
                    closed: c.closed,
                })
                .collect(),
        }
    }

    /// The winding number of `point`: how many times the contours turn around it, counted with
    /// their direction.
    pub fn winding(&self, point: Point) -> i32 {
        self.contours.iter().map(|c| c.winding(point)).sum()
    }

    /// Whether `point` lies inside the fill under `rule`.
    pub fn contains(&self, point: Point, rule: FillRule) -> bool {
        let winding = self.winding(point);
        match rule {
            FillRule::NonZero => winding != 0,
            FillRule::EvenOdd => winding % 2 != 0,
        }
    }

    /// The point of the contours nearest to `point`; `None` when there is no segment.
    pub fn nearest(&self, point: Point) -> Option<Nearest> {
        let mut best: Option<Nearest> = None;
        for (contour_index, contour) in self.contours.iter().enumerate() {
            for (segment_index, segment) in contour.segments.iter().enumerate() {
                // A segment whose bounds lie farther than the best so far cannot win.
                if let Some(found) = best {
                    let bounds = segment.bounds().inflated(found.distance);
                    if !bounds.contains(point) {
                        continue;
                    }
                }
                let (t, squared) = segment.nearest(point);
                let distance = squared.sqrt();
                if best.is_none_or(|found| distance < found.distance) {
                    best = Some(Nearest {
                        contour: contour_index,
                        segment: segment_index,
                        t,
                        point: segment.eval(t),
                        distance,
                    });
                }
            }
        }
        best
    }

    /// The distance from `point` to the nearest segment, infinite without segments.
    pub fn distance(&self, point: Point) -> f64 {
        self.nearest(point)
            .map_or(f64::INFINITY, |found| found.distance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Path, Rect};

    fn square(min: f64, max: f64, clockwise: bool) -> Vec<Segment> {
        let mut corners = [
            Point::new(min, min),
            Point::new(max, min),
            Point::new(max, max),
            Point::new(min, max),
        ];
        if !clockwise {
            corners.reverse();
        }
        (0..4)
            .map(|i| Segment::line(corners[i], corners[(i + 1) % 4]))
            .collect()
    }

    #[test]
    fn rules_decide_holes() {
        // An outer square and an inner one: same direction makes a hole only under even-odd;
        // the opposite direction makes a hole under both rules.
        let point = Point::new(5.0, 5.0);
        let mut same = Contours::default();
        same.push(Contour::new(square(0.0, 10.0, true), true));
        same.push(Contour::new(square(3.0, 7.0, true), true));
        assert!(same.contains(point, FillRule::NonZero));
        assert!(!same.contains(point, FillRule::EvenOdd));
        let mut opposite = Contours::default();
        opposite.push(Contour::new(square(0.0, 10.0, true), true));
        opposite.push(Contour::new(square(3.0, 7.0, false), true));
        assert!(!opposite.contains(point, FillRule::NonZero));
        assert!(opposite.contains(Point::new(1.0, 1.0), FillRule::NonZero));
        assert!(!opposite.contains(Point::new(11.0, 1.0), FillRule::NonZero));
    }

    #[test]
    fn rays_through_corners_count_once() {
        let mut square_contours = Contours::default();
        square_contours.push(Contour::new(square(0.0, 10.0, true), true));
        // The ray at the height of the top edge and of a corner.
        assert!(square_contours.contains(Point::new(5.0, 0.0), FillRule::NonZero));
        assert!(!square_contours.contains(Point::new(-1.0, 0.0), FillRule::NonZero));
        assert!(!square_contours.contains(Point::new(-1.0, 10.0), FillRule::NonZero));
    }

    #[test]
    fn curved_contours_contain_what_they_enclose() {
        let ellipse = Path::exact_ellipse(Point::new(0.0, 0.0), Vector::new(10.0, 5.0)).contours();
        assert!(ellipse.contains(Point::new(9.9, 0.0), FillRule::NonZero));
        assert!(!ellipse.contains(Point::new(10.1, 0.0), FillRule::NonZero));
        assert!(ellipse.contains(Point::new(0.0, 4.99), FillRule::EvenOdd));
        assert!(!ellipse.contains(Point::new(7.2, 3.6), FillRule::EvenOdd));
        let bounds = ellipse.bounds();
        assert!(bounds.min.distance(Point::new(-10.0, -5.0)) < 1e-9);
        assert!(bounds.max.distance(Point::new(10.0, 5.0)) < 1e-9);
    }

    #[test]
    fn an_open_subpath_fills_as_if_closed() {
        let mut open = Contours::default();
        let mut segments = square(0.0, 10.0, true);
        segments.pop();
        open.push(Contour::new(segments, false));
        assert!(open.contains(Point::new(5.0, 5.0), FillRule::NonZero));
    }

    #[test]
    fn distances_and_lengths_on_a_circle() {
        let circle = Path::exact_ellipse(Point::new(0.0, 0.0), Vector::new(2.0, 2.0)).contours();
        let distance = circle.distance(Point::new(5.0, 0.0));
        assert!((distance - 3.0).abs() < 1e-9, "{distance}");
        let contour = &circle.contours()[0];
        let length = contour.length();
        assert!(
            (length - 4.0 * core::f64::consts::PI).abs() < 1e-8,
            "{length}"
        );
        let (point, tangent) = contour.point_at_length(length / 2.0).unwrap();
        assert!(point.distance(Point::new(-2.0, 0.0)) < 1e-8, "{point:?}");
        assert!((tangent.length() - 1.0).abs() < 1e-12);
        assert!(Rect::EMPTY.is_empty());
    }
}
