//! Instanced fills on the GPU (ADR 0029): translucent stamps fused paint once where they
//! overlap, blended ones darken there, subtracted ones erase the fill alone and show the shape
//! below it. Every image matches the CPU reference.

mod common;

use std::sync::Arc;

use common::{Session, assert_matches};
use koebi_canvas::{View, reference};
use koebi_core::{Extent, LinearRgba};
use koebi_doc::eval::{Evaluator, Program};
use koebi_doc::instanced::{InstancedElement, InstancedSettings, StampMode};
use koebi_doc::kinds::{fill, instanced_fill, path_object};
use koebi_doc::ops::{self, Shape};
use koebi_doc::{Document, Edit, NodeId, Settings, SocketRef, Value};
use koebi_geom::{Point, Vector};
use koebi_testkit::gpu_or_skip;

fn set(doc: &mut Document, node: NodeId, index: usize, value: Value) {
    doc.apply(Edit::SetInput { node, index, value })
        .expect("a value of its type");
}

const HOST: LinearRgba = LinearRgba::new(0.6, 0.7, 0.9, 1.0);
const STAMP: LinearRgba = LinearRgba::new(0.9, 0.1, 0.1, 0.5);

/// A shape 40 × 30 centered at (32, 24), and an instanced fill over it: circles of radius 4
/// every 6 units, overlapping their neighbors, in `elements`' modes.
fn program(elements: Vec<InstancedElement>) -> Arc<Program> {
    let mut doc = Document::new();
    let page = ops::add_page(&mut doc).expect("a page");
    let host = ops::add_shape(
        &mut doc,
        page.stack,
        Shape::Rectangle(Vector::new(40.0, 30.0)),
    )
    .expect("a shape");
    set(
        &mut doc,
        host,
        path_object::LOCATION.index(),
        Value::Vector(Vector::new(32.0, 24.0)),
    );
    let paint = {
        let id = doc
            .graph()
            .links_into(SocketRef::new(host, path_object::PAINTS.index()))[0];
        doc.graph().link_of(id).expect("a paint link").from.node
    };
    set(&mut doc, paint, fill::COLOR.index(), Value::Color(HOST));
    let instanced = ops::add_instanced_fill(&mut doc, host).expect("a fill");
    set(
        &mut doc,
        instanced,
        instanced_fill::SPACING.index(),
        Value::Vector(Vector::new(6.0, 6.0)),
    );
    doc.apply(Edit::SetSettings {
        node: instanced,
        settings: Box::new(Settings::Instanced(Arc::new(InstancedSettings {
            elements,
            lines: Vec::new(),
        }))),
    })
    .expect("settings");
    Arc::clone(&Evaluator::new().programs(&doc)[0])
}

fn circle(radius: f64, mode: StampMode) -> InstancedElement {
    InstancedElement {
        size: [radius; 3],
        mode,
        fill_color: false,
        color: STAMP,
        ..InstancedElement::default()
    }
}

fn view() -> View {
    View {
        center: Point::new(32.0, 24.0),
        zoom: 3.0,
        extent: Extent::new(192, 144),
    }
}

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

/// `source` (straight) over `backdrop` (opaque), premultiplied: opaque.
fn over(backdrop: [f32; 3], source: LinearRgba) -> [f32; 3] {
    let a = source.a;
    [
        source.r * a + backdrop[0] * (1.0 - a),
        source.g * a + backdrop[1] * (1.0 - a),
        source.b * a + backdrop[2] * (1.0 - a),
    ]
}

#[track_caller]
fn assert_color(got: [f32; 4], want: [f32; 3], what: &str) {
    for channel in 0..3 {
        assert!(
            (got[channel] - want[channel]).abs() <= 1.5 / 255.0,
            "{what}: {got:?} against {want:?}"
        );
    }
    assert!((got[3] - 1.0).abs() <= 1.0 / 255.0, "{what}: {got:?}");
}

#[test]
fn fused_stamps_paint_once_blended_ones_darken_subtracted_ones_erase() {
    let Some(gpu) =
        gpu_or_skip("fused_stamps_paint_once_blended_ones_darken_subtracted_ones_erase")
    else {
        return;
    };
    let mut session = Session::new(&gpu, Extent::new(192, 144));
    let view = view();
    let host = [HOST.r, HOST.g, HOST.b];
    // Halfway between two stamps 6 apart, both cover: (35, 24) lies between (32, 24) and
    // (38, 24), lattice points from the shape's origin.
    let (on, between) = ((32.0, 24.0), (35.0, 24.25));
    let fused = program(vec![circle(4.0, StampMode::Fusion)]);
    let pixels = session.render(&fused, &view).expect("drawn");
    assert_matches(&pixels, &reference::render(&fused, &view), &view);
    let once = over(host, STAMP);
    assert_color(at(&pixels, &view, on.0, on.1), once, "a stamp");
    assert_color(
        at(&pixels, &view, between.0, between.1),
        once,
        "fused, painted once",
    );
    let blended = program(vec![circle(4.0, StampMode::Blend)]);
    let pixels = session.render(&blended, &view).expect("drawn");
    assert_matches(&pixels, &reference::render(&blended, &view), &view);
    assert_color(
        at(&pixels, &view, between.0, between.1),
        over(once, STAMP),
        "blended, painted twice",
    );
    // A smaller circle subtracted over the fused ones: the fill erased there, the shape shows.
    // Erase goes as far as its source's alpha: an opaque one erases all.
    let holed = program(vec![
        circle(4.0, StampMode::Fusion),
        InstancedElement {
            color: LinearRgba::new(0.0, 0.0, 0.0, 1.0),
            ..circle(1.5, StampMode::Subtract)
        },
    ]);
    let pixels = session.render(&holed, &view).expect("drawn");
    assert_matches(&pixels, &reference::render(&holed, &view), &view);
    assert_color(
        at(&pixels, &view, on.0, on.1),
        host,
        "the shape through the hole",
    );
    assert_color(
        at(&pixels, &view, between.0, between.1),
        once,
        "around the hole",
    );
    gpu.assert_no_validation_errors();
}
