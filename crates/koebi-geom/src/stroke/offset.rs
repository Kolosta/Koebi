//! The geometry of an outline's edges: offset curves approximated by quadratics, exact arcs,
//! joins, and where offset curves cross.
//!
//! An offset curve at distance `d` is `F(t) = Q(t) + d·n(t)`, where `n` is the unit normal (the
//! tangent turned a quarter from x toward y). Its tangent is the curve's, scaled by `1 - d·κ`,
//! so it runs along the curve while `d·κ < 1`: where it does not, the offset folds back on
//! itself (a cusp, then a loop), and the band's edge there is not an offset curve at all.

use crate::{Point, RationalBez, Segment, Vector, solve};

use super::Join;

/// Turns sharper than this between the tangents of two pieces make a corner, in radians.
pub(super) const CORNER: f64 = 1e-6;

/// Beyond this share of the radius of curvature on the concave side, an offset is not fitted
/// with quadratics but built from a polyline (see [`offset`]).
const FOLD: f64 = 0.8;

/// Fitting an offset halves its span at most this many times.
const DEPTH: u32 = 20;

/// Where an offset folds, its span is halved at most this many times to find the stretch
/// that folds, which alone is built from a polyline.
const SPANS: u32 = 5;

/// The unit tangent of `segment` at `t`, with a fallback for a segment reduced to a point.
pub(super) fn tangent(segment: &Segment, t: f64) -> Vector {
    segment.tangent(t).unwrap_or(Vector::new(1.0, 0.0))
}

/// The point at distance `d` along the normal of `segment` at `t`.
pub(super) fn offset_point(segment: &Segment, t: f64, d: f64) -> Point {
    segment.eval(t) + tangent(segment, t).perpendicular() * d
}

/// The derivative of the offset at distance `d`: the curve's, scaled by `1 - d·κ`.
fn offset_derivative(segment: &Segment, t: f64, d: f64) -> Vector {
    segment.derivative(t) * (1.0 - d * segment.curvature(t))
}

/// The signed angle turning `from` into `to`, in `-π..=π`: positive from x toward y.
pub(super) fn turn(from: Vector, to: Vector) -> f64 {
    from.cross(to).atan2(from.dot(to))
}

/// `vector` turned by `angle`, from x toward y.
pub(super) fn rotated(vector: Vector, angle: f64) -> Vector {
    let (sin, cos) = angle.sin_cos();
    Vector::new(
        vector.x * cos - vector.y * sin,
        vector.x * sin + vector.y * cos,
    )
}

/// A contour being written: segments end to end, from a start point.
pub(super) struct Chain<'a> {
    segments: &'a mut Vec<Segment>,
    start: Point,
    cursor: Point,
    /// Gaps up to this are rounding errors: closed by moving the next segment's start.
    snap: f64,
}

impl<'a> Chain<'a> {
    /// A chain appending to `segments` from `start`.
    pub(super) const fn new(segments: &'a mut Vec<Segment>, start: Point, snap: f64) -> Self {
        Self {
            segments,
            start,
            cursor: start,
            snap,
        }
    }

    /// Where the chain is.
    pub(super) const fn cursor(&self) -> Point {
        self.cursor
    }

    /// A line to `to`, unless it is already there.
    pub(super) fn line_to(&mut self, to: Point) {
        if self.cursor.distance(to) > self.snap {
            self.segments.push(Segment::line(self.cursor, to));
            self.cursor = to;
        }
    }

    /// Appends `segment`, joined to the cursor: its start moved there when they differ by a
    /// rounding error, a line between them otherwise.
    pub(super) fn push(&mut self, segment: Segment) {
        let start = segment.start();
        let segment = if self.cursor.distance(start) > self.snap {
            self.line_to(start);
            segment
        } else {
            segment.with_start(self.cursor)
        };
        self.cursor = segment.end();
        self.segments.push(segment);
    }

