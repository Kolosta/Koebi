//! Vector icons drawn on the GPU: the shader covers exactly what `koebi_svg::raster` computes on
//! the CPU, at every size; zones take the colors given, opacity and clips apply, and numbers
//! past the set draw nothing.

mod common;

use common::{assert_pixel, render_with_icons};
use koebi_core::{Color, Extent, Rect, Vec2};
use koebi_render::{IconColors, IconPrimitive, RectPrimitive, Scene};
use koebi_svg::pack::pack;
use koebi_svg::raster::{Colors, render};
use koebi_svg::{Icon, compile};
use koebi_testkit::{Image, gpu_or_skip};

/// Icons of every kind the compiler produces: a filled path with a root color (Material's
/// form), a stroke with round caps and joins, a linear and a radial gradient, an even-odd ring,
/// zones of three colors with a translucent one, and an icon against one side of its square.
const ICONS: [&str; 7] = [
    r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 -960 960 960" fill="#e3e3e3"><path class="ds-primary" d="m256-200-56-56 224-224-224-224 56-56 224 224 224-224 56 56-224 224 224 224-56 56-224-224-224 224Z"/></svg>"##,
    r##"<svg viewBox="0 0 24 24"><path d="M4 17L10 8L14 14L20 6" fill="none" stroke="#2266dd" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/></svg>"##,
    r##"<svg viewBox="0 0 24 24"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0.1" stop-color="#ff3300"/><stop offset="0.6" stop-color="#ffee00"/><stop offset="1" stop-color="#00aa44"/></linearGradient></defs><rect x="2" y="2" width="20" height="20" rx="5" fill="url(#g)"/></svg>"##,
    r##"<svg viewBox="0 0 24 24"><defs><radialGradient id="r" cx="12" cy="12" r="10" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#ffffff"/><stop offset="1" stop-color="#3344ff" stop-opacity="0.4"/></radialGradient></defs><circle cx="12" cy="12" r="10" fill="url(#r)"/></svg>"##,
    r#"<svg viewBox="0 0 20 20"><path fill-rule="evenodd" d="M10 1a9 9 0 1 1 0 18a9 9 0 1 1 0-18ZM10 5a5 5 0 1 1 0 10a5 5 0 1 1 0-10Z"/></svg>"#,
    r##"<svg viewBox="0 0 24 24"><rect class="ds-secondary" x="2" y="2" width="12" height="12" fill="#ea4335"/><circle class="ds-primary" cx="15" cy="15" r="7" fill="#1a73e8" fill-opacity="0.6"/><path class="ds-accent" d="M3 21h8" stroke="#34a853" stroke-width="2"/></svg>"##,
    r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 -960 960 960" fill="#e3e3e3"><path d="M800-80v-800h80v800h-80ZM560-280v-400h120v400H560Zm-240 0v-400h120v400H320Z"/></svg>"##,
];

fn icons() -> Vec<Icon> {
    ICONS
        .iter()
        .map(|text| compile(text).expect("test icons compile"))
        .collect()
}

fn square(x: f32, y: f32, side: f32) -> Rect {
    Rect::from_origin_size(Vec2::new(x, y), Vec2::new(side, side))
}

/// `value`, from 0 to 255, rounded to a byte.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped to the range of a byte first"
)]
fn byte(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

/// The CPU render of `icon`, premultiplied as the GPU target holds it.
fn reference(icon: &Icon, size: u32, colors: Colors<'_>) -> Vec<[u8; 4]> {
    render(icon, size, colors)
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[r, g, b, a]| {
            let alpha = f32::from(a) / 255.0;
            let premultiply = |channel: u8| byte(f32::from(channel) * alpha);
            [premultiply(r), premultiply(g), premultiply(b), a]
        })
        .collect()
}

