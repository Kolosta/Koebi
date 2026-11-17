//! Booleans against their definition, sampled: every point clear of both operands' boundaries
//! must be inside the result exactly when the operation keeps it, the result must wind once
//! at most (no overlap left), and fill the same under both rules.

use core::f64::consts::PI;

use super::*;
use crate::Path;

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn rect(min: (f64, f64), max: (f64, f64)) -> Rect {
    Rect::new(p(min.0, min.1), p(max.0, max.1))
}

fn rectangle(min: (f64, f64), max: (f64, f64)) -> Contours {
    Path::rectangle(rect(min, max)).contours()
}

fn circle(center: (f64, f64), radius: f64) -> Contours {
    Path::exact_ellipse(p(center.0, center.1), Vector::new(radius, radius)).contours()
}

fn polygon(points: &[(f64, f64)]) -> Contours {
    let points: Vec<Point> = points.iter().map(|&(x, y)| p(x, y)).collect();
    let segments = (0..points.len())
        .map(|i| Segment::line(points[i], points[(i + 1) % points.len()]))
        .collect();
    let mut contours = Contours::default();
    contours.push(Contour::new(segments, true));
    contours
}

const OPS: [BooleanOp; 4] = [
    BooleanOp::Union,
    BooleanOp::Intersect,
    BooleanOp::Subtract,
    BooleanOp::Xor,
];

/// Checks `result` against `expected` (inside or not) over `area`, skipping the points within
/// `margin` of the boundaries in `near`.
#[track_caller]
fn check(
    result: &Contours,
    expected: &dyn Fn(Point) -> bool,
    near: &[&Contours],
    area: Rect,
    margin: f64,
) {
    const STEPS: u32 = 60;
    let mut checked = 0;
    for i in 0..=STEPS {
        for j in 0..=STEPS {
            let q = p(
                area.min.x + (area.max.x - area.min.x) * f64::from(i) / f64::from(STEPS),
                area.min.y + (area.max.y - area.min.y) * f64::from(j) / f64::from(STEPS),
            );
            if near.iter().any(|contours| contours.distance(q) <= margin) {
                continue;
            }
            let winding = result.winding(q);
            assert!((0..=1).contains(&winding), "winding {winding} at {q:?}");
            assert_eq!(winding != 0, expected(q), "at {q:?}");
            assert_eq!(
                result.contains(q, FillRule::EvenOdd),
                expected(q),
                "at {q:?}"
            );
            checked += 1;
        }
    }
    assert!(checked > 1000, "only {checked} points checked");
}

/// Checks every operation on `a` and `b` against their sampled definition.
#[track_caller]
fn check_all(a: &Contours, a_rule: FillRule, b: &Contours, b_rule: FillRule, area: Rect) {
    for op in OPS {
        let result = boolean(a, a_rule, b, b_rule, op);
        let expected = |q: Point| op.keeps(a.contains(q, a_rule), b.contains(q, b_rule));
        check(&result, &expected, &[a, b], area, 1e-6);
    }
}

/// The area inside `contours`, by Green's theorem: half the integral of P × P' along them.
fn area(contours: &Contours) -> f64 {
    contours
        .contours()
        .iter()
        .flat_map(Contour::segments)
        .map(|segment| {
            let integrand = |t: f64| 0.5 * segment.eval(t).to_vector().cross(segment.derivative(t));
            crate::solve::integrate(&integrand, 0.0, 1.0, 1e-12)
        })
        .sum()
}

fn nonzero(a: &Contours, b: &Contours, op: BooleanOp) -> Contours {
    boolean(a, FillRule::NonZero, b, FillRule::NonZero, op)
}

#[test]
fn overlapping_squares() {
    let a = rectangle((0.0, 0.0), (10.0, 10.0));
    let b = rectangle((5.0, 5.0), (15.0, 15.0));
    let area_of = |op| area(&nonzero(&a, &b, op));
    assert!((area_of(BooleanOp::Union) - 175.0).abs() < 1e-9);
    assert!((area_of(BooleanOp::Intersect) - 25.0).abs() < 1e-9);
    assert!((area_of(BooleanOp::Subtract) - 75.0).abs() < 1e-9);
    assert!((area_of(BooleanOp::Xor) - 150.0).abs() < 1e-9);
    let rule = FillRule::NonZero;
    check_all(&a, rule, &b, rule, rect((-2.0, -2.0), (17.0, 17.0)));
}

