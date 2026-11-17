//! Booleans on filled outlines: union, intersection, difference and exclusive or, of contours
//! under their fill rules, as curves ([ADR 0029]).
//!
//! The planar arrangement of both outlines:
//!
//! 1. Every segment is split where it turns in x or in y: monotone edges, whose end points bound
//!    them exactly.
//! 2. Edges are split where they cross or touch, and the points within a hair of each other are
//!    made one vertex. Edges that run along each other (shared or coincident) become one edge.
//! 3. Each edge is classified by the winding numbers of both operands on either side of it.
//!    They are counted from its middle, with a ray that crosses it rather than runs along it,
//!    the edge itself left out and then added to each side by its direction alone, so no
//!    probe ever lands on the wrong side of a nearby edge.
//! 4. The edges where the result's inside changes are kept, turned to have the inside toward
//!    their normal, and chained into loops, at each vertex taking the edge that turns most
//!    toward the inside, so that regions touching at a point stay apart.
//!
//! The result's segments are pieces of the input's: lines, Béziers and exact conics, never a
//! polygon. Every loop has its inside toward its normal (winding +1) and holes run the other
//! way, so the result fills the same under either rule.
//!
//! [ADR 0029]: https://github.com/Kolosta/Koebi/blob/develop/docs/adr/0029-canvas-rendering-from-curves.md

mod intersect;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use crate::{Contour, Contours, FillRule, Join, Point, Rect, Segment, Vector};

use intersect::{Crossing, crossings, monotone_bounds};

/// A hash map with a multiplicative hash (as rustc's): its keys are small integers, which
/// the default hasher, built against attackers, spends far longer on.
type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<Fast>>;

/// The hasher of [`FastMap`].
#[derive(Default)]
struct Fast(u64);

impl Hasher for Fast {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.write_u64(u64::from(byte));
        }
    }

    fn write_u64(&mut self, value: u64) {
        self.0 = (self.0.rotate_left(5) ^ value).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }

    fn write_usize(&mut self, value: usize) {
        self.write_u64(value as u64);
    }

    fn write_i64(&mut self, value: i64) {
        self.write_u64(value.cast_unsigned());
    }
}

/// What a boolean keeps of its two operands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BooleanOp {
    /// Inside either.
    #[default]
    Union,
    /// Inside both.
    Intersect,
    /// Inside the first and not the second.
    Subtract,
    /// Inside exactly one.
    Xor,
}

impl BooleanOp {
    /// Whether a point inside `a` (or not) and `b` (or not) is inside the result.
    const fn keeps(self, a: bool, b: bool) -> bool {
        match self {
            Self::Union => a || b,
            Self::Intersect => a && b,
            Self::Subtract => a && !b,
            Self::Xor => a != b,
        }
    }
}

/// The boolean `op` of `a` filled by `a_rule` and `b` filled by `b_rule`: closed contours
/// whose inside is toward their normal, filling the same under either rule.
pub fn boolean(
    a: &Contours,
    a_rule: FillRule,
    b: &Contours,
    b_rule: FillRule,
    op: BooleanOp,
) -> Contours {
    Arrangement::new([a, b])
        .result(|winding| op.keeps(inside(winding[0], a_rule), inside(winding[1], b_rule)))
}

/// The region `contours` fill under `rule`, as loops that neither overlap nor cross: the outline
/// of a stroke, whose pieces overlap, made one region; a path crossing itself made simple.
pub fn simplify(contours: &Contours, rule: FillRule) -> Contours {
    Arrangement::new([contours, &Contours::default()]).result(|winding| inside(winding[0], rule))
}

/// The region `contours` fill under `rule`, grown by `distance`, or shrunk when it is negative:
/// the Offset node. Corners grow by `join` (round: every point within the distance), and the
/// curves of the outline are offset within `tolerance`.
pub fn offset_region(
    contours: &Contours,
    rule: FillRule,
    distance: f64,
    join: Join,
    tolerance: f64,
) -> Contours {
    grown(&simplify(contours, rule), distance, join, tolerance)
}

/// `shape`, made of loops that wind once with their inside toward their normal (as booleans
/// leave them), grown by `distance` (shrunk when negative): the points its raw offset loops
/// wind positively.
pub(crate) fn grown(shape: &Contours, distance: f64, join: Join, tolerance: f64) -> Contours {
    if distance == 0.0 || !distance.is_finite() {
        return shape.clone();
    }
    let tolerance = if tolerance > 0.0 && tolerance.is_finite() {
        tolerance
    } else {
        distance.abs() * 1e-3
    };
    let mut raw = Contours::default();
    for contour in shape.contours() {
        // Growing moves the outline away from its inside, against its normal.
        if let Some(loop_) = crate::stroke::raw_offset(contour, -distance, join, tolerance) {
            raw.push(loop_);
        }
    }
    Arrangement::new([&raw, &Contours::default()]).result(|winding| winding[0] > 0)
}

