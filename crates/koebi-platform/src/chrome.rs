//! Windows drawing their own title bar, keeping every behaviour of the system's.
//!
//! The application draws the bar; the system moves, snaps, maximizes and resizes the window. A
//! hit test tells it what each point of the window is: [`WindowRegions`] says where the bar is
//! and what lies on it, the same pure function answers on every platform, and SDL turns the answer
//! into the system's behaviour:
//!
//! - **Windows**: the bar answers as a caption (`HTCAPTION`): drags snap to the edges of the
//!   screen, a drag to the top maximizes, a drag from maximized restores under the pointer, a
//!   double click maximizes or restores, a right click opens the system menu, with the system's
//!   animations.
//! - **Linux**: the same through the window manager on X11 and Wayland. Double clicks on the bar
//!   reach neither the application nor the manager, so that X11's are counted here from the
//!   press SDL reports ([`CaptionClicks`]); Wayland reports nothing.
//! - **macOS**: the window keeps its title bar and its real traffic lights, whatever the system's
//!   version, with the application's content under them (`macos.rs`).

use koebi_core::{Rect, Vec2};
use sdl3_sys::rect::SDL_Point;
use sdl3_sys::video::{SDL_HITTEST_DRAGGABLE, SDL_HITTEST_NORMAL, SDL_HitTestResult, SDL_Window};

/// What the title bar of a window with custom chrome looks like, for the system.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WindowRegions<'a> {
    /// The bar, in logical pixels: the window moves from it.
    pub drag: Option<Rect>,
    /// What takes the pointer inside the bar, in logical pixels: menus, buttons, what floats
    /// over it. The system leaves these to the application.
    pub blocked: &'a [Rect],
    /// How far from the window's edges a press resizes it, in logical pixels.
    pub resize_border: f32,
    /// The maximize button, in logical pixels, when nothing lies over it: Windows 11 shows its
    /// snap layouts over it.
    pub maximize: Option<Rect>,
}

/// What the system reports of the pointer on the maximize button, on Windows: there, it comes as
/// non-client messages, which the application's frames do not see.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MaximizeButtonState {
    /// The pointer is over it.
    pub hovered: bool,
    /// The primary button went down on it and is still down.
    pub pressed: bool,
}

/// What a window shows of the system's own title bar, for the bar drawn around it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ChromeInsets {
    /// Room the system's window buttons take at the start of the bar, in logical pixels: the
    /// traffic lights of macOS. Without them, the application draws its window buttons.
    pub leading: f32,
    /// The height the bar needs for them to lie in its middle, in logical pixels.
    pub min_height: f32,
    /// The application draws window buttons: none where the system does, or on mobile.
    pub draws_buttons: bool,
}

/// What a point of the window is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hit {
    /// The application's.
    Client,
    /// The window moves from it.
    Drag,
    /// The window resizes from it, by this edge or corner.
    Resize(Edge),
}

/// An edge or a corner of a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edge {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

impl Edge {
    fn to_sdl(self) -> SDL_HitTestResult {
        match self {
            Self::TopLeft => SDL_HitTestResult::RESIZE_TOPLEFT,
            Self::Top => SDL_HitTestResult::RESIZE_TOP,
            Self::TopRight => SDL_HitTestResult::RESIZE_TOPRIGHT,
            Self::Right => SDL_HitTestResult::RESIZE_RIGHT,
            Self::BottomRight => SDL_HitTestResult::RESIZE_BOTTOMRIGHT,
            Self::Bottom => SDL_HitTestResult::RESIZE_BOTTOM,
            Self::BottomLeft => SDL_HitTestResult::RESIZE_BOTTOMLEFT,
            Self::Left => SDL_HitTestResult::RESIZE_LEFT,
        }
    }
}

/// Over a control on the bar, the top edge resizes in this few pixels only.
const TOP_OVER_CONTROLS: f32 = 2.0;

/// Corners reach this many times farther along the edges than edges reach inwards.
const CORNER_REACH: f32 = 2.0;

/// The window's state, as far as the hit test cares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Moves {
    /// The edges resize: not when maximized, nor in fullscreen.
    pub(crate) resizes: bool,
    /// The bar moves the window: not in fullscreen.
    pub(crate) drags: bool,
}

