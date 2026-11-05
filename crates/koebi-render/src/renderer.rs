//! GPU side of the 2D renderer.

use koebi_core::Extent;
use koebi_gpu::{
    BindGroupLayout, BlendMode, Buffer, BufferDesc, BufferUsage, ColorTargetDesc, CommandEncoder,
    Device, Error, FRAMES_IN_FLIGHT, LoadOp, MemoryLocation, Pod, PrimitiveTopology, RenderPass,
    RenderPipeline, RenderPipelineDesc, RenderTarget, Result, ShaderStage, TextureFormat,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};
use koebi_svg::pack::Packed;
use koebi_text::Fonts;

use crate::atlas::{GlyphAtlas, GlyphInstance, GlyphKey, Lookup};
use crate::fill::FillInstance;
use crate::icon::{IconInstance, IconStore};
use crate::scene::{Corners, LineInstance, PrimitiveKind, RectInstance, Scene};
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

/// Push constants of every 2D pipeline. Must match `ViewportConstants` in the vertex shaders and
/// `ClipConstants` in `shaders/clip.glsl`.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct ViewportConstants {
    viewport_px: [f32; 2],
    scale_factor: f32,
    padding: f32,
    /// The rounded clip: `low.xy, high.xy` in physical pixels, all zero without one.
    clip_rect: [f32; 4],
    /// Its radii in physical pixels: top-left, top-right, bottom-right, bottom-left.
    clip_radii: [f32; 4],
}

// SAFETY: `repr(C)` with twelve `f32`: 48 bytes, no padding, any bit pattern is valid.
unsafe impl Pod for ViewportConstants {}
const _: () = assert!(size_of::<ViewportConstants>() == 48);

impl ViewportConstants {
    fn new(viewport: Viewport) -> Self {
        Self {
            viewport_px: [viewport.extent.width as f32, viewport.extent.height as f32],
            scale_factor: viewport.scale_factor,
            padding: 0.0,
            clip_rect: [0.0; 4],
            clip_radii: [0.0; 4],
        }
    }

    /// The constants of a batch clipped to `rounded`, its rectangle in logical pixels. Its edges
    /// lie where rectangles' snapped edges lie and its radii are limited the same way, so that
    /// a panel and the rounded clip of the same shape end on the same pixels.
    fn with_clip(mut self, rounded: Option<(koebi_core::Rect, Corners)>) -> Self {
        (self.clip_rect, self.clip_radii) = match rounded {
            Some((rect, radii)) if radii.max() > 0.0 => {
                let scale = self.scale_factor;
                let snap = |value: f32| crate::scene::pixel_edge(value * scale);
                let low = [snap(rect.min.x), snap(rect.min.y)];
                let high = [snap(rect.max.x), snap(rect.max.y)];
                let limit = ((high[0] - low[0]).min(high[1] - low[1]) * 0.5).max(0.0);
                (
                    [low[0], low[1], high[0], high[1]],
                    radii
                        .to_array()
                        .map(|radius| (radius * scale).clamp(0.0, limit)),
                )
            }
            _ => ([0.0; 4], [0.0; 4]),
        };
        self
    }
}

