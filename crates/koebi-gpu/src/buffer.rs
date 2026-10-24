//! GPU buffers.

use std::fmt;
use std::ops::{BitOr, BitOrAssign};
use std::sync::Arc;

use ash::vk;

use crate::debug::set_object_name;
use crate::device::DeviceShared;
use crate::error::VkResultExt as _;
use crate::garbage::Garbage;
use crate::memory::{self, MappedPtr, MemoryLocation};
use crate::pod::{Pod, cast_slice, cast_slice_mut};
use crate::{Error, Result};

/// How a buffer will be used. Combine flags with `|`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BufferUsage(u32);

impl BufferUsage {
    /// Vertex (or per-instance) attributes.
    pub const VERTEX: Self = Self(1 << 0);
    /// Index data.
    pub const INDEX: Self = Self(1 << 1);
    /// Uniform data.
    pub const UNIFORM: Self = Self(1 << 2);
    /// Shader storage.
    pub const STORAGE: Self = Self(1 << 3);
    /// Source of copy commands.
    pub const TRANSFER_SRC: Self = Self(1 << 4);
    /// Destination of copy commands (e.g. readback).
    pub const TRANSFER_DST: Self = Self(1 << 5);

    const NAMES: [(Self, &'static str); 6] = [
        (Self::VERTEX, "VERTEX"),
        (Self::INDEX, "INDEX"),
        (Self::UNIFORM, "UNIFORM"),
        (Self::STORAGE, "STORAGE"),
        (Self::TRANSFER_SRC, "TRANSFER_SRC"),
        (Self::TRANSFER_DST, "TRANSFER_DST"),
    ];

    /// No usage.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// `true` if every flag of `other` is set.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// `true` if no flag is set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub(crate) fn to_vk(self) -> vk::BufferUsageFlags {
        use vk::BufferUsageFlags as F;
        let table = [
            (Self::VERTEX, F::VERTEX_BUFFER),
            (Self::INDEX, F::INDEX_BUFFER),
            (Self::UNIFORM, F::UNIFORM_BUFFER),
            (Self::STORAGE, F::STORAGE_BUFFER),
            (Self::TRANSFER_SRC, F::TRANSFER_SRC),
            (Self::TRANSFER_DST, F::TRANSFER_DST),
        ];
        table
            .into_iter()
            .filter(|(flag, _)| self.contains(*flag))
            .fold(F::empty(), |acc, (_, vk)| acc | vk)
    }
}

impl BitOr for BufferUsage {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for BufferUsage {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl fmt::Debug for BufferUsage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<_> = Self::NAMES
            .iter()
            .filter(|(flag, _)| self.contains(*flag))
            .map(|(_, n)| *n)
            .collect();
        write!(f, "BufferUsage({})", names.join(" | "))
    }
}

/// Parameters of [`crate::Device::create_buffer`].
#[derive(Clone, Copy, Debug)]
pub struct BufferDesc<'a> {
    /// Debug name shown by validation layers and GPU debuggers.
    pub label: &'a str,
    /// Size in bytes (non-zero).
    pub size: u64,
    /// Intended usage.
    pub usage: BufferUsage,
    /// Memory placement.
    pub location: MemoryLocation,
}

/// A linear GPU buffer with its own memory.
///
/// Dropping a buffer is always safe: destruction is deferred until the GPU work that may
/// reference it has completed.
pub struct Buffer {
    device: Arc<DeviceShared>,
    raw: vk::Buffer,
    memory: vk::DeviceMemory,
    mapped: Option<MappedPtr>,
    size: u64,
    usage: BufferUsage,
}

impl Buffer {
    pub(crate) fn new(device: &Arc<DeviceShared>, desc: &BufferDesc<'_>) -> Result<Self> {
        if desc.size == 0 {
            return Err(Error::InvalidArgument(format!(
                "buffer {:?} has size 0",
                desc.label
            )));
        }
        if desc.usage.is_empty() {
            return Err(Error::InvalidArgument(format!(
                "buffer {:?} has no usage",
                desc.label
            )));
        }
        let info = vk::BufferCreateInfo::default()
            .size(desc.size)
            .usage(desc.usage.to_vk())
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        // SAFETY: valid device and create info.
        let raw = unsafe { device.raw.create_buffer(&info, None) }.ctx("vkCreateBuffer")?;
        // SAFETY: `raw` is a live buffer of this device.
        let requirements = unsafe { device.raw.get_buffer_memory_requirements(raw) };

        let allocation = memory::allocate(device, requirements, desc.location)
            // SAFETY: `raw` is unused and destroyed exactly once on this error path.
            .inspect_err(|_| unsafe { device.raw.destroy_buffer(raw, None) })?;
        // SAFETY: memory satisfies the buffer's requirements and is bound once at offset 0.
        if let Err(result) = unsafe { device.raw.bind_buffer_memory(raw, allocation.memory, 0) } {
            // SAFETY: both objects are unused and destroyed exactly once on this error path.
            unsafe {
                device.raw.destroy_buffer(raw, None);
                device.raw.free_memory(allocation.memory, None);
            }
            return Err(Error::from_vk("vkBindBufferMemory", result));
        }
        set_object_name(device, raw, desc.label);

        Ok(Self {
            device: Arc::clone(device),
            raw,
            memory: allocation.memory,
            mapped: allocation.mapped,
            size: desc.size,
            usage: desc.usage,
        })
    }

