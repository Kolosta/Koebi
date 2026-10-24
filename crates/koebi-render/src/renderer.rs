//! GPU side of the 2D renderer.

use koebi_core::Extent;
use koebi_gpu::{
    BlendMode, Buffer, BufferDesc, BufferUsage, ColorTargetDesc, CommandEncoder, Device, Error,
    FRAMES_IN_FLIGHT, LoadOp, MemoryLocation, Pod, PrimitiveTopology, RenderPass, RenderPipeline,
    RenderPipelineDesc, RenderTarget, Result, ShaderStage, TextureFormat, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexStepMode,
};

use crate::scene::{RectInstance, Scene};
use crate::shaders;

/// Initial size of a per-frame instance buffer (256 rectangles).
const MIN_INSTANCE_BUFFER_BYTES: u64 = 256 * RectInstance::STRIDE as u64;

/// The render target's size and the logical→physical scale factor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    /// Target size in physical pixels.
    pub extent: Extent,
    /// Physical pixels per logical pixel (e.g. `2.0` on a "Retina" display).
    pub scale_factor: f32,
}

/// Push constants of every 2D pipeline. Must match `ViewportConstants` in the shaders.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct ViewportConstants {
    viewport_px: [f32; 2],
    scale_factor: f32,
    padding: f32,
}

// SAFETY: `repr(C)` with four `f32`: 16 bytes, no padding, any bit pattern is valid.
unsafe impl Pod for ViewportConstants {}
const _: () = assert!(size_of::<ViewportConstants>() == 16);

impl ViewportConstants {
    fn new(viewport: Viewport) -> Self {
        Self {
            viewport_px: [viewport.extent.width as f32, viewport.extent.height as f32],
            scale_factor: viewport.scale_factor,
            padding: 0.0,
        }
    }
}

/// Draws [`Scene`]s. Create one per target format; reuse it every frame.
#[derive(Debug)]
pub struct Renderer2D {
    device: Device,
    rect_pipeline: RenderPipeline,
    /// One streaming buffer per frame in flight: the CPU writes slot N while the GPU reads
    /// the others.
    instance_buffers: Vec<Option<Buffer>>,
    target_format: TextureFormat,
}

impl Renderer2D {
    /// Compiles the pipelines for targets of `target_format`.
    ///
    /// # Errors
    ///
    /// Fails if shader modules or pipelines cannot be created.
    pub fn new(device: &Device, target_format: TextureFormat) -> Result<Self> {
        let vertex = device.create_shader_module("rect.vert", shaders::RECT_VERT)?;
        let fragment = device.create_shader_module("rect.frag", shaders::RECT_FRAG)?;
        let attributes = [0_u32, 1, 2, 3].map(|location| VertexAttribute {
            location,
            format: VertexFormat::Float32x4,
            offset: location * 16,
        });
        let rect_pipeline = device.create_render_pipeline(&RenderPipelineDesc {
            label: "rect",
            vertex: ShaderStage {
                module: &vertex,
                entry_point: c"main",
            },
            fragment: ShaderStage {
                module: &fragment,
                entry_point: c"main",
            },
            vertex_buffers: &[VertexBufferLayout {
                stride: RectInstance::STRIDE,
                step_mode: VertexStepMode::Instance,
                attributes: &attributes,
            }],
            topology: PrimitiveTopology::TriangleStrip,
            color_targets: &[ColorTargetDesc {
                format: target_format,
                blend: BlendMode::PremultipliedAlpha,
            }],
            push_constant_size: 16,
        })?;
        Ok(Self {
            device: device.clone(),
            rect_pipeline,
            instance_buffers: (0..FRAMES_IN_FLIGHT).map(|_| None).collect(),
            target_format,
        })
    }

    /// The target format the pipelines were built for.
    pub fn target_format(&self) -> TextureFormat {
        self.target_format
    }

