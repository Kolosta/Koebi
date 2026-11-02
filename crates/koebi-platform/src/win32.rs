//! Windows: what custom chrome needs beyond SDL.
//!
//! - **The shadow**: the compositor draws it around a window with a frame, however thin.
//! - **The maximize button**: Windows 11 shows its snap layouts over a button that answers the hit
//!   test as `HTMAXBUTTON`, which SDL's hit test cannot answer. The window procedure is subclassed
//!   to answer it. The system then reports the pointer over the button as non-client messages, so
//!   that the application learns of its hover and press from [`MaximizeButton`], and the release
//!   toggles the window. Windows 10 goes the same way and shows no layouts.

use core::ffi::c_void;
use core::ptr::NonNull;

use koebi_core::{Rect, Vec2};

use crate::chrome::{Hit, HitData};

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

#[repr(C)]
struct TrackMouse {
    size: u32,
    flags: u32,
    window: *mut c_void,
    hover_time: u32,
}

type Procedure = unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize;

#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmExtendFrameIntoClientArea(window: *mut c_void, margins: *const Margins) -> i32;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn SetWindowLongPtrW(window: *mut c_void, index: i32, value: isize) -> isize;
    fn CallWindowProcW(
        previous: isize,
        window: *mut c_void,
        message: u32,
        wparam: usize,
        lparam: isize,
    ) -> isize;
    fn SetPropW(window: *mut c_void, name: *const u16, data: *mut c_void) -> i32;
    fn GetPropW(window: *mut c_void, name: *const u16) -> *mut c_void;
    fn RemovePropW(window: *mut c_void, name: *const u16) -> *mut c_void;
    fn ScreenToClient(window: *mut c_void, point: *mut Point) -> i32;
    fn TrackMouseEvent(track: *mut TrackMouse) -> i32;
    fn InvalidateRect(window: *mut c_void, rect: *const c_void, erase: i32) -> i32;
    fn IsZoomed(window: *mut c_void) -> i32;
    fn PostMessageW(window: *mut c_void, message: u32, wparam: usize, lparam: isize) -> i32;
    fn ClientToScreen(window: *mut c_void, point: *mut Point) -> i32;
}

const GWLP_WNDPROC: i32 = -4;
const WM_NCHITTEST: u32 = 0x0084;
const WM_NCMOUSEMOVE: u32 = 0x00A0;
const WM_NCLBUTTONDOWN: u32 = 0x00A1;
const WM_NCLBUTTONUP: u32 = 0x00A2;
const WM_NCLBUTTONDBLCLK: u32 = 0x00A3;
const WM_NCMOUSELEAVE: u32 = 0x02A2;
const WM_SYSCOMMAND: u32 = 0x0112;
const SC_MAXIMIZE: usize = 0xF030;
const SC_RESTORE: usize = 0xF120;
const HTMAXBUTTON: isize = 9;
/// The size Windows reads `TrackMouse` by.
const TRACK_MOUSE_SIZE: u32 = 24;
const _: () = assert!(size_of::<TrackMouse>() == TRACK_MOUSE_SIZE as usize);
const TME_LEAVE: u32 = 0x0002;
const TME_NONCLIENT: u32 = 0x0010;

/// The name of the property holding a window's chrome data, as UTF-16.
const PROPERTY: [u16; 13] = {
    let text = *b"KoebiChrome\0\0";
    let mut wide = [0; 13];
    let mut index = 0;
    while index < text.len() {
        wide[index] = text[index] as u16;
        index += 1;
    }
    wide
};

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

/// What the pointer does on the maximize button, as the system reports it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MaximizeButton {
    /// The pointer is over it.
    pub(crate) hovered: bool,
    /// The primary button went down on it and is still down.
    pub(crate) pressed: bool,
}

/// What a message about the button asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reaction {
    /// Pass it on: it is not about the button.
    PassOn,
    /// The state changed: show it.
    Changed,
    /// Maximize the window or bring it back.
    Toggle,
}

impl MaximizeButton {
    /// Follows a non-client message whose hit test value is `area`.
    fn on_message(&mut self, message: u32, area: isize) -> Reaction {
        let on_button = area == HTMAXBUTTON;
        match message {
            WM_NCMOUSEMOVE if on_button => {
                self.hovered = true;
                Reaction::Changed
            }
            WM_NCLBUTTONDOWN | WM_NCLBUTTONDBLCLK if on_button => {
                self.pressed = true;
                Reaction::Changed
            }
            WM_NCLBUTTONUP if on_button => {
                let toggles = self.pressed;
                self.pressed = false;
                if toggles {
                    Reaction::Toggle
                } else {
                    Reaction::Changed
                }
            }
            // Elsewhere on the bar, or off it: the press leaving the button does not act.
            WM_NCMOUSEMOVE | WM_NCMOUSELEAVE if self.hovered || self.pressed => {
                self.hovered = false;
                self.pressed = false;
                Reaction::PassOn
            }
            _ => Reaction::PassOn,
        }
    }
}

