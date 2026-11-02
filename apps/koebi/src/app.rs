//! The application state machine: SDL callbacks in, frames out, in every window.

use std::time::{Duration, Instant};

use koebi_core::{Rect, Vec2};
use koebi_gpu::RenderOutcome;
use koebi_platform::{
    App, AppContext, AppControl, AppError, Event, IterationRate, Window, WindowChrome, WindowDesc,
    WindowEvent, WindowId, WindowState, exit_code,
};
use koebi_text::Fonts;
use koebi_ui::WindowCommand;
use koebi_workspace::{EditorId, Workspace};

use crate::chrome::AppCommand;
use crate::cursors::AppCursors;
use crate::editors::{Editors, Entry, default_floating};
use crate::frame_times::FrameTimes;
use crate::gpu::Gpu;
use crate::interface::Interface;
use crate::options::{Options, StartWindow};
use crate::window::{AppWindow, WindowContent};

/// The smallest size of a window, in logical pixels: its bars and an editor still fit.
const MIN_WINDOW_SIZE: (u32, u32) = (480, 320);

/// The size a floating window opens at, in logical pixels.
const FLOATING_SIZE: (u32, u32) = (800, 600);

/// Koebi's application state.
pub(crate) struct KoebiApp {
    options: Options,
    interface: Interface,
    frames_presented: u64,
    frame_times: FrameTimes,
    /// The iteration rate last asked of the platform.
    rate: IterationRate,
    /// Cursors the application drew.
    cursors: AppCursors,
    // Field order is drop order: every window before what they share. The main window is first.
    windows: Vec<AppWindow>,
    gpu: Gpu,
}

impl KoebiApp {
    /// The workspace of the main window, what is saved.
    fn main_workspace(&self) -> Option<(&Workspace, &Editors)> {
        self.windows.first().and_then(AppWindow::workspace)
    }

    /// Saves what changed now.
    fn save(&mut self) {
        let Self {
            interface, windows, ..
        } = self;
        if let Some((workspace, editors)) = windows.first().and_then(AppWindow::workspace) {
            interface.save(workspace, editors);
        }
    }

    /// Applies what the bars asked for. Returns what the application does next.
    fn apply_commands(&mut self, context: &mut AppContext) -> AppControl {
        for index in (0..self.windows.len()).rev() {
            let Some(command) = self.windows[index].command.take() else {
                continue;
            };
            let window = &self.windows[index].window;
            match command {
                AppCommand::Quit => {
                    self.save();
                    return self.finish();
                }
                AppCommand::Window(WindowCommand::Close) => {
                    if self.windows[index].is_main {
                        self.save();
                        return self.finish();
                    }
                    self.close_window(index);
                    continue;
                }
                AppCommand::Window(WindowCommand::Minimize) => window.minimize(),
                AppCommand::Window(WindowCommand::ToggleMaximize) => match window.state() {
                    WindowState::Fullscreen => window.set_fullscreen(false),
                    WindowState::Maximized => window.restore(),
                    WindowState::Normal | WindowState::Minimized => window.maximize(),
                },
                AppCommand::ToggleFullscreen => {
                    window.set_fullscreen(window.state() != WindowState::Fullscreen);
                }
                AppCommand::SystemMenu(at) => window.show_system_menu(at),
                AppCommand::NewWindow => {
                    let (workspace, editors) = default_floating();
                    let content = WindowContent::workspace(workspace, editors);
                    if let Err(error) = self.open_window(context, content, None) {
                        koebi_core::warn!("cannot open a window: {}", error.message);
                    }
                }
            }
            if let Some(window) = self.windows.get_mut(index) {
                window.needs_redraw = true;
            }
        }
        AppControl::Continue
    }

