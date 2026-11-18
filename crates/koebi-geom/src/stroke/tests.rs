//! Stroke outlines against references built apart: polygons, discs, annular sectors and
//! distances to the path, never the outliner's own offsets.
//!
//! Each check samples a grid over the stroke, then points just off the reference's boundary,
//! found along the grid's rows and columns. Every point farther than a millionth of the width
//! from the reference's boundary must be inside the outline exactly when it is inside the
//! reference (#258's bar), and no point may wind negatively.

use core::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_2, FRAC_PI_6, PI};

use super::*;
use crate::{FillRule, RationalBez, Rect};

/// How close to the reference's boundary a point may lie and still be checked, as a share of
/// the width.
const MARGIN: f64 = 1e-6;

/// Whether a point is inside a reference, and a lower bound of its distance to the
/// reference's boundary.
type Probe = (bool, f64);

/// A reference region.
type Reference<'a> = Box<dyn Fn(Point) -> Probe + 'a>;

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn path(segments: Vec<Segment>, closed: bool) -> Contours {
    let mut contours = Contours::default();
    contours.push(Contour::new(segments, closed));
    contours
}

fn polyline(points: &[(f64, f64)], closed: bool) -> Contours {
    let points: Vec<Point> = points.iter().map(|&(x, y)| p(x, y)).collect();
    let mut segments: Vec<Segment> = points
        .windows(2)
        .map(|pair| Segment::line(pair[0], pair[1]))
        .collect();
    if closed {
        segments.push(Segment::line(points[points.len() - 1], points[0]));
    }
    path(segments, closed)
}

/// A quarter of a circle, exact, from `from` to `to` round the corner `control`.
fn quarter(from: Point, control: Point, to: Point) -> Segment {
    let arc = RationalBez::new(&[from, control, to], &[1.0, FRAC_1_SQRT_2, 1.0]);
    Segment::Rational(arc.expect("valid weights"))
}

fn style(width: f64) -> StrokeStyle {
    StrokeStyle {
        width,
        ..StrokeStyle::default()
    }
}

fn stroke(contours: &Contours, style: &StrokeStyle) -> Contours {
    outline(contours, style, style.width * 1e-7)
}

fn bounds(min: (f64, f64), max: (f64, f64)) -> Rect {
    Rect::new(p(min.0, min.1), p(max.0, max.1))
}

