//! The canvas on the GPU against its CPU reference ([`koebi_canvas::reference`]): curves, both
//! fill rules, thin shapes, outlines cut by the view and by cells, translucent layers over each
//! other, within 1/255; the same edge at zooms of 10⁻⁶ and 10⁹; a golden image.

mod common;

use std::sync::Arc;

use common::{assert_matches, circle, color, polyline, program, program_of, render, star};
use koebi_canvas::{View, reference};
use koebi_core::Extent;
use koebi_geom::{Affine, Contours, FillRule, Path, Point, Rect, Vector};
use koebi_testkit::{Image, Tolerance, assert_golden, gpu_or_skip};

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
    let page = program_of(
        Vector::new(200.0, 140.0),
        color(1.0, 1.0, 1.0, 1.0),
        &[
            (
                Arc::new(circle(Point::new(70.0, 70.0), 50.0)),
                FillRule::NonZero,
                color(0.9, 0.3, 0.1, 1.0),
            ),
            (
                Arc::new(star(Point::new(125.0, 70.0), 60.0)),
                FillRule::EvenOdd,
                color(0.1, 0.4, 0.9, 0.6),
            ),
            (
                Arc::new(
                    Path::rectangle(Rect::new(Point::new(20.0, 100.0), Point::new(180.0, 112.0)))
                        .contours()
                        .transformed(Affine::rotate(-0.12)),
                ),
                FillRule::NonZero,
                color(0.2, 0.8, 0.3, 0.8),
            ),
        ],
    );
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