    /// Opens a floating window showing `content`, its bar under `pointer` (global coordinates)
    /// if given. Returns its place among the windows.
    fn open_window(
        &mut self,
        context: &mut AppContext,
        content: WindowContent,
        pointer: Option<Vec2>,
    ) -> Result<usize, AppError> {
        let window = context
            .create_window(&WindowDesc {
                width: FLOATING_SIZE.0,
                height: FLOATING_SIZE.1,
                chrome: WindowChrome::Custom,
                min_size: MIN_WINDOW_SIZE,
                ..WindowDesc::default()
            })
            .map_err(|error| {
                AppError::new(
                    exit_code::FAILURE,
                    format!("cannot create a window: {error}"),
                )
            })?;
        if let Some(pointer) = pointer {
            window.center_top_at(pointer);
        }
        let gpu = self.gpu.for_window(&window)?;
        self.windows
            .push(AppWindow::new(false, window, gpu, content, &self.interface));
        Ok(self.windows.len() - 1)
    }

    /// Takes out of their windows the editors dropped outside them: into the window of the
    /// application under the pointer, or into a new one.
    fn apply_detached(&mut self, context: &mut AppContext) {
        for index in 0..self.windows.len() {
            let WindowContent::Workspace { view, .. } = &mut self.windows[index].content;
            if let Some(editor) = view.take_detached() {
                self.detach(context, index, editor);
                return;
            }
        }
    }

    /// Moves `editor` out of the window at `source`. It leaves only once it has found its place:
    /// when something fails, it stays where it was.
    fn detach(&mut self, context: &mut AppContext, source: usize, editor: EditorId) {
        let pointer = context.global_pointer();
        let target = (0..self.windows.len())
            .find(|&index| index != source && self.windows[index].window.contains_global(pointer));
        let WindowContent::Workspace {
            workspace, editors, ..
        } = &mut self.windows[source].content;
        let alone = workspace.editors().count() == 1;
        if target.is_none() && alone {
            // It is already alone in its window.
            return;
        }
        // Without it, if it can leave: the last editor of a window leaves with the window.
        let mut without = workspace.clone();
        if !alone && let Err(error) = without.remove_editor(editor) {
            koebi_core::warn!("cannot take the editor out: {error}");
            return;
        }
        let entry = editors.entry_of(editor);
        let placed = match target {
            Some(index) => self.add_to_window(index, pointer, entry),
            None => self.open_with(context, pointer, entry),
        };
        if !placed {
            return;
        }
        if alone && !self.windows[source].is_main {
            self.close_window(source);
            return;
        }
        let WindowContent::Workspace {
            workspace, editors, ..
        } = &mut self.windows[source].content;
        *workspace = without;
        editors.remove_editor(editor);
        self.windows[source].needs_redraw = true;
    }

    /// Adds an editor as a tab of the area under `pointer` in the window at `index`.
    fn add_to_window(&mut self, index: usize, pointer: Vec2, entry: Entry) -> bool {
        let shown = &mut self.windows[index];
        let at = shown.window.global_to_logical(pointer);
        let WindowContent::Workspace {
            workspace,
            view,
            editors,
        } = &mut shown.content;
        let Some(area) = view.area_at(at).or_else(|| first_area(workspace)) else {
            return false;
        };
        match workspace.add_tab(area) {
            Ok(added) => editors.adopt(added, entry),
            Err(error) => {
                koebi_core::warn!("cannot add the tab: {error}");
                return false;
            }
        }
        shown.window.raise();
        shown.needs_redraw = true;
        true
    }

    /// Opens a floating window holding an editor, under `pointer`.
    fn open_with(&mut self, context: &mut AppContext, pointer: Vec2, entry: Entry) -> bool {
        let (workspace, mut editors) = default_floating();
        if let Some(first) = workspace.editors().next() {
            editors.adopt(first, entry);
        }
        let content = WindowContent::workspace(workspace, editors);
        match self.open_window(context, content, Some(pointer)) {
            Ok(_) => true,
            Err(error) => {
                koebi_core::warn!("cannot open a window: {}", error.message);
                false
            }
        }
    }

    /// Closes the window at `index`. Keys and buttons held in it are forgotten: their releases
    /// will reach no window.
    fn close_window(&mut self, index: usize) {
        self.windows.remove(index);
        self.interface.keymap.cancel_presses();
    }

