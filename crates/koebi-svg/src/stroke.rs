//! Strokes turned into fills at compile time: every stroke becomes closed loops of quadratic
//! Béziers whose union, filled with the nonzero rule, covers what the stroke paints.
//!
//! - Each piece of the path (a line, or a quadratic of the path made of quadratics) becomes the
//!   band between its two offset curves, half the width on each side, themselves quadratics
//!   within the tolerance. A piece bending more tightly than half the width (its inner offset
//!   would fold back) is stroked as a fine polyline instead.
//! - Where two pieces meet at an angle, the join fills the outer side: a pie wedge of 45° arcs
//!   (round), the miter's tip within the miter limit, or a bevel. Pieces meeting tangentially
//!   (the quadratics of one cubic) need none.
//! - Open ends get their cap: a half disc (round), a half square (square), or nothing (butt).
//!
//! Loops overlap; all turn the same way, so that their windings add up and the renderer's
//! coverage, clamped, shows no seam where they meet.

use crate::geometry::{Outline, Point, Segment};

/// How a stroke ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Cap {
    /// At the end, square.
    #[default]
    Butt,
    /// A half disc past the end.
    Round,
    /// A half square past the end.
    Square,
}

/// How a stroke turns a corner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Join {
    /// Sharp, cut to a bevel past the miter limit.
    #[default]
    Miter,
    /// Round.
    Round,
    /// Cut straight across.
    Bevel,
}

/// The look of a stroke.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeStyle {
    /// Its width, in the outline's units.
    pub width: f32,
    /// Its ends.
    pub cap: Cap,
    /// Its corners.
    pub join: Join,
    /// How long a miter may be, in widths, before it becomes a bevel.
    pub miter_limit: f32,
}

/// A closed loop of quadratics.
type Loop = Vec<[Point; 3]>;

/// The largest arc one quadratic draws: its distance to the circle stays below 0.03 % of the
/// radius.
const ARC_STEP: f32 = core::f32::consts::FRAC_PI_4;

/// How many times an offset is halved, at most, to come within the tolerance.
const MAX_OFFSET_DEPTH: u32 = 10;

/// The loops covering `outline` stroked with `style`, within `tolerance`.
pub(crate) fn outline(outline: &Outline, style: StrokeStyle, tolerance: f32) -> Vec<Loop> {
    let half = style.width * 0.5;
    let mut loops = Vec::new();
    if half.is_nan() || half <= 0.0 {
        return loops;
    }
    for subpath in &outline.subpaths {
        let (Some(first), Some(last)) = (subpath.segments.first(), subpath.segments.last()) else {
            continue;
        };
        let mut quads = Vec::new();
        for segment in &subpath.segments {
            segment.quads(tolerance, &mut quads);
        }
        if subpath.closed && last.end().sub(first.start()).length() > 1e-7 {
            Segment::Line(last.end(), first.start()).quads(tolerance, &mut quads);
        }
        quads.retain(|&[a, b, c]| b.sub(a).length() + c.sub(b).length() > 1e-7);
        let (Some(&start), Some(&end)) = (quads.first(), quads.last()) else {
            // A lone point: only round and square caps paint it.
            let at = first.start();
            match style.cap {
                Cap::Round => loops.push(disc(at, half)),
                Cap::Square => loops.push(square(at, half)),
                Cap::Butt => {}
            }
            continue;
        };
        for &quad in &quads {
            piece(quad, half, tolerance, &mut loops);
        }
        let count = quads.len();
        let joins = if subpath.closed { count } else { count - 1 };
        for index in 0..joins {
            let (from, to) = (quads[index], quads[(index + 1) % count]);
            join(
                from[2],
                end_tangent(from),
                start_tangent(to),
                half,
                style,
                tolerance,
                &mut loops,
            );
        }
        if !subpath.closed {
            cap(
                start[0],
                start_tangent(start).scale(-1.0),
                half,
                style.cap,
                &mut loops,
            );
            cap(end[2], end_tangent(end), half, style.cap, &mut loops);
        }
    }
    for polygon in &mut loops {
        orient(polygon);
    }
    loops
}

