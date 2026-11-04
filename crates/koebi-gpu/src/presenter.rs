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
//! **Reading frames back** ([`Presenter::request_pixels`], for an eyedropper): when a square
//! of the frame is asked for, step 3 copies it into the slot's readback buffer between the
//! user commands and the barrier to `Present` (`ColorAttachment → TransferSrc → Present`).
//! The copy is read when the slot comes around again, after its fence: [`FRAMES_IN_FLIGHT`]
//! frames later, never waiting. Everything that can fail (the buffer's allocation, the checks
//! of the copy) happens before the fence is reset: a failure drops the request and the frame
//! goes on. Swapchains whose images cannot be copied, or whose format is not four 8-bit
//! channels, read nothing back ([`Presenter::can_read_pixels`]).
//!
//! `render_finished` is indexed by swapchain image (not frame slot): a semaphore is reused
//! only when its image is acquired again, which guarantees the previous present consumed it.
//!
//! When the queue supports it, two timestamps surround the frame's commands. They are read
//! when the slot comes around again, after its fence: measuring never waits.

use std::fmt;
use std::time::Duration;

use ash::vk;
use koebi_core::Extent;

use crate::buffer::{Buffer, BufferDesc, BufferUsage};
use crate::command::{CommandEncoder, RenderTarget, TextureRegion, TextureState, check_image_copy};
use crate::device::Device;
use crate::error::VkResultExt as _;
use crate::format::TextureFormat;
use crate::memory::MemoryLocation;
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

/// Pixels of a presented frame, read back: what an eyedropper picks from. Gamma-encoded RGBA,
/// as the frame showed them, row by row from the top left.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FramePixels {
    /// The top left pixel's place in the frame, in physical pixels.
    pub origin: (u32, u32),
    /// Width and height, in pixels.
    pub extent: Extent,
    /// The pixels.
    pub pixels: Vec<[u8; 4]>,
}

impl FramePixels {
    /// The pixel at `(x, y)` of the frame, if the read-back square holds it.
    pub fn at(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        let column = x.checked_sub(self.origin.0)?;
        let row = y.checked_sub(self.origin.1)?;
        if column >= self.extent.width || row >= self.extent.height {
            return None;
        }
        let index =
            usize::try_from(u64::from(row) * u64::from(self.extent.width) + u64::from(column))
                .ok()?;
        self.pixels.get(index).copied()
    }
}

/// What a frame slot's submission copied of its frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Copied {
    region: TextureRegion,
    /// The swapchain's format when it copied: a recreation may change it before the read.
    format: TextureFormat,
}

/// What a frame slot copies of its frame: a buffer, and what its last submission copied into
/// it, read once the submission completed.
#[derive(Default)]
struct Readback {
    buffer: Option<Buffer>,
    copied: Option<Copied>,
}

struct FrameSync {
    pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    image_available: vk::Semaphore,
    in_flight: vk::Fence,
    submission: u64,
    /// Two timestamps around the frame's commands; null without timestamp support.
    timestamps: vk::QueryPool,
    /// Whether the slot's last submission wrote the timestamps.
    timed: bool,
}

/// Presents frames to a window surface.
pub struct Presenter {
    swapchain: Option<Swapchain>,
    render_finished: Vec<vk::Semaphore>,
    frames: Vec<FrameSync>,
    /// One per frame slot.
    readbacks: Vec<Readback>,
    /// The square the next frame copies: its center and side, in physical pixels.
    requested: Option<((u32, u32), u32)>,
    /// The pixels last read back, their vector reused from one read to the next, and whether
    /// they were read since last taken.
    pixels: FramePixels,
    fresh: bool,
    frame_index: usize,
    desired_extent: Extent,
    present_mode: PresentMode,
    needs_recreate: bool,
    frames_presented: u64,
    gpu_frame_time: Option<Duration>,
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
            readbacks: (0..FRAMES_IN_FLIGHT).map(|_| Readback::default()).collect(),
            requested: None,
            pixels: FramePixels::default(),
            fresh: false,
            frame_index: 0,
            desired_extent: desc.extent,
            present_mode: desc.present_mode,
            needs_recreate: true,
            frames_presented: 0,
            gpu_frame_time: None,
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

