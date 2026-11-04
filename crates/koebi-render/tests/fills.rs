//! Fills on screen: corner colors blended exactly, checkerboards, hue strips, wheels, rings
//! and triangles, each pixel as the color models compute it on the CPU.

mod common;

use common::{assert_pixel, render};
use koebi_core::{Color, Extent, Hsv, Oklch, Rect, Vec2};
use koebi_render::{Fill, FillPrimitive, HueModel, Pointing, Scene};
use koebi_testkit::gpu_or_skip;

const BACKGROUND: Color = Color::from_rgba8(16, 32, 48, 255);

fn scene_with(fills: &[FillPrimitive]) -> Scene {
    let mut scene = Scene::new();
    scene.set_clear_color(BACKGROUND);
    for fill in fills {
        scene.push_fill(fill);
    }
    scene
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Vec2::new(x, y), Vec2::new(width, height))
}

/// The pixel the GPU should show for `color` over the background.
fn shown(color: Color) -> [u8; 4] {
    color.to_rgba8()
}

#[test]
fn corner_colors_blend_into_the_saturation_value_square_of_a_hue() {
    let Some(gpu) = gpu_or_skip("corner_colors_blend_into_the_saturation_value_square_of_a_hue")
    else {
        return;
    };
    let hue = Hsv {
        h: 0.6,
        s: 1.0,
        v: 1.0,
    }
    .to_color(1.0);
    let square = FillPrimitive::new(
        rect(0.0, 0.0, 100.0, 100.0),
        Fill::Corners {
            top_left: Color::WHITE,
            top_right: hue,
            bottom_right: Color::BLACK,
            bottom_left: Color::BLACK,
        },
    );
    let image = render(&gpu, Extent::new(100, 100), 1.0, &scene_with(&[square]));
    // At each pixel's center, the HSV color of its saturation and value.
    for (x, y) in [(0_u32, 0_u32), (49, 20), (99, 0), (75, 75), (10, 90)] {
        let s = (x as f32 + 0.5) / 100.0;
        let v = 1.0 - (y as f32 + 0.5) / 100.0;
        let expected = Hsv { h: 0.6, s, v }.to_color(1.0);
        assert_pixel(&image, x, y, shown(expected), 1);
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn checkerboards_alternate_from_their_top_left() {
    let Some(gpu) = gpu_or_skip("checkerboards_alternate_from_their_top_left") else {
        return;
    };
    let (light, dark) = (Color::from_hex(0xCC_CC_CC), Color::from_hex(0x88_88_88));
    let checker = FillPrimitive::new(
        rect(0.0, 0.0, 40.0, 40.0),
        Fill::Checker {
            size: 5.0,
            light,
            dark,
        },
    );
    let image = render(&gpu, Extent::new(40, 40), 2.0, &scene_with(&[checker]));
    // Squares of 5 logical pixels: 10 physical at a scale of 2.
    assert_pixel(&image, 2, 2, shown(light), 0);
    assert_pixel(&image, 12, 2, shown(dark), 0);
    assert_pixel(&image, 12, 12, shown(light), 0);
    gpu.assert_no_validation_errors();
}

#[test]
fn wheels_put_red_at_the_top_and_saturation_outward() {
    let Some(gpu) = gpu_or_skip("wheels_put_red_at_the_top_and_saturation_outward") else {
        return;
    };
    let wheel = FillPrimitive::new(
        rect(0.0, 0.0, 101.0, 101.0),
        Fill::Wheel {
            model: HueModel::Hsv,
            level: 1.0,
            chroma: 0.0,
        },
    );
    let image = render(&gpu, Extent::new(101, 101), 1.0, &scene_with(&[wheel]));
    // The center: no saturation, white.
    assert_pixel(&image, 50, 50, [255, 255, 255, 255], 8);
    // Near the top edge: red; near the right edge, a quarter turn: chartreuse.
    let top = Hsv {
        h: 0.0,
        s: 47.5 / 50.5,
        v: 1.0,
    };
    assert_pixel(&image, 50, 3, shown(top.to_color(1.0)), 6);
    let right = image.pixel(97, 50);
    assert!(right[1] > 240 && right[2] < 40, "{right:?}");
    // Outside the disc: the background.
    assert_pixel(&image, 2, 2, shown(BACKGROUND), 0);
    gpu.assert_no_validation_errors();
}

#[test]
fn oklch_rings_keep_one_lightness_around() {
    let Some(gpu) = gpu_or_skip("oklch_rings_keep_one_lightness_around") else {
        return;
    };
    let ring = FillPrimitive::new(
        rect(0.0, 0.0, 100.0, 100.0),
        Fill::Ring {
            model: HueModel::Oklch,
            inner: 0.7,
            lightness: 0.75,
            chroma: 0.1,
        },
    );
    let image = render(&gpu, Extent::new(100, 100), 1.0, &scene_with(&[ring]));
    // The hole shows the background.
    assert_pixel(&image, 50, 50, shown(BACKGROUND), 0);
    // Around the ring, every pixel has the lightness asked, whatever its hue.
    for (x, y) in [(50_u32, 7_u32), (92, 50), (50, 92), (7, 50), (79, 21)] {
        let [r, g, b, _] = image.pixel(x, y);
        let lightness = Color::from_rgba8(r, g, b, 255).to_oklch().l;
        assert!((lightness - 0.75).abs() < 0.02, "({x}, {y}): {lightness}");
    }
    // At the top, the hue of red.
    let top = Oklch::new(0.75, 0.1, 0.0).to_oklab().to_color();
    assert_pixel(&image, 50, 8, shown(top), 6);
    gpu.assert_no_validation_errors();
}

#[test]
fn a_triangle_has_its_hue_white_and_black_at_its_corners() {
    let Some(gpu) = gpu_or_skip("a_triangle_has_its_hue_white_and_black_at_its_corners") else {
        return;
    };
    let triangle = FillPrimitive::new(rect(0.0, 0.0, 120.0, 120.0), Fill::Triangle { hue: 0.0 });
    let image = render(&gpu, Extent::new(120, 120), 1.0, &scene_with(&[triangle]));
    // Red points up; white a third of a turn clockwise (lower right); black lower left.
    let near = |angle: f32| {
        let (sin, cos) = (angle * core::f32::consts::TAU).sin_cos();
        let radius = 50.0;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "inside a 120-pixel image"
        )]
        let point = ((60.0 + radius * sin) as u32, (60.0 - radius * cos) as u32);
        point
    };
    let (x, y) = near(0.0);
    let red = image.pixel(x, y);
    assert!(red[0] > 200 && red[1] < 60 && red[2] < 60, "{red:?}");
    let (x, y) = near(1.0 / 3.0);
    let white = image.pixel(x, y);
    assert!(white.iter().take(3).all(|&c| c > 200), "{white:?}");
    let (x, y) = near(2.0 / 3.0);
    let black = image.pixel(x, y);
    assert!(black.iter().take(3).all(|&c| c < 50), "{black:?}");
    // Outside, the background.
    assert_pixel(&image, 2, 2, shown(BACKGROUND), 0);
    gpu.assert_no_validation_errors();
}

