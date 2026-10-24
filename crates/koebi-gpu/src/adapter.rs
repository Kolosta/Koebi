//! Physical devices ("adapters") and the capabilities Koebi requires from them.

use std::ffi::CStr;

use ash::vk;

use crate::instance::{InstanceShared, MIN_API_VERSION};
use crate::surface::Surface;

/// Hardware category of an adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AdapterKind {
    /// Dedicated GPU.
    Discrete,
    /// GPU integrated in the CPU package.
    Integrated,
    /// Virtualized GPU (VMs, cloud instances).
    Virtual,
    /// Software rasterizer (lavapipe, `SwiftShader`).
    Cpu,
    /// Anything else.
    Other,
}

impl AdapterKind {
    fn from_vk(kind: vk::PhysicalDeviceType) -> Self {
        match kind {
            vk::PhysicalDeviceType::DISCRETE_GPU => Self::Discrete,
            vk::PhysicalDeviceType::INTEGRATED_GPU => Self::Integrated,
            vk::PhysicalDeviceType::VIRTUAL_GPU => Self::Virtual,
            vk::PhysicalDeviceType::CPU => Self::Cpu,
            _ => Self::Other,
        }
    }

    /// Lower is better.
    pub(crate) fn rank(self, preference: AdapterPreference) -> u8 {
        match (preference, self) {
            (AdapterPreference::HighPerformance, Self::Discrete)
            | (AdapterPreference::LowPower, Self::Integrated) => 0,
            (AdapterPreference::HighPerformance, Self::Integrated)
            | (AdapterPreference::LowPower, Self::Discrete) => 1,
            (_, Self::Virtual) => 2,
            (_, Self::Other) => 3,
            (_, Self::Cpu) => 4,
        }
    }
}

/// Which adapter to favour when several qualify.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AdapterPreference {
    /// Prefer discrete GPUs (default).
    #[default]
    HighPerformance,
    /// Prefer integrated GPUs (battery life).
    LowPower,
}

/// Human-readable description of an adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterInfo {
    /// Device name, e.g. `NVIDIA GeForce RTX 3060` or `llvmpipe (LLVM 19.1.1, 256 bits)`.
    pub name: String,
    /// Hardware category.
    pub kind: AdapterKind,
    /// PCI vendor identifier.
    pub vendor_id: u32,
    /// PCI device identifier.
    pub device_id: u32,
    /// Vulkan version supported by the device (packed).
    pub api_version: u32,
    /// Driver name, e.g. `NVIDIA` or `llvmpipe`.
    pub driver_name: String,
    /// Driver version details.
    pub driver_info: String,
}

/// A physical device meeting Koebi's baseline: Vulkan 1.2 with dynamic rendering and
/// synchronization2 (core in 1.3, extensions before), plus a graphics queue.
#[derive(Clone, Debug)]
pub struct Adapter {
    pub(crate) raw: vk::PhysicalDevice,
    pub(crate) info: AdapterInfo,
    pub(crate) queue_family: u32,
    pub(crate) core_1_3: bool,
    pub(crate) has_swapchain: bool,
    pub(crate) has_portability_subset: bool,
}

const PORTABILITY_SUBSET: &CStr = c"VK_KHR_portability_subset";

impl Adapter {
    /// Description of the device.
    pub fn info(&self) -> &AdapterInfo {
        &self.info
    }

    /// The raw physical device handle.
    pub fn handle(&self) -> vk::PhysicalDevice {
        self.raw
    }

    /// `true` when dynamic rendering and synchronization2 come from Vulkan 1.3 core.
    pub fn uses_vulkan_1_3_core(&self) -> bool {
        self.core_1_3
    }