    /// How long the GPU took to execute the commands of the latest measured frame, from its
    /// first to its last command, without the wait for the swapchain image.
    ///
    /// Frames are measured with timestamp queries read [`FRAMES_IN_FLIGHT`] frames later,
    /// once their fence has signaled, so measuring never stalls. `None` before the first
    /// measured frame, when the queue has no timestamps
    /// ([`crate::Adapter::supports_timestamps`]), or when its timestamps never advance (as on
    /// the iOS simulator).
    pub fn gpu_frame_time(&self) -> Option<Duration> {
        self.gpu_frame_time
    }

    /// Whether frames can be read back ([`Self::request_pixels`]): the swapchain's images can
    /// be copied and hold four 8-bit channels. `false` before the first swapchain.
    pub fn can_read_pixels(&self) -> bool {
        self.swapchain.as_ref().is_some_and(readable)
    }

    /// Asks the next frame to copy the square of `side` physical pixels centered on `center`,
    /// kept inside the frame: it is read once the GPU finished that frame, frames later and
    /// without waiting ([`Self::take_pixels_into`]). Nothing comes back when frames cannot be
    /// read ([`Self::can_read_pixels`]), or when the copy cannot be prepared (it is logged).
    pub fn request_pixels(&mut self, center: (u32, u32), side: u32) {
        self.requested = Some((center, side.max(1)));
    }