/// The regions a window's hit test answers from, kept with the window.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct HitRegions {
    drag: Option<Rect>,
    blocked: Vec<Rect>,
    resize_border: f32,
    maximize: Option<Rect>,
}

impl HitRegions {
    /// Takes `regions`, keeping the allocation.
    pub(crate) fn set(&mut self, regions: &WindowRegions<'_>) {
        self.drag = regions.drag;
        self.blocked.clear();
        self.blocked.extend_from_slice(regions.blocked);
        self.resize_border = regions.resize_border;
        self.maximize = regions.maximize;
    }

    /// The maximize button.
    #[cfg_attr(not(windows), allow(dead_code, reason = "only Windows asks"))]
    pub(crate) const fn maximize(&self) -> Option<Rect> {
        self.maximize
    }

    /// What `point` is in a window of `size`, both in logical pixels.
    pub(crate) fn hit(&self, point: Vec2, size: Vec2, moves: Moves) -> Hit {
        let on_control = self.blocked.iter().any(|rect| rect.contains(point));
        if moves.resizes
            && let Some(edge) = self.edge(point, size, on_control)
        {
            return Hit::Resize(edge);
        }
        if moves.drags && !on_control && self.drag.is_some_and(|bar| bar.contains(point)) {
            return Hit::Drag;
        }
        Hit::Client
    }

    /// The edge or corner `point` lies on.
    fn edge(&self, point: Vec2, size: Vec2, on_control: bool) -> Option<Edge> {
        let border = self.resize_border;
        if border <= 0.0 {
            return None;
        }
        let top_border = if on_control {
            border.min(TOP_OVER_CONTROLS)
        } else {
            border
        };
        let reach = border * CORNER_REACH;
        let (left, right) = (point.x < border, point.x >= size.x - border);
        let (top, bottom) = (point.y < top_border, point.y >= size.y - border);
        let (near_left, near_right) = (point.x < reach, point.x >= size.x - reach);
        let (near_top, near_bottom) = (point.y < reach, point.y >= size.y - reach);
        Some(match () {
            () if (top && near_left) || (left && near_top) => Edge::TopLeft,
            () if (top && near_right) || (right && near_top) => Edge::TopRight,
            () if (bottom && near_left) || (left && near_bottom) => Edge::BottomLeft,
            () if (bottom && near_right) || (right && near_bottom) => Edge::BottomRight,
            () if top => Edge::Top,
            () if bottom => Edge::Bottom,
            () if left => Edge::Left,
            () if right => Edge::Right,
            () => return None,
        })
    }
}

/// What the hit test of a window reads, at a stable address given to SDL.
#[derive(Debug, Default)]
pub(crate) struct HitData {
    pub(crate) regions: HitRegions,
    /// The procedure of the window before its subclass, on Windows.
    #[cfg(windows)]
    pub(crate) previous_procedure: isize,
    /// The maximize button as Windows reports it.
    #[cfg(windows)]
    pub(crate) maximize_button: crate::win32::MaximizeButton,
    /// The `SDL_Window` the data belongs to, on Windows.
    #[cfg(windows)]
    pub(crate) sdl_window: Option<core::ptr::NonNull<SDL_Window>>,
    /// The `NSWindow` of the window, on macOS.
    #[cfg(target_os = "macos")]
    pub(crate) ns_window: Option<core::ptr::NonNull<core::ffi::c_void>>,
    /// Presses on the bar, where the system counts no double clicks.
    #[cfg(target_os = "linux")]
    pub(crate) clicks: CaptionClicks,
    /// When the window was created: the time presses are counted on.
    #[cfg(target_os = "linux")]
    pub(crate) started: LinuxClock,
}

/// A clock started with the window.
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub(crate) struct LinuxClock(std::time::Instant);

#[cfg(target_os = "linux")]
impl Default for LinuxClock {
    fn default() -> Self {
        Self(std::time::Instant::now())
    }
}

#[cfg(target_os = "linux")]
impl LinuxClock {
    pub(crate) fn elapsed(&self) -> std::time::Duration {
        self.0.elapsed()
    }
}

