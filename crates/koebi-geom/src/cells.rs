//! Fill cells: an outline cut into the grid cells of a zoom octave, each holding all that a
//! pixel inside it needs to count its winding number by two rays, one going right and one
//! going down (ADR 0029).
//!
//! - **Curves:** the outline's quadratics inside the cell grown by a margin
//!   ([`Octave::margin`]), relative to the cell's origin: a pixel near the cell's side still
//!   sees the curves its footprint reaches across it.
//! - **Edges:** what lies beyond the grown cell's right side, folded onto it: a ray going right
//!   from inside the cell crosses there what it would have crossed beyond, so straight stretches
//!   along the side, each with the winding it stands for, replace every curve to the right.
//!   The same holds below for rays going down. What lies left or above crosses neither ray.
//!
//! A cell's coverage then needs its own curves and edges only, however large the outline and
//! wherever the view lies on it: a view inside a kilometre-wide shape at a zoom of 10⁹ prepares a
//! few cells, their curves within 2¹¹ pixels of their origins.

use crate::prepare::{closed, crossing, monotone, to_quadratics};
use crate::{Contours, Octave, Point, Quadratic, Rect, Segment, solve};

/// A straight stretch along a side of a cell standing for what lies beyond the side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edge {
    /// Where it starts along the side, relative to the cell's origin.
    pub from: f32,
    /// Where it ends, past `from`.
    pub to: f32,
    /// The winding a ray crossing the side there gains beyond it, never zero.
    pub winding: i32,
}

/// The curves of one grid cell of a fill, and what lies beyond its sides.
#[derive(Clone, Debug, PartialEq)]
pub struct FillCell {
    /// Its column: the cell spans `column * size..(column + 1) * size` in x.
    pub column: i64,
    /// Its row.
    pub row: i64,
    /// Its corner of least x and y, exact.
    pub origin: Point,
    /// The curves inside the cell grown by the margin, relative to its origin.
    pub curves: Vec<[[f32; 2]; 3]>,
    /// Along its grown right side (`x = size + margin`): the winding of what lies to the right,
    /// stretch by stretch of y. A ray going right gains it when it crosses one going down.
    pub right: Vec<Edge>,
    /// Along its grown bottom side (`y = size + margin`): the winding of what lies below,
    /// stretch by stretch of x, as a ray going down gains it.
    pub bottom: Vec<Edge>,
}

/// A stretch of a side and the winding it carries, before stretches merge.
#[derive(Clone, Copy, Debug)]
struct Span {
    from: f64,
    to: f64,
    winding: i32,
}