#[test]
fn circles_keep_their_exact_arcs() {
    let (a, b) = (circle((0.0, 0.0), 10.0), circle((12.0, 0.0), 10.0));
    let union = nonzero(&a, &b, BooleanOp::Union);
    // Two discs of radius r at distance d: the lens is 2r²·acos(d/2r) - (d/2)·√(4r² - d²).
    let (r, d): (f64, f64) = (10.0, 12.0);
    let lens = 2.0 * r * r * (d / (2.0 * r)).acos() - d / 2.0 * (4.0 * r * r - d * d).sqrt();
    let found = area(&union);
    assert!((found - (2.0 * PI * r * r - lens)).abs() < 1e-4, "{found}");
    let intersect = nonzero(&a, &b, BooleanOp::Intersect);
    assert!((area(&intersect) - lens).abs() < 1e-4);
    // Made of the circles' own rational pieces.
    let rational = union
        .contours()
        .iter()
        .flat_map(Contour::segments)
        .all(|s| matches!(s, Segment::Rational(_)));
    assert!(rational);
    let rule = FillRule::NonZero;
    check_all(&a, rule, &b, rule, rect((-12.0, -12.0), (24.0, 12.0)));
}

#[test]
fn curves_against_lines() {
    let mut blob = Contours::default();
    blob.push(Contour::new(
        vec![
            Segment::cubic(p(0.0, 0.0), p(20.0, -10.0), p(30.0, 20.0), p(10.0, 25.0)),
            Segment::cubic(p(10.0, 25.0), p(-5.0, 28.0), p(-15.0, 5.0), p(0.0, 0.0)),
        ],
        true,
    ));
    let bar = rectangle((-20.0, 8.0), (40.0, 14.0));
    let rule = FillRule::NonZero;
    check_all(&blob, rule, &bar, rule, rect((-22.0, -8.0), (42.0, 30.0)));
}

#[test]
fn shared_edges_and_identical_shapes() {
    let rule = FillRule::NonZero;
    // Side by side, one edge shared: one rectangle, and nothing in common.
    let a = rectangle((0.0, 0.0), (10.0, 10.0));
    let b = rectangle((10.0, 0.0), (20.0, 10.0));
    let union = nonzero(&a, &b, BooleanOp::Union);
    assert_eq!(union.contours().len(), 1);
    assert!((area(&union) - 200.0).abs() < 1e-9);
    assert!(nonzero(&a, &b, BooleanOp::Intersect).is_empty());
    check_all(&a, rule, &b, rule, rect((-2.0, -2.0), (22.0, 12.0)));
    // Part of an edge shared.
    let c = rectangle((10.0, 4.0), (16.0, 8.0));
    check_all(&a, rule, &c, rule, rect((-2.0, -2.0), (18.0, 12.0)));
    // A shape with itself.
    let disc = circle((0.0, 0.0), 5.0);
    let union = nonzero(&disc, &disc, BooleanOp::Union);
    assert!((area(&union) - 25.0 * PI).abs() < 1e-6);
    assert!(nonzero(&disc, &disc, BooleanOp::Subtract).is_empty());
    assert!(nonzero(&disc, &disc, BooleanOp::Xor).is_empty());
    check_all(&disc, rule, &disc, rule, rect((-6.0, -6.0), (6.0, 6.0)));
}

#[test]
fn shapes_touching_at_a_point_stay_apart() {
    let a = rectangle((0.0, 0.0), (10.0, 10.0));
    let b = rectangle((10.0, 10.0), (20.0, 20.0));
    let union = nonzero(&a, &b, BooleanOp::Union);
    assert_eq!(
        union.contours().len(),
        2,
        "two loops, not one through the corner twice"
    );
    assert!((area(&union) - 200.0).abs() < 1e-9);
    let rule = FillRule::NonZero;
    check_all(&a, rule, &b, rule, rect((-2.0, -2.0), (22.0, 22.0)));
}

