//! Outlines prepared for the canvas's coverage ([ADR 0029]): quadratic Béziers, monotone in x
//! and y, within a sixteenth of a pixel of the outline, cut along a square grid whose cells have
//! f64 origins, each curve stored relative to its cell in f32.
//!
//! Precision has no limit of zoom: the tolerance and the cell size follow the zoom's octave
//! ([`Octave`]), and only the part of the outline inside the area to draw is prepared. A cell is
//! 1024 pixels wide at its octave, so a curve's coordinates in its cell stay under 2¹¹ pixels,
//! far below the 2²⁰ where an f32 still holds sixteenths of a pixel, at a zoom of 10⁹ as at 1.
//!
//! [ADR 0029]: https://github.com/Kolosta/Koebi/blob/develop/docs/adr/0029-canvas-rendering-from-curves.md

use crate::{Contour, Contours, Point, Rect, Segment, solve};

/// A zoom octave: zooms from `2^k` to `2^(k+1)` pixels per document unit share a tolerance
/// and a cell size. There is no bound on `k`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Octave(pub i32);

impl Octave {
    /// The octave of `zoom` (pixels per unit, positive and finite).
    pub fn of(zoom: f64) -> Self {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "the binary exponent of an f64, within ±1100"
        )]
        Self(zoom.log2().floor() as i32)
    }

    /// The tolerance of its curves: a sixteenth of a pixel at the octave's largest zoom.
    pub fn tolerance(self) -> f64 {
        (-f64::from(self.0) - 1.0).exp2() / 16.0
    }

    /// The width of its cells: 1024 pixels at the octave's smallest zoom. A power of two, so
    /// that cell origins are exact.
    pub fn cell_size(self) -> f64 {
        (10.0 - f64::from(self.0)).exp2()
    }

    /// How far a cell's curves reach past its sides: one pixel at the octave's smallest zoom,
    /// more than a pixel's footprint reaches past the cell at any zoom of the octave. A power
    /// of two, like the cell size.
    pub fn margin(self) -> f64 {
        (-f64::from(self.0)).exp2()
    }
}

/// A quadratic Bézier curve, monotone in x and in y.
pub type Quadratic = [Point; 3];

/// The curves of one grid cell.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    /// Its column: the cell spans `column * size..(column + 1) * size` in x.
    pub column: i64,
    /// Its row: the cell spans `row * size..(row + 1) * size` in y.
    pub row: i64,
    /// Its corner of least x and y, exact.
    pub origin: Point,
    /// The curves inside it, relative to its origin.
    pub curves: Vec<[[f32; 2]; 3]>,
}

/// Appends the quadratics of `contours` within `area`, within `tolerance`: each contour closed by
/// a line if it is open (as fills close it), its segments cut to the area, made quadratic and
/// split where they turn in x or in y.
pub fn quadratics(contours: &Contours, tolerance: f64, area: Rect, out: &mut Vec<Quadratic>) {
    let tolerance = if tolerance > 0.0 && tolerance.is_finite() {
        tolerance
    } else {
        1e-3
    };
    let mut parts = Vec::new();
    for contour in contours.contours() {
        for segment in closed(contour) {
            if !segment.is_finite() || !segment.bounds().intersects(area) {
                continue;
            }
            monotone(&segment, &mut parts);
            for part in parts.drain(..) {
                if let Some(inside) = clip(&part, area) {
                    to_quadratics(&inside, tolerance, out);
                }
            }
        }
    }
}

/// The curves of `contours` prepared for drawing `area` at `octave`: grid cells of the octave's
/// size, each with the curves inside it relative to its origin, ordered by row then column.
/// Cells without curves are left out.
pub fn prepare(contours: &Contours, octave: Octave, area: Rect) -> Vec<Cell> {
    let size = octave.cell_size();
    // Whole cells around the area, so that the cells drawn hold every curve inside them.
    let snap = |value: f64, up: bool| {
        let cells = value / size;
        (if up { cells.ceil() } else { cells.floor() }) * size
    };
    let area = Rect::new(
        Point::new(snap(area.min.x, false), snap(area.min.y, false)),
        Point::new(snap(area.max.x, true), snap(area.max.y, true)),
    );
    let mut curves = Vec::new();
    quadratics(contours, octave.tolerance(), area, &mut curves);
    cells(&curves, size)
}

