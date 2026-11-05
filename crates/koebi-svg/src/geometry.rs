//! Geometry of outlines: points, affine transforms, segments (lines, quadratic and cubic
//! Béziers), the conversion of cubics to quadratics within a tolerance, and flattening for
//! strokes.

use crate::error::{Error, Result};

/// A point or a vector.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    /// Across.
    pub x: f32,
    /// Down.
    pub y: f32,
}

impl Point {
    /// The point at `(x, y)`.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub(crate) fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }

    pub(crate) fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }

    pub(crate) fn scale(self, factor: f32) -> Self {
        Self::new(self.x * factor, self.y * factor)
    }

    pub(crate) fn length(self) -> f32 {
        self.x.hypot(self.y)
    }

    pub(crate) fn lerp(self, other: Self, t: f32) -> Self {
        self.add(other.sub(self).scale(t))
    }

    pub(crate) fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    pub(crate) fn cross(self, other: Self) -> f32 {
        self.x * other.y - self.y * other.x
    }

    /// The vector turned a quarter clockwise in screen space (y down), keeping its length.
    pub(crate) fn perpendicular(self) -> Self {
        Self::new(-self.y, self.x)
    }

    pub(crate) fn normalized(self) -> Self {
        let length = self.length();
        if length > 1e-12 {
            self.scale(1.0 / length)
        } else {
            Self::default()
        }
    }
}

/// An affine transform `[a c e; b d f]`, as SVG's `matrix(a b c d e f)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine(pub [f32; 6]);

impl Affine {
    /// The identity.
    pub const IDENTITY: Self = Self([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    /// `self` applied after `other`: `self(other(p))`.
    #[must_use]
    pub fn then(self, other: Self) -> Self {
        // `self.then(other)` reads "other, then self" in the order of SVG's nesting: a parent's
        // transform applies to what its children's give.
        let [a1, b1, c1, d1, e1, f1] = self.0;
        let [a2, b2, c2, d2, e2, f2] = other.0;
        Self([
            a1 * a2 + c1 * b2,
            b1 * a2 + d1 * b2,
            a1 * c2 + c1 * d2,
            b1 * c2 + d1 * d2,
            a1 * e2 + c1 * f2 + e1,
            b1 * e2 + d1 * f2 + f1,
        ])
    }

    /// `point` transformed.
    pub fn apply(self, point: Point) -> Point {
        let [a, b, c, d, e, f] = self.0;
        Point::new(a * point.x + c * point.y + e, b * point.x + d * point.y + f)
    }

    /// How much lengths grow, on average: what a stroke's width becomes.
    pub(crate) fn scale_factor(self) -> f32 {
        let [a, b, c, d, ..] = self.0;
        (a * d - b * c).abs().sqrt()
    }

    /// The inverse, when there is one.
    pub fn inverse(self) -> Option<Self> {
        let [a, b, c, d, e, f] = self.0;
        let determinant = a * d - b * c;
        if determinant.abs() < 1e-12 {
            return None;
        }
        let inv = 1.0 / determinant;
        Some(Self([
            d * inv,
            -b * inv,
            -c * inv,
            a * inv,
            (c * f - d * e) * inv,
            (b * e - a * f) * inv,
        ]))
    }

    fn translate(x: f32, y: f32) -> Self {
        Self([1.0, 0.0, 0.0, 1.0, x, y])
    }

    fn scale(x: f32, y: f32) -> Self {
        Self([x, 0.0, 0.0, y, 0.0, 0.0])
    }

    fn rotate(degrees: f32) -> Self {
        let (sin, cos) = degrees.to_radians().sin_cos();
        Self([cos, sin, -sin, cos, 0.0, 0.0])
    }
}

/// Reads a `transform` list: `matrix`, `translate`, `scale`, `rotate` (about a point too),
/// `skewX`, `skewY`, applied from left to right as SVG does.
pub(crate) fn parse_transform(text: &str, line: usize) -> Result<Affine> {
    let mut result = Affine::IDENTITY;
    let mut rest = text.trim();
    while !rest.is_empty() {
        let open = rest
            .find('(')
            .ok_or_else(|| Error::at(line, format!("malformed transform {text:?}")))?;
        let name = rest[..open].trim().trim_start_matches(',').trim();
        let close = rest[open..]
            .find(')')
            .ok_or_else(|| Error::at(line, format!("malformed transform {text:?}")))?
            + open;
        let numbers = crate::path::numbers(&rest[open + 1..close])
            .ok_or_else(|| Error::at(line, format!("malformed numbers in {text:?}")))?;
        let arity = |count: &[usize]| {
            if count.contains(&numbers.len()) {
                Ok(())
            } else {
                Err(Error::at(
                    line,
                    format!("{name} takes {count:?} numbers in {text:?}"),
                ))
            }
        };
        let step = match name {
            "matrix" => {
                arity(&[6])?;
                Affine([
                    numbers[0], numbers[1], numbers[2], numbers[3], numbers[4], numbers[5],
                ])
            }
            "translate" => {
                arity(&[1, 2])?;
                Affine::translate(numbers[0], numbers.get(1).copied().unwrap_or(0.0))
            }
            "scale" => {
                arity(&[1, 2])?;
                Affine::scale(numbers[0], numbers.get(1).copied().unwrap_or(numbers[0]))
            }
            "rotate" => {
                arity(&[1, 3])?;
                let turn = Affine::rotate(numbers[0]);
                if numbers.len() == 3 {
                    Affine::translate(numbers[1], numbers[2])
                        .then(turn)
                        .then(Affine::translate(-numbers[1], -numbers[2]))
                } else {
                    turn
                }
            }
            "skewX" => {
                arity(&[1])?;
                Affine([1.0, 0.0, numbers[0].to_radians().tan(), 1.0, 0.0, 0.0])
            }
            "skewY" => {
                arity(&[1])?;
                Affine([1.0, numbers[0].to_radians().tan(), 0.0, 1.0, 0.0, 0.0])
            }
            other => {
                return Err(Error::at(line, format!("unknown transform {other:?}")));
            }
        };
        result = result.then(step);
        rest = rest[close + 1..].trim_start();
    }
    Ok(result)
}

/// A segment of an outline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Segment {
    Line(Point, Point),
    Quad(Point, Point, Point),
    Cubic(Point, Point, Point, Point),
}

impl Segment {
    pub(crate) fn transformed(self, by: Affine) -> Self {
        match self {
            Self::Line(a, b) => Self::Line(by.apply(a), by.apply(b)),
            Self::Quad(a, b, c) => Self::Quad(by.apply(a), by.apply(b), by.apply(c)),
            Self::Cubic(a, b, c, d) => {
                Self::Cubic(by.apply(a), by.apply(b), by.apply(c), by.apply(d))
            }
        }
    }