#[test]
fn fill_rules_decide_holes_and_crossings() {
    // A pentagram: its middle is wound twice, filled by non-zero and empty by even-odd.
    let star: Vec<(f64, f64)> = (0..5)
        .map(|i| {
            let angle = -PI / 2.0 + f64::from(i) * 4.0 * PI / 5.0;
            (10.0 * angle.cos(), 10.0 * angle.sin())
        })
        .collect();
    let star = polygon(&star);
    for rule in [FillRule::NonZero, FillRule::EvenOdd] {
        let simple = simplify(&star, rule);
        let area = rect((-11.0, -11.0), (11.0, 11.0));
        check(&simple, &|q| star.contains(q, rule), &[&star], area, 1e-6);
    }
    // A square with a hole against a bar through it, under each rule.
    let mut holed = rectangle((0.0, 0.0), (20.0, 20.0));
    let hole = polygon(&[(5.0, 5.0), (5.0, 15.0), (15.0, 15.0), (15.0, 5.0)]);
    holed.push(hole.contours()[0].clone());
    let bar = rectangle((-5.0, 8.0), (25.0, 12.0));
    let area = rect((-7.0, -2.0), (27.0, 22.0));
    check_all(&holed, FillRule::NonZero, &bar, FillRule::EvenOdd, area);
    check_all(&holed, FillRule::EvenOdd, &bar, FillRule::NonZero, area);
}

#[test]
fn a_stroke_outline_becomes_one_region() {
    use crate::{Join, StrokeStyle, outline};
    let count = |c: &Contours| {
        c.contours()
            .iter()
            .map(|c| c.segments().len())
            .sum::<usize>()
    };
    // A figure of eight: its outline overlaps itself (wound twice where it crosses); simplified,
    // it is one region, wound once.
    let points = [p(0.0, 0.0), p(20.0, 20.0), p(20.0, 0.0), p(0.0, 20.0)];
    let mut eight = Contours::default();
    let segments = points
        .windows(2)
        .map(|w| Segment::line(w[0], w[1]))
        .collect();
    eight.push(Contour::new(segments, false));
    let style = StrokeStyle {
        width: 4.0,
        join: Join::Round,
        ..StrokeStyle::default()
    };
    let stroke = outline(&eight, &style, 1e-6);
    assert_eq!(stroke.winding(p(10.0, 10.0)), 2);
    let simple = simplify(&stroke, FillRule::NonZero);
    let inside = |q: Point| stroke.contains(q, FillRule::NonZero);
    check(
        &simple,
        &inside,
        &[&stroke],
        rect((-4.0, -4.0), (24.0, 24.0)),
        1e-6,
    );
    // A hairpin tighter than the stroke: the fold's polyline and its pivots, inside the
    // region, go away.
    let mut hairpin = Contours::default();
    let bend = Segment::cubic(p(0.0, 0.0), p(40.0, 0.0), p(40.0, 6.0), p(0.0, 6.0));
    hairpin.push(Contour::new(vec![bend], false));
    let style = StrokeStyle {
        width: 10.0,
        ..StrokeStyle::default()
    };
    let stroke = outline(&hairpin, &style, 1e-3);
    let simple = simplify(&stroke, FillRule::NonZero);
    assert!(
        count(&simple) * 2 < count(&stroke),
        "{} vs {}",
        count(&simple),
        count(&stroke)
    );
    let inside = |q: Point| stroke.contains(q, FillRule::NonZero);
    check(
        &simple,
        &inside,
        &[&stroke],
        rect((-6.0, -6.0), (36.0, 12.0)),
        1e-6,
    );
}

#[test]
fn empty_operands() {
    let a = rectangle((0.0, 0.0), (10.0, 10.0));
    let empty = Contours::default();
    assert!((area(&nonzero(&a, &empty, BooleanOp::Union)) - 100.0).abs() < 1e-9);
    assert!(nonzero(&a, &empty, BooleanOp::Intersect).is_empty());
    assert!(nonzero(&empty, &a, BooleanOp::Subtract).is_empty());
    assert!(simplify(&empty, FillRule::NonZero).is_empty());
}

