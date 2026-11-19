//! Pattern fills on the GPU (ADR 0029): a motif drawn whole at every point of a lattice over a
//! shape, cut exactly at its outline, pinned to the object or to the document; picked as the
//! shape it fills; drawn from cells prepared once and moved, allocating nothing once warm.
//! Every image matches the CPU reference.

mod common;

use std::sync::Arc;

use common::{Session, assert_matches};
use koebi_canvas::{View, reference};
use koebi_core::{Extent, LinearRgba};
use koebi_doc::eval::{Evaluator, Program};
use koebi_doc::kinds::{fill, path_object, pattern_fill};
use koebi_doc::ops::{self, Shape};
use koebi_doc::{
    Document, Edit, NodeId, PatternAnchor, PatternClip, SocketRef, SocketValue, Value,
};
use koebi_geom::{Point, Vector};
use koebi_testkit::gpu_or_skip;

#[global_allocator]
static ALLOCATOR: koebi_testkit::CountingAllocator = koebi_testkit::CountingAllocator;

fn set(doc: &mut Document, node: NodeId, index: usize, value: Value) {
    doc.apply(Edit::SetInput { node, index, value })
        .expect("a value of its type");
}

/// The fill paint of `object`.
fn fill_of(doc: &Document, object: NodeId) -> NodeId {
    let id = doc
        .graph()
        .links_into(SocketRef::new(object, path_object::PAINTS.index()))[0];
    doc.graph().link_of(id).expect("a paint link").from.node
}

const HOST: LinearRgba = LinearRgba::new(0.9, 0.9, 0.7, 1.0);
const MOTIF: LinearRgba = LinearRgba::new(0.8, 0.1, 0.1, 1.0);

/// A shape 60 × 40 centered at (40, 30), filled pale, with a pattern of 4 × 4 red squares every
/// 10 units cut at its outline; the motif itself lies far out of sight.
fn document() -> (Document, NodeId, NodeId) {
    let mut doc = Document::new();
    let page = ops::add_page(&mut doc).expect("a page");
    let host = ops::add_shape(
        &mut doc,
        page.stack,
        Shape::Rectangle(Vector::new(60.0, 40.0)),
    )
    .expect("a shape");
    set(
        &mut doc,
        host,
        path_object::LOCATION.index(),
        Value::Vector(Vector::new(40.0, 30.0)),
    );
    let paint = fill_of(&doc, host);
    set(&mut doc, paint, fill::COLOR.index(), Value::Color(HOST));
    let motif = ops::add_shape(
        &mut doc,
        page.stack,
        Shape::Rectangle(Vector::new(4.0, 4.0)),
    )
    .expect("a shape");
    set(
        &mut doc,
        motif,
        path_object::LOCATION.index(),
        Value::Vector(Vector::new(900.0, 900.0)),
    );
    let paint = fill_of(&doc, motif);
    set(&mut doc, paint, fill::COLOR.index(), Value::Color(MOTIF));
    let pattern = ops::add_pattern_fill(&mut doc, host, motif).expect("a pattern");
    set(
        &mut doc,
        pattern,
        pattern_fill::SPACING.index(),
        Value::Vector(Vector::new(10.0, 10.0)),
    );
    set(
        &mut doc,
        pattern,
        pattern_fill::CLIP.index(),
        PatternClip::Contour.into_value(),
    );
    (doc, host, pattern)
}

fn program_of(evaluator: &mut Evaluator, doc: &Document) -> Arc<Program> {
    Arc::clone(&evaluator.programs(doc)[0])
}

fn view() -> View {
    View {
        center: Point::new(40.0, 30.0),
        zoom: 2.0,
        extent: Extent::new(180, 140),
    }
}

/// The pixel at document point (`x`, `y`), made straight.
fn at(pixels: &[[f32; 4]], view: &View, x: f64, y: f64) -> [f32; 4] {
    let [px, py] = view.to_target(Point::new(x, y));
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a pixel of the view"
    )]
    let index = (py as usize) * view.extent.width as usize + px as usize;
    pixels[index]
}

#[track_caller]
fn assert_paint(got: [f32; 4], want: LinearRgba, what: &str) {
    let want = [want.r, want.g, want.b, want.a];
    for channel in 0..4 {
        assert!(
            (got[channel] - want[channel]).abs() <= 1.0 / 255.0,
            "{what}: {got:?} against {want:?}"
        );
    }
}

/// The page, white, around the shape.
const PAGE: LinearRgba = LinearRgba::new(1.0, 1.0, 1.0, 1.0);

