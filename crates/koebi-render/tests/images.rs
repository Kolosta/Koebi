//! Images and dot grids on screen.
//!
//! - Images show their texels one per pixel, linear light encoded to sRGB, gamma-encoded texels
//!   as they are.
//! - They take their opacity, show the part of the texture asked for, and cut their rounded
//!   corners; a removed image draws nothing.
//! - Dot grids draw their two levels on their lattice, whole pixels each, the finer one faded,
//!   at several zooms.
//! - A golden image holds them together.

mod common;

use common::assert_pixel;
use koebi_core::{Color, Extent, Rect, Vec2};
use koebi_gpu::{
    Buffer, BufferDesc, BufferTexels, BufferUsage, MemoryLocation, Texture, TextureDesc,
    TextureFormat, TextureRegion, TextureState, TextureUsage,
};
use koebi_render::{
    DotGrid, Fill, FillPrimitive, ImageContent, ImageId, ImagePrimitive, Renderer2D, Scene,
};
use koebi_testkit::{HeadlessGpu, Image, Tolerance, assert_golden, gpu_or_skip};

const BACKGROUND: Color = Color::from_rgba8(16, 32, 48, 255);

/// Anti-aliased edges may differ by a few units between drivers.
const CROSS_DRIVER: Tolerance = Tolerance::new(3, 8);

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Vec2::new(x, y), Vec2::new(width, height))
}

/// The 16-bit float nearest `value`, for values a half holds exactly or nearly (zero, or
/// between 2^-14 and 65504).
fn half(value: f32) -> u16 {
    if value == 0.0 {
        return 0;
    }
    let bits = value.to_bits();
    let sign = u16::try_from((bits >> 16) & 0x8000).expect("a sign bit");
    let exponent = i32::try_from((bits >> 23) & 0xff).expect("an exponent") - 127 + 15;
    assert!((1..31).contains(&exponent), "{value} is not a normal half");
    let mantissa = bits & 0x007f_ffff;
    // Round to nearest, ties to even.
    let shifted = mantissa >> 13;
    let rest = mantissa & 0x1fff;
    let rounded = shifted + u32::from(rest > 0x1000 || (rest == 0x1000 && shifted & 1 == 1));
    let packed = (u32::try_from(exponent).expect("a positive exponent") << 10) + rounded;
    sign | u16::try_from(packed).expect("a half")
}

/// The sRGB byte of a linear component.
fn encoded(linear: f32) -> u8 {
    let x = linear.clamp(0.0, 1.0);
    let encoded = if x <= 0.003_130_8 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    };
    // Between 0 and 255 by construction.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a byte"
    )]
    let byte = (encoded * 255.0).round() as u8;
    byte
}

/// A texture to show: its texels, row after row, and how they encode their colors.
struct Upload {
    format: TextureFormat,
    extent: Extent,
    bytes: Vec<u8>,
    content: ImageContent,
}

impl Upload {
    /// Linear light, premultiplied, in halves.
    fn linear(width: u32, height: u32, texels: &[[f32; 4]]) -> Self {
        let bytes = texels
            .iter()
            .flat_map(|texel| texel.map(half))
            .flat_map(u16::to_le_bytes)
            .collect();
        Self {
            format: TextureFormat::Rgba16Float,
            extent: Extent::new(width, height),
            bytes,
            content: ImageContent::Linear,
        }
    }

    /// Gamma-encoded, premultiplied, in bytes.
    fn encoded(width: u32, height: u32, texels: &[[u8; 4]]) -> Self {
        Self {
            format: TextureFormat::Rgba8Unorm,
            extent: Extent::new(width, height),
            bytes: texels.iter().flatten().copied().collect(),
            content: ImageContent::Encoded,
        }
    }
}

/// Renders the scene `scene` builds from the numbers of `uploads`, kept by the renderer, into
/// an RGBA8 image of `extent`. The texels are uploaded in the same submission, before the pass.
fn render(
    gpu: &HeadlessGpu,
    extent: Extent,
    uploads: &[Upload],
    scene: impl FnOnce(&mut Renderer2D, &[ImageId], &[&Texture]) -> Scene,
) -> Image {
    render_at(gpu, extent, 1.0, uploads, scene)
}

