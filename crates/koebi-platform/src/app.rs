//! Application lifecycle on top of SDL3's main callbacks.

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fmt;
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI32, Ordering};

use koebi_core::{Rect, Vec2};
use sdl3_sys::clipboard::{SDL_GetClipboardText, SDL_SetClipboardText};
use sdl3_sys::events::SDL_Event;
use sdl3_sys::filesystem::SDL_GetPrefPath;
use sdl3_sys::hints::{
    SDL_HINT_MAIN_CALLBACK_RATE, SDL_HINT_MOUSE_RELATIVE_SYSTEM_SCALE, SDL_SetHint,
};
use sdl3_sys::init::{
    SDL_APP_CONTINUE, SDL_APP_FAILURE, SDL_APP_SUCCESS, SDL_AppResult, SDL_INIT_VIDEO, SDL_Init,
    SDL_SetAppMetadata,
};
use sdl3_sys::keyboard::{
    SDL_GetModState, SDL_SetTextInputArea, SDL_StartTextInput, SDL_StopTextInput,
};
use sdl3_sys::main::{SDL_EnterAppMainCallbacks, SDL_RunApp};
use sdl3_sys::mouse::{SDL_SetWindowRelativeMouseMode, SDL_WarpMouseInWindow};
use sdl3_sys::rect::SDL_Rect;
use sdl3_sys::stdinc::SDL_free;

use crate::cursor::{Cursor, CursorId, CursorImage, Cursors};
use crate::error::{PlatformError, sdl_error};
use crate::event::{Event, modifiers_from_sdl, text_chunks, text_input, to_logical, translate};
use crate::vulkan::VulkanSupport;
use crate::window::{Window, WindowDesc, logical_per_window_coordinate};

/// Process exit codes shared by every Koebi binary, its tests and CI scripts.
pub mod exit_code {
    /// Clean exit.
    pub const SUCCESS: i32 = 0;
    /// Runtime failure (bug, device lost...).
    pub const FAILURE: i32 = 1;
    /// The environment cannot run Koebi (no display, no Vulkan, GPU too old).
    pub const UNSUPPORTED: i32 = 2;
    /// The run completed but Vulkan validation reported errors.
    pub const VALIDATION_ERRORS: i32 = 3;
    /// Invalid command-line arguments (`EX_USAGE`).
    pub const USAGE: i32 = 64;
}

/// What the application wants after a callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppControl {
    /// Keep running.
    Continue,
    /// Quit with this exit code (see [`exit_code`]).
    Exit(i32),
}

/// An initialization failure: logged, then the process exits with `exit_code`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppError {
    /// Process exit code (see [`exit_code`]).
    pub exit_code: i32,
    /// Human-readable reason.
    pub message: String,
}

impl AppError {
    /// Creates an error.
    pub fn new(exit_code: i32, message: impl Into<String>) -> Self {
        Self {
            exit_code,
            message: message.into(),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (exit code {})", self.message, self.exit_code)
    }
}

impl std::error::Error for AppError {}

/// How often SDL calls [`App::iterate`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IterationRate {
    /// Only after events arrive: zero CPU when idle. Best for UI.
    WaitEvent,
    /// As fast as possible (presentation pacing still applies).
    Unlimited,
    /// A fixed rate in Hz.
    Hz(u32),
}

impl IterationRate {
    fn hint_value(self) -> CString {
        let text = match self {
            Self::WaitEvent => String::from("waitevent"),
            Self::Unlimited => String::from("0"),
            Self::Hz(hz) => hz.max(1).to_string(),
        };
        CString::new(text).unwrap_or_default()
    }
}

/// An application driven by SDL's main callbacks. All methods run on the main thread.
pub trait App: Sized + 'static {
    /// Creates the application. SDL video is initialized; create windows here.
    ///
    /// # Errors
    ///
    /// Returning an error quits with [`AppError::exit_code`].
    fn init(context: &mut AppContext, args: &[String]) -> Result<Self, AppError>;

    /// Called once per frame (see [`IterationRate`]).
    fn iterate(&mut self, context: &mut AppContext) -> AppControl;

    /// Called for every event, before the next `iterate`.
    fn event(&mut self, context: &mut AppContext, event: &Event) -> AppControl;

    /// Called once before SDL shuts down. Release GPU resources before windows.
    fn quit(self, context: &mut AppContext);
}

/// Platform services available to an [`App`].
///
/// Only exists between SDL initialization and shutdown, on the main thread.
#[derive(Debug)]
pub struct AppContext {
    cursors: Cursors,
    _main_thread_only: PhantomData<*mut ()>,
}