    /// Uploads the scene's instance data into the streaming buffer of `frame_index`.
    ///
    /// The frame slot must not be in use by the GPU, which [`koebi_gpu::Presenter`]
    /// guarantees for the index it provides. Buffers grow geometrically, so steady-state
    /// frames never allocate.
    ///
    /// # Errors
    ///
    /// Fails if `frame_index >= FRAMES_IN_FLIGHT` or a buffer cannot be created.
    pub fn prepare(&mut self, scene: &Scene, frame_index: usize) -> Result<()> {
        let instances = scene.rect_instances();
        if instances.is_empty() {
            return Ok(());
        }
        let slot = self
            .instance_buffers
            .get_mut(frame_index)
            .ok_or_else(|| Error::InvalidArgument(format!("frame index {frame_index}")))?;
        let needed = u64::try_from(size_of_val(instances))
            .map_err(|_| Error::InvalidArgument("scene too large".into()))?;
        if slot.as_ref().is_none_or(|buffer| buffer.size() < needed) {
            let size = needed.next_power_of_two().max(MIN_INSTANCE_BUFFER_BYTES);
            koebi_core::debug!("growing rect instance buffer {frame_index} to {size} bytes");
            *slot = Some(self.device.create_buffer(&BufferDesc {
                label: "rect instances",
                size,
                usage: BufferUsage::VERTEX,
                location: MemoryLocation::CpuToGpu,
            })?);
        }
        if let Some(buffer) = slot.as_mut() {
            buffer.write(0, instances)?;
        }
        Ok(())
    }

    /// Records the scene's draw calls into an active pass. Call [`Renderer2D::prepare`]
    /// with the same `frame_index` first.
    ///
    /// # Errors
    ///
    /// Fails if the scene was not prepared for this frame slot.
    pub fn draw(
        &self,
        pass: &mut RenderPass<'_>,
        scene: &Scene,
        frame_index: usize,
        viewport: Viewport,
    ) -> Result<()> {
        let count = scene.rect_count();
        if count == 0 {
            return Ok(());
        }
        let buffer = self
            .instance_buffers
            .get(frame_index)
            .and_then(Option::as_ref)
            .filter(|buffer| buffer.size() >= count as u64 * u64::from(RectInstance::STRIDE))
            .ok_or_else(|| {
                Error::InvalidArgument("scene was not prepared for this frame".into())
            })?;
        let instances =
            u32::try_from(count).map_err(|_| Error::InvalidArgument("too many rects".into()))?;
        pass.set_pipeline(&self.rect_pipeline);
        pass.set_push_constants(&self.rect_pipeline, &ViewportConstants::new(viewport))?;
        pass.set_vertex_buffer(0, buffer, 0);
        pass.draw(0..4, 0..instances);
        Ok(())
    }

    /// Prepares `scene`, clears `target` to the scene's clear color and draws the scene.
    ///
    /// # Errors
    ///
    /// See [`Renderer2D::prepare`] and [`Renderer2D::draw`].
    pub fn render(
        &mut self,
        encoder: &mut CommandEncoder<'_>,
        target: RenderTarget,
        scene: &Scene,
        frame_index: usize,
        scale_factor: f32,
    ) -> Result<()> {
        if target.format() != self.target_format {
            return Err(Error::InvalidArgument(format!(
                "target is {:?}, renderer was built for {:?}",
                target.format(),
                self.target_format
            )));
        }
        self.prepare(scene, frame_index)?;
        let clear = LoadOp::Clear(scene.clear_color().premultiplied());
        let mut pass = encoder.begin_render_pass(target, clear);
        self.draw(
            &mut pass,
            scene,
            frame_index,
            Viewport {
                extent: target.extent(),
                scale_factor,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_constants_carry_physical_size_and_scale() {
        let constants = ViewportConstants::new(Viewport {
            extent: Extent::new(1920, 1080),
            scale_factor: 1.5,
        });
        assert_eq!(constants.viewport_px, [1920.0, 1080.0]);
        assert!((constants.scale_factor - 1.5).abs() < f32::EPSILON);
    }
}