/// [`render`] at `scale` physical pixels per logical pixel.
fn render_at(
    gpu: &HeadlessGpu,
    extent: Extent,
    scale: f32,
    uploads: &[Upload],
    scene: impl FnOnce(&mut Renderer2D, &[ImageId], &[&Texture]) -> Scene,
) -> Image {
    let format = TextureFormat::Rgba8Unorm;
    let mut renderer = Renderer2D::new(&gpu.device, format).expect("renderer creation");
    let mut kept: Vec<(Texture, Buffer, u32)> = Vec::new();
    let mut ids = Vec::new();
    for upload in uploads {
        let texture = gpu
            .device
            .create_texture(&TextureDesc {
                label: "test image",
                extent: upload.extent,
                format: upload.format,
                usage: TextureUsage::SAMPLED | TextureUsage::TRANSFER_DST,
            })
            .expect("a texture");
        let mut staging = gpu
            .device
            .create_buffer(&BufferDesc {
                label: "test image staging",
                size: upload.bytes.len() as u64,
                usage: BufferUsage::TRANSFER_SRC,
                location: MemoryLocation::CpuToGpu,
            })
            .expect("a staging buffer");
        staging.write(0, &upload.bytes).expect("written");
        ids.push(
            renderer
                .add_image(&texture, upload.content)
                .expect("an image"),
        );
        let row = upload.extent.width * upload.format.bytes_per_pixel();
        kept.push((texture, staging, row));
    }
    let textures: Vec<&Texture> = kept.iter().map(|(texture, ..)| texture).collect();
    let scene = scene(&mut renderer, &ids, &textures);
    gpu.render_offscreen(extent, format, |encoder, target| {
        for (texture, staging, row) in &kept {
            encoder.texture_barrier(texture, TextureState::Undefined, TextureState::TransferDst);
            encoder
                .copy_buffer_to_texture(
                    staging,
                    BufferTexels {
                        offset: 0,
                        bytes_per_row: *row,
                    },
                    texture,
                    TextureRegion::whole(texture.extent()),
                )
                .expect("a copy");
            encoder.texture_barrier(texture, TextureState::TransferDst, TextureState::ShaderRead);
        }
        renderer
            .render(encoder, target, &scene, 0, scale)
            .expect("render");
    })
    .expect("offscreen rendering")
}

fn scene_of(primitives: &[ImagePrimitive]) -> Scene {
    let mut scene = Scene::new();
    scene.set_clear_color(BACKGROUND);
    for primitive in primitives {
        scene.push_image(primitive);
    }
    scene
}

