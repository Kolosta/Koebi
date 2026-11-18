//! Strokes whose lengths count a view's pixels (ADR 0029): outlined by the canvas at each
//! view's zoom level, they keep their width on screen at any zoom, within 14%; a zoom hovering
//! at a level's edge outlines nothing again; strokes out of sight are not outlined. Images drawn
//! at the level a new view takes match the CPU reference.

mod common;

use std::sync::Arc;

use common::{Session, assert_matches, circle, color};
use koebi_canvas::{View, reference};
use koebi_core::{Extent, LinearRgba};
use koebi_doc::eval::{Draw, MergeInput, Program, Step, StepId, ViewStroke};
use koebi_doc::{BlendMode, NodeId};
use koebi_geom::{Contour, Contours, FillRule, Point, Segment, StrokeStyle, Vector};
use koebi_testkit::gpu_or_skip;

/// A program drawing `steps` bottom first, on a transparent page.
fn program(mut steps: Vec<Step>) -> Program {
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
        Vector::new(100.0, 100.0),
        LinearRgba::new(0.0, 0.0, 0.0, 0.0),
        steps,
        root,
    )
}

/// `geometry` stroked `width` pixels wide in `paint`.
fn stroke(geometry: &Arc<Contours>, width: f64, paint: LinearRgba) -> Step {
    Step::ViewStroke(ViewStroke {
        geometry: Arc::clone(geometry),
        style: Arc::new(StrokeStyle {
            width,
            ..StrokeStyle::default()
        }),
        color: paint,
        opacity: 1.0,
    })
}

/// An open line from `a` to `b`.
fn line(a: Point, b: Point) -> Arc<Contours> {
    let mut contours = Contours::default();
    contours.push(Contour::new(vec![Segment::line(a, b)], false));
    Arc::new(contours)
}

fn view(zoom: f64) -> View {
    View {
        center: Point::new(0.3, 0.2),
        zoom,
        extent: Extent::new(64, 48),
    }
}

/// The coverage summed down the middle column: the width on screen of a horizontal stroke
/// crossing it.
fn width_down_the_middle(pixels: &[[f32; 4]], view: &View) -> f32 {
    let (width, height) = (view.extent.width as usize, view.extent.height as usize);
    (0..height)
        .map(|row| pixels[row * width + width / 2][3])
        .sum()
}

#[test]
fn a_stroke_in_pixels_keeps_its_width_on_screen_at_any_zoom() {
    let Some(gpu) = gpu_or_skip("a_stroke_in_pixels_keeps_its_width_on_screen_at_any_zoom") else {
        return;
    };
    let mut session = Session::new(&gpu, Extent::new(64, 48));
    let across = line(Point::new(-1.0e4, 0.2), Point::new(1.0e4, 0.2));
    for width in [1.0, 3.0] {
        let program = program(vec![stroke(&across, width, color(1.0, 1.0, 1.0, 1.0))]);
        for zoom in [1.0e-3, 0.37, 1.0, 13.0, 4096.0] {
            let view = view(zoom);
            let pixels = session.render(&program, &view).expect("drawn");
            assert_matches(&pixels, &reference::render(&program, &view), &view);
            #[allow(clippy::cast_possible_truncation, reason = "a small width")]
            let asked = width as f32;
            let shown = width_down_the_middle(&pixels, &view);
            assert!(
                (0.86 * asked..1.15 * asked).contains(&shown),
                "{width} px at a zoom of {zoom}: {shown} px"
            );
        }
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn a_zoom_hovering_at_a_level_edge_outlines_nothing_again() {
    let Some(gpu) = gpu_or_skip("a_zoom_hovering_at_a_level_edge_outlines_nothing_again") else {
        return;
    };
    let mut session = Session::new(&gpu, Extent::new(64, 48));
    let ring = Arc::new(circle(Point::new(0.3, 0.2), 10.0));
    let fill = Step::Draw(Draw {
        outline: Arc::clone(&ring),
        rule: FillRule::NonZero,
        color: color(0.2, 0.4, 0.9, 0.5),
        opacity: 1.0,
    });
    let program = program(vec![fill, stroke(&ring, 1.0, color(0.0, 0.0, 0.0, 1.0))]);
    let outlined = |session: &mut Session<'_>| {
        let outlined = session.canvas.stats().outlined;
        session.canvas.reset_stats();
        outlined
    };
    // The top of level 4 of a quarter octave: a zoom of 2^(5/4). The reference starts from no
    // level, as a new view does: it matches the first image only, the others keep the level.
    let edge = 1.25_f64.exp2();
    let first = view(edge * 0.99);
    let pixels = session.render(&program, &first).expect("drawn");
    assert_matches(&pixels, &reference::render(&program, &first), &first);
    for zoom in [edge * 1.01, edge * 0.99, edge * 1.02, edge * 0.995] {
        session.render(&program, &view(zoom)).expect("drawn");
    }
    assert_eq!(outlined(&mut session), 1);
    // Past the edge by more than a sixteenth of an octave: the next level, then both kept.
    for zoom in [edge * 1.1, edge * 0.99, edge * 1.1] {
        session.render(&program, &view(zoom)).expect("drawn");
    }
    assert_eq!(outlined(&mut session), 1);
    gpu.assert_no_validation_errors();
}

#[test]
fn strokes_out_of_sight_are_not_outlined() {
    let Some(gpu) = gpu_or_skip("strokes_out_of_sight_are_not_outlined") else {
        return;
    };
    let mut session = Session::new(&gpu, Extent::new(64, 48));
    // Twenty rings 30 units apart in a row; a view 64 units wide sees two or three.
    let steps = (0..20)
        .map(|i| {
            let ring = Arc::new(circle(Point::new(30.0 * f64::from(i), 0.0), 5.0));
            stroke(&ring, 2.0, color(0.9, 0.2, 0.1, 1.0))
        })
        .collect();
    let program = program(steps);
    let near = View {
        center: Point::new(15.0, 0.0),
        zoom: 1.0,
        extent: Extent::new(64, 48),
    };
    let pixels = session.render(&program, &near).expect("drawn");
    assert_matches(&pixels, &reference::render(&program, &near), &near);
    let stats = session.canvas.stats();
    assert_eq!(stats.outlined, 2);
    assert_eq!(stats.prepared, 2);
    gpu.assert_no_validation_errors();
}