    /// The pixels read back since the last call, swapped into `into`: the presenter reuses the
    /// vector it gets back for the next read, so that reading back allocates nothing once both
    /// have grown. Returns `false`, leaving `into` as it was, when nothing new was read.
    pub fn take_pixels_into(&mut self, into: &mut FramePixels) -> bool {
        if !self.fresh {
            return false;
        }
        self.fresh = false;
        core::mem::swap(&mut self.pixels, into);
        true
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
        // The slot's submission completed: what it copied can be read.
        let readback = &mut self.readbacks[self.frame_index];
        if let (Some(copied), Some(buffer)) = (readback.copied.take(), &readback.buffer) {
            match read_pixels(buffer, copied, &mut self.pixels) {
                Ok(()) => self.fresh = true,
                Err(error) => koebi_core::warn!("cannot read the frame's pixels back: {error}"),
            }
        }
        if frame.timed {
            frame.timed = false;
            if let Some(timestamps) = device.adapter.timestamps {
                let mut ticks = [0_u64; 2];
                // SAFETY: both queries were written by the submission whose fence was just
                // waited on, so their results are available without waiting.
                let read = unsafe {
                    device.raw.get_query_pool_results(
                        frame.timestamps,
                        0,
                        &mut ticks,
                        vk::QueryResultFlags::TYPE_64,
                    )
                };
                let elapsed = timestamps.elapsed(ticks[0], ticks[1]);
                // Some drivers (MoltenVK on the iOS simulator) announce timestamps that never
                // advance: a frame cannot take no time, so such a frame counts as unmeasured.
                if read.is_ok() && !elapsed.is_zero() {
                    self.gpu_frame_time = Some(elapsed);
                }
            }
        }

        // The square asked for, prepared before the fence is reset: a failure (no memory for
        // the buffer) only drops the request, and recording the copy cannot fail.
        let copy = self
            .requested
            .take()
            .filter(|_| readable(swapchain))
            .and_then(|(center, side)| square(center, side, swapchain.extent))
            .and_then(|region| {
                prepare_readback(&self.device, readback, swapchain.format, region)
                    .inspect_err(|error| {
                        koebi_core::warn!("cannot read the frame's pixels back: {error}");
                    })
                    .ok()
            });

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
        let timed = frame.timestamps != vk::QueryPool::null();
        if timed {
            // SAFETY: recording, outside any rendering; the pool belongs to this device and
            // its previous results were read above.
            unsafe {
                device
                    .raw
                    .cmd_reset_query_pool(frame.command_buffer, frame.timestamps, 0, 2);
            }
        }
        let mut encoder = CommandEncoder::new(device, frame.command_buffer);
        encoder.image_barrier(
            image,
            TextureState::Undefined,
            TextureState::ColorAttachment,
        );
        // Written once every earlier command completed, the acquire wait included.
        let stamp = |query| {
            if timed {
                // SAFETY: recording; the query was reset above and is written once.
                unsafe {
                    device.cmd_write_timestamp2(
                        frame.command_buffer,
                        vk::PipelineStageFlags2::ALL_COMMANDS,
                        frame.timestamps,
                        query,
                    );
                }
            }
        };
        stamp(0);
        record(&mut encoder, &target);
        stamp(1);
        // The square asked for, copied before presenting: prepared and checked above.
        match copy.zip(readback.buffer.as_ref()) {
            Some((region, buffer)) => {
                encoder.image_barrier(
                    image,
                    TextureState::ColorAttachment,
                    TextureState::TransferSrc,
                );
                encoder.copy_image_region(image, region, buffer);
                encoder.image_barrier(image, TextureState::TransferSrc, TextureState::Present);
                readback.copied = Some(Copied {
                    region,
                    format: swapchain.format,
                });
            }
            None => {
                encoder.image_barrier(image, TextureState::ColorAttachment, TextureState::Present);
            }
        }

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
        frame.timed = timed;

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
                shared.raw.destroy_query_pool(frame.timestamps, None);
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

/// Whether the frames of `swapchain` can be read back: its images can be copied, and their
/// format has four 8-bit channels, which [`read_pixels`] decodes.
fn readable(swapchain: &Swapchain) -> bool {
    swapchain.readable && four_bytes(swapchain.format)
}

/// Whether `format` has four 8-bit channels, in RGBA or BGRA order.
const fn four_bytes(format: TextureFormat) -> bool {
    matches!(
        format,
        TextureFormat::Rgba8Unorm
            | TextureFormat::Rgba8Srgb
            | TextureFormat::Bgra8Unorm
            | TextureFormat::Bgra8Srgb
    )
}

/// Makes sure `readback`'s buffer holds `region` of a frame of `format`, then checks the copy,
/// so that recording it cannot fail. Returns the region.
fn prepare_readback(
    device: &Device,
    readback: &mut Readback,
    format: TextureFormat,
    region: TextureRegion,
) -> Result<TextureRegion> {
    let needed =
        u64::from(region.width) * u64::from(region.height) * u64::from(format.bytes_per_pixel());
    if readback
        .buffer
        .as_ref()
        .is_none_or(|buffer| buffer.size() < needed)
    {
        // The slot's fence was waited: its last submission no longer uses the old buffer, which
        // goes through the deferred-deletion queue anyway.
        readback.buffer = Some(device.create_buffer(&BufferDesc {
            label: "frame readback",
            size: needed,
            usage: BufferUsage::TRANSFER_DST,
            location: MemoryLocation::GpuToCpu,
        })?);
    }
    let buffer = readback
        .buffer
        .as_ref()
        .ok_or_else(|| Error::InvalidArgument("no readback buffer".into()))?;
    check_image_copy(format, region, buffer)?;
    Ok(region)
}

/// Reads what a frame slot copied from `buffer` into `into`, reusing its vector, in RGBA order.
fn read_pixels(buffer: &Buffer, copied: Copied, into: &mut FramePixels) -> Result<()> {
    if !four_bytes(copied.format) {
        return Err(Error::InvalidArgument(format!(
            "frames in {:?} are not read back",
            copied.format
        )));
    }
    let region = copied.region;
    let count = usize::try_from(u64::from(region.width) * u64::from(region.height))
        .map_err(|_| Error::InvalidArgument(format!("region {region:?} too large")))?;
    into.pixels.clear();
    into.pixels.resize(count, [0; 4]);
    buffer.read(0, &mut into.pixels)?;
    if matches!(
        copied.format,
        TextureFormat::Bgra8Unorm | TextureFormat::Bgra8Srgb
    ) {
        for pixel in &mut into.pixels {
            pixel.swap(0, 2);
        }
    }
    into.origin = (region.x, region.y);
    into.extent = Extent::new(region.width, region.height);
    Ok(())
}

/// The square of `side` pixels centered on `center`, moved inside a frame of `extent` (and no
/// larger). `None` for an empty frame.
fn square(center: (u32, u32), side: u32, extent: Extent) -> Option<TextureRegion> {
    let width = side.min(extent.width);
    let height = side.min(extent.height);
    if width == 0 || height == 0 {
        return None;
    }
    let start =
        |center: u32, length: u32, room: u32| center.saturating_sub(length / 2).min(room - length);
    Some(TextureRegion::new(
        start(center.0, width, extent.width),
        start(center.1, height, extent.height),
        width,
        height,
    ))
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
            timestamps: vk::QueryPool::null(),
            timed: false,
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
            if device.shared.adapter.timestamps.is_some() {
                let info = vk::QueryPoolCreateInfo::default()
                    .query_type(vk::QueryType::TIMESTAMP)
                    .query_count(2);
                // SAFETY: valid device; its graphics queue supports timestamps.
                frame.timestamps =
                    unsafe { raw.create_query_pool(&info, None) }.ctx("vkCreateQueryPool")?;
            }
            Ok::<(), Error>(())
        })();
        if let Err(error) = result {
            // SAFETY: nothing was submitted; null handles are ignored by destroy functions.
            unsafe {
                raw.destroy_fence(frame.in_flight, None);
                raw.destroy_semaphore(frame.image_available, None);
                raw.destroy_command_pool(pool, None);
            }
            return Err(error);
        }
        Ok(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squares_stay_inside_the_frame() {
        let frame = Extent::new(100, 80);
        assert_eq!(
            square((50, 40), 9, frame),
            Some(TextureRegion::new(46, 36, 9, 9))
        );
        // At a corner: moved inside.
        assert_eq!(
            square((0, 0), 9, frame),
            Some(TextureRegion::new(0, 0, 9, 9))
        );
        assert_eq!(
            square((99, 79), 9, frame),
            Some(TextureRegion::new(91, 71, 9, 9))
        );
        // Larger than the frame: the frame.
        assert_eq!(
            square((10, 10), 500, frame),
            Some(TextureRegion::whole(frame))
        );
        assert_eq!(square((0, 0), 9, Extent::new(0, 10)), None);
    }

    #[test]
    fn only_formats_of_four_bytes_are_read_back() {
        for format in [
            TextureFormat::Rgba8Unorm,
            TextureFormat::Rgba8Srgb,
            TextureFormat::Bgra8Unorm,
            TextureFormat::Bgra8Srgb,
        ] {
            assert!(four_bytes(format), "{format:?}");
            assert_eq!(format.bytes_per_pixel(), 4);
        }
        assert!(!four_bytes(TextureFormat::Rgba16Float));
        assert!(!four_bytes(TextureFormat::R8Unorm));
    }

    #[test]
    fn read_back_pixels_answer_for_their_place_in_the_frame() {
        let pixels = FramePixels {
            origin: (10, 20),
            extent: Extent::new(2, 2),
            pixels: vec![
                [1, 0, 0, 255],
                [2, 0, 0, 255],
                [3, 0, 0, 255],
                [4, 0, 0, 255],
            ],
        };
        assert_eq!(pixels.at(11, 21), Some([4, 0, 0, 255]));
        assert_eq!(pixels.at(10, 20), Some([1, 0, 0, 255]));
        assert_eq!(pixels.at(12, 20), None);
        assert_eq!(pixels.at(9, 20), None);
    }
}
