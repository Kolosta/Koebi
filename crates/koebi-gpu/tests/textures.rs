//! Integration tests of sampled textures: uploads, partial updates, samplers and bind groups.
//!
//! Like `headless.rs`, each test skips without a usable GPU unless `KOEBI_REQUIRE_GPU=1`, and
//! asserts that the validation layers (synchronization validation included) stayed silent.

use koebi_core::Extent;
use koebi_gpu::{
    AddressMode, BindGroup, BindGroupDesc, BindGroupLayout, BindGroupLayoutDesc, BindingResource,
    BindingType, BlendMode, Buffer, BufferDesc, BufferTexels, BufferUsage, ColorTargetDesc, Device,
    Error, FilterMode, LoadOp, MemoryLocation, PrimitiveTopology, RenderPipeline,
    RenderPipelineDesc, Sampler, SamplerDesc, ShaderStage, Texture, TextureDesc, TextureFormat,
    TextureRegion, TextureState, TextureUsage,
};
use koebi_testkit::{HeadlessGpu, gpu_or_skip};

static TEXTURED_VERT: &[u32] =
    koebi_gpu::include_spirv!(include_bytes!("shaders/spirv/textured.vert.spv"));
static TEXTURED_FRAG: &[u32] =
    koebi_gpu::include_spirv!(include_bytes!("shaders/spirv/textured.frag.spv"));

fn texture(device: &Device, extent: Extent, format: TextureFormat) -> Texture {
    device
        .create_texture(&TextureDesc {
            label: "test texture",
            extent,
            format,
            usage: TextureUsage::SAMPLED | TextureUsage::TRANSFER_DST | TextureUsage::TRANSFER_SRC,
        })
        .expect("a small texture can be created")
}

fn staging(device: &Device, bytes: &[u8]) -> Buffer {
    let mut buffer = device
        .create_buffer(&BufferDesc {
            label: "test staging",
            size: bytes.len() as u64,
            usage: BufferUsage::TRANSFER_SRC,
            location: MemoryLocation::CpuToGpu,
        })
        .expect("a small staging buffer can be created");
    buffer
        .write(0, bytes)
        .expect("the staging buffer holds the bytes");
    buffer
}

/// The texels of `texture`, tightly packed; the texture must be in `ShaderRead`.
fn read_back(gpu: &HeadlessGpu, texture: &Texture) -> Vec<u8> {
    let extent = texture.extent();
    let size = extent.area() * u64::from(texture.format().bytes_per_pixel());
    let readback = gpu
        .device
        .create_buffer(&BufferDesc {
            label: "test readback",
            size,
            usage: BufferUsage::TRANSFER_DST,
            location: MemoryLocation::GpuToCpu,
        })
        .expect("a small readback buffer can be created");
    gpu.device
        .submit_and_wait(|encoder| {
            encoder.texture_barrier(texture, TextureState::ShaderRead, TextureState::TransferSrc);
            encoder.copy_texture_to_buffer(texture, &readback)?;
            encoder.texture_barrier(texture, TextureState::TransferSrc, TextureState::ShaderRead);
            Ok::<_, Error>(())
        })
        .expect("the readback submission completes")
        .expect("the readback copy is valid");
    let mut bytes = vec![0_u8; usize::try_from(size).expect("a small texture fits in memory")];
    readback
        .read(0, &mut bytes)
        .expect("the readback buffer holds the texture");
    bytes
}

fn sampled_layout(device: &Device) -> BindGroupLayout {
    device
        .create_bind_group_layout(&BindGroupLayoutDesc {
            label: "test layout",
            entries: &[BindingType::SampledTexture],
        })
        .expect("a one-texture layout can be created")
}

fn nearest(device: &Device) -> Sampler {
    device
        .create_sampler(&SamplerDesc {
            label: "test nearest",
            filter: FilterMode::Nearest,
            address_mode: AddressMode::ClampToEdge,
        })
        .expect("a sampler can be created")
}

