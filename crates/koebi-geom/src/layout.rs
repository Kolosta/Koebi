//! Layouts: where a fill stamps its instances. The points of a lattice over a region, and
//! points scattered at random but evenly (blue noise), from a seed: the same seed gives the
//! same points on every platform.
//!
//! - [`Rng`]: `SplitMix64`, a small generator whose sequence depends on its seed alone.
//! - [`lattice`]: the points `origin + i·a + j·b` in a rectangle.
//! - [`scatter`]: points over a region, by distance (Bridson's Poisson-disk sampling, each new
//!   point drawn between the least and the most distance from one already placed) or by count
//!   (Mitchell's best candidate: of a few points drawn at random, the one farthest from the
//!   others, until there are as many as asked). A grid of buckets keeps both near linear.

use crate::{Point, Rect, Vector};

/// A deterministic generator of random numbers: `SplitMix64`.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// A generator whose sequence is that of `seed`.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A number in `[0, 1)`, 53 bits of it random.
    pub fn unit(&mut self) -> f64 {
        #[allow(
            clippy::cast_precision_loss,
            reason = "53 bits, which an f64 holds exactly"
        )]
        let bits = (self.next_u64() >> 11) as f64;
        bits * (1.0 / 9_007_199_254_740_992.0)
    }

    /// A number in `[-1, 1)`.
    pub fn symmetric(&mut self) -> f64 {
        2.0 * self.unit() - 1.0
    }
}

/// Appends to `out` the points `origin + i·a + j·b`, for whole `i` and `j`, that lie in
/// `bounds`, row by row, `limit` at most. Nothing when the axes are parallel or not finite.
pub fn lattice(
    bounds: Rect,
    origin: Point,
    a: Vector,
    b: Vector,
    limit: usize,
    out: &mut Vec<Point>,
) {
    let determinant = a.x * b.y - a.y * b.x;
    if bounds.is_empty() || !determinant.is_finite() || determinant.abs() < f64::MIN_POSITIVE {
        return;
    }
    // The lattice coordinates of the corners give the rows and columns to walk.
    let steps = |point: Point| {
        let d = point - origin;
        (
            (d.x * b.y - d.y * b.x) / determinant,
            (a.x * d.y - a.y * d.x) / determinant,
        )
    };
    let (mut i0, mut i1, mut j0, mut j1) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for corner in [
        bounds.min,
        Point::new(bounds.max.x, bounds.min.y),
        bounds.max,
        Point::new(bounds.min.x, bounds.max.y),
    ] {
        let (i, j) = steps(corner);
        (i0, i1, j0, j1) = (i0.min(i), i1.max(i), j0.min(j), j1.max(j));
    }
    let (i0, i1, j0, j1) = (i0.floor(), i1.ceil(), j0.floor(), j1.ceil());
    if !(i0.is_finite() && i1.is_finite() && j0.is_finite() && j1.is_finite()) {
        return;
    }
    // Rows cross the bounds' corners: a walk visits a few times the points it keeps at most,
    // and stops there whatever the bounds.
    let mut visits = limit.saturating_mul(8).saturating_add(64);
    let start = out.len();
    let mut j = j0;
    while j <= j1 {
        let mut i = i0;
        while i <= i1 {
            if visits == 0 || out.len() - start >= limit {
                return;
            }
            visits -= 1;
            let point = origin + a * i + b * j;
            if bounds.contains(point) {
                out.push(point);
            }
            i += 1.0;
        }
        j += 1.0;
    }
}

/// How scattered points are spaced.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Spacing {
    /// About this many points over the region, as evenly as can be.
    Count(usize),
    /// Every point at least `min` from the others and within `max` of one: the region filled
    /// at that density, however many points that makes.
    Distance {
        /// The least distance between two points.
        min: f64,
        /// The most distance from a point to the one it grew from.
        max: f64,
    },
}

/// Attempts per point: of the candidates around a point (by distance) or drawn at random (by
/// count).
const ATTEMPTS: usize = 24;

/// Buckets of points, to find the ones near a place.
struct Buckets {
    origin: Point,
    side: f64,
    columns: usize,
    rows: usize,
    cells: Vec<Vec<usize>>,
}

