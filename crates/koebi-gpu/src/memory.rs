//! Device memory allocation.
//!
//! Strategy (see `docs/architecture/gpu-abstraction.md`): one dedicated allocation per
//! long-lived resource, plus per-frame host-visible buffers for streaming data. A UI renderer
//! owns few long-lived resources, so this stays far below `maxMemoryAllocationCount`. A
//! sub-allocator (TLSF) can replace this module without changing the public API.

use std::ptr::NonNull;

use ash::vk;

use crate::device::DeviceShared;
use crate::error::VkResultExt as _;
use crate::{Error, Result};

/// Where a resource's memory lives, from the CPU's point of view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MemoryLocation {
    /// Device-local, not CPU-accessible: textures, static geometry.
    GpuOnly,
    /// CPU-written every frame, GPU-read: instance data, uniforms. Prefers device-local
    /// host-visible memory (resizable BAR / unified memory) when available.
    CpuToGpu,
    /// GPU-written, CPU-read: screenshots, readback in tests.
    GpuToCpu,
}

impl MemoryLocation {
    /// `(required, preferred)` property flags.
    fn flags(self) -> (vk::MemoryPropertyFlags, vk::MemoryPropertyFlags) {
        use vk::MemoryPropertyFlags as F;
        match self {
            Self::GpuOnly => (F::empty(), F::DEVICE_LOCAL),
            Self::CpuToGpu => (F::HOST_VISIBLE | F::HOST_COHERENT, F::DEVICE_LOCAL),
            Self::GpuToCpu => (F::HOST_VISIBLE | F::HOST_COHERENT, F::HOST_CACHED),
        }
    }

    pub(crate) fn is_host_visible(self) -> bool {
        self != Self::GpuOnly
    }
}

/// A mapped pointer into host-visible device memory.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MappedPtr(NonNull<u8>);

// SAFETY: the pointer is an address inside device memory owned by one allocation. Rust-side
// aliasing is controlled by the owning resource (`&mut self` for writes), not by the pointer.
unsafe impl Send for MappedPtr {}
// SAFETY: see `Send`; shared access only reads.
unsafe impl Sync for MappedPtr {}

impl MappedPtr {
    pub(crate) fn as_ptr(self) -> *mut u8 {
        self.0.as_ptr()
    }
}

/// One `VkDeviceMemory` block backing exactly one resource.
#[derive(Debug)]
pub(crate) struct Allocation {
    pub(crate) memory: vk::DeviceMemory,
    pub(crate) mapped: Option<MappedPtr>,
}

/// Allocates (and maps, for host-visible locations) memory satisfying `requirements`.
pub(crate) fn allocate(
    device: &DeviceShared,
    requirements: vk::MemoryRequirements,
    location: MemoryLocation,
) -> Result<Allocation> {
    let (required, preferred) = location.flags();
    let type_bits = requirements.memory_type_bits;
    let memory_type = find_memory_type(&device.memory_properties, type_bits, required | preferred)
        .or_else(|| find_memory_type(&device.memory_properties, type_bits, required))
        .ok_or(Error::NoCompatibleMemoryType)?;

    let info = vk::MemoryAllocateInfo::default()
        .allocation_size(requirements.size)
        .memory_type_index(memory_type);
    // SAFETY: valid device; the memory type index comes from this device's properties.
    let memory = unsafe { device.raw.allocate_memory(&info, None) }.ctx("vkAllocateMemory")?;

    let mapped = if location.is_host_visible() {
        // SAFETY: the memory is host-visible, not mapped yet, and the range is the whole block.
        let pointer = unsafe {
            device
                .raw
                .map_memory(memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
        };
        if let Ok(Some(pointer)) = pointer.map(|p| NonNull::new(p.cast::<u8>())) {
            Some(MappedPtr(pointer))
        } else {
            // SAFETY: freshly allocated, unused memory.
            unsafe { device.raw.free_memory(memory, None) };
            return Err(Error::Vulkan {
                call: "vkMapMemory",
                result: vk::Result::ERROR_MEMORY_MAP_FAILED,
            });
        }
    } else {
        None
    };
    Ok(Allocation { memory, mapped })
}

/// Index of the first memory type allowed by `type_bits` that has all `flags`.
pub(crate) fn find_memory_type(
    properties: &vk::PhysicalDeviceMemoryProperties,
    type_bits: u32,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    properties
        .memory_types_as_slice()
        .iter()
        .zip(0_u32..)
        .find_map(|(memory_type, index)| {
            let allowed = type_bits & (1 << index) != 0;
            (allowed && memory_type.property_flags.contains(flags)).then_some(index)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn properties(types: &[vk::MemoryPropertyFlags]) -> vk::PhysicalDeviceMemoryProperties {
        let mut properties = vk::PhysicalDeviceMemoryProperties {
            memory_type_count: u32::try_from(types.len()).unwrap(),
            ..Default::default()
        };
        for (slot, flags) in properties.memory_types.iter_mut().zip(types) {
            slot.property_flags = *flags;
        }
        properties
    }

    #[test]
    fn picks_first_type_with_all_flags_allowed_by_mask() {
        use vk::MemoryPropertyFlags as F;
        let props = properties(&[
            F::DEVICE_LOCAL,
            F::HOST_VISIBLE | F::HOST_COHERENT,
            F::DEVICE_LOCAL | F::HOST_VISIBLE | F::HOST_COHERENT,
        ]);
        assert_eq!(find_memory_type(&props, 0b111, F::DEVICE_LOCAL), Some(0));
        assert_eq!(find_memory_type(&props, 0b110, F::DEVICE_LOCAL), Some(2));
        assert_eq!(find_memory_type(&props, 0b111, F::HOST_VISIBLE), Some(1));
        assert_eq!(find_memory_type(&props, 0b001, F::HOST_VISIBLE), None);
    }

    #[test]
    fn cpu_to_gpu_requires_coherent_host_memory() {
        let (required, preferred) = MemoryLocation::CpuToGpu.flags();
        assert!(required.contains(vk::MemoryPropertyFlags::HOST_COHERENT));
        assert!(preferred.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL));
        assert!(!MemoryLocation::GpuOnly.is_host_visible());
    }
}
