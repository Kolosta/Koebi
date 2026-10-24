//! Native windows.

use std::ffi::CString;
use std::fmt;
use std::ptr::NonNull;

use ash::vk;
use koebi_core::Extent;
use sdl3_sys::video::{
    SDL_CreateWindow, SDL_DestroyWindow, SDL_GetWindowDisplayScale, SDL_GetWindowID,
    SDL_GetWindowSizeInPixels, SDL_WINDOW_HIGH_PIXEL_DENSITY, SDL_WINDOW_RESIZABLE,
    SDL_WINDOW_VULKAN, SDL_Window,
};
use sdl3_sys::vulkan::SDL_Vulkan_CreateSurface;

use crate::error::PlatformError;

/// Identifies a window in [`crate::Event`]s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(pub u32);

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
}

impl Default for WindowDesc<'_> {
    fn default() -> Self {
        Self {
            title: "Koebi",
            width: 1280,
            height: 800,
            resizable: true,
        }
    }
}

/// A native window able to host a Vulkan surface. Destroyed when dropped.
///
/// Drop every Vulkan surface created for the window before the window itself.
pub struct Window {
    raw: NonNull<SDL_Window>,
    id: WindowId,
}

impl Window {
    pub(crate) fn new(desc: &WindowDesc<'_>) -> Result<Self, PlatformError> {
        let title = CString::new(desc.title.replace('\0', " ")).unwrap_or_default();
        let width = i32::try_from(desc.width).unwrap_or(i32::MAX);
        let height = i32::try_from(desc.height).unwrap_or(i32::MAX);
        let mut flags = SDL_WINDOW_VULKAN | SDL_WINDOW_HIGH_PIXEL_DENSITY;
        if desc.resizable {
            flags |= SDL_WINDOW_RESIZABLE;
        }
        // SAFETY: SDL video is initialized (AppContext only exists after SDL_Init) and the
        // title is a valid NUL-terminated string.
        let raw = unsafe { SDL_CreateWindow(title.as_ptr(), width, height, flags) };
        let raw = NonNull::new(raw).ok_or_else(|| PlatformError::from_sdl("SDL_CreateWindow"))?;
        // SAFETY: `raw` is a live window.
        let id = WindowId(unsafe { SDL_GetWindowID(raw.as_ptr()) }.0);
        Ok(Self { raw, id })
    }

    /// The window's identifier.
    pub fn id(&self) -> WindowId {
        self.id
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

impl fmt::Debug for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Window")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        // SAFETY: we own the window and destroy it exactly once.
        unsafe { SDL_DestroyWindow(self.raw.as_ptr()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_window_is_resizable_and_titled() {
        let desc = WindowDesc::default();
        assert!(desc.resizable);
        assert_eq!(desc.title, "Koebi");
        assert!(desc.width > 0 && desc.height > 0);
    }
}