/// The cells of `contours` needed to draw `area` at `octave` by the rays of ADR 0029: the
/// whole cells of the octave's grid around `area`, each with its curves and its edges
/// ([`FillCell`]), ordered by row then column. Cells with nothing to show are left out, and an
/// area without width or height, or not finite, prepares nothing.
///
/// Open contours are closed by a line, as fills close them.
pub fn fill_cells(contours: &Contours, octave: Octave, area: Rect) -> Vec<FillCell> {
    let (size, margin) = (octave.cell_size(), octave.margin());
    let finite = [area.min.x, area.min.y, area.max.x, area.max.y]
        .iter()
        .all(|value| value.is_finite());
    let extended = area.width() > 0.0 && area.height() > 0.0;
    if !(finite && extended) {
        return Vec::new();
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a cell index: a coordinate over the cell size, far below 2^63"
    )]
    let index = |value: f64, up: bool| {
        let cells = value / size;
        (if up { cells.ceil() } else { cells.floor() }) as i64
    };
    let (columns, rows) = (
        index(area.min.x, false)..index(area.max.x, true).max(index(area.min.x, false) + 1),
        index(area.min.y, false)..index(area.max.y, true).max(index(area.min.y, false) + 1),
    );
    #[allow(
        clippy::cast_precision_loss,
        reason = "a cell index times a power of two: exact while the result is"
    )]
    let at = |index: i64| index as f64 * size;
    // The region whose curves the cells hold, every cell grown by the margin.
    let region = Rect::new(
        Point::new(at(columns.start) - margin, at(rows.start) - margin),
        Point::new(at(columns.end) + margin, at(rows.end) + margin),
    );
    let mut inside = Vec::new();
    let (mut beyond_right, mut beyond_bottom) = (Vec::new(), Vec::new());
    let mut parts = Vec::new();
    for contour in contours.contours() {
        for segment in closed(contour) {
            if !segment.is_finite() {
                continue;
            }
            let bounds = segment.bounds();
            // Left of the region or above it, no ray from inside crosses it.
            if bounds.max.x < region.min.x || bounds.max.y < region.min.y {
                continue;
            }
            monotone(&segment, &mut parts);
            for part in parts.drain(..) {
                split_segment(
                    &part,
                    region,
                    octave.tolerance(),
                    &mut inside,
                    &mut beyond_right,
                    &mut beyond_bottom,
                );
            }
        }
    }
    let mut cells = Vec::new();
    let (mut right, mut bottom) = (Vec::new(), Vec::new());
    for row in rows {
        for column in columns.clone() {
            let origin = Point::new(at(column), at(row));
            let grown = Rect::new(
                Point::new(origin.x - margin, origin.y - margin),
                Point::new(origin.x + size + margin, origin.y + size + margin),
            );
            let mut curves = Vec::new();
            right.clear();
            bottom.clear();
            for quad in &inside {
                file_quadratic(*quad, grown, origin, &mut curves, &mut right, &mut bottom);
            }
            // What lies beyond the region, past this cell's sides too.
            for span in &beyond_right {
                clip_span(*span, grown.min.y, grown.max.y, &mut right);
            }
            for span in &beyond_bottom {
                clip_span(*span, grown.min.x, grown.max.x, &mut bottom);
            }
            let right = merged(&mut right, origin.y);
            let bottom = merged(&mut bottom, origin.x);
            if curves.is_empty() && right.is_empty() && bottom.is_empty() {
                continue;
            }
            cells.push(FillCell {
                column,
                row,
                origin,
                curves,
                right,
                bottom,
            });
        }
    }
    cells
}

/// Files a monotone `segment` against `region`: its part inside as quadratics, its part to the
/// right folded onto the right side, its part below folded onto the bottom.
fn split_segment(
    segment: &Segment,
    region: Rect,
    tolerance: f64,
    inside: &mut Vec<Quadratic>,
    right: &mut Vec<Span>,
    bottom: &mut Vec<Span>,
) {
    let x = |p: Point| p.x;
    let y = |p: Point| p.y;
    // The part with x beyond the right side.
    let (left_part, right_part) = split_by(segment, region.max.x, x);
    if let Some(part) = right_part {
        let (a, b) = (part.start().y, part.end().y);
        push_span(
            a.min(b).max(region.min.y),
            a.max(b).min(region.max.y),
            if b > a { 1 } else { -1 },
            right,
        );
    }
    let Some(left) = left_part else { return };
    let (upper, lower) = split_by(&left, region.max.y, y);
    if let Some(part) = lower {
        let (a, b) = (part.start().x, part.end().x);
        // A curve going left crosses a ray going down as one going down crosses a ray going
        // right.
        push_span(
            a.min(b).max(region.min.x),
            a.max(b).min(region.max.x),
            if b < a { 1 } else { -1 },
            bottom,
        );
    }
    let Some(upper) = upper else { return };
    // Left of the region or above it, nothing; the rest is inside.
    let (_, from_left) = split_by(&upper, region.min.x, x);
    let Some(from_left) = from_left else { return };
    let (_, from_top) = split_by(&from_left, region.min.y, y);
    if let Some(piece) = from_top {
        to_quadratics(&piece, tolerance, inside);
    }
}

/// A monotone `segment` cut where `coordinate` reaches `value`: the part below it and the part
/// above it, either missing when the segment lies on one side.
fn split_by(
    segment: &Segment,
    value: f64,
    coordinate: impl Fn(Point) -> f64 + Copy,
) -> (Option<Segment>, Option<Segment>) {
    let (a, b) = (coordinate(segment.start()), coordinate(segment.end()));
    if a.max(b) <= value {
        return (Some(*segment), None);
    }
    if a.min(b) >= value {
        return (None, Some(*segment));
    }
    let t = crossing(segment, value, coordinate);
    let (first, second) = (segment.subsegment(0.0, t), segment.subsegment(t, 1.0));
    if a < b {
        (Some(first), Some(second))
    } else {
        (Some(second), Some(first))
    }
}