/// The value of `HTMAXBUTTON` for `hit`: only where the regions say the button lies, and the
/// edges of the window keep the pointer first.
fn is_on_button(hit: Hit, button: Option<Rect>, point: Vec2) -> bool {
    hit == Hit::Client && button.is_some_and(|rect| rect.contains(point))
}

/// The window procedure of a window with custom chrome: SDL's, with the maximize button.
///
/// # Safety
///
/// Called by Windows for a window subclassed by [`subclass`], which holds the property until
/// [`unsubclass`] restores the previous procedure.
unsafe extern "system" fn procedure(
    window: *mut c_void,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    // SAFETY: the property holds the window's `HitData`, alive until `unsubclass`. It is only
    // dereferenced for moments without a call that could come back here: SDL's hit test reads it
    // during `forward`.
    let data = unsafe { GetPropW(window, PROPERTY.as_ptr()) }.cast::<HitData>();
    if data.is_null() {
        return 0;
    }
    // SAFETY: as above.
    let previous = unsafe { (*data).previous_procedure };
    let forward = || {
        // SAFETY: `previous` is the procedure this one replaced, called with what it was given.
        unsafe { CallWindowProcW(previous, window, message, wparam, lparam) }
    };
    if message == WM_NCHITTEST {
        let result = forward();
        #[allow(
            clippy::cast_possible_truncation,
            reason = "the low and high words of the message's parameter are signed coordinates"
        )]
        let mut point = Point {
            x: i32::from(lparam as i16),
            y: i32::from((lparam >> 16) as i16),
        };
        // SAFETY: `window` is live; the out-pointer references a local; `data` as above, on the
        // main thread where the window lives.
        unsafe {
            ScreenToClient(window, &raw mut point);
            let data = &*data;
            if let Some((hit, at)) = data.hit_at_client(point.x, point.y)
                && is_on_button(hit, data.regions.maximize(), at)
            {
                return HTMAXBUTTON;
            }
        }
        return result;
    }
    if matches!(
        message,
        WM_NCMOUSEMOVE | WM_NCMOUSELEAVE | WM_NCLBUTTONDOWN | WM_NCLBUTTONUP | WM_NCLBUTTONDBLCLK
    ) {
        #[allow(
            clippy::cast_possible_wrap,
            reason = "hit test values are small numbers"
        )]
        let area = wparam as isize;
        // SAFETY: `data` as above; nothing calls back during this borrow.
        let (changed, reaction) = unsafe {
            let button = &mut (*data).maximize_button;
            let before = *button;
            let reaction = button.on_message(message, area);
            (*button != before, reaction)
        };
        if changed {
            // SAFETY: `window` is live; a null rectangle asks for the whole window to be drawn.
            unsafe { InvalidateRect(window, core::ptr::null(), 0) };
        }
        if message == WM_NCMOUSEMOVE && area == HTMAXBUTTON {
            let mut track = TrackMouse {
                size: TRACK_MOUSE_SIZE,
                flags: TME_LEAVE | TME_NONCLIENT,
                window,
                hover_time: 0,
            };
            // SAFETY: `track` is valid and has its size set.
            unsafe { TrackMouseEvent(&raw mut track) };
        }
        match reaction {
            Reaction::PassOn => {}
            // Windows would draw and track a button of its own: it is the application's.
            Reaction::Changed => return 0,
            Reaction::Toggle => {
                // SAFETY: `window` is live; the message carries no pointer.
                unsafe {
                    let command = if IsZoomed(window) == 0 {
                        SC_MAXIMIZE
                    } else {
                        SC_RESTORE
                    };
                    PostMessageW(window, WM_SYSCOMMAND, command, 0);
                }
                return 0;
            }
        }
    }
    forward()
}

