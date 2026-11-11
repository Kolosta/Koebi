//! The application state machine: SDL callbacks in, frames out, in every window.
//!
//! Windows are main windows or floating ones ([`Role`]). The application quits when its last
//! main window closes; a main window's floating windows close with it. The workspace saved is
//! the one of the oldest main window open. `Window > New Instance` starts another process on a
//! workspace of its own (`--untitled`), sharing the design state and the shortcuts.

use std::path::Path;
use std::process::{Command, Stdio};
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

use crate::app_icon::AppIcon;
use crate::chrome::AppCommand;
use crate::cursors::AppCursors;
use crate::editors::{Editors, Entry, default_floating};
use crate::frame_times::FrameTimes;
use crate::gpu::Gpu;
use crate::interface::Interface;
use crate::options::{Options, StartWindow};
use crate::settings::SettingsView;
use crate::window::{AppWindow, Role, WindowContent, closed_with};

/// The smallest size of a window, in logical pixels: its bars and an editor still fit.
const MIN_WINDOW_SIZE: (u32, u32) = (480, 320);

/// The size a floating window opens at, in logical pixels.
const FLOATING_SIZE: (u32, u32) = (800, 600);

/// The size the settings window opens at, in logical pixels.
const SETTINGS_SIZE: (u32, u32) = (960, 680);

/// Mobile platforms show one window: the settings show inside it.
const MULTIPLE_WINDOWS: bool = !cfg!(any(target_os = "android", target_os = "ios"));

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
    /// The system's Korean and Chinese fonts, loaded off the main thread at start-up.
    system_fonts: Option<std::sync::mpsc::Receiver<Vec<koebi_text::Font>>>,
    /// The icon its windows show.
    icon: AppIcon,
    // Field order is drop order: every window before what they share.
    windows: Vec<AppWindow>,
    gpu: Gpu,
}

impl KoebiApp {
    /// The workspace of the oldest main window open, what is saved.
    fn main_workspace(&self) -> Option<(&Workspace, &Editors)> {
        saved_workspace(&self.windows)
    }

    /// Saves what changed now.
    fn save(&mut self) {
        let Self {
            interface, windows, ..
        } = self;
        if let Some((workspace, editors)) = saved_workspace(windows) {
            interface.save(workspace, editors);
        }
    }