#[allow(
    clippy::unused_self,
    reason = "the context is a capability: holding it proves SDL is initialized on this thread"
)]
impl AppContext {
    fn new() -> Self {
        Self {
            cursors: Cursors::default(),
            _main_thread_only: PhantomData,
        }
    }

    /// Creates a window able to host a Vulkan surface.
    ///
    /// # Errors
    ///
    /// Returns the SDL error (e.g. no video device).
    pub fn create_window(&mut self, desc: &WindowDesc<'_>) -> Result<Window, PlatformError> {
        Window::new(desc)
    }

    /// Loads the Vulkan library through SDL. `library = None` uses SDL's default search;
    /// pass `koebi_gpu::loader::preferred_library_path()` to honor Koebi's overrides.
    ///
    /// Call it before [`Self::create_window`]: creating the first Vulkan window otherwise
    /// makes SDL load its default library, ignoring `library`.
    ///
    /// # Errors
    ///
    /// Returns the SDL error when no Vulkan loader can be opened.
    pub fn load_vulkan(&mut self, library: Option<&str>) -> Result<VulkanSupport, PlatformError> {
        VulkanSupport::load(library)
    }

    /// The directory where the application keeps its settings and user data, created if
    /// needed: `%APPDATA%\Kolosta\Koebi\` on Windows, `~/.local/share/Kolosta/Koebi/` on
    /// Linux, `~/Library/Application Support/Kolosta/Koebi/` on macOS, the application's
    /// private storage on Android and iOS.
    ///
    /// # Errors
    ///
    /// Returns the SDL error when the directory cannot be found or created.
    pub fn preferences_dir(&self) -> Result<PathBuf, PlatformError> {
        // SAFETY: both arguments are valid NUL-terminated strings.
        let raw = unsafe { SDL_GetPrefPath(c"Kolosta".as_ptr(), c"Koebi".as_ptr()) };
        if raw.is_null() {
            return Err(PlatformError::from_sdl("SDL_GetPrefPath"));
        }
        // SAFETY: SDL returned a non-null, NUL-terminated UTF-8 string.
        let path = unsafe { CStr::from_ptr(raw) }
            .to_string_lossy()
            .into_owned();
        // SAFETY: the string was allocated by SDL for the caller and is not used afterwards.
        unsafe { SDL_free(raw.cast()) };
        Ok(PathBuf::from(path))
    }

    /// Shows `cursor` over the application's windows. Setting the cursor already shown costs
    /// nothing: call it every frame with the cursor the pointer's position calls for.
    ///
    /// Drags keep receiving pointer events outside the window: SDL captures the mouse while a
    /// button is held.
    pub fn set_cursor(&mut self, cursor: Cursor) {
        if let Err(error) = self.cursors.show(cursor) {
            koebi_core::warn!("cannot show the cursor: {error}");
        }
    }

    /// Creates a custom cursor from pixels, to show with [`Self::set_cursor`]. Cursors live as
    /// long as the context.
    ///
    /// # Errors
    ///
    /// When the image is invalid (see [`CursorImage`]) or SDL cannot create the cursor.
    pub fn create_cursor(&mut self, image: &CursorImage<'_>) -> Result<CursorId, PlatformError> {
        self.cursors.create(image)
    }

    /// Locks the pointer in `window`, or releases it. While locked, the cursor disappears and
    /// stays put, nothing under it is pointed at, and [`Event::MouseMotion`] keeps reporting
    /// relative motion (`delta`) at the speed of the system's pointer, its position
    /// unchanged. Release it, then [`Self::warp_pointer`] to where the cursor comes back.
    ///
    /// # Errors
    ///
    /// Returns the SDL error when the platform cannot lock the pointer.
    pub fn lock_pointer(&mut self, window: &Window, locked: bool) -> Result<(), PlatformError> {
        // SAFETY: `window` is a live window, on the main thread.
        if unsafe { SDL_SetWindowRelativeMouseMode(window.raw(), locked) } {
            Ok(())
        } else {
            Err(PlatformError::from_sdl("SDL_SetWindowRelativeMouseMode"))
        }
    }

    /// Moves the pointer to `position`, in logical pixels, in `window`.
    pub fn warp_pointer(&mut self, window: &Window, position: Vec2) {
        let scale = logical_per_window_coordinate(window.id());
        let at = position / scale;
        // SAFETY: `window` is a live window, on the main thread.
        unsafe { SDL_WarpMouseInWindow(window.raw(), at.x, at.y) };
    }