/// `curves` cut along the grid of `size` and filed by cell.
pub fn cells(curves: &[Quadratic], size: f64) -> Vec<Cell> {
    let mut cells: std::collections::BTreeMap<(i64, i64), Vec<[[f32; 2]; 3]>> =
        std::collections::BTreeMap::new();
    let mut pieces = Vec::new();
    for curve in curves {
        pieces.clear();
        cut_along_grid(*curve, size, &mut pieces);
        for piece in &pieces {
            let middle = piece[0].midpoint(piece[2]);
            #[allow(
                clippy::cast_possible_truncation,
                reason = "a cell index: the coordinate over the cell size, far below 2^63"
            )]
            let (column, row) = (
                (middle.x / size).floor() as i64,
                (middle.y / size).floor() as i64,
            );
            #[allow(
                clippy::cast_precision_loss,
                reason = "a cell index times a power of two: exact while the result is"
            )]
            let origin = Point::new(column as f64 * size, row as f64 * size);
            #[allow(
                clippy::cast_possible_truncation,
                reason = "within its cell, a coordinate fits an f32 with sixteenths of a pixel"
            )]
            let relative = piece.map(|p| [(p.x - origin.x) as f32, (p.y - origin.y) as f32]);
            cells.entry((row, column)).or_default().push(relative);
        }
    }
    cells
        .into_iter()
        .map(|((row, column), curves)| {
            #[allow(clippy::cast_precision_loss, reason = "as above")]
            let origin = Point::new(column as f64 * size, row as f64 * size);
            Cell {
                column,
                row,
                origin,
                curves,
            }
        })
        .collect()
}

/// The segments of `contour`, with the closing line of an open one.
pub(crate) fn closed(contour: &Contour) -> impl Iterator<Item = Segment> + '_ {
    let segments = contour.segments();
    let closing = match (segments.first(), segments.last()) {
        (Some(first), Some(last)) if last.end() != first.start() => {
            Some(Segment::line(last.end(), first.start()))
        }
        _ => None,
    };
    segments.iter().copied().chain(closing)
}

/// The pieces of `segment` between its turning points in x and in y.
pub(crate) fn monotone(segment: &Segment, out: &mut Vec<Segment>) {
    let (cuts, count) = crate::boolean::turning_points(segment);
    let mut t0 = 0.0;
    for &t1 in cuts[..count].iter().chain(core::iter::once(&1.0)) {
        if t1 > t0 {
            out.push(segment.subsegment(t0, t1));
        }
        t0 = t1;
    }
}

/// The part of a monotone `segment` inside `area`: one stretch, a monotone curve meeting a
/// box's sides once each at most. `None` when it misses the area.
pub(crate) fn clip(segment: &Segment, area: Rect) -> Option<Segment> {
    let (start, end) = (segment.start(), segment.end());
    let mut range = (0.0_f64, 1.0_f64);
    for (axis, low, high) in [(0, area.min.x, area.max.x), (1, area.min.y, area.max.y)] {
        let coordinate = |p: Point| if axis == 0 { p.x } else { p.y };
        let (a, b) = (coordinate(start), coordinate(end));
        if a.max(b) < low || a.min(b) > high {
            return None;
        }
        let at = |value: f64| crossing(segment, value, coordinate);
        // The parameters where the coordinate enters and leaves low..=high.
        let (enter, leave) = if a <= b {
            (
                if a < low { at(low) } else { 0.0 },
                if b > high { at(high) } else { 1.0 },
            )
        } else {
            (
                if a > high { at(high) } else { 0.0 },
                if b < low { at(low) } else { 1.0 },
            )
        };
        range = (range.0.max(enter), range.1.min(leave));
    }
    (range.1 > range.0).then(|| segment.subsegment(range.0, range.1))
}

/// Where a segment monotone in `coordinate` reaches `value`.
pub(crate) fn crossing(segment: &Segment, value: f64, coordinate: impl Fn(Point) -> f64) -> f64 {
    let rising = coordinate(segment.end()) >= coordinate(segment.start());
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for _ in 0..64 {
        let middle = lo.midpoint(hi);
        if (coordinate(segment.eval(middle)) < value) == rising {
            lo = middle;
        } else {
            hi = middle;
        }
    }
    lo.midpoint(hi)
}

