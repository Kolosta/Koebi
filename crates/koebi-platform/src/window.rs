//! Native windows.

use std::ffi::CString;
use std::fmt;
use std::ptr::NonNull;

use ash::vk;
use koebi_core::{Extent, Vec2};
use sdl3_sys::rect::SDL_Rect;
#[cfg(not(windows))]
use sdl3_sys::video::SDL_ShowWindowSystemMenu;
#[cfg(not(any(target_os = "macos", target_os = "android", target_os = "ios")))]
use sdl3_sys::video::SDL_WINDOW_BORDERLESS;
use sdl3_sys::video::{
    SDL_CreateWindow, SDL_DestroyWindow, SDL_GetWindowDisplayScale, SDL_GetWindowFlags,
    SDL_GetWindowFromID, SDL_GetWindowID, SDL_GetWindowPixelDensity, SDL_GetWindowPosition,
    SDL_GetWindowSize, SDL_GetWindowSizeInPixels, SDL_MaximizeWindow, SDL_MinimizeWindow,
    SDL_RaiseWindow, SDL_RestoreWindow, SDL_SetWindowFullscreen, SDL_SetWindowHitTest,
    SDL_SetWindowMinimumSize, SDL_SetWindowPosition, SDL_WINDOW_FULLSCREEN,
    SDL_WINDOW_HIGH_PIXEL_DENSITY, SDL_WINDOW_MAXIMIZED, SDL_WINDOW_MINIMIZED,
    SDL_WINDOW_RESIZABLE, SDL_WINDOW_VULKAN, SDL_Window, SDL_WindowFlags, SDL_WindowID,
};
use sdl3_sys::vulkan::SDL_Vulkan_CreateSurface;

#[cfg(target_os = "linux")]
use crate::chrome::Hit;
use crate::chrome::{self, ChromeInsets, HitData, Moves, WindowRegions};
use crate::error::PlatformError;

/// Identifies a window in [`crate::Event`]s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(pub u32);

/// Who draws the title bar of a window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowChrome {
    /// The system.
    #[default]
    System,
    /// The application, with [`Window::set_regions`] telling the system where its bar is. The
    /// window behaves as with the system's bar: see [`crate::WindowRegions`]. On macOS, the
    /// system still draws its traffic lights over it ([`Window::chrome_insets`]). Mobile
    /// platforms have no title bar.
    Custom,
}

/// What a window shows as on the screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowState {
    /// As large as it was made.
    #[default]
    Normal,
    /// In the taskbar, the dock or their equivalent.
    Minimized,
    /// Filling the room windows have on its display, the taskbar excluded.
    Maximized,
    /// Filling its display, alone on it.
    Fullscreen,
}

impl WindowState {
    fn from_flags(flags: SDL_WindowFlags) -> Self {
        if flags & SDL_WINDOW_MINIMIZED != 0 {
            Self::Minimized
        } else if flags & SDL_WINDOW_FULLSCREEN != 0 {
            Self::Fullscreen
        } else if flags & SDL_WINDOW_MAXIMIZED != 0 {
            Self::Maximized
        } else {
            Self::Normal
        }
    }
}

/// Parameters of [`crate::AppContext::create_window`].
#[derive(Clone, Copy, Debug)]
pub struct WindowDesc<'a> {
    /// Title bar text.
    pub title: &'a str,
    /// Initial width in logical units.
    pub width: u32,
    /// Initial height in logical units.
    pub height: u32,
    /// Whether the user can resize the window.
    pub resizable: bool,
    /// Who draws its title bar.
    pub chrome: WindowChrome,
    /// The smallest size the user can give it, in logical units; zero for no limit.
    pub min_size: (u32, u32),
}

impl Default for WindowDesc<'_> {
    fn default() -> Self {
        Self {
            title: "Koebi",
            width: 1280,
            height: 800,
            resizable: true,
            chrome: WindowChrome::System,
            min_size: (0, 0),
        }
    }
}

/// A native window able to host a Vulkan surface. Destroyed when dropped.
///
/// Drop every Vulkan surface created for the window before the window itself.
pub struct Window {
    raw: NonNull<SDL_Window>,
    id: WindowId,
    /// What its hit test reads, with custom chrome. SDL keeps this pointer: owned here, and freed
    /// after the window.
    hit: Option<NonNull<HitData>>,
}