/// Compares the square of `size` pixels at `(x, y)` in `image` with `expected`: the two
/// computations differ only by float rounding, every channel within 2.
#[track_caller]
fn assert_matches(image: &Image, x: u32, y: u32, size: u32, expected: &[[u8; 4]], what: &str) {
    for row in 0..size {
        for column in 0..size {
            let actual = image.pixel(x + column, y + row);
            let wanted = expected[(row * size + column) as usize];
            let close = actual.iter().zip(wanted).all(|(a, e)| a.abs_diff(e) <= 2);
            assert!(
                close,
                "{what}: pixel ({column}, {row}) is {actual:?}, the CPU gives {wanted:?}"
            );
        }
    }
}

#[test]
fn the_shader_covers_what_the_cpu_raster_computes() {
    let Some(gpu) = gpu_or_skip("the_shader_covers_what_the_cpu_raster_computes") else {
        return;
    };
    let icons = icons();
    let packed = pack(&icons.iter().collect::<Vec<_>>());
    for size in [16_u32, 24, 64] {
        let mut scene = Scene::new();
        let mut x = 0;
        for index in 0..icons.len() {
            let icon = u32::try_from(index).unwrap();
            scene.push_icon(&IconPrimitive {
                rect: square(x as f32, 0.0, size as f32),
                icon,
                colors: IconColors::Original,
                opacity: 1.0,
                rotation: 0.0,
            });
            x += size + 2;
        }
        let image = render_with_icons(&gpu, Extent::new(x, size), 1.0, &scene, &packed);
        for (index, icon) in icons.iter().enumerate() {
            let expected = reference(icon, size, Colors::Original);
            let at = u32::try_from(index).unwrap() * (size + 2);
            assert_matches(
                &image,
                at,
                0,
                size,
                &expected,
                &format!("icon {index} at {size} px"),
            );
        }
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn icons_scale_with_the_interface_and_stay_on_whole_pixels() {
    let Some(gpu) = gpu_or_skip("icons_scale_with_the_interface_and_stay_on_whole_pixels") else {
        return;
    };
    let icons = icons();
    let packed = pack(&icons.iter().collect::<Vec<_>>());
    let mut scene = Scene::new();
    // 16 logical pixels at 1.5: 24 physical, the corner at 3.2 × 1.5 = 4.8 rounded to 5.
    scene.push_icon(&IconPrimitive {
        rect: square(3.2, 3.2, 16.0),
        icon: 2,
        colors: IconColors::Original,
        opacity: 1.0,
        rotation: 0.0,
    });
    let image = render_with_icons(&gpu, Extent::new(40, 40), 1.5, &scene, &packed);
    let expected = reference(&icons[2], 24, Colors::Original);
    assert_matches(&image, 5, 5, 24, &expected, "scaled gradient");
    gpu.assert_no_validation_errors();
}

#[test]
fn zones_take_the_colors_given_and_keep_their_opacity() {
    let Some(gpu) = gpu_or_skip("zones_take_the_colors_given_and_keep_their_opacity") else {
        return;
    };
    let icons = icons();
    let packed = pack(&icons.iter().collect::<Vec<_>>());
    let white = Color::WHITE;
    let red = Color::rgb(1.0, 0.0, 0.0);
    let mut scene = Scene::new();
    // The three-color icon: primary (the translucent circle), secondary, accent.
    scene.push_icon(&IconPrimitive {
        rect: square(0.0, 0.0, 48.0),
        icon: 5,
        colors: IconColors::zones(&[white, red]),
        opacity: 1.0,
        rotation: 0.0,
    });
    // The same, one color for all.
    scene.push_icon(&IconPrimitive::tinted(square(50.0, 0.0, 48.0), 5, white));
    let image = render_with_icons(&gpu, Extent::new(98, 48), 1.0, &scene, &packed);
    let zones = [
        koebi_svg::Rgba([1.0, 1.0, 1.0, 1.0]),
        koebi_svg::Rgba([1.0, 0.0, 0.0, 1.0]),
    ];
    let expected = reference(&icons[5], 48, Colors::Zones(&zones));
    assert_matches(&image, 0, 0, 48, &expected, "two zones recolored");
    // The circle (primary) white at 60 %, the square (secondary) red, the accent's own green.
    assert_pixel(&image, 38, 38, [153, 153, 153, 153], 2);
    assert_pixel(&image, 6, 6, [255, 0, 0, 255], 0);
    assert_pixel(&image, 10, 42, [52, 168, 83, 255], 2);
    // Tinted: the accent white too.
    assert_pixel(&image, 60, 42, [255, 255, 255, 255], 0);
    gpu.assert_no_validation_errors();
}

#[test]
fn opacity_clips_and_numbers_past_the_set_apply() {
    let Some(gpu) = gpu_or_skip("opacity_clips_and_numbers_past_the_set_apply") else {
        return;
    };
    let icons = icons();
    let packed = pack(&icons.iter().collect::<Vec<_>>());
    let background = Color::from_rgba8(16, 32, 48, 255);
    let mut scene = Scene::new();
    scene.set_clear_color(background);
    // The cross, white at half opacity.
    scene.push_icon(&IconPrimitive {
        rect: square(0.0, 0.0, 24.0),
        icon: 0,
        colors: IconColors::uniform(Color::WHITE),
        opacity: 0.5,
        rotation: 0.0,
    });
    // Past the set: nothing.
    scene.push_icon(&IconPrimitive::tinted(
        square(24.0, 0.0, 24.0),
        99,
        Color::WHITE,
    ));
    // Clipped to its left half.
    scene.push_clip(Rect::from_origin_size(
        Vec2::new(48.0, 0.0),
        Vec2::new(12.0, 24.0),
    ));
    scene.push_icon(&IconPrimitive::tinted(
        square(48.0, 0.0, 24.0),
        0,
        Color::WHITE,
    ));
    scene.pop_clip();
    // A rectangle after the icons still draws over them, in order.
    scene.push_rect(&RectPrimitive::new(square(0.0, 22.0, 2.0), Color::BLACK));
    let image = render_with_icons(&gpu, Extent::new(72, 24), 1.0, &scene, &packed);
    // The cross's center: white at half over the background.
    let half = |channel: u8| byte(f32::midpoint(f32::from(channel), 255.0));
    assert_pixel(&image, 12, 12, [half(16), half(32), half(48), 255], 2);
    for x in 24..48 {
        for y in 0..24 {
            assert_pixel(&image, x, y, background.to_rgba8(), 0);
        }
    }
    assert_pixel(&image, 59, 12, [255, 255, 255, 255], 0);
    assert_pixel(&image, 61, 12, background.to_rgba8(), 0);
    assert_pixel(&image, 1, 23, [0, 0, 0, 255], 0);
    gpu.assert_no_validation_errors();
}

#[test]
fn without_a_set_icons_draw_nothing() {
    let Some(gpu) = gpu_or_skip("without_a_set_icons_draw_nothing") else {
        return;
    };
    let mut scene = Scene::new();
    scene.set_clear_color(Color::BLACK);
    scene.push_icon(&IconPrimitive::tinted(
        square(0.0, 0.0, 8.0),
        0,
        Color::WHITE,
    ));
    let image = common::render(&gpu, Extent::new(8, 8), 1.0, &scene);
    assert_pixel(&image, 4, 4, [0, 0, 0, 255], 0);
    gpu.assert_no_validation_errors();
}

#[test]
fn icons_turn_clockwise_about_their_center() {
    let Some(gpu) = gpu_or_skip("icons_turn_clockwise_about_their_center") else {
        return;
    };
    let icons = icons();
    let packed = pack(&icons.iter().collect::<Vec<_>>());
    let mut scene = Scene::new();
    // The bars standing against the right side, a quarter turn clockwise: against the bottom.
    scene.push_icon(&IconPrimitive {
        rect: square(4.0, 4.0, 24.0),
        icon: 6,
        colors: IconColors::Original,
        opacity: 1.0,
        rotation: core::f32::consts::FRAC_PI_2,
    });
    let image = render_with_icons(&gpu, Extent::new(32, 32), 1.0, &scene, &packed);
    let upright = reference(&icons[6], 24, Colors::Original);
    // Turned clockwise, the pixel at (x, y) shows the one at (y, 23 - x).
    let turned: Vec<[u8; 4]> = (0..24)
        .flat_map(|y| (0..24).map(move |x| (x, y)))
        .map(|(x, y)| upright[(23 - x) * 24 + y])
        .collect();
    assert_matches(&image, 4, 4, 24, &turned, "a quarter turn");
    gpu.assert_no_validation_errors();
}

#[test]
fn a_renderer_draws_the_set_it_was_given_last_and_refuses_corrupt_ones() {
    let Some(gpu) =
        gpu_or_skip("a_renderer_draws_the_set_it_was_given_last_and_refuses_corrupt_ones")
    else {
        return;
    };
    let icons = icons();
    // Icon 0 is the cross (its center covered), then the bars (their left third empty).
    let first = pack(&[&icons[0]]);
    let second = pack(&[&icons[6]]);
    let format = koebi_gpu::TextureFormat::Rgba8Unorm;
    let mut renderer = koebi_render::Renderer2D::new(&gpu.device, format).expect("renderer");
    let mut scene = Scene::new();
    scene.push_icon(&IconPrimitive::tinted(
        square(0.0, 0.0, 24.0),
        0,
        Color::WHITE,
    ));
    let frame = |renderer: &mut koebi_render::Renderer2D, index: usize| {
        gpu.render_offscreen(Extent::new(24, 24), format, |encoder, target| {
            renderer
                .render(encoder, target, &scene, index, 1.0)
                .expect("render");
        })
        .expect("offscreen rendering")
    };
    renderer.set_icons(&first).expect("a valid set");
    assert_pixel(&frame(&mut renderer, 0), 12, 12, [255, 255, 255, 255], 0);
    // Drawn again from the uploaded set, in the other frame slot.
    assert_pixel(&frame(&mut renderer, 1), 12, 12, [255, 255, 255, 255], 0);
    // Replaced: the bars, whose middle is empty.
    renderer.set_icons(&second).expect("a valid set");
    let replaced = frame(&mut renderer, 0);
    assert_pixel(&replaced, 3, 12, [0, 0, 0, 0], 0);
    assert_pixel(&replaced, 21, 12, [255, 255, 255, 255], 0);
    // A set whose records leave it is refused, and the last one stays.
    let mut corrupt = first.clone();
    corrupt.data[1][1] = 1e6;
    assert!(renderer.set_icons(&corrupt).is_err());
    assert_pixel(&frame(&mut renderer, 1), 21, 12, [255, 255, 255, 255], 0);
    gpu.assert_no_validation_errors();
}

#[test]
fn icons_keep_their_axis_on_a_pixel_center_or_on_a_pixel_edge() {
    let Some(gpu) = gpu_or_skip("icons_keep_their_axis_on_a_pixel_center_or_on_a_pixel_edge")
    else {
        return;
    };
    // A square filling its icon: what it covers is where the icon lies.
    let solid = compile(r#"<svg viewBox="0 0 1 1"><rect width="1" height="1"/></svg>"#).unwrap();
    let packed = pack(&[&solid]);
    // Centered on a pixel's center, the side is odd; on a pixel edge, even; the nearest to the
    // size asked for, the center where it was asked.
    let cases: [(f32, f32, (u32, u32)); 4] = [
        (10.5, 11.0, (5, 16)),
        (30.0, 11.0, (24, 36)),
        (50.5, 12.0, (44, 57)),
        (70.0, 12.0, (64, 76)),
    ];
    let mut scene = Scene::new();
    for (center, side, _) in cases {
        scene.push_icon(&IconPrimitive::tinted(
            square(center - side * 0.5, 2.0, side),
            0,
            Color::WHITE,
        ));
    }
    let image = render_with_icons(&gpu, Extent::new(80, 16), 1.0, &scene, &packed);
    let covered = |x: u32| image.pixel(x, 7)[3] == 255;
    for (center, side, (first, end)) in cases {
        for x in first.saturating_sub(2)..end + 2 {
            assert_eq!(
                covered(x),
                (first..end).contains(&x),
                "centered on {center}, {side} wide: column {x}"
            );
        }
    }
    gpu.assert_no_validation_errors();
}