fn segment_distance(point: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let length = ab.length_squared();
    let t = if length > 0.0 {
        ((point - a).dot(ab) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    point.distance(a + ab * t)
}

/// A polygon of any shape, filled with the non-zero rule.
fn polygon<'a>(points: &[(f64, f64)]) -> Reference<'a> {
    let points: Vec<Point> = points.iter().map(|&(x, y)| p(x, y)).collect();
    Box::new(move |q: Point| {
        let mut winding = 0;
        let mut distance = f64::INFINITY;
        for (index, &a) in points.iter().enumerate() {
            let b = points[(index + 1) % points.len()];
            distance = distance.min(segment_distance(q, a, b));
            if (a.y <= q.y) != (b.y <= q.y) {
                let x = a.x + (q.y - a.y) / (b.y - a.y) * (b.x - a.x);
                if x > q.x {
                    winding += if b.y > a.y { 1 } else { -1 };
                }
            }
        }
        (winding != 0, distance)
    })
}

fn rectangle<'a>(min: (f64, f64), max: (f64, f64)) -> Reference<'a> {
    polygon(&[min, (max.0, min.1), max, (min.0, max.1)])
}

fn disc<'a>(center: Point, radius: f64) -> Reference<'a> {
    Box::new(move |q: Point| {
        let distance = q.distance(center);
        (distance <= radius, (distance - radius).abs())
    })
}

/// The points between radii `inner` and `outer` round `center`, at angles (from x toward y)
/// from `from` to `to`, less than a half turn apart.
fn sector<'a>(center: Point, inner: f64, outer: f64, from: f64, to: f64) -> Reference<'a> {
    Box::new(move |q: Point| {
        let offset = q - center;
        let radius = offset.length();
        let span = (to - from).rem_euclid(2.0 * PI);
        let within = (offset.angle() - from).rem_euclid(2.0 * PI) <= span;
        let inside = within && (inner..=outer).contains(&radius);
        let arc = |r: f64| {
            if within {
                (radius - r).abs()
            } else {
                let ends = [from, to].map(|angle| center + Vector::from_angle(angle) * r);
                q.distance(ends[0]).min(q.distance(ends[1]))
            }
        };
        let side = |angle: f64| {
            let direction = Vector::from_angle(angle);
            segment_distance(q, center + direction * inner, center + direction * outer)
        };
        (
            inside,
            arc(inner).min(arc(outer)).min(side(from)).min(side(to)),
        )
    })
}

/// The points within `radius` of `contours`: a stroke with round joins and caps.
fn within(contours: &Contours, radius: f64) -> Reference<'_> {
    Box::new(move |q: Point| {
        let distance = contours.distance(q);
        (distance <= radius, (distance - radius).abs())
    })
}

fn union(parts: Vec<Reference<'_>>) -> Reference<'_> {
    Box::new(move |q: Point| {
        parts
            .iter()
            .fold((false, f64::INFINITY), |(inside, distance), part| {
                let (part_inside, part_distance) = part(q);
                (inside || part_inside, distance.min(part_distance))
            })
    })
}

fn minus<'a>(kept: Reference<'a>, removed: Reference<'a>) -> Reference<'a> {
    Box::new(move |q: Point| {
        let ((a, a_distance), (b, b_distance)) = (kept(q), removed(q));
        (a && !b, a_distance.min(b_distance))
    })
}

/// Checks `outline` against `reference` over `area`, for a stroke of `width`.
#[track_caller]
fn check(outline: &Contours, reference: &Reference<'_>, area: Rect, width: f64) {
    check_on(outline, reference, area, width, 64);
}

/// [`check`] on a grid of `steps` × `steps` cells: a coarser one for outlines of many
/// segments, which a debug build tests slowly.
#[track_caller]
fn check_on(outline: &Contours, reference: &Reference<'_>, area: Rect, width: f64, steps: u32) {
    let margin = MARGIN * width;
    let at = |i: u32, j: u32| {
        let (u, v) = (
            f64::from(i) / f64::from(steps),
            f64::from(j) / f64::from(steps),
        );
        p(
            area.min.x + (area.max.x - area.min.x) * u,
            area.min.y + (area.max.y - area.min.y) * v,
        )
    };
    let mut checked = 0_u32;
    let mut probe = |q: Point| {
        let (inside, distance) = reference(q);
        if distance <= margin {
            return;
        }
        let winding = outline.winding(q);
        assert!(winding >= 0, "winding {winding} at {q:?}");
        assert_eq!(
            winding != 0,
            inside,
            "at {q:?}, {distance} from the reference's boundary"
        );
        checked += 1;
    };
    for i in 0..=steps {
        for j in 0..=steps {
            probe(at(i, j));
        }
    }
    // Just off the boundary, where the rows and the columns cross it.
    for line in 0..=steps {
        for step in 0..steps {
            for (a, b) in [
                (at(step, line), at(step + 1, line)),
                (at(line, step), at(line, step + 1)),
            ] {
                let inside = reference(a).0;
                if reference(b).0 == inside {
                    continue;
                }
                let (mut lo, mut hi) = (a, b);
                for _ in 0..80 {
                    let middle = lo.midpoint(hi);
                    if reference(middle).0 == inside {
                        lo = middle;
                    } else {
                        hi = middle;
                    }
                }
                let direction = (b - a).normalized().expect("distinct points");
                for shift in [-8.0, -3.0, -1.5, 1.5, 3.0, 8.0] {
                    probe(lo + direction * (shift * margin));
                }
            }
        }
    }
    assert!(checked > steps * steps / 2, "only {checked} points checked");
}

#[test]
fn a_centered_stroke_is_the_points_within_half_its_width() {
    // An S-shaped cubic, then a corner; round joins and caps.
    let curve = path(
        vec![
            Segment::cubic(p(0.0, 0.0), p(30.0, -40.0), p(60.0, 40.0), p(90.0, 0.0)),
            Segment::line(p(90.0, 0.0), p(120.0, 30.0)),
        ],
        false,
    );
    let round = StrokeStyle {
        cap: Cap::Round,
        join: Join::Round,
        ..style(8.0)
    };
    let found = stroke(&curve, &round);
    let area = bounds((-8.0, -30.0), (128.0, 38.0));
    check(&found, &within(&curve, 4.0), area, 8.0);
}

#[test]
fn a_band_wider_than_the_curve_is_tight_stays_exact() {
    // A hairpin far tighter than the stroke's half width: the inner offset folds.
    let hairpin = path(
        vec![Segment::cubic(
            p(0.0, 0.0),
            p(40.0, 0.0),
            p(40.0, 6.0),
            p(0.0, 6.0),
        )],
        false,
    );
    let round = StrokeStyle {
        cap: Cap::Round,
        join: Join::Round,
        ..style(10.0)
    };
    let found = stroke(&hairpin, &round);
    let area = bounds((-6.0, -6.0), (36.0, 12.0));
    // Folds are polylines within the tolerance, here a ten-millionth of the width: thousands of
    // segments, so a coarser grid (the points by the boundary are as many).
    check_on(&found, &within(&hairpin, 5.0), area, 10.0, 32);

    // An exact circle of radius 2 under a stroke 6 wide: a disc of radius 5.
    let circle = crate::Path::exact_ellipse(p(0.0, 0.0), Vector::new(2.0, 2.0)).contours();
    let found = stroke(&circle, &style(6.0));
    let area = bounds((-6.0, -6.0), (6.0, 6.0));
    check_on(&found, &disc(p(0.0, 0.0), 5.0), area, 6.0, 32);
    // A ring under a stroke narrower than the circle.
    let found = stroke(&circle, &style(1.0));
    let ring = minus(disc(p(0.0, 0.0), 2.5), disc(p(0.0, 0.0), 1.5));
    check(&found, &ring, bounds((-3.0, -3.0), (3.0, 3.0)), 1.0);
}

#[test]
fn butt_square_round_and_taper_caps() {
    let line = polyline(&[(0.0, 0.0), (20.0, 0.0)], false);
    let area = bounds((-10.0, -4.0), (30.0, 4.0));
    let capped = |cap| stroke(&line, &StrokeStyle { cap, ..style(4.0) });
    let butt = capped(Cap::Butt { tilt: 0.0 });
    check(&butt, &rectangle((0.0, -2.0), (20.0, 2.0)), area, 4.0);
    let square = capped(Cap::Square);
    check(&square, &rectangle((-2.0, -2.0), (22.0, 2.0)), area, 4.0);
    let round = capped(Cap::Round);
    check(&round, &within(&line, 2.0), area, 4.0);
    // A taper of length 0 reaches twice the width beyond each end.
    let taper = capped(Cap::Taper { length: 0.0 });
    let tips = [
        (0.0, -2.0),
        (20.0, -2.0),
        (28.0, 0.0),
        (20.0, 2.0),
        (0.0, 2.0),
        (-8.0, 0.0),
    ];
    check(&taper, &polygon(&tips), area, 4.0);
    let short = capped(Cap::Taper { length: 3.0 });
    let tips = [
        (0.0, -2.0),
        (20.0, -2.0),
        (23.0, 0.0),
        (20.0, 2.0),
        (0.0, 2.0),
        (-3.0, 0.0),
    ];
    check(&short, &polygon(&tips), area, 4.0);
}

#[test]
fn a_tilted_butt_cap_turns_about_the_path_end() {
    let line = polyline(&[(0.0, 0.0), (20.0, 0.0)], false);
    let k = FRAC_PI_6.tan();
    let tilted = StrokeStyle {
        cap: Cap::Butt { tilt: FRAC_PI_6 },
        ..style(4.0)
    };
    // An edge at distance d reaches d·tan(tilt) beyond each end, outward.
    let found = stroke(&line, &tilted);
    let expected = polygon(&[
        (2.0 * k, -2.0),
        (20.0 - 2.0 * k, -2.0),
        (20.0 + 2.0 * k, 2.0),
        (-2.0 * k, 2.0),
    ]);
    check(&found, &expected, bounds((-4.0, -3.0), (24.0, 3.0)), 4.0);
    // Aligned to the right, the cut still turns about the path's end, at the band's edge.
    let right = StrokeStyle {
        align: Align::Right,
        ..tilted.clone()
    };
    let found = stroke(&line, &right);
    let expected = polygon(&[
        (0.0, 0.0),
        (20.0, 0.0),
        (20.0 + 4.0 * k, 4.0),
        (-4.0 * k, 4.0),
    ]);
    check(&found, &expected, bounds((-4.0, -1.0), (24.0, 5.0)), 4.0);
    // Over a corner close to the end, the cut reaches back into the previous segment.
    let hooked = polyline(&[(0.0, 0.0), (20.0, 0.0), (21.0, 0.0)], false);
    let found = stroke(&hooked, &tilted);
    let expected = polygon(&[
        (2.0 * k, -2.0),
        (21.0 - 2.0 * k, -2.0),
        (21.0 + 2.0 * k, 2.0),
        (-2.0 * k, 2.0),
    ]);
    check(&found, &expected, bounds((-4.0, -3.0), (25.0, 3.0)), 4.0);
    // Dashes are never tilted.
    let dashed = StrokeStyle {
        dashes: Dashes {
            pattern: vec![5.0, 5.0],
            ..Dashes::default()
        },
        ..tilted
    };
    let found = stroke(&line, &dashed);
    let expected = union(vec![
        rectangle((0.0, -2.0), (5.0, 2.0)),
        rectangle((10.0, -2.0), (15.0, 2.0)),
    ]);
    check(&found, &expected, bounds((-4.0, -3.0), (24.0, 3.0)), 4.0);
}

/// The reference of a centered stroke along a polyline with butt caps: each segment's
/// rectangle, and on the outer side of each corner the join, from the corner's geometry alone.
fn polyline_reference<'a>(points: &[(f64, f64)], half: f64, join: Join) -> Reference<'a> {
    let points: Vec<Point> = points.iter().map(|&(x, y)| p(x, y)).collect();
    let mut parts: Vec<Reference<'a>> = Vec::new();
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let n = (b - a).normalized().expect("distinct").perpendicular() * half;
        parts.push(polygon(&[a - n, b - n, b + n, a + n].map(|q| (q.x, q.y))));
    }
    for triple in points.windows(3) {
        let (a, b, c) = (triple[0], triple[1], triple[2]);
        let t1 = (b - a).normalized().expect("distinct");
        let t2 = (c - b).normalized().expect("distinct");
        let turn = t1.cross(t2).atan2(t1.dot(t2));
        let sign = -turn.signum();
        let (o1, o2) = (t1.perpendicular() * sign, t2.perpendicular() * sign);
        let (e1, e2) = (b + o1 * half, b + o2 * half);
        match join {
            Join::Round => parts.push(disc(b, half)),
            Join::Miter { limit } if 1.0 / (turn * 0.5).cos() <= limit => {
                let tip = b + (o1 + o2) * (half / (1.0 + o1.dot(o2)));
                parts.push(polygon(&[b, e1, tip, e2].map(|q| (q.x, q.y))));
            }
            _ => parts.push(polygon(&[b, e1, e2].map(|q| (q.x, q.y)))),
        }
    }
    union(parts)
}

#[test]
fn joins_fill_the_outer_side_of_corners() {
    // A gentle turn, a turn the other way, and one so sharp that a miter of limit 4 falls back
    // to a bevel.
    let points = [
        (0.0, 0.0),
        (20.0, 0.0),
        (30.0, 15.0),
        (45.0, 15.0),
        (25.0, 19.0),
    ];
    let corner = polyline(&points, false);
    let area = bounds((-4.0, -6.0), (60.0, 24.0));
    let joins = [
        Join::Miter { limit: 4.0 },
        Join::Miter { limit: 40.0 },
        Join::Bevel,
        Join::Round,
    ];
    for join in joins {
        let found = stroke(&corner, &StrokeStyle { join, ..style(4.0) });
        check(&found, &polyline_reference(&points, 2.0, join), area, 4.0);
    }
}

#[test]
fn inside_and_outside_follow_the_fill_of_a_closed_path() {
    let corners = [(0.0, 0.0), (40.0, 0.0), (40.0, 30.0), (0.0, 30.0)];
    let area = bounds((-8.0, -8.0), (48.0, 38.0));
    let grown = |margin: f64| rectangle((-margin, -margin), (40.0 + margin, 30.0 + margin));
    for reversed in [false, true] {
        let mut points = corners.to_vec();
        if reversed {
            points.reverse();
        }
        let shape = polyline(&points, true);
        let aligned = |align, offset, join| {
            stroke(
                &shape,
                &StrokeStyle {
                    align,
                    offset,
                    join,
                    ..style(4.0)
                },
            )
        };
        let half = StrokeOffset::default();
        let miter = Join::default();
        let inside = aligned(Align::Inside, half, miter);
        check(&inside, &minus(grown(0.0), grown(-4.0)), area, 4.0);
        let outside = aligned(Align::Outside, half, miter);
        check(&outside, &minus(grown(4.0), grown(0.0)), area, 4.0);
        // Round joins round an outside stroke's corners: the points outside within 4.
        let round = aligned(Align::Outside, half, Join::Round);
        let rectangle = grown(0.0);
        let near: Reference<'_> = Box::new(|q: Point| {
            let (inside, distance) = rectangle(q);
            let gap = if inside { 0.0 } else { distance };
            (!inside && gap <= 4.0, (gap - 4.0).abs().min(distance))
        });
        check(&round, &near, area, 4.0);
        // An offset of 100% leaves a gap of half the width along the edge.
        let away = aligned(Align::Inside, StrokeOffset::Percent(100.0), miter);
        check(&away, &minus(grown(-2.0), grown(-6.0)), area, 4.0);
        // A negative offset sends the stroke across the edge.
        let across = aligned(Align::Inside, StrokeOffset::Units(-2.0), miter);
        check(&across, &minus(grown(4.0), grown(0.0)), area, 4.0);
    }
}

