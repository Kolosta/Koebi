//! The Vulkan instance: loader entry, validation layers and debug messenger.

use std::ffi::{CStr, CString, c_char};
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use ash::vk;

use crate::adapter::{Adapter, AdapterPreference};
use crate::debug::{self, DebugState};
use crate::error::VkResultExt as _;
use crate::loader::VulkanLibrary;
use crate::surface::Surface;
use crate::{Error, Result};

/// The Khronos validation layer.
pub(crate) const VALIDATION_LAYER: &CStr = c"VK_LAYER_KHRONOS_validation";
/// Minimum loader and device version.
pub(crate) const MIN_API_VERSION: u32 = vk::API_VERSION_1_2;

/// Whether to enable the Khronos validation layers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Validation {
    /// Never enable validation (release builds).
    #[default]
    Disabled,
    /// Enable validation when the layers are installed, continue without them otherwise.
    Enabled,
    /// Fail instance creation when the layers are missing (CI).
    Required,
}

impl Validation {
    /// Reads `KOEBI_VALIDATION` (`0`/`off`, `1`/`on`, `required`), or returns `default`.
    pub fn from_env_or(default: Self) -> Self {
        match std::env::var("KOEBI_VALIDATION").map(|v| v.trim().to_ascii_lowercase()) {
            Ok(v) if matches!(v.as_str(), "0" | "off" | "false" | "no") => Self::Disabled,
            Ok(v) if matches!(v.as_str(), "1" | "on" | "true" | "yes") => Self::Enabled,
            Ok(v) if v == "required" => Self::Required,
            _ => default,
        }
    }
}

/// Parameters of [`Instance::new`].
#[derive(Clone, Copy, Debug)]
pub struct InstanceDesc<'a> {
    /// Application name reported to drivers and tools.
    pub application_name: &'a str,
    /// Application version `(major, minor, patch)`.
    pub application_version: (u32, u32, u32),
    /// Validation layer policy.
    pub validation: Validation,
    /// Instance extensions required by the windowing system (from `koebi-platform`).
    pub required_extensions: &'a [&'a CStr],
}

/// Validation messages counted since the instance was created.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ValidationReport {
    /// Messages of error severity: API misuse, to be fixed.
    pub errors: u32,
    /// Messages of warning severity.
    pub warnings: u32,
}

impl ValidationReport {
    /// `true` when no validation error was reported.
    pub fn is_clean(self) -> bool {
        self.errors == 0
    }
}

/// A Vulkan instance. Cheap to clone (reference counted).
///
/// Devices and surfaces keep their instance alive, so drop order cannot cause use-after-free.
#[derive(Clone)]
pub struct Instance {
    pub(crate) shared: Arc<InstanceShared>,
}

pub(crate) struct InstanceShared {
    pub(crate) raw: ash::Instance,
    pub(crate) api_version: u32,
    pub(crate) surface: Option<ash::khr::surface::Instance>,
    pub(crate) debug_utils: Option<ash::ext::debug_utils::Instance>,
    pub(crate) validation_enabled: bool,
    messenger: vk::DebugUtilsMessengerEXT,
    debug_state: Box<DebugState>,
    // Keeps the loader's function table alive alongside the instance.
    _entry: ash::Entry,
}

