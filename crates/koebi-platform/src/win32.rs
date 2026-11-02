//! Windows: what custom chrome needs beyond SDL.

use core::ffi::c_void;
use core::ptr::NonNull;

#[repr(C)]
struct Margins {
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
}

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn ClientToScreen(window: *mut c_void, point: *mut Point) -> i32;
    fn PostMessageW(window: *mut c_void, message: u32, wparam: usize, lparam: isize) -> i32;
}

/// The undocumented message that opens a window's system menu, which SDL sends too.
const WM_POPUPSYSTEMMENU: u32 = 0x0313;

/// Opens the system menu of `hwnd` at `(x, y)` in its client area, once the application has
/// returned to the message loop. The menu runs a modal loop, in which SDL calls the application's
/// iteration from a timer: from inside an iteration, that would run one within the other.
pub(crate) fn post_system_menu(hwnd: NonNull<c_void>, x: i32, y: i32) {
    let mut point = Point { x, y };
    // SAFETY: `hwnd` is a live window of this thread; the out-pointer references a local; the
    // message carries no pointer.
    let posted = unsafe {
        ClientToScreen(hwnd.as_ptr(), &raw mut point);
        let lparam = ((point.y & 0xFFFF) << 16) | (point.x & 0xFFFF);
        PostMessageW(hwnd.as_ptr(), WM_POPUPSYSTEMMENU, 0, lparam as isize)
    };
    if posted == 0 {
        koebi_core::debug!("cannot open the window's system menu");
    }
}

#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmExtendFrameIntoClientArea(window: *mut c_void, margins: *const Margins) -> i32;
}

/// Gives the borderless `hwnd` the shadow of other windows. The compositor draws it around a
/// window with a frame, however thin: one pixel of frame in the client area is enough, which the
/// application paints over.
pub(crate) fn keep_shadow(hwnd: NonNull<c_void>) {
    let margins = Margins {
        left: 0,
        right: 0,
        top: 0,
        bottom: 1,
    };
    // SAFETY: `hwnd` is a live window of this process; `margins` outlives the call.
    let result = unsafe { DwmExtendFrameIntoClientArea(hwnd.as_ptr(), &raw const margins) };
    if result < 0 {
        koebi_core::debug!("no shadow around the window: error {result:#x}");
    }
}