    pub(crate) fn start(self) -> Point {
        match self {
            Self::Line(a, _) | Self::Quad(a, ..) | Self::Cubic(a, ..) => a,
        }
    }

    pub(crate) fn end(self) -> Point {
        match self {
            Self::Line(_, b) | Self::Quad(_, _, b) | Self::Cubic(_, _, _, b) => b,
        }
    }

    /// The segment as quadratic Béziers within `tolerance` of it, added to `out`: a line has
    /// its control point halfway; a cubic is split into pieces each close to a quadratic.
    pub(crate) fn quads(self, tolerance: f32, out: &mut Vec<[Point; 3]>) {
        match self {
            Self::Line(a, b) => out.push([a, a.lerp(b, 0.5), b]),
            Self::Quad(a, b, c) => out.push([a, b, c]),
            Self::Cubic(p0, p1, p2, p3) => {
                // The error of approximating a cubic by one quadratic grows with the third
                // difference; the pieces needed follow its cube root.
                let third = p3.sub(p2.scale(3.0)).add(p1.scale(3.0)).sub(p0).length();
                let error = (3.0_f32).sqrt() / 36.0 * third;
                let pieces = (error / tolerance.max(1e-9)).cbrt().ceil().clamp(1.0, 64.0);
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "between 1 and 64"
                )]
                let count = pieces as usize;
                for index in 0..count {
                    let (from, to) = (index as f32 / pieces, (index + 1) as f32 / pieces);
                    let [q0, q1, q2, q3] = cubic_part(p0, p1, p2, p3, from, to);
                    let control = q1.add(q2).scale(3.0).sub(q0).sub(q3).scale(0.25);
                    out.push([q0, control, q3]);
                }
            }
        }
    }

    /// The segment as points of a polyline within `tolerance` of it, `start` excluded, added
    /// to `out`.
    pub(crate) fn flatten(self, tolerance: f32, out: &mut Vec<Point>) {
        let steps = |length: f32| {
            let count = (length / tolerance.max(1e-9))
                .sqrt()
                .ceil()
                .clamp(1.0, 256.0);
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "between 1 and 256"
            )]
            let count = count as usize;
            count
        };
        match self {
            Self::Line(_, b) => out.push(b),
            Self::Quad(a, b, c) => {
                let curvature = a.sub(b.scale(2.0)).add(c).length();
                let count = steps(curvature * 0.25);
                for index in 1..=count {
                    let t = index as f32 / count as f32;
                    out.push(a.lerp(b, t).lerp(b.lerp(c, t), t));
                }
            }
            Self::Cubic(p0, p1, p2, p3) => {
                let curvature = p0
                    .sub(p1.scale(2.0))
                    .add(p2)
                    .length()
                    .max(p1.sub(p2.scale(2.0)).add(p3).length());
                let count = steps(curvature * 0.75);
                for index in 1..=count {
                    let t = index as f32 / count as f32;
                    out.push(cubic_at(p0, p1, p2, p3, t));
                }
            }
        }
    }
}

