//! The logical device: queue, function tables, submissions and deferred destruction.

use std::ffi::{CStr, c_char};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use ash::vk;

use crate::adapter::Adapter;
use crate::buffer::{Buffer, BufferDesc};
use crate::command::CommandEncoder;
use crate::error::VkResultExt as _;
use crate::garbage::{Garbage, GarbageQueue};
use crate::instance::Instance;
use crate::pipeline::{RenderPipeline, RenderPipelineDesc};
use crate::shader::ShaderModule;
use crate::texture::{Texture, TextureDesc};
use crate::{Error, Result};

const PORTABILITY_SUBSET: &CStr = c"VK_KHR_portability_subset";

/// A logical GPU device. Cheap to clone (reference counted).
///
/// Every resource keeps its device alive, and the device keeps its instance alive, so Rust's
/// drop order can never destroy a parent before its children.
#[derive(Clone)]
pub struct Device {
    pub(crate) shared: Arc<DeviceShared>,
}

/// Command pool, command buffer and fence for blocking one-shot submissions.
struct Immediate {
    pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    fence: vk::Fence,
}

pub(crate) struct DeviceShared {
    pub(crate) raw: ash::Device,
    pub(crate) adapter: Adapter,
    pub(crate) memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub(crate) swapchain: Option<ash::khr::swapchain::Device>,
    pub(crate) debug_utils: Option<ash::ext::debug_utils::Device>,
    /// `None` when dynamic rendering is core (Vulkan 1.3).
    rendering_khr: Option<ash::khr::dynamic_rendering::Device>,
    /// `None` when synchronization2 is core (Vulkan 1.3).
    sync2_khr: Option<ash::khr::synchronization2::Device>,
    /// `vkQueueSubmit`/`vkQueuePresentKHR` require external synchronization of the queue.
    queue: Mutex<vk::Queue>,
    immediate: Mutex<Immediate>,
    garbage: Mutex<GarbageQueue>,
    last_submitted: AtomicU64,
    last_completed: AtomicU64,
    // Declared last: the instance must outlive the device (fields drop in order).
    pub(crate) instance: Instance,
}