/// Appends quadratics within `tolerance` of a monotone `segment`, themselves monotone.
pub(crate) fn to_quadratics(segment: &Segment, tolerance: f64, out: &mut Vec<Quadratic>) {
    let mut quads = Vec::new();
    match *segment {
        Segment::Line([a, b]) => quads.push([a, a.midpoint(b), b]),
        Segment::Quad(q) => quads.push(q),
        Segment::Cubic([p0, p1, p2, p3]) => {
            // The quadratic through the cubic's ends with the control point
            // (3·(P1 + P2) - P0 - P3) / 4 strays at most √3/36·|P3 - 3·P2 + 3·P1 - P0|, and
            // n equal pieces by 1/n³ of it.
            let third = (p3.to_vector() - p2.to_vector() * 3.0 + p1.to_vector() * 3.0
                - p0.to_vector())
            .length();
            let error = 3.0_f64.sqrt() / 36.0 * third;
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a count of pieces, a cube root of a ratio"
            )]
            let count = (error / tolerance).cbrt().ceil().clamp(1.0, 4096.0) as u32;
            for i in 0..count {
                let (t0, t1) = (
                    f64::from(i) / f64::from(count),
                    f64::from(i + 1) / f64::from(count),
                );
                let Segment::Cubic([a, b, c, d]) = segment.subsegment(t0, t1) else {
                    unreachable!("a cubic's part is a cubic");
                };
                let control =
                    (b.to_vector() * 3.0 + c.to_vector() * 3.0 - a.to_vector() - d.to_vector())
                        / 4.0;
                quads.push([a, control.to_point(), d]);
            }
        }
        Segment::Rational(_) => {
            let mut fitted = Vec::new();
            if crate::stroke::fit_quadratics(segment, tolerance, &mut fitted) {
                quads.extend(fitted.iter().filter_map(|s| match *s {
                    Segment::Quad(q) => Some(q),
                    Segment::Line([a, b]) => Some([a, a.midpoint(b), b]),
                    _ => None,
                }));
            } else {
                // A degenerate rational: a polyline within the tolerance.
                let mut points = vec![segment.start()];
                segment.flatten(tolerance, &mut points);
                quads.extend(points.windows(2).map(|w| [w[0], w[0].midpoint(w[1]), w[1]]));
            }
        }
    }
    // A fitted control point may overshoot an end by a hair: split where each quadratic turns.
    for quad in quads {
        let mut parts = Vec::new();
        monotone(&Segment::Quad(quad), &mut parts);
        for part in parts {
            if let Segment::Quad(q) = part {
                out.push(q);
            }
        }
    }
}

