//! The document part of the performance baseline (`cargo xtask bench`): how long evaluation
//! takes, from nothing and after one edit, on the machine that runs it.
//!
//! An ignored test: `cargo xtask bench` runs it in release and gathers the table it prints
//! between `BENCH-BEGIN` and `BENCH-END`.

#![allow(
    clippy::print_stdout,
    reason = "the table goes to stdout, where cargo xtask bench reads it"
)]

use std::time::{Duration, Instant};

use koebi_core::LinearRgba;
use koebi_doc::eval::Evaluator;
use koebi_doc::kinds::{fill, path_object};
use koebi_doc::ops::{self, Shape};
use koebi_doc::{Document, Edit, NodeId, SocketRef, Value};
use koebi_geom::Vector;

/// Runs of each measure, after one to warm up.
const RUNS: usize = 11;

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
    (
        micros(times[times.len() / 2]),
        micros(times[(times.len() - 1) * 95 / 100]),
        micros(times[times.len() - 1]),
    )
}

fn row(name: &str, what: &str, (median, p95, max): (f64, f64, f64), output: &str) {
    println!("| {name} | {what} | {median:.0} | {p95:.0} | {max:.0} | {output} |");
}

#[test]
#[ignore = "benchmark: cargo xtask bench"]
fn baseline() {
    let mut doc = Document::new();
    let page = ops::add_page(&mut doc).expect("a page");
    let objects: Vec<NodeId> = (0..10_000_u32)
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
    let paint = {
        let id = doc
            .graph()
            .links_into(SocketRef::new(objects[5000], path_object::PAINTS.index()))[0];
        doc.graph().link_of(id).expect("a paint link").from.node
    };
    println!("BENCH-BEGIN");
    println!("| Document | Measure | Median (µs) | p95 (µs) | Max (µs) | Output |");
    println!("| --- | --- | ---: | ---: | ---: | --- |");
    let mut steps = 0;
    let cold = measure(|| {
        let mut evaluator = Evaluator::new();
        steps = evaluator.programs(&doc)[0].steps().len();
    });
    row(
        "10 000 objects",
        "evaluate from nothing",
        cold,
        &format!("{steps} steps"),
    );
    let mut evaluator = Evaluator::new();
    evaluator.programs(&doc);
    let unchanged = measure(|| {
        evaluator.programs(&doc);
    });
    row(
        "10 000 objects",
        "evaluate again, nothing changed",
        unchanged,
        "",
    );
    let mut level = 0.0_f32;
    let mut evaluated = 0;
    let recolor = measure(|| {
        level = 1.0 - level;
        let color = Value::Color(LinearRgba::new(level, 0.0, 0.0, 1.0));
        doc.apply(Edit::SetInput {
            node: paint,
            index: fill::COLOR.index(),
            value: color,
        })
        .expect("a color");
        evaluator.reset_stats();
        evaluator.programs(&doc);
        evaluated = evaluator.stats.evaluated.values().sum::<u64>();
    });
    row(
        "10 000 objects",
        "one fill recolored, evaluated again",
        recolor,
        &format!("{evaluated} outputs computed"),
    );
    let mut x = 0.0;
    let moved = measure(|| {
        x += 1.0;
        let location = Value::Vector(Vector::new(x, 0.0));
        doc.apply(Edit::SetInput {
            node: objects[5000],
            index: path_object::LOCATION.index(),
            value: location,
        })
        .expect("a location");
        evaluator.reset_stats();
        evaluator.programs(&doc);
        evaluated = evaluator.stats.evaluated.values().sum::<u64>();
    });
    row(
        "10 000 objects",
        "one object moved, evaluated again",
        moved,
        &format!("{evaluated} outputs computed"),
    );
    println!("BENCH-END");
}