/// Files a monotone quadratic against a grown cell, as [`split_segment`] does the region's.
fn file_quadratic(
    quad: Quadratic,
    grown: Rect,
    origin: Point,
    curves: &mut Vec<[[f32; 2]; 3]>,
    right: &mut Vec<Span>,
    bottom: &mut Vec<Span>,
) {
    let (low, high) = (
        Point::new(quad[0].x.min(quad[2].x), quad[0].y.min(quad[2].y)),
        Point::new(quad[0].x.max(quad[2].x), quad[0].y.max(quad[2].y)),
    );
    if high.x < grown.min.x || high.y < grown.min.y || (low.x > grown.max.x && low.y > grown.max.y)
    {
        return;
    }
    let (left_part, right_part) = split_quad(quad, 0, grown.max.x);
    if let Some(part) = right_part {
        let (a, b) = (part[0].y, part[2].y);
        push_span(
            a.min(b).max(grown.min.y),
            a.max(b).min(grown.max.y),
            if b > a { 1 } else { -1 },
            right,
        );
    }
    let Some(left) = left_part else { return };
    let (upper, lower) = split_quad(left, 1, grown.max.y);
    if let Some(part) = lower {
        let (a, b) = (part[0].x, part[2].x);
        push_span(
            a.min(b).max(grown.min.x),
            a.max(b).min(grown.max.x),
            if b < a { 1 } else { -1 },
            bottom,
        );
    }
    let Some(upper) = upper else { return };
    let Some(from_left) = split_quad(upper, 0, grown.min.x).1 else {
        return;
    };
    let Some(piece) = split_quad(from_left, 1, grown.min.y).1 else {
        return;
    };
    #[allow(
        clippy::cast_possible_truncation,
        reason = "within its grown cell, a coordinate fits an f32 with sixteenths of a pixel"
    )]
    curves.push(piece.map(|p| [(p.x - origin.x) as f32, (p.y - origin.y) as f32]));
}

/// A quadratic monotone in `axis` (0 for x, 1 for y) cut where it reaches `value`: its part
/// below and its part above.
fn split_quad(quad: Quadratic, axis: usize, value: f64) -> (Option<Quadratic>, Option<Quadratic>) {
    let coordinate = |p: Point| if axis == 0 { p.x } else { p.y };
    let (a, b, c) = (
        coordinate(quad[0]),
        coordinate(quad[1]),
        coordinate(quad[2]),
    );
    if a.max(c) <= value {
        return (Some(quad), None);
    }
    if a.min(c) >= value {
        return (None, Some(quad));
    }
    let roots = solve::quadratic(a - 2.0 * b + c, 2.0 * (b - a), a - value);
    let t = roots
        .as_slice()
        .iter()
        .copied()
        .find(|t| (0.0..=1.0).contains(t))
        .unwrap_or(0.5);
    // De Casteljau at t, the cut point exactly on the line.
    let lerp = |p: Point, q: Point| Point::new(p.x + (q.x - p.x) * t, p.y + (q.y - p.y) * t);
    let (p01, p12) = (lerp(quad[0], quad[1]), lerp(quad[1], quad[2]));
    let mut cut = lerp(p01, p12);
    if axis == 0 {
        cut.x = value;
    } else {
        cut.y = value;
    }
    let (first, second) = ([quad[0], p01, cut], [cut, p12, quad[2]]);
    if a < c {
        (Some(first), Some(second))
    } else {
        (Some(second), Some(first))
    }
}

fn push_span(from: f64, to: f64, winding: i32, out: &mut Vec<Span>) {
    if to > from {
        out.push(Span { from, to, winding });
    }
}

/// `span` cut to `low..high`, appended to `out` if anything is left.
fn clip_span(span: Span, low: f64, high: f64, out: &mut Vec<Span>) {
    push_span(span.from.max(low), span.to.min(high), span.winding, out);
}

