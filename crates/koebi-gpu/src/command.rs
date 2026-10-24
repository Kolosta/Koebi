//! Command recording: barriers, copies and dynamic rendering passes.

use std::fmt;
use std::marker::PhantomData;
use std::ops::Range;

use ash::vk;
use koebi_core::Extent;

use crate::buffer::{Buffer, BufferUsage};
use crate::device::DeviceShared;
use crate::format::TextureFormat;
use crate::pipeline::RenderPipeline;
use crate::pod::{Pod, bytes_of};
use crate::texture::{COLOR_RANGE, Texture, TextureUsage};
use crate::{Error, Result};

/// Where a render pass draws: a texture or the current swapchain image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderTarget {
    pub(crate) view: vk::ImageView,
    pub(crate) extent: Extent,
    pub(crate) format: TextureFormat,
}

impl RenderTarget {
    /// Size in physical pixels.
    pub fn extent(&self) -> Extent {
        self.extent
    }

    /// Pixel format; pipelines drawing here must use the same format.
    pub fn format(&self) -> TextureFormat {
        self.format
    }
}

/// What happens to a target's previous contents when a pass begins.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LoadOp {
    /// Clear to this value, written as-is (premultiplied, in the target's encoding).
    Clear([f32; 4]),
    /// Keep the previous contents.
    Load,
    /// Contents are undefined: the pass overwrites every pixel.
    DontCare,
}

/// Usage state of a texture, for [`CommandEncoder::texture_barrier`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureState {
    /// Contents discarded. Use as the `from` state of a fresh texture or swapchain image.
    Undefined,
    /// Rendered into by a pass.
    ColorAttachment,
    /// Read by copy commands.
    TransferSrc,
    /// Written by copy commands.
    TransferDst,
    /// Sampled by fragment shaders.
    ShaderRead,
    /// Handed to the presentation engine.
    Present,
}

impl TextureState {
    /// Stage, access and layout of this state (synchronization2).
    fn sync(self) -> (vk::PipelineStageFlags2, vk::AccessFlags2, vk::ImageLayout) {
        use vk::{AccessFlags2 as A, ImageLayout as L, PipelineStageFlags2 as S};
        match self {
            // Waiting on COLOR_ATTACHMENT_OUTPUT chains with the swapchain acquire semaphore,
            // which is waited on at that stage.
            Self::Undefined => (S::COLOR_ATTACHMENT_OUTPUT, A::NONE, L::UNDEFINED),
            Self::ColorAttachment => (
                S::COLOR_ATTACHMENT_OUTPUT,
                A::COLOR_ATTACHMENT_READ | A::COLOR_ATTACHMENT_WRITE,
                L::COLOR_ATTACHMENT_OPTIMAL,
            ),
            Self::TransferSrc => (S::COPY, A::TRANSFER_READ, L::TRANSFER_SRC_OPTIMAL),
            Self::TransferDst => (S::COPY, A::TRANSFER_WRITE, L::TRANSFER_DST_OPTIMAL),
            Self::ShaderRead => (
                S::FRAGMENT_SHADER,
                A::SHADER_SAMPLED_READ,
                L::SHADER_READ_ONLY_OPTIMAL,
            ),
            Self::Present => (S::NONE, A::NONE, L::PRESENT_SRC_KHR),
        }
    }
}

/// Records commands into a command buffer.
///
/// Obtained from [`crate::Device::submit_and_wait`] or [`crate::Presenter::render`].
pub struct CommandEncoder<'a> {
    device: &'a DeviceShared,
    raw: vk::CommandBuffer,
}

impl<'a> CommandEncoder<'a> {
    pub(crate) fn new(device: &'a DeviceShared, raw: vk::CommandBuffer) -> Self {
        Self { device, raw }
    }

    /// The raw command buffer (escape hatch).
    pub fn handle(&self) -> vk::CommandBuffer {
        self.raw
    }

    /// Transitions `texture` between usage states.
    pub fn texture_barrier(&mut self, texture: &Texture, from: TextureState, to: TextureState) {
        self.image_barrier(texture.handle(), from, to);
    }