fn textured_pipeline(device: &Device, layout: &BindGroupLayout) -> RenderPipeline {
    let vertex = device
        .create_shader_module("textured.vert", TEXTURED_VERT)
        .expect("the test vertex shader is valid SPIR-V");
    let fragment = device
        .create_shader_module("textured.frag", TEXTURED_FRAG)
        .expect("the test fragment shader is valid SPIR-V");
    device
        .create_render_pipeline(&RenderPipelineDesc {
            label: "test textured",
            vertex: ShaderStage {
                module: &vertex,
                entry_point: c"main",
            },
            fragment: ShaderStage {
                module: &fragment,
                entry_point: c"main",
            },
            vertex_buffers: &[],
            topology: PrimitiveTopology::TriangleStrip,
            color_targets: &[ColorTargetDesc {
                format: TextureFormat::Rgba8Unorm,
                blend: BlendMode::Opaque,
            }],
            push_constant_size: 0,
            bind_group_layouts: &[layout],
        })
        .expect("the test pipeline can be created")
}

/// Uploads `bytes` (tightly packed) as the whole of `texture`, left in `ShaderRead`.
fn upload_whole(gpu: &HeadlessGpu, texture: &Texture, bytes: &[u8]) {
    let buffer = staging(&gpu.device, bytes);
    let extent = texture.extent();
    gpu.device
        .submit_and_wait(|encoder| {
            encoder.texture_barrier(texture, TextureState::Undefined, TextureState::TransferDst);
            encoder.copy_buffer_to_texture(
                &buffer,
                BufferTexels {
                    offset: 0,
                    bytes_per_row: extent.width * texture.format().bytes_per_pixel(),
                },
                texture,
                TextureRegion::whole(extent),
            )?;
            encoder.texture_barrier(texture, TextureState::TransferDst, TextureState::ShaderRead);
            Ok::<_, Error>(())
        })
        .expect("the upload submission completes")
        .expect("the upload copy is valid");
}

#[test]
fn uploads_round_trip_from_padded_rows_at_an_offset() {
    let Some(gpu) = gpu_or_skip("uploads_round_trip_from_padded_rows_at_an_offset") else {
        return;
    };
    let extent = Extent::new(5, 3);
    let atlas = texture(&gpu.device, extent, TextureFormat::R8Unorm);
    // Rows of 8 bytes for 5 texels, starting 4 bytes into the buffer.
    let mut bytes = vec![0xEE_u8; 4 + 8 * 3];
    for y in 0..3 {
        for x in 0..5 {
            bytes[4 + y * 8 + x] = u8::try_from(y * 10 + x).unwrap();
        }
    }
    let buffer = staging(&gpu.device, &bytes);
    gpu.device
        .submit_and_wait(|encoder| {
            encoder.texture_barrier(&atlas, TextureState::Undefined, TextureState::TransferDst);
            encoder.copy_buffer_to_texture(
                &buffer,
                BufferTexels {
                    offset: 4,
                    bytes_per_row: 8,
                },
                &atlas,
                TextureRegion::whole(extent),
            )?;
            encoder.texture_barrier(&atlas, TextureState::TransferDst, TextureState::ShaderRead);
            Ok::<_, Error>(())
        })
        .unwrap()
        .unwrap();
    assert_eq!(
        read_back(&gpu, &atlas),
        [0, 1, 2, 3, 4, 10, 11, 12, 13, 14, 20, 21, 22, 23, 24]
    );
    gpu.assert_no_validation_errors();
}

#[test]
fn partial_updates_keep_the_rest_of_the_texture() {
    let Some(gpu) = gpu_or_skip("partial_updates_keep_the_rest_of_the_texture") else {
        return;
    };
    let extent = Extent::new(4, 4);
    let atlas = texture(&gpu.device, extent, TextureFormat::R8Unorm);
    gpu.device
        .submit_and_wait(|encoder| {
            encoder.texture_barrier(&atlas, TextureState::Undefined, TextureState::TransferDst);
            encoder.clear_texture(&atlas, [0.0; 4])?;
            encoder.texture_barrier(&atlas, TextureState::TransferDst, TextureState::ShaderRead);
            Ok::<_, Error>(())
        })
        .unwrap()
        .unwrap();
    // Two later updates, as an atlas receives new glyphs frame after frame: each goes from
    // `ShaderRead` back to `TransferDst` and must preserve what was there.
    let updates = [
        (TextureRegion::new(1, 1, 2, 1), [0xAA_u8, 0xBB, 0, 0]),
        (TextureRegion::new(3, 2, 1, 2), [0xCC_u8, 0, 0, 0]),
    ];
    for (region, data) in updates {
        // The second update reads its column from rows of 4 bytes: one texel each.
        let rows = if region.width == 1 {
            [data[0], 0, 0, 0, 0xDD, 0, 0, 0]
        } else {
            [data[0], data[1], data[2], data[3], 0, 0, 0, 0]
        };
        let buffer = staging(&gpu.device, &rows);
        gpu.device
            .submit_and_wait(|encoder| {
                encoder.texture_barrier(
                    &atlas,
                    TextureState::ShaderRead,
                    TextureState::TransferDst,
                );
                encoder.copy_buffer_to_texture(
                    &buffer,
                    BufferTexels {
                        offset: 0,
                        bytes_per_row: 4,
                    },
                    &atlas,
                    region,
                )?;
                encoder.texture_barrier(
                    &atlas,
                    TextureState::TransferDst,
                    TextureState::ShaderRead,
                );
                Ok::<_, Error>(())
            })
            .unwrap()
            .unwrap();
    }
    #[rustfmt::skip]
    let expected = [
        0, 0, 0, 0,
        0, 0xAA, 0xBB, 0,
        0, 0, 0, 0xCC,
        0, 0, 0, 0xDD,
    ];
    assert_eq!(read_back(&gpu, &atlas), expected);
    gpu.assert_no_validation_errors();
}