/// A scissor rectangle in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Scissor {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl Scissor {
    /// The whole target.
    fn full(extent: Extent) -> Self {
        Self {
            x: 0,
            y: 0,
            width: extent.width,
            height: extent.height,
        }
    }

    /// `clip` (logical pixels) on the target, its edges rounded to the nearest pixel so that
    /// clips sharing an edge share a pixel boundary. `None` when nothing remains.
    fn of(clip: koebi_core::Rect, viewport: Viewport) -> Option<Self> {
        let scale = viewport.scale_factor;
        let edge = |value: f32, size: u32| {
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "rounded and clamped to the target first"
            )]
            let pixel = crate::scene::pixel_edge(value * scale).clamp(0.0, size as f32) as u32;
            pixel
        };
        let (width, height) = (viewport.extent.width, viewport.extent.height);
        let (x0, x1) = (edge(clip.min.x, width), edge(clip.max.x, width));
        let (y0, y1) = (edge(clip.min.y, height), edge(clip.max.y, height));
        (x1 > x0 && y1 > y0).then(|| Self {
            x: i32::try_from(x0).unwrap_or(i32::MAX),
            y: i32::try_from(y0).unwrap_or(i32::MAX),
            width: x1 - x0,
            height: y1 - y0,
        })
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
            push_constant_size: 48,
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
    fills: PrimitivePipeline,
    icons: PrimitivePipeline,
    icon_store: IconStore,
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
        let fills = PipelineDesc {
            label: "fill",
            vertex: ("fill.vert", shaders::FILL_VERT),
            fragment: ("fill.frag", shaders::FILL_FRAG),
            stride: FillInstance::STRIDE,
            bind_group_layouts: &[],
        };
        let icon_store = IconStore::new(device)?;
        let icons = PipelineDesc {
            label: "icon",
            vertex: ("icon.vert", shaders::ICON_VERT),
            fragment: ("icon.frag", shaders::ICON_FRAG),
            stride: IconInstance::STRIDE,
            bind_group_layouts: &[icon_store.layout()],
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
            fills: PrimitivePipeline::new(device, &fills, target_format)?,
            icons: PrimitivePipeline::new(device, &icons, target_format)?,
            icon_store,
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

    /// Sets the icons [`crate::IconPrimitive::icon`] numbers refer to: `koebi_svg` icons packed
    /// in order. They are uploaded by the next [`Renderer2D::prepare`]; until then, and without
    /// a set, icons draw nothing.
    pub fn set_icons(&mut self, icons: &Packed) {
        self.icon_store.set(icons);
    }

    /// How many icons the set given to [`Renderer2D::set_icons`] holds.
    pub fn icon_count(&self) -> usize {
        self.icon_store.len()
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
        self.fills
            .upload(&self.device, scene.fill_instances(), frame_index)?;
        self.icons
            .upload(&self.device, scene.icon_instances(), frame_index)?;
        self.icon_store.record_upload(&self.device, encoder)?;
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
        let fills = (scene.fill_count() > 0)
            .then(|| self.fills.prepared(frame_index, scene.fill_count()))
            .transpose()?;
        let glyphs = (scene.glyph_count() > 0)
            .then(|| self.glyphs.prepared(frame_index, scene.glyph_count()))
            .transpose()?;
        let icons = (scene.icon_count() > 0)
            .then(|| self.icons.prepared(frame_index, scene.icon_count()))
            .transpose()?;
        let constants = ViewportConstants::new(viewport);
        let full = Scissor::full(viewport.extent);
        let mut current = full;
        for batch in scene.batches() {
            let scissor = batch
                .clip
                .map_or(Some(full), |clip| Scissor::of(clip.rect, viewport));
            // Clipped away entirely.
            let Some(scissor) = scissor else { continue };
            if scissor != current {
                pass.set_scissor(scissor.x, scissor.y, scissor.width, scissor.height);
                current = scissor;
            }
            let (primitives, buffer) = match batch.kind {
                PrimitiveKind::Rect => (&self.rects, rects),
                PrimitiveKind::Line => (&self.lines, lines),
                PrimitiveKind::Glyph => (&self.glyphs, glyphs),
                PrimitiveKind::Fill => (&self.fills, fills),
                PrimitiveKind::Icon => (&self.icons, icons),
            };
            // A batch exists only for a type with instances, whose buffer was checked above.
            let Some(buffer) = buffer else { continue };
            pass.set_pipeline(&primitives.pipeline);
            let rounded = batch.clip.and_then(|clip| clip.rounded);
            pass.set_push_constants(&primitives.pipeline, &constants.with_clip(rounded))?;
            if batch.kind == PrimitiveKind::Glyph {
                // Prepared glyphs always have an atlas.
                let Some(atlas) = self.atlas.bind_group() else {
                    continue;
                };
                pass.set_bind_group(&primitives.pipeline, 0, atlas)?;
            }
            if batch.kind == PrimitiveKind::Icon {
                // Without a set, icons draw nothing.
                let Some(set) = self.icon_store.bind_group() else {
                    continue;
                };
                pass.set_bind_group(&primitives.pipeline, 0, set)?;
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
    fn clips_become_rounded_scissors_inside_the_target() {
        let viewport = Viewport {
            extent: Extent::new(100, 80),
            scale_factor: 1.5,
        };
        let rect = |x0, y0, x1, y1| {
            koebi_core::Rect::from_min_max(
                koebi_core::Vec2::new(x0, y0),
                koebi_core::Vec2::new(x1, y1),
            )
        };
        // 10.3 * 1.5 = 15.45 rounds to 15; 20.5 * 1.5 = 30.75 rounds to 31.
        assert_eq!(
            Scissor::of(rect(10.3, 4.0, 20.5, 8.0), viewport),
            Some(Scissor {
                x: 15,
                y: 6,
                width: 16,
                height: 6,
            })
        );
        // Clamped to the target; nothing left outside it.
        assert_eq!(
            Scissor::of(rect(-5.0, -5.0, 500.0, 500.0), viewport),
            Some(Scissor::full(viewport.extent))
        );
        assert_eq!(Scissor::of(rect(200.0, 0.0, 300.0, 10.0), viewport), None);
        assert_eq!(Scissor::of(koebi_core::Rect::ZERO, viewport), None);
    }

    #[test]
    fn rounded_clips_lie_on_the_pixels_of_a_rectangle_of_the_same_shape() {
        let viewport = Viewport {
            extent: Extent::new(100, 80),
            scale_factor: 1.25,
        };
        let constants = ViewportConstants::new(viewport);
        // Nothing to cut: the shaders skip it.
        assert_eq!(constants.clip_rect, [0.0; 4]);
        let rect = koebi_core::Rect::from_min_max(
            koebi_core::Vec2::new(2.3, 4.0),
            koebi_core::Vec2::new(20.5, 12.0),
        );
        let square = constants.with_clip(Some((rect, Corners::ZERO)));
        assert_eq!(square.clip_rect, [0.0; 4]);
        // 2.3 * 1.25 = 2.875 goes to 3, 20.5 * 1.25 = 25.625 to 26; 4 pixels of radius are 5,
        // the height of 10 pixels limits them to 5.
        let rounded = constants.with_clip(Some((rect, Corners::all(8.0))));
        assert_eq!(rounded.clip_rect, [3.0, 5.0, 26.0, 15.0]);
        assert_eq!(rounded.clip_radii, [5.0; 4]);
    }

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
