//! The canvas on the GPU against its CPU reference ([`koebi_canvas::reference`]): curves, both
//! fill rules, thin shapes, outlines cut by the view and by cells, translucent layers over each
//! other, within 1/255; the same edge at zooms of 10⁻⁶ and 10⁹; a golden image.

use std::sync::Arc;

use koebi_canvas::{Canvas, View, ViewTarget, reference};
use koebi_core::{Extent, LinearRgba};
use koebi_doc::eval::{Draw, MergeInput, Program, Step, StepId};
use koebi_doc::{BlendMode, NodeId};
use koebi_geom::{Affine, Contour, Contours, FillRule, Path, Point, Rect, Segment, Vector};
use koebi_gpu::{BufferDesc, BufferUsage, MemoryLocation, TextureState};
use koebi_testkit::{HeadlessGpu, Image, Tolerance, assert_golden, gpu_or_skip};

/// A program drawing `layers`, bottom first, on a transparent page of `size`.
fn program(size: Vector, layers: &[(Contours, FillRule, LinearRgba)]) -> Program {
    let mut steps: Vec<Step> = layers
        .iter()
        .map(|(outline, rule, color)| {
            Step::Draw(Draw {
                outline: Arc::new(outline.clone()),
                rule: *rule,
                color: *color,
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
    Program {
        page: NodeId(0),
        size,
        background: LinearRgba::new(0.0, 0.0, 0.0, 0.0),
        root: Some(StepId(steps.len() - 1)),
        steps,
    }
}

/// The 32-bit float of the half `bits`.
fn from_half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let mantissa = f32::from(bits & 0x3ff);
    sign * match exponent {
        0 => mantissa * (-24.0_f32).exp2(),
        31 => f32::INFINITY,
        _ => {
            (1.0 + mantissa / 1024.0) * f32::from(i16::try_from(exponent).unwrap_or(0) - 15).exp2()
        }
    }
}

/// `program` seen by `view`, drawn on the GPU and read back: linear light, premultiplied.
fn render(gpu: &HeadlessGpu, program: &Program, view: &View) -> Vec<[f32; 4]> {
    let mut canvas = Canvas::new(&gpu.device).expect("a canvas");
    let target = ViewTarget::new(&gpu.device, view.extent).expect("a target");
    let pixels = view.extent.area();
    let readback = gpu
        .device
        .create_buffer(&BufferDesc {
            label: "canvas readback",
            size: pixels * 8,
            usage: BufferUsage::TRANSFER_DST,
            location: MemoryLocation::GpuToCpu,
        })
        .expect("a readback buffer");
    gpu.device
        .submit_and_wait(|encoder| {
            canvas.render(encoder, &target, program, view, 0)?;
            encoder.texture_barrier(
                target.texture(),
                TextureState::ShaderRead,
                TextureState::TransferSrc,
            );
            encoder.copy_texture_to_buffer(target.texture(), &readback)
        })
        .expect("submitted")
        .expect("recorded");
    let mut halves = vec![0_u16; usize::try_from(pixels * 4).expect("a small view")];
    readback.read(0, &mut halves).expect("read back");
    halves
        .as_chunks::<4>()
        .0
        .iter()
        .map(|texel| texel.map(from_half))
        .collect()
}

/// Asserts that the GPU's pixels are the reference's within 1/255, half precision aside.
#[track_caller]
fn assert_matches(gpu: &[[f32; 4]], cpu: &[[f32; 4]], view: &View) {
    let width = view.extent.width as usize;
    let mut worst = (0.0_f32, 0, 0);
    for (index, (a, b)) in gpu.iter().zip(cpu).enumerate() {
        for channel in 0..4 {
            let delta = (a[channel] - b[channel]).abs();
            if delta > worst.0 {
                worst = (delta, index % width, index / width);
            }
        }
    }
    // A half holds 11 bits: 1/2048 of 1 at most beside the reference's 1/255.
    assert!(
        worst.0 <= 1.0 / 255.0 + 1.0 / 2048.0,
        "pixel ({}, {}) off by {} (×255: {:.2})",
        worst.1,
        worst.2,
        worst.0,
        worst.0 * 255.0
    );
}

fn circle(center: Point, radius: f64) -> Contours {
    Path::ellipse(center, Vector::new(radius, radius)).contours()
}

/// A closed polyline through `points`.
fn polyline(points: &[Point]) -> Contours {
    let segments = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(&a, &b)| Segment::line(a, b))
        .collect();
    let mut contours = Contours::default();
    contours.push(Contour::new(segments, true));
    contours
}

/// A five-pointed star drawn in one stroke: its middle winds twice.
fn star(center: Point, radius: f64) -> Contours {
    let points: Vec<Point> = (0..5)
        .map(|k| {
            let angle =
                f64::from(k) * 4.0 * core::f64::consts::PI / 5.0 - core::f64::consts::FRAC_PI_2;
            Point::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            )
        })
        .collect();
    polyline(&points)
}

fn color(r: f32, g: f32, b: f32, a: f32) -> LinearRgba {
    LinearRgba::new(r, g, b, a)
}

#[test]
fn curves_both_fill_rules_and_thin_shapes_match_the_reference() {
    let Some(gpu) = gpu_or_skip("curves_both_fill_rules_and_thin_shapes_match_the_reference")
    else {
        return;
    };
    // A sliver a third of a pixel wide, turned.
    let sliver = polyline(&[
        Point::new(10.0, 70.0),
        Point::new(90.0, 40.0),
        Point::new(90.1, 40.3),
        Point::new(10.1, 70.3),
    ]);
    let program = program(
        Vector::new(160.0, 100.0),
        &[
            (
                circle(Point::new(40.0, 40.0), 30.3),
                FillRule::NonZero,
                color(0.8, 0.2, 0.1, 1.0),
            ),
            (
                star(Point::new(110.0, 45.0), 38.0),
                FillRule::EvenOdd,
                color(0.1, 0.6, 0.9, 0.7),
            ),
            (
                star(Point::new(120.0, 60.0), 30.0),
                FillRule::NonZero,
                color(0.9, 0.9, 0.2, 0.5),
            ),
            (sliver, FillRule::NonZero, color(1.0, 1.0, 1.0, 1.0)),
        ],
    );
    for (zoom, center) in [
        (1.0, Point::new(80.0, 50.0)),
        (1.75, Point::new(80.3, 50.2)),
        (0.6, Point::new(60.0, 40.0)),
    ] {
        let view = View {
            center,
            zoom,
            extent: Extent::new(160, 100),
        };
        assert_matches(
            &render(&gpu, &program, &view),
            &reference::render(&program, &view),
            &view,
        );
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn outlines_across_cells_and_beyond_the_view_match_the_reference() {
    let Some(gpu) = gpu_or_skip("outlines_across_cells_and_beyond_the_view_match_the_reference")
    else {
        return;
    };
    // At 4 pixels per unit, cells are 256 units, 1024 pixels: a view of 300 pixels around a
    // cell corner sees four cells, and a disc reaching far past it on every side.
    let program = program(
        Vector::new(2000.0, 2000.0),
        &[
            (
                circle(Point::new(300.0, 250.0), 120.0),
                FillRule::NonZero,
                color(0.3, 0.7, 0.2, 1.0),
            ),
            (
                circle(Point::new(256.0, 256.0), 30.0),
                FillRule::NonZero,
                color(0.9, 0.1, 0.4, 0.6),
            ),
        ],
    );
    let view = View {
        center: Point::new(256.0, 256.0),
        zoom: 4.0,
        extent: Extent::new(300, 220),
    };
    assert_matches(
        &render(&gpu, &program, &view),
        &reference::render(&program, &view),
        &view,
    );
    gpu.assert_no_validation_errors();
}

#[test]
fn the_same_edge_renders_alike_from_a_zoom_of_a_millionth_to_a_billion() {
    let Some(gpu) =
        gpu_or_skip("the_same_edge_renders_alike_from_a_zoom_of_a_millionth_to_a_billion")
    else {
        return;
    };
    // A triangle and a disc whose edges cross the view, scaled with the zoom: on screen, the
    // same edges at every zoom, away from the origin. Not too far: at a zoom of 10⁹, f64 itself
    // spaces positions near x by x × 2⁻⁵² units, a tenth of a pixel already at 7·10⁵.
    let far = Vector::new(300.0, -200.0);
    let triangle = polyline(&[
        Point::new(-40.0, -30.0),
        Point::new(55.0, -25.0),
        Point::new(-10.0, 45.0),
    ]);
    let disc = circle(Point::new(20.0, 10.0), 17.3);
    let render_at = |shape: &Contours, zoom: f64| {
        let scaled =
            shape.transformed(Affine::translate(far) * Affine::scale(1.0 / zoom, 1.0 / zoom));
        let program = program(
            Vector::new(1.0, 1.0),
            &[(scaled, FillRule::EvenOdd, color(1.0, 0.5, 0.25, 1.0))],
        );
        let view = View {
            center: far.to_point(),
            zoom,
            extent: Extent::new(128, 96),
        };
        let gpu_pixels = render(&gpu, &program, &view);
        assert_matches(&gpu_pixels, &reference::render(&program, &view), &view);
        (gpu_pixels, view)
    };
    // Straight edges are exact: the same pixels.
    let (unit, view) = render_at(&triangle, 1.0);
    for zoom in [1.0e-6, 1.0e9] {
        assert_matches(&render_at(&triangle, zoom).0, &unit, &view);
    }
    // Curves are quadratics within a sixteenth of a pixel at each octave: within that.
    let (unit, _) = render_at(&disc, 1.0);
    for zoom in [1.0e-6, 1.0e9] {
        let pixels = render_at(&disc, zoom).0;
        let worst = pixels
            .iter()
            .zip(&unit)
            .flat_map(|(a, b)| (0..4).map(move |c| (a[c] - b[c]).abs()))
            .fold(0.0_f32, f32::max);
        assert!(worst <= 1.0 / 16.0 + 1.0 / 255.0, "zoom {zoom}: {worst}");
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn a_page_with_overlapping_shapes_matches_golden() {
    let Some(gpu) = gpu_or_skip("a_page_with_overlapping_shapes_matches_golden") else {
        return;
    };
    let mut page = program(
        Vector::new(200.0, 140.0),
        &[
            (
                circle(Point::new(70.0, 70.0), 50.0),
                FillRule::NonZero,
                color(0.9, 0.3, 0.1, 1.0),
            ),
            (
                star(Point::new(125.0, 70.0), 60.0),
                FillRule::EvenOdd,
                color(0.1, 0.4, 0.9, 0.6),
            ),
            (
                Path::rectangle(Rect::new(Point::new(20.0, 100.0), Point::new(180.0, 112.0)))
                    .contours()
                    .transformed(Affine::rotate(-0.12)),
                FillRule::NonZero,
                color(0.2, 0.8, 0.3, 0.8),
            ),
        ],
    );
    page.background = color(1.0, 1.0, 1.0, 1.0);
    let view = View {
        center: Point::new(100.0, 70.0),
        zoom: 1.0,
        extent: Extent::new(220, 160),
    };
    let pixels = render(&gpu, &page, &view);
    assert_matches(&pixels, &reference::render(&page, &view), &view);
    // Encoded to sRGB over a mid gray, as a window shows it.
    let encode = |linear: f32| {
        let x = linear.clamp(0.0, 1.0);
        let encoded = if x <= 0.003_130_8 {
            12.92 * x
        } else {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        };
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a byte"
        )]
        let byte = (encoded * 255.0).round() as u8;
        byte
    };
    let bytes = pixels
        .iter()
        .flat_map(|[r, g, b, a]| {
            let under = 0.2 * (1.0 - a);
            [encode(r + under), encode(g + under), encode(b + under), 255]
        })
        .collect();
    let image = Image::from_rgba8(view.extent.width, view.extent.height, bytes);
    assert_golden!(image, "page", Tolerance::new(2, 0));
    gpu.assert_no_validation_errors();
}