fn line(from: Point, to: Point) -> [Point; 3] {
    [from, from.lerp(to, 0.5), to]
}

/// The closed polygon through `points`, as straight quadratics.
fn polygon(points: &[Point]) -> Loop {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(&from, &to)| line(from, to))
        .collect()
}

fn quad_at([a, b, c]: [Point; 3], t: f32) -> Point {
    a.lerp(b, t).lerp(b.lerp(c, t), t)
}

/// The direction of the curve at `t`, of unit length: where its derivative vanishes (a control
/// point on an end), that of its chord.
fn direction([a, b, c]: [Point; 3], t: f32) -> Point {
    let derivative = b.sub(a).lerp(c.sub(b), t);
    if derivative.length() > 1e-9 {
        derivative.normalized()
    } else {
        c.sub(a).normalized()
    }
}

fn start_tangent(quad: [Point; 3]) -> Point {
    direction(quad, 0.0)
}

fn end_tangent(quad: [Point; 3]) -> Point {
    direction(quad, 1.0)
}

/// The smallest radius of curvature of `quad`: infinite for a straight one.
fn min_radius([a, b, c]: [Point; 3]) -> f32 {
    // B'(t) = 2 (e + t d), B'' = 2 d: their cross product is constant, the radius smallest
    // where the speed is.
    let e = b.sub(a);
    let d = a.sub(b.scale(2.0)).add(c);
    let bend = e.cross(d).abs();
    if bend < 1e-12 {
        return f32::INFINITY;
    }
    let squared = d.dot(d);
    let t = if squared > 1e-12 {
        (-e.dot(d) / squared).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let speed = e.add(d.scale(t)).length();
    2.0 * speed * speed * speed / bend
}

/// Whether `quad` is a straight segment: its control on the chord, between the ends.
fn is_straight([a, b, c]: [Point; 3]) -> bool {
    let chord = c.sub(a);
    let length = chord.dot(chord);
    let along = b.sub(a).dot(chord);
    b.sub(a).cross(chord).abs() <= 1e-6 * length && (0.0..=length).contains(&along)
}

/// The band covering `quad`, `half` on each side.
fn piece(quad: [Point; 3], half: f32, tolerance: f32, loops: &mut Vec<Loop>) {
    let [a, b, c] = quad;
    if is_straight(quad) {
        let normal = c.sub(a).normalized().perpendicular().scale(half);
        loops.push(polygon(&[
            a.add(normal),
            c.add(normal),
            c.sub(normal),
            a.sub(normal),
        ]));
        return;
    }
    if min_radius(quad) < half * 1.01 {
        // Bending more tightly than the stroke is wide: a fine polyline, its corners round.
        let mut points = vec![a];
        Segment::Quad(a, b, c).flatten(tolerance, &mut points);
        for pair in points.windows(2) {
            if pair[1].sub(pair[0]).length() > 1e-7 {
                piece(line(pair[0], pair[1]), half, tolerance, loops);
            }
        }
        for three in points.windows(3) {
            let incoming = three[1].sub(three[0]).normalized();
            let outgoing = three[2].sub(three[1]).normalized();
            wedge(three[1], incoming, outgoing, half, loops);
        }
        return;
    }
    let mut left = Vec::new();
    offset(quad, half, (0.0, 1.0), tolerance, 0, &mut left);
    let mut right = Vec::new();
    offset(quad, -half, (0.0, 1.0), tolerance, 0, &mut right);
    let (Some(left_first), Some(left_last)) = (left.first().copied(), left.last().copied()) else {
        return;
    };
    let (Some(right_first), Some(right_last)) = (right.first().copied(), right.last().copied())
    else {
        return;
    };
    let mut band = left;
    band.push(line(left_last[2], right_last[2]));
    band.extend(right.iter().rev().map(|&[a, b, c]| [c, b, a]));
    band.push(line(right_first[0], left_first[0]));
    loops.push(band);
}

/// The curve `distance` to the side of `quad` (along its perpendicular) over the parameters
/// `span`, as quadratics within `tolerance`, added to `out`.
fn offset(
    quad: [Point; 3],
    distance: f32,
    span: (f32, f32),
    tolerance: f32,
    depth: u32,
    out: &mut Vec<[Point; 3]>,
) {
    let (from, to) = span;
    let (d0, d1) = (direction(quad, from), direction(quad, to));
    let start = quad_at(quad, from).add(d0.perpendicular().scale(distance));
    let end = quad_at(quad, to).add(d1.perpendicular().scale(distance));
    // The control point where the tangents at both ends meet.
    let turn = d0.cross(d1);
    let control = if turn.abs() < 1e-6 {
        start.lerp(end, 0.5)
    } else {
        start.add(d0.scale(end.sub(start).cross(d1) / turn))
    };
    let middle = f32::midpoint(from, to);
    let exact = quad_at(quad, middle).add(direction(quad, middle).perpendicular().scale(distance));
    let approximate = quad_at([start, control, end], 0.5);
    let wide = d0.dot(d1) < 0.5;
    if depth < MAX_OFFSET_DEPTH && (wide || approximate.sub(exact).length() > tolerance) {
        offset(quad, distance, (from, middle), tolerance, depth + 1, out);
        offset(quad, distance, (middle, to), tolerance, depth + 1, out);
    } else {
        out.push([start, control, end]);
    }
}

/// The outer side of the corner at `at`, between the directions `incoming` and `outgoing`.
fn join(
    at: Point,
    incoming: Point,
    outgoing: Point,
    half: f32,
    style: StrokeStyle,
    tolerance: f32,
    loops: &mut Vec<Loop>,
) {
    let angle = incoming.cross(outgoing).atan2(incoming.dot(outgoing)).abs();
    // Tangent pieces: the sliver between their ends is narrower than the tolerance.
    if half * angle <= tolerance {
        return;
    }
    let (n1, n2) = outer_normals(incoming, outgoing, half);
    match style.join {
        Join::Round => wedge(at, incoming, outgoing, half, loops),
        Join::Bevel => loops.push(polygon(&[at, at.add(n1), at.add(n2)])),
        Join::Miter => {
            // The miter's length over the stroke's width is 1 / sin(half the angle between the
            // pieces), 1 / cos(half the turn).
            let cos = incoming.dot(outgoing).clamp(-1.0, 1.0);
            let ratio = 1.0 / f32::midpoint(1.0, cos).sqrt().max(1e-6);
            if ratio > style.miter_limit {
                loops.push(polygon(&[at, at.add(n1), at.add(n2)]));
            } else {
                let tip = at.add(n1.add(n2).normalized().scale(half * ratio));
                loops.push(polygon(&[at, at.add(n1), tip, at.add(n2)]));
            }
        }
    }
}

/// The normals on the outer side of a corner, `half` long, before and after it.
fn outer_normals(incoming: Point, outgoing: Point, half: f32) -> (Point, Point) {
    let side = if incoming.cross(outgoing) > 0.0 {
        -1.0
    } else {
        1.0
    };
    (
        incoming.perpendicular().scale(half * side),
        outgoing.perpendicular().scale(half * side),
    )
}

/// A round join: the pie wedge from the end of one piece to the start of the next.
fn wedge(at: Point, incoming: Point, outgoing: Point, half: f32, loops: &mut Vec<Loop>) {
    let (n1, n2) = outer_normals(incoming, outgoing, half);
    let from = n1.y.atan2(n1.x);
    let sweep = n1.cross(n2).atan2(n1.dot(n2));
    if sweep.abs() < 1e-6 {
        return;
    }
    let mut wedge = vec![line(at, at.add(n1))];
    arc(at, half, from, sweep, &mut wedge);
    wedge.push(line(at.add(n2), at));
    loops.push(wedge);
}

/// An open end at `at`, the stroke leaving it toward `outward`.
fn cap(at: Point, outward: Point, half: f32, cap: Cap, loops: &mut Vec<Loop>) {
    let normal = outward.perpendicular().scale(half);
    match cap {
        Cap::Butt => {}
        Cap::Square => {
            let reach = outward.scale(half);
            loops.push(polygon(&[
                at.add(normal),
                at.add(normal).add(reach),
                at.sub(normal).add(reach),
                at.sub(normal),
            ]));
        }
        Cap::Round => {
            let from = normal.y.atan2(normal.x);
            // Half a turn from one side to the other, through the outward direction.
            let sweep = if normal.cross(outward) > 0.0 {
                core::f32::consts::PI
            } else {
                -core::f32::consts::PI
            };
            let mut half_disc = Vec::new();
            arc(at, half, from, sweep, &mut half_disc);
            half_disc.push(line(at.sub(normal), at.add(normal)));
            loops.push(half_disc);
        }
    }
}

/// The arc of the circle about `center` from the angle `from` turning by `sweep` (radians), as
/// quadratics of at most 45°, added to `out`.
fn arc(center: Point, radius: f32, from: f32, sweep: f32, out: &mut Vec<[Point; 3]>) {
    let pieces = (sweep.abs() / ARC_STEP).ceil().max(1.0);
    let step = sweep / pieces;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "at most eight pieces for a full turn"
    )]
    let count = pieces as usize;
    let on = |angle: f32| center.add(Point::new(angle.cos(), angle.sin()).scale(radius));
    for index in 0..count {
        let start = from + step * index as f32;
        let middle = start + step * 0.5;
        // The control point where the tangents at both ends meet: on the bisector.
        let control =
            center.add(Point::new(middle.cos(), middle.sin()).scale(radius / (step * 0.5).cos()));
        out.push([on(start), control, on(start + step)]);
    }
}

