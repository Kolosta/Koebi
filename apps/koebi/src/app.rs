//! The application state machine: SDL callbacks in, frames out.

use std::time::{Duration, Instant};

use koebi_core::input::CursorShape;
use koebi_core::{Extent, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_design::persist::Store;
use koebi_gpu::{
    AdapterPreference, Device, Instance, InstanceDesc, PresentMode, Presenter, PresenterDesc,
    RenderOutcome, Surface, Validation,
};
use koebi_platform::{
    App, AppContext, AppControl, AppError, Cursor, Event, IterationRate, MouseButton, SystemCursor,
    VulkanSupport, Window, WindowDesc, WindowEvent, exit_code,
};
use koebi_render::{Renderer2D, Scene};
use koebi_ui::{Input, StyleCache, Ui, UiState};

use crate::design_options;
use crate::frame_times::FrameTimes;
use crate::gallery::Gallery;
use crate::options::Options;
use crate::overlay::Overlay;

/// Everything Vulkan. Declared before the window in [`KoebiApp`] so that it drops first:
/// surfaces must be destroyed before their native window.
struct Gpu {
    /// Rebuilt when the swapchain format changes.
    renderer: Option<Renderer2D>,
    /// `None` while the window surface is gone (mobile background).
    presenter: Option<Presenter>,
    present_mode: PresentMode,
    /// Whether the GPU measures frame times.
    timestamps: bool,
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
            timestamps: adapter.supports_timestamps(),
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

    /// Renders `scene`. Returns the outcome and the CPU time spent recording the frame's
    /// commands, without the waits for the display.
    fn render(
        &mut self,
        scene: &Scene,
        scale_factor: f32,
    ) -> koebi_gpu::Result<(RenderOutcome, Duration)> {
        let Self {
            renderer,
            presenter,
            device,
            ..
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
            recording = started.elapsed();
        })?;
        recorded.map(|()| (outcome, recording))
    }

    /// GPU time of the latest measured frame (see [`Presenter::gpu_frame_time`]).
    fn gpu_frame_time(&self) -> Option<Duration> {
        self.presenter.as_ref().and_then(Presenter::gpu_frame_time)
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

/// The interface: the design system, its saved state, and the gallery drawn with it.
struct Interface {
    design: DesignSystem,
    /// `None` without a place to save to: changes then last until the application quits.
    store: Option<Store>,
    styles: StyleCache,
    state: UiState,
    input: Input,
    gallery: Gallery,
    /// `None` when the frame-time overlay is hidden.
    overlay: Option<Overlay>,
    scene: Scene,
}

impl Interface {
    /// Restores the saved design state, then applies the command line's design changes.
    fn new(context: &AppContext, options: &Options) -> Result<Self, AppError> {
        let mut design = DesignSystem::new(&koebi_tokens::KOEBI).map_err(|error| {
            AppError::new(exit_code::FAILURE, format!("design system: {error}"))
        })?;
        let mut store = open_store(context, options, &mut design);
        design_options::apply(&mut design, options)
            .map_err(|message| AppError::new(exit_code::USAGE, message))?;
        koebi_core::info!("{}", design_options::summary(&design));
        // Changes asked on the command line, an imported theme above all, are kept at once.
        if let Some(store) = &mut store {
            let result = store.save(&design);
            report_save(store, result);
        }
        design_options::export(&design, options)
            .map_err(|message| AppError::new(exit_code::FAILURE, message))?;
        Ok(Self {
            styles: StyleCache::new(&design),
            design,
            store,
            state: UiState::new(),
            input: Input::new(),
            gallery: Gallery::new(),
            overlay: options.shows_frame_times().then(Overlay::new),
            scene: Scene::new(),
        })
    }

    /// Builds the scene of a frame covering `area`. When the frame changes the design system
    /// (a slider edits a token, a card switches the theme), the scene is built again with the
    /// new styles: every component shows the change in this very frame.
    fn build(&mut self, area: Rect, times: &FrameTimes) {
        let revision = self.design.revision();
        let input = self.input;
        self.draw(area, &input, times);
        if self.design.revision() != revision {
            self.draw(area, &input.settled(), times);
        }
        self.input.end_frame();
    }

    fn draw(&mut self, area: Rect, input: &Input, times: &FrameTimes) {
        let Self {
            design,
            styles,
            state,
            gallery,
            overlay,
            scene,
            ..
        } = self;
        scene.clear();
        let styles = styles.update(design);
        scene.set_clear_color(styles.background);
        let mut ui = Ui::new(scene, styles, input, state);
        gallery.show(&mut ui, design, area);
        drop(ui);
        if let Some(overlay) = overlay {
            overlay.draw(scene, design, times, area);
        }
    }

    /// Feeds pointer events to the interface. Returns `true` when the event concerns it.
    fn handle(&mut self, event: &Event) -> bool {
        match *event {
            Event::MouseMotion { position, .. } => self.input.pointer_moved(position),
            Event::MouseButton {
                button: MouseButton::Left,
                down,
                position,
                ..
            } => self.input.primary_button(down, position),
            Event::Window {
                event: WindowEvent::PointerLeft,
                ..
            } => self.input.pointer_left(),
            _ => return false,
        }
        true
    }

    /// `true` while changes wait for [`Self::save_if_due`].
    fn has_unsaved_changes(&self) -> bool {
        self.store
            .as_ref()
            .is_some_and(|store| store.is_dirty(&self.design))
    }

    /// Saves the design state once changes have settled: one write per pause while a slider
    /// drags, not one per frame.
    fn save_if_due(&mut self) {
        if let Some(store) = &mut self.store {
            let result = store.save_if_due(&self.design, Instant::now());
            report_save(store, result);
        }
    }

    /// Saves now if anything changed: before quitting or going to the background.
    fn save(&mut self) {
        if let Some(store) = &mut self.store {
            let result = store.save(&self.design);
            report_save(store, result);
        }
    }
}

/// The platform cursor showing `shape`.
fn platform_cursor(shape: CursorShape) -> Cursor {
    Cursor::System(match shape {
        CursorShape::Pointer => SystemCursor::Pointer,
        CursorShape::Text => SystemCursor::Text,
        CursorShape::Crosshair => SystemCursor::Crosshair,
        CursorShape::Move => SystemCursor::Move,
        CursorShape::NotAllowed => SystemCursor::NotAllowed,
        CursorShape::ResizeHorizontal => SystemCursor::ResizeHorizontal,
        CursorShape::ResizeVertical => SystemCursor::ResizeVertical,
        CursorShape::ResizeNwse => SystemCursor::ResizeNwse,
        CursorShape::ResizeNesw => SystemCursor::ResizeNesw,
        // The application registers no cursor of its own yet.
        CursorShape::Default | CursorShape::Custom(_) => SystemCursor::Default,
    })
}

/// Opens the store in the data directory and restores the saved state into `design`.
fn open_store(context: &AppContext, options: &Options, design: &mut DesignSystem) -> Option<Store> {
    let directory = match &options.data_dir {
        Some(directory) => directory.clone(),
        None => match context.preferences_dir() {
            Ok(directory) => directory,
            Err(error) => {
                koebi_core::warn!("no preferences directory ({error}): changes will not be saved");
                return None;
            }
        },
    };
    let mut store = Store::new(directory);
    match store.load(design) {
        Ok(Some(report)) => {
            for skipped in &report.skipped {
                koebi_core::warn!("design: {skipped}");
            }
            koebi_core::info!("design restored from {}", store.path().display());
        }
        Ok(None) => koebi_core::info!("design: nothing saved yet in {}", store.path().display()),
        Err(error) => koebi_core::warn!(
            "design: cannot restore {} ({error}); starting from the built-in themes",
            store.path().display()
        ),
    }
    Some(store)
}

fn report_save(store: &Store, result: koebi_design::Result<bool>) {
    match result {
        Ok(true) => koebi_core::info!("design saved to {}", store.path().display()),
        Ok(false) => {}
        Err(error) => koebi_core::warn!("cannot save the design: {error}"),
    }
}

/// Koebi's application state.
pub(crate) struct KoebiApp {
    options: Options,
    interface: Interface,
    frames_presented: u64,
    frame_times: FrameTimes,
    needs_redraw: bool,
    /// The iteration rate last asked of the platform.
    rate: IterationRate,
    // Field order is drop order: GPU objects (surface included) before the window.
    gpu: Gpu,
    window: Window,
}

impl KoebiApp {
    fn render_frame(&mut self) -> AppControl {
        let scale_factor = self.window.scale_factor();
        let logical_size = self.window.size_in_pixels().to_logical(scale_factor);
        let started = Instant::now();
        self.interface.build(
            Rect::from_origin_size(Vec2::ZERO, logical_size),
            &self.frame_times,
        );
        let built = started.elapsed();
        match self.gpu.render(&self.interface.scene, scale_factor) {
            Ok((RenderOutcome::Presented, recording)) => {
                let gpu = self.gpu.gpu_frame_time();
                self.frame_times
                    .record(built + recording, gpu, Instant::now());
                self.frames_presented += 1;
                self.needs_redraw = false;
            }
            // Minimized or swapchain recreated: keep `needs_redraw` and try again.
            Ok((RenderOutcome::Skipped, _)) => {}
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

    /// Waits for events, except while changes wait to be saved: then the application wakes up
    /// a few times per second until they are. Tests and benchmarks draw continuously.
    fn update_rate(&mut self, context: &mut AppContext) {
        let rate = if self.options.frames.is_some() {
            IterationRate::Unlimited
        } else if self.interface.has_unsaved_changes() {
            IterationRate::Hz(10)
        } else {
            IterationRate::WaitEvent
        };
        if rate != self.rate {
            context.set_iteration_rate(rate);
            self.rate = rate;
        }
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
        if let Some(run) = self.frame_times.run() {
            let gpu = run.gpu.map_or_else(
                || "not measured".to_owned(),
                |gpu| format!("{:.3} ms", gpu.as_secs_f64() * 1000.0),
            );
            koebi_core::info!(
                "frame times: CPU {:.3} ms, GPU {gpu}: {:.0} frames per second possible",
                run.cpu.as_secs_f64() * 1000.0,
                run.frames_per_second()
            );
        }
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
        let rate = if options.frames.is_some() {
            IterationRate::Unlimited
        } else {
            IterationRate::WaitEvent
        };
        context.set_iteration_rate(rate);
        let interface = Interface::new(context, &options)?;

        // Load Vulkan before creating the window: otherwise SDL loads its default library when
        // the first Vulkan window is created, which fails where the loader is not on the system
        // search path (the Vulkan SDK on macOS).
        let library = koebi_gpu::loader::preferred_library_path();
        let vulkan = context.load_vulkan(library.as_deref()).map_err(|error| {
            AppError::new(
                exit_code::UNSUPPORTED,
                format!("Vulkan is not available: {error}"),
            )
        })?;
        let window = context
            .create_window(&WindowDesc::default())
            .map_err(|error| {
                AppError::new(
                    exit_code::UNSUPPORTED,
                    format!("cannot create the window: {error}"),
                )
            })?;
        let gpu = Gpu::new(&window, &vulkan, options.present_mode)?;
        koebi_core::info!(
            "window {}x{} px, scale {:.2}",
            window.size_in_pixels().width,
            window.size_in_pixels().height,
            window.scale_factor()
        );

        if options.shows_frame_times() {
            koebi_core::info!(
                "frame-time overlay shown (GPU timestamps: {})",
                if gpu.timestamps { "yes" } else { "no" }
            );
        }
        Ok(Self {
            options,
            interface,
            frames_presented: 0,
            frame_times: FrameTimes::new(),
            needs_redraw: true,
            rate,
            gpu,
            window,
        })
    }

    fn iterate(&mut self, context: &mut AppContext) -> AppControl {
        if self.needs_redraw || self.options.frames.is_some() {
            let control = self.render_frame();
            if control != AppControl::Continue {
                return control;
            }
            context.set_cursor(platform_cursor(self.interface.state.cursor()));
        }
        self.interface.save_if_due();
        self.update_rate(context);
        match self.options.frames {
            Some(limit) if self.frames_presented >= limit => self.finish(),
            _ => AppControl::Continue,
        }
    }

    fn event(&mut self, _context: &mut AppContext, event: &Event) -> AppControl {
        if self.interface.handle(event) {
            self.needs_redraw = true;
            return AppControl::Continue;
        }
        match *event {
            Event::Quit | Event::Terminating => {
                self.interface.save();
                return self.finish();
            }
            Event::Window { window, event } if window == self.window.id() => match event {
                WindowEvent::CloseRequested => return self.finish(),
                WindowEvent::PixelSizeChanged(extent) => {
                    self.gpu.resize(extent);
                    self.needs_redraw = true;
                }
                _ => self.needs_redraw = true,
            },
            // The OS may kill a background application without notice: save first. Android
            // also destroys the native window: release the surface.
            Event::WillEnterBackground => {
                self.interface.save();
                self.gpu.detach();
            }
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

    fn quit(mut self, _context: &mut AppContext) {
        self.interface.save();
        if let Err(error) = self.gpu.device.wait_idle() {
            koebi_core::warn!("waiting for the GPU at exit failed: {error}");
        }
        koebi_core::info!("Koebi exits after {} frames", self.frames_presented);
    }
}
