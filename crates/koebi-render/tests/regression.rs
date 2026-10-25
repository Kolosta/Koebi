//! Visual regression tests: reference scenes compared with golden PNGs in `tests/golden/`.
//!
//! After an intended visual change, run `cargo xtask bless`, inspect the updated PNGs (GitHub
//! shows image diffs in pull requests) and commit them with the change.

mod common;

use common::render;
use koebi_core::{Color, Extent, Oklch, Rect, Vec2};
use koebi_render::{LinePrimitive, RectPrimitive, Scene};
use koebi_testkit::{Tolerance, assert_golden, gpu_or_skip};

/// Anti-aliased edges may differ by a few units between drivers (lavapipe, NVIDIA, MoltenVK).
const CROSS_DRIVER: Tolerance = Tolerance::new(3, 8);

/// A miniature of the application's card: rounded rect, border, translucent overlay.
fn showcase_scene() -> Scene {
    let accent = Oklch::new(0.62, 0.17, 265.0);
    let mut scene = Scene::new();
    scene.set_clear_color(Color::from_hex(0x11_13_18));
    scene.push_rect(
        &RectPrimitive::new(
            Rect::from_origin_size(Vec2::new(20.0, 20.0), Vec2::new(120.0, 80.0)),
            accent.to_color(),
        )
        .with_corner_radius(16.0)
        .with_border(
            2.0,
            accent.with_lightness(0.80).with_chroma(0.10).to_color(),
        ),
    );
    scene.push_rect(
        &RectPrimitive::new(
            Rect::from_origin_size(Vec2::new(90.0, 60.0), Vec2::new(56.0, 48.0)),
            Color::from_hex(0xFF_CC_00).with_alpha(0.6),
        )
        .with_corner_radius(8.0),
    );
    scene
}

#[test]
fn showcase_scene_matches_golden() {
    let Some(gpu) = gpu_or_skip("showcase_scene_matches_golden") else {
        return;
    };
    let image = render(&gpu, Extent::new(160, 120), 1.0, &showcase_scene());
    assert_golden!(image, "showcase", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}

#[test]
fn showcase_scene_at_2x_matches_golden() {
    let Some(gpu) = gpu_or_skip("showcase_scene_at_2x_matches_golden") else {
        return;
    };
    let image = render(&gpu, Extent::new(320, 240), 2.0, &showcase_scene());
    assert_golden!(image, "showcase_2x", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}

/// Strokes: a check mark on a checkbox, a chevron, a plus and a cross, a fan of widths from a
/// hairline to 4 px, dots, and a translucent stroke across a rectangle.
fn lines_scene() -> Scene {
    let white = Color::WHITE;
    let accent = Oklch::new(0.62, 0.17, 265.0).to_color();
    let point = Vec2::new;
    let stroke = |scene: &mut Scene, from: Vec2, to: Vec2, width: f32, color: Color| {
        scene.push_line(&LinePrimitive::new(from, to, width, color));
    };
    let mut scene = Scene::new();
    scene.set_clear_color(Color::from_hex(0x11_13_18));

    // Checked checkbox: the mark is two segments sharing an end point.
    scene.push_rect(
        &RectPrimitive::new(
            Rect::from_origin_size(point(12.0, 12.0), point(24.0, 24.0)),
            accent,
        )
        .with_corner_radius(4.0),
    );
    stroke(&mut scene, point(18.0, 24.5), point(22.5, 29.0), 2.5, white);
    stroke(&mut scene, point(22.5, 29.0), point(30.0, 19.0), 2.5, white);
    // Chevron, plus and cross.
    for [x0, y0, x1, y1] in [
        [48.0, 14.0, 56.0, 24.0],
        [56.0, 24.0, 48.0, 34.0],
        [72.0, 24.0, 92.0, 24.0],
        [82.0, 14.0, 82.0, 34.0],
        [104.0, 14.0, 124.0, 34.0],
        [124.0, 14.0, 104.0, 34.0],
    ] {
        stroke(&mut scene, point(x0, y0), point(x1, y1), 2.0, white);
    }
    // A fan of widths and angles.
    let center = point(16.0, 108.0);
    for (index, width) in [0.5_f32, 1.0, 1.5, 2.0, 3.0, 4.0].into_iter().enumerate() {
        let angle = 0.15 + 0.25 * index as f32;
        let tip = center + point(angle.cos(), -angle.sin()) * 52.0;
        stroke(&mut scene, center, tip, width, white);
    }
    // Dots: zero-length segments.
    for (index, width) in [1.0_f32, 2.0, 4.0, 8.0].into_iter().enumerate() {
        let x = 88.0 + 14.0 * index as f32;
        stroke(&mut scene, point(x, 52.0), point(x, 52.0), width, accent);
    }
    // Painter's order and premultiplied blending across primitive types.
    scene.push_rect(
        &RectPrimitive::new(
            Rect::from_origin_size(point(88.0, 70.0), point(56.0, 36.0)),
            Color::from_hex(0xFF_CC_00),
        )
        .with_corner_radius(6.0),
    );
    stroke(
        &mut scene,
        point(80.0, 112.0),
        point(152.0, 64.0),
        6.0,
        accent.with_alpha(0.6),
    );
    scene
}

#[test]
fn lines_scene_matches_golden() {
    let Some(gpu) = gpu_or_skip("lines_scene_matches_golden") else {
        return;
    };
    let image = render(&gpu, Extent::new(160, 120), 1.0, &lines_scene());
    assert_golden!(image, "lines", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}

#[test]
fn lines_scene_at_2x_matches_golden() {
    let Some(gpu) = gpu_or_skip("lines_scene_at_2x_matches_golden") else {
        return;
    };
    let image = render(&gpu, Extent::new(320, 240), 2.0, &lines_scene());
    assert_golden!(image, "lines_2x", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}
