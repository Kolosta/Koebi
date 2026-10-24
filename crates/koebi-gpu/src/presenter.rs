//! Frame pacing and presentation to a window surface.
//!
//! The [`Presenter`] owns the surface, the swapchain and the per-frame synchronization
//! objects. Each call to [`Presenter::render`] runs one frame:
//!
//! 1. wait for the frame slot's fence (the GPU finished the frame recorded
//!    [`FRAMES_IN_FLIGHT`] frames ago) and release deferred garbage;
//! 2. acquire a swapchain image (`image_available[slot]` semaphore);
//! 3. record: barrier `Undefined → ColorAttachment`, user commands, barrier → `Present`;
//! 4. submit, signalling `render_finished[image]` and the slot's fence;
//! 5. present, waiting on `render_finished[image]`.
//!
//! `render_finished` is indexed by swapchain image (not frame slot): a semaphore is reused
//! only when its image is acquired again, which guarantees the previous present consumed it.

use std::fmt;

use ash::vk;
use koebi_core::Extent;

use crate::command::{CommandEncoder, RenderTarget, TextureState};
use crate::device::Device;
use crate::error::VkResultExt as _;
use crate::format::TextureFormat;
use crate::surface::Surface;
use crate::swapchain::{PresentMode, Swapchain};
use crate::{Error, FRAMES_IN_FLIGHT, Result};

/// Parameters of [`Presenter::new`].
#[derive(Clone, Copy, Debug)]
pub struct PresenterDesc {
    /// Window size in physical pixels (used when the surface does not dictate it).
    pub extent: Extent,
    /// Presentation pacing.
    pub present_mode: PresentMode,
}

/// The target of the frame being recorded.
#[derive(Clone, Copy, Debug)]
pub struct FrameTarget {
    target: RenderTarget,
    frame_index: usize,
}

impl FrameTarget {
    /// The swapchain image to render into.
    pub fn render_target(&self) -> RenderTarget {
        self.target
    }

    /// Frame slot in `0..FRAMES_IN_FLIGHT`: index per-frame resources (e.g. streaming
    /// buffers) with it.
    pub fn frame_index(&self) -> usize {
        self.frame_index
    }
}

/// Result of [`Presenter::render`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderOutcome {
    /// A frame was submitted and queued for presentation.
    Presented,
    /// No frame this time (window minimized or swapchain being recreated).
    Skipped,
}

struct FrameSync {
    pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    image_available: vk::Semaphore,
    in_flight: vk::Fence,
    submission: u64,
}

/// Presents frames to a window surface.
pub struct Presenter {
    swapchain: Option<Swapchain>,
    render_finished: Vec<vk::Semaphore>,
    frames: Vec<FrameSync>,
    frame_index: usize,
    desired_extent: Extent,
    present_mode: PresentMode,
    needs_recreate: bool,
    frames_presented: u64,
    // Dropped after `Drop::drop` destroyed the swapchain, then the device reference.
    surface: Surface,
    device: Device,
}

impl Presenter {
    /// Creates the presenter and its first swapchain.
    ///
    /// # Errors
    ///
    /// Fails if synchronization objects or the swapchain cannot be created.
    pub fn new(device: &Device, surface: Surface, desc: &PresenterDesc) -> Result<Self> {
        let mut presenter = Self {
            swapchain: None,
            render_finished: Vec::new(),
            frames: Vec::with_capacity(FRAMES_IN_FLIGHT),
            frame_index: 0,
            desired_extent: desc.extent,
            present_mode: desc.present_mode,
            needs_recreate: true,
            frames_presented: 0,
            surface,
            device: device.clone(),
        };
        for _ in 0..FRAMES_IN_FLIGHT {
            let frame = FrameSync::new(&presenter.device)?;
            presenter.frames.push(frame);
        }
        presenter.recreate()?;
        Ok(presenter)
    }

    /// Requests a swapchain resize, applied at the next frame.
    pub fn resize(&mut self, extent: Extent) {
        if extent != self.desired_extent {
            self.desired_extent = extent;
            self.needs_recreate = true;
        }
    }

    /// Changes the presentation pacing, applied at the next frame.
    pub fn set_present_mode(&mut self, mode: PresentMode) {
        if mode != self.present_mode {
            self.present_mode = mode;
            self.needs_recreate = true;
        }
    }

    /// Format of the swapchain images, once a swapchain exists.
    pub fn format(&self) -> Option<TextureFormat> {
        self.swapchain.as_ref().map(|s| s.format)
    }

