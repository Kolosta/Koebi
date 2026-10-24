//! Integration tests of the Vulkan RHI on a headless device.
//!
//! Each test skips without a usable GPU, unless `KOEBI_REQUIRE_GPU=1` (CI, where lavapipe,
//! WARP-like software drivers or MoltenVK provide one). Validation layers are enabled when
//! installed and every test asserts that they reported no error.

use koebi_core::Extent;
use koebi_gpu::{
    BufferDesc, BufferUsage, Error, LoadOp, MemoryLocation, TextureDesc, TextureFormat,
    TextureUsage, vk,
};
use koebi_testkit::gpu_or_skip;

#[test]
fn adapter_meets_the_baseline() {
    let Some(gpu) = gpu_or_skip("adapter_meets_the_baseline") else {
        return;
    };
    let info = gpu.device.adapter().info();
    assert!(info.api_version >= vk::API_VERSION_1_2, "{info:?}");
    gpu.assert_no_validation_errors();
}

#[test]
fn host_visible_buffers_round_trip_and_check_bounds() {
    let Some(gpu) = gpu_or_skip("host_visible_buffers_round_trip_and_check_bounds") else {
        return;
    };
    let mut buffer = gpu
        .device
        .create_buffer(&BufferDesc {
            label: "round trip",
            size: 64,
            usage: BufferUsage::VERTEX,
            location: MemoryLocation::CpuToGpu,
        })
        .unwrap();
    let data: Vec<u32> = (0..16).collect();
    buffer.write(0, &data).unwrap();
    let mut back = vec![0_u32; 16];
    buffer.read(0, &mut back).unwrap();
    assert_eq!(back, data);
    assert!(matches!(
        buffer.write(60, &[1_u32, 2]),
        Err(Error::InvalidArgument(_))
    ));
    gpu.assert_no_validation_errors();
}

#[test]
fn invalid_descriptions_are_rejected_before_reaching_vulkan() {
    let Some(gpu) = gpu_or_skip("invalid_descriptions_are_rejected_before_reaching_vulkan") else {
        return;
    };
    let empty = gpu.device.create_buffer(&BufferDesc {
        label: "empty",
        size: 0,
        usage: BufferUsage::VERTEX,
        location: MemoryLocation::GpuOnly,
    });
    assert!(matches!(empty, Err(Error::InvalidArgument(_))));

    let mut device_local = gpu
        .device
        .create_buffer(&BufferDesc {
            label: "device local",
            size: 16,
            usage: BufferUsage::VERTEX,
            location: MemoryLocation::GpuOnly,
        })
        .unwrap();
    assert!(matches!(
        device_local.write(0, &[0_u8; 4]),
        Err(Error::InvalidArgument(_))
    ));

    let texture = gpu.device.create_texture(&TextureDesc {
        label: "empty",
        extent: Extent::new(0, 4),
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsage::SAMPLED,
    });
    assert!(matches!(texture, Err(Error::InvalidArgument(_))));
    gpu.assert_no_validation_errors();
}

#[test]
fn clear_pass_writes_the_exact_color() {
    let Some(gpu) = gpu_or_skip("clear_pass_writes_the_exact_color") else {
        return;
    };
    for format in [TextureFormat::Rgba8Unorm, TextureFormat::Bgra8Unorm] {
        let image = gpu
            .render_offscreen(Extent::new(16, 8), format, |encoder, target| {
                let _pass = encoder.begin_render_pass(target, LoadOp::Clear([1.0, 0.25, 0.0, 1.0]));
            })
            .unwrap();
        for y in 0..8 {
            for x in 0..16 {
                assert_eq!(
                    image.pixel(x, y),
                    [255, 64, 0, 255],
                    "{format:?} pixel ({x}, {y})"
                );
            }
        }
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn dropped_resources_are_released_safely() {
    let Some(gpu) = gpu_or_skip("dropped_resources_are_released_safely") else {
        return;
    };
    for i in 0..32 {
        let buffer = gpu
            .device
            .create_buffer(&BufferDesc {
                label: "transient",
                size: 256,
                usage: BufferUsage::VERTEX,
                location: MemoryLocation::CpuToGpu,
            })
            .unwrap();
        let texture = gpu
            .device
            .create_texture(&TextureDesc {
                label: "transient",
                extent: Extent::new(8, 8),
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::COLOR_ATTACHMENT,
            })
            .unwrap();
        // Dropping right after recording work that references them must be safe.
        gpu.device
            .submit_and_wait(|encoder| {
                let target = texture.render_target().unwrap();
                encoder.texture_barrier(
                    &texture,
                    koebi_gpu::TextureState::Undefined,
                    koebi_gpu::TextureState::ColorAttachment,
                );
                let _pass = encoder.begin_render_pass(target, LoadOp::Clear([0.0; 4]));
            })
            .unwrap();
        drop((buffer, texture));
        if i % 8 == 0 {
            gpu.device.wait_idle().unwrap();
        }
    }
    gpu.assert_no_validation_errors();
}
