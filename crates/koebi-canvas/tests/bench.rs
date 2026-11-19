//! The canvas part of the performance baseline (`cargo xtask bench`): how long views of 10 000
//! paths take to draw, from nothing, still, panned and zoomed, after one edit, how long a pick
//! takes, the same for 10 000 strokes in pixels, and 500 groups blended in every mode, on the
//! machine that runs it.
//!
//! An ignored test: `cargo xtask bench` runs it in release and gathers the table it prints
//! between `BENCH-BEGIN` and `BENCH-END`. Times are wall times of the render and the GPU's
//! work, waited for, unless a row says the CPU alone.

#![allow(
    clippy::print_stdout,
    reason = "the table goes to stdout, where cargo xtask bench reads it"
)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use koebi_canvas::{Canvas, View, ViewTarget};
use koebi_core::{Extent, LinearRgba};
use koebi_doc::eval::{Draw, Evaluator, MergeInput, Program, Step, StepId, ViewStroke};
use koebi_doc::kinds::{fill, path_object};
use koebi_doc::ops::{self, Shape};
use koebi_doc::{BlendMode, Document, Edit, NodeId, SocketRef, Value};
use koebi_geom::{Contour, Contours, FillRule, Path, Point, Rect, Segment, StrokeStyle, Vector};
use koebi_testkit::HeadlessGpu;

/// Runs of each measure, after one to warm up.
const RUNS: usize = 11;

/// The median, 95th percentile and largest of `times`, in microseconds.
fn stats(mut times: Vec<Duration>) -> (f64, f64, f64) {
    times.sort_unstable();
    let micros = |time: Duration| time.as_secs_f64() * 1e6;
    (
        micros(times[times.len() / 2]),
        micros(times[(times.len() - 1) * 95 / 100]),
        micros(times[times.len() - 1]),
    )
}

fn measure(mut run: impl FnMut()) -> (f64, f64, f64) {
    run();
    stats(
        (0..RUNS)
            .map(|_| {
                let start = Instant::now();
                run();
                start.elapsed()
            })
            .collect(),
    )
}

fn row(name: &str, what: &str, (median, p95, max): (f64, f64, f64), output: &str) {
    println!("| {name} | {what} | {median:.0} | {p95:.0} | {max:.0} | {output} |");
}

/// The view of the bench: full HD, the 300 × 300 units of the objects in sight.
fn view(center: Point, zoom: f64) -> View {
    View {
        center,
        zoom,
        extent: Extent::new(1920, 1080),
    }
}

/// Renders `program` through `view` into `target` and waits for the GPU; whether it drew.
fn render(
    gpu: &HeadlessGpu,
    canvas: &mut Canvas,
    target: &mut ViewTarget,
    program: &Program,
    view: &View,
) -> bool {
    gpu.device
        .submit_and_wait(|encoder| canvas.render(encoder, target, program, view))
        .expect("submitted")
        .expect("rendered")
}

/// A page of 10 000 rectangles of 1 to 7 by 1 to 5 units, 100 to a row 3 units apart, a fill
/// each, as the document benchmark has.
fn document() -> (Document, Vec<NodeId>) {
    let mut doc = Document::new();
    let page = ops::add_page(&mut doc).expect("a page");
    let objects = (0..10_000_u32)
        .map(|i| {
            let size = Vector::new(1.0 + f64::from(i % 7), 1.0 + f64::from(i % 5));
            let object =
                ops::add_shape(&mut doc, page.stack, Shape::Rectangle(size)).expect("a shape");
            let location = Value::Vector(Vector::new(
                f64::from(i % 100) * 3.0,
                f64::from(i / 100) * 3.0,
            ));
            doc.apply(Edit::SetInput {
                node: object,
                index: path_object::LOCATION.index(),
                value: location,
            })
            .expect("a location");
            object
        })
        .collect();
    (doc, objects)
}

