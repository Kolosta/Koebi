//! Where two monotone pieces of curve cross.
//!
//! Each piece is monotone in x and in y, so the box of its end points bounds it exactly. Two
//! pieces are halved, the longer first, while their boxes meet; when both are flat, their
//! chords cross where they do, refined by Newton's method on the curves themselves. Pieces that
//! run along each other (the same curve twice, or two lines on one line) are recognized by
//! their ends lying on each other, and give the ends of the stretch they share.

use crate::{Point, Rect, Segment, Vector};

/// A crossing: the parameter on each piece, and the point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Crossing {
    pub(super) t: f64,
    pub(super) u: f64,
    pub(super) point: Point,
}

/// The bounds of a monotone piece: the box of its ends.
pub(super) fn monotone_bounds(segment: &Segment) -> Rect {
    Rect::EMPTY
        .including(segment.start())
        .including(segment.end())
}

/// How far a piece's control points stray from its chord.
fn flatness(segment: &Segment) -> f64 {
    let (start, end) = (segment.start(), segment.end());
    let chord = end - start;
    let length = chord.length();
    let controls: &[Point] = match segment {
        Segment::Line(_) => return 0.0,
        Segment::Quad(p) => &p[1..2],
        Segment::Cubic(p) => &p[1..3],
        Segment::Rational(r) => &r.points()[1..r.degree()],
    };
    controls
        .iter()
        .map(|p| {
            if length > 0.0 {
                chord.cross(*p - start).abs() / length
            } else {
                p.distance(start)
            }
        })
        .fold(0.0, f64::max)
}

/// Where the lines `a0 a1` and `b0 b1` cross: the share along each, or `None` when parallel.
fn chords(a0: Point, a1: Point, b0: Point, b1: Point) -> Option<(f64, f64)> {
    let (da, db) = (a1 - a0, b1 - b0);
    let denominator = da.cross(db);
    if denominator == 0.0 {
        return None;
    }
    let offset = b0 - a0;
    Some((
        offset.cross(db) / denominator,
        offset.cross(da) / denominator,
    ))
}

/// Refines a crossing of `a` and `b` near `(t, u)` by Newton's method. `None` when it does
/// not converge (curves that touch rather than cross converge slowly: the caller keeps its
/// estimate).
fn refine(a: &Segment, b: &Segment, mut t: f64, mut u: f64, epsilon: f64) -> Option<(f64, f64)> {
    for _ in 0..16 {
        let gap = a.eval(t) - b.eval(u);
        if gap.length() <= epsilon * 1e-3 {
            return Some((t, u));
        }
        let (column_a, column_b) = (a.derivative(t), -b.derivative(u));
        let determinant = column_a.cross(column_b);
        if determinant.abs() <= f64::MIN_POSITIVE {
            return None;
        }
        let rhs = -gap;
        t = (t + rhs.cross(column_b) / determinant).clamp(0.0, 1.0);
        u = (u + column_a.cross(rhs) / determinant).clamp(0.0, 1.0);
    }
    (a.eval(t).distance(b.eval(u)) <= epsilon).then_some((t, u))
}

/// Halvings at most: a part of a piece then spans 2⁻⁴⁸ of it.
const DEPTH: u32 = 48;

/// More flat crossings than this between two pieces means they run along each other.
const ALONG: usize = 64;

/// A pair of parts still to look at: each part, its parameter range, and its depth.
pub(super) type Pair = (Segment, (f64, f64), Segment, (f64, f64), u32);