impl Window {
    pub(crate) fn new(desc: &WindowDesc<'_>) -> Result<Self, PlatformError> {
        let title = CString::new(desc.title.replace('\0', " ")).unwrap_or_default();
        let width = i32::try_from(desc.width).unwrap_or(i32::MAX);
        let height = i32::try_from(desc.height).unwrap_or(i32::MAX);
        let (width, height) = fitted((width, height), usable_room());
        let mut flags = SDL_WINDOW_VULKAN | SDL_WINDOW_HIGH_PIXEL_DENSITY;
        if desc.resizable {
            flags |= SDL_WINDOW_RESIZABLE;
        }
        // The bar is the application's. macOS keeps a titled window for its traffic lights.
        #[cfg(not(any(target_os = "macos", target_os = "android", target_os = "ios")))]
        if desc.chrome == WindowChrome::Custom {
            flags |= SDL_WINDOW_BORDERLESS;
        }
        // SAFETY: SDL video is initialized (AppContext only exists after SDL_Init) and the
        // title is a valid NUL-terminated string.
        let raw = unsafe { SDL_CreateWindow(title.as_ptr(), width, height, flags) };
        let raw = NonNull::new(raw).ok_or_else(|| PlatformError::from_sdl("SDL_CreateWindow"))?;
        // SAFETY: `raw` is a live window.
        let id = WindowId(unsafe { SDL_GetWindowID(raw.as_ptr()) }.0);
        let mut window = Self { raw, id, hit: None };
        if desc.min_size != (0, 0) {
            window.set_min_size(desc.min_size);
        }
        if desc.chrome == WindowChrome::Custom
            && !cfg!(any(target_os = "android", target_os = "ios"))
        {
            window.take_chrome();
        }
        Ok(window)
    }