    /// Draws and presents a frame of the window at `index`.
    fn render_window(&mut self, index: usize) -> AppControl {
        let Self {
            interface,
            windows,
            frame_times,
            frames_presented,
            gpu,
            ..
        } = self;
        let shown = &mut windows[index];
        let scale_factor = shown.window.scale_factor();
        let logical_size = shown.window.size_in_pixels().to_logical(scale_factor);
        let started = Instant::now();
        shown.state.set_scale_factor(scale_factor);
        shown.build(
            interface,
            Rect::from_origin_size(Vec2::ZERO, logical_size),
            frame_times,
        );
        shown.chrome.update_regions(&shown.state, &mut shown.window);
        let built = started.elapsed();
        match shown.gpu.render(gpu, &shown.scene, scale_factor) {
            Ok((RenderOutcome::Presented, recording)) => {
                if shown.is_main {
                    let gpu_time = shown.gpu.gpu_frame_time();
                    frame_times.record(built + recording, gpu_time, Instant::now());
                    *frames_presented += 1;
                }
                shown.needs_redraw = false;
            }
            // Minimized or swapchain recreated: keep `needs_redraw` and try again.
            Ok((RenderOutcome::Skipped, _)) => {}
            Err(koebi_gpu::Error::SurfaceLost) => {
                koebi_core::warn!("window surface lost, recreating it");
                shown.gpu.detach();
                if let Err(error) = shown.gpu.reattach(gpu, &shown.window) {
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
        let now = self.interface.now();
        let redraw = self
            .windows
            .iter()
            .filter(|window| window.window.state() != WindowState::Minimized)
            .filter_map(|window| window.redraw_in(now))
            .min();
        let unsaved = self.main_workspace().is_some_and(|(workspace, editors)| {
            self.interface.has_unsaved_changes(workspace, editors)
        });
        let rate = if self.options.frames.is_some() {
            IterationRate::Unlimited
        } else if redraw.is_some_and(|wait| wait < Duration::from_millis(20)) {
            // An animation: draw at the display's pace.
            IterationRate::Unlimited
        } else if redraw.is_some() {
            // A delay running out: check often enough to be on time.
            IterationRate::Hz(60)
        } else if unsaved {
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

    /// The place of the window `id` among the windows.
    fn window_index(&self, id: WindowId) -> Option<usize> {
        self.windows.iter().position(|window| window.id() == id)
    }
}

/// The first area of `workspace`.
fn first_area(workspace: &Workspace) -> Option<koebi_workspace::AreaId> {
    workspace.layout().areas().next().map(|(area, _)| area)
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
        let (interface, workspace, editors) = Interface::new(context, &options, fonts.clone())?;

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
        let window = main_window(context, &options)?;
        let (gpu, window_gpu) = Gpu::new(&window, &vulkan, options.present_mode, fonts)?;
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
        let content = WindowContent::workspace(workspace, editors);
        let main = AppWindow::new(true, window, window_gpu, content, &interface);
        let open = options.open_window;
        let mut app = Self {
            options,
            interface,
            frames_presented: 0,
            frame_times: FrameTimes::new(),
            rate,
            cursors,
            windows: vec![main],
            gpu,
        };
        if open == Some(StartWindow::Floating) {
            let (workspace, editors) = default_floating();
            let content = WindowContent::workspace(workspace, editors);
            app.open_window(context, content, None)?;
        }
        Ok(app)
    }

    fn iterate(&mut self, context: &mut AppContext) -> AppControl {
        let now = self.interface.now();
        let revision = self.interface.design.revision();
        let continuous = self.options.frames.is_some();
        for index in 0..self.windows.len() {
            let window = &self.windows[index];
            // A minimized window has nothing to show: it is drawn when it comes back, which asks
            // for a frame. Trying would recreate its swapchain, waiting for the GPU every time.
            let shown = window.window.state() != WindowState::Minimized;
            let due = window.needs_redraw
                || continuous
                // Another window changed the design system.
                || window.drawn_revision != revision
                || window.redraw_in(now).is_some_and(|wait| wait.is_zero());
            if !due || !shown {
                continue;
            }
            let control = self.render_window(index);
            if control != AppControl::Continue {
                return control;
            }
            let Self {
                windows, cursors, ..
            } = self;
            let window = &mut windows[index];
            // One cursor for the application: the one the window under the pointer asks for.
            if window.input.pointer().is_some() {
                context.set_cursor(cursors.cursor(window.state.cursor()));
            }
            window.apply_requests(context);
        }
        let control = self.apply_commands(context);
        if control != AppControl::Continue {
            return control;
        }
        self.apply_detached(context);
        let Self {
            interface, windows, ..
        } = self;
        if let Some((workspace, editors)) = windows.first().and_then(AppWindow::workspace) {
            interface.save_if_due(workspace, editors);
        }
        self.update_rate(context);
        match self.options.frames {
            Some(limit) if self.frames_presented >= limit => self.finish(),
            _ => AppControl::Continue,
        }
    }

    fn event(&mut self, _context: &mut AppContext, event: &Event) -> AppControl {
        match *event {
            Event::Quit | Event::Terminating => {
                self.save();
                return self.finish();
            }
            // The OS may kill a background application without notice: save first. Android
            // also destroys the native window: release the surface.
            Event::WillEnterBackground => {
                self.save();
                for window in &mut self.windows {
                    window.gpu.detach();
                }
                return AppControl::Continue;
            }
            Event::DidEnterForeground => {
                let Self { windows, gpu, .. } = self;
                for window in windows {
                    if let Err(error) = window.gpu.reattach(gpu, &window.window) {
                        koebi_core::error!("{}", error.message);
                        return AppControl::Exit(error.exit_code);
                    }
                    window.needs_redraw = true;
                }
                return AppControl::Continue;
            }
            _ => {}
        }
        let Some(index) = event.window().and_then(|id| self.window_index(id)) else {
            return AppControl::Continue;
        };
        let Self {
            interface, windows, ..
        } = self;
        let shown = &mut windows[index];
        if shown.handle(interface, event) {
            shown.needs_redraw = true;
            return AppControl::Continue;
        }
        if let Event::Window { event, .. } = *event {
            match event {
                WindowEvent::CloseRequested => shown.command = Some(close_command()),
                WindowEvent::PixelSizeChanged(extent) => shown.gpu.resize(extent),
                WindowEvent::FullscreenEntered | WindowEvent::FullscreenLeft => {
                    shown.window.refresh_chrome();
                }
                WindowEvent::RegionPressed => shown.window.region_pressed(),
                _ => {}
            }
            shown.needs_redraw = true;
        }
        AppControl::Continue
    }

    fn quit(mut self, _context: &mut AppContext) {
        self.save();
        if let Err(error) = self.gpu.device.wait_idle() {
            koebi_core::warn!("waiting for the GPU at exit failed: {error}");
        }
        koebi_core::info!("Koebi exits after {} frames", self.frames_presented);
    }
}

/// What the system asking to close a window does: what its close button does.
const fn close_command() -> AppCommand {
    AppCommand::Window(WindowCommand::Close)
}

/// Creates the main window, in the state the command line asks for.
fn main_window(context: &mut AppContext, options: &Options) -> Result<Window, AppError> {
    let window = context
        .create_window(&WindowDesc {
            chrome: WindowChrome::Custom,
            min_size: MIN_WINDOW_SIZE,
            ..WindowDesc::default()
        })
        .map_err(|error| {
            AppError::new(
                exit_code::UNSUPPORTED,
                format!("cannot create the window: {error}"),
            )
        })?;
    match options.window_state {
        WindowState::Maximized => window.maximize(),
        WindowState::Fullscreen => window.set_fullscreen(true),
        WindowState::Minimized | WindowState::Normal => {}
    }
    Ok(window)
}