/// Appends where `a` and `b`, two monotone pieces, cross or meet, within `epsilon` (a distance
/// below which points are one). `stack` is scratch.
pub(super) fn crossings(
    a: &Segment,
    b: &Segment,
    epsilon: f64,
    stack: &mut Vec<Pair>,
    out: &mut Vec<Crossing>,
) {
    let first = out.len();
    if let (Segment::Line([a0, a1]), Segment::Line([b0, b1])) = (a, b) {
        lines(*a0, *a1, *b0, *b1, epsilon, out);
        return;
    }
    stack.clear();
    stack.push((*a, (0.0, 1.0), *b, (0.0, 1.0), 0));
    let mut found = 0;
    while let Some((part_a, range_a, part_b, range_b, depth)) = stack.pop() {
        let (box_a, box_b) = (monotone_bounds(&part_a), monotone_bounds(&part_b));
        if !box_a.inflated(epsilon).intersects(box_b)
            || outside_band(&part_a, &part_b, epsilon)
            || outside_band(&part_b, &part_a, epsilon)
        {
            continue;
        }
        if let Some(point) = apart_but_at(&part_a, &part_b, epsilon) {
            // They leave a common point in directions that keep them apart: they meet there
            // only, at the parts' ends.
            let t = if point == part_a.start() {
                range_a.0
            } else {
                range_a.1
            };
            let u = if point == part_b.start() {
                range_b.0
            } else {
                range_b.1
            };
            out.push(Crossing { t, u, point });
            continue;
        }
        // Flat enough for a first guess from the chords, which Newton's method then makes
        // exact; where it cannot (curves that touch), halve on down to flat within epsilon.
        let size = |r: Rect| r.width().max(r.height());
        let rough = 1e-3 * size(box_a).max(size(box_b));
        let (flat_a, flat_b) = (flatness(&part_a), flatness(&part_b));
        let exact = flat_a <= epsilon && flat_b <= epsilon;
        if exact || depth >= DEPTH || (flat_a <= rough && flat_b <= rough) {
            let guess = chords(part_a.start(), part_a.end(), part_b.start(), part_b.end())
                .filter(|(sa, sb)| (-0.5..=1.5).contains(sa) && (-0.5..=1.5).contains(sb));
            let lerp =
                |range: (f64, f64), s: f64| range.0 + (range.1 - range.0) * s.clamp(0.0, 1.0);
            let refined = guess.and_then(|(sa, sb)| {
                let (t, u) = (lerp(range_a, sa), lerp(range_b, sb));
                refine(a, b, t, u, epsilon)
            });
            let settled = match (refined, guess) {
                (Some(found), _) => Some(found),
                (None, Some((sa, sb))) if exact || depth >= DEPTH => {
                    Some((lerp(range_a, sa), lerp(range_b, sb)))
                }
                (None, None) if exact || depth >= DEPTH => None,
                // Not settled yet: halve on.
                _ => {
                    split_larger(&mut *stack, (part_a, range_a, part_b, range_b, depth));
                    continue;
                }
            };
            found += 1;
            if found > ALONG {
                // Too many: the pieces run along each other.
                out.truncate(first);
                along(a, b, epsilon, out);
                return;
            }
            if let Some((t, u)) = settled {
                let (pa, pb) = (a.eval(t), b.eval(u));
                if pa.distance(pb) <= epsilon * 4.0 {
                    out.push(Crossing {
                        t,
                        u,
                        point: pa.midpoint(pb),
                    });
                }
            }
            continue;
        }
        split_larger(&mut *stack, (part_a, range_a, part_b, range_b, depth));
    }
    // Neighbouring flat parts find the same crossing: keep one.
    let mut found: Vec<Crossing> = out.drain(first..).collect();
    found.sort_unstable_by(|x, y| x.t.total_cmp(&y.t));
    for crossing in found {
        let duplicate = out[first..]
            .iter()
            .any(|kept| kept.point.distance(crossing.point) <= epsilon * 4.0);
        if !duplicate {
            out.push(crossing);
        }
    }
}

/// The control points of a part (which hold it).
fn controls(segment: &Segment) -> &[Point] {
    match segment {
        Segment::Line(p) => &p[..],
        Segment::Quad(p) => &p[..],
        Segment::Cubic(p) => &p[..],
        Segment::Rational(r) => r.points(),
    }
}

/// Whether `b` lies wholly to one side of the band that holds `a` along its chord (Bézier
/// clipping's fat line): then they cannot meet.
fn outside_band(a: &Segment, b: &Segment, epsilon: f64) -> bool {
    let start = a.start();
    let Some(normal) = (a.end() - start).normalized().map(Vector::perpendicular) else {
        return false;
    };
    let range = |points: &[Point]| {
        points
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                let d = (*p - start).dot(normal);
                (lo.min(d), hi.max(d))
            })
    };
    let ((low, high), (b_low, b_high)) = (range(controls(a)), range(controls(b)));
    b_low > high + epsilon || b_high < low - epsilon
}

/// The end point `a` and `b` share, when they meet nowhere else: seen from it, each lies in the
/// cone of its control points (which hold it), and the two cones are apart.
fn apart_but_at(a: &Segment, b: &Segment, epsilon: f64) -> Option<Point> {
    let shared = [a.start(), a.end()]
        .into_iter()
        .find(|p| b.start().distance(*p) <= epsilon || b.end().distance(*p) <= epsilon)?;
    // The cone of a part from `shared`: its chord's direction, and how far its control points
    // turn from it.
    let cone = |segment: &Segment| -> Option<(f64, f64)> {
        let far = if segment.start().distance(shared) <= epsilon {
            segment.end()
        } else {
            segment.start()
        };
        let axis = (far - shared).normalized()?;
        let mut spread: f64 = 0.0;
        for control in controls(segment) {
            if control.distance(shared) <= epsilon {
                continue;
            }
            let direction = (*control - shared).normalized()?;
            spread = spread.max(axis.cross(direction).atan2(axis.dot(direction)).abs());
        }
        Some((axis.y.atan2(axis.x), spread))
    };
    let ((angle_a, spread_a), (angle_b, spread_b)) = (cone(a)?, cone(b)?);
    let between = (angle_a - angle_b).rem_euclid(core::f64::consts::TAU);
    let between = between.min(core::f64::consts::TAU - between);
    (spread_a + spread_b + 1e-9 < between && spread_a < 1.5 && spread_b < 1.5).then_some(shared)
}

