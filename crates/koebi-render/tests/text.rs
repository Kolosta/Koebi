//! Functional tests of text: glyphs land on the GPU exactly as the CPU rasterized them.
//!
//! The reference is `koebi_text::Rasterizer` itself: the renderer only snaps pen positions,
//! caches bitmaps in its atlas and samples them one texel per pixel, so every pixel is known.

mod common;

use common::assert_pixel;
use koebi_core::{Color, Extent, Rect, Vec2};
use koebi_gpu::TextureFormat;
use koebi_render::{GlyphPrimitive, RectPrimitive, Renderer2D, Scene};
use koebi_testkit::{HeadlessGpu, Image, gpu_or_skip};
use koebi_text::{Font, FontId, Fonts, GlyphBitmap, GlyphId, Rasterizer, TextLine, TextStyle};

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
        color: Color::WHITE,
    }
}

fn rasterize(font: &Font, id: GlyphId, size: f32, offset: f32) -> GlyphBitmap {
    let mut bitmap = GlyphBitmap::default();
    Rasterizer::new()
        .rasterize(font, id, size, offset, &mut bitmap)
        .expect("Noto Sans glyphs rasterize");
    bitmap
}

/// Asserts that `image` shows `bitmap` in white over black with its pen position at
/// (`x`, `baseline`) physical pixels, and nothing else around it.
#[track_caller]
fn assert_glyph_at(image: &Image, bitmap: &GlyphBitmap, x: i32, baseline: i32) {
    let left = i64::from(x) + i64::from(bitmap.left);
    let top = i64::from(baseline) + i64::from(bitmap.top);
    for py in 0..image.height() {
        for px in 0..image.width() {
            let column = u32::try_from(i64::from(px) - left).ok();
            let row = u32::try_from(i64::from(py) - top).ok();
            let coverage = column
                .zip(row)
                .map_or(0, |(column, row)| bitmap.coverage_at(column, row));
            assert_pixel(image, px, py, [coverage, coverage, coverage, 255], 1);
        }
    }
}

#[test]
fn glyphs_show_exactly_their_rasterized_coverage() {
    let Some(gpu) = gpu_or_skip("glyphs_show_exactly_their_rasterized_coverage") else {
        return;
    };
    let (fonts, regular) = fonts();
    let font = fonts.get(regular).unwrap();
    let h = font.glyph_index('H').unwrap();
    let mut frames = Frames::new(&gpu, fonts.clone());

    let image = frames.render(
        Extent::new(32, 32),
        1.0,
        &glyph_scene(&[glyph(regular, h, 20.0, 4.0, 24.0)]),
    );
    assert_glyph_at(&image, &rasterize(font, h, 20.0, 0.0), 4, 24);

    // A quarter-pixel pen position draws the bitmap rasterized a quarter pixel right.
    let image = frames.render(
        Extent::new(32, 32),
        1.0,
        &glyph_scene(&[glyph(regular, h, 20.0, 4.25, 24.4)]),
    );
    assert_glyph_at(&image, &rasterize(font, h, 20.0, 0.25), 4, 24);

    // At scale 2, logical sizes and positions double: the glyph is rasterized at 20 pixels.
    let image = frames.render(
        Extent::new(32, 32),
        2.0,
        &glyph_scene(&[glyph(regular, h, 10.0, 2.0, 12.0)]),
    );
    assert_glyph_at(&image, &rasterize(font, h, 20.0, 0.0), 4, 24);
    gpu.assert_no_validation_errors();
}

#[test]
fn text_blends_its_color_with_coverage_in_painters_order() {
    let Some(gpu) = gpu_or_skip("text_blends_its_color_with_coverage_in_painters_order") else {
        return;
    };
    let (fonts, regular) = fonts();
    let font = fonts.get(regular).unwrap();
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

    // A fully covered pixel of the "I" stem: half orange over blue.
    let i = rasterize(font, line.glyphs()[0].glyph, 24.0, 0.0);
    let solid = (0..i.height)
        .flat_map(|y| (0..i.width).map(move |x| (x, y)))
        .find(|(x, y)| i.coverage_at(*x, *y) == 255)
        .expect("a 24-pixel stem has solid pixels");
    let stem = (
        u32::try_from(2 + i.left).unwrap() + solid.0,
        u32::try_from(26 + i.top).unwrap() + solid.1,
    );
    assert_pixel(&image, stem.0, stem.1, [128, 64, 128, 255], 1);
    assert_pixel(&image, 20, 16, cover.to_rgba8(), 0);
    gpu.assert_no_validation_errors();
}

#[test]
fn the_atlas_keeps_glyphs_across_frames_and_grows_when_full() {
    let Some(gpu) = gpu_or_skip("the_atlas_keeps_glyphs_across_frames_and_grows_when_full") else {
        return;
    };
    let (fonts, regular) = fonts();
    let font = fonts.get(regular).unwrap();
    let h = font.glyph_index('H').unwrap();
    let o = font.glyph_index('o').unwrap();
    let mut frames = Frames::new(&gpu, fonts.clone());
    let small = glyph_scene(&[glyph(regular, h, 20.0, 4.0, 24.0)]);
    frames.render(Extent::new(32, 32), 1.0, &small);

    // Enough large glyphs to outgrow 512 × 512 texels twice: the atlas grows during the
    // frame and still shows every glyph.
    let sizes = (0..24).map(|step| 60.0 + 10.0 * step as f32);
    let big: Vec<_> = sizes
        .flat_map(|size| {
            [
                glyph(regular, h, size, 0.0, 0.0),
                glyph(regular, o, size, 0.0, 0.0),
            ]
        })
        .collect();
    frames.render(Extent::new(32, 32), 1.0, &glyph_scene(&big));

    // A later frame draws the first glyph again, from the grown atlas, alongside a new one.
    let image = frames.render(
        Extent::new(64, 32),
        1.0,
        &glyph_scene(&[
            glyph(regular, h, 20.0, 4.0, 24.0),
            glyph(regular, o, 20.0, 36.0, 24.0),
        ]),
    );
    let reference = rasterize(font, h, 20.0, 0.0);
    let left = 4 + reference.left;
    let top = 24 + reference.top;
    for y in 0..reference.height {
        for x in 0..reference.width {
            let coverage = reference.coverage_at(x, y);
            let pixel = (
                u32::try_from(left).unwrap() + x,
                u32::try_from(top).unwrap() + y,
            );
            assert_pixel(
                &image,
                pixel.0,
                pixel.1,
                [coverage, coverage, coverage, 255],
                1,
            );
        }
    }
    let o_bitmap = rasterize(font, o, 20.0, 0.0);
    let o_center = (
        u32::try_from(36 + o_bitmap.left).unwrap() + o_bitmap.width / 2,
        u32::try_from(24 + o_bitmap.top).unwrap(),
    );
    // The top of the "o" bowl is ink, its centre a hole.
    assert!(image.pixel(o_center.0, o_center.1 + 1)[0] > 128);
    let hole = o_center.1 + o_bitmap.height / 2;
    assert_pixel(&image, o_center.0, hole, [0, 0, 0, 255], 1);
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