    /// Gives the window its smallest size, in logical units.
    fn set_min_size(&self, (width, height): (u32, u32)) {
        let scale = self.scale_factor() / self.pixel_density();
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_precision_loss,
            reason = "small sizes in pixels"
        )]
        let to_window = |value: u32| (value as f32 * scale).ceil() as i32;
        // SAFETY: live window, on the main thread.
        if !unsafe { SDL_SetWindowMinimumSize(self.raw(), to_window(width), to_window(height)) } {
            koebi_core::debug!(
                "cannot limit the window's size: {}",
                crate::error::sdl_error()
            );
        }
    }

    /// Answers the system's hit tests from here on.
    fn take_chrome(&mut self) {
        #[cfg_attr(
            not(target_os = "macos"),
            allow(unused_mut, reason = "only macOS has more to set")
        )]
        let mut data = HitData::default();
        #[cfg(target_os = "macos")]
        {
            data.ns_window = self.ns_window();
            if let Some(ns_window) = data.ns_window {
                crate::macos::extend_content(ns_window);
            }
        }
        #[cfg(windows)]
        if let Some(hwnd) = self.hwnd() {
            crate::win32::keep_shadow(hwnd);
        }
        let data = NonNull::from(Box::leak(Box::new(data)));
        // SAFETY: live window, on the main thread; `Drop` frees `data` after the window.
        if unsafe { SDL_SetWindowHitTest(self.raw(), Some(chrome::hit_test), data.as_ptr().cast()) }
        {
            self.hit = Some(data);
        } else {
            koebi_core::warn!(
                "the window cannot have custom chrome: {}",
                crate::error::sdl_error()
            );
            // SAFETY: from `Box::leak` above; SDL did not keep it.
            drop(unsafe { Box::from_raw(data.as_ptr()) });
        }
    }

    /// The `HWND` of the window.
    #[cfg(windows)]
    fn hwnd(&self) -> Option<NonNull<core::ffi::c_void>> {
        use sdl3_sys::properties::SDL_GetPointerProperty;
        use sdl3_sys::video::{SDL_GetWindowProperties, SDL_PROP_WINDOW_WIN32_HWND_POINTER};
        // SAFETY: live window, on the main thread; the property name is a valid string, and a
        // missing property gives the null default.
        NonNull::new(unsafe {
            SDL_GetPointerProperty(
                SDL_GetWindowProperties(self.raw()),
                SDL_PROP_WINDOW_WIN32_HWND_POINTER,
                core::ptr::null_mut(),
            )
        })
    }

    /// The `NSWindow` of the window.
    #[cfg(target_os = "macos")]
    fn ns_window(&self) -> Option<NonNull<core::ffi::c_void>> {
        use sdl3_sys::properties::SDL_GetPointerProperty;
        use sdl3_sys::video::{SDL_GetWindowProperties, SDL_PROP_WINDOW_COCOA_WINDOW_POINTER};
        // SAFETY: live window, on the main thread; the property name is a valid string, and a
        // missing property gives the null default.
        NonNull::new(unsafe {
            SDL_GetPointerProperty(
                SDL_GetWindowProperties(self.raw()),
                SDL_PROP_WINDOW_COCOA_WINDOW_POINTER,
                core::ptr::null_mut(),
            )
        })
    }

    /// Tells the system where the application's title bar is and what lies on it: call it after
    /// each frame that may have moved them. Does nothing without custom chrome.
    pub fn set_regions(&mut self, regions: &WindowRegions<'_>) {
        if let Some(hit) = &mut self.hit {
            // SAFETY: owned by this window. SDL reads it only during its own calls, on this
            // thread, and none happens while this reference lives.
            unsafe { hit.as_mut() }.regions.set(regions);
        }
    }

    /// What the system's own title bar takes from the application's.
    pub fn chrome_insets(&self) -> ChromeInsets {
        #[cfg(target_os = "macos")]
        if self.hit.is_some()
            && let Some(ns_window) = self.ns_window()
        {
            return crate::macos::insets(ns_window);
        }
        ChromeInsets {
            leading: 0.0,
            min_height: 0.0,
            draws_buttons: self.hit.is_some(),
        }
    }

    /// `true` when the application draws the title bar.
    pub fn has_custom_chrome(&self) -> bool {
        self.hit.is_some()
    }

    /// Sets up custom chrome again: SDL sets the style of the window again around fullscreen.
    /// Call it on [`crate::WindowEvent::FullscreenEntered`] and
    /// [`crate::WindowEvent::FullscreenLeft`].
    #[cfg_attr(
        not(target_os = "macos"),
        allow(clippy::unused_self, reason = "only macOS has anything to set again")
    )]
    pub fn refresh_chrome(&self) {
        #[cfg(target_os = "macos")]
        if self.hit.is_some()
            && let Some(ns_window) = self.ns_window()
        {
            crate::macos::extend_content(ns_window);
        }
    }

    /// The system reported a press on the window's regions (X11): a double click on the bar
    /// maximizes the window or brings it back. Call it on [`crate::WindowEvent::RegionPressed`].
    pub fn region_pressed(&mut self) {
        #[cfg(target_os = "linux")]
        {
            use sdl3_sys::mouse::SDL_GetGlobalMouseState;
            let raw = self.raw();
            let state = self.state();
            let Some(hit) = &mut self.hit else {
                return;
            };
            // SAFETY: as in `set_regions`; the calls to SDL below do not run its hit test.
            let hit = unsafe { hit.as_mut() };
            let (mut x, mut y) = (0.0, 0.0);
            let (mut left, mut top) = (0, 0);
            // SAFETY: live window, on the main thread; the out-pointers reference locals.
            unsafe {
                SDL_GetGlobalMouseState(&raw mut x, &raw mut y);
                SDL_GetWindowPosition(raw, &raw mut left, &raw mut top);
            }
            let at = Vec2::new(x - left as f32, y - top as f32);
            // SAFETY: live window, on the main thread.
            let (size, moves, scale) = unsafe { hit_test_state(raw) };
            if hit.regions.hit(at * scale, size, moves) != Hit::Drag {
                return;
            }
            if hit.clicks.pressed(hit.started.elapsed(), at) {
                // SAFETY: live window, on the main thread.
                let done = unsafe {
                    if state == WindowState::Maximized {
                        SDL_RestoreWindow(raw)
                    } else {
                        SDL_MaximizeWindow(raw)
                    }
                };
                if !done {
                    koebi_core::debug!("cannot maximize the window: {}", crate::error::sdl_error());
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        let _ = self;
    }

    /// What the window shows as.
    pub fn state(&self) -> WindowState {
        // SAFETY: live window.
        WindowState::from_flags(unsafe { SDL_GetWindowFlags(self.raw()) })
    }

    /// Asks the system to minimize the window.
    pub fn minimize(&self) {
        // SAFETY: live window, on the main thread.
        if !unsafe { SDL_MinimizeWindow(self.raw()) } {
            koebi_core::warn!("cannot minimize the window: {}", crate::error::sdl_error());
        }
    }

    /// Asks the system to maximize the window.
    pub fn maximize(&self) {
        // SAFETY: live window, on the main thread.
        if !unsafe { SDL_MaximizeWindow(self.raw()) } {
            koebi_core::warn!("cannot maximize the window: {}", crate::error::sdl_error());
        }
    }

    /// Asks the system to bring the window back from minimized or maximized.
    pub fn restore(&self) {
        // SAFETY: live window, on the main thread.
        if !unsafe { SDL_RestoreWindow(self.raw()) } {
            koebi_core::warn!("cannot restore the window: {}", crate::error::sdl_error());
        }
    }

    /// Enters or leaves fullscreen, at the display's own resolution. Leaving brings back the state
    /// the window had.
    pub fn set_fullscreen(&self, fullscreen: bool) {
        // SAFETY: live window, on the main thread.
        if !unsafe { SDL_SetWindowFullscreen(self.raw(), fullscreen) } {
            koebi_core::warn!("cannot change fullscreen: {}", crate::error::sdl_error());
        }
    }

    /// Where the window lies among the displays, and its size, in global coordinates.
    fn global_rect(&self) -> (Vec2, Vec2) {
        let (mut x, mut y, mut width, mut height) = (0, 0, 0, 0);
        // SAFETY: live window, on the main thread; the out-pointers reference locals.
        unsafe {
            SDL_GetWindowPosition(self.raw(), &raw mut x, &raw mut y);
            SDL_GetWindowSize(self.raw(), &raw mut width, &raw mut height);
        }
        (
            Vec2::new(x as f32, y as f32),
            Vec2::new(width as f32, height as f32),
        )
    }

    /// `true` when the window shows at `point`, in the global coordinates of
    /// [`crate::AppContext::global_pointer`].
    pub fn contains_global(&self, point: Vec2) -> bool {
        let (origin, size) = self.global_rect();
        let inside = point - origin;
        self.state() != WindowState::Minimized
            && inside.x >= 0.0
            && inside.y >= 0.0
            && inside.x < size.x
            && inside.y < size.y
    }

    /// `point`, in global coordinates, in the logical pixels of the window.
    pub fn global_to_logical(&self, point: Vec2) -> Vec2 {
        (point - self.global_rect().0) * logical_per_window_coordinate(self.id)
    }

    /// Moves the window so that the middle of its top lies at `point`, in global coordinates: a
    /// window opened by a drag, under the pointer.
    pub fn center_top_at(&self, point: Vec2) {
        let (_, size) = self.global_rect();
        #[allow(
            clippy::cast_possible_truncation,
            reason = "global coordinates, far inside the i32 range"
        )]
        let (x, y) = ((point.x - size.x * 0.5) as i32, point.y as i32);
        // SAFETY: live window, on the main thread.
        if !unsafe { SDL_SetWindowPosition(self.raw(), x, y) } {
            koebi_core::debug!("cannot move the window: {}", crate::error::sdl_error());
        }
    }

    /// Brings the window above the others and gives it the keyboard.
    pub fn raise(&self) {
        // SAFETY: live window, on the main thread.
        if !unsafe { SDL_RaiseWindow(self.raw()) } {
            koebi_core::debug!("cannot raise the window: {}", crate::error::sdl_error());
        }
    }

    /// Opens the system's window menu at `at`, in logical pixels, where the platform has one:
    /// not in fullscreen. It may open once the application is back in the event loop.
    pub fn show_system_menu(&self, at: Vec2) {
        if self.state() == WindowState::Fullscreen {
            return;
        }
        let at = at / logical_per_window_coordinate(self.id);
        #[allow(
            clippy::cast_possible_truncation,
            reason = "window coordinates, far inside the i32 range"
        )]
        let (x, y) = (at.x as i32, at.y as i32);
        #[cfg(windows)]
        if let Some(hwnd) = self.hwnd() {
            crate::win32::post_system_menu(hwnd, x, y);
        }
        // SAFETY: live window, on the main thread.
        #[cfg(not(windows))]
        if !unsafe { SDL_ShowWindowSystemMenu(self.raw(), x, y) } {
            koebi_core::debug!("no window menu here: {}", crate::error::sdl_error());
        }
    }

    /// Pixels per window coordinate: 2 on a Retina display, usually 1 elsewhere.
    fn pixel_density(&self) -> f32 {
        // SAFETY: live window.
        let density = unsafe { SDL_GetWindowPixelDensity(self.raw()) };
        if density > 0.0 { density } else { 1.0 }
    }

    /// The window's identifier.
    pub fn id(&self) -> WindowId {
        self.id
    }

    /// The SDL window, for the platform services of [`crate::AppContext`].
    pub(crate) fn raw(&self) -> *mut SDL_Window {
        self.raw.as_ptr()
    }

    /// Drawable size in physical pixels (the swapchain size).
    pub fn size_in_pixels(&self) -> Extent {
        let (mut width, mut height) = (0, 0);
        // SAFETY: live window; the out-pointers reference locals.
        let ok = unsafe {
            SDL_GetWindowSizeInPixels(self.raw.as_ptr(), &raw mut width, &raw mut height)
        };
        if ok {
            Extent::new(
                u32::try_from(width).unwrap_or(0),
                u32::try_from(height).unwrap_or(0),
            )
        } else {
            Extent::default()
        }
    }

    /// Physical pixels per logical pixel: display density times the user's content scale.
    pub fn scale_factor(&self) -> f32 {
        // SAFETY: live window.
        let scale = unsafe { SDL_GetWindowDisplayScale(self.raw.as_ptr()) };
        if scale > 0.0 { scale } else { 1.0 }
    }

    /// Creates a Vulkan surface for this window.
    ///
    /// # Safety
    ///
    /// `instance` must be a live Vulkan instance created with the extensions returned by
    /// [`crate::VulkanSupport::required_extensions`], from the library SDL loaded. The caller
    /// owns the surface and must destroy it before the instance and before this window.
    ///
    /// # Errors
    ///
    /// Returns the SDL error if the surface cannot be created.
    pub unsafe fn create_vulkan_surface(
        &self,
        instance: vk::Instance,
    ) -> Result<vk::SurfaceKHR, PlatformError> {
        let mut surface = vk::SurfaceKHR::null();
        // SAFETY: live window created with SDL_WINDOW_VULKAN; the caller guarantees the
        // instance requirements; the out-pointer references a local.
        let ok = unsafe {
            SDL_Vulkan_CreateSurface(
                self.raw.as_ptr(),
                instance,
                core::ptr::null(),
                &raw mut surface,
            )
        };
        if ok {
            Ok(surface)
        } else {
            Err(PlatformError::from_sdl("SDL_Vulkan_CreateSurface"))
        }
    }
}