/// Pushes the two halves of the larger part of `pair`, each with the other part.
fn split_larger(stack: &mut Vec<Pair>, (part_a, range_a, part_b, range_b, depth): Pair) {
    let size = |r: Rect| r.width().max(r.height());
    let middle = |range: (f64, f64)| range.0.midpoint(range.1);
    if size(monotone_bounds(&part_a)) >= size(monotone_bounds(&part_b)) {
        let (left, right) = part_a.split(0.5);
        let m = middle(range_a);
        stack.push((left, (range_a.0, m), part_b, range_b, depth + 1));
        stack.push((right, (m, range_a.1), part_b, range_b, depth + 1));
    } else {
        let (left, right) = part_b.split(0.5);
        let m = middle(range_b);
        stack.push((part_a, range_a, left, (range_b.0, m), depth + 1));
        stack.push((part_a, range_a, right, (m, range_b.1), depth + 1));
    }
}

/// Two lines: where they cross, or the ends of the stretch they share when on one line.
fn lines(a0: Point, a1: Point, b0: Point, b1: Point, epsilon: f64, out: &mut Vec<Crossing>) {
    let (da, db) = (a1 - a0, b1 - b0);
    let (la, lb) = (da.length(), db.length());
    if la <= 0.0 || lb <= 0.0 {
        return;
    }
    // Distances of each end to the other line.
    let off_b = |p: Point| (p - b0).cross(db) / lb;
    if off_b(a0).abs() <= epsilon && off_b(a1).abs() <= epsilon {
        // On one line: the ends of each that lie on the other.
        let along_a = |p: Point| (p - a0).dot(da) / (la * la);
        let along_b = |p: Point| (p - b0).dot(db) / (lb * lb);
        let within = |s: f64, length: f64| s >= -epsilon / length && s <= 1.0 + epsilon / length;
        for (p, t, u) in [
            (a0, 0.0, along_b(a0)),
            (a1, 1.0, along_b(a1)),
            (b0, along_a(b0), 0.0),
            (b1, along_a(b1), 1.0),
        ] {
            if within(t, la) && within(u, lb) {
                let crossing = Crossing {
                    t: t.clamp(0.0, 1.0),
                    u: u.clamp(0.0, 1.0),
                    point: p,
                };
                if !out.iter().any(|kept| kept.point.distance(p) <= epsilon) {
                    out.push(crossing);
                }
            }
        }
        return;
    }
    let Some((t, u)) = chords(a0, a1, b0, b1) else {
        return;
    };
    // Within epsilon of either piece, measured along it.
    let (slack_a, slack_b) = (epsilon / la, epsilon / lb);
    if t >= -slack_a && t <= 1.0 + slack_a && u >= -slack_b && u <= 1.0 + slack_b {
        let (t, u) = (t.clamp(0.0, 1.0), u.clamp(0.0, 1.0));
        out.push(Crossing {
            t,
            u,
            point: a0.lerp(a1, t).midpoint(b0.lerp(b1, u)),
        });
    }
}

/// Two pieces that run along each other: the ends of each that lie on the other.
fn along(a: &Segment, b: &Segment, epsilon: f64, out: &mut Vec<Crossing>) {
    let on = |segment: &Segment, p: Point| {
        let (t, squared) = segment.nearest(p);
        (squared.sqrt() <= epsilon * 16.0).then_some(t)
    };
    for (p, t, other) in [
        (a.start(), 0.0, on(b, a.start())),
        (a.end(), 1.0, on(b, a.end())),
    ] {
        if let Some(u) = other {
            out.push(Crossing { t, u, point: p });
        }
    }
    for (p, u, other) in [
        (b.start(), 0.0, on(a, b.start())),
        (b.end(), 1.0, on(a, b.end())),
    ] {
        if let Some(t) = other
            && !out
                .iter()
                .any(|kept| kept.point.distance(p) <= epsilon * 4.0)
        {
            out.push(Crossing { t, u, point: p });
        }
    }
}