    /// The main window the window at `index` belongs to: what a window opened from it belongs
    /// to.
    fn owner_of(&self, index: usize) -> Option<WindowId> {
        self.windows.get(index).and_then(AppWindow::owner)
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
                    let roles: Vec<(WindowId, Role)> = self
                        .windows
                        .iter()
                        .map(|window| (window.id(), window.role))
                        .collect();
                    let Some(closed) = closed_with(&roles, index) else {
                        self.save();
                        return self.finish();
                    };
                    // The workspace saved may be the next main window's: save this one first.
                    if self.windows[index].is_main() {
                        self.save();
                    }
                    for window in closed {
                        self.close_window(window);
                    }
                    // Windows after it moved down: the loop goes on from the next lower one.
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
                    let role = Role::Floating {
                        owner: self.owner_of(index),
                    };
                    if let Err(error) =
                        self.open_window(context, content, None, FLOATING_SIZE, role)
                    {
                        koebi_core::warn!("cannot open a window: {}", error.message);
                    }
                }
                AppCommand::NewMainWindow => {
                    if let Err(error) = self.open_main_window(context, index) {
                        koebi_core::warn!("cannot open a window: {}", error.message);
                    }
                }
                AppCommand::NewInstance => self.start_instance(),
                AppCommand::OpenSettings => {
                    if let Err(error) = self.open_settings(context) {
                        koebi_core::warn!("cannot open the settings: {}", error.message);
                    }
                }
            }
            if let Some(window) = self.windows.get_mut(index) {
                window.needs_redraw = true;
            }
        }
        AppControl::Continue
    }

    /// Opens the settings window, brings it to the front if it is open; on platforms with one
    /// window, shows the settings in it or takes them away.
    fn open_settings(&mut self, context: &mut AppContext) -> Result<(), AppError> {
        if !MULTIPLE_WINDOWS {
            if let Some(main) = self.windows.first_mut() {
                main.settings_in_place = match main.settings_in_place {
                    Some(_) => None,
                    None => Some(Box::new(SettingsView::new())),
                };
            }
            return Ok(());
        }
        let open = self
            .windows
            .iter()
            .find(|window| matches!(window.content, WindowContent::Settings(_)));
        if let Some(window) = open {
            window.window.raise();
            return Ok(());
        }
        let content = WindowContent::Settings(Box::new(SettingsView::new()));
        // The application's own: it belongs to no main window.
        let role = Role::Floating { owner: None };
        self.open_window(context, content, None, SETTINGS_SIZE, role)?;
        Ok(())
    }

    /// Opens another main window, of the same document: the workspace of the window at `index`
    /// (or of the oldest main window) as it is now.
    fn open_main_window(&mut self, context: &mut AppContext, index: usize) -> Result<(), AppError> {
        let (workspace, editors) = self
            .windows
            .get(index)
            .and_then(AppWindow::workspace)
            .or_else(|| self.main_workspace())
            .map_or_else(crate::editors::default_workspace, |(workspace, editors)| {
                (workspace.clone(), editors.clone())
            });
        let content = WindowContent::workspace(workspace, editors);
        // The size the first main window opens at.
        let desc = WindowDesc::default();
        let size = (desc.width, desc.height);
        self.open_window(context, content, None, size, Role::Main)?;
        Ok(())
    }

    /// Starts another instance of the application, on a workspace of its own.
    fn start_instance(&self) {
        let started =
            std::env::current_exe().and_then(|exe| instance_command(&exe, &self.options).spawn());
        match started {
            Ok(child) => koebi_core::info!("another instance started (process {})", child.id()),
            Err(error) => koebi_core::warn!("cannot start another instance: {error}"),
        }
    }

    /// Opens a window of `size` and `role` showing `content`, its bar under `pointer` (global
    /// coordinates) if given. Returns its place among the windows.
    fn open_window(
        &mut self,
        context: &mut AppContext,
        content: WindowContent,
        pointer: Option<Vec2>,
        size: (u32, u32),
        role: Role,
    ) -> Result<usize, AppError> {
        let window = context
            .create_window(&WindowDesc {
                width: size.0,
                height: size.1,
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
        self.icon.apply(&window);
        let gpu = self.gpu.for_window(&window)?;
        self.windows
            .push(AppWindow::new(role, window, gpu, content, &self.interface));
        Ok(self.windows.len() - 1)
    }

    /// Takes out of their windows the editors dropped outside them: into the window of the
    /// application under the pointer, or into a new one.
    fn apply_detached(&mut self, context: &mut AppContext) {
        for index in 0..self.windows.len() {
            if let WindowContent::Workspace { view, .. } = &mut self.windows[index].content
                && let Some(editor) = view.take_detached()
            {
                self.detach(context, index, editor);
                return;
            }
        }
    }

    /// Moves `editor` out of the window at `source`. It leaves only once it has found its place:
    /// when something fails, it stays where it was.
    fn detach(&mut self, context: &mut AppContext, source: usize, editor: EditorId) {
        let pointer = context.global_pointer();
        let target = (0..self.windows.len()).find(|&index| {
            index != source
                && self.windows[index].workspace().is_some()
                && self.windows[index].window.contains_global(pointer)
        });
        let WindowContent::Workspace {
            workspace, editors, ..
        } = &mut self.windows[source].content
        else {
            return;
        };
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
        let owner = self.owner_of(source);
        let placed = match target {
            Some(index) => self.add_to_window(index, pointer, entry),
            None => self.open_with(context, pointer, entry, owner),
        };
        if !placed {
            return;
        }
        if alone && !self.windows[source].is_main() {
            self.close_window(source);
            return;
        }
        let WindowContent::Workspace {
            workspace, editors, ..
        } = &mut self.windows[source].content
        else {
            return;
        };
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
        } = &mut shown.content
        else {
            return false;
        };
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

    /// Opens a floating window of the main window `owner` holding an editor, under `pointer`.
    fn open_with(
        &mut self,
        context: &mut AppContext,
        pointer: Vec2,
        entry: Entry,
        owner: Option<WindowId>,
    ) -> bool {
        let (workspace, mut editors) = default_floating();
        if let Some(first) = workspace.editors().next() {
            editors.adopt(first, entry);
        }
        let content = WindowContent::workspace(workspace, editors);
        let role = Role::Floating { owner };
        match self.open_window(context, content, Some(pointer), FLOATING_SIZE, role) {
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
        // Fonts added since its last frame: a system font, the system's fallbacks.
        if shown.fonts_revision != interface.fonts_revision() {
            shown.state.set_fonts(interface.fonts.clone());
            shown.gpu.set_fonts(interface.fonts.clone());
            gpu.set_fonts(interface.fonts.clone());
            shown.fonts_revision = interface.fonts_revision();
        }
        let scale_factor = shown.window.scale_factor();
        let logical_size = shown.window.size_in_pixels().to_logical(scale_factor);
        let started = Instant::now();
        shown.state.set_scale_factor(scale_factor);
        // What an eyedropper read back from a frame shown earlier, then what it wants next.
        shown.state.set_screen_readable(shown.gpu.can_read_pixels());
        shown.gpu.deliver_pixels(&mut shown.state);
        shown.build(
            interface,
            Rect::from_origin_size(Vec2::ZERO, logical_size),
            frame_times,
        );
        if let Some(center) = shown.state.take_pixels_wanted() {
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a pointer inside the window, in physical pixels"
            )]
            let at = (
                (center.x * scale_factor).max(0.0) as u32,
                (center.y * scale_factor).max(0.0) as u32,
            );
            shown.gpu.request_pixels(at, koebi_ui::EYEDROPPER_PIXELS);
        }
        shown.chrome.update_regions(&shown.state, &mut shown.window);
        let built = started.elapsed();
        match shown.gpu.render(gpu, &shown.scene, scale_factor) {
            Ok((RenderOutcome::Presented, recording)) => {
                if shown.is_main() {
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

    /// Adds the fonts that arrived: the system's fallbacks once loaded, a system font the
    /// gallery asked for.
    fn take_fonts(&mut self) {
        if let Some(receiver) = &self.system_fonts {
            match receiver.try_recv() {
                Ok(fonts) => {
                    koebi_core::info!("{} system fonts for Korean and Chinese", fonts.len());
                    for font in fonts {
                        let family = font.family().to_owned();
                        self.interface.add_font(&family, font, true);
                    }
                    self.system_fonts = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.system_fonts = None,
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        if let Some(request) = self.interface.content.gallery.typography.take_request() {
            match request.load() {
                Ok(font) => {
                    koebi_core::info!(
                        "font {} loaded from {}",
                        request.family,
                        request.path.display()
                    );
                    self.interface.add_font(&request.family, font, false);
                }
                Err(error) => koebi_core::warn!("cannot load {}: {error}", request.family),
            }
        }
    }

    /// Waits for events, except while a window has to draw (see [`iteration_rate`]) or
    /// changes wait to be saved. Tests and benchmarks draw continuously.
    fn update_rate(&mut self, context: &mut AppContext) {
        let now = self.interface.now();
        let revision = self.interface.revision();
        let shown = || {
            self.windows
                .iter()
                .filter(|window| window.window.state() != WindowState::Minimized)
        };
        let due = shown().any(|window| window.is_due(revision, now));
        let redraw = shown().filter_map(|window| window.redraw_in(now)).min();
        let unsaved = self.main_workspace().is_some_and(|(workspace, editors)| {
            self.interface.has_unsaved_changes(workspace, editors)
        });
        let rate = iteration_rate(self.options.frames.is_some(), due, redraw, unsaved);
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
/// How soon the application iterates again. `due`: a window has to draw now, because a
/// window drawn later in the last iteration changed what it shows (a setting turning the
/// overlay on, a token) or a command asked for it: the next iteration comes at once instead of
/// waiting for an event of that window. `redraw`: the nearest delay of a window's own.
fn iteration_rate(
    continuous: bool,
    due: bool,
    redraw: Option<Duration>,
    unsaved: bool,
) -> IterationRate {
    if continuous || due {
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
    }
}

/// Loads the system's Korean and Chinese fonts on a thread of their own, on desktop platforms:
/// tens of megabytes read without delaying the first frame. The embedded fonts cover Latin and
/// Japanese meanwhile.
fn start_system_fonts() -> Option<std::sync::mpsc::Receiver<Vec<koebi_text::Font>>> {
    if cfg!(any(target_os = "android", target_os = "ios")) {
        return None;
    }
    let (sender, receiver) = std::sync::mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("system fonts".to_owned())
        .spawn(move || {
            let fonts: Vec<koebi_text::Font> = koebi_text::system::cjk_fallbacks()
                .into_iter()
                .filter_map(|path| {
                    let data = std::fs::read(&path).ok()?;
                    koebi_text::Font::from_collection(data, 0).ok()
                })
                .collect();
            // The application may have quit meanwhile: nobody to tell.
            drop(sender.send(fonts));
        });
    match spawned {
        Ok(_) => Some(receiver),
        Err(error) => {
            koebi_core::warn!("system fonts not loaded: {error}");
            None
        }
    }
}

fn first_area(workspace: &Workspace) -> Option<koebi_workspace::AreaId> {
    workspace.layout().areas().next().map(|(area, _)| area)
}

impl App for KoebiApp {
    type Options = Options;

    fn parse(args: &[String]) -> Result<Options, AppError> {
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
        Ok(options)
    }

    fn init(context: &mut AppContext, options: Options) -> Result<Self, AppError> {
        // A UI only needs to draw after input; benchmarks and tests draw continuously.
        let rate = if options.frames.is_some() {
            IterationRate::Unlimited
        } else {
            IterationRate::WaitEvent
        };
        context.set_iteration_rate(rate);
        let fonts = Fonts::with_embedded();
        let (interface, workspace, editors) = Interface::new(context, &options, fonts.clone())?;
        let system_fonts = start_system_fonts();

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
        let icon = AppIcon::new();
        icon.apply(&window);
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
        let main = AppWindow::new(Role::Main, window, window_gpu, content, &interface);
        let open = options.open_window;
        let mut app = Self {
            options,
            interface,
            frames_presented: 0,
            frame_times: FrameTimes::new(),
            rate,
            cursors,
            system_fonts,
            icon,
            windows: vec![main],
            gpu,
        };
        match open {
            Some(StartWindow::Floating) => {
                let (workspace, editors) = default_floating();
                let content = WindowContent::workspace(workspace, editors);
                let role = Role::Floating {
                    owner: app.owner_of(0),
                };
                app.open_window(context, content, None, FLOATING_SIZE, role)?;
            }
            Some(StartWindow::Main) => app.open_main_window(context, 0)?,
            Some(StartWindow::Settings) => app.open_settings(context)?,
            None => {}
        }
        Ok(app)
    }

    fn iterate(&mut self, context: &mut AppContext) -> AppControl {
        let now = self.interface.now();
        let revision = self.interface.revision();
        let continuous = self.options.frames.is_some();
        for index in 0..self.windows.len() {
            let window = &self.windows[index];
            // A minimized window has nothing to show: it is drawn when it comes back, which asks
            // for a frame. Trying would recreate its swapchain, waiting for the GPU every time.
            let shown = window.window.state() != WindowState::Minimized;
            // Another window may have changed the design system or the overlay.
            let due = continuous || window.is_due(revision, now);
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
        route_screen_pick(&mut self.windows);
        self.take_fonts();
        let control = self.apply_commands(context);
        if control != AppControl::Continue {
            return control;
        }
        self.apply_detached(context);
        let Self {
            interface, windows, ..
        } = self;
        if let Some((workspace, editors)) = saved_workspace(windows) {
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

/// An eyedropper started in one window picks in every window: the others learn there is a
/// session (and draw again), and what one of them picks goes to the window that started it.
fn route_screen_pick(windows: &mut [AppWindow]) {
    let owner = windows.iter().position(|window| window.state.picks_here());
    for (index, window) in windows.iter_mut().enumerate() {
        let elsewhere = owner.is_some_and(|owner| owner != index);
        if window.state.set_pick_elsewhere(elsewhere) {
            window.needs_redraw = true;
        }
    }
    let Some(owner) = owner else {
        return;
    };
    for index in 0..windows.len() {
        if index != owner
            && let Some(outcome) = windows[index].state.take_foreign_pick()
        {
            windows[owner].state.deliver_pick(outcome);
            windows[owner].needs_redraw = true;
        }
    }
}

/// The workspace of the oldest main window among `windows`: what is saved.
fn saved_workspace(windows: &[AppWindow]) -> Option<(&Workspace, &Editors)> {
    windows
        .iter()
        .find(|window| window.is_main())
        .and_then(AppWindow::workspace)
}

/// The command starting another instance of the application at `exe`, on a workspace of its
/// own, in the data directory `options` names: its design state and shortcuts are shared.
fn instance_command(exe: &Path, options: &Options) -> Command {
    let mut command = Command::new(exe);
    command.arg("--untitled");
    if let Some(dir) = &options.data_dir {
        command.arg("--data-dir").arg(dir);
    }
    command.stdin(Stdio::null());
    command
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

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn another_instance_starts_untitled_in_the_same_data_directory() {
        let exe = Path::new("koebi");
        let args = |options: &Options| -> Vec<String> {
            instance_command(exe, options)
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect()
        };
        let command = instance_command(exe, &Options::default());
        assert_eq!(command.get_program(), OsStr::new("koebi"));
        assert_eq!(args(&Options::default()), ["--untitled"]);
        let options = Options {
            data_dir: Some(PathBuf::from("state")),
            frames: Some(3),
            ..Options::default()
        };
        // The data directory only: the other options were this instance's.
        assert_eq!(args(&options), ["--untitled", "--data-dir", "state"]);
    }

    #[test]
    fn a_window_due_to_draw_does_not_wait_for_an_event() {
        // Regression: the settings window turned the overlay on after the main window's turn
        // in the iteration, and the application then waited for an event before drawing the
        // main window again: the overlay appeared once the pointer moved over it.
        assert_eq!(
            iteration_rate(false, true, None, false),
            IterationRate::Unlimited
        );
        assert_eq!(
            iteration_rate(false, false, None, false),
            IterationRate::WaitEvent
        );
        assert_eq!(
            iteration_rate(false, false, Some(Duration::from_millis(200)), false),
            IterationRate::Hz(60)
        );
        assert_eq!(
            iteration_rate(false, false, None, true),
            IterationRate::Hz(10)
        );
        assert_eq!(
            iteration_rate(true, false, None, false),
            IterationRate::Unlimited
        );
    }
}