/// How many logical pixels one SDL window coordinate is worth in the window `id`: its pixel
/// density divided by its display scale.
///
/// Window coordinates are pixels on Windows and most Linux setups (density 1) and points on
/// Apple platforms, while logical pixels are physical pixels divided by the display scale
/// ([`Window::scale_factor`]). Returns 1 for an unknown window.
pub(crate) fn logical_per_window_coordinate(id: WindowId) -> f32 {
    // SAFETY: called on the main thread from an SDL callback; SDL returns null for an
    // unknown identifier.
    let raw = unsafe { SDL_GetWindowFromID(SDL_WindowID(id.0)) };
    if raw.is_null() {
        return 1.0;
    }
    // SAFETY: `raw` is a live window, just looked up.
    let (density, display_scale) = unsafe {
        (
            SDL_GetWindowPixelDensity(raw),
            SDL_GetWindowDisplayScale(raw),
        )
    };
    coordinate_scale(density, display_scale)
}

/// The room windows have on the primary display, if SDL can tell.
fn usable_room() -> Option<(i32, i32)> {
    use sdl3_sys::video::{SDL_GetDisplayUsableBounds, SDL_GetPrimaryDisplay};
    let mut bounds = SDL_Rect::default();
    // SAFETY: SDL video is initialized (only `AppContext` creates windows); the out-pointer
    // references a local.
    let known = unsafe { SDL_GetDisplayUsableBounds(SDL_GetPrimaryDisplay(), &raw mut bounds) };
    known.then_some((bounds.w, bounds.h))
}