impl Buckets {
    fn new(bounds: Rect, side: f64) -> Option<Self> {
        let side = side.max(bounds.width().max(bounds.height()) / 2048.0);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "at most 2048 buckets a side"
        )]
        let count = |length: f64| ((length / side).ceil() as usize).clamp(1, 2048);
        let (columns, rows) = (count(bounds.width()), count(bounds.height()));
        side.is_finite().then(|| Self {
            origin: bounds.min,
            side,
            columns,
            rows,
            cells: vec![Vec::new(); columns * rows],
        })
    }

    fn cell(&self, point: Point) -> (usize, usize) {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a bucket index, clamped"
        )]
        let at = |value: f64, count: usize| {
            ((value / self.side).floor().max(0.0) as usize).min(count - 1)
        };
        (
            at(point.x - self.origin.x, self.columns),
            at(point.y - self.origin.y, self.rows),
        )
    }

    fn add(&mut self, point: Point, index: usize) {
        let (column, row) = self.cell(point);
        self.cells[row * self.columns + column].push(index);
    }

    /// The distance from `point` to the nearest of `points` within `reach`, `reach` itself
    /// when none is that near.
    fn nearest(&self, points: &[Point], point: Point, reach: f64) -> f64 {
        let (column, row) = self.cell(point);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a count of buckets, clamped"
        )]
        let rings = ((reach / self.side).ceil() as usize + 1).min(self.columns.max(self.rows));
        let mut best = reach;
        for r in row.saturating_sub(rings)..(row + rings + 1).min(self.rows) {
            for c in column.saturating_sub(rings)..(column + rings + 1).min(self.columns) {
                for &index in &self.cells[r * self.columns + c] {
                    best = best.min((points[index] - point).length());
                }
            }
        }
        best
    }
}

