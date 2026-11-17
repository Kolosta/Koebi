//! NURBS subpaths to rational Bézier segments.
//!
//! A NURBS curve of order `k` (degree `p = k - 1`) is a piecewise rational polynomial. Each span
//! between two distinct knots is one rational Bézier of degree `p`, whose control points are
//! the curve's *blossom* at `(a, …, a, b, …, b)` for the span `[a, b]`. The blossom is
//! evaluated with de Boor's algorithm in homogeneous coordinates, a different argument at each
//! level. This is exact: a NURBS circle gives circular arcs, not approximations.

use crate::segment::{MAX_DEGREE, RationalBez};
use crate::{Point, Segment};

/// How the knots of a NURBS subpath are laid out, as in Blender.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NurbsKnots {
    /// The curve reaches its first and last control points (clamped knots).
    pub endpoint: bool,
    /// Every `order - 1` control points form a Bézier-like span (knots repeated `order - 1`
    /// times); falls back to `endpoint` when the count does not fit.
    pub bezier: bool,
}

/// The knot vector for `count` control points (already wrapped for cyclic curves), of `order`.
fn knots(count: usize, order: usize, layout: NurbsKnots, cyclic: bool) -> Vec<f64> {
    let degree = order - 1;
    let total = count + order;
    let bezier_fits = layout.bezier && degree > 0 && (count - 1).is_multiple_of(degree);
    if !cyclic && bezier_fits {
        let mut knots = vec![0.0; order];
        let spans = (count - 1) / degree;
        for span in 1..spans {
            knots.extend(core::iter::repeat_n(span as f64, degree));
        }
        knots.extend(core::iter::repeat_n(spans as f64, order));
        return knots;
    }
    if !cyclic && (layout.endpoint || layout.bezier) {
        let inner = total - 2 * order;
        let mut knots = vec![0.0; order];
        knots.extend((1..=inner).map(|i| i as f64));
        knots.extend(core::iter::repeat_n((inner + 1) as f64, order));
        return knots;
    }
    (0..total).map(|i| i as f64).collect()
}

/// The blossom of the curve at `args` (`degree` of them), for the span ending at knot index
/// `span` (`knots[span] <= args < knots[span + 1]`): de Boor with one argument per level.
fn blossom(
    control: &[[f64; 3]],
    knots: &[f64],
    degree: usize,
    span: usize,
    args: &[f64],
) -> [f64; 3] {
    let mut d = [[0.0; 3]; MAX_DEGREE + 1];
    for (j, slot) in d.iter_mut().enumerate().take(degree + 1) {
        *slot = control[span - degree + j];
    }
    for r in 1..=degree {
        let u = args[r - 1];
        for j in (r..=degree).rev() {
            let i = span - degree + j;
            let denom = knots[i + degree + 1 - r] - knots[i];
            let alpha = if denom.abs() > 0.0 {
                (u - knots[i]) / denom
            } else {
                0.0
            };
            let before = d[j - 1];
            for (value, previous) in d[j].iter_mut().zip(before) {
                *value = (1.0 - alpha) * previous + alpha * *value;
            }
        }
    }
    d[degree]
}