#[test]
fn the_inside_of_a_hole_is_the_shape() {
    let mut shape = polyline(&[(0.0, 0.0), (40.0, 0.0), (40.0, 40.0), (0.0, 40.0)], true);
    let hole = polyline(
        &[(10.0, 10.0), (10.0, 30.0), (30.0, 30.0), (30.0, 10.0)],
        true,
    );
    shape.push(hole.contours()[0].clone());
    let found = stroke(
        &shape,
        &StrokeStyle {
            align: Align::Inside,
            ..style(2.0)
        },
    );
    let square = |a: f64, b: f64| rectangle((a, a), (b, b));
    let expected = union(vec![
        minus(square(0.0, 40.0), square(2.0, 38.0)),
        minus(square(8.0, 32.0), square(10.0, 30.0)),
    ]);
    check(&found, &expected, bounds((-4.0, -4.0), (44.0, 44.0)), 2.0);
}

#[test]
fn left_and_right_cut_their_inner_corners() {
    // A right turn on screen, toward the normal: (1, 0) then (0, 1).
    let corner = polyline(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], false);
    let area = bounds((-3.0, -4.0), (14.0, 12.0));
    let sided = |align, join| {
        stroke(
            &corner,
            &StrokeStyle {
                align,
                join,
                ..style(2.0)
            },
        )
    };
    let right = sided(Align::Right, Join::default());
    let inner = polygon(&[
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (8.0, 10.0),
        (8.0, 2.0),
        (0.0, 2.0),
    ]);
    check(&right, &inner, area, 2.0);
    let left = sided(Align::Left, Join::default());
    let outer = polygon(&[
        (0.0, -2.0),
        (12.0, -2.0),
        (12.0, 10.0),
        (10.0, 10.0),
        (10.0, 0.0),
        (0.0, 0.0),
    ]);
    check(&left, &outer, area, 2.0);
    let left_round = sided(Align::Left, Join::Round);
    let rounded = union(vec![
        rectangle((0.0, -2.0), (10.0, 0.0)),
        rectangle((10.0, 0.0), (12.0, 10.0)),
        sector(p(10.0, 0.0), 0.0, 2.0, -FRAC_PI_2, 0.0),
    ]);
    check(&left_round, &rounded, area, 2.0);

    // A turn sharper than a right angle: uncut, the inner band would cross the other segment.
    let sharp = polyline(&[(0.0, 0.0), (10.0, 0.0), (0.0, 3.0)], false);
    let found = stroke(
        &sharp,
        &StrokeStyle {
            align: Align::Right,
            ..style(1.0)
        },
    );
    let along = Vector::new(-10.0, 3.0).normalized().expect("distinct");
    let normal = along.perpendicular();
    // The edges at distance 1 cross where y = 1 meets the second segment's offset line.
    let start = p(10.0, 0.0) + normal;
    let cross = start + along * ((1.0 - start.y) / along.y);
    let end = p(0.0, 3.0) + normal;
    let expected = polygon(&[
        (0.0, 0.0),
        (10.0, 0.0),
        (0.0, 3.0),
        (end.x, end.y),
        (cross.x, cross.y),
        (0.0, 1.0),
    ]);
    check(&found, &expected, bounds((-1.0, -1.0), (11.0, 4.0)), 1.0);
}