    /// Size of the swapchain images, once a swapchain exists.
    pub fn extent(&self) -> Option<Extent> {
        self.swapchain.as_ref().map(|s| s.extent)
    }

    /// Number of frames presented so far.
    pub fn frames_presented(&self) -> u64 {
        self.frames_presented
    }

    /// Records and presents one frame. `record` receives an encoder whose target is already
    /// in [`TextureState::ColorAttachment`]; it must leave it there.
    ///
    /// # Errors
    ///
    /// Returns [`Error::SurfaceLost`] when the surface must be recreated (mobile background),
    /// [`Error::DeviceLost`] on GPU loss, or other Vulkan failures.
    pub fn render(
        &mut self,
        record: impl FnOnce(&mut CommandEncoder<'_>, &FrameTarget),
    ) -> Result<RenderOutcome> {
        if (self.needs_recreate || self.swapchain.is_none()) && !self.recreate()? {
            return Ok(RenderOutcome::Skipped);
        }
        let Some(swapchain) = self.swapchain.as_ref() else {
            return Ok(RenderOutcome::Skipped);
        };
        let device = &*self.device.shared;
        let loader = device
            .swapchain
            .as_ref()
            .ok_or_else(|| Error::MissingExtension("VK_KHR_swapchain".into()))?;
        let frame = &mut self.frames[self.frame_index];

        // SAFETY: the fence belongs to this device (created signaled).
        unsafe {
            device
                .raw
                .wait_for_fences(&[frame.in_flight], true, u64::MAX)
        }
        .ctx("vkWaitForFences")?;
        device.mark_completed(frame.submission);
        device.collect_garbage();

        // SAFETY: valid swapchain; the semaphore is unsignaled (its last wait completed).
        let acquired = unsafe {
            loader.acquire_next_image(
                swapchain.raw,
                u64::MAX,
                frame.image_available,
                vk::Fence::null(),
            )
        };
        let (image_index, suboptimal) = match acquired {
            Ok(acquired) => acquired,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.needs_recreate = true;
                return Ok(RenderOutcome::Skipped);
            }
            Err(result) => return Err(Error::from_vk("vkAcquireNextImageKHR", result)),
        };
        if suboptimal {
            self.needs_recreate = true;
        }
        let image_slot = image_index as usize;

        // Reset only after a successful acquire, so an early return never leaves an
        // unsignaled fence that the next frame would wait on forever.
        // SAFETY: the fence is signaled and not used by pending work.
        unsafe { device.raw.reset_fences(&[frame.in_flight]) }.ctx("vkResetFences")?;
        // SAFETY: the pool's previous submission completed (fence waited above).
        unsafe {
            device
                .raw
                .reset_command_pool(frame.pool, vk::CommandPoolResetFlags::empty())
        }
        .ctx("vkResetCommandPool")?;
        let begin = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        // SAFETY: the command buffer belongs to the reset pool.
        unsafe {
            device
                .raw
                .begin_command_buffer(frame.command_buffer, &begin)
        }
        .ctx("vkBeginCommandBuffer")?;

        let image = swapchain.images[image_slot];
        let target = FrameTarget {
            target: RenderTarget {
                view: swapchain.views[image_slot],
                extent: swapchain.extent,
                format: swapchain.format,
            },
            frame_index: self.frame_index,
        };
        let mut encoder = CommandEncoder::new(device, frame.command_buffer);
        encoder.image_barrier(
            image,
            TextureState::Undefined,
            TextureState::ColorAttachment,
        );
        record(&mut encoder, &target);
        encoder.image_barrier(image, TextureState::ColorAttachment, TextureState::Present);

        // SAFETY: recording started above.
        unsafe { device.raw.end_command_buffer(frame.command_buffer) }.ctx("vkEndCommandBuffer")?;
        let render_finished = self.render_finished[image_slot];
        let waits = [vk::SemaphoreSubmitInfo::default()
            .semaphore(frame.image_available)
            .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)];
        let signals = [vk::SemaphoreSubmitInfo::default()
            .semaphore(render_finished)
            .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)];
        let command_buffers =
            [vk::CommandBufferSubmitInfo::default().command_buffer(frame.command_buffer)];
        let submits = [vk::SubmitInfo2::default()
            .wait_semaphore_infos(&waits)
            .command_buffer_infos(&command_buffers)
            .signal_semaphore_infos(&signals)];
        frame.submission = device.submit(&submits, frame.in_flight)?;

        let wait_semaphores = [render_finished];
        let swapchains = [swapchain.raw];
        let image_indices = [image_index];
        let present = vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);
        match device.present(loader, &present) {
            Ok(false) => {}
            Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => self.needs_recreate = true,
            Err(result) => return Err(Error::from_vk("vkQueuePresentKHR", result)),
        }

        self.frame_index = (self.frame_index + 1) % FRAMES_IN_FLIGHT;
        self.frames_presented += 1;
        Ok(RenderOutcome::Presented)
    }

    /// Rebuilds the swapchain. Returns `false` while the surface has a zero size.
    fn recreate(&mut self) -> Result<bool> {
        // Rare event (resize): a full wait keeps old images and semaphores trivially safe.
        self.device.wait_idle()?;
        let shared = &*self.device.shared;
        let old = self.swapchain.take();
        let old_raw = old.as_ref().map_or(vk::SwapchainKHR::null(), |s| s.raw);
        let created = Swapchain::new(
            shared,
            &self.surface,
            self.desired_extent,
            self.present_mode,
            old_raw,
        );
        if let Some(mut old) = old {
            // SAFETY: the device is idle; the old swapchain was retired by the creation above.
            unsafe { old.destroy(shared) };
        }
        for semaphore in self.render_finished.drain(..) {
            // SAFETY: the device is idle, so no submission or present uses the semaphore.
            unsafe { shared.raw.destroy_semaphore(semaphore, None) };
        }
        self.swapchain = created?;
        if let Some(swapchain) = &self.swapchain {
            for _ in &swapchain.images {
                // SAFETY: valid device.
                let semaphore = unsafe {
                    shared
                        .raw
                        .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                }
                .ctx("vkCreateSemaphore")?;
                self.render_finished.push(semaphore);
            }
            self.needs_recreate = false;
        }
        Ok(self.swapchain.is_some())
    }
}