/// `curve` split where it turns in y, so that every piece is monotonic in y: what the coverage's
/// ray needs, each piece crossing a horizontal line once at most.
pub(crate) fn monotonic(curve: [Point; 3], out: &mut Vec<[Point; 3]>) {
    let (first, second) = split_at_turn(curve, |point| point.y, |point, y| point.y = y);
    for part in [Some(first), second].into_iter().flatten() {
        let (left, right) = split_at_turn(part, |point| point.x, |point, x| point.x = x);
        out.push(left);
        out.extend(right);
    }
}

/// `curve` split where the coordinate `get` turns back, if it does inside: the control points
/// of both parts set level with the turning point (`set`), so that each part is exactly
/// monotonic in that coordinate.
fn split_at_turn(
    curve: [Point; 3],
    get: impl Fn(Point) -> f32,
    set: impl Fn(&mut Point, f32),
) -> ([Point; 3], Option<[Point; 3]>) {
    let [a, b, c] = curve;
    let denominator = get(a) - 2.0 * get(b) + get(c);
    if denominator.abs() > 1e-12 {
        let t = (get(a) - get(b)) / denominator;
        if t > 1e-6 && t < 1.0 - 1e-6 {
            let mut ab = a.lerp(b, t);
            let mut bc = b.lerp(c, t);
            let middle = ab.lerp(bc, t);
            set(&mut ab, get(middle));
            set(&mut bc, get(middle));
            return ([a, ab, middle], Some([middle, bc, c]));
        }
    }
    (curve, None)
}

fn cubic_at(p0: Point, p1: Point, p2: Point, p3: Point, t: f32) -> Point {
    let a = p0.lerp(p1, t);
    let b = p1.lerp(p2, t);
    let c = p2.lerp(p3, t);
    a.lerp(b, t).lerp(b.lerp(c, t), t)
}

/// The part of a cubic from `from` to `to`.
fn cubic_part(p0: Point, p1: Point, p2: Point, p3: Point, from: f32, to: f32) -> [Point; 4] {
    // Split at `to`, keep the first part, then split that at `from / to`.
    let split = |p0: Point, p1: Point, p2: Point, p3: Point, t: f32| {
        let a = p0.lerp(p1, t);
        let b = p1.lerp(p2, t);
        let c = p2.lerp(p3, t);
        let d = a.lerp(b, t);
        let e = b.lerp(c, t);
        let f = d.lerp(e, t);
        ([p0, a, d, f], [f, e, c, p3])
    };
    let (first, _) = split(p0, p1, p2, p3, to);
    if from <= 0.0 || to <= 0.0 {
        return first;
    }
    let (_, second) = split(first[0], first[1], first[2], first[3], from / to);
    second
}

/// An outline: closed or open subpaths of segments.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Outline {
    pub(crate) subpaths: Vec<Subpath>,
}