#[test]
fn an_open_path_swaps_sides_where_it_bends_the_other_way() {
    // Two quarter circles of radius 10 make an S, bending one way then the other at (10, 10).
    let s_curve = path(
        vec![
            quarter(p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0)),
            quarter(p(10.0, 10.0), p(10.0, 20.0), p(20.0, 20.0)),
        ],
        false,
    );
    let area = bounds((-2.0, -3.0), (23.0, 23.0));
    let (first, second) = (p(0.0, 10.0), p(20.0, 10.0));
    // The step where the sides swap: the centered stroke of the band's middle crossing the
    // path along its normal, mitered to the bands.
    let step = || rectangle((8.0, 9.0), (12.0, 11.0));
    let outside = stroke(
        &s_curve,
        &StrokeStyle {
            align: Align::Outside,
            ..style(2.0)
        },
    );
    let expected = union(vec![
        sector(first, 10.0, 12.0, -FRAC_PI_2, 0.0),
        sector(second, 10.0, 12.0, FRAC_PI_2, PI),
        step(),
    ]);
    check(&outside, &expected, area, 2.0);
    let inside = stroke(
        &s_curve,
        &StrokeStyle {
            align: Align::Inside,
            ..style(2.0)
        },
    );
    let expected = union(vec![
        sector(first, 8.0, 10.0, -FRAC_PI_2, 0.0),
        sector(second, 8.0, 10.0, FRAC_PI_2, PI),
        step(),
    ]);
    check(&inside, &expected, area, 2.0);
}