    pub(crate) fn image_barrier(&mut self, image: vk::Image, from: TextureState, to: TextureState) {
        let (src_stage, src_access, old_layout) = from.sync();
        let (dst_stage, dst_access, new_layout) = to.sync();
        let barriers = [vk::ImageMemoryBarrier2::default()
            .src_stage_mask(src_stage)
            .src_access_mask(src_access)
            .dst_stage_mask(dst_stage)
            .dst_access_mask(dst_access)
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(COLOR_RANGE)];
        let dependency = vk::DependencyInfo::default().image_memory_barriers(&barriers);
        // SAFETY: the command buffer is recording; the image belongs to this device.
        unsafe { self.device.cmd_pipeline_barrier2(self.raw, &dependency) };
    }

    /// Copies the whole texture into `buffer`, tightly packed, and makes the data visible to
    /// the host once the submission completes.
    ///
    /// The texture must be in [`TextureState::TransferSrc`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`] if usages are missing or the buffer is too small.
    pub fn copy_texture_to_buffer(&mut self, texture: &Texture, buffer: &Buffer) -> Result<()> {
        let extent = texture.extent();
        let needed = extent.area() * u64::from(texture.format().bytes_per_pixel());
        if buffer.size() < needed {
            return Err(Error::InvalidArgument(format!(
                "readback buffer holds {} bytes, texture needs {needed}",
                buffer.size()
            )));
        }
        if !texture.usage().contains(TextureUsage::TRANSFER_SRC)
            || !buffer.usage().contains(BufferUsage::TRANSFER_DST)
        {
            return Err(Error::InvalidArgument(
                "copy requires TRANSFER_SRC/TRANSFER_DST".into(),
            ));
        }
        let region = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: vk::Offset3D::default(),
            image_extent: vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            },
        };
        // SAFETY: recording command buffer; objects belong to this device; sizes checked.
        unsafe {
            self.device.raw.cmd_copy_image_to_buffer(
                self.raw,
                texture.handle(),
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                buffer.handle(),
                &[region],
            );
        }
        // A fence wait alone does not make device writes visible to the host.
        let barriers = [vk::MemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::COPY)
            .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::HOST)
            .dst_access_mask(vk::AccessFlags2::HOST_READ)];
        let dependency = vk::DependencyInfo::default().memory_barriers(&barriers);
        // SAFETY: recording command buffer.
        unsafe { self.device.cmd_pipeline_barrier2(self.raw, &dependency) };
        Ok(())
    }

    /// Begins a dynamic rendering pass on `target`. The pass ends when the returned value is
    /// dropped. Viewport and scissor are preset to the whole target.
    pub fn begin_render_pass(&mut self, target: RenderTarget, load: LoadOp) -> RenderPass<'_> {
        let (load_op, clear) = match load {
            LoadOp::Clear(color) => (vk::AttachmentLoadOp::CLEAR, color),
            LoadOp::Load => (vk::AttachmentLoadOp::LOAD, [0.0; 4]),
            LoadOp::DontCare => (vk::AttachmentLoadOp::DONT_CARE, [0.0; 4]),
        };
        let attachments = [vk::RenderingAttachmentInfo::default()
            .image_view(target.view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(load_op)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(vk::ClearValue {
                color: vk::ClearColorValue { float32: clear },
            })];
        let area = vk::Rect2D {
            offset: vk::Offset2D::default(),
            extent: vk::Extent2D {
                width: target.extent.width,
                height: target.extent.height,
            },
        };
        let info = vk::RenderingInfo::default()
            .render_area(area)
            .layer_count(1)
            .color_attachments(&attachments);
        // SAFETY: recording command buffer; the target view belongs to this device and is in
        // COLOR_ATTACHMENT_OPTIMAL layout (caller's barrier).
        unsafe { self.device.cmd_begin_rendering(self.raw, &info) };
        let mut pass = RenderPass {
            device: self.device,
            raw: self.raw,
            _encoder: PhantomData,
        };
        pass.set_viewport(
            0.0,
            0.0,
            target.extent.width as f32,
            target.extent.height as f32,
        );
        pass.set_scissor(0, 0, target.extent.width, target.extent.height);
        pass
    }
}

impl fmt::Debug for CommandEncoder<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CommandEncoder").field(&self.raw).finish()
    }
}

