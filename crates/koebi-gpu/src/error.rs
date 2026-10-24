//! Error type of the GPU layer.

use std::fmt;

use ash::vk;

/// Errors returned by `koebi-gpu`.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// No Vulkan loader library could be opened. Lists every attempt and its reason.
    LibraryNotFound {
        /// `"<path>: <reason>"` for each candidate.
        tried: Vec<String>,
    },
    /// A mandatory entry point is missing from the Vulkan library.
    MissingEntryPoint(&'static str),
    /// The Vulkan implementation is older than Koebi's baseline (Vulkan 1.2).
    UnsupportedVulkanVersion {
        /// Version reported by the loader or device.
        found: u32,
        /// Minimum version required.
        required: u32,
    },
    /// A required layer (e.g. validation, when required) is not installed.
    MissingLayer(String),
    /// A required instance or device extension is not available.
    MissingExtension(String),
    /// No physical device satisfies Koebi's requirements.
    NoSuitableAdapter,
    /// No memory type satisfies a resource's requirements.
    NoCompatibleMemoryType,
    /// The presentation surface is gone (window destroyed, Android surface lost).
    SurfaceLost,
    /// The GPU device was lost (driver reset or hang). The device must be recreated.
    DeviceLost,
    /// The caller passed an invalid argument (caught at runtime instead of reaching Vulkan).
    InvalidArgument(String),
    /// Any other failing Vulkan call.
    Vulkan {
        /// Name of the failing Vulkan function.
        call: &'static str,
        /// Result code it returned.
        result: vk::Result,
    },
}

impl Error {
    /// Converts a failing Vulkan result, mapping the codes callers must handle specially.
    pub(crate) fn from_vk(call: &'static str, result: vk::Result) -> Self {
        match result {
            vk::Result::ERROR_DEVICE_LOST => Self::DeviceLost,
            vk::Result::ERROR_SURFACE_LOST_KHR => Self::SurfaceLost,
            _ => Self::Vulkan { call, result },
        }
    }

    /// Returns `true` for errors meaning "this machine cannot run Koebi" (no Vulkan, too old,
    /// missing features) as opposed to bugs or transient failures.
    pub fn is_unsupported_environment(&self) -> bool {
        matches!(
            self,
            Self::LibraryNotFound { .. }
                | Self::MissingEntryPoint(_)
                | Self::UnsupportedVulkanVersion { .. }
                | Self::MissingLayer(_)
                | Self::MissingExtension(_)
                | Self::NoSuitableAdapter
                | Self::Vulkan {
                    result: vk::Result::ERROR_INCOMPATIBLE_DRIVER,
                    ..
                }
        )
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LibraryNotFound { tried } => {
                write!(f, "no Vulkan library found (tried: {})", tried.join("; "))
            }
            Self::MissingEntryPoint(name) => write!(f, "Vulkan entry point {name} is missing"),
            Self::UnsupportedVulkanVersion { found, required } => write!(
                f,
                "Vulkan {} is required, found {}",
                version_string(*required),
                version_string(*found)
            ),
            Self::MissingLayer(name) => write!(f, "Vulkan layer {name} is not installed"),
            Self::MissingExtension(name) => write!(f, "Vulkan extension {name} is not available"),
            Self::NoSuitableAdapter => f.write_str(
                "no GPU supports Vulkan 1.2 with dynamic rendering and synchronization2",
            ),
            Self::NoCompatibleMemoryType => f.write_str("no compatible GPU memory type"),
            Self::SurfaceLost => f.write_str("the presentation surface was lost"),
            Self::DeviceLost => f.write_str("the GPU device was lost"),
            Self::InvalidArgument(message) => write!(f, "invalid argument: {message}"),
            Self::Vulkan { call, result } => write!(f, "{call} failed: {result}"),
        }
    }
}

impl std::error::Error for Error {}

/// Result alias used throughout `koebi-gpu`.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Adds the failing call name to raw Vulkan results.
pub(crate) trait VkResultExt<T> {
    fn ctx(self, call: &'static str) -> Result<T>;
}

impl<T> VkResultExt<T> for ash::prelude::VkResult<T> {
    fn ctx(self, call: &'static str) -> Result<T> {
        self.map_err(|result| Error::from_vk(call, result))
    }
}

/// Formats a packed Vulkan version as `major.minor.patch`.
pub fn version_string(version: u32) -> String {
    format!(
        "{}.{}.{}",
        vk::api_version_major(version),
        vk::api_version_minor(version),
        vk::api_version_patch(version)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_and_surface_loss_get_dedicated_variants() {
        assert!(matches!(
            Error::from_vk("x", vk::Result::ERROR_DEVICE_LOST),
            Error::DeviceLost
        ));
        assert!(matches!(
            Error::from_vk("x", vk::Result::ERROR_SURFACE_LOST_KHR),
            Error::SurfaceLost
        ));
        assert!(matches!(
            Error::from_vk("vkCreateBuffer", vk::Result::ERROR_OUT_OF_DEVICE_MEMORY),
            Error::Vulkan {
                call: "vkCreateBuffer",
                ..
            }
        ));
    }

    #[test]
    fn unsupported_environment_classification() {
        assert!(Error::NoSuitableAdapter.is_unsupported_environment());
        assert!(Error::LibraryNotFound { tried: vec![] }.is_unsupported_environment());
        assert!(!Error::DeviceLost.is_unsupported_environment());
        assert!(!Error::InvalidArgument("x".into()).is_unsupported_environment());
    }

    #[test]
    fn versions_are_formatted_semantically() {
        assert_eq!(
            version_string(vk::make_api_version(0, 1, 3, 281)),
            "1.3.281"
        );
        let message = Error::UnsupportedVulkanVersion {
            found: vk::API_VERSION_1_1,
            required: vk::API_VERSION_1_2,
        }
        .to_string();
        assert_eq!(message, "Vulkan 1.2.0 is required, found 1.1.0");
    }
}
