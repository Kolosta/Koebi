//! [`Segment`]: the pieces paths are made of: lines, quadratic and cubic Béziers, and rational
//! Béziers up to degree 5 (the spans of NURBS, and exact conics such as circles).
//!
//! Every segment is parametrized on `t ∈ [0, 1]`. Rational segments are evaluated in
//! homogeneous coordinates `(x·w, y·w, w)`, with de Casteljau's algorithm, so a NURBS circle is a
//! circle to the last bit, not an approximation. Nothing here allocates.

use crate::solve;
use crate::{Affine, Point, Rect, Vector};

/// The highest degree of a rational segment: NURBS of order 6.
pub const MAX_DEGREE: usize = 5;

/// A rational Bézier: control points with weights, of degree 1 to [`MAX_DEGREE`]. With every
/// weight equal it is an ordinary Bézier.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RationalBez {
    degree: usize,
    points: [Point; MAX_DEGREE + 1],
    weights: [f64; MAX_DEGREE + 1],
}

impl RationalBez {
    /// The rational Bézier through `points` with `weights`, or `None` when the degree is not in
    /// `1..=MAX_DEGREE`, the counts differ, or a weight is not positive and finite.
    pub fn new(points: &[Point], weights: &[f64]) -> Option<Self> {
        let count = points.len();
        if !(2..=MAX_DEGREE + 1).contains(&count) || weights.len() != count {
            return None;
        }
        if weights.iter().any(|w| !(w.is_finite() && *w > 0.0)) {
            return None;
        }
        let mut segment = Self {
            degree: count - 1,
            points: [Point::ORIGIN; MAX_DEGREE + 1],
            weights: [1.0; MAX_DEGREE + 1],
        };
        segment.points[..count].copy_from_slice(points);
        segment.weights[..count].copy_from_slice(weights);
        Some(segment)
    }

    /// Its degree.
    pub const fn degree(&self) -> usize {
        self.degree
    }

    /// Its control points.
    pub fn points(&self) -> &[Point] {
        &self.points[..=self.degree]
    }

    /// The weights of its control points.
    pub fn weights(&self) -> &[f64] {
        &self.weights[..=self.degree]
    }
}

/// A piece of a path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Segment {
    /// A straight line from the first point to the second.
    Line([Point; 2]),
    /// A quadratic Bézier: start, control, end.
    Quad([Point; 3]),
    /// A cubic Bézier: start, two controls, end.
    Cubic([Point; 4]),
    /// A rational Bézier: a NURBS span or an exact conic.
    Rational(RationalBez),
}

/// Up to a dozen parameters in `(0, 1)`, ascending: where a segment's derivative vanishes along
/// an axis.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Params {
    values: [f64; 12],
    len: usize,
}

impl Params {
    fn push(&mut self, t: f64) {
        if t > 1e-12 && t < 1.0 - 1e-12 && self.len < self.values.len() {
            self.values[self.len] = t;
            self.len += 1;
        }
    }

    pub(crate) fn as_slice(&self) -> &[f64] {
        &self.values[..self.len]
    }

    fn sort(&mut self) {
        self.values[..self.len].sort_unstable_by(f64::total_cmp);
    }
}

/// Homogeneous control points `(x·w, y·w, w)` of any segment.
type Homogeneous = [[f64; 3]; MAX_DEGREE + 1];