/// Appends to `out` points scattered over `bounds` where `inside` holds, by `spacing`, each at
/// least `2 · clearance` farther from the others than the spacing asks (shapes of that radius
/// then never overlap), `limit` at most, drawn from `rng`. See the module.
pub fn scatter(
    bounds: Rect,
    spacing: Spacing,
    clearance: f64,
    rng: &mut Rng,
    limit: usize,
    inside: impl Fn(Point) -> bool,
    out: &mut Vec<Point>,
) {
    let area = bounds.width() * bounds.height();
    if !(area.is_finite() && area > 0.0) || limit == 0 {
        return;
    }
    let clearance = clearance.max(0.0) * 2.0;
    let random = |rng: &mut Rng| {
        Point::new(
            bounds.min.x + rng.unit() * bounds.width(),
            bounds.min.y + rng.unit() * bounds.height(),
        )
    };
    let start = out.len();
    match spacing {
        Spacing::Distance { min, max } => {
            let min = min.max(0.0) + clearance;
            let max = max.max(min);
            if !(min > 0.0 && max.is_finite()) {
                return;
            }
            let Some(mut buckets) = Buckets::new(bounds, min / core::f64::consts::SQRT_2) else {
                return;
            };
            let mut placed: Vec<Point> = Vec::new();
            let mut active: Vec<usize> = Vec::new();
            // A first point inside, if one is found.
            for _ in 0..ATTEMPTS * 4 {
                let point = random(rng);
                if inside(point) {
                    buckets.add(point, 0);
                    placed.push(point);
                    active.push(0);
                    break;
                }
            }
            while !active.is_empty() && placed.len() < limit {
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    clippy::cast_precision_loss,
                    reason = "an index into the active points"
                )]
                let pick = ((rng.unit() * active.len() as f64) as usize).min(active.len() - 1);
                let from = placed[active[pick]];
                let mut grown = false;
                for _ in 0..ATTEMPTS {
                    let angle = rng.unit() * core::f64::consts::TAU;
                    let distance = min + rng.unit() * (max - min);
                    let point = from + Vector::new(angle.cos(), angle.sin()) * distance;
                    if bounds.contains(point)
                        && inside(point)
                        && buckets.nearest(&placed, point, min) >= min
                    {
                        buckets.add(point, placed.len());
                        active.push(placed.len());
                        placed.push(point);
                        grown = true;
                        break;
                    }
                }
                if !grown {
                    active.swap_remove(pick);
                }
            }
            out.extend_from_slice(&placed);
        }
        Spacing::Count(count) => {
            let count = count.min(limit);
            #[allow(clippy::cast_precision_loss, reason = "a count of points")]
            let typical = (area / count.max(1) as f64).sqrt();
            let Some(mut buckets) = Buckets::new(bounds, typical) else {
                return;
            };
            let mut placed: Vec<Point> = Vec::new();
            let reach = typical * 4.0;
            let mut misses = 0;
            while placed.len() < count && misses < ATTEMPTS * 4 {
                // Of a few candidates inside, the one farthest from the points placed.
                let mut best: Option<(Point, f64)> = None;
                for _ in 0..ATTEMPTS {
                    let point = random(rng);
                    if !inside(point) {
                        continue;
                    }
                    let distance = buckets.nearest(&placed, point, reach);
                    if distance >= clearance && best.is_none_or(|(_, far)| distance > far) {
                        best = Some((point, distance));
                    }
                }
                if let Some((point, _)) = best {
                    buckets.add(point, placed.len());
                    placed.push(point);
                    misses = 0;
                } else {
                    misses += 1;
                }
            }
            out.extend_from_slice(&placed);
        }
    }
    debug_assert!(out.len() - start <= limit);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
        Rect::new(Point::new(x0, y0), Point::new(x1, y1))
    }

    #[test]
    fn the_generator_depends_on_its_seed_alone() {
        let (mut a, mut b, mut c) = (Rng::new(7), Rng::new(7), Rng::new(8));
        let first: Vec<u64> = (0..4).map(|_| a.next_u64()).collect();
        assert_eq!(first, (0..4).map(|_| b.next_u64()).collect::<Vec<_>>());
        assert_ne!(first[0], c.next_u64());
        // SplitMix64's first output for a seed of 0.
        assert_eq!(Rng::new(0).next_u64(), 0xe220_a839_7b1d_cdaf);
        let units: Vec<f64> = (0..1000).map(|_| a.unit()).collect();
        assert!(units.iter().all(|u| (0.0..1.0).contains(u)));
        let mean = units.iter().sum::<f64>() / 1000.0;
        assert!((mean - 0.5).abs() < 0.05, "{mean}");
    }

    #[test]
    fn a_lattice_gives_its_points_in_the_bounds() {
        let mut points = Vec::new();
        lattice(
            rect(0.0, 0.0, 30.0, 20.0),
            Point::new(5.0, 5.0),
            Vector::new(10.0, 0.0),
            Vector::new(0.0, 10.0),
            1000,
            &mut points,
        );
        // x in 5, 15, 25 and y in 5, 15.
        assert_eq!(points.len(), 6);
        assert!(points.contains(&Point::new(25.0, 15.0)));
        // A turned lattice, and a limit.
        points.clear();
        let (sin, cos) = 0.3_f64.sin_cos();
        lattice(
            rect(-50.0, -50.0, 50.0, 50.0),
            Point::new(0.0, 0.0),
            Vector::new(cos, sin) * 7.0,
            Vector::new(-sin, cos) * 7.0,
            40,
            &mut points,
        );
        assert_eq!(points.len(), 40);
        // Parallel axes give nothing.
        points.clear();
        let along = Vector::new(1.0, 0.0);
        lattice(
            rect(0.0, 0.0, 9.0, 9.0),
            Point::ORIGIN,
            along,
            along,
            9,
            &mut points,
        );
        assert_eq!(points.len(), 0);
    }

    fn least_distance(points: &[Point]) -> f64 {
        let mut least = f64::INFINITY;
        for (at, a) in points.iter().enumerate() {
            for b in &points[at + 1..] {
                least = least.min((*a - *b).length());
            }
        }
        least
    }

    #[test]
    fn points_scattered_by_distance_fill_the_region_and_keep_apart() {
        let bounds = rect(0.0, 0.0, 200.0, 120.0);
        let inside = |point: Point| (point - Point::new(100.0, 60.0)).length() < 55.0;
        let mut points = Vec::new();
        let spacing = Spacing::Distance {
            min: 8.0,
            max: 16.0,
        };
        scatter(
            bounds,
            spacing,
            0.0,
            &mut Rng::new(3),
            100_000,
            inside,
            &mut points,
        );
        assert!(least_distance(&points) >= 8.0 - 1e-9);
        assert!(points.iter().all(|point| inside(*point)));
        // The disc of radius 55 holds about its area over the area each point keeps.
        assert!(points.len() > 60 && points.len() < 200, "{}", points.len());
        // The same seed, the same points; shapes of radius 2 keep 4 more apart.
        let mut again = Vec::new();
        scatter(
            bounds,
            spacing,
            0.0,
            &mut Rng::new(3),
            100_000,
            inside,
            &mut again,
        );
        assert_eq!(points, again);
        again.clear();
        scatter(
            bounds,
            spacing,
            2.0,
            &mut Rng::new(3),
            100_000,
            inside,
            &mut again,
        );
        assert!(least_distance(&again) >= 12.0 - 1e-9);
    }

    #[test]
    fn points_scattered_by_count_are_as_many_and_spread() {
        let bounds = rect(0.0, 0.0, 100.0, 100.0);
        let mut points = Vec::new();
        scatter(
            bounds,
            Spacing::Count(200),
            0.0,
            &mut Rng::new(11),
            100_000,
            |_| true,
            &mut points,
        );
        assert_eq!(points.len(), 200);
        // Blue noise: far more spread than 200 points at random, whose nearest pair is
        // typically a fraction of a unit apart.
        assert!(least_distance(&points) > 2.0, "{}", least_distance(&points));
        // A limit caps the count.
        points.clear();
        scatter(
            bounds,
            Spacing::Count(200),
            0.0,
            &mut Rng::new(11),
            50,
            |_| true,
            &mut points,
        );
        assert_eq!(points.len(), 50);
    }
}