fn inside(winding: i32, rule: FillRule) -> bool {
    match rule {
        FillRule::NonZero => winding != 0,
        FillRule::EvenOdd => winding % 2 != 0,
    }
}

/// A monotone edge of the arrangement.
#[derive(Clone, Copy, Debug)]
struct Edge {
    segment: Segment,
    /// The operand it comes from, 0 or 1.
    operand: usize,
    /// Its end vertices, once split and merged.
    from: usize,
    to: usize,
}

/// Both outlines, split into monotone edges that meet only at their ends.
struct Arrangement {
    edges: Vec<Edge>,
    vertices: Vec<Point>,
    /// Points closer than this are one.
    epsilon: f64,
}

impl Arrangement {
    fn new(operands: [&Contours; 2]) -> Self {
        let bounds = operands
            .iter()
            .flat_map(|contours| contours.contours())
            .flat_map(Contour::segments)
            .fold(Rect::EMPTY, |bounds, segment| {
                bounds.union(segment.bounds())
            });
        let scale = if bounds.is_empty() {
            1.0
        } else {
            [bounds.min.x, bounds.min.y, bounds.max.x, bounds.max.y]
                .iter()
                .map(|v| v.abs())
                .fold(bounds.width().max(bounds.height()), f64::max)
                .max(f64::MIN_POSITIVE)
        };
        let epsilon = scale * 1e-10;
        let mut pieces = Vec::new();
        for (operand, contours) in operands.into_iter().enumerate() {
            for contour in contours.contours() {
                monotone(contour, operand, epsilon, &mut pieces);
            }
        }
        let mut arrangement = Self {
            edges: Vec::new(),
            vertices: Vec::new(),
            epsilon,
        };
        arrangement.split(&pieces);
        arrangement
    }