/// An active dynamic rendering pass. Ends when dropped.
///
/// Holds a mutable borrow of its [`CommandEncoder`], so no barrier or copy can be recorded
/// inside the pass by mistake.
pub struct RenderPass<'p> {
    device: &'p DeviceShared,
    raw: vk::CommandBuffer,
    _encoder: PhantomData<&'p mut ()>,
}

impl RenderPass<'_> {
    /// Binds a graphics pipeline.
    pub fn set_pipeline(&mut self, pipeline: &RenderPipeline) {
        // SAFETY: recording inside a pass; the pipeline belongs to this device.
        unsafe {
            self.device.raw.cmd_bind_pipeline(
                self.raw,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.handle(),
            );
        }
    }

    /// Sets the viewport in physical pixels.
    pub fn set_viewport(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let viewport = vk::Viewport {
            x,
            y,
            width,
            height,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        // SAFETY: recording; viewport is dynamic state in every Koebi pipeline.
        unsafe { self.device.raw.cmd_set_viewport(self.raw, 0, &[viewport]) };
    }

    /// Sets the scissor rectangle in physical pixels.
    pub fn set_scissor(&mut self, x: i32, y: i32, width: u32, height: u32) {
        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x, y },
            extent: vk::Extent2D { width, height },
        };
        // SAFETY: recording; scissor is dynamic state in every Koebi pipeline.
        unsafe { self.device.raw.cmd_set_scissor(self.raw, 0, &[scissor]) };
    }

    /// Uploads push constants for `pipeline` (both stages).
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`] if `value` does not match the pipeline's block size.
    pub fn set_push_constants<T: Pod>(
        &mut self,
        pipeline: &RenderPipeline,
        value: &T,
    ) -> Result<()> {
        let bytes = bytes_of(value);
        if u32::try_from(bytes.len()).ok() != Some(pipeline.push_constant_size()) {
            return Err(Error::InvalidArgument(format!(
                "push constants are {} bytes, pipeline expects {}",
                bytes.len(),
                pipeline.push_constant_size()
            )));
        }
        // SAFETY: recording; the range matches the pipeline layout's push constant range.
        unsafe {
            self.device.raw.cmd_push_constants(
                self.raw,
                pipeline.layout(),
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                bytes,
            );
        }
        Ok(())
    }

    /// Binds a vertex buffer to `slot`.
    pub fn set_vertex_buffer(&mut self, slot: u32, buffer: &Buffer, offset: u64) {
        // SAFETY: recording; the buffer belongs to this device. Deferred destruction keeps it
        // alive until this submission completes even if dropped right after.
        unsafe {
            self.device
                .raw
                .cmd_bind_vertex_buffers(self.raw, slot, &[buffer.handle()], &[offset]);
        }
    }

    /// Draws `vertices` for each of `instances`.
    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        let vertex_count = vertices.end.saturating_sub(vertices.start);
        let instance_count = instances.end.saturating_sub(instances.start);
        if vertex_count == 0 || instance_count == 0 {
            return;
        }
        // SAFETY: recording inside a pass with a pipeline bound by the caller.
        unsafe {
            self.device.raw.cmd_draw(
                self.raw,
                vertex_count,
                instance_count,
                vertices.start,
                instances.start,
            );
        }
    }
}

impl fmt::Debug for RenderPass<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RenderPass").field(&self.raw).finish()
    }
}

impl Drop for RenderPass<'_> {
    fn drop(&mut self) {
        // SAFETY: `begin_render_pass` began exactly one rendering scope on this buffer.
        unsafe { self.device.cmd_end_rendering(self.raw) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undefined_waits_on_color_output_to_chain_with_acquire() {
        let (stage, access, layout) = TextureState::Undefined.sync();
        assert_eq!(stage, vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT);
        assert_eq!(access, vk::AccessFlags2::NONE);
        assert_eq!(layout, vk::ImageLayout::UNDEFINED);
    }

    #[test]
    fn each_state_has_its_optimal_layout() {
        assert_eq!(
            TextureState::ColorAttachment.sync().2,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
        );
        assert_eq!(
            TextureState::TransferSrc.sync().2,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL
        );
        assert_eq!(
            TextureState::Present.sync().2,
            vk::ImageLayout::PRESENT_SRC_KHR
        );
    }
}
