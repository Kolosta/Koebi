//! Locating and opening the Vulkan loader.
//!
//! Koebi never links against `vulkan-1`/`libvulkan` at build time. The library is opened at
//! runtime so that a missing Vulkan installation is a recoverable error with a clear message,
//! and so that GPU tests can be skipped on machines without a GPU. This costs about a hundred
//! lines of FFI instead of a `libloading` dependency.
//!
//! On iOS there is no loader: MoltenVK is linked statically by this crate's build script and
//! [`VulkanLibrary::open_default`] returns its `vkGetInstanceProcAddr` directly.
//!
//! Windowed applications usually let SDL open the library (so that SDL and Koebi share the
//! same instance functions); [`preferred_library_path`] tells SDL which file to use.

use ash::vk;

use crate::{Error, Result};

/// Environment variable overriding the Vulkan library path for every Koebi process.
pub const LIBRARY_ENV: &str = "KOEBI_VULKAN_LIBRARY";

/// An opened Vulkan loader (or driver) library.
///
/// The library is intentionally never unloaded: Vulkan objects may live until the end of the
/// process, and unloading the code under them would be undefined behaviour.
#[derive(Debug)]
pub struct VulkanLibrary {
    path: String,
    get_instance_proc_addr: vk::PFN_vkGetInstanceProcAddr,
}

impl VulkanLibrary {
    /// Opens the first library that loads among [`default_candidates`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::LibraryNotFound`] listing every candidate and why it failed.
    pub fn open_default() -> Result<Self> {
        #[cfg(target_os = "ios")]
        {
            Ok(Self {
                path: String::from("MoltenVK (statically linked)"),
                get_instance_proc_addr: ios::vkGetInstanceProcAddr,
            })
        }
        #[cfg(not(target_os = "ios"))]
        {
            let mut tried = Vec::new();
            for candidate in default_candidates() {
                match Self::open(&candidate) {
                    Ok(library) => return Ok(library),
                    Err(error) => tried.push(format!("{candidate}: {error}")),
                }
            }
            Err(Error::LibraryNotFound { tried })
        }
    }

    /// Opens the library at `path` (a file name searched by the OS, or a full path).
    ///
    /// # Errors
    ///
    /// Returns [`Error::LibraryNotFound`] if the library cannot be opened, or
    /// [`Error::MissingEntryPoint`] if it does not export `vkGetInstanceProcAddr`.
    pub fn open(path: &str) -> Result<Self> {
        let get_instance_proc_addr = sys::open(path)?;
        Ok(Self {
            path: path.to_owned(),
            get_instance_proc_addr,
        })
    }

    /// The path or name the library was opened from.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The library's `vkGetInstanceProcAddr`, valid for the rest of the process.
    pub fn get_instance_proc_addr(&self) -> vk::PFN_vkGetInstanceProcAddr {
        self.get_instance_proc_addr
    }
}

/// Returns an explicit library path when the platform's default search would pick the wrong
/// file, or `None` to let the OS (or SDL) search.
///
/// The [`LIBRARY_ENV`] variable always wins. On macOS, a LunarG SDK designated by
/// `VULKAN_SDK` is preferred so that the loader (and its validation layers) is used rather
/// than a bare MoltenVK.
pub fn preferred_library_path() -> Option<String> {
    if let Ok(path) = std::env::var(LIBRARY_ENV)
        && !path.is_empty()
    {
        return Some(path);
    }
    #[cfg(target_os = "macos")]
    if let Some(sdk) = std::env::var_os("VULKAN_SDK") {
        let path = std::path::Path::new(&sdk)
            .join("lib")
            .join("libvulkan.1.dylib");
        if path.is_file() {
            return Some(path.to_string_lossy().into_owned());
        }
    }
    None
}

/// Library names tried by [`VulkanLibrary::open_default`], in order.
pub fn default_candidates() -> Vec<String> {
    let mut candidates: Vec<String> = preferred_library_path().into_iter().collect();
    let defaults: &[&str] = if cfg!(windows) {
        &["vulkan-1.dll"]
    } else if cfg!(target_os = "android") {
        &["libvulkan.so"]
    } else if cfg!(target_os = "macos") {
        &[
            "libvulkan.1.dylib",
            "/usr/local/lib/libvulkan.1.dylib",
            "/opt/homebrew/lib/libvulkan.1.dylib",
            "libMoltenVK.dylib",
        ]
    } else {
        &["libvulkan.so.1", "libvulkan.so"]
    };
    candidates.extend(defaults.iter().map(|name| (*name).to_owned()));
    candidates
}