    /// Splits `pieces` where they cross, merges the vertices, and keeps the edges.
    fn split(&mut self, pieces: &[(Segment, usize)]) {
        let epsilon = self.epsilon;
        let bounds: Vec<Rect> = pieces.iter().map(|(s, _)| monotone_bounds(s)).collect();
        let mut cuts: Vec<Vec<(f64, Point)>> = vec![Vec::new(); pieces.len()];
        // Sweep along x: each piece meets those whose x ranges overlap its own.
        let mut order: Vec<usize> = (0..pieces.len()).collect();
        order.sort_unstable_by(|&i, &j| bounds[i].min.x.total_cmp(&bounds[j].min.x));
        let mut active: Vec<usize> = Vec::new();
        let (mut found, mut stack) = (Vec::new(), Vec::new());
        for &i in &order {
            let reach = bounds[i].min.x - epsilon;
            active.retain(|&j| bounds[j].max.x >= reach);
            for &j in &active {
                if !bounds[i].inflated(epsilon).intersects(bounds[j]) {
                    continue;
                }
                found.clear();
                crossings(&pieces[i].0, &pieces[j].0, epsilon, &mut stack, &mut found);
                for &Crossing { t, u, point } in &found {
                    cuts[i].push((t, point));
                    cuts[j].push((u, point));
                }
            }
            active.push(i);
        }
        // Vertices: points within epsilon are one.
        let mut grid: FastMap<(i64, i64), Vec<usize>> = FastMap::default();
        // Cells much wider than the merging radius: a point looks at its neighbours only when
        // it lies that close to a side.
        let (cell, radius) = (epsilon * 64.0, epsilon * 4.0);
        let mut vertex = |vertices: &mut Vec<Point>, point: Point| -> usize {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "a grid key: coordinates over the cell size stay far below 2^63"
            )]
            let key = |x: f64| (x / cell).floor() as i64;
            let (kx, ky) = (key(point.x), key(point.y));
            let reach = |value: f64, key: i64| {
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "a grid key back to a coordinate"
                )]
                let low = key as f64 * cell;
                (-i64::from(value - low <= radius))..=i64::from(low + cell - value <= radius)
            };
            for dx in reach(point.x, kx) {
                for dy in reach(point.y, ky) {
                    if let Some(found) = grid.get(&(kx + dx, ky + dy)) {
                        for &index in found {
                            if vertices[index].distance(point) <= radius {
                                return index;
                            }
                        }
                    }
                }
            }
            vertices.push(point);
            grid.entry((kx, ky)).or_default().push(vertices.len() - 1);
            vertices.len() - 1
        };
        for (index, (segment, operand)) in pieces.iter().enumerate() {
            let cuts = &mut cuts[index];
            cuts.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
            let mut start = (0.0, segment.start());
            let ends = cuts
                .iter()
                .copied()
                .filter(|&(t, point)| {
                    t > 0.0
                        && t < 1.0
                        && point.distance(segment.start()) > epsilon * 4.0
                        && point.distance(segment.end()) > epsilon * 4.0
                })
                .chain(core::iter::once((1.0, segment.end())));
            for end in ends {
                if end.1.distance(start.1) <= epsilon * 4.0 || end.0 <= start.0 {
                    continue;
                }
                let from = vertex(&mut self.vertices, start.1);
                let to = vertex(&mut self.vertices, end.1);
                if from != to {
                    let part = segment
                        .subsegment(start.0, end.0)
                        .with_start(self.vertices[from])
                        .with_end(self.vertices[to]);
                    self.edges.push(Edge {
                        segment: part,
                        operand: *operand,
                        from,
                        to,
                    });
                }
                start = end;
            }
        }
    }

    /// The loops bounding where `keeps` holds, given the winding numbers of both operands.
    fn result(&self, keeps: impl Fn([i32; 2]) -> bool) -> Contours {
        let index = Rays::new(&self.edges);
        // Coincident edges: one stands for all, the others counted with it.
        let mut groups: FastMap<(usize, usize), Vec<usize>> = FastMap::default();
        for (i, edge) in self.edges.iter().enumerate() {
            groups
                .entry((edge.from.min(edge.to), edge.from.max(edge.to)))
                .or_default()
                .push(i);
        }
        let mut twin_of: Vec<Option<usize>> = vec![None; self.edges.len()];
        for members in groups.values() {
            for (k, &i) in members.iter().enumerate() {
                if twin_of[i].is_some() {
                    continue;
                }
                let segment = &self.edges[i].segment;
                let middle = segment.eval(0.5);
                for &j in &members[k + 1..] {
                    if twin_of[j].is_some() {
                        continue;
                    }
                    // Two lines between the same vertices are one; curves, when one runs
                    // through the other's middle.
                    let other = &self.edges[j].segment;
                    let same = match (segment, other) {
                        (Segment::Line(_), Segment::Line(_)) => true,
                        (Segment::Line(_), _) | (_, Segment::Line(_)) => {
                            other.nearest(middle).1.sqrt() <= self.epsilon * 64.0
                        }
                        _ => {
                            other.eval(0.5).distance(middle) <= self.epsilon * 64.0
                                || other.nearest(middle).1.sqrt() <= self.epsilon * 64.0
                        }
                    };
                    if same {
                        twin_of[j] = Some(i);
                    }
                }
            }
        }
        let mut twins: Vec<Vec<usize>> = vec![Vec::new(); self.edges.len()];
        for (j, twin) in twin_of.iter().enumerate() {
            if let Some(i) = twin {
                twins[*i].push(j);
            }
        }
        // Keep the edges where the inside changes, its inside toward the normal.
        let mut kept: Vec<Segment> = Vec::new();
        for (i, edge) in self.edges.iter().enumerate() {
            if twin_of[i].is_some() {
                continue;
            }
            let middle = edge.segment.eval(0.5);
            let tangent = edge
                .segment
                .tangent(0.5)
                .unwrap_or_else(|| Vector::new(1.0, 0.0));
            let normal = tangent.perpendicular();
            let steep = tangent.y.abs() >= tangent.x.abs();
            let group: Vec<usize> = core::iter::once(i)
                .chain(twins[i].iter().copied())
                .collect();
            let base = index.winding(middle, steep, &group);
            // Each member of the group adds its own crossing to each side.
            let mut sides = [base, base];
            for &member in &group {
                let other = &self.edges[member];
                let direction = if other.from == edge.from {
                    tangent
                } else {
                    -tangent
                };
                for (side, side_vector) in sides.iter_mut().zip([normal, -normal]) {
                    side[other.operand] += own_crossing(direction, side_vector, steep);
                }
            }
            let (plus, minus) = (keeps(sides[0]), keeps(sides[1]));
            if plus != minus {
                kept.push(if plus {
                    edge.segment
                } else {
                    edge.segment.reversed()
                });
            }
        }
        Self::chain(&kept)
    }

    /// Chains `kept` edges, each with the inside toward its normal, into loops.
    fn chain(kept: &[Segment]) -> Contours {
        // Every end point is a vertex exactly: its bits are its key.
        let key = |point: Point| (point.x.to_bits(), point.y.to_bits());
        let mut outgoing: FastMap<(u64, u64), Vec<usize>> = FastMap::default();
        for (index, segment) in kept.iter().enumerate() {
            outgoing
                .entry(key(segment.start()))
                .or_default()
                .push(index);
        }
        let mut used = vec![false; kept.len()];
        let mut out = Contours::default();
        for first in 0..kept.len() {
            if used[first] {
                continue;
            }
            let mut segments = Vec::new();
            let mut current = first;
            let start = key(kept[first].start());
            loop {
                used[current] = true;
                segments.push(kept[current]);
                let segment = &kept[current];
                let at = key(segment.end());
                if at == start {
                    break;
                }
                let arriving = segment.tangent(1.0).unwrap_or(Vector::new(1.0, 0.0));
                // The edge that turns most toward the inside: the tightest loop.
                let next = outgoing.get(&at).and_then(|candidates| {
                    candidates
                        .iter()
                        .copied()
                        .filter(|&c| !used[c])
                        .max_by(|&x, &y| {
                            let turn = |c: usize| {
                                let leaving = kept[c].tangent(0.0).unwrap_or(arriving);
                                arriving.cross(leaving).atan2(arriving.dot(leaving))
                            };
                            turn(x).total_cmp(&turn(y))
                        })
                });
                match next {
                    Some(next) => current = next,
                    None => break,
                }
            }
            out.push(Contour::new(segments, true));
        }
        out
    }
}