impl fmt::Debug for Presenter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Presenter")
            .field("extent", &self.extent())
            .field("format", &self.format())
            .field("present_mode", &self.present_mode)
            .field("frames_presented", &self.frames_presented)
            .finish_non_exhaustive()
    }
}

impl Drop for Presenter {
    fn drop(&mut self) {
        if let Err(error) = self.device.wait_idle() {
            koebi_core::error!("presenter teardown: {error}");
        }
        let shared = &*self.device.shared;
        // SAFETY: the device is idle; every object below belongs to it and is destroyed once.
        unsafe {
            for frame in self.frames.drain(..) {
                shared.raw.destroy_fence(frame.in_flight, None);
                shared.raw.destroy_semaphore(frame.image_available, None);
                shared.raw.destroy_command_pool(frame.pool, None);
            }
            for semaphore in self.render_finished.drain(..) {
                shared.raw.destroy_semaphore(semaphore, None);
            }
            if let Some(mut swapchain) = self.swapchain.take() {
                swapchain.destroy(shared);
            }
        }
    }
}

impl FrameSync {
    fn new(device: &Device) -> Result<Self> {
        let raw = &device.shared.raw;
        let pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::TRANSIENT)
            .queue_family_index(device.shared.adapter.queue_family);
        // SAFETY: valid device and queue family.
        let pool =
            unsafe { raw.create_command_pool(&pool_info, None) }.ctx("vkCreateCommandPool")?;
        let mut frame = Self {
            pool,
            command_buffer: vk::CommandBuffer::null(),
            image_available: vk::Semaphore::null(),
            in_flight: vk::Fence::null(),
            submission: 0,
        };
        let result = (|| {
            let allocate = vk::CommandBufferAllocateInfo::default()
                .command_pool(pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);
            // SAFETY: valid pool.
            let buffers = unsafe { raw.allocate_command_buffers(&allocate) }
                .ctx("vkAllocateCommandBuffers")?;
            frame.command_buffer = buffers.first().copied().unwrap_or_default();
            // SAFETY: valid device.
            frame.image_available =
                unsafe { raw.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }
                    .ctx("vkCreateSemaphore")?;
            let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
            // SAFETY: valid device.
            frame.in_flight =
                unsafe { raw.create_fence(&fence_info, None) }.ctx("vkCreateFence")?;
            Ok::<(), Error>(())
        })();
        if let Err(error) = result {
            // SAFETY: nothing was submitted; null handles are ignored by destroy functions.
            unsafe {
                raw.destroy_semaphore(frame.image_available, None);
                raw.destroy_command_pool(pool, None);
            }
            return Err(error);
        }
        Ok(frame)
    }
}
