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

    /// The copies of texels `start..end` of a texture `width` texels wide whose texels follow
    /// one another row by row, as a store of records does: the end of a first row, whole rows,
    /// the start of a last row, three regions at most. Appended to `out`, each with where its
    /// texels start in a buffer that holds them one after the other from `offset`, each
    /// `texel_bytes` long.
    pub fn spans(
        start: usize,
        end: usize,
        width: u32,
        mut offset: u64,
        texel_bytes: u64,
        out: &mut Vec<(Self, u64)>,
    ) {
        let row = width as usize;
        let mut push = |at: usize, len: usize, rows: usize| {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "texel coordinates of a texture, below 2^32"
            )]
            let (x, y) = ((at % row) as u32, (at / row) as u32);
            #[allow(
                clippy::cast_possible_truncation,
                reason = "a length within one row, or a count of rows, below 2^32"
            )]
            let region = if rows == 0 {
                Self::new(x, y, len as u32, 1)
            } else {
                Self::new(0, y, width, rows as u32)
            };
            out.push((region, offset));
            offset += texel_bytes * len as u64;
        };
        if row == 0 {
            return;
        }
        let mut at = start;
        if !at.is_multiple_of(row) && at < end {
            let stop = end.min(at.next_multiple_of(row));
            push(at, stop - at, 0);
            at = stop;
        }
        let whole = end.saturating_sub(at) / row;
        if whole > 0 {
            push(at, whole * row, whole);
            at += whole * row;
        }
        if at < end {
            push(at, end - at, 0);
        }
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
    /// The submission its commands will belong to, fixed when recording starts.
    submission: u64,
}

impl<'a> CommandEncoder<'a> {
    pub(crate) fn new(device: &'a DeviceShared, raw: vk::CommandBuffer) -> Self {
        Self {
            device,
            raw,
            submission: device.pending_submission(),
        }
    }

    /// The raw command buffer (escape hatch).
    pub fn handle(&self) -> vk::CommandBuffer {
        self.raw
    }

    /// The index of the submission the recorded commands will belong to: the device's next
    /// when recording started. Nothing else is submitted while an encoder records, as deferred
    /// destruction assumes too. Once [`crate::Device::completed_submission`] reaches it, the
    /// GPU is done with what these commands read: a buffer they read may be written again.
    pub fn submission(&self) -> u64 {
        self.submission
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
        self.copy_texture_region_to_buffer(texture, TextureRegion::whole(texture.extent()), buffer)
    }

    /// Copies the texels of `texture` in `region` into `buffer`, tightly packed row by row, and makes them visible to the host once the
    /// submission completes: a part of a frame read back, for an eyedropper.
    ///
    /// The texture must be in [`TextureState::TransferSrc`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`] if usages are missing, the rectangle is empty or
    /// lies outside the texture, or the buffer is too small.
    pub fn copy_texture_region_to_buffer(
        &mut self,
        texture: &Texture,
        region: TextureRegion,
        buffer: &Buffer,
    ) -> Result<()> {
        if !texture.usage().contains(TextureUsage::TRANSFER_SRC) {
            return Err(Error::InvalidArgument("copy requires TRANSFER_SRC".into()));
        }
        let size = texture.extent();
        if !region.fits_in(size) {
            return Err(Error::InvalidArgument(format!(
                "region {region:?} lies outside the {size:?} texture"
            )));
        }
        check_image_copy(texture.format(), region, buffer)?;
        self.copy_image_region(texture.handle(), region, buffer);
        Ok(())
    }

    /// Records the copy of `region` of `image` (in `TRANSFER_SRC_OPTIMAL`) into `buffer`, then
    /// the barrier making it visible to the host. The caller checked the copy
    /// ([`check_image_copy`]) and keeps the region inside the image: recording cannot fail.
    pub(crate) fn copy_image_region(
        &mut self,
        image: vk::Image,
        region: TextureRegion,
        buffer: &Buffer,
    ) {
        // `check_image_copy` keeps the origin within `i32`.
        let offset = |value: u32| i32::try_from(value).unwrap_or(i32::MAX);
        let copy = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: vk::Offset3D {
                x: offset(region.x),
                y: offset(region.y),
                z: 0,
            },
            image_extent: vk::Extent3D {
                width: region.width,
                height: region.height,
                depth: 1,
            },
        };
        // SAFETY: recording command buffer; objects belong to this device; the caller keeps
        // the region inside the image, and `check_image_copy` checked the buffer's size and
        // usage.
        unsafe {
            self.device.raw.cmd_copy_image_to_buffer(
                self.raw,
                image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                buffer.handle(),
                &[copy],
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
    }

    /// Copies `region` of `source` to the same place in `destination`, of the same format: what
    /// a texture held, kept on the GPU when a larger one replaces it.
    ///
    /// `source` must be in [`TextureState::TransferSrc`] and `destination` in
    /// [`TextureState::TransferDst`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`] if usages are missing, the formats differ, or the
    /// region is empty or lies outside either texture.
    pub fn copy_texture_to_texture(
        &mut self,
        source: &Texture,
        destination: &Texture,
        region: TextureRegion,
    ) -> Result<()> {
        if !source.usage().contains(TextureUsage::TRANSFER_SRC)
            || !destination.usage().contains(TextureUsage::TRANSFER_DST)
        {
            return Err(Error::InvalidArgument(
                "copy requires TRANSFER_SRC/TRANSFER_DST".into(),
            ));
        }
        if source.format() != destination.format() {
            return Err(Error::InvalidArgument(format!(
                "copy from {:?} to {:?}: the formats differ",
                source.format(),
                destination.format()
            )));
        }
        if region.is_empty()
            || !region.fits_in(source.extent())
            || !region.fits_in(destination.extent())
        {
            return Err(Error::InvalidArgument(format!(
                "region {region:?} is empty or outside a texture of {:?} or {:?}",
                source.extent(),
                destination.extent()
            )));
        }
        let offset = |value: u32| {
            i32::try_from(value).map_err(|_| Error::InvalidArgument("region offset".into()))
        };
        let at = vk::Offset3D {
            x: offset(region.x)?,
            y: offset(region.y)?,
            z: 0,
        };
        let layers = vk::ImageSubresourceLayers {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            mip_level: 0,
            base_array_layer: 0,
            layer_count: 1,
        };
        let copy = vk::ImageCopy {
            src_subresource: layers,
            src_offset: at,
            dst_subresource: layers,
            dst_offset: at,
            extent: vk::Extent3D {
                width: region.width,
                height: region.height,
                depth: 1,
            },
        };
        // SAFETY: recording command buffer; both textures belong to this device and have the
        // usages and the format checked above; the region lies inside both; the caller's
        // barriers put them in the layouts named here.
        unsafe {
            self.device.raw.cmd_copy_image(
                self.raw,
                source.handle(),
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                destination.handle(),
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[copy],
            );
        }
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
        let whole = TextureRegion::whole(target.extent);
        self.begin_rendering(target, load, whole)
    }

    /// Begins a dynamic rendering pass on `target` that renders `area` only: a clear clears it
    /// alone, and draws outside it are discarded. The viewport stays the whole target, so
    /// coordinates do not change; the scissor is preset to `area`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`] if `area` is empty or does not lie inside the target.
    pub fn begin_render_pass_in(
        &mut self,
        target: RenderTarget,
        load: LoadOp,
        area: TextureRegion,
    ) -> Result<RenderPass<'_>> {
        if area.is_empty() || !area.fits_in(target.extent) {
            return Err(Error::InvalidArgument(format!(
                "render area {area:?} outside a target of {:?}",
                target.extent
            )));
        }
        Ok(self.begin_rendering(target, load, area))
    }