#[test]
fn a_pattern_draws_its_motif_on_its_lattice_cut_at_the_outline() {
    let Some(gpu) = gpu_or_skip("a_pattern_draws_its_motif_on_its_lattice_cut_at_the_outline")
    else {
        return;
    };
    let (mut doc, host, pattern) = document();
    let mut evaluator = Evaluator::new();
    let program = program_of(&mut evaluator, &doc);
    let mut session = Session::new(&gpu, Extent::new(180, 140));
    let view = view();
    let pixels = session.render(&program, &view).expect("drawn");
    assert_matches(&pixels, &reference::render(&program, &view), &view);
    // A motif at every point of the lattice, from the shape's origin; the shape between them.
    assert_paint(
        at(&pixels, &view, 40.25, 30.25),
        MOTIF,
        "the motif at the origin",
    );
    assert_paint(
        at(&pixels, &view, 60.25, 20.25),
        MOTIF,
        "a motif further on",
    );
    assert_paint(
        at(&pixels, &view, 45.25, 35.25),
        HOST,
        "the shape between motifs",
    );
    // Cut at the outline: the motif on the left edge shows inside only.
    assert_paint(at(&pixels, &view, 11.25, 30.25), MOTIF, "inside the edge");
    assert_paint(
        at(&pixels, &view, 8.75, 30.25),
        PAGE,
        "the page outside the edge",
    );
    // Picked as the shape it fills.
    assert_eq!(
        session
            .target
            .pick(view.to_target(Point::new(40.25, 30.25)), 0.0),
        Some(host)
    );
    // Moved by 3: pinned to the object, the motifs follow; pinned to the document, they stay.
    set(
        &mut doc,
        host,
        path_object::LOCATION.index(),
        Value::Vector(Vector::new(43.0, 30.0)),
    );
    let moved = program_of(&mut evaluator, &doc);
    let pixels = session.render(&moved, &view).expect("drawn");
    assert_matches(&pixels, &reference::render(&moved, &view), &view);
    assert_paint(
        at(&pixels, &view, 43.25, 30.25),
        MOTIF,
        "the motif followed",
    );
    assert_paint(at(&pixels, &view, 40.25, 30.25), HOST, "the old place");
    set(
        &mut doc,
        pattern,
        pattern_fill::ANCHOR.index(),
        PatternAnchor::Document.into_value(),
    );
    let pinned = program_of(&mut evaluator, &doc);
    let pixels = session.render(&pinned, &view).expect("drawn");
    assert_matches(&pixels, &reference::render(&pinned, &view), &view);
    // A motif spans 4 units centered on its point: 38 to 42 here.
    assert_paint(at(&pixels, &view, 40.25, 30.25), MOTIF, "the motif stayed");
    assert_paint(at(&pixels, &view, 41.75, 30.25), MOTIF, "within its extent");
    assert_paint(at(&pixels, &view, 42.75, 30.25), HOST, "past it");
    gpu.assert_no_validation_errors();
}

#[test]
fn a_turned_pattern_matches_the_reference_and_pans_allocate_nothing() {
    let Some(gpu) = gpu_or_skip("a_turned_pattern_matches_the_reference_and_pans_allocate_nothing")
    else {
        return;
    };
    let (mut doc, _, pattern) = document();
    set(
        &mut doc,
        pattern,
        pattern_fill::ROTATION.index(),
        Value::Float(0.4),
    );
    set(
        &mut doc,
        pattern,
        pattern_fill::MOTIF_ROTATION.index(),
        Value::Float(0.3),
    );
    set(
        &mut doc,
        pattern,
        pattern_fill::SCALE.index(),
        Value::Float(1.5),
    );
    let mut evaluator = Evaluator::new();
    let program = program_of(&mut evaluator, &doc);
    let mut session = Session::new(&gpu, Extent::new(180, 140));
    let pans: Vec<View> = (0..6)
        .map(|i| View {
            center: Point::new(40.0 + f64::from(i) * 0.75, 30.0),
            ..view()
        })
        .collect();
    for pan in &pans {
        let pixels = session.render(&program, pan).expect("drawn");
        assert_matches(&pixels, &reference::render(&program, pan), pan);
    }
    let mut allocated = Vec::new();
    for pan in &pans {
        let (canvas, target) = (&mut session.canvas, &mut session.target);
        let (count, drawn) = gpu
            .device
            .submit_and_wait(|encoder| {
                koebi_testkit::allocations(|| canvas.render(encoder, target, &program, pan))
            })
            .expect("submitted");
        assert!(drawn.expect("rendered"));
        allocated.push(count);
    }
    assert!(
        allocated.iter().all(|count| *count == 0),
        "pans allocated {allocated:?} times"
    );
    // The motif's cells are prepared once, in its own space, whatever the points.
    assert!(session.canvas.stats().prepared <= 3);
    gpu.assert_no_validation_errors();
}
