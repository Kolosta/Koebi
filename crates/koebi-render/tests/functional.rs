//! Functional tests: the renderer draws exactly what the scene describes.
//!
//! Expectations are computed analytically (pixel bounds, premultiplied blending), so they
//! hold on every conformant driver. Visual regressions of complex scenes are covered by the
//! golden-image tests in `regression.rs`.

mod common;

use common::{assert_pixel, render};
use koebi_core::{Color, Extent, Rect, Vec2};
use koebi_render::{LinePrimitive, RectPrimitive, Scene};
use koebi_testkit::gpu_or_skip;

const BACKGROUND: Color = Color::from_rgba8(16, 32, 48, 255);
const FILL: Color = Color::from_rgba8(200, 100, 50, 255);
const STROKE: Color = Color::from_rgba8(30, 220, 140, 255);

fn scene_with(rects: &[RectPrimitive]) -> Scene {
    let mut scene = Scene::new();
    scene.set_clear_color(BACKGROUND);
    for rect in rects {
        scene.push_rect(rect);
    }
    scene
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Vec2::new(x, y), Vec2::new(width, height))
}

#[test]
fn opaque_rect_covers_exactly_its_pixels() {
    let Some(gpu) = gpu_or_skip("opaque_rect_covers_exactly_its_pixels") else {
        return;
    };
    let scene = scene_with(&[RectPrimitive::new(rect(8.0, 4.0, 16.0, 8.0), FILL)]);
    let image = render(&gpu, Extent::new(32, 16), 1.0, &scene);
    for y in 0..16 {
        for x in 0..32 {
            let inside = (8..24).contains(&x) && (4..12).contains(&y);
            let expected = if inside { FILL } else { BACKGROUND };
            assert_pixel(&image, x, y, expected.to_rgba8(), 1);
        }
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn scale_factor_maps_logical_to_physical_pixels() {
    let Some(gpu) = gpu_or_skip("scale_factor_maps_logical_to_physical_pixels") else {
        return;
    };
    let scene = scene_with(&[RectPrimitive::new(rect(8.0, 4.0, 16.0, 8.0), FILL)]);
    let image = render(&gpu, Extent::new(64, 32), 2.0, &scene);
    assert_pixel(&image, 16, 8, FILL.to_rgba8(), 1);
    assert_pixel(&image, 47, 23, FILL.to_rgba8(), 1);
    assert_pixel(&image, 15, 8, BACKGROUND.to_rgba8(), 1);
    assert_pixel(&image, 48, 23, BACKGROUND.to_rgba8(), 1);
    gpu.assert_no_validation_errors();
}

#[test]
fn translucent_rects_blend_with_premultiplied_alpha() {
    let Some(gpu) = gpu_or_skip("translucent_rects_blend_with_premultiplied_alpha") else {
        return;
    };
    let mut scene = Scene::new();
    scene.set_clear_color(Color::BLACK);
    scene.push_rect(&RectPrimitive::new(
        rect(0.0, 0.0, 8.0, 8.0),
        Color::WHITE.with_alpha(0.5),
    ));
    let image = render(&gpu, Extent::new(8, 8), 1.0, &scene);
    // 1.0 * 0.5 + 0.0 * (1 - 0.5) = 0.5 → 127.5, rounded either way by the hardware.
    assert_pixel(&image, 4, 4, [128, 128, 128, 255], 1);
    gpu.assert_no_validation_errors();
}

#[test]
fn later_rects_are_drawn_on_top() {
    let Some(gpu) = gpu_or_skip("later_rects_are_drawn_on_top") else {
        return;
    };
    let top = Color::from_rgba8(10, 220, 120, 255);
    let scene = scene_with(&[
        RectPrimitive::new(rect(0.0, 0.0, 12.0, 12.0), FILL),
        RectPrimitive::new(rect(4.0, 4.0, 12.0, 12.0), top),
    ]);
    let image = render(&gpu, Extent::new(16, 16), 1.0, &scene);
    assert_pixel(&image, 2, 2, FILL.to_rgba8(), 1);
    assert_pixel(&image, 8, 8, top.to_rgba8(), 1);
    gpu.assert_no_validation_errors();
}

#[test]
fn rounded_corners_are_anti_aliased() {
    let Some(gpu) = gpu_or_skip("rounded_corners_are_anti_aliased") else {
        return;
    };
    let scene =
        scene_with(
            &[RectPrimitive::new(rect(0.0, 0.0, 32.0, 32.0), FILL).with_corner_radius(10.0)],
        );
    let image = render(&gpu, Extent::new(32, 32), 1.0, &scene);
    assert_pixel(&image, 0, 0, BACKGROUND.to_rgba8(), 1);
    assert_pixel(&image, 16, 16, FILL.to_rgba8(), 1);
    assert_pixel(&image, 16, 0, FILL.to_rgba8(), 1);
    // Pixel (1, 5) straddles the arc (about 88 % coverage): strictly between both colors.
    let [r, ..] = image.pixel(1, 5);
    assert!(
        r > BACKGROUND.to_rgba8()[0] && r < FILL.to_rgba8()[0],
        "pixel (1, 5) red = {r}"
    );
    gpu.assert_no_validation_errors();
}

#[test]
fn borders_are_drawn_inside_the_bounds() {
    let Some(gpu) = gpu_or_skip("borders_are_drawn_inside_the_bounds") else {
        return;
    };
    let border = Color::from_rgba8(240, 240, 240, 255);
    let scene =
        scene_with(
            &[RectPrimitive::new(rect(4.0, 4.0, 24.0, 24.0), FILL).with_border(2.0, border)],
        );
    let image = render(&gpu, Extent::new(32, 32), 1.0, &scene);
    assert_pixel(&image, 3, 16, BACKGROUND.to_rgba8(), 1);
    assert_pixel(&image, 4, 16, border.to_rgba8(), 1);
    assert_pixel(&image, 5, 16, border.to_rgba8(), 1);
    assert_pixel(&image, 6, 16, FILL.to_rgba8(), 1);
    gpu.assert_no_validation_errors();
}

fn line(from: (f32, f32), to: (f32, f32), width: f32) -> LinePrimitive {
    LinePrimitive::new(
        Vec2::new(from.0, from.1),
        Vec2::new(to.0, to.1),
        width,
        STROKE,
    )
}

/// Asserts that a pixel is strictly between the background and the stroke (anti-aliased).
#[track_caller]
fn assert_partial(image: &koebi_testkit::Image, x: u32, y: u32) {
    let [r, ..] = image.pixel(x, y);
    let (low, high) = (BACKGROUND.to_rgba8()[0], STROKE.to_rgba8()[0]);
    assert!(r > low && r < high, "pixel ({x}, {y}) red = {r}");
}

#[test]
fn horizontal_lines_cover_their_width_and_round_caps() {
    let Some(gpu) = gpu_or_skip("horizontal_lines_cover_their_width_and_round_caps") else {
        return;
    };
    let mut scene = scene_with(&[]);
    scene.push_line(&line((4.0, 8.0), (28.0, 8.0), 4.0));
    let image = render(&gpu, Extent::new(32, 16), 1.0, &scene);
    // Rows 6 to 9 have their centers within 2 px of y = 8; rows 5 and 10 do not.
    for y in 6..10 {
        assert_pixel(&image, 16, y, STROKE.to_rgba8(), 1);
    }
    assert_pixel(&image, 16, 5, BACKGROUND.to_rgba8(), 1);
    assert_pixel(&image, 16, 10, BACKGROUND.to_rgba8(), 1);
    // Round caps reach 2 px beyond both ends: (3, 7) is inside, (1, 8) and (30, 8) are not,
    // and (2, 8) straddles the arc (about 92 % coverage).
    assert_pixel(&image, 3, 7, STROKE.to_rgba8(), 1);
    assert_pixel(&image, 1, 8, BACKGROUND.to_rgba8(), 1);
    assert_pixel(&image, 30, 8, BACKGROUND.to_rgba8(), 1);
    assert_partial(&image, 2, 8);
    gpu.assert_no_validation_errors();
}

#[test]
fn diagonal_lines_are_anti_aliased() {
    let Some(gpu) = gpu_or_skip("diagonal_lines_are_anti_aliased") else {
        return;
    };
    let mut scene = scene_with(&[]);
    scene.push_line(&line((2.0, 2.0), (14.0, 14.0), 2.0));
    let image = render(&gpu, Extent::new(16, 16), 1.0, &scene);
    // (8, 8) is centered on the line, (8, 12) is 2.8 px away, (9, 8) is 0.7 px away: its
    // coverage is about 79 %.
    assert_pixel(&image, 8, 8, STROKE.to_rgba8(), 1);
    assert_pixel(&image, 8, 12, BACKGROUND.to_rgba8(), 1);
    assert_partial(&image, 9, 8);
    gpu.assert_no_validation_errors();
}

#[test]
fn zero_length_lines_draw_dots() {
    let Some(gpu) = gpu_or_skip("zero_length_lines_draw_dots") else {
        return;
    };
    let mut scene = scene_with(&[]);
    scene.push_line(&line((8.0, 8.0), (8.0, 8.0), 6.0));
    let image = render(&gpu, Extent::new(16, 16), 1.0, &scene);
    // A disc of radius 3 centered on (8, 8).
    assert_pixel(&image, 7, 7, STROKE.to_rgba8(), 1);
    assert_pixel(&image, 9, 8, STROKE.to_rgba8(), 1);
    assert_pixel(&image, 4, 8, BACKGROUND.to_rgba8(), 1);
    assert_pixel(&image, 10, 10, BACKGROUND.to_rgba8(), 1);
    gpu.assert_no_validation_errors();
}

#[test]
fn scale_factor_maps_line_widths_to_physical_pixels() {
    let Some(gpu) = gpu_or_skip("scale_factor_maps_line_widths_to_physical_pixels") else {
        return;
    };
    let mut scene = scene_with(&[]);
    scene.push_line(&line((4.0, 8.0), (28.0, 8.0), 2.0));
    let image = render(&gpu, Extent::new(64, 32), 2.0, &scene);
    // 2 logical px at scale 2: physical rows 14 to 17 around y = 16.
    assert_pixel(&image, 32, 14, STROKE.to_rgba8(), 1);
    assert_pixel(&image, 32, 17, STROKE.to_rgba8(), 1);
    assert_pixel(&image, 32, 13, BACKGROUND.to_rgba8(), 1);
    assert_pixel(&image, 32, 18, BACKGROUND.to_rgba8(), 1);
    gpu.assert_no_validation_errors();
}

#[test]
fn painter_order_holds_across_primitive_types() {
    let Some(gpu) = gpu_or_skip("painter_order_holds_across_primitive_types") else {
        return;
    };
    let top = Color::from_rgba8(240, 240, 20, 255);
    let mut scene = scene_with(&[RectPrimitive::new(rect(0.0, 0.0, 16.0, 16.0), FILL)]);
    scene.push_line(&line((0.0, 8.0), (16.0, 8.0), 4.0));
    scene.push_rect(&RectPrimitive::new(rect(8.0, 0.0, 8.0, 16.0), top));
    assert_eq!(scene.draw_count(), 3);
    let image = render(&gpu, Extent::new(16, 16), 1.0, &scene);
    assert_pixel(&image, 4, 2, FILL.to_rgba8(), 1);
    assert_pixel(&image, 4, 8, STROKE.to_rgba8(), 1);
    assert_pixel(&image, 12, 8, top.to_rgba8(), 1);
    gpu.assert_no_validation_errors();
}

#[test]
fn clips_cut_primitives_exactly_at_their_edges() {
    let Some(gpu) = gpu_or_skip("clips_cut_primitives_exactly_at_their_edges") else {
        return;
    };
    let mut scene = Scene::new();
    scene.set_clear_color(BACKGROUND);
    scene.push_clip(rect(4.0, 4.0, 8.0, 8.0));
    scene.push_rect(&RectPrimitive::new(rect(0.0, 0.0, 32.0, 16.0), FILL));
    scene.pop_clip();
    // Drawn after the clip ends: not clipped.
    scene.push_rect(&RectPrimitive::new(rect(20.0, 0.0, 4.0, 4.0), STROKE));
    for scale in [1.0_f32, 2.0] {
        let factor = if scale > 1.0 { 2 } else { 1 };
        let image = render(&gpu, Extent::new(32 * factor, 16 * factor), scale, &scene);
        for y in 0..16 * factor {
            for x in 0..32 * factor {
                let (lx, ly) = (x / factor, y / factor);
                let expected = if (4..12).contains(&lx) && (4..12).contains(&ly) {
                    FILL
                } else if (20..24).contains(&lx) && ly < 4 {
                    STROKE
                } else {
                    BACKGROUND
                };
                assert_pixel(&image, x, y, expected.to_rgba8(), 1);
            }
        }
    }
    gpu.assert_no_validation_errors();
}