#[cfg(windows)]
impl HitData {
    /// What the point at `(x, y)` of the client area is, and where it lies in logical pixels;
    /// `None` where the window has no such thing as a button (in fullscreen).
    ///
    /// # Safety
    ///
    /// On the main thread, while the window lives.
    pub(crate) unsafe fn hit_at_client(&self, x: i32, y: i32) -> Option<(Hit, Vec2)> {
        let window = self.sdl_window?;
        // SAFETY: live according to the caller.
        let (size, moves, scale) = unsafe { crate::window::hit_test_state(window.as_ptr()) };
        if !moves.drags {
            return None;
        }
        let point = Vec2::new(x as f32, y as f32) * scale;
        Some((self.regions.hit(point, size, moves), point))
    }
}

/// The hit test SDL calls for a window with custom chrome.
///
/// # Safety
///
/// `window` is a live window; `area` points to a point; `data` is the `HitData` given with this
/// callback, which lives as long as the window.
pub(crate) unsafe extern "C" fn hit_test(
    window: *mut SDL_Window,
    area: *const SDL_Point,
    data: *mut core::ffi::c_void,
) -> SDL_HitTestResult {
    // A panic must not cross into SDL: the window then behaves as a plain client area.
    std::panic::catch_unwind(|| {
        // SAFETY: SDL passes a valid point for the duration of the call.
        let Some(area) = (unsafe { area.as_ref() }) else {
            return SDL_HITTEST_NORMAL;
        };
        // SAFETY: `data` is the window's `HitData`, read only here and written only between SDL
        // calls, on this thread, while the application updates the regions.
        let Some(data) = (unsafe { data.cast::<HitData>().as_ref() }) else {
            return SDL_HITTEST_NORMAL;
        };
        // SAFETY: SDL passes the live window the test is for.
        let (size, moves, scale) = unsafe { crate::window::hit_test_state(window) };
        let point = Vec2::new(area.x as f32, area.y as f32) * scale;
        match data.regions.hit(point, size, moves) {
            Hit::Client => SDL_HITTEST_NORMAL,
            Hit::Drag => {
                #[cfg(target_os = "macos")]
                crate::macos::caption_pressed(data.ns_window);
                SDL_HITTEST_DRAGGABLE
            }
            Hit::Resize(edge) => edge.to_sdl(),
        }
    })
    .unwrap_or(SDL_HITTEST_NORMAL)
}

/// Counts presses on the bar into double clicks, where the system does not (X11).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct CaptionClicks {
    last: Option<(std::time::Duration, Vec2)>,
}

impl CaptionClicks {
    /// Longest time between the presses of a double click: the usual system default.
    const TIME: std::time::Duration = std::time::Duration::from_millis(500);
    /// Farthest apart the presses of a double click lie, in window coordinates.
    const DISTANCE: f32 = 4.0;

