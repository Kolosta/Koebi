//! The application state machine: SDL callbacks in, frames out.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use koebi_core::{Extent, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_design::persist::Store;
use koebi_gpu::{
    AdapterPreference, Device, Instance, InstanceDesc, PresentMode, Presenter, PresenterDesc,
    RenderOutcome, Surface, Validation,
};
use koebi_input::{Context, InputSettings, Keymap};
use koebi_platform::{
    App, AppContext, AppControl, AppError, Event, IterationRate, VulkanSupport, Window, WindowDesc,
    WindowEvent, exit_code,
};
use koebi_render::{Renderer2D, Scene};
use koebi_text::Fonts;
use koebi_tokens::semantic;
use koebi_ui::{Input, StyleCache, Ui, UiState, WorkspaceView};
use koebi_workspace::Workspace;

use crate::actions;
use crate::cursors::AppCursors;
use crate::design_options;
use crate::editors::{self, Content, Editors, Host};
use crate::frame_times::FrameTimes;
use crate::options::Options;
use crate::overlay::Overlay;
use crate::preferences::Preferences;

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
    /// What glyphs are rasterized from: the interface's fonts.
    fonts: Fonts,
    device: Device,
    instance: Instance,
}

impl Gpu {
    fn new(
        window: &Window,
        vulkan: &VulkanSupport,
        present_mode: PresentMode,
        fonts: Fonts,
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
            fonts,
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
            fonts,
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
                    Ok(mut created) => {
                        created.set_fonts(fonts.clone());
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
    /// The saved workspace and shortcuts, `None` without a place to save to.
    preferences: Option<Preferences>,
    styles: StyleCache,
    state: UiState,
    input: Input,
    /// The areas and their editors.
    workspace: Workspace,
    view: WorkspaceView,
    editors: Editors,
    /// What the editors show: the gallery, the document, the messages.
    content: Content,
    /// `None` when the frame-time overlay is hidden.
    overlay: Option<Overlay>,
    /// The fonts text is laid out with; the renderer rasterizes glyphs from the same ones.
    fonts: Fonts,
    /// Turns input events into actions, through the contexts under the pointer.
    keymap: Keymap,
    input_settings: InputSettings,
    /// Reused for the chain of contexts of each event.
    chain: Vec<Context>,
    /// The origin of the interface's time ([`Input::set_time`]).
    started: Instant,
    scene: Scene,
    /// While the pointer is locked, motion only moves what holds it: positions stay.
    pointer_locked: bool,
}

impl Interface {
    /// Restores the saved design state, then applies the command line's design changes.
    fn new(context: &AppContext, options: &Options, fonts: Fonts) -> Result<Self, AppError> {
        let mut design = DesignSystem::new(&koebi_tokens::KOEBI).map_err(|error| {
            AppError::new(exit_code::FAILURE, format!("design system: {error}"))
        })?;
        let directory = data_directory(context, options);
        let mut store = directory
            .as_deref()
            .map(|directory| open_store(directory, &mut design));
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
        let mut keymap = Keymap::new(&actions::GROUPS)
            .map_err(|error| AppError::new(exit_code::FAILURE, format!("shortcuts: {error}")))?;
        let mut state = UiState::new();
        state.set_fonts(fonts.clone());
        let mut preferences = directory.as_deref().map(Preferences::new);
        let (workspace, editors) = preferences
            .as_mut()
            .and_then(Preferences::load_workspace)
            .unwrap_or_else(editors::default_workspace);
        if let Some(preferences) = &mut preferences {
            preferences.load_shortcuts(&mut keymap);
        }
        Ok(Self {
            styles: StyleCache::new(&design),
            design,
            store,
            preferences,
            state,
            input: Input::new(),
            workspace,
            view: WorkspaceView::new(),
            editors,
            content: Content::new(),
            overlay: options.shows_frame_times().then(Overlay::new),
            fonts,
            keymap,
            input_settings: InputSettings::default(),
            chain: Vec::new(),
            started: Instant::now(),
            scene: Scene::new(),
            pointer_locked: false,
        })
    }

    /// Builds the scene of a frame covering `area`. When the frame changes the design system
    /// (a slider edits a token, a card switches the theme), the scene is built again with the
    /// new styles: every component shows the change in this very frame.
    fn build(&mut self, area: Rect, times: &FrameTimes) {
        self.input.set_time(self.started.elapsed());
        if self.input.triggered(&actions::FRAME_TIMES) {
            self.overlay = match self.overlay {
                Some(_) => None,
                None => Some(Overlay::new()),
            };
        }
        // Thresholds and delays are tokens: a theme or the user may change them.
        self.input_settings = InputSettings {
            drag_threshold: self.design.get(semantic::INPUT_DRAG_THRESHOLD),
            double_click_time: self.design.get(semantic::INPUT_DOUBLE_CLICK_TIME),
        };
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
            workspace,
            view,
            editors,
            content,
            overlay,
            fonts,
            scene,
            ..
        } = self;
        scene.clear();
        let styles = styles.update(design);
        scene.set_clear_color(styles.background);
        let mut ui = Ui::new(scene, styles, input, state);
        content.messages.collect();
        let mut host = Host {
            editors,
            content,
            design,
            window: area,
        };
        ui.zone(area, actions::APP, |ui| {
            ui.workspace(view, workspace, area, &mut host);
        });
        drop(ui);
        if let Some(overlay) = overlay {
            overlay.draw(scene, design, fonts, times, area);
        }
    }

    /// Feeds an input event to the interface, and to the keymap through the contexts under
    /// the pointer. Returns `true` when the event concerns them.
    fn handle(&mut self, event: &Event) -> bool {
        match *event {
            Event::MouseMotion {
                position,
                delta,
                modifiers,
                ..
            } => {
                if !self.pointer_locked {
                    self.input.pointer_moved(position);
                }
                self.input.pointer_moved_by(delta);
                self.input.set_modifiers(modifiers);
            }
            Event::TextInput { text, .. } => {
                self.input.typed(text.as_str());
                return true;
            }
            Event::MouseButton {
                button,
                down,
                position,
                modifiers,
                ..
            } => {
                if let Some(button) = actions::pointer_button(button) {
                    self.input.button(button, down, position);
                }
                self.input.set_modifiers(modifiers);
            }
            Event::MouseWheel {
                delta, modifiers, ..
            } => {
                self.input.scrolled(delta);
                self.input.set_modifiers(modifiers);
            }
            Event::Key { modifiers, .. } => self.input.set_modifiers(modifiers),
            Event::Window {
                event: WindowEvent::PointerLeft,
                ..
            } => {
                self.input.pointer_left();
                return true;
            }
            _ => {}
        }
        let Some((input_event, position)) = actions::input_event(event, self.input.pointer())
        else {
            return false;
        };
        self.state.chain_at(position, &mut self.chain);
        let triggers = self.keymap.handle(
            input_event,
            &self.chain,
            &self.input_settings,
            Instant::now(),
        );
        for triggered in triggers.iter() {
            self.input.trigger(triggered.action);
        }
        true
    }

    /// How long until the interface wants to be drawn again without input, if it does: zero
    /// when it is due.
    fn redraw_in(&self) -> Option<Duration> {
        let at = self.state.redraw_at()?;
        Some(at.saturating_sub(self.started.elapsed()))
    }

    /// `true` while changes wait for [`Self::save_if_due`].
    fn has_unsaved_changes(&self) -> bool {
        self.store
            .as_ref()
            .is_some_and(|store| store.is_dirty(&self.design))
            || self
                .preferences
                .as_ref()
                .is_some_and(|preferences| preferences.is_dirty(&self.workspace, &self.editors))
    }

    /// Saves the design state once changes have settled: one write per pause while a slider
    /// drags, not one per frame.
    fn save_if_due(&mut self) {
        if let Some(store) = &mut self.store {
            let result = store.save_if_due(&self.design, Instant::now());
            report_save(store, result);
        }
        if let Some(preferences) = &mut self.preferences {
            preferences.save_if_due(&self.workspace, &self.editors, Instant::now());
        }
    }

    /// Saves now if anything changed: before quitting or going to the background.
    fn save(&mut self) {
        if let Some(store) = &mut self.store {
            let result = store.save(&self.design);
            report_save(store, result);
        }
        if let Some(preferences) = &mut self.preferences {
            preferences.save(&self.workspace, &self.editors, &self.keymap);
        }
    }
}

/// Where the design state, the workspace and the shortcuts are saved: the command line's
/// directory, else the platform's preferences directory; `None` when there is none.
fn data_directory(context: &AppContext, options: &Options) -> Option<PathBuf> {
    match &options.data_dir {
        Some(directory) => Some(directory.clone()),
        None => match context.preferences_dir() {
            Ok(directory) => Some(directory),
            Err(error) => {
                koebi_core::warn!("no preferences directory ({error}): changes will not be saved");
                None
            }
        },
    }
}

/// Opens the store in `directory` and restores the saved state into `design`.
fn open_store(directory: &Path, design: &mut DesignSystem) -> Store {
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
    store
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
    /// Cursors the application drew.
    cursors: AppCursors,
    /// Where the pointer comes back while it is locked.
    locked_at: Option<Vec2>,
    /// Where text is edited while text input is on.
    typing: Option<Rect>,
    // Field order is drop order: GPU objects (surface included) before the window.
    gpu: Gpu,
    window: Window,
}

impl KoebiApp {
    /// Applies what the last frame asked of the platform: a locked pointer (released where the
    /// interface says) and text input.
    fn apply_requests(&mut self, context: &mut AppContext) {
        let state = &self.interface.state;
        match (self.locked_at, state.pointer_lock()) {
            (None, Some(_)) => {
                if let Err(error) = context.lock_pointer(&self.window, true) {
                    koebi_core::warn!("cannot lock the pointer: {error}");
                }
            }
            (Some(at), None) => {
                if let Err(error) = context.lock_pointer(&self.window, false) {
                    koebi_core::warn!("cannot release the pointer: {error}");
                }
                context.warp_pointer(&self.window, at);
                self.interface.input.pointer_moved(at);
            }
            _ => {}
        }
        self.locked_at = self.interface.state.pointer_lock();
        self.interface.pointer_locked = self.locked_at.is_some();
        let area = self.interface.state.text_input_area();
        match (self.typing, area) {
            (None, Some(_)) => {
                if let Err(error) = context.start_text_input(&self.window, area) {
                    koebi_core::warn!("cannot start text input: {error}");
                }
            }
            (Some(old), Some(new)) if old != new => {
                context.set_text_input_area(&self.window, area);
            }
            (Some(_), None) => context.stop_text_input(&self.window),
            _ => {}
        }
        self.typing = area;
    }

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
        let redraw = self.interface.redraw_in();
        let rate = if self.options.frames.is_some() {
            IterationRate::Unlimited
        } else if redraw.is_some_and(|wait| wait < Duration::from_millis(20)) {
            // An animation: draw at the display's pace.
            IterationRate::Unlimited
        } else if redraw.is_some() {
            // A delay running out: check often enough to be on time.
            IterationRate::Hz(60)
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
        let fonts = Fonts::with_embedded();
        let interface = Interface::new(context, &options, fonts.clone())?;

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
        let gpu = Gpu::new(&window, &vulkan, options.present_mode, fonts)?;
        let cursors = AppCursors::create(context, window.scale_factor());
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
            cursors,
            locked_at: None,
            typing: None,
            gpu,
            window,
        })
    }

    fn iterate(&mut self, context: &mut AppContext) -> AppControl {
        if self
            .interface
            .redraw_in()
            .is_some_and(|wait| wait.is_zero())
        {
            self.needs_redraw = true;
        }
        if self.needs_redraw || self.options.frames.is_some() {
            let control = self.render_frame();
            if control != AppControl::Continue {
                return control;
            }
            context.set_cursor(self.cursors.cursor(self.interface.state.cursor()));
            self.apply_requests(context);
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
