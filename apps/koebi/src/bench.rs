//! The performance baseline (`cargo xtask bench`, issue #30): how long the application takes to
//! build the scenes of real screens on the CPU, to lay text out, and to render a frame
//! offscreen, measured on the machine that runs it.
//!
//! These are tests ignored by default: `cargo xtask bench` runs them in release, one at a time,
//! and gathers the table each prints between `BENCH-BEGIN` and `BENCH-END`. Timings are wall
//! time with `Instant`: compare them on one machine, never across machines.

use core::time::Duration;
use std::time::Instant;

use koebi_core::{Extent, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_render::{Renderer2D, Scene};
use koebi_text::{Align as TextAlign, Fonts, Paragraph, TextBlock, TextStyle};
use koebi_tokens::{KOEBI, component};
use koebi_ui::{Input, StyleCache, Ui, UiState};

use crate::gallery::Gallery;
use crate::settings::Page;
use crate::settings::tests::Harness;

/// Frames measured per screen, after the warm-up.
const FRAMES: usize = 200;
const WARM_UP: usize = 10;
/// Offscreen frames measured per screen: each waits for the GPU and reads the frame back.
const GPU_FRAMES: usize = 40;

/// The median, 95th percentile and slowest of `times`, in microseconds.
fn summary(times: &mut [Duration]) -> (f64, f64, f64) {
    times.sort_unstable();
    let micros = |time: Duration| time.as_secs_f64() * 1e6;
    let at = |share: f64| {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "an index into a few hundred times"
        )]
        let index = ((times.len() - 1) as f64 * share).round() as usize;
        micros(times[index])
    };
    (at(0.5), at(0.95), micros(times[times.len() - 1]))
}

fn row(name: &str, what: &str, times: &mut [Duration], scene: Option<&Scene>) {
    let (median, p95, max) = summary(times);
    let counts = scene.map_or_else(String::new, |scene| {
        format!(
            "{} rects, {} glyphs, {} icons, {} draws",
            scene.rect_count(),
            scene.glyph_count(),
            scene.icon_count(),
            scene.draw_count()
        )
    });
    println!("| {name} | {what} | {median:.0} | {p95:.0} | {max:.0} | {counts} |");
}

fn header() {
    println!("BENCH-BEGIN");
    println!("| Screen | Measure | Median (µs) | p95 (µs) | Max (µs) | Scene |");
    println!("| --- | --- | ---: | ---: | ---: | --- |");
}

/// The settings pages measured, with their names in the table.
const PAGES: [(Page, &str); 6] = [
    (Page::Theme, "Settings › Theme"),
    (Page::Customisation, "Settings › Customisation"),
    (Page::Keymaps, "Settings › Keymaps"),
    (Page::Inputs, "Settings › Inputs"),
    (Page::Icons, "Settings › Icons"),
    (Page::Language, "Settings › Language"),
];

/// Builds the scene of the gallery in a window of `size`.
fn gallery_frame(
    gallery: &mut Gallery,
    design: &mut DesignSystem,
    styles: &mut StyleCache,
    state: &mut UiState,
    size: Vec2,
) -> Scene {
    let area = Rect::from_origin_size(Vec2::ZERO, size);
    let mut scene = Scene::new();
    let input = Input::new();
    let styles = styles.update(design, 1.0);
    let mut ui = Ui::new(&mut scene, styles, &input, state);
    gallery.show(&mut ui, design, area, area);
    drop(ui);
    scene
}

#[test]
#[ignore = "benchmark: cargo xtask bench"]
fn baseline() {
    header();
    let mut scenes: Vec<(&str, Scene, Extent)> = Vec::new();

    // Scenes built on the CPU, as each frame builds them.
    for (page, name) in PAGES {
        let mut harness = Harness::new();
        harness.settings.page = page;
        for _ in 0..WARM_UP {
            harness.frame();
        }
        let mut times = Vec::with_capacity(FRAMES);
        let mut last = Scene::new();
        for _ in 0..FRAMES {
            let start = Instant::now();
            last = harness.frame();
            times.push(start.elapsed());
        }
        let background = harness
            .design
            .get(component::WINDOW_BACKGROUND_COLOR_DEFAULT);
        last.set_clear_color(background);
        row(name, "CPU scene", &mut times, Some(&last));
        scenes.push((name, last, Extent::new(960, 680)));
    }
    let mut design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
    let mut gallery = Gallery::new();
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let mut styles = StyleCache::new(&design);
    let size = Vec2::new(1600.0, 1000.0);
    for _ in 0..WARM_UP {
        gallery_frame(&mut gallery, &mut design, &mut styles, &mut state, size);
    }
    let mut times = Vec::with_capacity(FRAMES);
    let mut last = Scene::new();
    for _ in 0..FRAMES {
        let start = Instant::now();
        last = gallery_frame(&mut gallery, &mut design, &mut styles, &mut state, size);
        times.push(start.elapsed());
    }
    last.set_clear_color(design.get(component::WINDOW_BACKGROUND_COLOR_DEFAULT));
    row("Gallery", "CPU scene", &mut times, Some(&last));
    scenes.push(("Gallery", last, Extent::new(1600, 1000)));

    // Text laid out in paragraphs, Latin and Japanese.
    let fonts = Fonts::with_embedded();
    for (script, sample) in [
        ("Latin", crate::gallery::typography::SAMPLES[0].1),
        ("Japanese", crate::gallery::typography::SAMPLES[1].1),
    ] {
        let family = if script == "Latin" {
            koebi_text::embedded::NOTO_SANS
        } else {
            koebi_text::embedded::NOTO_SANS_JP
        };
        let font = fonts.select(family, 400).expect("an embedded font");
        let style = TextStyle::new(font, 16.0);
        let paragraph = Paragraph {
            width: Some(600.0),
            line_height: 1.4,
            align: TextAlign::Start,
        };
        let mut block = TextBlock::new();
        let mut times = Vec::with_capacity(FRAMES);
        for _ in 0..FRAMES {
            let start = Instant::now();
            block.layout(&fonts, sample, &style, &paragraph);
            times.push(start.elapsed());
        }
        row(
            &format!("Paragraph ({script})"),
            "text layout",
            &mut times,
            None,
        );
    }

    // Frames rendered offscreen: recording, the GPU's work, and reading the frame back.
    if let Some(gpu) = koebi_testkit::gpu_or_skip("bench") {
        let format = koebi_gpu::TextureFormat::Rgba8Unorm;
        let mut renderer = Renderer2D::new(&gpu.device, format).expect("renderer creation");
        renderer.set_fonts(Fonts::with_embedded());
        renderer
            .set_icons(koebi_icons::packed())
            .expect("a valid icon set");
        for (name, scene, extent) in &scenes {
            let mut times = Vec::with_capacity(GPU_FRAMES);
            for frame in 0..GPU_FRAMES + 2 {
                let start = Instant::now();
                gpu.render_offscreen(*extent, format, |encoder, target| {
                    renderer
                        .render(
                            encoder,
                            target,
                            scene,
                            frame % koebi_gpu::FRAMES_IN_FLIGHT,
                            1.0,
                        )
                        .expect("render");
                })
                .expect("offscreen rendering");
                // The first frames store the glyphs and icons: measured apart from the rest.
                if frame >= 2 {
                    times.push(start.elapsed());
                }
            }
            row(name, "offscreen frame", &mut times, None);
        }
    }
    println!("BENCH-END");
}