/// `size`, no larger than `room`: a window opens with all of it on its display, and comes back at
/// that size from maximized.
fn fitted(size: (i32, i32), room: Option<(i32, i32)>) -> (i32, i32) {
    match room {
        Some((width, height)) if width > 0 && height > 0 => (size.0.min(width), size.1.min(height)),
        _ => size,
    }
}

/// What a hit test needs of `window`: its size in logical pixels, what the state lets the user
/// do, and the logical pixels per window coordinate.
///
/// # Safety
///
/// `window` is a live window, on the main thread.
pub(crate) unsafe fn hit_test_state(window: *mut SDL_Window) -> (Vec2, Moves, f32) {
    let (mut width, mut height) = (0, 0);
    // SAFETY: live window according to the caller; the out-pointers reference locals.
    let (flags, density, display_scale) = unsafe {
        SDL_GetWindowSize(window, &raw mut width, &raw mut height);
        (
            SDL_GetWindowFlags(window),
            SDL_GetWindowPixelDensity(window),
            SDL_GetWindowDisplayScale(window),
        )
    };
    let scale = coordinate_scale(density, display_scale);
    let state = WindowState::from_flags(flags);
    let moves = Moves {
        resizes: state == WindowState::Normal,
        drags: state != WindowState::Fullscreen,
    };
    (Vec2::new(width as f32, height as f32) * scale, moves, scale)
}