    /// Starts text input in `window`: typed text arrives as [`Event::TextInput`], and touch
    /// devices show their on-screen keyboard. `area`, in logical pixels, is where the text is
    /// edited: input methods place their windows beside it.
    ///
    /// # Errors
    ///
    /// Returns the SDL error when text input cannot start.
    pub fn start_text_input(
        &mut self,
        window: &Window,
        area: Option<Rect>,
    ) -> Result<(), PlatformError> {
        self.set_text_input_area(window, area);
        // SAFETY: `window` is a live window, on the main thread.
        if unsafe { SDL_StartTextInput(window.raw()) } {
            Ok(())
        } else {
            Err(PlatformError::from_sdl("SDL_StartTextInput"))
        }
    }

    /// Moves the area where text is edited (see [`Self::start_text_input`]).
    pub fn set_text_input_area(&mut self, window: &Window, area: Option<Rect>) {
        let scale = logical_per_window_coordinate(window.id());
        let rect = area.map(|area| {
            let round = |value: f32| {
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "window coordinates, far inside the i32 range"
                )]
                let value = (value / scale).round() as i32;
                value
            };
            SDL_Rect {
                x: round(area.min.x),
                y: round(area.min.y),
                w: round(area.width()),
                h: round(area.height()),
            }
        });
        let pointer = rect.as_ref().map_or(core::ptr::null(), core::ptr::from_ref);
        // SAFETY: `window` is a live window; `pointer` is null or points to a local rect.
        if !unsafe { SDL_SetTextInputArea(window.raw(), pointer, 0) } {
            koebi_core::debug!("cannot set the text input area: {}", sdl_error());
        }
    }

    /// Stops text input in `window`, hiding the on-screen keyboard.
    pub fn stop_text_input(&mut self, window: &Window) {
        // SAFETY: `window` is a live window, on the main thread.
        if !unsafe { SDL_StopTextInput(window.raw()) } {
            koebi_core::debug!("cannot stop text input: {}", sdl_error());
        }
    }

    /// The text on the clipboard, if any.
    pub fn clipboard_text(&self) -> Option<String> {
        // SAFETY: on the main thread; SDL returns a string the caller frees, possibly empty.
        let raw = unsafe { SDL_GetClipboardText() };
        if raw.is_null() {
            return None;
        }
        // SAFETY: non-null, NUL-terminated UTF-8, freed right after the copy.
        let text = unsafe { CStr::from_ptr(raw) }
            .to_string_lossy()
            .into_owned();
        // SAFETY: allocated by SDL for the caller, not used afterwards.
        unsafe { SDL_free(raw.cast()) };
        (!text.is_empty()).then_some(text)
    }

    /// Puts `text` on the clipboard.
    ///
    /// # Errors
    ///
    /// Returns the SDL error when the clipboard refuses the text.
    pub fn set_clipboard_text(&mut self, text: &str) -> Result<(), PlatformError> {
        let text = CString::new(text.replace('\0', "")).unwrap_or_default();
        // SAFETY: a valid NUL-terminated string, on the main thread.
        if unsafe { SDL_SetClipboardText(text.as_ptr()) } {
            Ok(())
        } else {
            Err(PlatformError::from_sdl("SDL_SetClipboardText"))
        }
    }

    /// Changes how often [`App::iterate`] runs.
    pub fn set_iteration_rate(&mut self, rate: IterationRate) {
        let value = rate.hint_value();
        // SAFETY: both arguments are valid NUL-terminated strings.
        if !unsafe { SDL_SetHint(SDL_HINT_MAIN_CALLBACK_RATE, value.as_ptr()) } {
            koebi_core::warn!("cannot set the main callback rate: {}", sdl_error());
        }
    }
}

/// Sentinel meaning "no exit code recorded yet".
const UNSET: i32 = i32::MIN;
static EXIT_CODE: AtomicI32 = AtomicI32::new(UNSET);

fn record_exit_code(code: i32) {
    // The first recorded code wins: it is the root cause.
    let _previous = EXIT_CODE.compare_exchange(UNSET, code, Ordering::AcqRel, Ordering::Acquire);
}

fn take_exit_code(fallback: c_int) -> i32 {
    match EXIT_CODE.swap(UNSET, Ordering::AcqRel) {
        UNSET => fallback,
        code => code,
    }
}