#[test]
fn copies_are_validated_before_reaching_vulkan() {
    let Some(gpu) = gpu_or_skip("copies_are_validated_before_reaching_vulkan") else {
        return;
    };
    let rgba = texture(&gpu.device, Extent::new(4, 4), TextureFormat::Rgba8Unorm);
    let buffer = staging(&gpu.device, &[0; 64]);
    let rows = |offset, bytes_per_row| BufferTexels {
        offset,
        bytes_per_row,
    };
    let refused = [
        // Outside the texture, or empty.
        (rows(0, 16), TextureRegion::new(2, 0, 3, 1)),
        (rows(0, 16), TextureRegion::new(0, 0, 0, 1)),
        // An offset that is not a multiple of the texel size (and of 4).
        (rows(2, 16), TextureRegion::new(0, 0, 1, 1)),
        // Rows shorter than the region, or not made of whole texels.
        (rows(0, 8), TextureRegion::new(0, 0, 4, 1)),
        (rows(0, 18), TextureRegion::new(0, 0, 4, 1)),
        // Beyond the end of the buffer.
        (rows(4, 16), TextureRegion::new(0, 0, 4, 4)),
    ];
    let without_usage = gpu
        .device
        .create_texture(&TextureDesc {
            label: "sampled only",
            extent: Extent::new(4, 4),
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsage::SAMPLED,
        })
        .unwrap();
    gpu.device
        .submit_and_wait(|encoder| {
            for (texels, region) in refused {
                let copy = encoder.copy_buffer_to_texture(&buffer, texels, &rgba, region);
                assert!(
                    matches!(copy, Err(Error::InvalidArgument(_))),
                    "{texels:?} {region:?}"
                );
            }
            let region = TextureRegion::whole(without_usage.extent());
            let copy = encoder.copy_buffer_to_texture(&buffer, rows(0, 16), &without_usage, region);
            assert!(matches!(copy, Err(Error::InvalidArgument(_))));
            let clear = encoder.clear_texture(&without_usage, [0.0; 4]);
            assert!(matches!(clear, Err(Error::InvalidArgument(_))));
        })
        .unwrap();
    gpu.assert_no_validation_errors();
}

