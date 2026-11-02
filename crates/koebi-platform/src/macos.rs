//! macOS: the real traffic lights over the application's own title bar.
//!
//! The window keeps its system title bar, made transparent and without a title, with the content
//! under it: the system draws the window buttons as it draws them in this version of macOS, and
//! they keep all their behaviours (fullscreen in its own space, zoom with Option, the Window
//! menu). A few calls to AppKit do it, through the Objective-C runtime that every macOS process
//! already holds.

use core::ffi::{CStr, c_char, c_long, c_void};
use core::ptr::NonNull;

use crate::chrome::ChromeInsets;

#[link(name = "objc")]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
}

/// `NSWindowStyleMaskFullSizeContentView`.
const FULL_SIZE_CONTENT: u64 = 1 << 15;
/// `NSWindowTitleHidden`.
const TITLE_HIDDEN: c_long = 1;
/// `NSWindowZoomButton`.
const ZOOM_BUTTON: c_long = 2;
/// `NSEventTypeLeftMouseDown`.
const LEFT_MOUSE_DOWN: u64 = 1;
/// The height of the system's title bar, in points, in which the traffic lights lie centred.
const TITLE_BAR_HEIGHT: f32 = 28.0;
/// The room of the traffic lights when the system cannot tell.
const BUTTONS_WIDTH: f32 = 78.0;
/// The space after the traffic lights.
const BUTTONS_MARGIN: f32 = 8.0;

/// `NSRect`.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct NsRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

/// The selector named `name`.
fn selector(name: &CStr) -> *mut c_void {
    // SAFETY: `name` is a valid NUL-terminated string; selectors are registered for the process.
    unsafe { sel_registerName(name.as_ptr()) }
}

/// Sends a message without arguments, returning `R`.
///
/// # Safety
///
/// `object` is a live object or nil, answering `name` with a result of type `R` in registers.
unsafe fn send0<R>(object: *mut c_void, name: &CStr) -> R {
    // SAFETY: `objc_msgSend` must be called through a pointer of the method's exact signature,
    // as Apple documents; the caller guarantees it.
    let call: unsafe extern "C" fn(*mut c_void, *mut c_void) -> R =
        unsafe { core::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
    // SAFETY: see above.
    unsafe { call(object, selector(name)) }
}

/// Sends a message with one argument, returning `R`.
///
/// # Safety
///
/// `object` is a live object or nil, answering `name` with `A` and a result of type `R` in
/// registers.
unsafe fn send1<A, R>(object: *mut c_void, name: &CStr, argument: A) -> R {
    // SAFETY: as in `send0`.
    let call: unsafe extern "C" fn(*mut c_void, *mut c_void, A) -> R =
        unsafe { core::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
    // SAFETY: as in `send0`.
    unsafe { call(object, selector(name), argument) }
}

/// Puts the content under the title bar of `ns_window`, keeping its traffic lights, and the
/// window's size. SDL sets the style again around fullscreen: call it after these changes too.
pub(crate) fn extend_content(ns_window: NonNull<c_void>) {
    let window = ns_window.as_ptr();
    // SAFETY: `ns_window` is the live `NSWindow` of an SDL window, on the main thread; these
    // methods exist since macOS 10.10, which SDL 3 requires, with these signatures.
    unsafe {
        let style: u64 = send0(window, c"styleMask");
        let () = send1(window, c"setTitlebarAppearsTransparent:", true);
        let () = send1(window, c"setTitleVisibility:", TITLE_HIDDEN);
        if style & FULL_SIZE_CONTENT != 0 {
            return;
        }
        // Changing the style upsets the responder chain, through which SDL receives the pointer:
        // SDL takes the same care.
        let content: *mut c_void = send0(window, c"contentView");
        let next: *mut c_void = send0(content, c"nextResponder");
        let height = frame(content).map(|rect| rect.height);
        let () = send1(
            content,
            c"setNextResponder:",
            core::ptr::null_mut::<c_void>(),
        );
        let () = send1(window, c"setStyleMask:", style | FULL_SIZE_CONTENT);
        let () = send1(content, c"setNextResponder:", next);
        // The content now fills the frame: the frame comes down to the content's height, top
        // edge in place. SDL learns the window's size from this change, which its hit tests use.
        if let (Some(height), Some(mut rect)) = (height, frame(window)) {
            rect.y += rect.height - height;
            rect.height = height;
            let call: unsafe extern "C" fn(*mut c_void, *mut c_void, NsRect, bool) =
                core::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            call(window, selector(c"setFrame:display:"), rect, true);
        }
    }
}

/// The frame of a view or a window, or nothing on nil.
fn frame(view: *mut c_void) -> Option<NsRect> {
    if view.is_null() {
        return None;
    }
    // SAFETY: `view` is a live `NSView` or `NSWindow`; `-frame` returns an `NSRect`, in registers
    // on arm64 and through a hidden pointer on x86_64.
    #[cfg(target_arch = "aarch64")]
    let rect = unsafe { send0::<NsRect>(view, c"frame") };
    #[cfg(not(target_arch = "aarch64"))]
    let rect = {
        #[link(name = "objc")]
        unsafe extern "C" {
            fn objc_msgSend_stret();
        }
        // SAFETY: as above, the x86_64 convention for large results.
        unsafe {
            let call: unsafe extern "C" fn(*mut c_void, *mut c_void) -> NsRect =
                core::mem::transmute(objc_msgSend_stret as unsafe extern "C" fn());
            call(view, selector(c"frame"))
        }
    };
    Some(rect)
}

/// What the traffic lights of `ns_window` take from the bar.
pub(crate) fn insets(ns_window: NonNull<c_void>) -> ChromeInsets {
    // SAFETY: `ns_window` is a live `NSWindow`, on the main thread; the zoom button is the last
    // of the traffic lights, or nil if the window has none.
    let zoom: *mut c_void =
        unsafe { send1(ns_window.as_ptr(), c"standardWindowButton:", ZOOM_BUTTON) };
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a few dozen points, which f32 holds"
    )]
    let end = frame(zoom).map_or(BUTTONS_WIDTH, |rect| {
        (rect.x + rect.width) as f32 + BUTTONS_MARGIN
    });
    ChromeInsets {
        leading: end,
        min_height: TITLE_BAR_HEIGHT,
        draws_buttons: false,
    }
}

