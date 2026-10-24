//! Presentation surfaces created by the windowing layer.

use std::fmt;

use ash::vk;

use crate::instance::Instance;
use crate::{Error, Result};

/// A `VkSurfaceKHR` owned by Koebi.
///
/// The windowing layer (`koebi-platform`) creates the raw surface; ownership is then
/// transferred here so that destruction happens before the instance, automatically.
pub struct Surface {
    raw: vk::SurfaceKHR,
    instance: Instance,
}

impl Surface {
    /// Takes ownership of a raw surface.
    ///
    /// # Safety
    ///
    /// `raw` must be a valid surface created from `instance`, and nothing else may destroy
    /// it. The native window must outlive the returned value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingExtension`] if the instance was created without
    /// `VK_KHR_surface`.
    pub unsafe fn from_raw(instance: &Instance, raw: vk::SurfaceKHR) -> Result<Self> {
        if instance.shared.surface.is_none() {
            return Err(Error::MissingExtension("VK_KHR_surface".into()));
        }
        Ok(Self {
            raw,
            instance: instance.clone(),
        })
    }

    /// The raw surface handle.
    pub fn handle(&self) -> vk::SurfaceKHR {
        self.raw
    }

    pub(crate) fn loader(&self) -> &ash::khr::surface::Instance {
        // `from_raw` guarantees the loader exists.
        self.instance
            .shared
            .surface
            .as_ref()
            .unwrap_or_else(|| unreachable!())
    }
}

impl fmt::Debug for Surface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Surface").field(&self.raw).finish()
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        // SAFETY: we own the surface (contract of `from_raw`); any swapchain using it is owned
        // by a `Presenter`, which destroys it before dropping its `Surface`.
        unsafe { self.loader().destroy_surface(self.raw, None) };
    }
}
