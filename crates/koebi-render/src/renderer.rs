//! GPU side of the 2D renderer.

use koebi_core::Extent;
use koebi_gpu::{
    BindGroupLayout, BlendMode, Buffer, BufferDesc, BufferUsage, ColorTargetDesc, CommandEncoder,
    Device, Error, FRAMES_IN_FLIGHT, LoadOp, MemoryLocation, Pod, PrimitiveTopology, RenderPass,
    RenderPipeline, RenderPipelineDesc, RenderTarget, Result, ShaderStage, TextureFormat,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};
use koebi_text::Fonts;

use crate::atlas::{GlyphAtlas, GlyphInstance, GlyphKey, Lookup};
use crate::scene::{LineInstance, PrimitiveKind, RectInstance, Scene};
use crate::shaders;

/// Times a frame's glyphs may make room in the atlas (grow, grow, clear) before giving up.
const ATLAS_ATTEMPTS: usize = 4;

/// Instances an instance buffer holds when first created.
const MIN_INSTANCES_PER_BUFFER: u64 = 256;

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

/// What differs between the pipelines of two primitive types.
struct PipelineDesc<'a> {
    label: &'static str,
    vertex: (&'static str, &'static [u32]),
    fragment: (&'static str, &'static [u32]),
    /// Bytes per instance: `vec4` attributes at locations `0..stride / 16`.
    stride: u32,
    /// The bind groups the shaders read.
    bind_group_layouts: &'a [&'a BindGroupLayout],
}

/// The pipeline of one primitive type and its streaming instance buffers.
#[derive(Debug)]
struct PrimitivePipeline {
    label: &'static str,
    pipeline: RenderPipeline,
    stride: u32,
    /// One streaming buffer per frame in flight: the CPU writes slot N while the GPU reads
    /// the others.
    buffers: Vec<Option<Buffer>>,
}

impl PrimitivePipeline {
    fn new(device: &Device, desc: &PipelineDesc<'_>, target_format: TextureFormat) -> Result<Self> {
        let vertex = device.create_shader_module(desc.vertex.0, desc.vertex.1)?;
        let fragment = device.create_shader_module(desc.fragment.0, desc.fragment.1)?;
        let attributes: Vec<_> = (0..desc.stride / 16)
            .map(|location| VertexAttribute {
                location,
                format: VertexFormat::Float32x4,
                offset: location * 16,
            })
            .collect();
        let pipeline = device.create_render_pipeline(&RenderPipelineDesc {
            label: desc.label,
            vertex: ShaderStage {
                module: &vertex,
                entry_point: c"main",
            },
            fragment: ShaderStage {
                module: &fragment,
                entry_point: c"main",
            },
            vertex_buffers: &[VertexBufferLayout {
                stride: desc.stride,
                step_mode: VertexStepMode::Instance,
                attributes: &attributes,
            }],
            topology: PrimitiveTopology::TriangleStrip,
            color_targets: &[ColorTargetDesc {
                format: target_format,
                blend: BlendMode::PremultipliedAlpha,
            }],
            push_constant_size: 16,
            bind_group_layouts: desc.bind_group_layouts,
        })?;
        Ok(Self {
            label: desc.label,
            pipeline,
            stride: desc.stride,
            buffers: (0..FRAMES_IN_FLIGHT).map(|_| None).collect(),
        })
    }