/// A press reached the drag region of `ns_window`: a second one in a row does what the user set
/// in System Settings for double clicks on a title bar. SDL handles the press itself and tells
/// the application nothing.
pub(crate) fn caption_pressed(ns_window: Option<NonNull<c_void>>) {
    let Some(ns_window) = ns_window else {
        return;
    };
    // SAFETY: on the main thread, in SDL's handling of an event; `NSApp` and `currentEvent` exist
    // on every version SDL 3 runs on, and answer nil when there is none.
    unsafe {
        let class = objc_getClass(c"NSApplication".as_ptr());
        let application: *mut c_void = send0(class, c"sharedApplication");
        let event: *mut c_void = send0(application, c"currentEvent");
        if event.is_null() {
            return;
        }
        let kind: u64 = send0(event, c"type");
        if kind != LEFT_MOUSE_DOWN {
            return;
        }
        let clicks: c_long = send0(event, c"clickCount");
        if clicks != 2 {
            return;
        }
        match double_click_action() {
            DoubleClick::Zoom => {
                let () = send1(
                    ns_window.as_ptr(),
                    c"zoom:",
                    core::ptr::null_mut::<c_void>(),
                );
            }
            DoubleClick::Minimize => {
                let () = send1(
                    ns_window.as_ptr(),
                    c"miniaturize:",
                    core::ptr::null_mut::<c_void>(),
                );
            }
            DoubleClick::Nothing => {}
        }
    }
}

/// What the user asked of double clicks on title bars.
enum DoubleClick {
    Zoom,
    Minimize,
    Nothing,
}

/// Reads `AppleActionOnDoubleClick` from the user's defaults: `Maximize`, the default, `Minimize`
/// or `None`.
fn double_click_action() -> DoubleClick {
    // SAFETY: on the main thread; `NSUserDefaults` and `NSString` exist on every version SDL 3
    // runs on; `stringForKey:` answers an `NSString` or nil, and `UTF8String` a string alive for
    // the duration of this autorelease scope, read at once.
    unsafe {
        let defaults_class = objc_getClass(c"NSUserDefaults".as_ptr());
        let defaults: *mut c_void = send0(defaults_class, c"standardUserDefaults");
        let string_class = objc_getClass(c"NSString".as_ptr());
        let key: *mut c_void = send1(
            string_class,
            c"stringWithUTF8String:",
            c"AppleActionOnDoubleClick".as_ptr(),
        );
        let value: *mut c_void = send1(defaults, c"stringForKey:", key);
        if value.is_null() {
            return DoubleClick::Zoom;
        }
        let text: *const c_char = send0(value, c"UTF8String");
        if text.is_null() {
            return DoubleClick::Zoom;
        }
        match CStr::from_ptr(text).to_bytes() {
            b"Minimize" => DoubleClick::Minimize,
            b"None" => DoubleClick::Nothing,
            _ => DoubleClick::Zoom,
        }
    }
}