    /// Closes the chain back to its start.
    pub(super) fn close(&mut self) {
        if self.cursor.distance(self.start) > self.snap {
            self.line_to(self.start);
        } else if let Some(last) = self.segments.pop() {
            self.segments.push(last.with_end(self.start));
        }
        self.cursor = self.start;
    }

    /// The circular arc around `center`, of `radius`, from the direction `from` (the cursor
    /// being there) turning by `sweep`: exact rational quadratics of a quarter turn at most.
    pub(super) fn arc(&mut self, center: Point, radius: f64, from: Vector, sweep: f64) {
        if radius <= 0.0 || sweep == 0.0 {
            return;
        }
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a whole turn makes 4 quarters"
        )]
        let quarters = (sweep.abs() / core::f64::consts::FRAC_PI_2 - 1e-9)
            .ceil()
            .clamp(1.0, 8.0) as u32;
        let step = sweep / f64::from(quarters);
        let weight = (step * 0.5).cos();
        let mut u = from;
        for _ in 0..quarters {
            let v = rotated(u, step);
            let a = center + u * radius;
            let b = center + v * radius;
            // The corner of the tangents at both ends.
            let control = center + (u + v) * (radius / (1.0 + u.dot(v)));
            match RationalBez::new(&[a, control, b], &[1.0, weight, 1.0]) {
                Some(arc) => self.push(Segment::Rational(arc)),
                None => self.line_to(b),
            }
            u = v;
        }
    }
}

/// A corner of the path, where a band's edges meet by a join or a cut.
#[derive(Clone, Copy, Debug)]
pub(super) struct Corner {
    /// The corner point.
    pub(super) point: Point,
    /// The normal of the piece arriving, and of the piece leaving.
    pub(super) from: Vector,
    pub(super) to: Vector,
    /// The turn from the arriving tangent to the leaving one, in `-π..=π`.
    pub(super) turn: f64,
}

impl Corner {
    /// Whether the edge at distance `d` lies on the outer side of the turn, where the join is.
    pub(super) fn is_outer(&self, d: f64) -> bool {
        d * self.turn < 0.0
    }

    /// Whether a miter of this turn stays within `limit` (miter length over width): it is
    /// `1 / cos(turn / 2)`.
    fn miter_fits(&self, limit: f64) -> bool {
        let cos = (self.turn * 0.5).cos();
        cos > 0.0 && limit * cos >= 1.0
    }

    /// The directions of the edge at distance `d` before and after the corner, pointing away
    /// from it.
    fn directions(&self, d: f64) -> (Vector, Vector) {
        let sign = d.signum();
        (self.from * sign, self.to * sign)
    }

    /// The direction of the bisector of the edge's directions.
    fn bisector_direction(&self, d: f64) -> Vector {
        rotated(self.directions(d).0, self.turn * 0.5)
    }

    /// Where the join of the edge at distance `d` crosses the corner's bisector: the miter's
    /// tip, the round's middle, or the middle of the bevel.
    pub(super) fn bisector_point(&self, d: f64, join: Join) -> Point {
        let (from, to) = self.directions(d);
        let radius = d.abs();
        match join {
            Join::Round => self.point + self.bisector_direction(d) * radius,
            Join::Miter { limit } if self.miter_fits(limit) => {
                self.point + (from + to) * (radius / (1.0 + from.dot(to)))
            }
            Join::Miter { .. } | Join::Bevel => self.point + (from + to) * (radius * 0.5),
        }
    }

    /// The whole join of the edge at distance `d`, from its end before the corner (the
    /// cursor) to its start after.
    pub(super) fn join(&self, chain: &mut Chain<'_>, d: f64, join: Join) {
        let (from, to) = self.directions(d);
        let end = self.point + to * d.abs();
        match join {
            Join::Round => chain.arc(self.point, d.abs(), from, self.turn),
            Join::Miter { limit } if self.miter_fits(limit) => {
                chain.line_to(self.bisector_point(d, join));
            }
            Join::Miter { .. } | Join::Bevel => {}
        }
        chain.line_to(end);
    }