    /// Writes `instances` into the buffer of `frame_index`, growing it geometrically.
    fn upload<T: Pod>(
        &mut self,
        device: &Device,
        instances: &[T],
        frame_index: usize,
    ) -> Result<()> {
        if instances.is_empty() {
            return Ok(());
        }
        let slot = self
            .buffers
            .get_mut(frame_index)
            .ok_or_else(|| Error::InvalidArgument(format!("frame index {frame_index}")))?;
        let needed = u64::try_from(size_of_val(instances))
            .map_err(|_| Error::InvalidArgument("scene too large".into()))?;
        if slot.as_ref().is_none_or(|buffer| buffer.size() < needed) {
            let minimum = MIN_INSTANCES_PER_BUFFER * u64::from(self.stride);
            let size = needed.next_power_of_two().max(minimum);
            koebi_core::debug!(
                "growing {} instance buffer {frame_index} to {size} bytes",
                self.label
            );
            *slot = Some(device.create_buffer(&BufferDesc {
                label: self.label,
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

    /// The buffer of `frame_index`, if it holds at least `count` instances.
    fn prepared(&self, frame_index: usize, count: usize) -> Result<&Buffer> {
        let bytes = u64::try_from(count)
            .ok()
            .and_then(|count| count.checked_mul(u64::from(self.stride)));
        self.buffers
            .get(frame_index)
            .and_then(Option::as_ref)
            .filter(|buffer| bytes.is_some_and(|bytes| buffer.size() >= bytes))
            .ok_or_else(|| Error::InvalidArgument("scene was not prepared for this frame".into()))
    }
}

/// Draws [`Scene`]s. Create one per target format; reuse it every frame.
///
/// Glyphs are rasterized from the fonts given to [`Renderer2D::set_fonts`] and cached in a
/// glyph atlas; without fonts, glyphs draw nothing.
#[derive(Debug)]
pub struct Renderer2D {
    device: Device,
    rects: PrimitivePipeline,
    lines: PrimitivePipeline,
    glyphs: PrimitivePipeline,
    atlas: GlyphAtlas,
    /// This frame's glyph quads, rebuilt by every `prepare`.
    glyph_instances: Vec<GlyphInstance>,
    target_format: TextureFormat,
}

impl Renderer2D {
    /// Compiles the pipelines for targets of `target_format`.
    ///
    /// # Errors
    ///
    /// Fails if shader modules or pipelines cannot be created.
    pub fn new(device: &Device, target_format: TextureFormat) -> Result<Self> {
        let rects = PipelineDesc {
            label: "rect",
            vertex: ("rect.vert", shaders::RECT_VERT),
            fragment: ("rect.frag", shaders::RECT_FRAG),
            stride: RectInstance::STRIDE,
            bind_group_layouts: &[],
        };
        let lines = PipelineDesc {
            label: "line",
            vertex: ("line.vert", shaders::LINE_VERT),
            fragment: ("line.frag", shaders::LINE_FRAG),
            stride: LineInstance::STRIDE,
            bind_group_layouts: &[],
        };
        let atlas = GlyphAtlas::new(device)?;
        let glyphs = PipelineDesc {
            label: "glyph",
            vertex: ("glyph.vert", shaders::GLYPH_VERT),
            fragment: ("glyph.frag", shaders::GLYPH_FRAG),
            stride: GlyphInstance::STRIDE,
            bind_group_layouts: &[atlas.layout()],
        };
        Ok(Self {
            device: device.clone(),
            rects: PrimitivePipeline::new(device, &rects, target_format)?,
            lines: PrimitivePipeline::new(device, &lines, target_format)?,
            glyphs: PrimitivePipeline::new(device, &glyphs, target_format)?,
            atlas,
            glyph_instances: Vec::new(),
            target_format,
        })
    }

    /// Sets the fonts glyphs are rasterized from: the collection the scene's
    /// [`koebi_text::FontId`]s refer to. Cached glyphs are forgotten.
    pub fn set_fonts(&mut self, fonts: Fonts) {
        self.atlas.set_fonts(fonts);
    }

    /// The target format the pipelines were built for.
    pub fn target_format(&self) -> TextureFormat {
        self.target_format
    }

    /// Uploads the scene's instance data into the streaming buffers of `frame_index`, and
    /// records into `encoder` the atlas uploads of glyphs drawn for the first time. Call it
    /// before beginning the render pass, with the scale factor the scene will be drawn at.
    ///
    /// The frame slot must not be in use by the GPU, which [`koebi_gpu::Presenter`]
    /// guarantees for the index it provides. Buffers grow geometrically, so steady-state
    /// frames never allocate.
    ///
    /// # Errors
    ///
    /// Fails if `frame_index >= FRAMES_IN_FLIGHT` or a buffer or texture cannot be created.
    pub fn prepare(
        &mut self,
        encoder: &mut CommandEncoder<'_>,
        scene: &Scene,
        frame_index: usize,
        scale_factor: f32,
    ) -> Result<()> {
        self.rects
            .upload(&self.device, scene.rect_instances(), frame_index)?;
        self.lines
            .upload(&self.device, scene.line_instances(), frame_index)?;
        if scene.glyph_count() == 0 {
            return Ok(());
        }
        self.place_glyphs(scene, scale_factor);
        self.atlas
            .record_uploads(&self.device, encoder, frame_index)?;
        self.glyphs
            .upload(&self.device, &self.glyph_instances, frame_index)
    }

    /// Builds one quad per glyph of the scene, rasterizing new glyphs into the atlas. Glyphs
    /// that draw nothing get an empty quad, so quads stay aligned with the scene's batches.
    fn place_glyphs(&mut self, scene: &Scene, scale_factor: f32) {
        self.atlas.begin_frame();
        for attempt in 1..=ATLAS_ATTEMPTS {
            self.glyph_instances.clear();
            let mut full = false;
            for glyph in scene.glyphs() {
                let snapped = GlyphKey::snap(glyph, scale_factor);
                let entry = match snapped.map(|(key, ..)| self.atlas.glyph(key)) {
                    Some(Lookup::Found(entry)) => entry,
                    Some(Lookup::Full) => {
                        full = true;
                        None
                    }
                    None => None,
                };
                let instance = match (entry, snapped) {
                    (Some(entry), Some((_, x, baseline))) => GlyphInstance::new(
                        entry,
                        x,
                        baseline,
                        self.atlas.size(),
                        glyph.color.premultiplied(),
                    ),
                    _ => GlyphInstance::default(),
                };
                self.glyph_instances.push(instance);
            }
            if !full {
                return;
            }
            if attempt == ATLAS_ATTEMPTS {
                koebi_core::warn!("the glyph atlas cannot hold this frame's glyphs");
                return;
            }
            self.atlas.make_room();
        }
    }

    /// Records the scene's draw calls into an active pass: one instanced draw per run of
    /// consecutive primitives of the same type, in painter's order. Call
    /// [`Renderer2D::prepare`] with the same `frame_index` and scale factor first.
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
        if scene.is_empty() {
            return Ok(());
        }
        // Check every buffer before recording anything.
        let rects = (scene.rect_count() > 0)
            .then(|| self.rects.prepared(frame_index, scene.rect_count()))
            .transpose()?;
        let lines = (scene.line_count() > 0)
            .then(|| self.lines.prepared(frame_index, scene.line_count()))
            .transpose()?;
        let glyphs = (scene.glyph_count() > 0)
            .then(|| self.glyphs.prepared(frame_index, scene.glyph_count()))
            .transpose()?;
        let constants = ViewportConstants::new(viewport);
        for batch in scene.batches() {
            let (primitives, buffer) = match batch.kind {
                PrimitiveKind::Rect => (&self.rects, rects),
                PrimitiveKind::Line => (&self.lines, lines),
                PrimitiveKind::Glyph => (&self.glyphs, glyphs),
            };
            // A batch exists only for a type with instances, whose buffer was checked above.
            let Some(buffer) = buffer else { continue };
            pass.set_pipeline(&primitives.pipeline);
            pass.set_push_constants(&primitives.pipeline, &constants)?;
            if batch.kind == PrimitiveKind::Glyph {
                // Prepared glyphs always have an atlas.
                let Some(atlas) = self.atlas.bind_group() else {
                    continue;
                };
                pass.set_bind_group(&primitives.pipeline, 0, atlas)?;
            }
            // From instance zero, at the run's offset: some drivers have no base instance.
            let (offset, count) = batch.window(primitives.stride);
            pass.set_vertex_buffer(0, buffer, offset);
            pass.draw(0..4, 0..count);
        }
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
        self.prepare(encoder, scene, frame_index, scale_factor)?;
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