/// Runs the application until it quits and returns the process exit code.
///
/// Use from `fn main()` on desktop and iOS: `std::process::exit(koebi_platform::run::<MyApp>())`.
pub fn run<A: App>() -> i32 {
    let args: Vec<CString> = std::env::args_os()
        .map(|arg| CString::new(arg.to_string_lossy().into_owned()).unwrap_or_default())
        .collect();
    let mut argv: Vec<*mut c_char> = args.iter().map(|arg| arg.as_ptr().cast_mut()).collect();
    argv.push(core::ptr::null_mut());
    let argc = c_int::try_from(args.len()).unwrap_or(0);
    // SAFETY: `argv` is a NULL-terminated array of valid C strings that outlives the call, and
    // `sdl_main::<A>` matches `SDL_main_func`.
    let code = unsafe {
        SDL_RunApp(
            argc,
            argv.as_mut_ptr(),
            Some(sdl_main::<A>),
            core::ptr::null_mut(),
        )
    };
    take_exit_code(code)
}

/// Runs the application from an `SDL_main` entry point (Android, where SDL's Java glue calls
/// `SDL_main` in the app's shared library).
///
/// # Safety
///
/// `argv` must hold `argc` valid C strings, as provided by SDL.
pub unsafe fn run_sdl_main<A: App>(argc: c_int, argv: *mut *mut c_char) -> c_int {
    // SAFETY: forwarded from the caller.
    let code = unsafe { sdl_main::<A>(argc, argv) };
    take_exit_code(code)
}

unsafe extern "C" fn sdl_main<A: App>(argc: c_int, argv: *mut *mut c_char) -> c_int {
    // SAFETY: SDL provides valid arguments; the callbacks match SDL's signatures.
    unsafe {
        SDL_EnterAppMainCallbacks(
            argc,
            argv,
            Some(app_init::<A>),
            Some(app_iterate::<A>),
            Some(app_event::<A>),
            Some(app_quit::<A>),
        )
    }
}

struct AppState<A> {
    app: Option<A>,
    context: AppContext,
}

/// Runs a callback body, turning panics into an orderly failure instead of an abort.
fn guarded(body: impl FnOnce() -> SDL_AppResult) -> SDL_AppResult {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or_else(|_| {
        koebi_core::error!("panic in an application callback");
        record_exit_code(exit_code::FAILURE);
        SDL_APP_FAILURE
    })
}

fn to_sdl_result(control: AppControl) -> SDL_AppResult {
    match control {
        AppControl::Continue => SDL_APP_CONTINUE,
        AppControl::Exit(code) => {
            record_exit_code(code);
            if code == exit_code::SUCCESS {
                SDL_APP_SUCCESS
            } else {
                SDL_APP_FAILURE
            }
        }
    }
}

unsafe extern "C" fn app_init<A: App>(
    appstate: *mut *mut c_void,
    argc: c_int,
    argv: *mut *mut c_char,
) -> SDL_AppResult {
    guarded(|| {
        let args: Vec<String> = (0..usize::try_from(argc).unwrap_or(0))
            .map(|i| {
                // SAFETY: SDL passes `argc` valid NUL-terminated strings.
                let arg = unsafe { *argv.add(i) };
                if arg.is_null() {
                    String::new()
                } else {
                    // SAFETY: checked non-null; valid for the duration of the callback.
                    unsafe { CStr::from_ptr(arg) }
                        .to_string_lossy()
                        .into_owned()
                }
            })
            .collect();

        // SAFETY: valid NUL-terminated strings with static lifetime.
        let metadata_set = unsafe {
            SDL_SetAppMetadata(
                c"Koebi".as_ptr(),
                c"0.1.0".as_ptr(),
                c"io.github.kolosta.koebi".as_ptr(),
            )
        };
        if !metadata_set {
            koebi_core::debug!("SDL_SetAppMetadata failed: {}", sdl_error());
        }
        // A locked pointer moves at the speed of the system's pointer.
        // SAFETY: both arguments are valid NUL-terminated strings.
        if !unsafe { SDL_SetHint(SDL_HINT_MOUSE_RELATIVE_SYSTEM_SCALE, c"1".as_ptr()) } {
            koebi_core::debug!("cannot scale relative motion: {}", sdl_error());
        }
        // SAFETY: called once, on the main thread, before any other SDL video call.
        if !unsafe { SDL_Init(SDL_INIT_VIDEO) } {
            koebi_core::error!("cannot initialize SDL video: {}", sdl_error());
            record_exit_code(exit_code::UNSUPPORTED);
            return SDL_APP_FAILURE;
        }

        let mut context = AppContext::new();
        match A::init(&mut context, &args) {
            Ok(app) => {
                let state = Box::new(AppState {
                    app: Some(app),
                    context,
                });
                // SAFETY: SDL passes a valid out-pointer for the app state.
                unsafe { *appstate = Box::into_raw(state).cast() };
                SDL_APP_CONTINUE
            }
            Err(error) => {
                if !error.message.is_empty() {
                    koebi_core::error!("{}", error.message);
                }
                record_exit_code(error.exit_code);
                SDL_APP_FAILURE
            }
        }
    })
}

