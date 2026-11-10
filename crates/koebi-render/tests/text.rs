//! Functional tests of text: glyphs land on the GPU as their curves give them on the CPU.
//!
//! The reference is `koebi_svg::raster::coverage`, the CPU twin of the coverage the glyph
//! shader computes, over the curves `koebi_text::Outline::monotonic_curves` gives: every pixel
//! is known to a rounding.

mod common;

use common::assert_pixel;
use koebi_core::{Color, Extent, Rect, Vec2};
use koebi_gpu::TextureFormat;
use koebi_render::{GlyphPrimitive, RectPrimitive, Renderer2D, Scene};
use koebi_svg::raster::coverage;
use koebi_svg::{FillRule, Layer, Paint, Point, Rgba, Stop};
use koebi_testkit::{HeadlessGpu, Image, gpu_or_skip};
use koebi_text::{Font, FontId, Fonts, GlyphId, Outline, TextLine, TextStyle};

const BACKGROUND: Color = Color::BLACK;

fn fonts() -> (Fonts, FontId) {
    let fonts = Fonts::with_embedded();
    let regular = fonts
        .select(koebi_text::embedded::NOTO_SANS, 400)
        .expect("Noto Sans Regular is embedded");
    (fonts, regular)
}

/// A renderer kept across frames, as the application keeps it.
struct Frames<'a> {
    gpu: &'a HeadlessGpu,
    renderer: Renderer2D,
    frame: usize,
}

impl<'a> Frames<'a> {
    fn new(gpu: &'a HeadlessGpu, fonts: Fonts) -> Self {
        let mut renderer =
            Renderer2D::new(&gpu.device, TextureFormat::Rgba8Unorm).expect("renderer creation");
        renderer.set_fonts(fonts);
        Self {
            gpu,
            renderer,
            frame: 0,
        }
    }

    fn render(&mut self, extent: Extent, scale_factor: f32, scene: &Scene) -> Image {
        let frame = self.frame;
        self.frame = (self.frame + 1) % koebi_gpu::FRAMES_IN_FLIGHT;
        let renderer = &mut self.renderer;
        self.gpu
            .render_offscreen(extent, TextureFormat::Rgba8Unorm, |encoder, target| {
                renderer
                    .render(encoder, target, scene, frame, scale_factor)
                    .expect("render");
            })
            .expect("offscreen rendering")
    }
}

fn glyph_scene(glyphs: &[GlyphPrimitive]) -> Scene {
    let mut scene = Scene::new();
    scene.set_clear_color(BACKGROUND);
    for glyph in glyphs {
        scene.push_glyph(glyph);
    }
    scene
}

fn glyph(font: FontId, id: GlyphId, size: f32, x: f32, y: f32) -> GlyphPrimitive {
    GlyphPrimitive {
        font,
        glyph: id,
        size,
        origin: Vec2::new(x, y),
        rotation: 0.0,
        color: Color::WHITE,
    }
}

/// The glyph's curves as a layer in ems (y down, the pen at the origin).
fn layer(font: &Font, id: GlyphId) -> Layer {
    let mut outline = Outline::new();
    outline.load(font, id).expect("an embedded glyph");
    let mut curves = Vec::new();
    outline.monotonic_curves(f32::from(font.units_per_em()), &mut curves);
    Layer {
        curves: curves
            .iter()
            .map(|curve| curve.map(|p| Point::new(p.x, p.y)))
            .collect(),
        rule: FillRule::NonZero,
        paint: Paint::Solid(Stop {
            offset: 0.0,
            color: Rgba([1.0; 4]),
            zone: 0,
        }),
    }
}

/// Where a glyph is drawn: its pen in physical pixels, its size in pixels per em, its turn.
#[derive(Clone, Copy)]
struct Placement {
    pen: Vec2,
    size: f32,
    rotation: f32,
}

impl Placement {
    fn upright(x: f32, y: f32, size: f32) -> Self {
        Self {
            pen: Vec2::new(x, y),
            size,
            rotation: 0.0,
        }
    }
}

