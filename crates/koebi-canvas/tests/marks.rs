//! Stroke repeats and marks on the GPU (ADR 0029): fence ticks fused into a translucent stroke
//! paint once where they cross it, a gap cuts the stroke open and shows the page, an object
//! blended in front of the stroke shows over it. Every image matches the CPU reference.

mod common;

use std::sync::Arc;

use common::{Session, assert_matches};
use koebi_canvas::{View, reference};
use koebi_core::{Extent, LinearRgba};
use koebi_doc::eval::{Evaluator, Program};
use koebi_doc::instanced::StampMode;
use koebi_doc::kinds::{fill, path_object, stroke};
use koebi_doc::marks::{MarkObject, MarkShape, StrokeMark, StrokeRepeat, StrokeSettings};
use koebi_doc::ops::{self, Shape};
use koebi_doc::{Document, Edit, NodeId, Settings, SocketRef, Value};
use koebi_geom::{Path, Point, Rect, Vector};
use koebi_testkit::gpu_or_skip;

fn set(doc: &mut Document, node: NodeId, index: usize, value: Value) {
    doc.apply(Edit::SetInput { node, index, value })
        .expect("a value of its type");
}

const INK: LinearRgba = LinearRgba::new(0.9, 0.1, 0.1, 0.5);
const OBJECT: LinearRgba = LinearRgba::new(0.1, 0.2, 0.9, 1.0);

/// A shape 40 × 20 centered at (32, 24) with no fill, stroked 2 wide in translucent red, with
/// `settings` as the stroke's repeats and marks.
fn program(settings: StrokeSettings) -> Arc<Program> {
    let mut doc = Document::new();
    let page = ops::add_page(&mut doc).expect("a page");
    let shape = ops::add_shape(
        &mut doc,
        page.stack,
        Shape::Rectangle(Vector::new(40.0, 20.0)),
    )
    .expect("a shape");
    set(
        &mut doc,
        shape,
        path_object::LOCATION.index(),
        Value::Vector(Vector::new(32.0, 24.0)),
    );
    let paint = {
        let id = doc
            .graph()
            .links_into(SocketRef::new(shape, path_object::PAINTS.index()))[0];
        doc.graph().link_of(id).expect("a paint link").from.node
    };
    set(
        &mut doc,
        paint,
        fill::COLOR.index(),
        Value::Color(LinearRgba::new(0.0, 0.0, 0.0, 0.0)),
    );
    let stroked = ops::add_stroke(&mut doc, shape).expect("a stroke");
    set(&mut doc, stroked, stroke::WIDTH.index(), Value::Float(2.0));
    set(&mut doc, stroked, stroke::COLOR.index(), Value::Color(INK));
    doc.apply(Edit::SetSettings {
        node: stroked,
        settings: Box::new(Settings::Stroke(Arc::new(settings))),
    })
    .expect("settings");
    Arc::clone(&Evaluator::new().programs(&doc)[0])
}

/// The point of the shape's outline at arc length `at`: the same rectangle, built here.
fn along(at: f64) -> Point {
    let outline =
        Path::rectangle(Rect::new(Point::new(12.0, 14.0), Point::new(52.0, 34.0))).contours();
    outline.contours()[0]
        .point_at_length(at)
        .expect("a point")
        .0
}

fn view() -> View {
    View {
        center: Point::new(32.0, 24.0),
        zoom: 3.0,
        extent: Extent::new(192, 144),
    }
}

fn at(pixels: &[[f32; 4]], view: &View, point: Point) -> [f32; 4] {
    let [px, py] = view.to_target(point);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a pixel of the view"
    )]
    let index = (py as usize) * view.extent.width as usize + px as usize;
    pixels[index]
}

#[track_caller]
fn assert_color(got: [f32; 4], want: [f32; 3], what: &str) {
    for channel in 0..3 {
        assert!(
            (got[channel] - want[channel]).abs() <= 1.5 / 255.0,
            "{what}: {got:?} against {want:?}"
        );
    }
}

/// The stroke's ink over the white page.
fn ink() -> [f32; 3] {
    let a = INK.a;
    [
        INK.r * a + (1.0 - a),
        INK.g * a + (1.0 - a),
        INK.b * a + (1.0 - a),
    ]
}

#[test]
fn ticks_fuse_gaps_cut_and_objects_show_in_front() {
    let Some(gpu) = gpu_or_skip("ticks_fuse_gaps_cut_and_objects_show_in_front") else {
        return;
    };
    let mut session = Session::new(&gpu, Extent::new(192, 144));
    let view = view();
    // Fence ticks every 8: the first at 4 along the outline, crossing the stroke.
    let ticked = program(StrokeSettings {
        repeats: vec![StrokeRepeat::default()],
        ..StrokeSettings::default()
    });
    let pixels = session.render(&ticked, &view).expect("drawn");
    assert_matches(&pixels, &reference::render(&ticked, &view), &view);
    assert_color(
        at(&pixels, &view, along(4.0)),
        ink(),
        "a tick over the stroke, once",
    );
    assert_color(
        at(&pixels, &view, along(8.0)),
        ink(),
        "the stroke between ticks",
    );
    // A gap 6 long at 12 along, and a blue circle blended in front at 30.
    let marked = program(StrokeSettings {
        marks: vec![
            StrokeMark {
                at: 0.1,
                objects: vec![MarkObject {
                    shape: MarkShape::Gap,
                    size: 6.0,
                    percent: false,
                    mode: StampMode::Subtract,
                    ..MarkObject::default()
                }],
                ..StrokeMark::default()
            },
            StrokeMark {
                at: 0.25,
                objects: vec![MarkObject {
                    size: 2.0,
                    percent: false,
                    mode: StampMode::Blend,
                    stroke_color: false,
                    color: OBJECT,
                    ..MarkObject::default()
                }],
                ..StrokeMark::default()
            },
        ],
        ..StrokeSettings::default()
    });
    let pixels = session.render(&marked, &view).expect("drawn");
    assert_matches(&pixels, &reference::render(&marked, &view), &view);
    assert_color(
        at(&pixels, &view, along(12.0)),
        [1.0, 1.0, 1.0],
        "the page in the gap",
    );
    assert_color(
        at(&pixels, &view, along(17.0)),
        ink(),
        "the stroke past the gap",
    );
    assert_color(
        at(&pixels, &view, along(30.0)),
        [OBJECT.r, OBJECT.g, OBJECT.b],
        "the object in front",
    );
    gpu.assert_no_validation_errors();
}