#[test]
fn arrows_point_their_tip_and_keep_their_outline_inside() {
    let Some(gpu) = gpu_or_skip("arrows_point_their_tip_and_keep_their_outline_inside") else {
        return;
    };
    let arrow = FillPrimitive::new(
        rect(0.0, 0.0, 20.0, 20.0),
        Fill::Arrow {
            pointing: Pointing::Right,
            color: Color::WHITE,
            outline: Color::BLACK,
            outline_width: 2.0,
        },
    );
    let image = render(&gpu, Extent::new(20, 20), 1.0, &scene_with(&[arrow]));
    // Its middle, white; along its base on the left, the outline; past its tip's sides, the
    // background.
    assert_pixel(&image, 6, 10, [255, 255, 255, 255], 2);
    assert_pixel(&image, 0, 10, [0, 0, 0, 255], 2);
    assert_pixel(&image, 18, 2, shown(BACKGROUND), 0);
    gpu.assert_no_validation_errors();
}

#[test]
fn hue_strips_and_planes_follow_hsv() {
    let Some(gpu) = gpu_or_skip("hue_strips_and_planes_follow_hsv") else {
        return;
    };
    let strip = FillPrimitive::new(
        rect(0.0, 0.0, 10.0, 120.0),
        Fill::Hues {
            model: HueModel::Hsv,
            vertical: true,
            lightness: 0.0,
            chroma: 0.0,
        },
    );
    let plane = FillPrimitive::new(
        rect(20.0, 0.0, 100.0, 100.0),
        Fill::HueSaturation { value: 0.8 },
    );
    let image = render(
        &gpu,
        Extent::new(120, 120),
        1.0,
        &scene_with(&[strip, plane]),
    );
    for y in [0_u32, 30, 60, 119] {
        let h = (y as f32 + 0.5) / 120.0;
        let expected = Hsv { h, s: 1.0, v: 1.0 }.to_color(1.0);
        assert_pixel(&image, 5, y, shown(expected), 1);
    }
    for (x, y) in [(20_u32, 0_u32), (70, 50), (119, 99)] {
        let h = (x as f32 - 20.0 + 0.5) / 100.0;
        let s = 1.0 - (y as f32 + 0.5) / 100.0;
        let expected = Hsv { h, s, v: 0.8 }.to_color(1.0);
        assert_pixel(&image, x, y, shown(expected), 1);
    }
    gpu.assert_no_validation_errors();
}