/// Appends the parts of a monotone `curve` between the lines of the grid of `size`.
fn cut_along_grid(curve: Quadratic, size: f64, out: &mut Vec<Quadratic>) {
    let segment = Segment::Quad(curve);
    let mut cuts: Vec<f64> = Vec::new();
    for axis in 0..2 {
        let coordinate = |p: Point| if axis == 0 { p.x } else { p.y };
        let (a, b, c) = (
            coordinate(curve[0]),
            coordinate(curve[1]),
            coordinate(curve[2]),
        );
        let (low, high) = (a.min(c), a.max(c));
        let mut line = (low / size).floor() + 1.0;
        while line * size < high {
            let value = line * size;
            // (1-t)²a + 2t(1-t)b + t²c = value.
            let roots = solve::quadratic(a - 2.0 * b + c, 2.0 * (b - a), a - value);
            if let Some(&t) = roots.as_slice().iter().find(|t| (0.0..=1.0).contains(*t)) {
                cuts.push(t);
            }
            line += 1.0;
        }
    }
    cuts.sort_unstable_by(f64::total_cmp);
    let mut t0 = 0.0;
    for t1 in cuts.into_iter().chain(core::iter::once(1.0)) {
        if t1 > t0
            && let Segment::Quad(part) = segment.subsegment(t0, t1)
        {
            out.push(part);
        }
        t0 = t1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FillRule, Path, Vector};

    #[test]
    fn octaves_halve_the_tolerance_and_the_cells() {
        assert_eq!(Octave::of(1.0), Octave(0));
        assert_eq!(Octave::of(3.0), Octave(1));
        assert_eq!(Octave::of(0.3), Octave(-2));
        assert!((Octave(0).tolerance() - 1.0 / 32.0).abs() < 1e-15);
        assert!((Octave(3).cell_size() - 128.0).abs() < 1e-15);
        assert_eq!(Octave::of(1e9), Octave(29));
    }

    /// The largest distance from points sampled on `curves` to `contours`.
    fn stray(curves: &[Quadratic], contours: &Contours) -> f64 {
        curves
            .iter()
            .flat_map(|q| (0..=8).map(move |i| Segment::Quad(*q).eval(f64::from(i) / 8.0)))
            .map(|p| contours.distance(p))
            .fold(0.0, f64::max)
    }

    #[test]
    fn every_kind_becomes_monotone_quadratics_within_the_tolerance() {
        let mut path = Contours::default();
        path.push(Contour::new(
            vec![
                Segment::cubic(
                    Point::new(0.0, 0.0),
                    Point::new(60.0, -40.0),
                    Point::new(-20.0, 80.0),
                    Point::new(50.0, 50.0),
                ),
                Segment::line(Point::new(50.0, 50.0), Point::new(0.0, 0.0)),
            ],
            true,
        ));
        let circle = Path::exact_ellipse(Point::new(100.0, 0.0), Vector::new(30.0, 20.0));
        for contour in circle.contours().contours() {
            path.push(contour.clone());
        }
        let area = Rect::new(Point::new(-100.0, -100.0), Point::new(200.0, 200.0));
        let mut curves = Vec::new();
        quadratics(&path, 1e-3, area, &mut curves);
        assert!(stray(&curves, &path) <= 1e-3, "{}", stray(&curves, &path));
        for q in &curves {
            for axis in 0..2 {
                assert!(
                    Segment::Quad(*q).extrema(axis).as_slice().is_empty(),
                    "{q:?}"
                );
            }
        }
        // They make the same fill: windings agree away from the outline. Each curve counts on
        // its own, so the loose curves of a whole outline wind as the outline does.
        let mut prepared = Contours::default();
        prepared.push(Contour::new(
            curves.iter().map(|q| Segment::Quad(*q)).collect(),
            true,
        ));
        for i in 0..40 {
            for j in 0..40 {
                let q = Point::new(-20.0 + f64::from(i) * 4.0, -40.0 + f64::from(j) * 3.0);
                if path.distance(q) > 1e-2 {
                    let expected = path.contains(q, FillRule::NonZero);
                    assert_eq!(
                        prepared.contains(q, FillRule::NonZero),
                        expected,
                        "at {q:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn only_the_area_is_prepared() {
        let circle = Path::exact_ellipse(Point::new(0.0, 0.0), Vector::new(100.0, 100.0));
        let contours = circle.contours();
        let area = Rect::new(Point::new(90.0, -10.0), Point::new(110.0, 10.0));
        let mut curves = Vec::new();
        quadratics(&contours, 1e-4, area, &mut curves);
        assert_ne!(curves.len(), 0);
        for q in &curves {
            for p in q {
                assert!(area.inflated(1e-6).contains(*p), "{p:?}");
            }
        }
    }

    #[test]
    fn a_kilometre_wide_outline_at_a_zoom_of_a_billion_keeps_small_cell_coordinates() {
        // A circle a kilometre across, in metres, drawn at 10^9 pixels per metre.
        let circle = Path::exact_ellipse(Point::new(0.0, 0.0), Vector::new(500.0, 500.0));
        let contours = circle.contours();
        let zoom = 1e9;
        let octave = Octave::of(zoom);
        // A screen of 1920 × 1080 pixels on the circle's right edge.
        let (width, height) = (1920.0 / zoom, 1080.0 / zoom);
        let center = Point::new(500.0, 0.0);
        let area = Rect::new(
            Point::new(center.x - width / 2.0, center.y - height / 2.0),
            Point::new(center.x + width / 2.0, center.y + height / 2.0),
        );
        let cells = prepare(&contours, octave, area);
        assert_ne!(cells.len(), 0);
        for cell in &cells {
            for curve in &cell.curves {
                for point in curve {
                    for value in point {
                        let pixels = f64::from(value.abs()) * zoom;
                        assert!(pixels < f64::from(1_u32 << 20), "{pixels}");
                    }
                    // Back in metres, within a sixteenth of a pixel of the circle.
                    let back = Point::new(
                        cell.origin.x + f64::from(point[0]),
                        cell.origin.y + f64::from(point[1]),
                    );
                    let off = (back.to_vector().length() - 500.0).abs() * zoom;
                    assert!(off < 1.0 / 16.0, "{off} px");
                }
            }
        }
    }

    #[test]
    fn curves_are_cut_at_cell_sides() {
        let line = [
            Point::new(0.5, 0.5),
            Point::new(2.0, 1.0),
            Point::new(3.5, 1.5),
        ];
        let cells = cells(&[line], 1.0);
        // From x 0.5 to 3.5 and y 0.5 to 1.5: cells (0,0), (1,0)…(1,1), (2,1), (3,1).
        for cell in &cells {
            for curve in &cell.curves {
                for point in curve {
                    assert!((-1e-6..=1.0 + 1e-6).contains(&point[0]), "{cell:?}");
                    assert!((-1e-6..=1.0 + 1e-6).contains(&point[1]), "{cell:?}");
                }
            }
        }
        assert!(cells.len() >= 4);
    }
}