impl Instance {
    /// Creates an instance from the loader's `vkGetInstanceProcAddr`.
    ///
    /// # Safety
    ///
    /// `get_instance_proc_addr` must be the `vkGetInstanceProcAddr` of a Vulkan loader or
    /// driver that stays loaded until the process exits (as with [`VulkanLibrary`] or SDL).
    ///
    /// # Errors
    ///
    /// Fails if the loader is older than Vulkan 1.2, a required extension or layer is
    /// missing, or `vkCreateInstance` fails.
    pub unsafe fn new(
        get_instance_proc_addr: vk::PFN_vkGetInstanceProcAddr,
        desc: &InstanceDesc<'_>,
    ) -> Result<Self> {
        // SAFETY: the caller guarantees the function pointer is valid for the process lifetime.
        let entry = unsafe {
            ash::Entry::from_static_fn(ash::StaticFn {
                get_instance_proc_addr,
            })
        };

        // SAFETY: valid entry; the call has no other preconditions.
        let loader_version = unsafe { entry.try_enumerate_instance_version() }
            .ctx("vkEnumerateInstanceVersion")?
            .unwrap_or(vk::API_VERSION_1_0);
        if loader_version < MIN_API_VERSION {
            return Err(Error::UnsupportedVulkanVersion {
                found: loader_version,
                required: MIN_API_VERSION,
            });
        }

        // SAFETY: valid entry; `None` queries the loader and implicit layers.
        let available = unsafe { entry.enumerate_instance_extension_properties(None) }
            .ctx("vkEnumerateInstanceExtensionProperties")?;
        let has_extension = |name: &CStr| {
            available
                .iter()
                .any(|ext| ext.extension_name_as_c_str().is_ok_and(|n| n == name))
        };

        let validation = resolve_validation(&entry, desc.validation)?;
        let validation_features = validation && {
            // SAFETY: valid entry and NUL-terminated layer name.
            unsafe { entry.enumerate_instance_extension_properties(Some(VALIDATION_LAYER)) }
                .unwrap_or_default()
                .iter()
                .any(|ext| {
                    ext.extension_name_as_c_str()
                        .is_ok_and(|n| n == ash::ext::validation_features::NAME)
                })
        };

        let mut extensions: Vec<*const c_char> = Vec::new();
        for name in desc.required_extensions {
            if !has_extension(name) {
                return Err(Error::MissingExtension(name.to_string_lossy().into_owned()));
            }
            extensions.push(name.as_ptr());
        }
        let debug_utils = has_extension(ash::ext::debug_utils::NAME);
        if debug_utils {
            extensions.push(ash::ext::debug_utils::NAME.as_ptr());
        }
        // Required to see MoltenVK (and other non-conformant drivers) through the loader.
        let portability = has_extension(ash::khr::portability_enumeration::NAME);
        if portability {
            extensions.push(ash::khr::portability_enumeration::NAME.as_ptr());
        }
        if validation_features {
            extensions.push(ash::ext::validation_features::NAME.as_ptr());
        }
        // MoltenVK's Metal argument buffers fail on the iOS simulator's GPU: Metal validation
        // reports "No argument buffer is set" at the first descriptor write, and without it the
        // application crashes. There, descriptor sets bind their resources directly instead.
        let moltenvk_settings =
            cfg!(ios_simulator) && has_extension(ash::ext::layer_settings::NAME);
        if moltenvk_settings {
            extensions.push(ash::ext::layer_settings::NAME.as_ptr());
        } else if cfg!(ios_simulator) {
            koebi_core::warn!("no VK_EXT_layer_settings: MoltenVK keeps Metal argument buffers");
        }
        let surface = desc.required_extensions.contains(&ash::khr::surface::NAME);

        let application_name = CString::new(desc.application_name).unwrap_or_default();
        let (major, minor, patch) = desc.application_version;
        let engine_version = env!("CARGO_PKG_VERSION_MINOR").parse().unwrap_or(0);
        let application_info = vk::ApplicationInfo::default()
            .application_name(&application_name)
            .application_version(vk::make_api_version(0, major, minor, patch))
            .engine_name(c"Koebi")
            .engine_version(vk::make_api_version(0, 0, engine_version, 0))
            .api_version(vk::API_VERSION_1_3);
        let layers = if validation {
            vec![VALIDATION_LAYER.as_ptr()]
        } else {
            Vec::new()
        };
        let flags = if portability {
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            vk::InstanceCreateFlags::empty()
        };

        let debug_state = Box::<DebugState>::default();
        let mut creation_messenger = debug::messenger_create_info(&debug_state);
        // Synchronization validation catches missing barriers in our hand-written frame logic.
        let enabled_features = [vk::ValidationFeatureEnableEXT::SYNCHRONIZATION_VALIDATION];
        let mut features =
            vk::ValidationFeaturesEXT::default().enabled_validation_features(&enabled_features);

        // MVK_CONFIG_USE_METAL_ARGUMENT_BUFFERS_NEVER.
        let never = 0_i32;
        let mut argument_buffers = vk::LayerSettingEXT::default()
            .layer_name(c"MoltenVK")
            .setting_name(c"MVK_CONFIG_USE_METAL_ARGUMENT_BUFFERS")
            .ty(vk::LayerSettingTypeEXT::INT32);
        // One 32-bit value: ash's `values` would count bytes.
        argument_buffers.value_count = 1;
        argument_buffers.p_values = (&raw const never).cast();
        let settings = [argument_buffers];
        let mut layer_settings = vk::LayerSettingsCreateInfoEXT::default().settings(&settings);

        let mut create_info = vk::InstanceCreateInfo::default()
            .flags(flags)
            .application_info(&application_info)
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions);
        if debug_utils {
            create_info = create_info.push_next(&mut creation_messenger);
        }
        if validation_features {
            create_info = create_info.push_next(&mut features);
        }
        if moltenvk_settings {
            create_info = create_info.push_next(&mut layer_settings);
        }

        // SAFETY: every pointer in `create_info` references locals that outlive the call.
        let raw = unsafe { entry.create_instance(&create_info, None) }.ctx("vkCreateInstance")?;

        let debug_utils = debug_utils.then(|| ash::ext::debug_utils::Instance::new(&entry, &raw));
        let messenger = match &debug_utils {
            Some(loader) => {
                let info = debug::messenger_create_info(&debug_state);
                // SAFETY: valid instance; `debug_state` outlives the messenger (see Drop).
                unsafe { loader.create_debug_utils_messenger(&info, None) }
                    .unwrap_or_else(|_| vk::DebugUtilsMessengerEXT::null())
            }
            None => vk::DebugUtilsMessengerEXT::null(),
        };
        let surface = surface.then(|| ash::khr::surface::Instance::new(&entry, &raw));

        koebi_core::info!(
            "Vulkan instance created (loader {}, validation {})",
            crate::error::version_string(loader_version),
            if validation { "on" } else { "off" }
        );