#[test]
fn regions_grow_and_shrink_by_their_join() {
    let square = rectangle((0.0, 0.0), (20.0, 20.0));
    let rule = FillRule::NonZero;
    // Grown round: the points within 3; grown mitered: a square 6 wider.
    let round = offset_region(&square, rule, 3.0, Join::Round, 1e-6);
    let within = |q: Point| square.contains(q, rule) || square.distance(q) <= 3.0;
    let area = rect((-5.0, -5.0), (25.0, 25.0));
    check(&round, &within, &[&square, &round], area, 1e-5);
    let mitered = offset_region(&square, rule, 3.0, Join::default(), 1e-6);
    let grown = rectangle((-3.0, -3.0), (23.0, 23.0));
    check(
        &mitered,
        &|q| grown.contains(q, rule),
        &[&grown],
        area,
        1e-5,
    );
    // Shrunk: a square 6 narrower, whatever the join, its corners being convex.
    let shrunk = offset_region(&square, rule, -3.0, Join::Round, 1e-6);
    let inner = rectangle((3.0, 3.0), (17.0, 17.0));
    check(&shrunk, &|q| inner.contains(q, rule), &[&inner], area, 1e-5);
    // Shrunk past its half width: nothing.
    assert!(offset_region(&square, rule, -11.0, Join::Round, 1e-6).is_empty());
}

#[test]
fn aligned_strokes_are_exact_on_shapes_thinner_than_their_band() {
    use crate::{Align, StrokeStyle, outline};
    let rule = FillRule::NonZero;
    // Inside a bar 4 thick, a stroke 6 wide fills the bar, never more.
    let bar = rectangle((0.0, 0.0), (40.0, 4.0));
    let inside = StrokeStyle {
        width: 6.0,
        align: Align::Inside,
        ..StrokeStyle::default()
    };
    let found = outline(&bar, &inside, 1e-6);
    check(
        &found,
        &|q| bar.contains(q, rule),
        &[&bar],
        rect((-2.0, -2.0), (42.0, 6.0)),
        1e-5,
    );
    // Inside a circle of radius 2, a stroke 6 wide is the disc.
    let small = circle((0.0, 0.0), 2.0);
    let found = outline(&small, &inside, 1e-6);
    check(
        &found,
        &|q| small.contains(q, rule),
        &[&small],
        rect((-3.0, -3.0), (3.0, 3.0)),
        1e-5,
    );
    // Inside with an offset of 100% (band 3..9 in): in the bar nothing is 3 from the edge.
    let away = StrokeStyle {
        offset: crate::StrokeOffset::Percent(100.0),
        ..inside
    };
    assert!(outline(&bar, &away, 1e-6).is_empty());
}

#[test]
fn shrinking_past_the_curvature_removes_the_swallowtails() {
    // An ellipse 20 by 8 shrunk by 5: tighter than 5 at the ends of its long axis, where the
    // raw offset folds into swallowtails. What is left: the points 5 or more inside.
    let ellipse = Path::exact_ellipse(p(0.0, 0.0), Vector::new(20.0, 8.0)).contours();
    let rule = FillRule::NonZero;
    let shrunk = offset_region(&ellipse, rule, -5.0, Join::Round, 1e-5);
    let deep = |q: Point| ellipse.contains(q, rule) && ellipse.distance(q) >= 5.0;
    let near = |q: Point| (ellipse.distance(q) - 5.0).abs() <= 1e-4;
    let mut checked = 0;
    for i in 0..=80 {
        for j in 0..=40 {
            let q = p(
                -21.0 + 42.0 * f64::from(i) / 80.0,
                -9.0 + 18.0 * f64::from(j) / 40.0,
            );
            if near(q) {
                continue;
            }
            let winding = shrunk.winding(q);
            assert!((0..=1).contains(&winding), "winding {winding} at {q:?}");
            assert_eq!(winding != 0, deep(q), "at {q:?}");
            checked += 1;
        }
    }
    assert!(checked > 2000);
    // Grown by 5 it is every point within 5, the round joins being the curve's own.
    let grown = offset_region(&ellipse, rule, 5.0, Join::Round, 1e-5);
    let within = |q: Point| ellipse.contains(q, rule) || ellipse.distance(q) <= 5.0;
    check(
        &grown,
        &within,
        &[&ellipse, &grown],
        rect((-26.0, -14.0), (26.0, 14.0)),
        1e-3,
    );
}