/// The monotone pieces of `contour`, closed by a line if it does not end where it starts (fills
/// close open subpaths).
fn monotone(contour: &Contour, operand: usize, epsilon: f64, out: &mut Vec<(Segment, usize)>) {
    let segments = contour.segments();
    let mut push = |segment: &Segment| {
        if !segment.is_finite() {
            return;
        }
        let (cuts, count) = turning_points(segment);
        let mut t0 = 0.0;
        for &t1 in cuts[..count].iter().chain(core::iter::once(&1.0)) {
            if t1 - t0 > 1e-12 {
                let piece = segment.subsegment(t0, t1);
                if piece.hull_length() > epsilon {
                    out.push((piece, operand));
                }
            }
            t0 = t1;
        }
    };
    for segment in segments {
        push(segment);
    }
    if let (Some(first), Some(last)) = (segments.first(), segments.last())
        && last.end().distance(first.start()) > epsilon
    {
        push(&Segment::line(last.end(), first.start()));
    }
}

/// Where `segment` turns in x or in y, ascending, in a fixed array: no allocation.
pub(crate) fn turning_points(segment: &Segment) -> ([f64; 24], usize) {
    let mut cuts = [0.0; 24];
    let mut count = 0;
    for axis in 0..2 {
        for &t in segment.extrema(axis).as_slice() {
            cuts[count] = t;
            count += 1;
        }
    }
    cuts[..count].sort_unstable_by(f64::total_cmp);
    (cuts, count)
}

/// What an edge running along `direction` through a point adds to the winding number on the
/// side `side` of it (a unit vector off it), for the ray that counts there: toward +x when
/// `steep`, toward +y otherwise.
fn own_crossing(direction: Vector, side: Vector, steep: bool) -> i32 {
    let cross = direction.cross(side);
    if steep {
        // The ray toward +x meets the edge δ·cross / direction.y to the probe's right.
        let ahead = cross / direction.y;
        if ahead > 0.0 {
            if direction.y > 0.0 { 1 } else { -1 }
        } else {
            0
        }
    } else {
        // The ray toward +y meets it -δ·cross / direction.x below the probe.
        let ahead = -cross / direction.x;
        if ahead > 0.0 {
            if direction.x < 0.0 { 1 } else { -1 }
        } else {
            0
        }
    }
}

/// The edges filed by bands, for the winding number at a point by a ray toward +x (rows) or
/// toward +y (columns).
struct Rays<'a> {
    edges: &'a [Edge],
    bounds: Rect,
    rows: Vec<Vec<usize>>,
    columns: Vec<Vec<usize>>,
}