    /// The first half of the join, up to the bisector.
    pub(super) fn join_to_bisector(&self, chain: &mut Chain<'_>, d: f64, join: Join) {
        if let Join::Round = join {
            chain.arc(self.point, d.abs(), self.directions(d).0, self.turn * 0.5);
        }
        chain.line_to(self.bisector_point(d, join));
    }

    /// The second half of the join, from the bisector (the cursor) on.
    pub(super) fn join_from_bisector(&self, chain: &mut Chain<'_>, d: f64, join: Join) {
        let end = self.point + self.directions(d).1 * d.abs();
        if let Join::Round = join {
            chain.arc(
                self.point,
                d.abs(),
                self.bisector_direction(d),
                self.turn * 0.5,
            );
        }
        chain.line_to(end);
    }

    /// The bisector, as a function that is zero on it and changes sign across it.
    pub(super) fn bisector_side(&self) -> impl Fn(Point) -> f64 + use<> {
        let (point, direction) = (self.point, self.bisector_direction(1.0));
        move |p: Point| (p - point).cross(direction)
    }
}

/// Appends the offset at distance `d` of `segment` over `t0..t1`, within `tolerance`. The
/// cursor is at its start. `points` is scratch.
pub(super) fn offset(
    chain: &mut Chain<'_>,
    segment: &Segment,
    (t0, t1): (f64, f64),
    d: f64,
    tolerance: f64,
    points: &mut Vec<Point>,
) {
    if t1 <= t0 {
        return;
    }
    if d == 0.0 {
        chain.push(segment.subsegment(t0, t1));
        return;
    }
    if let Segment::Line(_) = segment {
        chain.push(Segment::line(
            offset_point(segment, t0, d),
            offset_point(segment, t1, d),
        ));
        return;
    }
    span(chain, segment, (t0, t1), d, tolerance, points, SPANS);
}

/// The offset over `t0..t1`: fitted where it does not fold, built from a polyline where it
/// does, halving the span `depth` more times at most to keep the polyline to the fold.
fn span(
    chain: &mut Chain<'_>,
    segment: &Segment,
    (t0, t1): (f64, f64),
    d: f64,
    tolerance: f64,
    points: &mut Vec<Point>,
    depth: u32,
) {
    let folds = (0..=8).any(|i| {
        let t = t0 + (t1 - t0) * f64::from(i) / 8.0;
        d * segment.curvature(t) > FOLD
    });
    if !folds {
        let (kept, cursor) = (chain.segments.len(), chain.cursor);
        if fit(chain, segment, (t0, t1), d, tolerance, DEPTH) {
            return;
        }
        // The offset folds where the samples did not see it.
        chain.segments.truncate(kept);
        chain.cursor = cursor;
    }
    if depth == 0 {
        polyline(chain, segment, (t0, t1), d, tolerance, points);
    } else {
        let middle = t0.midpoint(t1);
        span(
            chain,
            segment,
            (t0, middle),
            d,
            tolerance,
            points,
            depth - 1,
        );
        span(
            chain,
            segment,
            (middle, t1),
            d,
            tolerance,
            points,
            depth - 1,
        );
    }
}