/// Subclasses `hwnd`, whose chrome data is `data`, boxed and alive until [`unsubclass`].
pub(crate) fn subclass(hwnd: NonNull<c_void>, data: &mut HitData) {
    let window = hwnd.as_ptr();
    let pointer = core::ptr::from_mut(data).cast();
    // SAFETY: `window` is a live window of this thread; `procedure` matches `WNDPROC` and keeps
    // calling the previous procedure, stored before it can be called; `data` outlives the
    // subclass, which `unsubclass` removes before the box drops.
    unsafe {
        if SetPropW(window, PROPERTY.as_ptr(), pointer) == 0 {
            return;
        }
        let new: Procedure = procedure;
        #[allow(
            clippy::cast_possible_wrap,
            reason = "an address, which Windows takes as a pointer-sized integer"
        )]
        let address = new as usize as isize;
        data.previous_procedure = SetWindowLongPtrW(window, GWLP_WNDPROC, address);
    }
}

/// Removes the subclass of [`subclass`] from `hwnd`.
pub(crate) fn unsubclass(hwnd: NonNull<c_void>, data: &HitData) {
    let window = hwnd.as_ptr();
    // SAFETY: `window` is a live window of this thread, whose procedure is `procedure`; the
    // previous one is still valid: SDL's window class outlives the window.
    unsafe {
        SetWindowLongPtrW(window, GWLP_WNDPROC, data.previous_procedure);
        RemovePropW(window, PROPERTY.as_ptr());
    }
}

/// Gives the borderless `hwnd` the shadow of other windows: one pixel of frame in the client area
/// is enough, which the application paints over.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_property_name_is_terminated_text() {
        assert_eq!(PROPERTY[0], u16::from(b'K'));
        assert_eq!(PROPERTY[10], u16::from(b'e'));
        assert_eq!(PROPERTY[11], 0);
    }

    #[test]
    fn a_click_on_the_button_toggles_on_its_release() {
        let mut button = MaximizeButton::default();
        assert_eq!(
            button.on_message(WM_NCMOUSEMOVE, HTMAXBUTTON),
            Reaction::Changed
        );
        assert!(button.hovered && !button.pressed);
        assert_eq!(
            button.on_message(WM_NCLBUTTONDOWN, HTMAXBUTTON),
            Reaction::Changed
        );
        assert!(button.pressed);
        assert_eq!(
            button.on_message(WM_NCLBUTTONUP, HTMAXBUTTON),
            Reaction::Toggle
        );
        assert!(button.hovered && !button.pressed);
    }

    #[test]
    fn a_press_leaving_the_button_does_not_toggle() {
        let mut button = MaximizeButton::default();
        button.on_message(WM_NCMOUSEMOVE, HTMAXBUTTON);
        button.on_message(WM_NCLBUTTONDOWN, HTMAXBUTTON);
        // Over the caption beside it: the hit test value is no longer the button's.
        assert_eq!(button.on_message(WM_NCMOUSEMOVE, 2), Reaction::PassOn);
        assert_eq!(button, MaximizeButton::default());
        // Coming back and releasing: it was not pressed there.
        button.on_message(WM_NCMOUSEMOVE, HTMAXBUTTON);
        assert_eq!(
            button.on_message(WM_NCLBUTTONUP, HTMAXBUTTON),
            Reaction::Changed
        );
    }

    #[test]
    fn leaving_clears_the_button_and_a_double_click_presses_it() {
        let mut button = MaximizeButton::default();
        button.on_message(WM_NCMOUSEMOVE, HTMAXBUTTON);
        assert_eq!(button.on_message(WM_NCMOUSELEAVE, 0), Reaction::PassOn);
        assert!(!button.hovered);
        assert_eq!(
            button.on_message(WM_NCLBUTTONDBLCLK, HTMAXBUTTON),
            Reaction::Changed
        );
        assert!(button.pressed);
        // Messages about other parts are not its business.
        let mut idle = MaximizeButton::default();
        assert_eq!(idle.on_message(WM_NCLBUTTONDOWN, 2), Reaction::PassOn);
    }

    #[test]
    fn the_button_answers_only_where_the_application_has_it() {
        let button = Rect::from_origin_size(Vec2::new(100.0, 0.0), Vec2::new(36.0, 24.0));
        let inside = Vec2::new(110.0, 10.0);
        assert!(is_on_button(Hit::Client, Some(button), inside));
        assert!(!is_on_button(
            Hit::Client,
            Some(button),
            Vec2::new(50.0, 10.0)
        ));
        assert!(!is_on_button(Hit::Client, None, inside));
        // The system's edges come first, and a fullscreen window has no such part.
        assert!(!is_on_button(Hit::Drag, Some(button), inside));
    }
}
