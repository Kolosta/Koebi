//! Everything Vulkan: one instance and one device for the application, and what each window
//! needs of its own to show frames (a presenter and a renderer, whose streaming buffers follow
//! the frames of that presenter).

use std::time::{Duration, Instant};

use koebi_core::Extent;
use koebi_gpu::{
    AdapterPreference, Device, Instance, InstanceDesc, PresentMode, Presenter, PresenterDesc,
    RenderOutcome, Surface, Validation,
};
use koebi_platform::{AppError, VulkanSupport, Window, exit_code};
use koebi_render::{Renderer2D, Scene};
use koebi_text::Fonts;

/// What every window shares. Declared after the windows in
/// [`KoebiApp`](crate::app::KoebiApp), so that it drops last.
pub(crate) struct Gpu {
    present_mode: PresentMode,
    /// Whether the GPU measures frame times.
    pub(crate) timestamps: bool,
    /// What glyphs are rasterized from: the interface's fonts.
    fonts: Fonts,
    pub(crate) device: Device,
    pub(crate) instance: Instance,
}

impl Gpu {
    /// Creates the instance and the device for `window`, and what it needs to show frames.
    pub(crate) fn new(
        window: &Window,
        vulkan: &VulkanSupport,
        present_mode: PresentMode,
        fonts: Fonts,
    ) -> Result<(Self, WindowGpu), AppError> {
        let extensions = vulkan.required_extensions();
        let default_validation = if cfg!(debug_assertions) {
            Validation::Enabled
        } else {
            Validation::Disabled
        };
        let desc = InstanceDesc {
            application_name: "Koebi",
            application_version: version(),
            validation: Validation::from_env_or(default_validation),
            required_extensions: &extensions,
        };
        // SAFETY: SDL keeps its Vulkan loader loaded until after `App::quit`, which drops
        // every Vulkan object.
        let instance = unsafe { Instance::new(vulkan.get_instance_proc_addr(), &desc) }
            .map_err(|e| gpu_error(&e))?;
        let surface = create_surface(&instance, window)?;
        let adapter = instance
            .select_adapter(AdapterPreference::default(), Some(&surface))
            .map_err(|e| gpu_error(&e))?;
        let device = Device::new(&instance, &adapter).map_err(|e| gpu_error(&e))?;
        let gpu = Self {
            present_mode,
            timestamps: adapter.supports_timestamps(),
            fonts,
            device,
            instance,
        };
        let mut first = WindowGpu::empty();
        first.attach(&gpu, window, surface)?;
        Ok((gpu, first))
    }

    /// What `window` needs to show frames.
    pub(crate) fn for_window(&self, window: &Window) -> Result<WindowGpu, AppError> {
        let surface = create_surface(&self.instance, window)?;
        let mut gpu = WindowGpu::empty();
        gpu.attach(self, window, surface)?;
        Ok(gpu)
    }
}

/// What one window needs to show frames. Declared before the native window in
/// [`AppWindow`](crate::window::AppWindow): surfaces must be destroyed before their window.
pub(crate) struct WindowGpu {
    /// Rebuilt when the swapchain format changes.
    renderer: Option<Renderer2D>,
    /// `None` while the window surface is gone (mobile background).
    presenter: Option<Presenter>,
}

impl WindowGpu {
    const fn empty() -> Self {
        Self {
            renderer: None,
            presenter: None,
        }
    }

    /// Creates the presenter for a (new) surface.
    fn attach(&mut self, gpu: &Gpu, window: &Window, surface: Surface) -> Result<(), AppError> {
        let desc = PresenterDesc {
            extent: window.size_in_pixels(),
            present_mode: gpu.present_mode,
        };
        self.presenter =
            Some(Presenter::new(&gpu.device, surface, &desc).map_err(|e| gpu_error(&e))?);
        Ok(())
    }

    /// Drops the swapchain and surface (the native window is going away).
    pub(crate) fn detach(&mut self) {
        self.presenter = None;
    }

    /// Creates the presenter again after [`Self::detach`].
    pub(crate) fn reattach(&mut self, gpu: &Gpu, window: &Window) -> Result<(), AppError> {
        if self.presenter.is_none() {
            let surface = create_surface(&gpu.instance, window)?;
            self.attach(gpu, window, surface)?;
        }
        Ok(())
    }

    /// The window has a new size in pixels.
    pub(crate) fn resize(&mut self, extent: Extent) {
        if let Some(presenter) = &mut self.presenter {
            presenter.resize(extent);
        }
    }

    /// Renders `scene`. Returns the outcome and the CPU time spent recording the frame's
    /// commands, without the waits for the display.
    pub(crate) fn render(
        &mut self,
        gpu: &Gpu,
        scene: &Scene,
        scale_factor: f32,
    ) -> koebi_gpu::Result<(RenderOutcome, Duration)> {
        let Self {
            renderer,
            presenter,
        } = self;
        let Some(presenter) = presenter.as_mut() else {
            return Ok((RenderOutcome::Skipped, Duration::ZERO));
        };
        let mut recorded = Ok(());
        let mut recording = Duration::ZERO;
        let outcome = presenter.render(|encoder, frame| {
            let started = Instant::now();
            let target = frame.render_target();
            if renderer
                .as_ref()
                .is_none_or(|r| r.target_format() != target.format())
            {
                match Renderer2D::new(&gpu.device, target.format()) {
                    Ok(mut created) => {
                        created.set_fonts(gpu.fonts.clone());
                        *renderer = Some(created);
                    }
                    Err(error) => {
                        recorded = Err(error);
                        return;
                    }
                }
            }
            if let Some(renderer) = renderer.as_mut() {
                recorded =
                    renderer.render(encoder, target, scene, frame.frame_index(), scale_factor);
            }
            recording = started.elapsed();
        })?;
        recorded.map(|()| (outcome, recording))
    }

    /// GPU time of the latest measured frame (see [`Presenter::gpu_frame_time`]).
    pub(crate) fn gpu_frame_time(&self) -> Option<Duration> {
        self.presenter.as_ref().and_then(Presenter::gpu_frame_time)
    }
}

fn create_surface(instance: &Instance, window: &Window) -> Result<Surface, AppError> {
    // SAFETY: the instance comes from SDL's loader with SDL's required extensions. `Surface`
    // takes ownership and is dropped before the window (field order of `AppWindow`).
    let raw = unsafe { window.create_vulkan_surface(instance.handle()) }
        .map_err(|error| AppError::new(exit_code::FAILURE, format!("surface creation: {error}")))?;
    // SAFETY: `raw` was just created from `instance` and is owned by nobody else.
    unsafe { Surface::from_raw(instance, raw) }.map_err(|e| gpu_error(&e))
}

fn gpu_error(error: &koebi_gpu::Error) -> AppError {
    let code = if error.is_unsupported_environment() {
        exit_code::UNSUPPORTED
    } else {
        exit_code::FAILURE
    };
    AppError::new(code, format!("GPU initialization failed: {error}"))
}

fn version() -> (u32, u32, u32) {
    let part = |text: &str| text.parse().unwrap_or(0);
    (
        part(env!("CARGO_PKG_VERSION_MAJOR")),
        part(env!("CARGO_PKG_VERSION_MINOR")),
        part(env!("CARGO_PKG_VERSION_PATCH")),
    )
}