/// Fits the offset over `a..b` with quadratics whose control point is where the end
/// tangents cross, halving the span where one misses the offset by more than `tolerance`.
/// False when the offset cannot be fitted: it runs backward, or `depth` runs out.
fn fit(
    chain: &mut Chain<'_>,
    segment: &Segment,
    (a, b): (f64, f64),
    d: f64,
    tolerance: f64,
    depth: u32,
) -> bool {
    let split = |chain: &mut Chain<'_>| {
        let middle = a.midpoint(b);
        depth > 0
            && fit(chain, segment, (a, middle), d, tolerance, depth - 1)
            && fit(chain, segment, (middle, b), d, tolerance, depth - 1)
    };
    let (start_tangent, end_tangent) = (tangent(segment, a), tangent(segment, b));
    if start_tangent.dot(end_tangent) < 0.5 {
        // More than 60° of turn for one quadratic.
        return split(chain);
    }
    let (start, end) = (offset_point(segment, a, d), offset_point(segment, b, d));
    let chord = end - start;
    if chord.dot(start_tangent + end_tangent) <= 0.0 {
        // The offset runs backward: it folds here.
        return false;
    }
    let denominator = start_tangent.cross(end_tangent);
    let candidate = if denominator.abs() <= 1e-12 {
        Segment::line(start, end)
    } else {
        let along_start = chord.cross(end_tangent) / denominator;
        let along_end = -chord.cross(start_tangent) / denominator;
        if along_start < 0.0 || along_end < 0.0 {
            return split(chain);
        }
        Segment::Quad([start, start + start_tangent * along_start, end])
    };
    if fit_error(&candidate, segment, (a, b), d) <= tolerance {
        chain.push(candidate);
        true
    } else {
        split(chain)
    }
}

/// How far `candidate` strays from the offset over `a..b`: the largest distance, along the
/// curve's normal at five inner samples, between the offset and where the candidate crosses
/// that normal.
fn fit_error(candidate: &Segment, segment: &Segment, (a, b): (f64, f64), d: f64) -> f64 {
    let (p0, control, p2) = match *candidate {
        Segment::Quad([p0, control, p2]) => (p0, control, p2),
        Segment::Line([p0, p2]) => (p0, p0.midpoint(p2), p2),
        _ => return f64::INFINITY,
    };
    let mut error: f64 = 0.0;
    for i in 1..6 {
        let t = a + (b - a) * f64::from(i) / 6.0;
        let base = segment.eval(t);
        let normal = tangent(segment, t).perpendicular();
        let target = base + normal * d;
        // B(s) - base crossed with the normal is a quadratic in s.
        let c2 = (p0.to_vector() - control.to_vector() * 2.0 + p2.to_vector()).cross(normal);
        let c1 = 2.0 * (control - p0).cross(normal);
        let c0 = (p0 - base).cross(normal);
        let nearest = solve::quadratic(c2, c1, c0)
            .as_slice()
            .iter()
            .filter(|s| (-1e-9..=1.0 + 1e-9).contains(*s))
            .map(|&s| {
                let m = 1.0 - s;
                let on = Point::new(
                    m * m * p0.x + 2.0 * m * s * control.x + s * s * p2.x,
                    m * m * p0.y + 2.0 * m * s * control.y + s * s * p2.y,
                );
                on.distance(target)
            })
            .fold(f64::INFINITY, f64::min);
        error = error.max(nearest);
    }
    error
}

/// The offset over `t0..t1` built from a polyline within `tolerance` of the curve: each chord
/// offset, round joins between chords on the outer side, and on the inner side a pivot through
/// the polyline's vertex. That is the band of the polyline, so the outline stays exact where
/// the curve is tighter than the offset distance and the offset curve folds.
fn polyline(
    chain: &mut Chain<'_>,
    segment: &Segment,
    (t0, t1): (f64, f64),
    d: f64,
    tolerance: f64,
    points: &mut Vec<Point>,
) {
    let part = segment.subsegment(t0, t1);
    points.clear();
    points.push(part.start());
    part.flatten(tolerance, points);
    let mut direction = tangent(segment, t0);
    for index in 1..points.len() {
        let (from, to) = (points[index - 1], points[index]);
        let Some(chord) = (to - from).normalized() else {
            continue;
        };
        bend(chain, from, d, direction, chord);
        chain.line_to(to + chord.perpendicular() * d);
        direction = chord;
    }
    bend(chain, part.end(), d, direction, tangent(segment, t1));
}