#[test]
fn a_zigzag_swaps_sides_along_the_bisectors() {
    // Right, down, right: the second corner turns the other way, and the stroke outside goes
    // over to the new outer side along that corner's bisector.
    let zigzag = polyline(
        &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (20.0, 10.0)],
        false,
    );
    let found = stroke(
        &zigzag,
        &StrokeStyle {
            align: Align::Outside,
            ..style(2.0)
        },
    );
    // Above, then right of the path, then below it. The arriving band is cut along the second
    // corner's bisector (x + y = 20), the leaving one starts there with half a miter, and the
    // step between them is the centered stroke of the band's middle crossing over: from
    // (11, 9) to (9, 11), mitered to the stretches of middle on either side.
    let expected = union(vec![
        rectangle((0.0, -2.0), (12.0, 0.0)),
        polygon(&[(10.0, 0.0), (12.0, 0.0), (12.0, 8.0), (10.0, 10.0)]),
        polygon(&[(10.0, 10.0), (20.0, 10.0), (20.0, 12.0), (8.0, 12.0)]),
        polyline_reference(
            &[(11.0, 8.0), (11.0, 9.0), (9.0, 11.0), (10.0, 11.0)],
            1.0,
            Join::default(),
        ),
    ]);
    check(&found, &expected, bounds((-2.0, -3.0), (22.0, 14.0)), 2.0);
}

