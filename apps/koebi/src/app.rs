//! The application state machine: SDL callbacks in, frames out.

use koebi_core::Extent;
use koebi_gpu::{
    AdapterPreference, Device, Instance, InstanceDesc, PresentMode, Presenter, PresenterDesc,
    RenderOutcome, Surface, Validation,
};
use koebi_platform::{
    App, AppContext, AppControl, AppError, Event, IterationRate, VulkanSupport, Window, WindowDesc,
    WindowEvent, exit_code,
};
use koebi_render::{Renderer2D, Scene};

use crate::options::Options;
use crate::scene;
use crate::theme::Theme;

/// Everything Vulkan. Declared before the window in [`KoebiApp`] so that it drops first:
/// surfaces must be destroyed before their native window.
struct Gpu {
    /// Rebuilt when the swapchain format changes.
    renderer: Option<Renderer2D>,
    /// `None` while the window surface is gone (mobile background).
    presenter: Option<Presenter>,
    present_mode: PresentMode,
    device: Device,
    instance: Instance,
}

impl Gpu {
    fn new(
        window: &Window,
        vulkan: &VulkanSupport,
        present_mode: PresentMode,
    ) -> Result<Self, AppError> {
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
        let mut gpu = Self {
            renderer: None,
            presenter: None,
            present_mode,
            device,
            instance,
        };
        gpu.attach(window, surface)?;
        Ok(gpu)
    }

    /// Creates the presenter for a (new) surface.
    fn attach(&mut self, window: &Window, surface: Surface) -> Result<(), AppError> {
        let desc = PresenterDesc {
            extent: window.size_in_pixels(),
            present_mode: self.present_mode,
        };
        self.presenter =
            Some(Presenter::new(&self.device, surface, &desc).map_err(|e| gpu_error(&e))?);
        Ok(())
    }

    /// Drops the swapchain and surface (the native window is going away).
    fn detach(&mut self) {
        self.presenter = None;
    }

    fn reattach(&mut self, window: &Window) -> Result<(), AppError> {
        if self.presenter.is_none() {
            let surface = create_surface(&self.instance, window)?;
            self.attach(window, surface)?;
        }
        Ok(())
    }

    fn resize(&mut self, extent: Extent) {
        if let Some(presenter) = &mut self.presenter {
            presenter.resize(extent);
        }
    }