    /// Checks a physical device against Koebi's requirements.
    pub(crate) fn probe(
        instance: &InstanceShared,
        raw: vk::PhysicalDevice,
        surface: Option<&Surface>,
    ) -> Result<Self, String> {
        let vk_instance = &instance.raw;
        // SAFETY: `raw` was enumerated from this instance.
        let properties = unsafe { vk_instance.get_physical_device_properties(raw) };
        let name = properties.device_name_as_c_str().map_or_else(
            |_| String::from("<unnamed>"),
            |name| name.to_string_lossy().into_owned(),
        );
        let api_version = properties.api_version;
        if api_version < MIN_API_VERSION {
            return Err(format!(
                "{name}: Vulkan {} < 1.2",
                crate::error::version_string(api_version)
            ));
        }

        // SAFETY: `raw` was enumerated from this instance.
        let extensions = unsafe { vk_instance.enumerate_device_extension_properties(raw) }
            .map_err(|error| format!("{name}: cannot list extensions: {error}"))?;
        let has = |wanted: &CStr| {
            extensions
                .iter()
                .any(|ext| ext.extension_name_as_c_str().is_ok_and(|n| n == wanted))
        };

        let core_1_3 = api_version >= vk::API_VERSION_1_3;
        if !core_1_3
            && (!has(ash::khr::dynamic_rendering::NAME) || !has(ash::khr::synchronization2::NAME))
        {
            return Err(format!(
                "{name}: no dynamic rendering / synchronization2 support"
            ));
        }

        let mut dynamic_rendering = vk::PhysicalDeviceDynamicRenderingFeatures::default();
        let mut synchronization2 = vk::PhysicalDeviceSynchronization2Features::default();
        let mut features = vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut dynamic_rendering)
            .push_next(&mut synchronization2);
        // SAFETY: the chained structures are core 1.3 or come from extensions checked above.
        unsafe { vk_instance.get_physical_device_features2(raw, &mut features) };
        if dynamic_rendering.dynamic_rendering == vk::FALSE
            || synchronization2.synchronization2 == vk::FALSE
        {
            return Err(format!(
                "{name}: dynamic rendering / synchronization2 disabled"
            ));
        }

        let mut driver = vk::PhysicalDeviceDriverProperties::default();
        let mut properties2 = vk::PhysicalDeviceProperties2::default().push_next(&mut driver);
        // SAFETY: driver properties are core in Vulkan 1.2, checked above.
        unsafe { vk_instance.get_physical_device_properties2(raw, &mut properties2) };
        let to_string = |text: Result<&CStr, _>| {
            text.map_or_else(
                |_| String::new(),
                |t: &CStr| t.to_string_lossy().into_owned(),
            )
        };
        let driver_name = to_string(driver.driver_name_as_c_str());
        let driver_info = to_string(driver.driver_info_as_c_str());

        let queue_family = find_queue_family(instance, raw, surface)
            .ok_or_else(|| format!("{name}: no graphics queue able to present"))?;

        let has_swapchain = has(ash::khr::swapchain::NAME);
        if surface.is_some() && !has_swapchain {
            return Err(format!("{name}: no VK_KHR_swapchain"));
        }

        Ok(Self {
            raw,
            info: AdapterInfo {
                name,
                kind: AdapterKind::from_vk(properties.device_type),
                vendor_id: properties.vendor_id,
                device_id: properties.device_id,
                api_version,
                driver_name,
                driver_info,
            },
            queue_family,
            core_1_3,
            has_swapchain,
            has_portability_subset: has(PORTABILITY_SUBSET),
        })
    }
}

/// Finds a queue family with graphics support (and presentation support when a surface is
/// given). One queue does everything: Koebi's workloads are small and latency-bound.
fn find_queue_family(
    instance: &InstanceShared,
    raw: vk::PhysicalDevice,
    surface: Option<&Surface>,
) -> Option<u32> {
    // SAFETY: `raw` was enumerated from this instance.
    let families = unsafe {
        instance
            .raw
            .get_physical_device_queue_family_properties(raw)
    };
    families.iter().zip(0_u32..).find_map(|(family, index)| {
        if !family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
            return None;
        }
        match (surface, instance.surface.as_ref()) {
            (None, _) => Some(index),
            (Some(surface), Some(loader)) => {
                // SAFETY: the surface was created from this instance; index is in range.
                let supported = unsafe {
                    loader.get_physical_device_surface_support(raw, index, surface.handle())
                };
                supported.unwrap_or(false).then_some(index)
            }
            (Some(_), None) => None,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_performance_prefers_discrete_then_integrated_and_cpu_last() {
        let p = AdapterPreference::HighPerformance;
        let mut kinds = [
            AdapterKind::Cpu,
            AdapterKind::Integrated,
            AdapterKind::Other,
            AdapterKind::Discrete,
            AdapterKind::Virtual,
        ];
        kinds.sort_by_key(|k| k.rank(p));
        assert_eq!(
            kinds,
            [
                AdapterKind::Discrete,
                AdapterKind::Integrated,
                AdapterKind::Virtual,
                AdapterKind::Other,
                AdapterKind::Cpu
            ]
        );
    }

    #[test]
    fn low_power_prefers_integrated() {
        let p = AdapterPreference::LowPower;
        assert!(AdapterKind::Integrated.rank(p) < AdapterKind::Discrete.rank(p));
    }
}
