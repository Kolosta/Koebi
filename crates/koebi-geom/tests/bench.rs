//! The geometry part of the performance baseline (`cargo xtask bench`): how long stroke
//! outlines take, on the machine that runs it.
//!
//! An ignored test: `cargo xtask bench` runs it in release and gathers the table it prints
//! between `BENCH-BEGIN` and `BENCH-END`. Wall times with `Instant`: compare them on one
//! machine, never across machines.

#![allow(
    clippy::print_stdout,
    reason = "the table goes to stdout, where cargo xtask bench reads it"
)]

use std::time::{Duration, Instant};

use koebi_geom::{
    Align, Cap, Contour, Contours, Dashes, Join, Path, Point, Segment, StrokeStyle, Stroker, Vector,
};

/// Runs of each measure, after one to warm up.
const RUNS: usize = 11;

/// The tolerance of the outlines: a sixteenth of a pixel at a zoom of 1.
const TOLERANCE: f64 = 1.0 / 16.0;

/// A small deterministic generator, so that every run draws the same strokes.
struct Random(u64);

impl Random {
    /// A number in `0..1`.
    fn next(&mut self) -> f64 {
        // Knuth's MMIX linear congruential generator; the high bits are the good ones.
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        #[allow(clippy::cast_precision_loss, reason = "53 bits of a uniform number")]
        let value = (self.0 >> 11) as f64 / (1_u64 << 53) as f64;
        value
    }

    fn point(&mut self, scale: f64) -> Point {
        Point::new(self.next() * scale, self.next() * scale)
    }
}

/// Times `run` `RUNS` times after a warm-up: median, 95th percentile and slowest, in µs.
fn measure(mut run: impl FnMut()) -> (f64, f64, f64) {
    run();
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            run();
            start.elapsed()
        })
        .collect();
    times.sort_unstable();
    let micros = |time: Duration| time.as_secs_f64() * 1e6;
    let p95 = (times.len() - 1) * 95 / 100;
    (
        micros(times[times.len() / 2]),
        micros(times[p95]),
        micros(times[times.len() - 1]),
    )
}

fn row(name: &str, what: &str, times: (f64, f64, f64), output: &str) {
    let (median, p95, max) = times;
    println!("| {name} | {what} | {median:.0} | {p95:.0} | {max:.0} | {output} |");
}

/// The number of segments of `outline`.
fn segments(outline: &Contours) -> usize {
    outline.contours().iter().map(|c| c.segments().len()).sum()
}

/// Outlines every path of `paths` in `style`, keeping the stroker and the output: what a
/// document re-outlining its strokes does. Returns the segments made.
fn outline_all(
    paths: &[Contours],
    style: &StrokeStyle,
    stroker: &mut Stroker,
    out: &mut Contours,
) -> usize {
    let mut total = 0;
    for path in paths {
        stroker.outline_into(path, style, TOLERANCE, out);
        total += segments(out);
    }
    total
}