/// Asserts that the pixels of `image` from `(x0, 0)`, `width` wide, show the glyph's coverage
/// in white over black, placed by `at`; checks every `step`-th pixel.
///
/// A ray passing exactly through the end of a curve counts it or not by a rounding, which the
/// GPU and the CPU may round apart: a few such pixels (one in 200) may differ by up to 24/255,
/// every other one by 2/255 at most.
#[track_caller]
fn assert_glyph(image: &Image, x0: u32, width: u32, layer: &Layer, at: Placement, step: usize) {
    let (sin, cos) = at.rotation.sin_cos();
    let (mut checked, mut ties) = (0_usize, Vec::new());
    for py in (0..image.height()).step_by(step) {
        for px in (0..width).step_by(step) {
            let d = Vec2::new(px as f32 + 0.5, py as f32 + 0.5) - at.pen;
            // Back into the em: turned the other way, then scaled.
            let local = Vec2::new(d.x * cos + d.y * sin, -d.x * sin + d.y * cos) / at.size;
            let cover = coverage(layer, Point::new(local.x, local.y), at.size);
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a coverage of 0 to 1"
            )]
            let expected = (cover * 255.0).round() as u8;
            let got = image.pixel(x0 + px, py)[0];
            checked += 1;
            let difference = got.abs_diff(expected);
            assert!(
                difference <= 24,
                "pixel ({}, {py}): {got}, its curves give {expected}",
                x0 + px
            );
            if difference > 2 {
                ties.push((x0 + px, py, got, expected));
            }
        }
    }
    assert!(
        ties.len() <= (checked / 200).max(1),
        "{} of {checked} pixels differ: {ties:?}",
        ties.len()
    );
}

#[test]
fn glyphs_show_the_coverage_of_their_curves() {
    let Some(gpu) = gpu_or_skip("glyphs_show_the_coverage_of_their_curves") else {
        return;
    };
    let (fonts, regular) = fonts();
    let font = fonts.get(regular).unwrap();
    let mut frames = Frames::new(&gpu, fonts.clone());
    for (character, size, x, y) in [
        ('H', 20.0, 4.0, 24.0),
        ('g', 13.0, 4.25, 20.4),
        ('o', 31.5, 2.6, 27.0),
        ('&', 11.0, 7.1, 18.0),
    ] {
        let id = font.glyph_index(character).unwrap();
        let image = frames.render(
            Extent::new(40, 36),
            1.0,
            &glyph_scene(&[glyph(regular, id, size, x, y)]),
        );
        // The baseline goes to a whole pixel; the pen keeps its place across.
        let at = Placement::upright(x, y.round(), size);
        assert_glyph(&image, 0, 40, &layer(font, id), at, 1);
    }
    // At scale 2, logical sizes and positions double.
    let h = font.glyph_index('H').unwrap();
    let image = frames.render(
        Extent::new(32, 32),
        2.0,
        &glyph_scene(&[glyph(regular, h, 10.0, 2.0, 12.0)]),
    );
    let at = Placement::upright(4.0, 24.0, 20.0);
    assert_glyph(&image, 0, 32, &layer(font, h), at, 1);
    gpu.assert_no_validation_errors();
}