    /// A press at `at`, at the time `now`. Returns `true` when it completes a double click.
    #[cfg_attr(
        not(target_os = "linux"),
        allow(dead_code, reason = "only X11 needs it; its tests run everywhere")
    )]
    pub(crate) fn pressed(&mut self, now: std::time::Duration, at: Vec2) -> bool {
        let double = self.last.is_some_and(|(time, place)| {
            now.saturating_sub(time) <= Self::TIME && (at - place).length() <= Self::DISTANCE
        });
        // A double click ends the run: a third press starts over.
        self.last = (!double).then_some((now, at));
        double
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::*;

    const SIZE: Vec2 = Vec2::new(800.0, 600.0);
    const NORMAL: Moves = Moves {
        resizes: true,
        drags: true,
    };
    const MAXIMIZED: Moves = Moves {
        resizes: false,
        drags: true,
    };
    const FULLSCREEN: Moves = Moves {
        resizes: false,
        drags: false,
    };

    fn regions() -> HitRegions {
        let mut regions = HitRegions::default();
        regions.set(&WindowRegions {
            drag: Some(Rect::from_origin_size(Vec2::ZERO, Vec2::new(800.0, 24.0))),
            // A menu and the window buttons.
            blocked: &[
                Rect::from_origin_size(Vec2::new(4.0, 2.0), Vec2::new(40.0, 20.0)),
                Rect::from_origin_size(Vec2::new(690.0, 0.0), Vec2::new(110.0, 24.0)),
            ],
            resize_border: 5.0,
            maximize: None,
        });
        regions
    }

    #[test]
    fn the_bar_moves_the_window_except_where_a_control_lies() {
        let regions = regions();
        assert_eq!(regions.hit(Vec2::new(300.0, 12.0), SIZE, NORMAL), Hit::Drag);
        assert_eq!(
            regions.hit(Vec2::new(20.0, 12.0), SIZE, NORMAL),
            Hit::Client
        );
        assert_eq!(
            regions.hit(Vec2::new(750.0, 12.0), SIZE, NORMAL),
            Hit::Client
        );
        // Below the bar, the application's.
        assert_eq!(
            regions.hit(Vec2::new(300.0, 100.0), SIZE, NORMAL),
            Hit::Client
        );
    }

    #[test]
    fn the_edges_resize_and_corners_reach_farther() {
        let regions = regions();
        let hit = |x, y| regions.hit(Vec2::new(x, y), SIZE, NORMAL);
        assert_eq!(hit(300.0, 2.0), Hit::Resize(Edge::Top));
        assert_eq!(hit(300.0, 598.0), Hit::Resize(Edge::Bottom));
        assert_eq!(hit(2.0, 300.0), Hit::Resize(Edge::Left));
        assert_eq!(hit(798.0, 300.0), Hit::Resize(Edge::Right));
        assert_eq!(hit(2.0, 2.0), Hit::Resize(Edge::TopLeft));
        assert_eq!(hit(9.0, 598.0), Hit::Resize(Edge::BottomLeft));
        assert_eq!(hit(798.0, 591.0), Hit::Resize(Edge::BottomRight));
        assert_eq!(hit(798.0, 1.0), Hit::Resize(Edge::TopRight));
    }

    #[test]
    fn over_a_control_the_top_edge_leaves_it_the_pointer() {
        let regions = regions();
        // In the top band, away from a control: the edge; on a button: only its first pixels.
        assert_eq!(
            regions.hit(Vec2::new(300.0, 4.0), SIZE, NORMAL),
            Hit::Resize(Edge::Top)
        );
        assert_eq!(
            regions.hit(Vec2::new(740.0, 4.0), SIZE, NORMAL),
            Hit::Client
        );
        assert_eq!(
            regions.hit(Vec2::new(740.0, 1.0), SIZE, NORMAL),
            Hit::Resize(Edge::Top)
        );
    }

    #[test]
    fn a_maximized_window_has_no_edges_and_still_moves() {
        let regions = regions();
        assert_eq!(
            regions.hit(Vec2::new(300.0, 1.0), SIZE, MAXIMIZED),
            Hit::Drag
        );
        assert_eq!(
            regions.hit(Vec2::new(1.0, 300.0), SIZE, MAXIMIZED),
            Hit::Client
        );
    }

    #[test]
    fn a_fullscreen_window_neither_moves_nor_resizes() {
        let regions = regions();
        for point in [
            Vec2::new(300.0, 12.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(798.0, 300.0),
        ] {
            assert_eq!(regions.hit(point, SIZE, FULLSCREEN), Hit::Client);
        }
    }

    #[test]
    fn no_regions_leave_everything_to_the_application() {
        let regions = HitRegions::default();
        assert_eq!(regions.hit(Vec2::new(1.0, 1.0), SIZE, NORMAL), Hit::Client);
    }

    #[test]
    fn two_quick_presses_nearby_are_a_double_click() {
        let mut clicks = CaptionClicks::default();
        let ms = Duration::from_millis;
        let at = Vec2::new(100.0, 10.0);
        assert!(!clicks.pressed(ms(0), at));
        assert!(clicks.pressed(ms(200), at + Vec2::new(2.0, 0.0)));
        // A third press starts over.
        assert!(!clicks.pressed(ms(300), at));
        // Too late, or too far.
        assert!(!clicks.pressed(ms(1000), at));
        assert!(!clicks.pressed(ms(1100), at + Vec2::new(50.0, 0.0)));
    }
}