/// The stretches of `spans` merged: where they overlap, their windings add, and where the sum
/// is zero, nothing is left. Relative to `origin`.
fn merged(spans: &mut [Span], origin: f64) -> Vec<Edge> {
    let mut events: Vec<(f64, i32)> = spans
        .iter()
        .flat_map(|span| [(span.from, span.winding), (span.to, -span.winding)])
        .collect();
    events.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
    let mut edges: Vec<Edge> = Vec::new();
    let mut winding = 0;
    let mut start = 0.0;
    let mut index = 0;
    while index < events.len() {
        let at = events[index].0;
        // Every change at this place at once.
        let before = winding;
        while index < events.len() && events[index].0 == at {
            winding += events[index].1;
            index += 1;
        }
        if before != 0 && at > start {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "within its grown cell, a coordinate fits an f32"
            )]
            let (from, to) = ((start - origin) as f32, (at - origin) as f32);
            match edges.last_mut() {
                // Touching stretches of one winding make one.
                Some(last) if last.to == from && last.winding == before => last.to = to,
                _ => edges.push(Edge {
                    from,
                    to,
                    winding: before,
                }),
            }
        }
        start = at;
    }
    edges
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Affine, Path, Vector};

    /// The winding of `point` as the rays of ADR 0029 count it in `cell`: the curves crossing
    /// a ray going right, then the edges of the right side. And the same going down.
    fn rays(cell: &FillCell, size: f64, margin: f64, point: Point) -> (i32, i32) {
        let p = [point.x - cell.origin.x, point.y - cell.origin.y];
        let side = size + margin;
        let mut right = 0;
        let mut down = 0;
        for curve in &cell.curves {
            let [a, b, c] = curve.map(|q| [f64::from(q[0]), f64::from(q[1])]);
            let at = |axis: usize, other: usize, value: f64| {
                let roots = solve::quadratic(
                    a[axis] - 2.0 * b[axis] + c[axis],
                    2.0 * (b[axis] - a[axis]),
                    a[axis] - value,
                );
                let t = roots
                    .as_slice()
                    .iter()
                    .copied()
                    .find(|t| (-1e-9..=1.0 + 1e-9).contains(t))
                    .unwrap_or(0.0);
                let u = 1.0 - t;
                u * u * a[other] + 2.0 * u * t * b[other] + t * t * c[other]
            };
            let within =
                |from: f64, to: f64, value: f64| value >= from.min(to) && value < from.max(to);
            if within(a[1], c[1], p[1]) && at(1, 0, p[1]) > p[0] {
                right += if c[1] > a[1] { 1 } else { -1 };
            }
            if within(a[0], c[0], p[0]) && at(0, 1, p[0]) > p[1] {
                down += if c[0] < a[0] { 1 } else { -1 };
            }
        }
        for edge in &cell.right {
            if p[1] >= f64::from(edge.from) && p[1] < f64::from(edge.to) && side > p[0] {
                right += edge.winding;
            }
        }
        for edge in &cell.bottom {
            if p[0] >= f64::from(edge.from) && p[0] < f64::from(edge.to) && side > p[1] {
                down += edge.winding;
            }
        }
        (right, down)
    }

    /// Checks both rays against the exact winding at a grid of points of every cell.
    fn check(contours: &Contours, octave: Octave, area: Rect) {
        let (size, margin) = (octave.cell_size(), octave.margin());
        let cells = fill_cells(contours, octave, area);
        for cell in &cells {
            for i in 0..17 {
                for j in 0..17 {
                    // Points off the grid's lines, inside the cell proper.
                    let point = Point::new(
                        cell.origin.x + (f64::from(i) + 0.37) / 17.0 * size,
                        cell.origin.y + (f64::from(j) + 0.61) / 17.0 * size,
                    );
                    if !area.contains(point) {
                        continue;
                    }
                    let exact = contours.winding(point);
                    assert_eq!(rays(cell, size, margin, point), (exact, exact), "{point:?}");
                }
            }
        }
        // Cells left out hold nothing: their points lie outside.
        for i in 0..23 {
            for j in 0..23 {
                let point = Point::new(
                    area.min.x + (f64::from(i) + 0.5) / 23.0 * area.width(),
                    area.min.y + (f64::from(j) + 0.5) / 23.0 * area.height(),
                );
                #[allow(clippy::cast_possible_truncation, reason = "a cell index")]
                let (column, row) = (
                    (point.x / size).floor() as i64,
                    (point.y / size).floor() as i64,
                );
                if !cells.iter().any(|c| c.column == column && c.row == row) {
                    assert_eq!(contours.winding(point), 0, "{point:?}");
                }
            }
        }
    }

    fn circle(center: Point, radius: f64) -> Contours {
        Path::ellipse(center, Vector::new(radius, radius)).contours()
    }

    /// The contours of `first` and `second` together.
    fn both(first: &Contours, second: &Contours) -> Contours {
        let mut all = first.clone();
        for contour in second.contours() {
            all.push(contour.clone());
        }
        all
    }

    #[test]
    fn rays_in_every_cell_count_the_exact_winding() {
        let octave = Octave(0);
        let size = octave.cell_size();
        // A circle across four cells, seen whole.
        let shape = circle(Point::new(size, size), 700.0);
        check(
            &shape,
            octave,
            Rect::new(Point::new(0.0, 0.0), Point::new(2.0 * size, 2.0 * size)),
        );
        // Seen through a window that cuts it: what lies beyond folds onto the sides.
        check(
            &shape,
            octave,
            Rect::new(Point::new(900.0, 400.0), Point::new(1300.0, 1500.0)),
        );
    }

    #[test]
    fn overlapping_and_reversed_contours_add_their_windings() {
        let octave = Octave(1);
        let size = octave.cell_size();
        // Turning the same way inside it: winding 2; the other way, mirrored onto itself: 0
        // where it overlaps the disc, -1 beyond.
        let shape = both(
            &circle(Point::new(300.0, 300.0), 250.0),
            &circle(Point::new(320.0, 310.0), 120.0),
        );
        let mirrored = Affine::translate(Vector::new(1100.0, 0.0)) * Affine::scale(-1.0, 1.0);
        let hole = Path::rectangle(Rect::new(
            Point::new(400.0, 100.0),
            Point::new(700.0, 200.0),
        ))
        .contours()
        .transformed(mirrored);
        let shape = both(&shape, &hole);
        check(
            &shape,
            octave,
            Rect::new(Point::new(0.0, 0.0), Point::new(3.0 * size, 2.0 * size)),
        );
        check(
            &shape,
            octave,
            Rect::new(Point::new(310.0, 250.0), Point::new(330.0, 270.0)),
        );
    }

    #[test]
    fn a_view_deep_inside_a_vast_shape_prepares_a_few_cells_that_still_count_right() {
        // A kilometre-wide disc (in millimetres), seen at a zoom of 10⁹ near its edge.
        let octave = Octave::of(1e9);
        let size = octave.cell_size();
        let radius = 1e6;
        let shape = circle(Point::new(0.0, 0.0), radius);
        let near = Point::new(radius - 2.0 * size, 0.25 * size);
        let area = Rect::new(near, Point::new(near.x + 4.0 * size, near.y + 2.0 * size));
        let cells = fill_cells(&shape, octave, area);
        assert!(!cells.is_empty() && cells.len() <= 15, "{}", cells.len());
        // Curves stay small around their cells' origins.
        for cell in &cells {
            for curve in &cell.curves {
                for point in curve {
                    #[allow(clippy::cast_possible_truncation, reason = "a small power of two")]
                    let extent = (size + 2.0 * octave.margin()) as f32;
                    assert!(
                        point[0].abs() <= extent && point[1].abs() <= extent,
                        "{point:?}"
                    );
                }
            }
        }
        check(&shape, octave, area);
    }

    #[test]
    fn nothing_beyond_or_without_an_area_prepares_nothing() {
        let shape = circle(Point::new(0.0, 0.0), 10.0);
        let far = Rect::new(Point::new(5000.0, 5000.0), Point::new(6000.0, 6000.0));
        assert_eq!(fill_cells(&shape, Octave(0), far), []);
        let flat = Rect::new(Point::new(0.0, 0.0), Point::new(0.0, 10.0));
        assert_eq!(fill_cells(&shape, Octave(0), flat), []);
        let nan = Rect::new(Point::new(f64::NAN, 0.0), Point::new(1.0, 1.0));
        assert_eq!(fill_cells(&shape, Octave(0), nan), []);
    }
}
