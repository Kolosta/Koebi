//! Helpers shared by the components' integration tests.
//!
//! Each test binary compiles this module separately and may not use every helper.
#![allow(dead_code, reason = "not every test binary uses every helper")]

use koebi_core::Extent;
use koebi_gpu::TextureFormat;
use koebi_render::{Renderer2D, Scene};
use koebi_testkit::{HeadlessGpu, Image};
use koebi_text::Fonts;

/// Renders `scene` offscreen into an RGBA8 image, at scale factor 1, its text set in the
/// embedded fonts.
pub(crate) fn render(gpu: &HeadlessGpu, extent: Extent, scene: &Scene) -> Image {
    let format = TextureFormat::Rgba8Unorm;
    let mut renderer = Renderer2D::new(&gpu.device, format).expect("renderer creation");
    renderer.set_fonts(Fonts::with_embedded());
    gpu.render_offscreen(extent, format, |encoder, target| {
        renderer
            .render(encoder, target, scene, 0, 1.0)
            .expect("render");
    })
    .expect("offscreen rendering")
}

/// Asserts that a pixel differs from `expected` by at most 1/255 per channel.
#[track_caller]
pub(crate) fn assert_pixel(image: &Image, x: u32, y: u32, expected: [u8; 4]) {
    let actual = image.pixel(x, y);
    let close = actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= 1);
    assert!(
        close,
        "pixel ({x}, {y}) is {actual:?}, expected {expected:?}"
    );
}
