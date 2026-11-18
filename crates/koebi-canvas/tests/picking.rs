//! Picking on the CPU (ADR 0029): a view finds the topmost object it drew under a point,
//! exactly, through an index of the bounds of what it drew. An instance is picked as itself, a
//! hairline within a tolerance, and picks allocate nothing once the index has grown.

mod common;

use std::sync::Arc;

use common::Session;
use koebi_canvas::View;
use koebi_core::Extent;
use koebi_doc::eval::{Evaluator, Program};
use koebi_doc::kinds::{path_object, stroke};
use koebi_doc::ops::{self, Shape};
use koebi_doc::{Document, Edit, NodeId, SocketValue, StrokeUnits, Value};
use koebi_geom::{Point, Vector};
use koebi_testkit::gpu_or_skip;

#[global_allocator]
static ALLOCATOR: koebi_testkit::CountingAllocator = koebi_testkit::CountingAllocator;

fn set(doc: &mut Document, node: NodeId, index: usize, value: Value) {
    doc.apply(Edit::SetInput { node, index, value })
        .expect("a value of its type");
}

fn place(doc: &mut Document, object: NodeId, x: f64, y: f64) {
    set(
        doc,
        object,
        path_object::LOCATION.index(),
        Value::Vector(Vector::new(x, y)),
    );
}

/// A rectangle 30 × 20 at (20, 20); a disc of radius 8 over its right side at (32, 20); a
/// linked duplicate of the rectangle at (80, 20); a sliver 40 units long and a hundredth wide
/// at (60, 50), stroked 2 pixels wide.
fn document() -> (Document, [NodeId; 4]) {
    let mut doc = Document::new();
    let page = ops::add_page(&mut doc).expect("a page");
    let rectangle = ops::add_shape(
        &mut doc,
        page.stack,
        Shape::Rectangle(Vector::new(30.0, 20.0)),
    )
    .expect("a shape");
    place(&mut doc, rectangle, 20.0, 20.0);
    let disc = ops::add_shape(&mut doc, page.stack, Shape::Ellipse(Vector::new(8.0, 8.0)))
        .expect("a shape");
    place(&mut doc, disc, 32.0, 20.0);
    let instance = ops::duplicate_linked(&mut doc, rectangle).expect("an instance");
    set(
        &mut doc,
        instance,
        koebi_doc::kinds::instance_object::LOCATION.index(),
        Value::Vector(Vector::new(80.0, 20.0)),
    );
    let sliver = ops::add_shape(
        &mut doc,
        page.stack,
        Shape::Rectangle(Vector::new(40.0, 0.01)),
    )
    .expect("a shape");
    place(&mut doc, sliver, 60.0, 50.0);
    let hairline = ops::add_stroke(&mut doc, sliver).expect("a stroke");
    set(&mut doc, hairline, stroke::WIDTH.index(), Value::Float(2.0));
    set(
        &mut doc,
        hairline,
        stroke::UNITS.index(),
        StrokeUnits::Viewport.into_value(),
    );
    (doc, [rectangle, disc, instance, sliver])
}

fn program_of(doc: &Document) -> Arc<Program> {
    Arc::clone(&Evaluator::new().programs(doc)[0])
}

#[test]
fn a_view_picks_the_topmost_object_it_drew() {
    let Some(gpu) = gpu_or_skip("a_view_picks_the_topmost_object_it_drew") else {
        return;
    };
    let (doc, [rectangle, disc, instance, sliver]) = document();
    let program = program_of(&doc);
    let mut session = Session::new(&gpu, Extent::new(240, 160));
    assert_eq!(
        session.target.pick([10.0, 10.0], 0.0),
        None,
        "nothing drawn yet"
    );
    let mut view = View {
        center: Point::new(50.0, 35.0),
        zoom: 2.0,
        extent: Extent::new(240, 160),
    };
    for pan in [0.0, 13.5] {
        view.center.x = 50.0 + pan;
        session.render(&program, &view).expect("drawn");
        let mut pick = |x: f64, y: f64, tolerance: f64| {
            session
                .target
                .pick(view.to_target(Point::new(x, y)), tolerance)
        };
        // The disc lies over the rectangle; the rectangle alone, beside it.
        assert_eq!(pick(30.0, 20.0, 0.0), Some(disc));
        assert_eq!(pick(10.0, 15.0, 0.0), Some(rectangle));
        // The instance is picked as itself, not as the rectangle it shows.
        assert_eq!(pick(85.0, 22.0, 0.0), Some(instance));
        // Between them, nothing. A unit left of the rectangle's left edge: missed, but found
        // within a tolerance of four pixels, two units at this zoom.
        assert_eq!(pick(55.0, 20.0, 0.0), None);
        assert_eq!(pick(4.0, 25.0, 0.0), None);
        assert_eq!(pick(4.0, 25.0, 4.0), Some(rectangle));
        // The sliver's hairline: hit on it, missed a pixel and a half away, found within a
        // tolerance of three.
        assert_eq!(pick(60.0, 50.0, 0.0), Some(sliver));
        assert_eq!(pick(60.0, 50.75, 0.0), None);
        assert_eq!(pick(60.0, 50.75, 3.0), Some(sliver));
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn picks_allocate_nothing_once_the_index_has_grown() {
    let Some(gpu) = gpu_or_skip("picks_allocate_nothing_once_the_index_has_grown") else {
        return;
    };
    let (doc, _) = document();
    let program = program_of(&doc);
    let mut session = Session::new(&gpu, Extent::new(240, 160));
    let view = View {
        center: Point::new(50.0, 35.0),
        zoom: 2.0,
        extent: Extent::new(240, 160),
    };
    session.render(&program, &view).expect("drawn");
    let sweep = |session: &mut Session<'_>| {
        let mut hits = 0;
        for row in 0..40 {
            for column in 0..60 {
                let pixel = [f64::from(column) * 4.0 + 0.5, f64::from(row) * 4.0 + 0.5];
                hits += usize::from(session.target.pick(pixel, 2.0).is_some());
            }
        }
        hits
    };
    let warm = sweep(&mut session);
    assert!(warm > 0);
    let (count, hits) = koebi_testkit::allocations(|| sweep(&mut session));
    assert_eq!(hits, warm);
    assert_eq!(count, 0, "picks allocated {count} times");
    gpu.assert_no_validation_errors();
}