/// A disc of radius `radius` about `center`.
fn disc(center: Point, radius: f32) -> Loop {
    let mut out = Vec::new();
    arc(center, radius, 0.0, core::f32::consts::TAU, &mut out);
    out
}

fn square(center: Point, half: f32) -> Loop {
    polygon(&[
        center.add(Point::new(-half, -half)),
        center.add(Point::new(half, -half)),
        center.add(Point::new(half, half)),
        center.add(Point::new(-half, half)),
    ])
}

/// The signed area of a loop (positive counterclockwise in y-up terms), from its control
/// polygon: enough to tell which way it turns.
fn area(curves: &[[Point; 3]]) -> f32 {
    curves
        .iter()
        .map(|&[a, b, c]| a.cross(b) + b.cross(c))
        .sum()
}

/// Turns `curves` counterclockwise (in y-up terms: a positive signed area), so that every
/// loop of a stroke winds the same way.
fn orient(curves: &mut Loop) {
    if area(curves) < 0.0 {
        curves.reverse();
        for curve in curves.iter_mut() {
            curve.reverse();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Subpath, monotonic};
    use crate::raster::winding;

    fn polyline(points: &[(f32, f32)], closed: bool) -> Outline {
        let points: Vec<Point> = points.iter().map(|&(x, y)| Point::new(x, y)).collect();
        Outline {
            subpaths: vec![Subpath {
                segments: points
                    .windows(2)
                    .map(|p| Segment::Line(p[0], p[1]))
                    .collect(),
                closed,
            }],
        }
    }

    fn style(cap: Cap, join: Join) -> StrokeStyle {
        StrokeStyle {
            width: 2.0,
            cap,
            join,
            miter_limit: 4.0,
        }
    }

    fn round(outline: &Outline, half: f32) -> Vec<Loop> {
        super::outline(
            outline,
            StrokeStyle {
                width: half * 2.0,
                cap: Cap::Round,
                join: Join::Round,
                miter_limit: 4.0,
            },
            1e-3,
        )
    }

    /// Whether `p` is inside the union of `loops`, filled nonzero.
    fn inside(loops: &[Loop], p: Point) -> bool {
        let mut curves = Vec::new();
        for curve in loops.iter().flatten() {
            monotonic(*curve, &mut curves);
        }
        winding(&curves, p) != 0
    }

    fn segment_distance(a: Point, b: Point, p: Point) -> f32 {
        let edge = b.sub(a);
        let t = (p.sub(a).dot(edge) / edge.dot(edge).max(1e-12)).clamp(0.0, 1.0);
        p.sub(a.add(edge.scale(t))).length()
    }

    #[test]
    fn segments_joins_and_caps_each_give_a_loop() {
        let corner = polyline(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], false);
        // Two segments and one join; butt caps add nothing.
        assert_eq!(
            outline(&corner, style(Cap::Butt, Join::Miter), 0.01).len(),
            3
        );
        // Round caps add a half disc at each end.
        assert_eq!(
            outline(&corner, style(Cap::Round, Join::Round), 0.01).len(),
            5
        );
        // A closed square: four sides and four joins.
        let square = polyline(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)], true);
        assert_eq!(
            outline(&square, style(Cap::Butt, Join::Bevel), 0.01).len(),
            8
        );
    }

    #[test]
    fn a_miter_reaches_its_tip_unless_too_long() {
        let corner = polyline(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], false);
        let loops = outline(&corner, style(Cap::Butt, Join::Miter), 0.01);
        // The square corner's miter tip lies at (11, -1).
        assert!(
            loops
                .iter()
                .flatten()
                .any(|[p, ..]| (p.x - 11.0).abs() < 1e-4 && (p.y + 1.0).abs() < 1e-4)
        );
        // A very sharp corner falls back to a bevel: a triangle.
        let sharp = polyline(&[(0.0, 0.0), (10.0, 0.0), (0.0, 0.5)], false);
        let loops = outline(&sharp, style(Cap::Butt, Join::Miter), 0.01);
        assert_eq!(loops.last().map(Vec::len), Some(3));
    }

    #[test]
    fn every_loop_turns_the_same_way() {
        let corner = polyline(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], false);
        for curves in outline(&corner, style(Cap::Round, Join::Round), 0.01) {
            assert!(area(&curves) > 0.0);
        }
    }

    #[test]
    fn round_strokes_cover_exactly_the_points_within_half_their_width() {
        // A cubic bending tightly at its top, a quadratic, a corner and a straight end: every
        // kind of piece, a join and both caps.
        let outline = Outline {
            subpaths: vec![Subpath {
                segments: vec![
                    Segment::Cubic(
                        Point::new(2.0, 18.0),
                        Point::new(6.0, 18.0),
                        Point::new(8.0, 2.0),
                        Point::new(12.0, 2.0),
                    ),
                    Segment::Quad(
                        Point::new(12.0, 2.0),
                        Point::new(16.0, 2.0),
                        Point::new(17.0, 10.0),
                    ),
                    Segment::Line(Point::new(17.0, 10.0), Point::new(10.0, 15.0)),
                ],
                closed: false,
            }],
        };
        let half = 1.5;
        let loops = round(&outline, half);
        // The centerline, finely.
        let mut points = vec![Point::new(2.0, 18.0)];
        for segment in &outline.subpaths[0].segments {
            segment.flatten(1e-5, &mut points);
        }
        let tolerance = 0.02;
        for row in 0..80 {
            for column in 0..80 {
                let p = Point::new(column as f32 * 0.25 + 0.1, row as f32 * 0.25 + 0.1);
                let distance = points
                    .windows(2)
                    .map(|pair| segment_distance(pair[0], pair[1], p))
                    .fold(f32::INFINITY, f32::min);
                if distance < half - tolerance {
                    assert!(inside(&loops, p), "({}, {}) is uncovered", p.x, p.y);
                } else if distance > half + tolerance {
                    assert!(!inside(&loops, p), "({}, {}) is covered", p.x, p.y);
                }
            }
        }
    }

    #[test]
    fn smooth_curves_take_few_curves() {
        // The quadratics of one cubic meet tangentially: no joins between them, and offsets of
        // a few quadratics each.
        let outline = Outline {
            subpaths: vec![Subpath {
                segments: vec![Segment::Cubic(
                    Point::new(4.0, 17.0),
                    Point::new(8.0, 17.0),
                    Point::new(9.0, 7.0),
                    Point::new(12.0, 7.0),
                )],
                closed: false,
            }],
        };
        let loops = round(&outline, 0.8);
        let curves: usize = loops.iter().map(Vec::len).sum();
        assert!(curves < 120, "{curves} curves");
    }
}