#[test]
fn linear_images_show_their_texels_encoded_to_srgb_one_per_pixel() {
    let Some(gpu) = gpu_or_skip("linear_images_show_their_texels_encoded_to_srgb_one_per_pixel")
    else {
        return;
    };
    // Premultiplied linear light, every value exact in halves; the last texel half covered.
    let texels = [
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 1.0, 1.0, 1.0],
        [0.5, 0.25, 0.125, 1.0],
        [0.75, 0.0, 0.5, 1.0],
        [0.25, 0.5, 0.75, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
        [0.25, 0.125, 0.0, 0.5],
    ];
    let image = render(
        &gpu,
        Extent::new(8, 6),
        &[Upload::linear(4, 2, &texels)],
        |_, ids, _| scene_of(&[ImagePrimitive::new(ids[0], rect(2.0, 2.0, 4.0, 2.0))]),
    );
    let background = BACKGROUND.to_rgba8();
    for (index, texel) in texels.iter().enumerate() {
        let index = u32::try_from(index).expect("a few texels");
        let (x, y) = (2 + index % 4, 2 + index / 4);
        let alpha = texel[3];
        // Encoded straight, then over the background in the window's values.
        let over = |channel: usize| {
            let straight = texel[channel] / alpha;
            let color = f32::from(encoded(straight)) * alpha;
            let under = f32::from(background[channel]) * (1.0 - alpha);
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a byte"
            )]
            let byte = (color + under).round() as u8;
            byte
        };
        assert_pixel(&image, x, y, [over(0), over(1), over(2), 255], 1);
    }
    // Nothing around it.
    for (x, y) in [(1, 2), (6, 3), (3, 1), (4, 4)] {
        assert_pixel(&image, x, y, background, 0);
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn encoded_images_show_their_texels_as_they_are_with_their_opacity() {
    let Some(gpu) = gpu_or_skip("encoded_images_show_their_texels_as_they_are_with_their_opacity")
    else {
        return;
    };
    let texels = [
        [200, 100, 50, 255],
        [10, 220, 140, 255],
        [255, 255, 255, 255],
        [60, 30, 0, 128],
    ];
    let image = render(
        &gpu,
        Extent::new(8, 4),
        &[Upload::encoded(2, 2, &texels)],
        |_, ids, _| {
            scene_of(&[
                ImagePrimitive::new(ids[0], rect(0.0, 0.0, 2.0, 2.0)),
                ImagePrimitive::new(ids[0], rect(4.0, 0.0, 2.0, 2.0)).with_opacity(0.5),
            ])
        },
    );
    let background = BACKGROUND.to_rgba8();
    for (index, texel) in texels.iter().enumerate() {
        let index = u32::try_from(index).expect("a few texels");
        let (x, y) = (index % 2, index / 2);
        let blend = |opacity: f32| -> [u8; 4] {
            let alpha = f32::from(texel[3]) / 255.0 * opacity;
            core::array::from_fn(|channel| {
                if channel == 3 {
                    return 255;
                }
                let mixed = f32::from(texel[channel]) * opacity
                    + f32::from(background[channel]) * (1.0 - alpha);
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a byte"
                )]
                let byte = mixed.round() as u8;
                byte
            })
        };
        assert_pixel(&image, x, y, blend(1.0), 1);
        assert_pixel(&image, x + 4, y, blend(0.5), 1);
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn a_part_of_an_image_fills_its_rectangle_and_rounded_corners_cut_it() {
    let Some(gpu) =
        gpu_or_skip("a_part_of_an_image_fills_its_rectangle_and_rounded_corners_cut_it")
    else {
        return;
    };
    let (red, green) = ([255, 0, 0, 255], [0, 255, 0, 255]);
    let halves = [red, red, green, green];
    let solid = [[200, 100, 50, 255]; 16 * 16];
    let image = render(
        &gpu,
        Extent::new(40, 20),
        &[
            Upload::encoded(4, 1, &halves),
            Upload::encoded(16, 16, &solid),
        ],
        |_, ids, _| {
            scene_of(&[
                // The right half, two texels in two pixels.
                ImagePrimitive::new(ids[0], rect(0.0, 0.0, 2.0, 1.0))
                    .with_uv(Rect::from_min_max(Vec2::new(0.5, 0.0), Vec2::new(1.0, 1.0))),
                ImagePrimitive::new(ids[1], rect(20.0, 2.0, 16.0, 16.0)).with_corner_radius(6.0),
            ])
        },
    );
    assert_pixel(&image, 0, 0, green, 1);
    assert_pixel(&image, 1, 0, green, 1);
    // The corner cut, its middle kept.
    assert_pixel(&image, 20, 2, BACKGROUND.to_rgba8(), 0);
    assert_pixel(&image, 28, 10, solid[0], 1);
    // On the arc: 6.36 pixels from the corner's center, about a seventh covered.
    let [r, ..] = image.pixel(21, 3);
    assert!(r > BACKGROUND.to_rgba8()[0] && r < solid[0][0], "{r}");
    gpu.assert_no_validation_errors();
}

#[test]
fn removed_images_draw_nothing_and_replaced_ones_show_their_new_texture() {
    let Some(gpu) =
        gpu_or_skip("removed_images_draw_nothing_and_replaced_ones_show_their_new_texture")
    else {
        return;
    };
    let (red, green) = ([255, 0, 0, 255], [0, 255, 0, 255]);
    let image = render(
        &gpu,
        Extent::new(8, 2),
        &[
            Upload::encoded(1, 1, &[red]),
            Upload::encoded(1, 1, &[green]),
            Upload::linear(1, 1, &[[0.0, 0.0, 1.0, 1.0]]),
        ],
        |renderer, ids, textures| {
            assert!(renderer.remove_image(ids[0]));
            assert!(!renderer.remove_image(ids[0]));
            assert_eq!(renderer.image_count(), 2);
            // A removed image, or a texture of another content, cannot be replaced: nothing
            // changes.
            assert!(
                renderer
                    .replace_image(ids[0], textures[1], ImageContent::Encoded)
                    .is_err()
            );
            assert!(
                renderer
                    .replace_image(ids[1], textures[2], ImageContent::Encoded)
                    .is_err()
            );
            assert_eq!(renderer.image_count(), 2);
            // The linear blue for the green, under the same number.
            renderer
                .replace_image(ids[1], textures[2], ImageContent::Linear)
                .expect("replaced");
            assert_eq!(renderer.image_count(), 2);
            scene_of(&[
                ImagePrimitive::new(ids[0], rect(0.0, 0.0, 2.0, 2.0)),
                ImagePrimitive::new(ids[1], rect(4.0, 0.0, 2.0, 2.0)),
            ])
        },
    );
    assert_pixel(&image, 0, 0, BACKGROUND.to_rgba8(), 0);
    assert_pixel(&image, 4, 0, [0, 0, 255, 255], 1);
    gpu.assert_no_validation_errors();
}

#[test]
fn images_take_textures_only_of_the_formats_their_content_holds() {
    let Some(gpu) = gpu_or_skip("images_take_textures_only_of_the_formats_their_content_holds")
    else {
        return;
    };
    let mut renderer =
        Renderer2D::new(&gpu.device, TextureFormat::Rgba8Unorm).expect("renderer creation");
    let texture = |format, usage| {
        gpu.device
            .create_texture(&TextureDesc {
                label: "test image",
                extent: Extent::new(2, 2),
                format,
                usage,
            })
            .expect("a texture")
    };
    let sampled = TextureUsage::SAMPLED;
    // Never filtered on every device; gamma-encoded texels in a float texture; not sampled.
    let unfiltered = texture(TextureFormat::Rgba32Float, sampled);
    let float = texture(TextureFormat::Rgba16Float, sampled);
    let unsampled = texture(TextureFormat::Rgba8Unorm, TextureUsage::TRANSFER_DST);
    assert!(
        renderer
            .add_image(&unfiltered, ImageContent::Linear)
            .is_err()
    );
    assert!(renderer.add_image(&float, ImageContent::Encoded).is_err());
    assert!(
        renderer
            .add_image(&unsampled, ImageContent::Encoded)
            .is_err()
    );
    assert_eq!(renderer.image_count(), 0);
    assert!(renderer.add_image(&float, ImageContent::Linear).is_ok());
    assert_eq!(renderer.image_count(), 1);
    gpu.assert_no_validation_errors();
}

#[test]
fn images_and_dots_keep_their_pixels_at_twice_the_scale() {
    let Some(gpu) = gpu_or_skip("images_and_dots_keep_their_pixels_at_twice_the_scale") else {
        return;
    };
    let texels = [
        [200, 100, 50, 255],
        [10, 220, 140, 255],
        [255, 255, 255, 255],
        [60, 30, 0, 255],
    ];
    // An image of 2×2 texels in one logical pixel, and one-unit dots 8 units apart: at twice
    // the scale, one texel per physical pixel and dots two pixels wide.
    let image = render_at(
        &gpu,
        Extent::new(48, 32),
        2.0,
        &[Upload::encoded(2, 2, &texels)],
        |_, ids, _| {
            let mut scene = scene_of(&[ImagePrimitive::new(ids[0], rect(1.0, 1.0, 1.0, 1.0))]);
            scene.push_fill(&FillPrimitive::new(
                rect(0.0, 6.0, 24.0, 10.0),
                Fill::Dots {
                    origin: Vec2::new(4.25, 8.25),
                    spacing: 8.0,
                    subdivisions: 2,
                    size: 1.0,
                    color: Color::WHITE,
                    fine: 0.0,
                },
            ));
            scene
        },
    );
    for (index, texel) in texels.iter().enumerate() {
        let index = u32::try_from(index).expect("a few texels");
        assert_pixel(&image, 2 + index % 2, 2 + index / 2, *texel, 1);
    }
    // At (8.5, 16.5) physical pixels, on the corner (9, 17): pixels 8 and 9, 16 and 17; the
    // next dot 16 pixels further, on (25, 17).
    for corner in [9, 25] {
        let pixel = image.pixel(corner - 1, 16);
        assert!(pixel[0] > BACKGROUND.to_rgba8()[0] + 100, "{pixel:?}");
        for (x, y) in [(corner, 16), (corner - 1, 17), (corner, 17)] {
            assert_eq!(image.pixel(x, y), pixel, "({x}, {y})");
        }
        for (x, y) in [(corner + 1, 16), (corner - 2, 17), (corner, 18)] {
            assert_pixel(&image, x, y, BACKGROUND.to_rgba8(), 0);
        }
    }
    gpu.assert_no_validation_errors();
}

/// A scene of one dot grid over `area`.
fn dots(area: Rect, fill: Fill) -> Scene {
    let mut scene = Scene::new();
    scene.set_clear_color(BACKGROUND);
    scene.push_fill(&FillPrimitive::new(area, fill));
    scene
}

#[test]
fn dot_grids_draw_both_levels_on_their_lattice_on_whole_pixels() {
    let Some(gpu) = gpu_or_skip("dot_grids_draw_both_levels_on_their_lattice_on_whole_pixels")
    else {
        return;
    };
    let grid = |size: f32, fine: f32| Fill::Dots {
        origin: Vec2::new(4.3, 4.2),
        spacing: 8.0,
        subdivisions: 2,
        size,
        color: Color::WHITE,
        fine,
    };
    let area = rect(0.0, 0.0, 32.0, 32.0);
    let background = BACKGROUND.to_rgba8();
    let white = Color::WHITE.to_rgba8();
    // One pixel each, on the pixel holding the lattice point; the finer level hidden.
    let image = common::render(&gpu, Extent::new(32, 32), 1.0, &dots(area, grid(1.0, 0.0)));
    for (x, y) in [(4, 4), (12, 4), (4, 12), (20, 28)] {
        assert_pixel(&image, x, y, white, 1);
    }
    for (x, y) in [(5, 4), (3, 4), (8, 8), (8, 4), (16, 16)] {
        assert_pixel(&image, x, y, background, 0);
    }
    // The finer level at half strength, between the coarse dots.
    let image = common::render(&gpu, Extent::new(32, 32), 1.0, &dots(area, grid(1.0, 0.5)));
    let half: [u8; 4] = core::array::from_fn(|channel| {
        if channel == 3 {
            255
        } else {
            // A mean of two bytes: within one.
            u8::midpoint(white[channel], background[channel])
        }
    });
    for (x, y) in [(8, 8), (8, 4), (4, 8), (16, 12)] {
        assert_pixel(&image, x, y, half, 1);
    }
    assert_pixel(&image, 12, 12, white, 1);
    // Two pixels wide: on the pixel corner nearest the lattice point, four pixels alike.
    let image = common::render(&gpu, Extent::new(32, 32), 1.0, &dots(area, grid(2.0, 0.0)));
    let corner = image.pixel(3, 3);
    for (x, y) in [(4, 3), (3, 4), (4, 4)] {
        assert_eq!(image.pixel(x, y), corner, "({x}, {y})");
    }
    assert!(corner[0] > background[0] + 100, "{corner:?}");
    for (x, y) in [(2, 3), (5, 4), (4, 5)] {
        assert_pixel(&image, x, y, background, 0);
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn dot_grids_of_a_view_keep_their_dots_on_the_canvas_lattice_at_every_zoom() {
    let Some(gpu) =
        gpu_or_skip("dot_grids_of_a_view_keep_their_dots_on_the_canvas_lattice_at_every_zoom")
    else {
        return;
    };
    let grid = DotGrid {
        step: 1.0,
        subdivisions: 4,
        min_distance: 12.0,
        size: 1.0,
        color: Color::WHITE,
    };
    let area = rect(0.0, 0.0, 64.0, 64.0);
    let white = Color::WHITE.to_rgba8();
    // The canvas's origin far away, on a pixel center: its dots too.
    let origin = [1.0e7 + 0.5, -2.0e7 + 0.5];
    for zoom in [0.37, 3.0, 12.0, 47.5, 1.0e4] {
        let levels = grid.levels(zoom).expect("a zoom");
        let fill = grid.fill(area, origin, zoom).expect("a grid");
        let mut scene = Scene::new();
        scene.set_clear_color(BACKGROUND);
        scene.push_fill(&fill);
        let image = common::render(&gpu, Extent::new(64, 64), 1.0, &scene);
        // The first coarse dot inside the area, on the canvas's lattice; one lying on a pixel
        // edge goes to the pixel after it, as the shader rounds ties.
        let first = |origin: f64| {
            let near = origin.rem_euclid(levels.on_screen) + 1.0 / 256.0;
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a pixel of the area"
            )]
            let pixel = near.floor() as u32;
            pixel
        };
        let (x, y) = (first(origin[0]), first(origin[1]));
        assert_pixel(&image, x, y, white, 2);
        gpu.assert_no_validation_errors();
    }
}

#[test]
fn images_and_dots_match_golden() {
    let Some(gpu) = gpu_or_skip("images_and_dots_match_golden") else {
        return;
    };
    // A ramp of linear light across, opaque, and one fading out downwards.
    let (width, height) = (48, 24);
    let mut ramp = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let across = f32::from(u8::try_from(x).expect("a column")) / 47.0;
            let down = 1.0 - f32::from(u8::try_from(y).expect("a row")) / 23.0;
            let alpha = if y < height / 2 { 1.0 } else { down.max(0.25) };
            // Values a half holds: quantized to sixteenths.
            let q = |value: f32| ((value * 16.0).round() / 16.0).max(1.0 / 16.0);
            ramp.push([
                q(across) * alpha,
                q(0.5) * alpha,
                q(1.0 - across) * alpha,
                q(alpha),
            ]);
        }
    }
    let grid = DotGrid {
        step: 1.0,
        subdivisions: 4,
        min_distance: 12.0,
        size: 2.0,
        color: Color::from_rgba8(255, 255, 255, 96),
    };
    let image = render(
        &gpu,
        Extent::new(160, 96),
        &[Upload::linear(width, height, &ramp)],
        |_, ids, _| {
            let mut scene = Scene::new();
            scene.set_clear_color(BACKGROUND);
            let area = rect(0.0, 0.0, 160.0, 96.0);
            scene.push_fill(&grid.fill(area, [7.5, 5.5], 30.0).expect("a grid"));
            scene.push_image(&ImagePrimitive::new(ids[0], rect(8.0, 8.0, 48.0, 24.0)));
            scene.push_image(
                &ImagePrimitive::new(ids[0], rect(64.0, 8.0, 88.0, 44.0)).with_corner_radius(10.0),
            );
            scene.push_image(
                &ImagePrimitive::new(ids[0], rect(8.0, 48.0, 48.0, 40.0))
                    .with_uv(Rect::from_min_max(
                        Vec2::new(0.25, 0.0),
                        Vec2::new(0.75, 1.0),
                    ))
                    .with_opacity(0.6),
            );
            scene
        },
    );
    assert_golden!(image, "images_and_dots", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}