impl<'a> Rays<'a> {
    fn new(edges: &'a [Edge]) -> Self {
        let bounds = edges
            .iter()
            .fold(Rect::EMPTY, |b, e| b.union(monotone_bounds(&e.segment)));
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss,
            reason = "a band count from an edge count"
        )]
        let bands = ((edges.len() as f64).sqrt().ceil() as usize).clamp(1, 1024);
        let mut rows = vec![Vec::new(); bands];
        let mut columns = vec![Vec::new(); bands];
        for (index, edge) in edges.iter().enumerate() {
            let b = monotone_bounds(&edge.segment);
            let (low, high) = (bounds.min.y, bounds.max.y);
            let span = band(b.min.y, low, high, bands)..=band(b.max.y, low, high, bands);
            for row in &mut rows[span] {
                row.push(index);
            }
            let (low, high) = (bounds.min.x, bounds.max.x);
            let span = band(b.min.x, low, high, bands)..=band(b.max.x, low, high, bands);
            for column in &mut columns[span] {
                column.push(index);
            }
        }
        Self {
            edges,
            bounds,
            rows,
            columns,
        }
    }

    /// The winding numbers of both operands at `point`, leaving out the edges `skip`: by a ray
    /// toward +x when `horizontal`, toward +y otherwise.
    fn winding(&self, point: Point, horizontal: bool, skip: &[usize]) -> [i32; 2] {
        let mut winding = [0, 0];
        let candidates = if horizontal {
            if point.y < self.bounds.min.y || point.y > self.bounds.max.y {
                return winding;
            }
            &self.rows[band(
                point.y,
                self.bounds.min.y,
                self.bounds.max.y,
                self.rows.len(),
            )]
        } else {
            if point.x < self.bounds.min.x || point.x > self.bounds.max.x {
                return winding;
            }
            &self.columns[band(
                point.x,
                self.bounds.min.x,
                self.bounds.max.x,
                self.columns.len(),
            )]
        };
        for &index in candidates {
            if skip.contains(&index) {
                continue;
            }
            let edge = &self.edges[index];
            winding[edge.operand] += if horizontal {
                crossing_right(&edge.segment, point)
            } else {
                crossing_below(&edge.segment, point)
            };
        }
        winding
    }
}

/// The band of `count` over `low..=high` that holds `value`.
fn band(value: f64, low: f64, high: f64, count: usize) -> usize {
    let span = high - low;
    if span <= 0.0 {
        return 0;
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "an index into at most 1024 bands"
    )]
    let index = (((value - low) / span) * count as f64).floor().max(0.0) as usize;
    index.min(count - 1)
}

/// What a monotone edge adds to the winding number at `point` by the ray toward +x: ±1 when
/// it crosses the ray's line (half-open in y) to the right of the point, by its direction.
fn crossing_right(segment: &Segment, point: Point) -> i32 {
    let (a, b) = (segment.start(), segment.end());
    if (a.y <= point.y) == (b.y <= point.y) {
        return 0;
    }
    let sign = if b.y > a.y { 1 } else { -1 };
    if point.x < a.x.min(b.x) {
        return sign;
    }
    if point.x > a.x.max(b.x) {
        return 0;
    }
    let x = at_coordinate(segment, point.y, |p| p.y).x;
    if x > point.x { sign } else { 0 }
}

/// What a monotone edge adds by the ray toward +y: +1 crossing it below the point going -x, -1
/// going +x (half-open in x), which counts the same as the ray toward +x.
fn crossing_below(segment: &Segment, point: Point) -> i32 {
    let (a, b) = (segment.start(), segment.end());
    if (a.x <= point.x) == (b.x <= point.x) {
        return 0;
    }
    let sign = if b.x < a.x { 1 } else { -1 };
    if point.y < a.y.min(b.y) {
        return sign;
    }
    if point.y > a.y.max(b.y) {
        return 0;
    }
    let y = at_coordinate(segment, point.x, |p| p.x).y;
    if y > point.y { sign } else { 0 }
}

/// The point of a monotone edge where `coordinate` equals `value`, by bisection.
fn at_coordinate(segment: &Segment, value: f64, coordinate: impl Fn(Point) -> f64) -> Point {
    if let Segment::Line([a, b]) = segment {
        let (ca, cb) = (coordinate(*a), coordinate(*b));
        return a.lerp(*b, (value - ca) / (cb - ca));
    }
    let rising = coordinate(segment.end()) > coordinate(segment.start());
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for _ in 0..60 {
        let middle = lo.midpoint(hi);
        if (coordinate(segment.eval(middle)) <= value) == rising {
            lo = middle;
        } else {
            hi = middle;
        }
    }
    segment.eval(lo.midpoint(hi))
}
