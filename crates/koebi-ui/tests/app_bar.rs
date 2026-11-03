//! The application bar on screen: glyphs that stay sharp and symmetric at every scale, and the
//! whole bar matched against its golden image.

mod common;

use common::render;
use koebi_core::input::{Key, ModifierKey};
use koebi_core::{Color, Extent, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_gpu::TextureFormat;
use koebi_input::Shortcut;
use koebi_render::{Renderer2D, Scene};
use koebi_testkit::{HeadlessGpu, Image, Tolerance, assert_golden, gpu_or_skip};
use koebi_text::Fonts;
use koebi_tokens::KOEBI;
use koebi_ui::{AppBar, Input, StatusEntry, Styles, Ui, UiState, WidgetId, WindowGlyph};

/// Anti-aliased edges may differ by a few units between drivers.
const CROSS_DRIVER: Tolerance = Tolerance::new(3, 8);

const SCALES: [f32; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];

/// `glyph` in white on black, around the centre of a square of `side` pixels, at `scale`.
fn glyph_image(gpu: &HeadlessGpu, glyph: WindowGlyph, scale: f32, side: u32) -> Image {
    let design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
    let styles = Styles::resolve(&design);
    let mut state = UiState::new();
    state.set_scale_factor(scale);
    let input = Input::new();
    let mut scene = Scene::new();
    scene.set_clear_color(Color::BLACK);
    {
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        let center = side as f32 * 0.5 / scale;
        ui.window_glyph(glyph, Vec2::new(center, center), Color::WHITE);
    }
    let format = TextureFormat::Rgba8Unorm;
    let mut renderer = Renderer2D::new(&gpu.device, format).expect("renderer creation");
    gpu.render_offscreen(Extent::new(side, side), format, |encoder, target| {
        renderer
            .render(encoder, target, &scene, 0, scale)
            .expect("render");
    })
    .expect("offscreen rendering")
}

/// The pixels the glyph touches, as `(left, top, right, bottom)` inclusive.
fn bounds(image: &Image) -> (u32, u32, u32, u32) {
    let (mut left, mut top, mut right, mut bottom) = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..image.height() {
        for x in 0..image.width() {
            if image.pixel(x, y)[0] > 0 {
                left = left.min(x);
                top = top.min(y);
                right = right.max(x);
                bottom = bottom.max(y);
            }
        }
    }
    (left, top, right, bottom)
}

#[test]
fn the_close_cross_has_four_equal_branches_at_every_scale() {
    // Regression test: in the C++ application, the two branches on the right were shorter.
    let Some(gpu) = gpu_or_skip("the_close_cross_has_four_equal_branches_at_every_scale") else {
        return;
    };
    for scale in SCALES {
        let image = glyph_image(&gpu, WindowGlyph::Close, scale, 48);
        let (left, top, right, bottom) = bounds(&image);
        // Mirrored left to right and top to bottom around the box it fills.
        for y in top..=bottom {
            for x in left..=right {
                let pixel = image.pixel(x, y)[0];
                let mirrored_x = image.pixel(left + right - x, y)[0];
                let mirrored_y = image.pixel(x, top + bottom - y)[0];
                assert!(
                    pixel.abs_diff(mirrored_x) <= 2 && pixel.abs_diff(mirrored_y) <= 2,
                    "at scale {scale}, ({x}, {y}): {pixel}, mirrored {mirrored_x} and {mirrored_y}"
                );
            }
        }
        // As high as it is wide.
        assert_eq!(right - left, bottom - top, "at scale {scale}");
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn straight_glyphs_are_sharp_at_every_scale() {
    let Some(gpu) = gpu_or_skip("straight_glyphs_are_sharp_at_every_scale") else {
        return;
    };
    for glyph in [
        WindowGlyph::Minimize,
        WindowGlyph::Maximize,
        WindowGlyph::Restore,
    ] {
        for scale in SCALES {
            let image = glyph_image(&gpu, glyph, scale, 48);
            for y in 0..image.height() {
                for x in 0..image.width() {
                    let value = image.pixel(x, y)[0];
                    assert!(
                        value == 0 || value == 255,
                        "{glyph:?} at scale {scale}: an intermediate pixel at ({x}, {y}): {value}"
                    );
                }
            }
        }
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn the_maximize_square_is_square_and_the_restore_glyph_fills_the_same_box() {
    let Some(gpu) = gpu_or_skip("the_maximize_square_is_square") else {
        return;
    };
    for scale in SCALES {
        let square = bounds(&glyph_image(&gpu, WindowGlyph::Maximize, scale, 48));
        assert_eq!(square.2 - square.0, square.3 - square.1, "at scale {scale}");
        let restore = bounds(&glyph_image(&gpu, WindowGlyph::Restore, scale, 48));
        assert_eq!(square, restore, "at scale {scale}");
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn the_bar_matches_golden() {
    let Some(gpu) = gpu_or_skip("the_bar_matches_golden") else {
        return;
    };
    let design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
    let styles = Styles::resolve(&design);
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let mut input = Input::new();
    let bar = Rect::from_origin_size(Vec2::ZERO, Vec2::new(320.0, 24.0));
    let id = WidgetId::new("bar");
    let width = styles.app_bar.button_width;
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    // The close button under the pointer, as the system's turns red.
    input.pointer_moved(Vec2::new(320.0 - width * 0.5, 12.0));
    for _ in 0..2 {
        scene.clear();
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        AppBar::new(id, "Koebi")
            .filled(true)
            .show(&mut ui, bar, |_, _| ());
    }
    let image = render(&gpu, Extent::new(320, 24), &scene);
    assert_golden!(image, "app_bar_dark", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}

#[test]
fn the_status_bar_matches_golden() {
    let Some(gpu) = gpu_or_skip("the_status_bar_matches_golden") else {
        return;
    };
    let design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
    let styles = Styles::resolve(&design);
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let input = Input::new();
    let height = styles.status_bar.height;
    let bar = Rect::from_origin_size(Vec2::ZERO, Vec2::new(480.0, height));
    let entries = [
        StatusEntry {
            shortcut: Shortcut::key(Key::Delete),
            label: "Delete",
            priority: 7,
        },
        StatusEntry {
            shortcut: Shortcut::key(Key::G).with(ModifierKey::Ctrl),
            label: "Group",
            priority: 6,
        },
        StatusEntry {
            shortcut: Shortcut::key(Key::Home),
            label: "Frame all",
            priority: 5,
        },
        // Lowest, and no room left for it.
        StatusEntry {
            shortcut: Shortcut::key(Key::F12),
            label: "Frame times, a long label that does not fit",
            priority: 1,
        },
    ];
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
    ui.status_bar(bar, &entries, "Koebi 0.4.0");
    drop(ui);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a bar a few dozen pixels high"
    )]
    let image = render(&gpu, Extent::new(480, height as u32), &scene);
    assert_golden!(image, "status_bar_dark", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}