#[test]
#[ignore = "benchmark: cargo xtask bench"]
fn baseline() {
    let mut random = Random(0x6b6f_6562);
    println!("BENCH-BEGIN");
    println!("| Geometry | Measure | Median (µs) | p95 (µs) | Max (µs) | Output |");
    println!("| --- | --- | ---: | ---: | ---: | --- |");

    // 10 000 smooth open strokes, as drawn: six anchors 20 to 40 apart, turning by 45° at
    // most from one to the next, joined by cubics whose handles keep the tangent
    // (Catmull-Rom).
    let smooth: Vec<Contours> = (0..10_000)
        .map(|_| {
            let mut anchors = vec![random.point(1000.0)];
            let mut heading = random.next() * core::f64::consts::TAU;
            for _ in 0..5 {
                heading += (random.next() - 0.5) * core::f64::consts::FRAC_PI_2;
                let step = 20.0 + random.next() * 20.0;
                let last = anchors[anchors.len() - 1];
                anchors.push(last + Vector::from_angle(heading) * step);
            }
            let handle = |i: usize| (anchors[(i + 1).min(5)] - anchors[i.saturating_sub(1)]) / 6.0;
            let segments = (0..5)
                .map(|i| {
                    let (a, b) = (anchors[i], anchors[i + 1]);
                    Segment::cubic(a, a + handle(i), b - handle(i + 1), b)
                })
                .collect();
            let mut contours = Contours::default();
            contours.push(Contour::new(segments, false));
            contours
        })
        .collect();
    // 10 000 scribbles: four cubics with handles at random, full of loops and near cusps,
    // where the offsets fold.
    let scribbles: Vec<Contours> = (0..10_000)
        .map(|_| {
            let mut segments = Vec::with_capacity(4);
            let mut start = random.point(1000.0);
            for _ in 0..4 {
                let end =
                    start + Vector::new(random.next() * 60.0 - 30.0, random.next() * 60.0 - 30.0);
                let a =
                    start + Vector::new(random.next() * 40.0 - 20.0, random.next() * 40.0 - 20.0);
                let b = end + Vector::new(random.next() * 40.0 - 20.0, random.next() * 40.0 - 20.0);
                segments.push(Segment::cubic(start, a, b, end));
                start = end;
            }
            let mut contours = Contours::default();
            contours.push(Contour::new(segments, false));
            contours
        })
        .collect();
    let (mut stroker, mut out) = (Stroker::new(), Contours::default());
    let round = StrokeStyle {
        width: 4.0,
        cap: Cap::Round,
        join: Join::Round,
        ..StrokeStyle::default()
    };
    let miter = StrokeStyle {
        width: 4.0,
        ..StrokeStyle::default()
    };
    let right = StrokeStyle {
        width: 4.0,
        align: Align::Right,
        ..StrokeStyle::default()
    };
    for (name, paths, what, style) in [
        (
            "10 000 smooth strokes",
            &smooth,
            "outline, centered, round",
            &round,
        ),
        (
            "10 000 smooth strokes",
            &smooth,
            "outline, centered, miter",
            &miter,
        ),
        (
            "10 000 smooth strokes",
            &smooth,
            "outline, right, miter",
            &right,
        ),
        (
            "10 000 scribbles",
            &scribbles,
            "outline, centered, round",
            &round,
        ),
    ] {
        let mut made = 0;
        let times = measure(|| made = outline_all(paths, style, &mut stroker, &mut out));
        row(name, what, times, &format!("{made} segments"));
    }

    // 10 000 closed shapes stroked inside: ellipses and polygons.
    let shapes: Vec<Contours> = (0..10_000)
        .map(|index| {
            let center = random.point(1000.0);
            let radius = 5.0 + random.next() * 25.0;
            let path = if index % 2 == 0 {
                Path::exact_ellipse(center, Vector::new(radius, radius * (0.5 + random.next())))
            } else {
                Path::polygon(center, radius, 3 + index % 7, random.next())
            };
            path.contours()
        })
        .collect();
    let inside = StrokeStyle {
        width: 3.0,
        align: Align::Inside,
        ..StrokeStyle::default()
    };
    let mut made = 0;
    let times = measure(|| made = outline_all(&shapes, &inside, &mut stroker, &mut out));
    row(
        "10 000 closed shapes",
        "outline, inside, miter",
        times,
        &format!("{made} segments"),
    );

    // One long dashed curve: a wave of 2000 cubics, 160 000 units long, in short dashes.
    let mut wave = Vec::with_capacity(2000);
    for index in 0..2000 {
        let x = f64::from(index) * 60.0;
        let bend = if index % 2 == 0 { 40.0 } else { -40.0 };
        wave.push(Segment::cubic(
            Point::new(x, 0.0),
            Point::new(x + 20.0, bend),
            Point::new(x + 40.0, bend),
            Point::new(x + 60.0, 0.0),
        ));
    }
    let mut long = Contours::default();
    long.push(Contour::new(wave, false));
    let length = long.contours()[0].length();
    let dashed = StrokeStyle {
        width: 2.0,
        cap: Cap::Round,
        join: Join::Round,
        dashes: Dashes {
            pattern: vec![6.0, 4.0],
            ..Dashes::default()
        },
        ..StrokeStyle::default()
    };
    let times = measure(|| stroker.outline_into(&long, &dashed, TOLERANCE, &mut out));
    let output = format!(
        "{:.0} long, {} dashes, {} segments",
        length,
        out.contours().len(),
        segments(&out)
    );
    row(
        "One long dashed curve",
        "outline, round caps",
        times,
        &output,
    );
    println!("BENCH-END");
}