/// Locks a mutex, recovering the data if another thread panicked while holding it.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Device {
    /// Creates a logical device with one graphics (and present) queue.
    ///
    /// # Errors
    ///
    /// Fails if `vkCreateDevice` or the creation of the internal command objects fails.
    pub fn new(instance: &Instance, adapter: &Adapter) -> Result<Self> {
        let vk_instance = &instance.shared.raw;

        let priorities = [1.0_f32];
        let queue_infos = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(adapter.queue_family)
            .queue_priorities(&priorities)];

        // VK_KHR_swapchain requires the instance-level VK_KHR_surface: headless instances
        // (tests, tools) have neither.
        let swapchain = adapter.has_swapchain && instance.shared.surface.is_some();
        let mut extensions: Vec<*const c_char> = Vec::new();
        if swapchain {
            extensions.push(ash::khr::swapchain::NAME.as_ptr());
        }
        if adapter.has_portability_subset {
            extensions.push(PORTABILITY_SUBSET.as_ptr());
        }
        if !adapter.core_1_3 {
            extensions.push(ash::khr::dynamic_rendering::NAME.as_ptr());
            extensions.push(ash::khr::synchronization2::NAME.as_ptr());
        }

        let mut dynamic_rendering =
            vk::PhysicalDeviceDynamicRenderingFeatures::default().dynamic_rendering(true);
        let mut synchronization2 =
            vk::PhysicalDeviceSynchronization2Features::default().synchronization2(true);
        let create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_infos)
            .enabled_extension_names(&extensions)
            .push_next(&mut dynamic_rendering)
            .push_next(&mut synchronization2);

        // SAFETY: the adapter was enumerated from this instance and probed for every
        // extension and feature requested here.
        let raw = unsafe { vk_instance.create_device(adapter.raw, &create_info, None) }
            .ctx("vkCreateDevice")?;

        let immediate = match Immediate::new(&raw, adapter.queue_family) {
            Ok(immediate) => immediate,
            Err(error) => {
                // SAFETY: the device has no child objects yet.
                unsafe { raw.destroy_device(None) };
                return Err(error);
            }
        };

        // SAFETY: queue 0 of this family was requested in `queue_infos`.
        let queue = unsafe { raw.get_device_queue(adapter.queue_family, 0) };
        // SAFETY: valid physical device of this instance.
        let memory_properties =
            unsafe { vk_instance.get_physical_device_memory_properties(adapter.raw) };

        let shared = DeviceShared {
            rendering_khr: (!adapter.core_1_3)
                .then(|| ash::khr::dynamic_rendering::Device::new(vk_instance, &raw)),
            sync2_khr: (!adapter.core_1_3)
                .then(|| ash::khr::synchronization2::Device::new(vk_instance, &raw)),
            swapchain: swapchain.then(|| ash::khr::swapchain::Device::new(vk_instance, &raw)),
            debug_utils: instance
                .shared
                .debug_utils
                .is_some()
                .then(|| ash::ext::debug_utils::Device::new(vk_instance, &raw)),
            raw,
            adapter: adapter.clone(),
            memory_properties,
            queue: Mutex::new(queue),
            immediate: Mutex::new(immediate),
            garbage: Mutex::new(GarbageQueue::default()),
            last_submitted: AtomicU64::new(0),
            last_completed: AtomicU64::new(0),
            instance: instance.clone(),
        };

        let info = adapter.info();
        koebi_core::info!(
            "GPU device: {} ({:?}, Vulkan {}, driver {} {}){}",
            info.name,
            info.kind,
            crate::error::version_string(info.api_version),
            info.driver_name,
            info.driver_info,
            if adapter.core_1_3 {
                ""
            } else {
                " via KHR extensions"
            }
        );
        Ok(Self {
            shared: Arc::new(shared),
        })
    }

    /// The adapter this device was created on.
    pub fn adapter(&self) -> &Adapter {
        &self.shared.adapter
    }

    /// The instance this device belongs to.
    pub fn instance(&self) -> &Instance {
        &self.shared.instance
    }

    /// The raw device handle (escape hatch).
    pub fn handle(&self) -> vk::Device {
        self.shared.raw.handle()
    }

    /// Creates a buffer. See [`BufferDesc`].
    ///
    /// # Errors
    ///
    /// Fails on invalid descriptions or when memory is exhausted.
    pub fn create_buffer(&self, desc: &BufferDesc<'_>) -> Result<Buffer> {
        Buffer::new(&self.shared, desc)
    }

    /// Creates a 2D texture with a default view. See [`TextureDesc`].
    ///
    /// # Errors
    ///
    /// Fails on invalid descriptions or when memory is exhausted.
    pub fn create_texture(&self, desc: &TextureDesc<'_>) -> Result<Texture> {
        Texture::new(&self.shared, desc)
    }

    /// Creates a shader module from SPIR-V words.
    ///
    /// # Errors
    ///
    /// Fails if the code is not SPIR-V or the driver rejects it.
    pub fn create_shader_module(&self, label: &str, spirv: &[u32]) -> Result<ShaderModule> {
        ShaderModule::new(&self.shared, label, spirv)
    }

    /// Creates a graphics pipeline for dynamic rendering.
    ///
    /// # Errors
    ///
    /// Fails on invalid descriptions or when pipeline compilation fails.
    pub fn create_render_pipeline(&self, desc: &RenderPipelineDesc<'_>) -> Result<RenderPipeline> {
        RenderPipeline::new(&self.shared, desc)
    }

    /// Records commands with `record`, submits them and **blocks** until the GPU finished.
    ///
    /// Meant for uploads, readbacks, tests and tools; never call it per frame. Do not call it
    /// recursively from `record`.
    ///
    /// # Errors
    ///
    /// Fails if recording, submission or the wait fails.
    pub fn submit_and_wait<R>(
        &self,
        record: impl FnOnce(&mut CommandEncoder<'_>) -> R,
    ) -> Result<R> {
        let shared = &*self.shared;
        let immediate = lock(&shared.immediate);
        let raw = &shared.raw;
        // SAFETY: the pool is idle: the previous immediate submission was waited on.
        unsafe { raw.reset_command_pool(immediate.pool, vk::CommandPoolResetFlags::empty()) }
            .ctx("vkResetCommandPool")?;
        let begin = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        // SAFETY: the command buffer belongs to the reset pool.
        unsafe { raw.begin_command_buffer(immediate.command_buffer, &begin) }
            .ctx("vkBeginCommandBuffer")?;

        let mut encoder = CommandEncoder::new(shared, immediate.command_buffer);
        let output = record(&mut encoder);

        // SAFETY: recording started above; the encoder borrow has ended.
        unsafe { raw.end_command_buffer(immediate.command_buffer) }.ctx("vkEndCommandBuffer")?;
        // SAFETY: the fence is not used by any pending submission (we always wait on it).
        unsafe { raw.reset_fences(&[immediate.fence]) }.ctx("vkResetFences")?;
        let command_buffers =
            [vk::CommandBufferSubmitInfo::default().command_buffer(immediate.command_buffer)];
        let submits = [vk::SubmitInfo2::default().command_buffer_infos(&command_buffers)];
        let index = shared.submit(&submits, immediate.fence)?;
        // SAFETY: the fence was just submitted.
        unsafe { raw.wait_for_fences(&[immediate.fence], true, u64::MAX) }
            .ctx("vkWaitForFences")?;
        shared.mark_completed(index);
        shared.collect_garbage();
        Ok(output)
    }

    /// Blocks until the GPU is idle, then releases all deferred garbage.
    ///
    /// Use only at teardown or around rare events (swapchain recreation), never per frame.
    ///
    /// # Errors
    ///
    /// Fails if the device is lost.
    pub fn wait_idle(&self) -> Result<()> {
        self.shared.wait_idle()
    }
}

impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Device")
            .field("adapter", &self.shared.adapter.info().name)
            .field("handle", &self.handle())
            .finish_non_exhaustive()
    }
}

impl Immediate {
    fn new(device: &ash::Device, queue_family: u32) -> Result<Self> {
        let pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::TRANSIENT)
            .queue_family_index(queue_family);
        // SAFETY: valid device and queue family.
        let pool =
            unsafe { device.create_command_pool(&pool_info, None) }.ctx("vkCreateCommandPool")?;
        let allocate = vk::CommandBufferAllocateInfo::default()
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let destroy_pool = || {
            // SAFETY: nothing was submitted; destroying the pool frees its command buffers.
            unsafe { device.destroy_command_pool(pool, None) };
        };
        // SAFETY: valid pool of this device.
        let command_buffer = match unsafe { device.allocate_command_buffers(&allocate) } {
            Ok(buffers) if !buffers.is_empty() => buffers[0],
            Ok(_) => {
                destroy_pool();
                return Err(Error::from_vk(
                    "vkAllocateCommandBuffers",
                    vk::Result::ERROR_UNKNOWN,
                ));
            }
            Err(result) => {
                destroy_pool();
                return Err(Error::from_vk("vkAllocateCommandBuffers", result));
            }
        };
        // SAFETY: valid device.
        match unsafe { device.create_fence(&vk::FenceCreateInfo::default(), None) } {
            Ok(fence) => Ok(Self {
                pool,
                command_buffer,
                fence,
            }),
            Err(result) => {
                destroy_pool();
                Err(Error::from_vk("vkCreateFence", result))
            }
        }
    }
}

