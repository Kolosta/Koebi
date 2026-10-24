//! Headless GPU access for tests.

use koebi_core::Extent;
use koebi_gpu::{
    AdapterPreference, BufferDesc, BufferUsage, CommandEncoder, Device, Instance, InstanceDesc,
    MemoryLocation, RenderTarget, TextureDesc, TextureFormat, TextureState, TextureUsage,
    Validation, ValidationReport, VulkanLibrary,
};

use crate::image::Image;

/// `true` when `KOEBI_REQUIRE_GPU` is set to `1`/`true`: GPU tests must not skip.
pub fn require_gpu() -> bool {
    std::env::var("KOEBI_REQUIRE_GPU")
        .is_ok_and(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true"))
}

/// A Vulkan instance and device without any window.
///
/// Validation layers are enabled when installed (`KOEBI_VALIDATION` overrides), and
/// `KOEBI_ADAPTER` selects a specific adapter (e.g. `llvmpipe`).
#[derive(Debug)]
pub struct HeadlessGpu {
    /// The instance.
    pub instance: Instance,
    /// The device.
    pub device: Device,
}

impl HeadlessGpu {
    /// Creates the instance and device.
    ///
    /// # Errors
    ///
    /// Propagates loader, instance, adapter and device errors.
    pub fn new() -> koebi_gpu::Result<Self> {
        let library = VulkanLibrary::open_default()?;
        let instance = Instance::from_library(
            &library,
            &InstanceDesc {
                application_name: "koebi-tests",
                application_version: (0, 1, 0),
                validation: Validation::from_env_or(Validation::Enabled),
                required_extensions: &[],
            },
        )?;
        let adapter = instance.select_adapter(AdapterPreference::default(), None)?;
        let device = Device::new(&instance, &adapter)?;
        Ok(Self { instance, device })
    }

    /// Validation messages reported so far.
    pub fn validation_report(&self) -> ValidationReport {
        self.instance.validation_report()
    }

    /// Fails the test if the validation layers reported any error.
    ///
    /// # Panics
    ///
    /// Panics when the report contains errors (they are printed in the log above).
    pub fn assert_no_validation_errors(&self) {
        let report = self.validation_report();
        assert!(
            report.is_clean(),
            "{} Vulkan validation error(s) were reported, see the log above",
            report.errors
        );
    }

    /// Renders with `record` into a fresh texture and returns its pixels as RGBA8.
    ///
    /// `record` receives the encoder and the target, already in
    /// [`TextureState::ColorAttachment`]. Only 4-byte RGBA/BGRA formats are supported.
    ///
    /// # Errors
    ///
    /// Propagates resource creation and submission errors.
    pub fn render_offscreen(
        &self,
        extent: Extent,
        format: TextureFormat,
        record: impl FnOnce(&mut CommandEncoder<'_>, RenderTarget),
    ) -> koebi_gpu::Result<Image> {
        let texture = self.device.create_texture(&TextureDesc {
            label: "test offscreen target",
            extent,
            format,
            usage: TextureUsage::COLOR_ATTACHMENT | TextureUsage::TRANSFER_SRC,
        })?;
        let size = extent.area() * u64::from(format.bytes_per_pixel());
        let readback = self.device.create_buffer(&BufferDesc {
            label: "test readback",
            size,
            usage: BufferUsage::TRANSFER_DST,
            location: MemoryLocation::GpuToCpu,
        })?;
        let target = texture.render_target()?;
        self.device.submit_and_wait(|encoder| {
            encoder.texture_barrier(
                &texture,
                TextureState::Undefined,
                TextureState::ColorAttachment,
            );
            record(encoder, target);
            encoder.texture_barrier(
                &texture,
                TextureState::ColorAttachment,
                TextureState::TransferSrc,
            );
            encoder.copy_texture_to_buffer(&texture, &readback)
        })??;

        let len = usize::try_from(size)
            .map_err(|_| koebi_gpu::Error::InvalidArgument("readback too large".into()))?;
        let mut bytes = vec![0_u8; len];
        readback.read(0, &mut bytes)?;
        Image::from_gpu(extent, format, bytes).map_err(koebi_gpu::Error::InvalidArgument)
    }
}

/// Returns a headless GPU, or `None` when the machine has no usable Vulkan device.
///
/// Tests call it first and return early on `None`. With `KOEBI_REQUIRE_GPU=1` (CI) a missing
/// device fails the test instead. Errors that are not about the environment (device lost,
/// invalid usage...) always fail.
///
/// # Panics
///
/// Panics when a GPU is required but unavailable, or on unexpected errors.
pub fn gpu_or_skip(test_name: &str) -> Option<HeadlessGpu> {
    // Mobile CI validates rendering with app smoke tests; emulator and simulator GPUs are not
    // reference drivers, and golden files are not deployed to devices.
    if cfg!(any(target_os = "android", target_os = "ios")) && !require_gpu() {
        eprintln!(
            "SKIPPED {test_name}: GPU tests run on desktop (set KOEBI_REQUIRE_GPU=1 to force)"
        );
        return None;
    }
    match HeadlessGpu::new() {
        Ok(gpu) => Some(gpu),
        Err(error) if error.is_unsupported_environment() && !require_gpu() => {
            eprintln!("SKIPPED {test_name}: no usable Vulkan device ({error})");
            None
        }
        Err(error) => panic!("{test_name}: cannot create a headless GPU device: {error}"),
    }
}