        Ok(Self {
            shared: Arc::new(InstanceShared {
                raw,
                api_version: loader_version.min(vk::API_VERSION_1_3),
                surface,
                debug_utils,
                validation_enabled: validation,
                messenger,
                debug_state,
                _entry: entry,
            }),
        })
    }

    /// Creates an instance from a library opened by [`VulkanLibrary`].
    ///
    /// # Errors
    ///
    /// See [`Instance::new`].
    pub fn from_library(library: &VulkanLibrary, desc: &InstanceDesc<'_>) -> Result<Self> {
        // SAFETY: `VulkanLibrary` is never unloaded and yields a valid vkGetInstanceProcAddr.
        unsafe { Self::new(library.get_instance_proc_addr(), desc) }
    }

    /// The raw instance handle, e.g. for windowing-system surface creation.
    pub fn handle(&self) -> vk::Instance {
        self.shared.raw.handle()
    }

    /// The instance API version in use (`min(loader, 1.3)`).
    pub fn api_version(&self) -> u32 {
        self.shared.api_version
    }

    /// `true` when the validation layers are active.
    pub fn validation_enabled(&self) -> bool {
        self.shared.validation_enabled
    }

    /// Validation messages reported so far.
    pub fn validation_report(&self) -> ValidationReport {
        ValidationReport {
            errors: self.shared.debug_state.errors.load(Ordering::Relaxed),
            warnings: self.shared.debug_state.warnings.load(Ordering::Relaxed),
        }
    }

    /// Lists every adapter meeting Koebi's requirements, optionally able to present to
    /// `surface`.
    ///
    /// # Errors
    ///
    /// Fails if the physical devices cannot be enumerated.
    pub fn adapters(&self, surface: Option<&Surface>) -> Result<Vec<Adapter>> {
        // SAFETY: valid instance.
        let devices = unsafe { self.shared.raw.enumerate_physical_devices() }
            .ctx("vkEnumeratePhysicalDevices")?;
        Ok(devices
            .into_iter()
            .filter_map(|raw| match Adapter::probe(&self.shared, raw, surface) {
                Ok(adapter) => Some(adapter),
                Err(reason) => {
                    koebi_core::debug!("skipping adapter: {reason}");
                    None
                }
            })
            .collect())
    }

    /// Picks the best adapter for `preference`, able to present to `surface` if given.
    ///
    /// Setting `KOEBI_ADAPTER=<substring>` forces the first adapter whose name contains the
    /// substring (case-insensitive), e.g. `llvmpipe` to test the software rasterizer.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NoSuitableAdapter`] when nothing qualifies.
    pub fn select_adapter(
        &self,
        preference: AdapterPreference,
        surface: Option<&Surface>,
    ) -> Result<Adapter> {
        let adapters = self.adapters(surface)?;
        for adapter in &adapters {
            koebi_core::debug!("candidate adapter: {:?}", adapter.info());
        }
        let forced = std::env::var("KOEBI_ADAPTER")
            .ok()
            .filter(|name| !name.is_empty());
        let chosen = match forced {
            Some(filter) => {
                let filter = filter.to_lowercase();
                adapters
                    .into_iter()
                    .find(|a| a.info().name.to_lowercase().contains(&filter))
            }
            None => adapters
                .into_iter()
                .min_by_key(|a| a.info().kind.rank(preference)),
        };
        chosen.ok_or(Error::NoSuitableAdapter)
    }
}

impl fmt::Debug for Instance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Instance")
            .field("handle", &self.handle())
            .field(
                "api_version",
                &crate::error::version_string(self.api_version()),
            )
            .field("validation", &self.validation_enabled())
            .finish()
    }
}

impl Drop for InstanceShared {
    fn drop(&mut self) {
        if let Some(debug_utils) = &self.debug_utils
            && self.messenger != vk::DebugUtilsMessengerEXT::null()
        {
            // SAFETY: the messenger belongs to this instance and is destroyed exactly once.
            unsafe { debug_utils.destroy_debug_utils_messenger(self.messenger, None) };
        }
        // SAFETY: devices and surfaces hold an `Arc` to this instance, so all child objects
        // are already destroyed when the last reference drops.
        unsafe { self.raw.destroy_instance(None) };
    }
}

fn resolve_validation(entry: &ash::Entry, policy: Validation) -> Result<bool> {
    if policy == Validation::Disabled {
        return Ok(false);
    }
    // SAFETY: valid entry; the call has no other preconditions.
    let layers = unsafe { entry.enumerate_instance_layer_properties() }
        .ctx("vkEnumerateInstanceLayerProperties")?;
    let available = layers.iter().any(|layer| {
        layer
            .layer_name_as_c_str()
            .is_ok_and(|n| n == VALIDATION_LAYER)
    });
    match (policy, available) {
        (_, true) => Ok(true),
        (Validation::Required, false) => Err(Error::MissingLayer(
            VALIDATION_LAYER.to_string_lossy().into_owned(),
        )),
        _ => {
            koebi_core::warn!("Vulkan validation requested but the layers are not installed");
            Ok(false)
        }
    }
}