#[cfg(target_os = "ios")]
mod ios {
    use ash::vk;

    unsafe extern "system" {
        /// Exported by the statically linked MoltenVK.
        pub(super) fn vkGetInstanceProcAddr(
            instance: vk::Instance,
            name: *const core::ffi::c_char,
        ) -> vk::PFN_vkVoidFunction;
    }
}

#[cfg(windows)]
mod sys {
    use std::ffi::{c_char, c_void};
    use std::os::windows::ffi::OsStrExt as _;

    use ash::vk;

    use crate::{Error, Result};

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryW(file_name: *const u16) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
        fn GetLastError() -> u32;
    }

    pub(super) fn open(path: &str) -> Result<vk::PFN_vkGetInstanceProcAddr> {
        let wide: Vec<u16> = std::ffi::OsStr::new(path)
            .encode_wide()
            .chain([0])
            .collect();
        // SAFETY: `wide` is a valid NUL-terminated UTF-16 string for the duration of the call.
        let module = unsafe { LoadLibraryW(wide.as_ptr()) };
        if module.is_null() {
            // SAFETY: no other Win32 call happened on this thread since LoadLibraryW.
            let code = unsafe { GetLastError() };
            return Err(Error::LibraryNotFound {
                tried: vec![format!("Win32 error {code}")],
            });
        }
        // SAFETY: `module` is a valid module handle and the name is NUL-terminated.
        let symbol = unsafe { GetProcAddress(module, c"vkGetInstanceProcAddr".as_ptr()) };
        if symbol.is_null() {
            return Err(Error::MissingEntryPoint("vkGetInstanceProcAddr"));
        }
        // SAFETY: the Vulkan specification defines this symbol with exactly the signature of
        // `PFN_vkGetInstanceProcAddr`; function and data pointers have the same size here.
        Ok(unsafe { std::mem::transmute::<*mut c_void, vk::PFN_vkGetInstanceProcAddr>(symbol) })
    }
}

#[cfg(all(unix, not(target_os = "ios")))]
mod sys {
    use std::ffi::{CStr, CString, c_char, c_int, c_void};

    use ash::vk;

    use crate::{Error, Result};

    const RTLD_NOW: c_int = 2;

    #[cfg_attr(target_os = "android", link(name = "dl"))]
    unsafe extern "C" {
        fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        fn dlerror() -> *mut c_char;
    }

    fn last_error() -> String {
        // SAFETY: dlerror returns null or a valid NUL-terminated, thread-local string.
        let message = unsafe { dlerror() };
        if message.is_null() {
            String::from("unknown dlopen error")
        } else {
            // SAFETY: checked non-null above; the string stays valid until the next dl* call.
            unsafe { CStr::from_ptr(message) }
                .to_string_lossy()
                .into_owned()
        }
    }

    pub(super) fn open(path: &str) -> Result<vk::PFN_vkGetInstanceProcAddr> {
        let c_path = CString::new(path)
            .map_err(|_| Error::InvalidArgument(format!("library path {path:?} contains NUL")))?;
        // SAFETY: `c_path` is a valid NUL-terminated string for the duration of the call.
        let handle = unsafe { dlopen(c_path.as_ptr(), RTLD_NOW) };
        if handle.is_null() {
            return Err(Error::LibraryNotFound {
                tried: vec![last_error()],
            });
        }
        // SAFETY: `handle` was returned by dlopen and the symbol name is NUL-terminated.
        let symbol = unsafe { dlsym(handle, c"vkGetInstanceProcAddr".as_ptr()) };
        if symbol.is_null() {
            return Err(Error::MissingEntryPoint("vkGetInstanceProcAddr"));
        }
        // SAFETY: the Vulkan specification defines this symbol with exactly the signature of
        // `PFN_vkGetInstanceProcAddr`; POSIX guarantees dlsym results convert to fn pointers.
        Ok(unsafe { std::mem::transmute::<*mut c_void, vk::PFN_vkGetInstanceProcAddr>(symbol) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_end_with_platform_defaults() {
        let candidates = default_candidates();
        let last = candidates.last().map(String::as_str);
        assert!(last.is_some());
        if cfg!(windows) {
            assert_eq!(last, Some("vulkan-1.dll"));
        } else if cfg!(target_os = "linux") {
            assert_eq!(last, Some("libvulkan.so"));
        }
    }

    #[cfg(not(target_os = "ios"))]
    #[test]
    fn opening_a_missing_library_is_a_recoverable_error() {
        let error = VulkanLibrary::open("koebi-definitely-missing-library").unwrap_err();
        assert!(error.is_unsupported_environment(), "{error}");
    }
}