#[test]
fn sampled_textures_draw_their_exact_texels() {
    let Some(gpu) = gpu_or_skip("sampled_textures_draw_their_exact_texels") else {
        return;
    };
    let extent = Extent::new(4, 2);
    let source = texture(&gpu.device, extent, TextureFormat::Rgba8Unorm);
    let texels: Vec<u8> = (0..8_u8)
        .flat_map(|i| [i * 30, 255 - i * 30, i * 7, 255])
        .collect();
    upload_whole(&gpu, &source, &texels);
    let layout = sampled_layout(&gpu.device);
    let sampler = nearest(&gpu.device);
    let group = gpu
        .device
        .create_bind_group(&BindGroupDesc {
            label: "test group",
            layout: &layout,
            entries: &[BindingResource::SampledTexture {
                texture: &source,
                sampler: &sampler,
            }],
        })
        .unwrap();
    let pipeline = textured_pipeline(&gpu.device, &layout);
    let image = gpu
        .render_offscreen(extent, TextureFormat::Rgba8Unorm, |encoder, target| {
            let mut pass = encoder.begin_render_pass(target, LoadOp::DontCare);
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(&pipeline, 0, &group).unwrap();
            pass.draw(0..4, 0..1);
        })
        .unwrap();
    for y in 0..2 {
        for x in 0..4 {
            let i = usize::try_from(y * 4 + x).unwrap() * 4;
            assert_eq!(image.pixel(x, y), texels[i..i + 4], "pixel ({x}, {y})");
        }
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn bind_groups_keep_their_resources_alive() {
    let Some(gpu) = gpu_or_skip("bind_groups_keep_their_resources_alive") else {
        return;
    };
    let extent = Extent::new(2, 2);
    let layout = sampled_layout(&gpu.device);
    let group = {
        let source = texture(&gpu.device, extent, TextureFormat::Rgba8Unorm);
        upload_whole(&gpu, &source, &[200; 16]);
        let sampler = nearest(&gpu.device);
        gpu.device
            .create_bind_group(&BindGroupDesc {
                label: "test group",
                layout: &layout,
                entries: &[BindingResource::SampledTexture {
                    texture: &source,
                    sampler: &sampler,
                }],
            })
            .unwrap()
        // The texture and the sampler are dropped here, while the group still uses them.
    };
    // Even with every completed submission's garbage collected.
    gpu.device.wait_idle().unwrap();
    let pipeline = textured_pipeline(&gpu.device, &layout);
    drop(layout);
    let image = gpu
        .render_offscreen(extent, TextureFormat::Rgba8Unorm, |encoder, target| {
            let mut pass = encoder.begin_render_pass(target, LoadOp::DontCare);
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(&pipeline, 0, &group).unwrap();
            pass.draw(0..4, 0..1);
        })
        .unwrap();
    assert_eq!(image.pixel(1, 1), [200; 4]);
    drop(group);
    gpu.device.wait_idle().unwrap();
    gpu.assert_no_validation_errors();
}

#[test]
fn mismatched_bind_groups_are_rejected() {
    let Some(gpu) = gpu_or_skip("mismatched_bind_groups_are_rejected") else {
        return;
    };
    let empty = gpu.device.create_bind_group_layout(&BindGroupLayoutDesc {
        label: "empty",
        entries: &[],
    });
    assert!(matches!(empty, Err(Error::InvalidArgument(_))));

    let layout = sampled_layout(&gpu.device);
    let source = texture(&gpu.device, Extent::new(2, 2), TextureFormat::Rgba8Unorm);
    let sampler = nearest(&gpu.device);
    let resource = BindingResource::SampledTexture {
        texture: &source,
        sampler: &sampler,
    };
    let too_many = gpu.device.create_bind_group(&BindGroupDesc {
        label: "too many",
        layout: &layout,
        entries: &[resource, resource],
    });
    assert!(matches!(too_many, Err(Error::InvalidArgument(_))));
    let target_only = gpu
        .device
        .create_texture(&TextureDesc {
            label: "not sampled",
            extent: Extent::new(2, 2),
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsage::COLOR_ATTACHMENT,
        })
        .unwrap();
    let not_sampled = gpu.device.create_bind_group(&BindGroupDesc {
        label: "not sampled",
        layout: &layout,
        entries: &[BindingResource::SampledTexture {
            texture: &target_only,
            sampler: &sampler,
        }],
    });
    assert!(matches!(not_sampled, Err(Error::InvalidArgument(_))));

    // A group of two textures cannot stand for the pipeline's group of one, and the pipeline
    // has no second group.
    let pair_layout = gpu
        .device
        .create_bind_group_layout(&BindGroupLayoutDesc {
            label: "pair",
            entries: &[BindingType::SampledTexture, BindingType::SampledTexture],
        })
        .unwrap();
    let pair: BindGroup = gpu
        .device
        .create_bind_group(&BindGroupDesc {
            label: "pair",
            layout: &pair_layout,
            entries: &[resource, resource],
        })
        .unwrap();
    let single = gpu
        .device
        .create_bind_group(&BindGroupDesc {
            label: "single",
            layout: &layout,
            entries: &[resource],
        })
        .unwrap();
    let pipeline = textured_pipeline(&gpu.device, &layout);
    gpu.render_offscreen(
        Extent::new(2, 2),
        TextureFormat::Rgba8Unorm,
        |encoder, target| {
            let mut pass = encoder.begin_render_pass(target, LoadOp::Clear([0.0; 4]));
            pass.set_pipeline(&pipeline);
            assert!(matches!(
                pass.set_bind_group(&pipeline, 0, &pair),
                Err(Error::InvalidArgument(_))
            ));
            assert!(matches!(
                pass.set_bind_group(&pipeline, 1, &single),
                Err(Error::InvalidArgument(_))
            ));
        },
    )
    .unwrap();
    gpu.assert_no_validation_errors();
}

#[test]
fn linear_samplers_blend_neighbouring_texels() {
    let Some(gpu) = gpu_or_skip("linear_samplers_blend_neighbouring_texels") else {
        return;
    };
    // Two texels drawn over four pixels: the inner pixels sample a quarter of the way between
    // the texel centres, the outer ones clamp to the edge texels.
    let source = texture(&gpu.device, Extent::new(2, 1), TextureFormat::Rgba8Unorm);
    upload_whole(&gpu, &source, &[0, 0, 0, 255, 200, 200, 200, 255]);
    let layout = sampled_layout(&gpu.device);
    let sampler = gpu
        .device
        .create_sampler(&SamplerDesc {
            label: "test linear",
            filter: FilterMode::Linear,
            address_mode: AddressMode::ClampToEdge,
        })
        .unwrap();
    let group = gpu
        .device
        .create_bind_group(&BindGroupDesc {
            label: "test group",
            layout: &layout,
            entries: &[BindingResource::SampledTexture {
                texture: &source,
                sampler: &sampler,
            }],
        })
        .unwrap();
    let pipeline = textured_pipeline(&gpu.device, &layout);
    let image = gpu
        .render_offscreen(
            Extent::new(4, 1),
            TextureFormat::Rgba8Unorm,
            |encoder, target| {
                let mut pass = encoder.begin_render_pass(target, LoadOp::DontCare);
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(&pipeline, 0, &group).unwrap();
                pass.draw(0..4, 0..1);
            },
        )
        .unwrap();
    let red: Vec<i32> = (0..4).map(|x| i32::from(image.pixel(x, 0)[0])).collect();
    for (actual, expected) in red.iter().zip([0, 50, 150, 200]) {
        // Filtering precision is implementation-defined: allow a couple of steps.
        assert!((actual - expected).abs() <= 2, "{red:?}");
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn a_region_of_a_texture_reads_back_row_by_row() {
    let Some(gpu) = gpu_or_skip("a_region_of_a_texture_reads_back_row_by_row") else {
        return;
    };
    // A 4×4 texture whose texel (x, y) holds x and y.
    let extent = Extent::new(4, 4);
    let texture = texture(&gpu.device, extent, TextureFormat::Rgba8Unorm);
    let bytes: Vec<u8> = (0..4_u8)
        .flat_map(|y| (0..4_u8).flat_map(move |x| [x, y, 7, 255]))
        .collect();
    upload_whole(&gpu, &texture, &bytes);
    let readback = gpu
        .device
        .create_buffer(&BufferDesc {
            label: "region readback",
            size: 2 * 3 * 4,
            usage: BufferUsage::TRANSFER_DST,
            location: MemoryLocation::GpuToCpu,
        })
        .unwrap();
    gpu.device
        .submit_and_wait(|encoder| {
            encoder.texture_barrier(
                &texture,
                TextureState::ShaderRead,
                TextureState::TransferSrc,
            );
            // Outside the texture or empty: refused before reaching Vulkan.
            for region in [
                TextureRegion::new(3, 0, 2, 1),
                TextureRegion::new(0, 0, 0, 1),
            ] {
                let copy = encoder.copy_texture_region_to_buffer(&texture, region, &readback);
                assert!(matches!(copy, Err(Error::InvalidArgument(_))), "{region:?}");
            }
            encoder.copy_texture_region_to_buffer(
                &texture,
                TextureRegion::new(1, 2, 2, 2),
                &readback,
            )?;
            encoder.texture_barrier(
                &texture,
                TextureState::TransferSrc,
                TextureState::ShaderRead,
            );
            Ok::<_, Error>(())
        })
        .expect("the readback submission completes")
        .expect("the region copy is valid");
    let mut texels = [[0_u8; 4]; 4];
    readback.read(0, &mut texels).unwrap();
    assert_eq!(
        texels,
        [
            [1, 2, 7, 255],
            [2, 2, 7, 255],
            [1, 3, 7, 255],
            [2, 3, 7, 255]
        ]
    );
    gpu.assert_no_validation_errors();
}

#[test]
fn float_data_textures_are_sampled_never_rendered_to() {
    let Some(gpu) = gpu_or_skip("float_data_textures_are_sampled_never_rendered_to") else {
        return;
    };
    // Sampled and copied to: the icon set's data texture.
    let data = gpu
        .device
        .create_texture(&TextureDesc {
            label: "data",
            extent: Extent::new(16, 2),
            format: TextureFormat::Rgba32Float,
            usage: TextureUsage::SAMPLED | TextureUsage::TRANSFER_DST,
        })
        .expect("a sampled float texture can be created");
    drop(data);
    // Vulkan does not require rendering to the format, or blending it.
    let target = gpu.device.create_texture(&TextureDesc {
        label: "float target",
        extent: Extent::new(16, 2),
        format: TextureFormat::Rgba32Float,
        usage: TextureUsage::COLOR_ATTACHMENT,
    });
    assert!(matches!(target, Err(Error::InvalidArgument(_))));
    let vertex = gpu
        .device
        .create_shader_module("textured.vert", TEXTURED_VERT)
        .expect("the test vertex shader is valid SPIR-V");
    let fragment = gpu
        .device
        .create_shader_module("textured.frag", TEXTURED_FRAG)
        .expect("the test fragment shader is valid SPIR-V");
    let layout = sampled_layout(&gpu.device);
    let pipeline = gpu.device.create_render_pipeline(&RenderPipelineDesc {
        label: "float target",
        vertex: ShaderStage {
            module: &vertex,
            entry_point: c"main",
        },
        fragment: ShaderStage {
            module: &fragment,
            entry_point: c"main",
        },
        vertex_buffers: &[],
        topology: PrimitiveTopology::TriangleStrip,
        color_targets: &[ColorTargetDesc {
            format: TextureFormat::Rgba32Float,
            blend: BlendMode::PremultipliedAlpha,
        }],
        push_constant_size: 0,
        bind_group_layouts: &[&layout],
    });
    assert!(matches!(pipeline, Err(Error::InvalidArgument(_))));
    gpu.assert_no_validation_errors();
}

#[test]
fn a_texture_region_copies_into_a_larger_texture() {
    let Some(gpu) = gpu_or_skip("a_texture_region_copies_into_a_larger_texture") else {
        return;
    };
    // A store that grows: the old rows go over on the GPU, the new ones stay as they were.
    let small = texture(&gpu.device, Extent::new(4, 2), TextureFormat::R8Unorm);
    upload_whole(&gpu, &small, &[1, 2, 3, 4, 5, 6, 7, 8]);
    let large = texture(&gpu.device, Extent::new(4, 4), TextureFormat::R8Unorm);
    let other = texture(&gpu.device, Extent::new(4, 4), TextureFormat::Rgba8Unorm);
    gpu.device
        .submit_and_wait(|encoder| {
            encoder.texture_barrier(&small, TextureState::ShaderRead, TextureState::TransferSrc);
            encoder.texture_barrier(&large, TextureState::Undefined, TextureState::TransferDst);
            encoder.clear_texture(&large, [0.0; 4])?;
            // Two writes of the same texture: the second waits for the first.
            encoder.texture_barrier(&large, TextureState::TransferDst, TextureState::TransferDst);
            let rows = TextureRegion::new(0, 0, 4, 2);
            encoder.copy_texture_to_texture(&small, &large, rows)?;
            // Formats must match, and the region lie inside both.
            assert!(
                encoder
                    .copy_texture_to_texture(&small, &other, rows)
                    .is_err()
            );
            let tall = TextureRegion::new(0, 0, 4, 3);
            assert!(
                encoder
                    .copy_texture_to_texture(&small, &large, tall)
                    .is_err()
            );
            encoder.texture_barrier(&large, TextureState::TransferDst, TextureState::ShaderRead);
            encoder.texture_barrier(&small, TextureState::TransferSrc, TextureState::ShaderRead);
            Ok::<_, Error>(())
        })
        .unwrap()
        .unwrap();
    assert_eq!(
        read_back(&gpu, &large),
        [1, 2, 3, 4, 5, 6, 7, 8, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    gpu.assert_no_validation_errors();
}