/// 10 000 strokes 1 pixel wide: open zigzags of four lines, 100 to a row 3 units apart.
fn hairlines() -> Program {
    let style = Arc::new(StrokeStyle {
        width: 1.0,
        ..StrokeStyle::default()
    });
    let mut steps: Vec<Step> = (0..10_000_u32)
        .map(|i| {
            let origin = Point::new(f64::from(i % 100) * 3.0, f64::from(i / 100) * 3.0);
            let points: Vec<Point> = [(0.0, 0.0), (1.0, 2.0), (2.0, 0.0), (2.5, 2.0), (2.8, 1.0)]
                .iter()
                .map(|&(x, y)| Point::new(origin.x + x, origin.y + y))
                .collect();
            let segments = points
                .windows(2)
                .map(|pair| Segment::line(pair[0], pair[1]))
                .collect();
            let mut geometry = Contours::default();
            geometry.push(Contour::new(segments, false));
            Step::ViewStroke(ViewStroke {
                object: None,
                geometry: Arc::new(geometry),
                style: Arc::clone(&style),
                color: LinearRgba::new(0.1, 0.1, 0.1, 1.0),
                opacity: 1.0,
            })
        })
        .collect();
    let inputs = (0..steps.len())
        .map(|index| MergeInput {
            step: StepId(index),
            blend: BlendMode::Normal,
            opacity: 1.0,
        })
        .collect();
    steps.push(Step::Merge(inputs));
    let root = Some(StepId(steps.len() - 1));
    Program::new(
        NodeId(0),
        Vector::new(297.0, 210.0),
        LinearRgba::new(1.0, 1.0, 1.0, 1.0),
        steps,
        root,
    )
}

/// 500 groups, 25 to a row 12 units apart, each a square and a smaller one blended over it in
/// one of the sixteen modes other than Normal, at 80%, every group at 90% over the page.
fn blended_groups() -> Program {
    const MODES: [BlendMode; 16] = [
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::ColorDodge,
        BlendMode::ColorBurn,
        BlendMode::HardLight,
        BlendMode::SoftLight,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Color,
        BlendMode::Luminosity,
        BlendMode::Erase,
    ];
    let mut steps = Vec::new();
    let mut push = |step: Step| {
        steps.push(step);
        StepId(steps.len() - 1)
    };
    let square = |x: f64, y: f64, side: f64| {
        Arc::new(
            Path::rectangle(Rect::new(Point::new(x, y), Point::new(x + side, side + y))).contours(),
        )
    };
    let draw = |outline: Arc<Contours>, color: LinearRgba| {
        Step::Draw(Draw {
            object: None,
            outline,
            rule: FillRule::NonZero,
            color,
            opacity: 1.0,
        })
    };
    let input = |step: StepId, blend: BlendMode, opacity: f64| MergeInput {
        step,
        blend,
        opacity,
    };
    let mut groups = Vec::new();
    for i in 0..500_u32 {
        let (x, y) = (f64::from(i % 25) * 12.0, f64::from(i / 25) * 12.0);
        #[allow(clippy::cast_precision_loss, reason = "a small index")]
        let shade = (i % 9) as f32 / 9.0;
        let below = push(draw(
            square(x, y, 10.0),
            LinearRgba::new(shade, 0.4, 0.8, 1.0),
        ));
        let above = push(draw(
            square(x + 3.0, y + 3.0, 6.0),
            LinearRgba::new(0.9, shade, 0.2, 0.9),
        ));
        let mode = MODES[i as usize % MODES.len()];
        let group = push(Step::Merge(vec![
            input(below, BlendMode::Normal, 1.0),
            input(above, mode, 0.8),
        ]));
        groups.push(input(group, BlendMode::Normal, 0.9));
    }
    let root = push(Step::Merge(groups));
    Program::new(
        NodeId(0),
        Vector::new(300.0, 240.0),
        LinearRgba::new(1.0, 1.0, 1.0, 1.0),
        steps,
        Some(root),
    )
}