/// One subpath: its segments, and whether it closes.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Subpath {
    pub(crate) segments: Vec<Segment>,
    pub(crate) closed: bool,
}

impl Outline {
    pub(crate) fn transformed(&self, by: Affine) -> Self {
        Self {
            subpaths: self
                .subpaths
                .iter()
                .map(|subpath| Subpath {
                    segments: subpath.segments.iter().map(|s| s.transformed(by)).collect(),
                    closed: subpath.closed,
                })
                .collect(),
        }
    }

    /// The outline as quadratics, every subpath closed (a fill closes them).
    pub(crate) fn fill_quads(&self, tolerance: f32) -> Vec<[Point; 3]> {
        let mut out = Vec::new();
        for subpath in &self.subpaths {
            let (Some(first), Some(last)) = (subpath.segments.first(), subpath.segments.last())
            else {
                continue;
            };
            for segment in &subpath.segments {
                segment.quads(tolerance, &mut out);
            }
            if (last.end().sub(first.start())).length() > 1e-6 {
                Segment::Line(last.end(), first.start()).quads(tolerance, &mut out);
            }
        }
        out
    }

    /// The bounding box of the control points.
    pub(crate) fn bounds(&self) -> Option<(Point, Point)> {
        let mut bounds: Option<(Point, Point)> = None;
        let mut take = |point: Point| {
            bounds = Some(match bounds {
                Some((low, high)) => (
                    Point::new(low.x.min(point.x), low.y.min(point.y)),
                    Point::new(high.x.max(point.x), high.y.max(point.y)),
                ),
                None => (point, point),
            });
        };
        for subpath in &self.subpaths {
            for segment in &subpath.segments {
                match *segment {
                    Segment::Line(a, b) => {
                        take(a);
                        take(b);
                    }
                    Segment::Quad(a, b, c) => {
                        take(a);
                        take(b);
                        take(c);
                    }
                    Segment::Cubic(a, b, c, d) => {
                        take(a);
                        take(b);
                        take(c);
                        take(d);
                    }
                }
            }
        }
        bounds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Point, b: Point) -> bool {
        (a.sub(b)).length() < 1e-4
    }

    #[test]
    fn transforms_compose_as_svg_nests_them() {
        let moved = parse_transform("translate(10 0) scale(2)", 1).unwrap();
        assert!(close(
            moved.apply(Point::new(1.0, 1.0)),
            Point::new(12.0, 2.0)
        ));
        let turned = parse_transform("rotate(90 5 5)", 1).unwrap();
        assert!(close(
            turned.apply(Point::new(10.0, 5.0)),
            Point::new(5.0, 10.0)
        ));
        let inverse = moved.inverse().unwrap();
        assert!(close(
            inverse.apply(Point::new(12.0, 2.0)),
            Point::new(1.0, 1.0)
        ));
        assert!(parse_transform("spin(3)", 1).is_err());
        assert!(parse_transform("matrix(1 2)", 1).is_err());
    }

    #[test]
    fn cubics_become_quadratics_close_to_them() {
        let cubic = Segment::Cubic(
            Point::new(0.0, 0.0),
            Point::new(0.0, 1.0),
            Point::new(1.0, 1.0),
            Point::new(1.0, 0.0),
        );
        let mut quads = Vec::new();
        cubic.quads(1e-4, &mut quads);
        assert!(quads.len() > 1);
        assert!(close(quads[0][0], Point::new(0.0, 0.0)));
        assert!(close(quads.last().unwrap()[2], Point::new(1.0, 0.0)));
        // The pieces join, and pass near the cubic's middle.
        for pair in quads.windows(2) {
            assert!(close(pair[0][2], pair[1][0]));
        }
        let middle = cubic_at(
            Point::new(0.0, 0.0),
            Point::new(0.0, 1.0),
            Point::new(1.0, 1.0),
            Point::new(1.0, 0.0),
            0.5,
        );
        assert!((middle.y - 0.75).abs() < 1e-5);
        let mut points = Vec::new();
        cubic.flatten(1e-3, &mut points);
        assert!(points.len() > 4 && close(*points.last().unwrap(), Point::new(1.0, 0.0)));
    }
}
