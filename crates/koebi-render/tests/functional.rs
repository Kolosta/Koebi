//! Functional tests: the renderer draws exactly what the scene describes.
//!
//! Expectations are computed analytically (pixel bounds, premultiplied blending), so they
//! hold on every conformant driver. Visual regressions of complex scenes are covered by the
//! golden-image tests in `regression.rs`.

mod common;

use common::{assert_pixel, render};
use koebi_core::{Color, Extent, Rect, Vec2};
use koebi_render::{RectPrimitive, Scene};
use koebi_testkit::gpu_or_skip;

const BACKGROUND: Color = Color::from_rgba8(16, 32, 48, 255);
const FILL: Color = Color::from_rgba8(200, 100, 50, 255);

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