impl DeviceShared {
    /// Submits work and returns its submission index.
    pub(crate) fn submit(&self, submits: &[vk::SubmitInfo2<'_>], fence: vk::Fence) -> Result<u64> {
        let queue = lock(&self.queue);
        let index = self.last_submitted.fetch_add(1, Ordering::AcqRel) + 1;
        // SAFETY: the queue is externally synchronized by the mutex; the caller provides valid
        // submit infos whose objects belong to this device.
        let result = unsafe {
            match &self.sync2_khr {
                Some(khr) => khr.queue_submit2(*queue, submits, fence),
                None => self.raw.queue_submit2(*queue, submits, fence),
            }
        };
        result.ctx("vkQueueSubmit2")?;
        Ok(index)
    }

    /// Presents a swapchain image. Returns `Ok(true)` when the swapchain is suboptimal.
    pub(crate) fn present(
        &self,
        loader: &ash::khr::swapchain::Device,
        info: &vk::PresentInfoKHR<'_>,
    ) -> ash::prelude::VkResult<bool> {
        let queue = lock(&self.queue);
        // SAFETY: the queue is externally synchronized by the mutex; `info` is valid.
        unsafe { loader.queue_present(*queue, info) }
    }

    /// Records that every submission up to `index` has completed.
    ///
    /// A fence signal covers all work submitted earlier on the same queue, so this is exact.
    pub(crate) fn mark_completed(&self, index: u64) {
        self.last_completed.fetch_max(index, Ordering::AcqRel);
    }

    /// Queues an object for destruction once the GPU can no longer use it.
    pub(crate) fn defer_destroy(&self, garbage: Garbage) {
        // Work recorded right now will be submitted with index `last_submitted + 1`.
        let safe_after = self.last_submitted.load(Ordering::Acquire) + 1;
        lock(&self.garbage).push(safe_after, garbage);
    }

    /// Destroys every object whose last submission has completed.
    pub(crate) fn collect_garbage(&self) {
        let completed = self.last_completed.load(Ordering::Acquire);
        let ready = lock(&self.garbage).pop_completed(completed);
        for garbage in ready {
            // SAFETY: the submission that could reference the object has completed.
            unsafe { garbage.destroy(&self.raw) };
        }
    }

    pub(crate) fn wait_idle(&self) -> Result<()> {
        {
            // Holding the queue lock makes the wait cover every submission made so far.
            let _queue = lock(&self.queue);
            // SAFETY: valid device; queue access is synchronized by the lock.
            unsafe { self.raw.device_wait_idle() }.ctx("vkDeviceWaitIdle")?;
        }
        self.mark_completed(self.last_submitted.load(Ordering::Acquire));
        self.collect_garbage();
        Ok(())
    }

    // --- Vulkan 1.3 core / KHR extension dispatch ------------------------------------------

    /// # Safety
    /// `command_buffer` must be recording and `info` valid for this device.
    pub(crate) unsafe fn cmd_begin_rendering(
        &self,
        command_buffer: vk::CommandBuffer,
        info: &vk::RenderingInfo<'_>,
    ) {
        // SAFETY: forwarded from the caller; both paths call the same promoted function.
        unsafe {
            match &self.rendering_khr {
                Some(khr) => khr.cmd_begin_rendering(command_buffer, info),
                None => self.raw.cmd_begin_rendering(command_buffer, info),
            }
        }
    }

    /// # Safety
    /// `command_buffer` must be inside a dynamic rendering pass begun on this device.
    pub(crate) unsafe fn cmd_end_rendering(&self, command_buffer: vk::CommandBuffer) {
        // SAFETY: forwarded from the caller.
        unsafe {
            match &self.rendering_khr {
                Some(khr) => khr.cmd_end_rendering(command_buffer),
                None => self.raw.cmd_end_rendering(command_buffer),
            }
        }
    }

    /// # Safety
    /// `command_buffer` must be recording, `pool` a timestamp query pool of this device, and
    /// `query` reset since its last use.
    pub(crate) unsafe fn cmd_write_timestamp2(
        &self,
        command_buffer: vk::CommandBuffer,
        stage: vk::PipelineStageFlags2,
        pool: vk::QueryPool,
        query: u32,
    ) {
        // SAFETY: forwarded from the caller.
        unsafe {
            match &self.sync2_khr {
                Some(khr) => khr.cmd_write_timestamp2(command_buffer, stage, pool, query),
                None => self
                    .raw
                    .cmd_write_timestamp2(command_buffer, stage, pool, query),
            }
        }
    }

    /// # Safety
    /// `command_buffer` must be recording and `info` valid for this device.
    pub(crate) unsafe fn cmd_pipeline_barrier2(
        &self,
        command_buffer: vk::CommandBuffer,
        info: &vk::DependencyInfo<'_>,
    ) {
        // SAFETY: forwarded from the caller.
        unsafe {
            match &self.sync2_khr {
                Some(khr) => khr.cmd_pipeline_barrier2(command_buffer, info),
                None => self.raw.cmd_pipeline_barrier2(command_buffer, info),
            }
        }
    }
}

impl Drop for DeviceShared {
    fn drop(&mut self) {
        if let Err(error) = self.wait_idle() {
            koebi_core::error!("device teardown: {error}");
        }
        let remaining = lock(&self.garbage).drain_all();
        let immediate = lock(&self.immediate);
        // SAFETY: the device is idle; every child object is destroyed exactly once, before
        // the device itself. Resources hold an `Arc` to this struct, so none is still alive.
        unsafe {
            for garbage in remaining {
                garbage.destroy(&self.raw);
            }
            self.raw.destroy_fence(immediate.fence, None);
            self.raw.destroy_command_pool(immediate.pool, None);
            self.raw.destroy_device(None);
        }
    }
}