unsafe extern "C" fn app_iterate<A: App>(appstate: *mut c_void) -> SDL_AppResult {
    // SAFETY: `appstate` is the `AppState<A>` created by `app_init`; SDL calls the callbacks
    // serially on the main thread, so no other reference exists.
    let Some(state) = (unsafe { appstate.cast::<AppState<A>>().as_mut() }) else {
        return SDL_APP_FAILURE;
    };
    guarded(|| match state.app.as_mut() {
        Some(app) => to_sdl_result(app.iterate(&mut state.context)),
        None => SDL_APP_FAILURE,
    })
}

unsafe extern "C" fn app_event<A: App>(
    appstate: *mut c_void,
    event: *mut SDL_Event,
) -> SDL_AppResult {
    // SAFETY: see `app_iterate`.
    let Some(state) = (unsafe { appstate.cast::<AppState<A>>().as_mut() }) else {
        return SDL_APP_FAILURE;
    };
    // Typed text, in chunks: one event each.
    // SAFETY: SDL passes a valid event for the duration of the callback.
    if let Some((window, text)) = unsafe { event.as_ref() }.and_then(text_input) {
        let text = text.to_string_lossy();
        return guarded(|| {
            let Some(app) = state.app.as_mut() else {
                return SDL_APP_FAILURE;
            };
            for chunk in text_chunks(&text) {
                let control = app.event(
                    &mut state.context,
                    &Event::TextInput {
                        window,
                        text: chunk,
                    },
                );
                if control != AppControl::Continue {
                    return to_sdl_result(control);
                }
            }
            SDL_APP_CONTINUE
        });
    }
    // SAFETY: called on the main thread inside an SDL callback.
    let modifiers = modifiers_from_sdl(unsafe { SDL_GetModState() });
    // SAFETY: SDL passes a valid event for the duration of the callback.
    let Some(event) = (unsafe { event.as_ref() }).and_then(|event| translate(event, modifiers))
    else {
        return SDL_APP_CONTINUE;
    };
    // Pointer positions arrive in window coordinates; scenes are in logical pixels.
    let event = match event.pointer_window() {
        Some(window) => to_logical(event, logical_per_window_coordinate(window)),
        None => event,
    };
    guarded(|| match state.app.as_mut() {
        Some(app) => to_sdl_result(app.event(&mut state.context, &event)),
        None => SDL_APP_FAILURE,
    })
}

unsafe extern "C" fn app_quit<A: App>(appstate: *mut c_void, result: SDL_AppResult) {
    if result == SDL_APP_FAILURE {
        record_exit_code(exit_code::FAILURE);
    }
    if appstate.is_null() {
        return;
    }
    // SAFETY: `appstate` came from `Box::into_raw` in `app_init` and SDL calls quit once.
    let mut state = unsafe { Box::from_raw(appstate.cast::<AppState<A>>()) };
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        if let Some(app) = state.app.take() {
            app.quit(&mut state.context);
        }
    }));
    if outcome.is_err() {
        koebi_core::error!("panic while quitting");
        record_exit_code(exit_code::FAILURE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iteration_rates_map_to_sdl_hint_values() {
        assert_eq!(
            IterationRate::WaitEvent.hint_value().as_c_str(),
            c"waitevent"
        );
        assert_eq!(IterationRate::Unlimited.hint_value().as_c_str(), c"0");
        assert_eq!(IterationRate::Hz(120).hint_value().as_c_str(), c"120");
        assert_eq!(IterationRate::Hz(0).hint_value().as_c_str(), c"1");
    }

    #[test]
    fn exit_controls_map_to_sdl_results_and_record_the_first_code() {
        assert_eq!(to_sdl_result(AppControl::Continue).0, SDL_APP_CONTINUE.0);
        assert_eq!(
            to_sdl_result(AppControl::Exit(exit_code::VALIDATION_ERRORS)).0,
            SDL_APP_FAILURE.0
        );
        assert_eq!(
            to_sdl_result(AppControl::Exit(exit_code::SUCCESS)).0,
            SDL_APP_SUCCESS.0
        );
        assert_eq!(take_exit_code(99), exit_code::VALIDATION_ERRORS);
        assert_eq!(take_exit_code(99), 99);
    }

    #[test]
    fn app_error_display_includes_exit_code() {
        let error = AppError::new(exit_code::UNSUPPORTED, "no Vulkan");
        assert_eq!(error.to_string(), "no Vulkan (exit code 2)");
    }
}
