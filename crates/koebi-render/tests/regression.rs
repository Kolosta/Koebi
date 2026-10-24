//! Visual regression tests: reference scenes compared with golden PNGs in `tests/golden/`.
//!
//! After an intended visual change, run `cargo xtask bless`, inspect the updated PNGs (GitHub
//! shows image diffs in pull requests) and commit them with the change.

mod common;

use common::render;
use koebi_core::{Color, Extent, Oklch, Rect, Vec2};
use koebi_render::{RectPrimitive, Scene};
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
