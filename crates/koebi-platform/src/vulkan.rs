//! Vulkan loading through SDL.

use std::ffi::{CStr, CString};
use std::fmt;

use ash::vk;
use sdl3_sys::vulkan::{
    SDL_Vulkan_GetInstanceExtensions, SDL_Vulkan_GetVkGetInstanceProcAddr, SDL_Vulkan_LoadLibrary,
};

use crate::error::PlatformError;

/// The Vulkan loader as opened by SDL, and the instance extensions windows need.
///
/// Creating the instance from SDL's loader guarantees SDL and Koebi share the same driver
/// functions, which surface creation requires.
pub struct VulkanSupport {
    get_instance_proc_addr: vk::PFN_vkGetInstanceProcAddr,
    extensions: Vec<CString>,
}

impl VulkanSupport {
    /// Loads the Vulkan library (`library = None` lets SDL search) and queries extensions.
    pub(crate) fn load(library: Option<&str>) -> Result<Self, PlatformError> {
        let path = library
            .map(CString::new)
            .transpose()
            .map_err(|_| PlatformError::new("SDL_Vulkan_LoadLibrary", "path contains NUL"))?;
        let path_ptr = path.as_ref().map_or(core::ptr::null(), |p| p.as_ptr());
        // SAFETY: SDL video is initialized; the path is null or a valid C string.
        if !unsafe { SDL_Vulkan_LoadLibrary(path_ptr) } {
            return Err(PlatformError::from_sdl("SDL_Vulkan_LoadLibrary"));
        }
        // SAFETY: the library was loaded successfully above.
        let get_instance_proc_addr = unsafe { SDL_Vulkan_GetVkGetInstanceProcAddr() }
            .ok_or_else(|| PlatformError::from_sdl("SDL_Vulkan_GetVkGetInstanceProcAddr"))?;

        let mut count = 0_u32;
        // SAFETY: the library is loaded; the out-pointer references a local.
        let names = unsafe { SDL_Vulkan_GetInstanceExtensions(&raw mut count) };
        if names.is_null() {
            return Err(PlatformError::from_sdl("SDL_Vulkan_GetInstanceExtensions"));
        }
        let extensions = (0..count as usize)
            .map(|i| {
                // SAFETY: SDL returns `count` valid NUL-terminated strings it keeps alive while
                // the library is loaded; they are copied immediately.
                unsafe { CStr::from_ptr(*names.add(i)) }.to_owned()
            })
            .collect();
        Ok(Self {
            get_instance_proc_addr,
            extensions,
        })
    }

    /// The loader's `vkGetInstanceProcAddr`. SDL keeps the library loaded until it quits,
    /// after the application's `quit` callback.
    pub fn get_instance_proc_addr(&self) -> vk::PFN_vkGetInstanceProcAddr {
        self.get_instance_proc_addr
    }

    /// Instance extensions required to create surfaces on this platform.
    pub fn required_extensions(&self) -> Vec<&CStr> {
        self.extensions.iter().map(CString::as_c_str).collect()
    }
}

impl fmt::Debug for VulkanSupport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VulkanSupport")
            .field("extensions", &self.extensions)
            .finish_non_exhaustive()
    }
}
