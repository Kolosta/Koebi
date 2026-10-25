//! Command recording: barriers, copies and dynamic rendering passes.

use std::fmt;
use std::marker::PhantomData;
use std::ops::Range;
use std::sync::Arc;

use ash::vk;
use koebi_core::Extent;

use crate::binding::BindGroup;
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

/// A rectangle of texels, for copies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextureRegion {
    /// Left column.
    pub x: u32,
    /// Top row.
    pub y: u32,
    /// Width in texels.
    pub width: u32,
    /// Height in texels.
    pub height: u32,
}

impl TextureRegion {
    /// The region of `width` × `height` texels whose top-left texel is (`x`, `y`).
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// The whole of a texture of `extent`.
    pub const fn whole(extent: Extent) -> Self {
        Self::new(0, 0, extent.width, extent.height)
    }

    /// `true` when the region holds no texel.
    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// `true` when the region lies inside a texture of `extent`.
    pub fn fits_in(self, extent: Extent) -> bool {
        let right = self.x.checked_add(self.width);
        let bottom = self.y.checked_add(self.height);
        right.is_some_and(|right| right <= extent.width)
            && bottom.is_some_and(|bottom| bottom <= extent.height)
    }
}

/// Where the texel rows of a copy lie in a buffer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BufferTexels {
    /// Byte offset of the first texel: a multiple of 4 and of the texel size.
    pub offset: u64,
    /// Bytes from the start of one row to the next: a multiple of the texel size, at least
    /// the length of one row of the region.
    pub bytes_per_row: u32,
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
            // Copies and clears write textures, at different stages: cover every transfer.
            Self::TransferDst => (S::ALL_TRANSFER, A::TRANSFER_WRITE, L::TRANSFER_DST_OPTIMAL),
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

    /// Copies texels from `buffer` into `region` of `texture`, leaving the rest of the texture
    /// unchanged: uploads through a staging buffer, whole or partial.
    ///
    /// The texture must be in [`TextureState::TransferDst`]; the buffer must hold the data
    /// before the submission executes (host writes to mapped memory are visible to the GPU at
    /// submission).
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`] if usages are missing, the region is empty or lies
    /// outside the texture, the layout is misaligned, or the buffer is too small.
    pub fn copy_buffer_to_texture(
        &mut self,
        buffer: &Buffer,
        texels: BufferTexels,
        texture: &Texture,
        region: TextureRegion,
    ) -> Result<()> {
        if !buffer.usage().contains(BufferUsage::TRANSFER_SRC)
            || !texture.usage().contains(TextureUsage::TRANSFER_DST)
        {
            return Err(Error::InvalidArgument(
                "copy requires TRANSFER_SRC/TRANSFER_DST".into(),
            ));
        }
        let copy = buffer_image_copy(buffer, texels, texture, region)?;
        // SAFETY: recording command buffer; objects belong to this device; the region and the
        // buffer range were checked.
        unsafe {
            self.device.raw.cmd_copy_buffer_to_image(
                self.raw,
                buffer.handle(),
                texture.handle(),
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[copy],
            );
        }
        Ok(())
    }

    /// Fills the whole texture with `color`, given in the texture's encoding.
    ///
    /// The texture must be in [`TextureState::TransferDst`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`] without [`TextureUsage::TRANSFER_DST`].
    pub fn clear_texture(&mut self, texture: &Texture, color: [f32; 4]) -> Result<()> {
        if !texture.usage().contains(TextureUsage::TRANSFER_DST) {
            return Err(Error::InvalidArgument(
                "clearing a texture requires TRANSFER_DST".into(),
            ));
        }
        let clear = vk::ClearColorValue { float32: color };
        // SAFETY: recording command buffer; the texture belongs to this device and is in
        // TRANSFER_DST_OPTIMAL layout (caller's barrier).
        unsafe {
            self.device.raw.cmd_clear_color_image(
                self.raw,
                texture.handle(),
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &clear,
                &[COLOR_RANGE],
            );
        }
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

    /// Binds `group` as bind group `index` of `pipeline`'s layout, for the draws that follow.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`] if the pipeline has no bind group `index`, or if the
    /// group's layout has other entries than the pipeline's.
    pub fn set_bind_group(
        &mut self,
        pipeline: &RenderPipeline,
        index: u32,
        group: &BindGroup,
    ) -> Result<()> {
        let expected = pipeline.bind_group_layout(index).ok_or_else(|| {
            Error::InvalidArgument(format!("the pipeline has no bind group {index}"))
        })?;
        // Layouts with identical bindings are compatible.
        if expected.entries != group.layout().entries
            || !Arc::ptr_eq(pipeline.device(), &group.layout().device)
        {
            return Err(Error::InvalidArgument(format!(
                "bind group {index} has entries {:?}, the pipeline expects {:?}",
                group.entries(),
                expected.entries
            )));
        }
        // SAFETY: recording inside a pass; the set's layout is compatible with set `index` of
        // the pipeline's layout (checked above), and the group keeps its resources alive.
        unsafe {
            self.device.raw.cmd_bind_descriptor_sets(
                self.raw,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.layout(),
                index,
                &[group.handle()],
                &[],
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

/// Validates a copy between `buffer` and `region` of `texture`, and describes it.
fn buffer_image_copy(
    buffer: &Buffer,
    texels: BufferTexels,
    texture: &Texture,
    region: TextureRegion,
) -> Result<vk::BufferImageCopy> {
    let fail = |message: String| Err(Error::InvalidArgument(message));
    if region.is_empty() || !region.fits_in(texture.extent()) {
        return fail(format!(
            "region {region:?} is empty or outside a texture of {:?}",
            texture.extent()
        ));
    }
    let texel_size = texture.format().bytes_per_pixel();
    let row = u64::from(region.width) * u64::from(texel_size);
    if !texels.offset.is_multiple_of(4) || !texels.offset.is_multiple_of(u64::from(texel_size)) {
        return fail(format!(
            "offset {} is not a multiple of 4 and of the texel size {texel_size}",
            texels.offset
        ));
    }
    if !texels.bytes_per_row.is_multiple_of(texel_size) || u64::from(texels.bytes_per_row) < row {
        return fail(format!(
            "rows of {} bytes cannot hold {} texels of {texel_size} bytes",
            texels.bytes_per_row, region.width
        ));
    }
    let end = u64::from(region.height - 1)
        .checked_mul(u64::from(texels.bytes_per_row))
        .and_then(|rows| rows.checked_add(row))
        .and_then(|length| length.checked_add(texels.offset));
    if end.is_none_or(|end| end > buffer.size()) {
        return fail(format!(
            "the copy reads beyond the {} bytes of the buffer",
            buffer.size()
        ));
    }
    let offset = |value: u32| {
        i32::try_from(value).map_err(|_| Error::InvalidArgument("region offset".into()))
    };
    Ok(vk::BufferImageCopy {
        buffer_offset: texels.offset,
        buffer_row_length: texels.bytes_per_row / texel_size,
        buffer_image_height: 0,
        image_subresource: vk::ImageSubresourceLayers {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            mip_level: 0,
            base_array_layer: 0,
            layer_count: 1,
        },
        image_offset: vk::Offset3D {
            x: offset(region.x)?,
            y: offset(region.y)?,
            z: 0,
        },
        image_extent: vk::Extent3D {
            width: region.width,
            height: region.height,
            depth: 1,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_destinations_wait_for_clears_as_well_as_copies() {
        // Clears run at the CLEAR stage and copies at COPY: a barrier leaving `TransferDst`
        // must wait for both, or synchronization validation reports a write-after-write hazard.
        assert_eq!(
            TextureState::TransferDst.sync().0,
            vk::PipelineStageFlags2::ALL_TRANSFER
        );
    }

    #[test]
    fn regions_fit_inside_their_texture() {
        let extent = Extent::new(8, 4);
        assert!(TextureRegion::whole(extent).fits_in(extent));
        assert!(TextureRegion::new(6, 2, 2, 2).fits_in(extent));
        assert!(!TextureRegion::new(7, 0, 2, 1).fits_in(extent));
        assert!(!TextureRegion::new(0, u32::MAX, 1, 2).fits_in(extent));
        assert!(TextureRegion::new(1, 1, 0, 3).is_empty());
    }

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