#[test]
fn dashes_get_real_caps() {
    let line = polyline(&[(0.0, 0.0), (100.0, 0.0)], false);
    let dashed = StrokeStyle {
        cap: Cap::Round,
        dashes: Dashes {
            pattern: vec![10.0, 5.0],
            offset: 3.0,
            ..Dashes::default()
        },
        ..style(2.0)
    };
    let found = stroke(&line, &dashed);
    let intervals = [
        (0.0, 7.0),
        (12.0, 22.0),
        (27.0, 37.0),
        (42.0, 52.0),
        (57.0, 67.0),
        (72.0, 82.0),
        (87.0, 97.0),
    ];
    assert_eq!(found.contours().len(), intervals.len());
    let dashes: Vec<Contours> = intervals
        .iter()
        .map(|&(a, b)| polyline(&[(a, 0.0), (b, 0.0)], false))
        .collect();
    let capsules = dashes.iter().map(|dash| within(dash, 1.0)).collect();
    check(
        &found,
        &union(capsules),
        bounds((-2.0, -2.0), (102.0, 2.0)),
        2.0,
    );

    // Dots: dashes of no length, drawn by round caps.
    let dotted = StrokeStyle {
        cap: Cap::Round,
        dashes: Dashes {
            pattern: vec![0.0, 10.0],
            ..Dashes::default()
        },
        ..style(4.0)
    };
    let found = stroke(&line, &dotted);
    assert_eq!(found.contours().len(), 11);
    let discs = (0..=10)
        .map(|i| disc(p(f64::from(i) * 10.0, 0.0), 2.0))
        .collect();
    check(
        &found,
        &union(discs),
        bounds((-3.0, -3.0), (103.0, 3.0)),
        4.0,
    );
}

#[test]
fn dashes_follow_the_arc_length_of_curves_and_join_over_the_start() {
    let radius = 20.0;
    let circle = crate::Path::exact_ellipse(p(0.0, 0.0), Vector::new(radius, radius)).contours();
    // Ten dashes and ten gaps of equal length, offset so that one dash runs over the start.
    let length = 2.0 * PI * radius / 20.0;
    let dashed = StrokeStyle {
        dashes: Dashes {
            pattern: vec![length, length],
            offset: length * 0.5,
            ..Dashes::default()
        },
        ..style(2.0)
    };
    let found = stroke(&circle, &dashed);
    assert_eq!(found.contours().len(), 10, "the dash over the start is one");
    // The exact circle starts at angle 0 and runs toward y.
    let sectors = (0..10)
        .map(|i| {
            let from = (f64::from(i) * 2.0 - 0.5) * length / radius;
            sector(p(0.0, 0.0), 19.0, 21.0, from, from + length / radius)
        })
        .collect();
    check(
        &found,
        &union(sectors),
        bounds((-22.0, -22.0), (22.0, 22.0)),
        2.0,
    );
}