/// The segments of a NURBS subpath through `points` with `weights`, of `order` (clamped to
/// what the points allow), appended to `out`. Fewer than two points give nothing.
pub(crate) fn segments(
    points: &[Point],
    weights: &[f64],
    order: usize,
    layout: NurbsKnots,
    cyclic: bool,
    out: &mut Vec<Segment>,
) {
    let count = points.len();
    if count < 2 {
        return;
    }
    let order = order.clamp(2, (MAX_DEGREE + 1).min(count));
    let degree = order - 1;
    let weight = |i: usize| {
        let w = weights.get(i).copied().unwrap_or(1.0);
        if w.is_finite() && w > 0.0 { w } else { 1.0 }
    };
    let mut control: Vec<[f64; 3]> = (0..count)
        .map(|i| {
            let w = weight(i);
            [points[i].x * w, points[i].y * w, w]
        })
        .collect();
    let cyclic_bezier = cyclic && layout.bezier && count.is_multiple_of(degree);
    if cyclic_bezier {
        // A closed Bézier-like curve: the first point closes the last span.
        control.push(control[0]);
    } else if cyclic {
        // A periodic curve: the first `degree` points again.
        control.extend_from_within(..degree);
    }
    let n = control.len();
    let knots = knots(n, order, layout, cyclic && !cyclic_bezier);
    let (first_span, last_span) = (degree, n - 1);
    let mut args = [0.0; MAX_DEGREE];
    for span in first_span..=last_span {
        let (a, b) = (knots[span], knots[span + 1]);
        if b <= a {
            continue;
        }
        let mut h = [[0.0; 3]; MAX_DEGREE + 1];
        for (j, slot) in h.iter_mut().enumerate().take(degree + 1) {
            for (index, arg) in args.iter_mut().enumerate().take(degree) {
                *arg = if index < degree - j { a } else { b };
            }
            *slot = blossom(&control, &knots, degree, span, &args[..degree]);
        }
        let mut bez_points = [Point::ORIGIN; MAX_DEGREE + 1];
        let mut bez_weights = [1.0; MAX_DEGREE + 1];
        for j in 0..=degree {
            bez_weights[j] = h[j][2];
            bez_points[j] = Point::new(h[j][0] / h[j][2], h[j][1] / h[j][2]);
        }
        let uniform = bez_weights[..=degree]
            .iter()
            .all(|w| (w - bez_weights[0]).abs() <= 1e-15 * bez_weights[0].abs());
        let segment = match (uniform, degree) {
            (true, 1) => Segment::Line([bez_points[0], bez_points[1]]),
            (true, 2) => Segment::Quad([bez_points[0], bez_points[1], bez_points[2]]),
            (true, 3) => {
                Segment::Cubic([bez_points[0], bez_points[1], bez_points[2], bez_points[3]])
            }
            _ => match RationalBez::new(&bez_points[..=degree], &bez_weights[..=degree]) {
                Some(rational) => Segment::Rational(rational),
                None => continue,
            },
        };
        out.push(segment);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::FRAC_1_SQRT_2;

    #[test]
    fn a_clamped_quadratic_nurbs_with_bezier_knots_is_a_circle() {
        // The classic 9-point circle: square corners weighted √2/2.
        let points = [
            Point::new(1.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(0.0, 1.0),
            Point::new(-1.0, 1.0),
            Point::new(-1.0, 0.0),
            Point::new(-1.0, -1.0),
            Point::new(0.0, -1.0),
            Point::new(1.0, -1.0),
            Point::new(1.0, 0.0),
        ];
        let weights = [
            1.0,
            FRAC_1_SQRT_2,
            1.0,
            FRAC_1_SQRT_2,
            1.0,
            FRAC_1_SQRT_2,
            1.0,
            FRAC_1_SQRT_2,
            1.0,
        ];
        let layout = NurbsKnots {
            endpoint: true,
            bezier: true,
        };
        let mut out = Vec::new();
        segments(&points, &weights, 3, layout, false, &mut out);
        assert_eq!(out.len(), 4);
        for segment in &out {
            for i in 0..=100 {
                let p = segment.eval(f64::from(i) / 100.0);
                assert!((p.to_vector().length() - 1.0).abs() < 1e-14, "{p:?}");
            }
        }
    }

    #[test]
    fn a_uniform_cubic_nurbs_matches_de_boor_at_its_knots() {
        // An open uniform cubic B-spline on 6 points has 3 spans; its value at the span joints
        // is (P[i] + 4·P[i+1] + P[i+2]) / 6.
        let points: Vec<Point> = (0..6)
            .map(|i| Point::new(f64::from(i), f64::from(i % 2) * 3.0))
            .collect();
        let mut out = Vec::new();
        segments(
            &points,
            &[1.0; 6],
            4,
            NurbsKnots::default(),
            false,
            &mut out,
        );
        assert_eq!(out.len(), 3);
        for (i, segment) in out.iter().enumerate() {
            let expected = Point::new(
                (points[i].x + 4.0 * points[i + 1].x + points[i + 2].x) / 6.0,
                (points[i].y + 4.0 * points[i + 1].y + points[i + 2].y) / 6.0,
            );
            assert!(segment.start().distance(expected) < 1e-12, "{i}");
        }
        // Spans join continuously.
        for pair in out.windows(2) {
            assert!(pair[0].end().distance(pair[1].start()) < 1e-12);
        }
    }

    #[test]
    fn a_cyclic_nurbs_closes_on_itself() {
        let points: Vec<Point> = (0..5)
            .map(|i| Vector::from_angle(f64::from(i) * 1.256_637_061_435_917_2).to_point())
            .collect();
        let mut out = Vec::new();
        segments(&points, &[1.0; 5], 3, NurbsKnots::default(), true, &mut out);
        assert_eq!(out.len(), 5);
        assert!(out[4].end().distance(out[0].start()) < 1e-12);
    }

    use crate::Vector;
}