fn lerp3(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// De Casteljau on homogeneous points: the point at `t`, and the control points of the halves.
fn de_casteljau(
    control: &Homogeneous,
    degree: usize,
    t: f64,
) -> ([f64; 3], Homogeneous, Homogeneous) {
    let mut work = *control;
    let (mut left, mut right) = ([[0.0; 3]; MAX_DEGREE + 1], [[0.0; 3]; MAX_DEGREE + 1]);
    left[0] = work[0];
    right[degree] = work[degree];
    for level in 1..=degree {
        for i in 0..=degree - level {
            work[i] = lerp3(work[i], work[i + 1], t);
        }
        left[level] = work[0];
        right[degree - level] = work[degree - level];
    }
    (work[0], left, right)
}

/// The control points of the derivative of the homogeneous curve of `degree` (its hodograph),
/// of degree `degree - 1`.
fn hodograph(control: &Homogeneous, degree: usize) -> Homogeneous {
    let mut diff = [[0.0; 3]; MAX_DEGREE + 1];
    #[allow(clippy::cast_precision_loss, reason = "a degree of at most 5")]
    let scale = degree as f64;
    for (slot, pair) in diff.iter_mut().zip(control.windows(2)).take(degree) {
        for k in 0..3 {
            slot[k] = (pair[1][k] - pair[0][k]) * scale;
        }
    }
    diff
}

fn project(h: [f64; 3]) -> Point {
    Point::new(h[0] / h[2], h[1] / h[2])
}

impl Segment {
    /// A line.
    pub const fn line(p0: Point, p1: Point) -> Self {
        Self::Line([p0, p1])
    }

    /// A cubic Bézier.
    pub const fn cubic(p0: Point, p1: Point, p2: Point, p3: Point) -> Self {
        Self::Cubic([p0, p1, p2, p3])
    }

    /// Where it starts.
    pub fn start(&self) -> Point {
        match self {
            Self::Line(p) => p[0],
            Self::Quad(p) => p[0],
            Self::Cubic(p) => p[0],
            Self::Rational(r) => r.points[0],
        }
    }

    /// Where it ends.
    pub fn end(&self) -> Point {
        match self {
            Self::Line(p) => p[1],
            Self::Quad(p) => p[2],
            Self::Cubic(p) => p[3],
            Self::Rational(r) => r.points[r.degree],
        }
    }

    /// Its degree and homogeneous control points.
    fn homogeneous(&self) -> (usize, Homogeneous) {
        let mut h = [[0.0; 3]; MAX_DEGREE + 1];
        let fill = |h: &mut Homogeneous, points: &[Point], weights: &[f64]| {
            for (slot, (p, w)) in h.iter_mut().zip(points.iter().zip(weights)) {
                *slot = [p.x * w, p.y * w, *w];
            }
        };
        match self {
            Self::Line(p) => {
                fill(&mut h, p, &[1.0; 2]);
                (1, h)
            }
            Self::Quad(p) => {
                fill(&mut h, p, &[1.0; 3]);
                (2, h)
            }
            Self::Cubic(p) => {
                fill(&mut h, p, &[1.0; 4]);
                (3, h)
            }
            Self::Rational(r) => {
                fill(&mut h, r.points(), r.weights());
                (r.degree, h)
            }
        }
    }

    /// A segment of the same kind from homogeneous control points.
    fn with_homogeneous(&self, h: &Homogeneous) -> Self {
        let p = |i: usize| project(h[i]);
        match self {
            Self::Line(_) => Self::Line([p(0), p(1)]),
            Self::Quad(_) => Self::Quad([p(0), p(1), p(2)]),
            Self::Cubic(_) => Self::Cubic([p(0), p(1), p(2), p(3)]),
            Self::Rational(r) => {
                let mut out = *r;
                let slots = out.points.iter_mut().zip(out.weights.iter_mut()).zip(h);
                for ((point, weight), homogeneous) in slots.take(r.degree + 1) {
                    *point = project(*homogeneous);
                    *weight = homogeneous[2];
                }
                out.normalize_weights();
                Self::Rational(out)
            }
        }
    }

    /// The point at `t`.
    pub fn eval(&self, t: f64) -> Point {
        match self {
            Self::Line([a, b]) => a.lerp(*b, t),
            Self::Quad([a, b, c]) => {
                let mt = 1.0 - t;
                let (wa, wb, wc) = (mt * mt, 2.0 * mt * t, t * t);
                Point::new(
                    wa * a.x + wb * b.x + wc * c.x,
                    wa * a.y + wb * b.y + wc * c.y,
                )
            }
            Self::Cubic([a, b, c, d]) => {
                let mt = 1.0 - t;
                let (wa, wb, wc, wd) =
                    (mt * mt * mt, 3.0 * mt * mt * t, 3.0 * mt * t * t, t * t * t);
                Point::new(
                    wa * a.x + wb * b.x + wc * c.x + wd * d.x,
                    wa * a.y + wb * b.y + wc * c.y + wd * d.y,
                )
            }
            Self::Rational(_) => {
                let (degree, h) = self.homogeneous();
                project(de_casteljau(&h, degree, t).0)
            }
        }
    }

    /// The derivative with respect to `t` at `t`: the tangent, scaled by the parametrization's
    /// speed.
    pub fn derivative(&self, t: f64) -> Vector {
        match self {
            Self::Line([a, b]) => *b - *a,
            Self::Quad([a, b, c]) => 2.0 * ((1.0 - t) * (*b - *a) + t * (*c - *b)),
            Self::Cubic([a, b, c, d]) => {
                let mt = 1.0 - t;
                3.0 * (mt * mt * (*b - *a) + 2.0 * mt * t * (*c - *b) + t * t * (*d - *c))
            }
            Self::Rational(_) => {
                let (degree, h) = self.homogeneous();
                let (point, _, _) = de_casteljau(&h, degree, t);
                // The hodograph of the homogeneous curve, of degree n - 1.
                let diff = hodograph(&h, degree);
                let (d, _, _) = if degree == 1 {
                    (diff[0], diff, diff)
                } else {
                    de_casteljau(&diff, degree - 1, t)
                };
                let w = point[2];
                Vector::new(
                    (d[0] * w - point[0] * d[2]) / (w * w),
                    (d[1] * w - point[1] * d[2]) / (w * w),
                )
            }
        }
    }

    /// The second derivative with respect to `t` at `t`.
    pub fn second_derivative(&self, t: f64) -> Vector {
        match self {
            Self::Line(_) => Vector::ZERO,
            Self::Quad([a, b, c]) => 2.0 * ((*c - *b) - (*b - *a)),
            Self::Cubic([a, b, c, d]) => {
                let (first, second) = ((*c - *b) - (*b - *a), (*d - *c) - (*c - *b));
                6.0 * ((1.0 - t) * first + t * second)
            }
            Self::Rational(_) => {
                // With H = w·P: P' = (H' - w'·P) / w and P'' = (H'' - 2·w'·P' - w''·P) / w.
                let (degree, h) = self.homogeneous();
                let first = hodograph(&h, degree);
                let (h0, _, _) = de_casteljau(&h, degree, t);
                let (h1, _, _) = de_casteljau(&first, degree - 1, t);
                let h2 = if degree >= 2 {
                    de_casteljau(&hodograph(&first, degree - 1), degree - 2, t).0
                } else {
                    [0.0; 3]
                };
                let w = h0[2];
                let p = Vector::new(h0[0] / w, h0[1] / w);
                let d1 = Vector::new((h1[0] - h1[2] * p.x) / w, (h1[1] - h1[2] * p.y) / w);
                Vector::new(
                    (h2[0] - 2.0 * h1[2] * d1.x - h2[2] * p.x) / w,
                    (h2[1] - 2.0 * h1[2] * d1.y - h2[2] * p.y) / w,
                )
            }
        }
    }

    /// The unit tangent at `t`, also where the derivative vanishes (a control point on an end
    /// point): then the direction in which the curve leaves. `None` only for a segment reduced
    /// to a point.
    pub fn tangent(&self, t: f64) -> Option<Vector> {
        if let Some(tangent) = self.derivative(t).normalized() {
            return Some(tangent);
        }
        // The derivative vanishes at t, but the curve still runs along a direction: the one of
        // a short chord from t toward the inside of the segment.
        for step in [1e-7, 1e-5, 1e-3] {
            let chord = if t + step <= 1.0 {
                self.eval(t + step) - self.eval(t)
            } else {
                self.eval(t) - self.eval(t - step)
            };
            if let Some(tangent) = chord.normalized() {
                return Some(tangent);
            }
        }
        (self.end() - self.start()).normalized()
    }

    /// The signed curvature at `t`: positive where the curve turns from x toward y, that is
    /// toward its normal (the tangent turned a quarter that way); 0 on a line, and where the
    /// speed vanishes.
    pub fn curvature(&self, t: f64) -> f64 {
        let first = self.derivative(t);
        let speed = first.length();
        if speed <= f64::MIN_POSITIVE || !speed.is_finite() {
            return 0.0;
        }
        first.cross(self.second_derivative(t)) / (speed * speed * speed)
    }

    /// The same curve, run backward.
    #[must_use]
    pub fn reversed(&self) -> Self {
        match self {
            Self::Line([a, b]) => Self::Line([*b, *a]),
            Self::Quad([a, b, c]) => Self::Quad([*c, *b, *a]),
            Self::Cubic([a, b, c, d]) => Self::Cubic([*d, *c, *b, *a]),
            Self::Rational(r) => {
                let mut out = *r;
                out.points[..=r.degree].reverse();
                out.weights[..=r.degree].reverse();
                out.normalize_weights();
                Self::Rational(out)
            }
        }
    }

    /// The segment with its first control point moved to `start`: how pieces computed apart
    /// are joined end to end once they agree within a rounding error.
    #[must_use]
    pub(crate) fn with_start(mut self, start: Point) -> Self {
        match &mut self {
            Self::Line(p) => p[0] = start,
            Self::Quad(p) => p[0] = start,
            Self::Cubic(p) => p[0] = start,
            Self::Rational(r) => r.points[0] = start,
        }
        self
    }

    /// The segment with its last control point moved to `end`.
    #[must_use]
    pub(crate) fn with_end(mut self, end: Point) -> Self {
        match &mut self {
            Self::Line(p) => p[1] = end,
            Self::Quad(p) => p[2] = end,
            Self::Cubic(p) => p[3] = end,
            Self::Rational(r) => r.points[r.degree] = end,
        }
        self
    }

    /// The two halves at `t`.
    #[must_use]
    pub fn split(&self, t: f64) -> (Self, Self) {
        let (degree, h) = self.homogeneous();
        let (_, left, right) = de_casteljau(&h, degree, t);
        (self.with_homogeneous(&left), self.with_homogeneous(&right))
    }

    /// The part between `t0` and `t1` (`t0 < t1`).
    #[must_use]
    pub fn subsegment(&self, t0: f64, t1: f64) -> Self {
        let after = if t0 > 0.0 { self.split(t0).1 } else { *self };
        if t1 >= 1.0 {
            return after;
        }
        let local = if t0 < 1.0 {
            (t1 - t0) / (1.0 - t0)
        } else {
            1.0
        };
        after.split(local.clamp(0.0, 1.0)).0
    }

    /// The segment mapped by `transform`. Affine maps keep rational segments' weights.
    #[must_use]
    pub fn transformed(&self, transform: Affine) -> Self {
        let map = |p: &Point| transform.apply(*p);
        match self {
            Self::Line(p) => Self::Line(p.each_ref().map(map)),
            Self::Quad(p) => Self::Quad(p.each_ref().map(map)),
            Self::Cubic(p) => Self::Cubic(p.each_ref().map(map)),
            Self::Rational(r) => {
                let mut out = *r;
                for point in &mut out.points[..=r.degree] {
                    *point = transform.apply(*point);
                }
                Self::Rational(out)
            }
        }
    }

    /// Whether every control point (and weight) is finite.
    pub fn is_finite(&self) -> bool {
        match self {
            Self::Line(p) => p.iter().all(|p| p.is_finite()),
            Self::Quad(p) => p.iter().all(|p| p.is_finite()),
            Self::Cubic(p) => p.iter().all(|p| p.is_finite()),
            Self::Rational(r) => {
                r.points().iter().all(|p| p.is_finite())
                    && r.weights().iter().all(|w| w.is_finite())
            }
        }
    }

    /// The parameters in `(0, 1)` where the derivative's `axis` component (0 = x, 1 = y)
    /// vanishes: the turning points along that axis.
    pub(crate) fn extrema(&self, axis: usize) -> Params {
        let c = |p: Point| if axis == 0 { p.x } else { p.y };
        let mut params = Params::default();
        match self {
            Self::Line(_) => {}
            Self::Quad([a, b, d]) => {
                let denom = c(*a) - 2.0 * c(*b) + c(*d);
                if denom.abs() > f64::EPSILON * (c(*a).abs() + c(*b).abs() + c(*d).abs() + 1.0) {
                    params.push((c(*a) - c(*b)) / denom);
                }
            }
            Self::Cubic([p0, p1, p2, p3]) => {
                // B'(t)/3 = (p1-p0)(1-t)² + 2(p2-p1)(1-t)t + (p3-p2)t²
                let (d0, d1, d2) = (c(*p1) - c(*p0), c(*p2) - c(*p1), c(*p3) - c(*p2));
                let roots = solve::quadratic(d0 - 2.0 * d1 + d2, 2.0 * (d1 - d0), d0);
                for &t in roots.as_slice() {
                    params.push(t);
                }
            }
            Self::Rational(r) if r.degree == 2 => {
                // A conic: with X = w·x and W the weight, both quadratic, the numerator of the
                // derivative, X'·W - X·W', is quadratic too (its cubic terms cancel).
                let (h, w) = ([0, 1, 2].map(|i| c(r.points[i]) * r.weights[i]), r.weights);
                let power = |v: [f64; 3]| [v[0], 2.0 * (v[1] - v[0]), v[0] - 2.0 * v[1] + v[2]];
                let ([a0, a1, a2], [b0, b1, b2]) = (power(h), power([w[0], w[1], w[2]]));
                let roots = solve::quadratic(
                    a2 * b1 - a1 * b2,
                    2.0 * (a2 * b0 - a0 * b2),
                    a1 * b0 - a0 * b1,
                );
                for &t in roots.as_slice() {
                    params.push(t);
                }
            }
            Self::Rational(_) => {
                // No closed form: sign changes of the derivative on a fine grid, then bisection.
                const STEPS: u32 = 64;
                let component = |t: f64| {
                    let d = self.derivative(t);
                    if axis == 0 { d.x } else { d.y }
                };
                let mut prev_t = 0.0;
                let mut prev = component(0.0);
                for step in 1..=STEPS {
                    let t = f64::from(step) / f64::from(STEPS);
                    let value = component(t);
                    if prev == 0.0 {
                        params.push(prev_t);
                    } else if prev.signum() != value.signum() && value != 0.0 {
                        let (mut lo, mut hi) = (prev_t, t);
                        for _ in 0..60 {
                            let mid = lo.midpoint(hi);
                            if component(mid).signum() == prev.signum() {
                                lo = mid;
                            } else {
                                hi = mid;
                            }
                        }
                        params.push(lo.midpoint(hi));
                    }
                    prev_t = t;
                    prev = value;
                }
            }
        }
        params.sort();
        params
    }

    /// Its tight bounds: the end points and the turning points along each axis.
    pub fn bounds(&self) -> Rect {
        let mut bounds = Rect::EMPTY.including(self.start()).including(self.end());
        for axis in 0..2 {
            for &t in self.extrema(axis).as_slice() {
                bounds = bounds.including(self.eval(t));
            }
        }
        bounds
    }

    /// The length of the control polygon: an upper bound of the arc length, and a scale for
    /// tolerances.
    pub fn hull_length(&self) -> f64 {
        let (degree, h) = self.homogeneous();
        (0..degree)
            .map(|i| project(h[i]).distance(project(h[i + 1])))
            .sum()
    }

    /// The arc length from `t0` to `t1` within `accuracy` (absolute).
    pub fn arc_length_between(&self, t0: f64, t1: f64, accuracy: f64) -> f64 {
        if let Self::Line([a, b]) = self {
            return a.distance(*b) * (t1 - t0).abs();
        }
        solve::integrate(&|t| self.derivative(t).length(), t0, t1, accuracy)
    }

    /// Its arc length, within a billionth of its control polygon's length.
    pub fn arc_length(&self) -> f64 {
        self.arc_length_between(0.0, 1.0, self.hull_length() * 1e-9)
    }

    /// The parameter at arc length `length` from the start, within `accuracy`. Lengths beyond
    /// the segment clamp to its ends.
    pub fn param_at_length(&self, length: f64, accuracy: f64) -> f64 {
        if length <= 0.0 {
            return 0.0;
        }
        let total = self.arc_length_between(0.0, 1.0, accuracy * 0.1);
        if length >= total {
            return 1.0;
        }
        if let Self::Line(_) = self {
            return length / total;
        }
        let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
        let mut t = length / total;
        for _ in 0..64 {
            let at = self.arc_length_between(0.0, t, accuracy * 0.1);
            let error = at - length;
            if error.abs() <= accuracy {
                return t;
            }
            if error > 0.0 {
                hi = t;
            } else {
                lo = t;
            }
            let speed = self.derivative(t).length();
            let newton = t - error / speed;
            t = if speed > 0.0 && newton > lo && newton < hi {
                newton
            } else {
                lo.midpoint(hi)
            };
        }
        t
    }

    /// The parameter of the point nearest to `point`, and the squared distance to it.
    pub fn nearest(&self, point: Point) -> (f64, f64) {
        const SAMPLES: u32 = 64;
        if let Self::Line([a, b]) = self {
            let d = *b - *a;
            let length = d.length_squared();
            let t = if length > 0.0 {
                ((point - *a).dot(d) / length).clamp(0.0, 1.0)
            } else {
                0.0
            };
            return (t, self.eval(t).distance_squared(point));
        }
        // Sample, then refine each local minimum of the samples, the end samples included, by a
        // golden-section search over its neighbours: the distance is unimodal there for a fine
        // enough grid.
        let distance = |t: f64| self.eval(t).distance_squared(point);
        let at = |i: u32| f64::from(i) / f64::from(SAMPLES);
        let samples: [f64; SAMPLES as usize + 1] = core::array::from_fn(|i| {
            #[allow(clippy::cast_possible_truncation, reason = "at most 64")]
            distance(at(i as u32))
        });
        let mut best = (0.0, samples[0]);
        for i in 0..=SAMPLES {
            let (before, after) = (i.saturating_sub(1), (i + 1).min(SAMPLES));
            let current = samples[i as usize];
            if current > samples[before as usize] || current > samples[after as usize] {
                continue;
            }
            if current < best.1 {
                best = (at(i), current);
            }
            let (mut lo, mut hi) = (at(before), at(after));
            let ratio = 0.618_033_988_749_894_9;
            let (mut x1, mut x2) = (hi - ratio * (hi - lo), lo + ratio * (hi - lo));
            let (mut f1, mut f2) = (distance(x1), distance(x2));
            for _ in 0..80 {
                if f1 < f2 {
                    hi = x2;
                    x2 = x1;
                    f2 = f1;
                    x1 = hi - ratio * (hi - lo);
                    f1 = distance(x1);
                } else {
                    lo = x1;
                    x1 = x2;
                    f1 = f2;
                    x2 = lo + ratio * (hi - lo);
                    f2 = distance(x2);
                }
            }
            let t = lo.midpoint(hi);
            let d = distance(t);
            if d < best.1 {
                best = (t, d);
            }
        }
        best
    }

    /// The signed number of times the ray from `point` toward +x crosses this segment: +1 for
    /// each crossing going toward +y, -1 toward -y. Crossings count on half-open spans, so a
    /// ray through a shared end point counts once.
    pub(crate) fn winding(&self, point: Point) -> i32 {
        if let Self::Line([a, b]) = self {
            return line_winding(*a, *b, point);
        }
        // Quick reject: the ray passes above or below every control point, so the curve.
        let (degree, h) = self.homogeneous();
        let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in h.iter().take(degree + 1) {
            let y = p[1] / p[2];
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        if point.y < min_y || point.y > max_y {
            return 0;
        }
        let extrema = self.extrema(1);
        let mut winding = 0;
        let mut t0: f64 = 0.0;
        let mut y0 = self.start().y;
        let ends = extrema
            .as_slice()
            .iter()
            .copied()
            .chain(core::iter::once(1.0));
        for t1 in ends {
            let y1 = self.eval(t1).y;
            if (y0 <= point.y) != (y1 <= point.y) {
                // A monotone piece crossing the ray's line: find where by bisection.
                let rising = y1 > y0;
                let (mut lo, mut hi) = (t0, t1);
                for _ in 0..64 {
                    let mid = lo.midpoint(hi);
                    if (self.eval(mid).y <= point.y) == rising {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                if self.eval(lo.midpoint(hi)).x > point.x {
                    winding += if rising { 1 } else { -1 };
                }
            }
            t0 = t1;
            y0 = y1;
        }
        winding
    }

    /// Points along the segment, excluding its start, such that the polyline from the start
    /// stays within `tolerance` of the curve. Appended to `out`.
    pub fn flatten(&self, tolerance: f64, out: &mut Vec<Point>) {
        fn flat_enough(segment: &Segment, tolerance: f64) -> bool {
            let (degree, h) = segment.homogeneous();
            let (a, b) = (project(h[0]), project(h[degree]));
            let chord = b - a;
            let length = chord.length();
            (1..degree).all(|i| {
                let p = project(h[i]) - a;
                let distance = if length > 0.0 {
                    chord.cross(p).abs() / length
                } else {
                    p.length()
                };
                distance <= tolerance
            })
        }
        fn recurse(segment: &Segment, tolerance: f64, depth: u32, out: &mut Vec<Point>) {
            if depth == 0 || flat_enough(segment, tolerance) {
                out.push(segment.end());
                return;
            }
            let (left, right) = segment.split(0.5);
            recurse(&left, tolerance, depth - 1, out);
            recurse(&right, tolerance, depth - 1, out);
        }
        recurse(self, tolerance.max(1e-12), 24, out);
    }
}

impl RationalBez {
    /// Scales the weights so that the first is 1: the same curve, with weights that stay in
    /// range through repeated subdivision.
    fn normalize_weights(&mut self) {
        let first = self.weights[0];
        if first > 0.0 && first.is_finite() {
            for w in &mut self.weights[..=self.degree] {
                *w /= first;
            }
        }
    }
}

/// The winding of the ray from `point` toward +x against the line from `a` to `b`.
fn line_winding(a: Point, b: Point, point: Point) -> i32 {
    if (a.y <= point.y) == (b.y <= point.y) {
        return 0;
    }
    let t = (point.y - a.y) / (b.y - a.y);
    let x = a.x + (b.x - a.x) * t;
    if x > point.x {
        if b.y > a.y { 1 } else { -1 }
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_2, PI};

    /// A quarter of the unit circle centered at the origin, from (1, 0) to (0, 1), exact.
    fn quarter_circle() -> Segment {
        Segment::Rational(
            RationalBez::new(
                &[
                    Point::new(1.0, 0.0),
                    Point::new(1.0, 1.0),
                    Point::new(0.0, 1.0),
                ],
                &[1.0, FRAC_1_SQRT_2, 1.0],
            )
            .unwrap(),
        )
    }

    fn sample_bounds(segment: &Segment) -> Rect {
        (0..=20_000).fold(Rect::EMPTY, |bounds, i| {
            bounds.including(segment.eval(f64::from(i) / 20_000.0))
        })
    }

    #[test]
    fn a_rational_quarter_is_an_exact_circle() {
        let quarter = quarter_circle();
        for i in 0..=1000 {
            let p = quarter.eval(f64::from(i) / 1000.0);
            assert!((p.to_vector().length() - 1.0).abs() < 1e-15, "{p:?}");
        }
        let length = quarter.arc_length();
        assert!((length - FRAC_PI_2).abs() < 1e-9 * FRAC_PI_2, "{length}");
        // Its tangent is perpendicular to its radius everywhere.
        for i in 0..=10 {
            let t = f64::from(i) / 10.0;
            assert!(quarter.derivative(t).dot(quarter.eval(t).to_vector()).abs() < 1e-12);
        }
    }

    #[test]
    fn bounds_are_tight_for_every_kind() {
        let cubic = Segment::cubic(
            Point::new(0.0, 0.0),
            Point::new(10.0, -40.0),
            Point::new(30.0, 50.0),
            Point::new(40.0, 0.0),
        );
        let quad = Segment::Quad([
            Point::new(0.0, 0.0),
            Point::new(5.0, 20.0),
            Point::new(10.0, 0.0),
        ]);
        for segment in [cubic, quad, quarter_circle()] {
            let (found, sampled) = (segment.bounds(), sample_bounds(&segment));
            for (a, b) in [(found.min, sampled.min), (found.max, sampled.max)] {
                assert!(a.distance(b) < 1e-6, "{segment:?}: {a:?} vs {b:?}");
            }
        }
    }

    #[test]
    fn splitting_keeps_the_curve() {
        let cubic = Segment::cubic(
            Point::new(0.0, 0.0),
            Point::new(1.0, 3.0),
            Point::new(4.0, -2.0),
            Point::new(5.0, 1.0),
        );
        for segment in [cubic, quarter_circle()] {
            let (left, right) = segment.split(0.3);
            assert!(left.end().distance(segment.eval(0.3)) < 1e-12);
            assert!(right.eval(0.5).distance(segment.eval(0.65)) < 1e-12);
            let middle = segment.subsegment(0.2, 0.7);
            assert!(middle.start().distance(segment.eval(0.2)) < 1e-12);
            assert!(middle.end().distance(segment.eval(0.7)) < 1e-12);
        }
    }

    #[test]
    fn arc_lengths_and_their_inverse_agree() {
        let cubic = Segment::cubic(
            Point::new(0.0, 0.0),
            Point::new(0.0, 100.0),
            Point::new(100.0, 100.0),
            Point::new(100.0, 0.0),
        );
        let mut polyline = Vec::new();
        cubic.flatten(1e-6, &mut polyline);
        let mut previous = cubic.start();
        let reference: f64 = polyline
            .iter()
            .map(|p| {
                let d = p.distance(previous);
                previous = *p;
                d
            })
            .sum();
        let length = cubic.arc_length();
        assert!(
            (length - reference).abs() < 1e-6 * reference,
            "{length} {reference}"
        );
        let t = cubic.param_at_length(length * 0.37, 1e-10);
        let back = cubic.arc_length_between(0.0, t, 1e-12);
        assert!((back - length * 0.37).abs() < 1e-9, "{back}");
        let quarter = quarter_circle();
        let t = quarter.param_at_length(PI / 8.0, 1e-12);
        let p = quarter.eval(t);
        // An arc of length π/8 on the unit circle spans an angle of π/8.
        assert!((p.y.atan2(p.x) - PI / 8.0).abs() < 1e-9, "{p:?}");
    }

    #[test]
    fn the_nearest_point_is_found_near_an_end() {
        // Regression: a minimum between the first two samples was never refined, so the start
        // itself came out as the nearest point.
        let curve = Segment::cubic(
            Point::new(0.0, 0.0),
            Point::new(40.0, 0.0),
            Point::new(40.0, 6.0),
            Point::new(0.0, 6.0),
        );
        let query = Point::new(0.5625, -4.968);
        let (t, squared) = curve.nearest(query);
        let dense = (0..=100_000)
            .map(|i| curve.eval(f64::from(i) / 100_000.0).distance_squared(query))
            .fold(f64::INFINITY, f64::min);
        assert!(t > 0.0 && squared <= dense + 1e-12, "{t} {squared} {dense}");
        // The same near the end.
        let (t, squared) = curve.nearest(Point::new(0.5625, 10.968));
        assert!(t < 1.0 && squared < 4.97 * 4.97, "{t} {squared}");
    }

    #[test]
    fn the_nearest_point_is_found() {
        let quarter = quarter_circle();
        let (t, distance) = quarter.nearest(Point::new(3.0, 3.0));
        assert!(
            (distance.sqrt() - (18.0_f64.sqrt() - 1.0)).abs() < 1e-9,
            "{distance}"
        );
        assert!(
            quarter
                .eval(t)
                .distance(Point::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2))
                < 1e-6
        );
        let line = Segment::line(Point::new(0.0, 0.0), Point::new(10.0, 0.0));
        let (t, distance) = line.nearest(Point::new(4.0, 3.0));
        assert!((t - 0.4).abs() < 1e-15 && (distance - 9.0).abs() < 1e-12);
    }

    #[test]
    fn transforms_map_rational_segments_exactly() {
        let moved = quarter_circle()
            .transformed(Affine::translate(Vector::new(5.0, 5.0)) * Affine::scale(2.0, 2.0));
        for i in 0..=100 {
            let p = moved.eval(f64::from(i) / 100.0);
            assert!((p.distance(Point::new(5.0, 5.0)) - 2.0).abs() < 1e-14);
        }
    }
}