    /// Size in bytes.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Usage flags.
    pub fn usage(&self) -> BufferUsage {
        self.usage
    }

    /// The raw buffer handle.
    pub fn handle(&self) -> vk::Buffer {
        self.raw
    }

    /// Copies `data` into the buffer at byte `offset`.
    ///
    /// The buffer must be host-visible ([`MemoryLocation::CpuToGpu`] or
    /// [`MemoryLocation::GpuToCpu`]). The GPU must not be reading the written range: stream
    /// per-frame data through one buffer per frame in flight ([`crate::FRAMES_IN_FLIGHT`]).
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`] if the buffer is not host-visible or the range is
    /// out of bounds.
    pub fn write<T: Pod>(&mut self, offset: u64, data: &[T]) -> Result<()> {
        let bytes = cast_slice(data);
        let start = self.host_range(offset, bytes.len())?;
        // SAFETY: `host_range` checked that the range lies inside the mapped allocation; the
        // source is a distinct CPU slice; `&mut self` excludes concurrent CPU access.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), start, bytes.len()) };
        Ok(())
    }

    /// Copies bytes from the buffer at `offset` into `out`.
    ///
    /// Read only after the GPU work writing the buffer has completed (e.g. after
    /// [`crate::Device::submit_and_wait`]).
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`] if the buffer is not host-visible or the range is
    /// out of bounds.
    pub fn read<T: Pod>(&self, offset: u64, out: &mut [T]) -> Result<()> {
        let bytes = cast_slice_mut(out);
        let start = self.host_range(offset, bytes.len())?;
        // SAFETY: range checked by `host_range`; `T: Pod` accepts any bit pattern; the
        // destination is a distinct CPU slice.
        unsafe {
            std::ptr::copy_nonoverlapping(start.cast_const(), bytes.as_mut_ptr(), bytes.len());
        }
        Ok(())
    }

    /// Validates a host access and returns a pointer to its first byte.
    fn host_range(&self, offset: u64, len: usize) -> Result<*mut u8> {
        let mapped = self
            .mapped
            .ok_or_else(|| Error::InvalidArgument("buffer is not host-visible".into()))?;
        let len_u64 = u64::try_from(len).map_err(|_| Error::InvalidArgument("length".into()))?;
        let in_bounds = offset
            .checked_add(len_u64)
            .is_some_and(|end| end <= self.size);
        if !in_bounds {
            return Err(Error::InvalidArgument(format!(
                "range {offset}..{} exceeds buffer size {}",
                offset.saturating_add(len_u64),
                self.size
            )));
        }
        let offset =
            usize::try_from(offset).map_err(|_| Error::InvalidArgument("offset".into()))?;
        // SAFETY: `offset + len <= size` and the whole allocation (at least `size` bytes) is
        // mapped, so the result stays inside the same mapping.
        Ok(unsafe { mapped.as_ptr().add(offset) })
    }
}

impl fmt::Debug for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Buffer")
            .field("handle", &self.raw)
            .field("size", &self.size)
            .field("usage", &self.usage)
            .field("host_visible", &self.mapped.is_some())
            .finish_non_exhaustive()
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.device.defer_destroy(Garbage::Buffer {
            buffer: self.raw,
            memory: self.memory,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_flags_combine_and_map_to_vulkan() {
        let usage = BufferUsage::VERTEX | BufferUsage::TRANSFER_DST;
        assert!(usage.contains(BufferUsage::VERTEX));
        assert!(!usage.contains(BufferUsage::INDEX));
        assert_eq!(
            usage.to_vk(),
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST
        );
        assert_eq!(format!("{usage:?}"), "BufferUsage(VERTEX | TRANSFER_DST)");
        assert!(BufferUsage::empty().is_empty());
    }
}