#[test]
fn anchored_dashes_stretch_by_their_fit() {
    let line = polyline(&[(0.0, 0.0), (100.0, 0.0)], false);
    let third = 25.0 / 3.0;
    let cases = [
        (
            DashFit::ScaleBoth,
            [(29.5, 38.5), (43.0, 52.0), (56.5, 65.5)],
        ),
        (
            DashFit::AdaptDashes,
            [
                (30.0, 30.0 + third),
                (35.0 + third, 35.0 + 2.0 * third),
                (40.0 + 2.0 * third, 65.0),
            ],
        ),
        (
            DashFit::AdaptGaps,
            [(28.75, 38.75), (42.5, 52.5), (56.25, 66.25)],
        ),
    ];
    for (fit, between) in cases {
        let anchors = [0.2, 0.75].map(|at| DashAnchor {
            contour: 0,
            at,
            center: DashCenter::Dash,
            size: None,
        });
        let dashed = StrokeStyle {
            dashes: Dashes {
                pattern: vec![10.0, 5.0],
                fit,
                anchors: anchors.to_vec(),
                ..Dashes::default()
            },
            ..style(2.0)
        };
        let found = stroke(&line, &dashed);
        let mut intervals = vec![(0.0, 10.0), (15.0, 25.0), (70.0, 80.0), (85.0, 95.0)];
        intervals.extend(between);
        let parts = intervals
            .iter()
            .map(|&(a, b)| rectangle((a, -1.0), (b, 1.0)))
            .collect();
        check(
            &found,
            &union(parts),
            bounds((-2.0, -2.0), (102.0, 2.0)),
            2.0,
        );
    }
}

/// The area of `inside`, sampled at the centers of a grid of `steps` × `steps` over `area`.
fn sampled_area(inside: &dyn Fn(Point) -> bool, area: Rect, steps: u32) -> f64 {
    let (width, height) = (area.max.x - area.min.x, area.max.y - area.min.y);
    let cell = width * height / f64::from(steps * steps);
    let mut count = 0_u32;
    for i in 0..steps {
        for j in 0..steps {
            let q = p(
                area.min.x + width * (f64::from(i) + 0.5) / f64::from(steps),
                area.min.y + height * (f64::from(j) + 0.5) / f64::from(steps),
            );
            count += u32::from(inside(q));
        }
    }
    f64::from(count) * cell
}

/// Checks that `outline` covers the area of `reference` once, where the stroke overlaps
/// itself: the same sampled area as the reference, near `exact`, far from the sum of its parts.
#[track_caller]
fn assert_area(outline: &Contours, reference: &Reference<'_>, area: Rect, exact: f64) {
    let found = sampled_area(&|q| outline.contains(q, FillRule::NonZero), area, 250);
    let expected = sampled_area(&|q| reference(q).0, area, 250);
    let cell = (area.max.x - area.min.x) * (area.max.y - area.min.y) / 62_500.0;
    assert!(
        (found - expected).abs() <= 4.0 * cell,
        "{found} != {expected}"
    );
    assert!((found - exact).abs() <= exact * 0.02, "{found} != {exact}");
}

#[test]
fn a_stroke_over_itself_is_one_region() {
    // There and back along the same line: one rectangle and a half disc, not two rectangles.
    let back = polyline(&[(0.0, 0.0), (10.0, 0.0), (0.0, 0.0)], false);
    let found = stroke(
        &back,
        &StrokeStyle {
            join: Join::Round,
            ..style(2.0)
        },
    );
    let reference = union(vec![
        rectangle((0.0, -1.0), (10.0, 1.0)),
        sector(p(10.0, 0.0), 0.0, 1.0, -FRAC_PI_2, FRAC_PI_2),
    ]);
    let area = bounds((-1.0, -1.5), (11.5, 1.5));
    assert_area(&found, &reference, area, 20.0 + PI / 2.0);
    check(&found, &reference, area, 2.0);
    // Two subpaths crossing: 2 × 20, less the 4 they share.
    let mut cross = polyline(&[(0.0, 5.0), (10.0, 5.0)], false);
    cross.push(polyline(&[(5.0, 0.0), (5.0, 10.0)], false).contours()[0].clone());
    let found = stroke(&cross, &style(2.0));
    let reference = union(vec![
        rectangle((0.0, 4.0), (10.0, 6.0)),
        rectangle((4.0, 0.0), (6.0, 10.0)),
    ]);
    assert_area(&found, &reference, bounds((-1.0, -1.0), (11.0, 11.0)), 36.0);
    // A figure of eight crossing itself: the union of its parts, as its reference.
    let points = [(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)];
    let eight = polyline(&points, false);
    let found = stroke(&eight, &style(2.0));
    let reference = polyline_reference(&points, 1.0, Join::default());
    check(&found, &reference, bounds((-3.0, -3.0), (13.0, 13.0)), 2.0);
}

