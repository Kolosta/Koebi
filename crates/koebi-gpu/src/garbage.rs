//! Deferred destruction of GPU objects.
//!
//! Dropping a [`crate::Buffer`] while a submitted frame still reads it would be a
//! use-after-free on the GPU. Instead, dropped objects are queued with the index of the
//! submission that may still use them and destroyed once that submission has completed.
//! This makes resource lifetimes safe to manage with ordinary Rust ownership.

use std::collections::VecDeque;

use ash::vk;

/// A Vulkan object waiting for the GPU to finish with it.
#[derive(Debug)]
pub(crate) enum Garbage {
    Buffer {
        buffer: vk::Buffer,
        memory: vk::DeviceMemory,
    },
    Texture {
        image: vk::Image,
        view: vk::ImageView,
        memory: vk::DeviceMemory,
    },
    Pipeline {
        pipeline: vk::Pipeline,
        layout: vk::PipelineLayout,
    },
    Sampler {
        sampler: vk::Sampler,
    },
    BindGroupLayout {
        layout: vk::DescriptorSetLayout,
    },
    /// Destroying the pool frees the bind group's descriptor set.
    BindGroup {
        pool: vk::DescriptorPool,
    },
}

impl Garbage {
    /// Destroys the object.
    ///
    /// # Safety
    ///
    /// The object must belong to `device` and no pending GPU work may reference it.
    pub(crate) unsafe fn destroy(self, device: &ash::Device) {
        // SAFETY: guaranteed by the caller; each handle is destroyed exactly once because
        // `self` is consumed.
        unsafe {
            match self {
                Self::Buffer { buffer, memory } => {
                    device.destroy_buffer(buffer, None);
                    device.free_memory(memory, None);
                }
                Self::Texture {
                    image,
                    view,
                    memory,
                } => {
                    device.destroy_image_view(view, None);
                    device.destroy_image(image, None);
                    device.free_memory(memory, None);
                }
                Self::Pipeline { pipeline, layout } => {
                    device.destroy_pipeline(pipeline, None);
                    device.destroy_pipeline_layout(layout, None);
                }
                Self::Sampler { sampler } => device.destroy_sampler(sampler, None),
                Self::BindGroupLayout { layout } => {
                    device.destroy_descriptor_set_layout(layout, None);
                }
                Self::BindGroup { pool } => device.destroy_descriptor_pool(pool, None),
            }
        }
    }
}

/// FIFO of garbage tagged with the submission index after which it is safe to destroy.
///
/// Tags are pushed in non-decreasing order because submission indices only grow.
#[derive(Debug, Default)]
pub(crate) struct GarbageQueue {
    entries: VecDeque<(u64, Garbage)>,
}

impl GarbageQueue {
    pub(crate) fn push(&mut self, safe_after: u64, garbage: Garbage) {
        debug_assert!(
            self.entries
                .back()
                .is_none_or(|(tag, _)| *tag <= safe_after)
        );
        self.entries.push_back((safe_after, garbage));
    }

    /// Removes and returns everything whose submission has completed.
    pub(crate) fn pop_completed(&mut self, completed: u64) -> Vec<Garbage> {
        let ready = self
            .entries
            .iter()
            .take_while(|(tag, _)| *tag <= completed)
            .count();
        self.entries
            .drain(..ready)
            .map(|(_, garbage)| garbage)
            .collect()
    }

    /// Removes and returns everything (device teardown, after a full wait).
    pub(crate) fn drain_all(&mut self) -> Vec<Garbage> {
        self.entries.drain(..).map(|(_, garbage)| garbage).collect()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use ash::vk::Handle as _;

    use super::*;

    fn buffer(id: u64) -> Garbage {
        Garbage::Buffer {
            buffer: vk::Buffer::from_raw(id),
            memory: vk::DeviceMemory::from_raw(id),
        }
    }

    #[test]
    fn only_completed_submissions_are_released() {
        let mut queue = GarbageQueue::default();
        queue.push(1, buffer(10));
        queue.push(2, buffer(20));
        queue.push(2, buffer(21));
        queue.push(5, buffer(50));

        assert!(queue.pop_completed(0).is_empty());
        assert_eq!(queue.pop_completed(2).len(), 3);
        assert_eq!(queue.len(), 1);
        assert!(queue.pop_completed(4).is_empty());
        assert_eq!(queue.pop_completed(5).len(), 1);
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn drain_all_empties_the_queue() {
        let mut queue = GarbageQueue::default();
        queue.push(7, buffer(1));
        queue.push(9, buffer(2));
        assert_eq!(queue.drain_all().len(), 2);
        assert!(queue.pop_completed(u64::MAX).is_empty());
    }
}