#[test]
#[ignore = "benchmark: cargo xtask bench"]
#[allow(
    clippy::too_many_lines,
    reason = "one table of measures over the same documents, in order"
)]
fn baseline() {
    println!("BENCH-BEGIN");
    println!("| Canvas | Measure | Median (µs) | p95 (µs) | Max (µs) | Output |");
    println!("| --- | --- | ---: | ---: | ---: | --- |");
    let Some(gpu) = koebi_testkit::gpu_or_skip("canvas benchmark") else {
        println!("| 10 000 paths | not measured: no GPU | | | | |");
        println!("BENCH-END");
        return;
    };
    let (mut doc, objects) = document();
    let mut evaluator = Evaluator::new();
    let mut program = Arc::clone(&evaluator.programs(&doc)[0]);
    let whole = view(Point::new(150.0, 150.0), 3.375);
    let name = "10 000 paths";

    // From nothing: every cell prepared and uploaded.
    let mut cells = 0;
    let cold = measure(|| {
        let mut canvas = Canvas::new(&gpu.device).expect("a canvas");
        let mut target = ViewTarget::new(&gpu.device, whole.extent).expect("a target");
        render(&gpu, &mut canvas, &mut target, &program, &whole);
        cells = canvas.stats().cells;
    });
    row(
        name,
        "first render, every cell prepared",
        cold,
        &format!("{cells} cells"),
    );

    let mut canvas = Canvas::new(&gpu.device).expect("a canvas");
    let mut target = ViewTarget::new(&gpu.device, whole.extent).expect("a target");
    render(&gpu, &mut canvas, &mut target, &program, &whole);

    // Still: nothing recorded (the CPU alone).
    let still: Vec<Duration> = gpu
        .device
        .submit_and_wait(|encoder| {
            (0..=RUNS)
                .map(|_| {
                    let start = Instant::now();
                    let drawn = canvas.render(encoder, &mut target, &program, &whole);
                    assert!(!drawn.expect("rendered"));
                    start.elapsed()
                })
                .skip(1)
                .collect()
        })
        .expect("submitted");
    row(name, "still view, nothing recorded (CPU)", stats(still), "");

    // Panned by 7 pixels back and forth: drawn again from the cells kept.
    canvas.reset_stats();
    let mut pan = 0.0;
    let panned = measure(|| {
        pan = 7.0 / whole.zoom - pan;
        let view = view(Point::new(150.0 + pan, 150.0), whole.zoom);
        render(&gpu, &mut canvas, &mut target, &program, &view);
    });
    row(
        name,
        "pan by 7 px, cells kept",
        panned,
        &format!("{} outlines prepared", canvas.stats().prepared),
    );

    // A zoom sweep over three octaves, in and out: each octave crossed prepares the outlines
    // in sight at it; the way back finds two octaves kept.
    canvas.reset_stats();
    let steps: Vec<f64> = (0..=24)
        .chain((0..24).rev())
        .map(|step| whole.zoom * (f64::from(step) / 8.0).exp2())
        .collect();
    let sweep: Vec<Duration> = steps
        .iter()
        .map(|&zoom| {
            let start = Instant::now();
            render(
                &gpu,
                &mut canvas,
                &mut target,
                &program,
                &view(Point::new(150.0, 150.0), zoom),
            );
            start.elapsed()
        })
        .collect();
    row(
        name,
        "zoom sweep, 49 steps over 3 octaves and back",
        stats(sweep),
        &format!("{} outlines prepared", canvas.stats().prepared),
    );
    render(&gpu, &mut canvas, &mut target, &program, &whole);

    // One object moved, then one fill recolored: evaluated, then drawn.
    let paint = {
        let id = doc
            .graph()
            .links_into(SocketRef::new(objects[5000], path_object::PAINTS.index()))[0];
        doc.graph().link_of(id).expect("a paint link").from.node
    };
    canvas.reset_stats();
    let mut x = 0.0;
    let moved = measure(|| {
        x = 1.0 - x;
        doc.apply(Edit::SetInput {
            node: objects[5000],
            index: path_object::LOCATION.index(),
            value: Value::Vector(Vector::new(x, 150.0)),
        })
        .expect("a location");
        program = Arc::clone(&evaluator.programs(&doc)[0]);
        render(&gpu, &mut canvas, &mut target, &program, &whole);
    });
    row(
        name,
        "one object moved: evaluated and drawn",
        moved,
        &format!("{} outlines prepared", canvas.stats().prepared),
    );
    canvas.reset_stats();
    let mut level = 0.0_f32;
    let recolored = measure(|| {
        level = 1.0 - level;
        doc.apply(Edit::SetInput {
            node: paint,
            index: fill::COLOR.index(),
            value: Value::Color(LinearRgba::new(level, 0.0, 0.0, 1.0)),
        })
        .expect("a color");
        program = Arc::clone(&evaluator.programs(&doc)[0]);
        render(&gpu, &mut canvas, &mut target, &program, &whole);
    });
    row(
        name,
        "one fill recolored: evaluated and drawn",
        recolored,
        &format!("{} outlines prepared", canvas.stats().prepared),
    );

    // Picks under a pointer crossing the view (the CPU alone), per pick.
    let pixels: Vec<[f64; 2]> = (0..1000_u32)
        .map(|i| {
            [
                f64::from(i * 37 % 1920) + 0.5,
                f64::from(i * 53 % 1080) + 0.5,
            ]
        })
        .collect();
    let mut hits = 0;
    target.pick(pixels[0], 2.0);
    let picks: Vec<Duration> = pixels
        .iter()
        .map(|&pixel| {
            let start = Instant::now();
            hits += usize::from(target.pick(pixel, 2.0).is_some());
            start.elapsed()
        })
        .collect();
    row(
        name,
        "pick under the pointer, 2 px of tolerance (CPU)",
        stats(picks),
        &format!("{hits} hits in 1 000"),
    );

    // Groups blended in every mode: each in a target of its own, within its area.
    let groups = blended_groups();
    let name = "500 blended groups";
    let framed = view(Point::new(150.0, 120.0), 4.0);
    let cold = measure(|| {
        let mut canvas = Canvas::new(&gpu.device).expect("a canvas");
        let mut target = ViewTarget::new(&gpu.device, framed.extent).expect("a target");
        render(&gpu, &mut canvas, &mut target, &groups, &framed);
    });
    row(
        name,
        "first render, targets made",
        cold,
        "1 000 groups, 500 blended in a mode",
    );
    let mut canvas = Canvas::new(&gpu.device).expect("a canvas");
    let mut target = ViewTarget::new(&gpu.device, framed.extent).expect("a target");
    render(&gpu, &mut canvas, &mut target, &groups, &framed);
    let mut pan = 0.0;
    let panned = measure(|| {
        pan = 7.0 / framed.zoom - pan;
        let view = view(Point::new(150.0 + pan, 120.0), framed.zoom);
        render(&gpu, &mut canvas, &mut target, &groups, &view);
    });
    row(name, "pan by 7 px, targets pooled", panned, "");
    let recording: Vec<Duration> = (0..=RUNS)
        .map(|_| {
            pan = 7.0 / framed.zoom - pan;
            let view = view(Point::new(150.0 + pan, 120.0), framed.zoom);
            gpu.device
                .submit_and_wait(|encoder| {
                    let start = Instant::now();
                    let drawn = canvas.render(encoder, &mut target, &groups, &view);
                    assert!(drawn.expect("rendered"));
                    start.elapsed()
                })
                .expect("submitted")
        })
        .skip(1)
        .collect();
    row(
        name,
        "pan by 7 px, recording alone (CPU)",
        stats(recording),
        "",
    );

    // Strokes in pixels: outlined by the canvas, at the view's zoom level.
    let strokes = hairlines();
    let name = "10 000 hairlines";
    let mut outlined = 0;
    let cold = measure(|| {
        let mut canvas = Canvas::new(&gpu.device).expect("a canvas");
        let mut target = ViewTarget::new(&gpu.device, whole.extent).expect("a target");
        render(&gpu, &mut canvas, &mut target, &strokes, &whole);
        outlined = canvas.stats().outlined;
    });
    row(
        name,
        "first render, every stroke outlined",
        cold,
        &format!("{outlined} outlined"),
    );
    let mut canvas = Canvas::new(&gpu.device).expect("a canvas");
    let mut target = ViewTarget::new(&gpu.device, whole.extent).expect("a target");
    render(&gpu, &mut canvas, &mut target, &strokes, &whole);
    canvas.reset_stats();
    // A zoom to the next level and back: outlined once at the next, then both kept.
    let mut up = false;
    let level = measure(|| {
        up = !up;
        let zoom = if up { whole.zoom * 1.25 } else { whole.zoom };
        render(
            &gpu,
            &mut canvas,
            &mut target,
            &strokes,
            &view(whole.center, zoom),
        );
    });
    row(
        name,
        "zoom to the next level and back",
        level,
        &format!("{} outlined", canvas.stats().outlined),
    );
    println!("BENCH-END");
}