#[test]
fn degenerate_inputs_give_nothing_or_a_dot() {
    let line = polyline(&[(0.0, 0.0), (10.0, 0.0)], false);
    assert!(stroke(&line, &style(0.0)).is_empty());
    assert!(stroke(&line, &style(f64::NAN)).is_empty());
    assert!(stroke(&Contours::default(), &style(1.0)).is_empty());
    // A subpath of no length: a dot with round caps, nothing with butt ones.
    let point = polyline(&[(5.0, 5.0), (5.0, 5.0)], false);
    assert!(stroke(&point, &style(2.0)).is_empty());
    let dot = stroke(
        &point,
        &StrokeStyle {
            cap: Cap::Round,
            ..style(2.0)
        },
    );
    check(
        &dot,
        &disc(p(5.0, 5.0), 1.0),
        bounds((3.0, 3.0), (7.0, 7.0)),
        2.0,
    );
}

#[test]
fn outlines_lie_within_their_reach_of_the_path() {
    // Sharp corners, open ends and a closed star, under every cap, join and alignment.
    let zigzag = polyline(&[(0.0, 0.0), (10.0, 1.0), (0.0, 2.0), (10.0, 6.0)], false);
    let star = polyline(
        &[
            (0.0, -10.0),
            (2.0, -2.0),
            (10.0, 0.0),
            (2.0, 2.0),
            (0.0, 10.0),
            (-2.0, 2.0),
            (-10.0, 0.0),
            (-2.0, -2.0),
        ],
        true,
    );
    let caps = [
        Cap::Butt { tilt: 0.0 },
        Cap::Butt { tilt: 0.6 },
        Cap::Round,
        Cap::Square,
        Cap::Taper { length: 0.0 },
        Cap::Taper { length: 5.0 },
    ];
    let joins = [
        Join::Miter { limit: 4.0 },
        Join::Miter { limit: 20.0 },
        Join::Round,
        Join::Bevel,
    ];
    let aligns = [
        Align::Center,
        Align::Inside,
        Align::Outside,
        Align::Left,
        Align::Right,
    ];
    let offsets = [
        StrokeOffset::Percent(50.0),
        StrokeOffset::Percent(-120.0),
        StrokeOffset::Units(1.5),
    ];
    for contours in [&zigzag, &star] {
        let bounds = contours.bounds();
        for cap in caps {
            for join in joins {
                for align in aligns {
                    for offset in offsets {
                        let style = StrokeStyle {
                            width: 1.2,
                            align,
                            offset,
                            cap,
                            join,
                            dashes: Dashes::default(),
                        };
                        let outline = stroke(contours, &style);
                        let reach = bounds.inflated(style.reach() + 1e-6);
                        assert!(
                            reach.contains_rect(outline.bounds()),
                            "{style:?}: {:?} beyond {reach:?}",
                            outline.bounds()
                        );
                    }
                }
            }
        }
    }
    assert_eq!(
        StrokeStyle {
            width: f64::NAN,
            ..style(1.0)
        }
        .reach(),
        f64::INFINITY
    );
}

#[test]
fn a_style_scaled_outlines_the_scaled_path() {
    let path = polyline(&[(0.0, 0.0), (10.0, 1.0), (4.0, 6.0)], false);
    let style = StrokeStyle {
        width: 1.0,
        align: Align::Left,
        offset: StrokeOffset::Units(0.8),
        cap: Cap::Taper { length: 3.0 },
        join: Join::Round,
        dashes: Dashes {
            pattern: vec![2.0, 1.0],
            offset: 0.5,
            ..Dashes::default()
        },
    };
    let scaled = style.scaled(4.0);
    assert!((scaled.width - 4.0).abs() < 1e-12);
    assert_eq!(scaled.offset, StrokeOffset::Units(3.2));
    assert_eq!(scaled.cap, Cap::Taper { length: 12.0 });
    assert_eq!(scaled.dashes.pattern, [8.0, 4.0]);
    assert!((scaled.reach() - 4.0 * style.reach()).abs() < 1e-9);
    // The outline of the scaled path in the scaled style is the outline scaled.
    let big = path.transformed(crate::Affine::scale(4.0, 4.0));
    let (a, b) = (
        stroke(&big, &scaled).bounds(),
        stroke(&path, &style).bounds(),
    );
    for (x, y) in [
        (a.min.x, b.min.x),
        (a.min.y, b.min.y),
        (a.max.x, b.max.x),
        (a.max.y, b.max.y),
    ] {
        assert!((x - 4.0 * y).abs() < 1e-3, "{x} against {}", 4.0 * y);
    }
}