    fn begin_rendering(
        &mut self,
        target: RenderTarget,
        load: LoadOp,
        area: TextureRegion,
    ) -> RenderPass<'_> {
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
        #[allow(
            clippy::cast_possible_wrap,
            reason = "an offset inside a target, far below 2^31"
        )]
        let render_area = vk::Rect2D {
            offset: vk::Offset2D {
                x: area.x as i32,
                y: area.y as i32,
            },
            extent: vk::Extent2D {
                width: area.width,
                height: area.height,
            },
        };
        let info = vk::RenderingInfo::default()
            .render_area(render_area)
            .layer_count(1)
            .color_attachments(&attachments);
        // SAFETY: recording command buffer; the target view belongs to this device and is in
        // COLOR_ATTACHMENT_OPTIMAL layout (caller's barrier); the area lies inside it.
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
        #[allow(
            clippy::cast_possible_wrap,
            reason = "an offset inside a target, far below 2^31"
        )]
        pass.set_scissor(area.x as i32, area.y as i32, area.width, area.height);
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

/// Checks a copy of `region` of an image of `format` into `buffer`, tightly packed: the region
/// holds texels, its origin fits Vulkan's signed offsets, and the buffer can receive it and is
/// large enough. Whether the region lies inside the image is the caller's to check.
///
/// # Errors
///
/// Returns [`Error::InvalidArgument`] naming what does not hold.
pub(crate) fn check_image_copy(
    format: TextureFormat,
    region: TextureRegion,
    buffer: &Buffer,
) -> Result<()> {
    if region.is_empty() {
        return Err(Error::InvalidArgument("empty copy region".into()));
    }
    if i32::try_from(region.x).is_err() || i32::try_from(region.y).is_err() {
        return Err(Error::InvalidArgument(format!(
            "copy origin {region:?} too large"
        )));
    }
    let needed =
        u64::from(region.width) * u64::from(region.height) * u64::from(format.bytes_per_pixel());
    if buffer.size() < needed {
        return Err(Error::InvalidArgument(format!(
            "readback buffer holds {} bytes, the region needs {needed}",
            buffer.size()
        )));
    }
    if !buffer.usage().contains(BufferUsage::TRANSFER_DST) {
        return Err(Error::InvalidArgument("copy requires TRANSFER_DST".into()));
    }
    Ok(())
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

    fn spans(start: usize, end: usize, width: u32) -> Vec<(u32, u32, u32, u32, u64)> {
        let mut out = Vec::new();
        TextureRegion::spans(start, end, width, 16, 16, &mut out);
        out.into_iter()
            .map(|(r, offset)| (r.x, r.y, r.width, r.height, offset))
            .collect()
    }

    #[test]
    fn a_span_of_texels_copies_in_three_regions_at_most() {
        let w = 4096;
        let width = w as usize;
        // Inside one row.
        assert_eq!(spans(10, 30, w), [(10, 0, 20, 1, 16)]);
        // The end of a row, whole rows, the start of another; offsets follow one another.
        let texels = |n: u32| 16 + 16 * u64::from(n);
        assert_eq!(
            spans(width - 6, 3 * width + 5, w),
            [
                (w - 6, 0, 6, 1, 16),
                (0, 1, w, 2, texels(6)),
                (0, 3, 5, 1, texels(6 + 2 * w)),
            ]
        );
        // Whole rows only, and nothing at all.
        assert_eq!(spans(width, 3 * width, w), [(0, 1, w, 2, 16)]);
        assert_eq!(spans(40, 40, w), []);
        assert_eq!(spans(0, 10, 0), []);
    }

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