#[test]
fn glyphs_turn_about_their_pen() {
    let Some(gpu) = gpu_or_skip("glyphs_turn_about_their_pen") else {
        return;
    };
    let (fonts, regular) = fonts();
    let font = fonts.get(regular).unwrap();
    let r = font.glyph_index('R').unwrap();
    let mut frames = Frames::new(&gpu, fonts.clone());
    for rotation in [core::f32::consts::FRAC_PI_2, 0.6, -2.4] {
        let mut primitive = glyph(regular, r, 24.0, 24.0, 24.0);
        primitive.rotation = rotation;
        let image = frames.render(Extent::new(48, 48), 1.0, &glyph_scene(&[primitive]));
        // A turned glyph keeps its pen exactly: no baseline rounding.
        let at = Placement {
            pen: Vec2::new(24.0, 24.0),
            size: 24.0,
            rotation,
        };
        assert_glyph(&image, 0, 48, &layer(font, r), at, 1);
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn a_glyph_stays_sharp_at_any_size() {
    let Some(gpu) = gpu_or_skip("a_glyph_stays_sharp_at_any_size") else {
        return;
    };
    let (fonts, regular) = fonts();
    let font = fonts.get(regular).unwrap();
    let o = font.glyph_index('O').unwrap();
    let mut frames = Frames::new(&gpu, fonts.clone());
    let image = frames.render(
        Extent::new(420, 440),
        1.0,
        &glyph_scene(&[glyph(regular, o, 500.0, 10.0, 400.0)]),
    );
    // Every seventh pixel: the ring, its hole and its anti-aliased edges.
    let at = Placement::upright(10.0, 400.0, 500.0);
    assert_glyph(&image, 0, 420, &layer(font, o), at, 7);
    gpu.assert_no_validation_errors();
}

#[test]
fn stored_glyphs_last_across_frames_beside_new_ones() {
    let Some(gpu) = gpu_or_skip("stored_glyphs_last_across_frames_beside_new_ones") else {
        return;
    };
    let (fonts, regular) = fonts();
    let font = fonts.get(regular).unwrap();
    let h = font.glyph_index('H').unwrap();
    let o = font.glyph_index('o').unwrap();
    let mut frames = Frames::new(&gpu, fonts.clone());
    frames.render(
        Extent::new(32, 32),
        1.0,
        &glyph_scene(&[glyph(regular, h, 20.0, 4.0, 24.0)]),
    );
    // Many new glyphs in between, stored after the first.
    let many: Vec<_> = ('a'..='z')
        .chain('A'..='Z')
        .filter_map(|c| font.glyph_index(c))
        .map(|id| glyph(regular, id, 18.0, 0.0, 0.0))
        .collect();
    frames.render(Extent::new(32, 32), 1.0, &glyph_scene(&many));
    // A later frame draws the first glyph again, stored, alongside one stored since.
    let image = frames.render(
        Extent::new(64, 32),
        1.0,
        &glyph_scene(&[
            glyph(regular, h, 20.0, 4.0, 24.0),
            glyph(regular, o, 20.0, 36.0, 24.0),
        ]),
    );
    assert_glyph(
        &image,
        0,
        32,
        &layer(font, h),
        Placement::upright(4.0, 24.0, 20.0),
        1,
    );
    let at = Placement::upright(4.0, 24.0, 20.0);
    assert_glyph(&image, 32, 32, &layer(font, o), at, 1);
    gpu.assert_no_validation_errors();
}

#[test]
fn text_blends_its_color_with_coverage_in_painters_order() {
    let Some(gpu) = gpu_or_skip("text_blends_its_color_with_coverage_in_painters_order") else {
        return;
    };
    let (fonts, regular) = fonts();
    let mut frames = Frames::new(&gpu, fonts.clone());
    let mut line = TextLine::new();
    let style = TextStyle {
        font: regular,
        size: 24.0,
        tracking: 0.0,
    };
    line.layout(&fonts, "IH", &style);
    let ink = Color::from_rgba8(255, 128, 0, 255).with_alpha(0.5);
    let mut scene = Scene::new();
    scene.set_clear_color(Color::from_rgba8(0, 0, 255, 255));
    scene.push_text(&line, Vec2::new(2.0, 26.0), ink);
    // A rectangle drawn after the text covers the right half of the target.
    let cover = Color::from_rgba8(20, 200, 20, 255);
    scene.push_rect(&RectPrimitive::new(
        Rect::from_origin_size(Vec2::new(16.0, 0.0), Vec2::new(16.0, 32.0)),
        cover,
    ));
    let image = frames.render(Extent::new(32, 32), 1.0, &scene);
    // A pixel the "I" stem covers fully: half orange over blue.
    let i = layer(fonts.get(regular).unwrap(), line.glyphs()[0].glyph);
    let stem = (0..32_u32)
        .flat_map(|y| (0..16_u32).map(move |x| (x, y)))
        .find(|&(x, y)| {
            let p = (Vec2::new(x as f32 + 0.5, y as f32 + 0.5) - Vec2::new(2.0, 26.0)) / 24.0;
            coverage(&i, Point::new(p.x, p.y), 24.0) >= 1.0
        })
        .expect("a 24-pixel stem has solid pixels");
    assert_pixel(&image, stem.0, stem.1, [128, 64, 128, 255], 1);
    assert_pixel(&image, 20, 16, cover.to_rgba8(), 0);
    gpu.assert_no_validation_errors();
}

#[test]
fn glyphs_without_fonts_or_ink_draw_nothing() {
    let Some(gpu) = gpu_or_skip("glyphs_without_fonts_or_ink_draw_nothing") else {
        return;
    };
    let (fonts, regular) = fonts();
    let space = fonts.get(regular).unwrap().glyph_index(' ').unwrap();
    // No fonts given to the renderer: every glyph is empty.
    let mut bare = Frames::new(&gpu, Fonts::new());
    let a = GlyphId(36);
    let image = bare.render(
        Extent::new(16, 16),
        1.0,
        &glyph_scene(&[glyph(regular, a, 12.0, 2.0, 12.0)]),
    );
    assert!((0..16).all(|y| (0..16).all(|x| image.pixel(x, y) == [0, 0, 0, 255])));
    // Spaces and glyphs of unknown fonts draw nothing either.
    let mut frames = Frames::new(&gpu, fonts.clone());
    let image = frames.render(
        Extent::new(16, 16),
        1.0,
        &glyph_scene(&[
            glyph(regular, space, 12.0, 2.0, 12.0),
            glyph(unknown_font(), a, 12.0, 2.0, 12.0),
        ]),
    );
    assert!((0..16).all(|y| (0..16).all(|x| image.pixel(x, y) == [0, 0, 0, 255])));
    gpu.assert_no_validation_errors();
}

/// A font identifier the two embedded fonts do not cover.
fn unknown_font() -> FontId {
    let mut fonts = Fonts::with_embedded();
    let font = Font::from_static(koebi_text::embedded::NOTO_SANS_REGULAR)
        .expect("the embedded font is valid");
    fonts.add("Other", font)
}