/// Turns the edge at distance `d` around `vertex` from the direction `from` to `to`, as the
/// band of a smooth curve does: a round join on the outer side, a pivot on the inner side.
fn bend(chain: &mut Chain<'_>, vertex: Point, d: f64, from: Vector, to: Vector) {
    let angle = turn(from, to);
    let target = vertex + to.perpendicular() * d;
    if angle.abs() > CORNER {
        if d * angle < 0.0 {
            chain.arc(vertex, d.abs(), from.perpendicular() * d.signum(), angle);
        } else {
            chain.line_to(vertex);
        }
    }
    chain.line_to(target);
}

/// Where the offsets at distance `d` of `arriving` (near its end) and of `leaving` (near its
/// start) cross, by Newton's method from the crossing of their end tangents: the parameters
/// `(t, u)` on each. `None` when the crossing is not on the two pieces, or lies where one of
/// the offsets folds.
pub(super) fn crossing(
    arriving: &Segment,
    leaving: &Segment,
    d: f64,
    turn: f64,
) -> Option<(f64, f64)> {
    // For two lines, the crossing is this far from the corner along each.
    let reach = d.abs() * (turn.abs() * 0.5).tan();
    let start = |speed: f64| if speed > 0.0 { reach / speed } else { 0.0 };
    let mut t = (1.0 - start(arriving.derivative(1.0).length())).clamp(0.0, 1.0);
    let mut u = start(leaving.derivative(0.0).length()).clamp(0.0, 1.0);
    for _ in 0..48 {
        let (a, b) = (offset_point(arriving, t, d), offset_point(leaving, u, d));
        let gap = a - b;
        if gap.length() <= 1e-12 * (1.0 + a.x.abs() + a.y.abs() + d.abs()) {
            let regular = |segment: &Segment, t: f64| 1.0 - d * segment.curvature(t) > 0.0;
            let on = (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u);
            return (on && regular(arriving, t) && regular(leaving, u)).then_some((t, u));
        }
        // Solve [A, -B]·(dt, du) = -gap by Cramer's rule.
        let (column_a, column_b) = (
            offset_derivative(arriving, t, d),
            -offset_derivative(leaving, u, d),
        );
        let determinant = column_a.cross(column_b);
        if determinant.abs() <= f64::MIN_POSITIVE {
            return None;
        }
        let rhs = -gap;
        t = (t + rhs.cross(column_b) / determinant).clamp(-0.25, 1.25);
        u = (u + column_a.cross(rhs) / determinant).clamp(-0.25, 1.25);
    }
    None
}

/// The parameter in `t0..t1` where the offset at distance `d` crosses the zero of `side`, a
/// function of the plane that is zero on a line: the crossing nearest to `t1` when `from_end`,
/// to `t0` otherwise. `None` when the offset does not cross it there.
pub(super) fn line_crossing(
    segment: &Segment,
    (t0, t1): (f64, f64),
    d: f64,
    side: impl Fn(Point) -> f64,
    from_end: bool,
) -> Option<f64> {
    const STEPS: u32 = 32;
    let value = |t: f64| side(offset_point(segment, t, d));
    let at = |i: u32| {
        let share = f64::from(i) / f64::from(STEPS);
        if from_end {
            t1 - (t1 - t0) * share
        } else {
            t0 + (t1 - t0) * share
        }
    };
    let (mut near, mut near_value) = (at(0), value(at(0)));
    for i in 1..=STEPS {
        let (far, far_value) = (at(i), value(at(i)));
        if near_value == 0.0 {
            return Some(near);
        }
        if far_value == 0.0 || far_value.signum() != near_value.signum() {
            let (mut lo, mut hi) = (near, far);
            for _ in 0..64 {
                let middle = lo.midpoint(hi);
                if value(middle).signum() == near_value.signum() {
                    lo = middle;
                } else {
                    hi = middle;
                }
            }
            return Some(lo.midpoint(hi));
        }
        (near, near_value) = (far, far_value);
    }
    None
}