/// `density / display_scale`, or 1 when SDL could not tell (it returns 0 on failure).
fn coordinate_scale(density: f32, display_scale: f32) -> f32 {
    let scale = density / display_scale;
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

impl fmt::Debug for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Window")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        // SAFETY: we own the window and destroy it exactly once. Its hit test goes with it,
        // before the data it reads, which came from `Box::leak` in `take_chrome`.
        unsafe {
            SDL_DestroyWindow(self.raw.as_ptr());
            if let Some(hit) = self.hit {
                drop(Box::from_raw(hit.as_ptr()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_coordinates_scale_by_density_over_display_scale() {
        // Windows at 125 %: pixels are window coordinates, 1.25 of them per logical pixel.
        assert!((coordinate_scale(1.0, 1.25) - 0.8).abs() < f32::EPSILON);
        // macOS Retina: points are logical pixels.
        assert!((coordinate_scale(2.0, 2.0) - 1.0).abs() < f32::EPSILON);
        // SDL failures (0) fall back to no conversion.
        assert!((coordinate_scale(0.0, 1.25) - 1.0).abs() < f32::EPSILON);
        assert!((coordinate_scale(1.0, 0.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn windows_open_no_larger_than_the_room_of_their_display() {
        assert_eq!(fitted((1280, 800), Some((1920, 1040))), (1280, 800));
        assert_eq!(fitted((1280, 800), Some((1024, 728))), (1024, 728));
        assert_eq!(fitted((1280, 800), Some((1024, 1000))), (1024, 800));
        // SDL could not tell.
        assert_eq!(fitted((1280, 800), None), (1280, 800));
        assert_eq!(fitted((1280, 800), Some((0, 0))), (1280, 800));
    }

    #[test]
    fn default_window_is_resizable_and_titled() {
        let desc = WindowDesc::default();
        assert!(desc.resizable);
        assert_eq!(desc.title, "Koebi");
        assert!(desc.width > 0 && desc.height > 0);
        assert_eq!(desc.chrome, WindowChrome::System);
    }

    #[test]
    fn window_states_read_from_flags_fullscreen_over_maximized() {
        let state = WindowState::from_flags;
        assert_eq!(state(SDL_WindowFlags(0)), WindowState::Normal);
        assert_eq!(state(SDL_WINDOW_MAXIMIZED), WindowState::Maximized);
        assert_eq!(state(SDL_WINDOW_FULLSCREEN), WindowState::Fullscreen);
        assert_eq!(
            state(SDL_WINDOW_FULLSCREEN | SDL_WINDOW_MAXIMIZED),
            WindowState::Fullscreen
        );
        assert_eq!(
            state(SDL_WINDOW_MINIMIZED | SDL_WINDOW_MAXIMIZED),
            WindowState::Minimized
        );
    }
}