    fn render(&mut self, scene: &Scene, scale_factor: f32) -> koebi_gpu::Result<RenderOutcome> {
        let Self {
            renderer,
            presenter,
            device,
            ..
        } = self;
        let Some(presenter) = presenter.as_mut() else {
            return Ok(RenderOutcome::Skipped);
        };
        let mut recorded = Ok(());
        let outcome = presenter.render(|encoder, frame| {
            let target = frame.render_target();
            if renderer
                .as_ref()
                .is_none_or(|r| r.target_format() != target.format())
            {
                match Renderer2D::new(device, target.format()) {
                    Ok(created) => *renderer = Some(created),
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
        })?;
        recorded.map(|()| outcome)
    }
}

fn create_surface(instance: &Instance, window: &Window) -> Result<Surface, AppError> {
    // SAFETY: the instance comes from SDL's loader with SDL's required extensions. `Surface`
    // takes ownership and is dropped before the window (field order of `KoebiApp`).
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

/// Koebi's application state.
pub(crate) struct KoebiApp {
    options: Options,
    theme: Theme,
    scene: Scene,
    frames_presented: u64,
    needs_redraw: bool,
    // Field order is drop order: GPU objects (surface included) before the window.
    gpu: Gpu,
    window: Window,
}

impl KoebiApp {
    fn render_frame(&mut self) -> AppControl {
        let scale_factor = self.window.scale_factor();
        let logical_size = self.window.size_in_pixels().to_logical(scale_factor);
        scene::build(&mut self.scene, logical_size, &self.theme);
        match self.gpu.render(&self.scene, scale_factor) {
            Ok(RenderOutcome::Presented) => {
                self.frames_presented += 1;
                self.needs_redraw = false;
            }
            // Minimized or swapchain recreated: keep `needs_redraw` and try again.
            Ok(RenderOutcome::Skipped) => {}
            Err(koebi_gpu::Error::SurfaceLost) => {
                koebi_core::warn!("window surface lost, recreating it");
                self.gpu.detach();
                if let Err(error) = self.gpu.reattach(&self.window) {
                    koebi_core::error!("{}", error.message);
                    return AppControl::Exit(error.exit_code);
                }
            }
            Err(error) => {
                koebi_core::error!("rendering failed: {error}");
                return AppControl::Exit(exit_code::FAILURE);
            }
        }
        AppControl::Continue
    }

    /// Exit code reflecting the Vulkan validation report.
    fn finish(&self) -> AppControl {
        let report = self.gpu.instance.validation_report();
        koebi_core::info!(
            "presented {} frames (validation: {} errors, {} warnings)",
            self.frames_presented,
            report.errors,
            report.warnings
        );
        if report.is_clean() {
            AppControl::Exit(exit_code::SUCCESS)
        } else {
            AppControl::Exit(exit_code::VALIDATION_ERRORS)
        }
    }
}

impl App for KoebiApp {
    fn init(context: &mut AppContext, args: &[String]) -> Result<Self, AppError> {
        let options = Options::parse(args.iter().skip(1)).map_err(|error| {
            AppError::new(exit_code::USAGE, format!("{error}\n\n{}", Options::USAGE))
        })?;
        if options.help || options.version {
            let text = if options.help {
                Options::USAGE.to_owned()
            } else {
                format!("koebi {}", env!("CARGO_PKG_VERSION"))
            };
            #[allow(clippy::print_stdout, reason = "--help and --version print to stdout")]
            {
                println!("{text}");
            }
            return Err(AppError::new(exit_code::SUCCESS, String::new()));
        }

        // A UI only needs to draw after input; benchmarks and tests draw continuously.
        context.set_iteration_rate(if options.frames.is_some() {
            IterationRate::Unlimited
        } else {
            IterationRate::WaitEvent
        });

        let window = context
            .create_window(&WindowDesc::default())
            .map_err(|error| {
                AppError::new(
                    exit_code::UNSUPPORTED,
                    format!("cannot create the window: {error}"),
                )
            })?;
        let library = koebi_gpu::loader::preferred_library_path();
        let vulkan = context.load_vulkan(library.as_deref()).map_err(|error| {
            AppError::new(
                exit_code::UNSUPPORTED,
                format!("Vulkan is not available: {error}"),
            )
        })?;
        let gpu = Gpu::new(&window, &vulkan, options.present_mode)?;
        koebi_core::info!(
            "window {}x{} px, scale {:.2}",
            window.size_in_pixels().width,
            window.size_in_pixels().height,
            window.scale_factor()
        );

        Ok(Self {
            options,
            theme: Theme::dark(),
            scene: Scene::new(),
            frames_presented: 0,
            needs_redraw: true,
            gpu,
            window,
        })
    }

    fn iterate(&mut self, _context: &mut AppContext) -> AppControl {
        if self.needs_redraw || self.options.frames.is_some() {
            let control = self.render_frame();
            if control != AppControl::Continue {
                return control;
            }
        }
        match self.options.frames {
            Some(limit) if self.frames_presented >= limit => self.finish(),
            _ => AppControl::Continue,
        }
    }

    fn event(&mut self, _context: &mut AppContext, event: &Event) -> AppControl {
        match *event {
            Event::Quit | Event::Terminating => return self.finish(),
            Event::Window { window, event } if window == self.window.id() => match event {
                WindowEvent::CloseRequested => return self.finish(),
                WindowEvent::PixelSizeChanged(extent) => {
                    self.gpu.resize(extent);
                    self.needs_redraw = true;
                }
                _ => self.needs_redraw = true,
            },
            // Android destroys the native window in the background: release the surface first.
            Event::WillEnterBackground => self.gpu.detach(),
            Event::DidEnterForeground => {
                if let Err(error) = self.gpu.reattach(&self.window) {
                    koebi_core::error!("{}", error.message);
                    return AppControl::Exit(error.exit_code);
                }
                self.needs_redraw = true;
            }
            _ => {}
        }
        AppControl::Continue
    }

    fn quit(self, _context: &mut AppContext) {
        if let Err(error) = self.gpu.device.wait_idle() {
            koebi_core::warn!("waiting for the GPU at exit failed: {error}");
        }
        koebi_core::info!("Koebi exits after {} frames", self.frames_presented);
    }
}
