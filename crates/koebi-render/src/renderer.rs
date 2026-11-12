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

use crate::batch::DrawOrder;
use crate::fill::FillInstance;
use crate::glyphs::{GlyphInstance, GlyphStore, Lookup};
use crate::icon::{IconInstance, IconStore};
use crate::scene::{Clip, GlyphPrimitive, LineInstance, PrimitiveKind, RectInstance, Scene};
use crate::shaders;

/// Times a frame's glyphs may make room in the glyph store (grow, grow, clear) before giving up.
const STORE_ATTEMPTS: usize = 2;

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

/// Push constants of every 2D pipeline. Must match `ViewportConstants` in the vertex shaders.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct ViewportConstants {
    viewport_px: [f32; 2],
    scale_factor: f32,
    padding: f32,
}

// SAFETY: `repr(C)` with four `f32`: 16 bytes, no padding, any bit pattern is valid.
unsafe impl Pod for ViewportConstants {}
const _: () = assert!(size_of::<ViewportConstants>() == ViewportConstants::SIZE as usize);

impl ViewportConstants {
    /// Bytes of the block.
    const SIZE: u32 = 16;

    fn new(viewport: Viewport) -> Self {
        Self {
            viewport_px: [viewport.extent.width as f32, viewport.extent.height as f32],
            scale_factor: viewport.scale_factor,
            padding: 0.0,
        }
    }
}

/// The clip of one instance, in a vertex stream of its own beside the instances of every type.
/// Must match `shaders/clip_vertex.glsl`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
struct ClipInstance {
    /// The pixels kept: `low.xy, high.xy`, whole physical pixels, as a scissor keeps them.
    rect: [f32; 4],
    /// The rounded clip: `low.xy, high.xy` in physical pixels, all zero without one.
    rounded: [f32; 4],
    /// Its radii in physical pixels: top-left, top-right, bottom-right, bottom-left.
    radii: [f32; 4],
}

// SAFETY: `repr(C)` with twelve `f32`: 48 bytes, no padding, any bit pattern is valid.
unsafe impl Pod for ClipInstance {}
const _: () = assert!(size_of::<ClipInstance>() == ClipInstance::STRIDE as usize);

impl ClipInstance {
    const STRIDE: u32 = 48;

    /// Nothing clipped.
    const NONE: Self = Self {
        rect: [f32::MIN, f32::MIN, f32::MAX, f32::MAX],
        rounded: [0.0; 4],
        radii: [0.0; 4],
    };

    /// `clip` at `scale` physical pixels per logical pixel. Its edges go to the nearest pixel
    /// edges, where rectangles' snapped edges go, and its radii are limited the same way: a
    /// panel and the clip of the same shape end on the same pixels.
    fn new(clip: &Clip, scale: f32) -> Self {
        let snap = |value: f32| crate::scene::pixel_edge(value * scale);
        let edges = |rect: koebi_core::Rect| {
            [
                snap(rect.min.x),
                snap(rect.min.y),
                snap(rect.max.x),
                snap(rect.max.y),
            ]
        };
        let (rounded, radii) = match clip.rounded {
            Some((rect, radii)) if radii.max() > 0.0 => {
                let rounded = edges(rect);
                let limit = ((rounded[2] - rounded[0]).min(rounded[3] - rounded[1]) * 0.5).max(0.0);
                (
                    rounded,
                    radii
                        .to_array()
                        .map(|radius| (radius * scale).clamp(0.0, limit)),
                )
            }
            _ => ([0.0; 4], [0.0; 4]),
        };
        Self {
            rect: edges(clip.rect),
            rounded,
            radii,
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

/// Buffers streamed to the GPU every frame, one per frame in flight: the CPU writes slot N
/// while the GPU reads the others.
#[derive(Debug)]
struct Stream {
    label: &'static str,
    /// Bytes per element.
    stride: u32,
    buffers: Vec<Option<Buffer>>,
}

impl Stream {
    fn new(label: &'static str, stride: u32) -> Self {
        Self {
            label,
            stride,
            buffers: (0..FRAMES_IN_FLIGHT).map(|_| None).collect(),
        }
    }

    /// Writes `elements` into the buffer of `frame_index`, growing it geometrically.
    fn upload<T: Pod>(
        &mut self,
        device: &Device,
        elements: &[T],
        frame_index: usize,
    ) -> Result<()> {
        if elements.is_empty() {
            return Ok(());
        }
        let slot = self
            .buffers
            .get_mut(frame_index)
            .ok_or_else(|| Error::InvalidArgument(format!("frame index {frame_index}")))?;
        let needed = u64::try_from(size_of_val(elements))
            .map_err(|_| Error::InvalidArgument("scene too large".into()))?;
        if slot.as_ref().is_none_or(|buffer| buffer.size() < needed) {
            let minimum = MIN_INSTANCES_PER_BUFFER * u64::from(self.stride);
            let size = needed.next_power_of_two().max(minimum);
            koebi_core::debug!(
                "growing {} buffer {frame_index} to {size} bytes",
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
            buffer.write(0, elements)?;
        }
        Ok(())
    }

    /// The buffer of `frame_index`, if it holds at least `count` elements.
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

/// The pipeline of one primitive type, its instances and their clips.
#[derive(Debug)]
struct PrimitivePipeline {
    pipeline: RenderPipeline,
    instances: Stream,
    clips: Stream,
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
        // The clip of each instance, at the same locations in every vertex shader.
        let clip_attributes: Vec<_> = (0..ClipInstance::STRIDE / 16)
            .map(|at| VertexAttribute {
                location: CLIP_LOCATION + at,
                format: VertexFormat::Float32x4,
                offset: at * 16,
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
            vertex_buffers: &[
                VertexBufferLayout {
                    stride: desc.stride,
                    step_mode: VertexStepMode::Instance,
                    attributes: &attributes,
                },
                VertexBufferLayout {
                    stride: ClipInstance::STRIDE,
                    step_mode: VertexStepMode::Instance,
                    attributes: &clip_attributes,
                },
            ],
            topology: PrimitiveTopology::TriangleStrip,
            color_targets: &[ColorTargetDesc {
                format: target_format,
                blend: BlendMode::PremultipliedAlpha,
            }],
            push_constant_size: ViewportConstants::SIZE,
            bind_group_layouts: desc.bind_group_layouts,
        })?;
        Ok(Self {
            pipeline,
            instances: Stream::new(desc.label, desc.stride),
            clips: Stream::new(desc.label, ClipInstance::STRIDE),
        })
    }

    /// The instance and clip buffers of `frame_index`, if they hold `count` instances.
    fn prepared(&self, frame_index: usize, count: usize) -> Result<(&Buffer, &Buffer)> {
        Ok((
            self.instances.prepared(frame_index, count)?,
            self.clips.prepared(frame_index, count)?,
        ))
    }
}

/// The location of the first of the clip's attributes in the vertex shaders.
const CLIP_LOCATION: u32 = 12;

/// Draws [`Scene`]s. Create one per target format; reuse it every frame.
///
/// Glyphs are drawn from the curves of the fonts given to [`Renderer2D::set_fonts`], stored the
/// first time a frame draws them (ADR 0022); without fonts, glyphs draw nothing.
#[derive(Debug)]
pub struct Renderer2D {
    device: Device,
    rects: PrimitivePipeline,
    lines: PrimitivePipeline,
    fills: PrimitivePipeline,
    icons: PrimitivePipeline,
    icon_store: IconStore,
    glyphs: PrimitivePipeline,
    glyph_store: GlyphStore,
    /// This frame's glyph quads, rebuilt by every `prepare`.
    glyph_instances: Vec<GlyphInstance>,
    /// Where each batch's instances lie, and the instances gathered batch after batch for the
    /// GPU: rebuilt by every `prepare`, their storage kept.
    order: DrawOrder,
    gathered: Gathered,
    /// The scene's clips for the shaders, by number: rebuilt by every `prepare`.
    clips: Vec<ClipInstance>,
    target_format: TextureFormat,
}

/// The instances of a scene gathered batch after batch ([`DrawOrder::gather`]).
#[derive(Debug, Default)]
struct Gathered {
    rects: Vec<RectInstance>,
    lines: Vec<LineInstance>,
    fills: Vec<FillInstance>,
    icons: Vec<IconInstance>,
    glyphs: Vec<GlyphPrimitive>,
    /// The clip numbers of one type's instances, gathered: reused for each type.
    clip_ids: Vec<u32>,
    /// The clips of each type's instances, gathered, by type.
    clips: [Vec<ClipInstance>; PrimitiveKind::COUNT],
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
        let glyph_store = GlyphStore::new(device)?;
        let glyphs = PipelineDesc {
            label: "glyph",
            vertex: ("glyph.vert", shaders::GLYPH_VERT),
            fragment: ("glyph.frag", shaders::GLYPH_FRAG),
            stride: GlyphInstance::STRIDE,
            bind_group_layouts: &[glyph_store.layout()],
        };
        Ok(Self {
            device: device.clone(),
            rects: PrimitivePipeline::new(device, &rects, target_format)?,
            lines: PrimitivePipeline::new(device, &lines, target_format)?,
            fills: PrimitivePipeline::new(device, &fills, target_format)?,
            icons: PrimitivePipeline::new(device, &icons, target_format)?,
            icon_store,
            glyphs: PrimitivePipeline::new(device, &glyphs, target_format)?,
            glyph_store,
            glyph_instances: Vec::new(),
            order: DrawOrder::default(),
            gathered: Gathered::default(),
            clips: Vec::new(),
            target_format,
        })
    }

    /// Sets the fonts glyphs are drawn from: the collection the scene's
    /// [`koebi_text::FontId`]s refer to. Cached glyphs are forgotten.
    pub fn set_fonts(&mut self, fonts: Fonts) {
        self.glyph_store.set_fonts(fonts);
    }

    /// Sets the icons [`crate::IconPrimitive::icon`] numbers refer to: `koebi_svg` icons packed
    /// in order. They are uploaded by the next [`Renderer2D::prepare`]; until then, and without
    /// a set, icons draw nothing.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`], keeping the current set, for a set whose offsets or
    /// counts leave it ([`Packed::validate`]): the shader could otherwise read past the data.
    pub fn set_icons(&mut self, icons: &Packed) -> Result<()> {
        self.icon_store.set(icons)
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
    /// records into `encoder` the uploads of the curves of glyphs drawn for the first time. Call it
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
        koebi_core::profile_scope!("render.prepare");
        // Each batch's instances one after the other, whatever order they were pushed in.
        self.order.compute(scene.batching());
        // Each instance's clip, gathered the same way.
        self.clips.clear();
        self.clips.push(ClipInstance::NONE);
        self.clips.extend(
            scene
                .clip_table()
                .iter()
                .map(|clip| ClipInstance::new(clip, scale_factor)),
        );
        for kind in PrimitiveKind::ALL {
            let gathered = &mut self.gathered;
            self.order.gather(
                scene.clip_ids(kind),
                scene.owners(kind),
                &mut gathered.clip_ids,
            );
            let clips = &mut gathered.clips[kind as usize];
            clips.clear();
            clips.extend(gathered.clip_ids.iter().map(|&id| {
                self.clips
                    .get(id as usize)
                    .copied()
                    .unwrap_or(ClipInstance::NONE)
            }));
        }
        let gathered = &mut self.gathered;
        let order = &mut self.order;
        order.gather(
            scene.rect_instances(),
            scene.owners(PrimitiveKind::Rect),
            &mut gathered.rects,
        );
        order.gather(
            scene.line_instances(),
            scene.owners(PrimitiveKind::Line),
            &mut gathered.lines,
        );
        order.gather(
            scene.fill_instances(),
            scene.owners(PrimitiveKind::Fill),
            &mut gathered.fills,
        );
        order.gather(
            scene.icon_instances(),
            scene.owners(PrimitiveKind::Icon),
            &mut gathered.icons,
        );
        order.gather(
            scene.glyphs(),
            scene.owners(PrimitiveKind::Glyph),
            &mut gathered.glyphs,
        );
        let device = &self.device;
        let gathered = &self.gathered;
        self.rects
            .instances
            .upload(device, &gathered.rects, frame_index)?;
        self.lines
            .instances
            .upload(device, &gathered.lines, frame_index)?;
        self.fills
            .instances
            .upload(device, &gathered.fills, frame_index)?;
        self.icons
            .instances
            .upload(device, &gathered.icons, frame_index)?;
        for (pipeline, kind) in [
            (&mut self.rects, PrimitiveKind::Rect),
            (&mut self.lines, PrimitiveKind::Line),
            (&mut self.fills, PrimitiveKind::Fill),
            (&mut self.icons, PrimitiveKind::Icon),
            (&mut self.glyphs, PrimitiveKind::Glyph),
        ] {
            pipeline
                .clips
                .upload(device, &gathered.clips[kind as usize], frame_index)?;
        }
        self.icon_store.record_upload(&self.device, encoder);
        if scene.glyph_count() == 0 {
            return Ok(());
        }
        {
            koebi_core::profile_scope!("render.place_glyphs");
            self.place_glyphs(scale_factor);
        }
        self.glyph_store
            .record_uploads(&self.device, encoder, frame_index)?;
        self.glyphs
            .instances
            .upload(&self.device, &self.glyph_instances, frame_index)
    }

    /// Builds one instance per glyph of the scene, storing the curves of glyphs drawn for the
    /// first time. Upright glyphs keep their baseline on a whole physical pixel, so that
    /// horizontal stems stay crisp; their pen keeps its exact position across. Glyphs that draw
    /// nothing get an empty instance, so instances stay aligned with the scene's batches.
    fn place_glyphs(&mut self, scale_factor: f32) {
        for attempt in 1..=STORE_ATTEMPTS {
            self.glyph_instances.clear();
            let mut full = false;
            for glyph in &self.gathered.glyphs {
                let stored = match self.glyph_store.glyph(glyph.font, glyph.glyph) {
                    Lookup::Found(stored) => stored,
                    Lookup::Full => {
                        full = true;
                        None
                    }
                };
                let mut origin = glyph.origin * scale_factor;
                if glyph.rotation == 0.0 {
                    origin.y = origin.y.round();
                }
                let instance = stored.map_or_else(GlyphInstance::default, |stored| {
                    GlyphInstance::new(
                        stored,
                        origin,
                        glyph.size * scale_factor,
                        glyph.rotation,
                        glyph.color.premultiplied(),
                    )
                });
                self.glyph_instances.push(instance);
            }
            if !full {
                return;
            }
            if attempt == STORE_ATTEMPTS {
                koebi_core::warn!("the glyph store cannot hold this frame's glyphs");
                return;
            }
            self.glyph_store.make_room();
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
        koebi_core::profile_scope!("render.draw");
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
        // Every instance carries its clip: the whole target stays open.
        pass.set_scissor(0, 0, viewport.extent.width, viewport.extent.height);
        // The pipeline bound last: a draw binds only what differs.
        let mut bound = None;
        for (id, batch) in scene.batches() {
            let (primitives, buffers) = match batch.kind {
                PrimitiveKind::Rect => (&self.rects, rects),
                PrimitiveKind::Line => (&self.lines, lines),
                PrimitiveKind::Glyph => (&self.glyphs, glyphs),
                PrimitiveKind::Fill => (&self.fills, fills),
                PrimitiveKind::Icon => (&self.icons, icons),
            };
            // A batch exists only for a type with instances, whose buffers were checked above.
            let Some((instances, clips)) = buffers else {
                continue;
            };
            if bound != Some(batch.kind) {
                pass.set_pipeline(&primitives.pipeline);
                let store = match batch.kind {
                    // Prepared glyphs always have a store.
                    PrimitiveKind::Glyph => self.glyph_store.bind_group(),
                    // Without a set, icons draw nothing.
                    PrimitiveKind::Icon => self.icon_store.bind_group(),
                    _ => None,
                };
                let needs_store = matches!(batch.kind, PrimitiveKind::Glyph | PrimitiveKind::Icon);
                match store {
                    Some(store) => pass.set_bind_group(&primitives.pipeline, 0, store)?,
                    None if needs_store => {
                        bound = None;
                        continue;
                    }
                    None => {}
                }
                pass.set_push_constants(&primitives.pipeline, &constants)?;
                bound = Some(batch.kind);
            }
            // From instance zero, at the run's offset: some drivers have no base instance.
            let (offset, count) = self.order.window(id, *batch, primitives.instances.stride);
            let (clip_offset, _) = self.order.window(id, *batch, ClipInstance::STRIDE);
            pass.set_vertex_buffer(0, instances, offset);
            pass.set_vertex_buffer(1, clips, clip_offset);
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
    use koebi_core::{Rect, Vec2};

    use super::*;
    use crate::Corners;

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(Vec2::new(x0, y0), Vec2::new(x1, y1))
    }

    #[test]
    fn clips_keep_whole_pixels_as_scissors_do() {
        let clip = Clip {
            rect: rect(10.3, 4.0, 20.5, 8.0),
            rounded: None,
        };
        // 10.3 * 1.5 = 15.45 rounds to 15; 20.5 * 1.5 = 30.75 rounds to 31.
        let instance = ClipInstance::new(&clip, 1.5);
        assert_eq!(instance.rect, [15.0, 6.0, 31.0, 12.0]);
        // Nothing to cut at the corners: the shaders skip it.
        assert_eq!(instance.rounded, [0.0; 4]);
        assert_eq!(ClipInstance::NONE.rect[2], f32::MAX);
    }

    #[test]
    fn rounded_clips_lie_on_the_pixels_of_a_rectangle_of_the_same_shape() {
        let shape = rect(2.3, 4.0, 20.5, 12.0);
        let clip = |radii| Clip {
            rect: shape,
            rounded: Some((shape, radii)),
        };
        let square = ClipInstance::new(&clip(Corners::ZERO), 1.25);
        assert_eq!(square.rounded, [0.0; 4]);
        // 2.3 * 1.25 = 2.875 goes to 3, 20.5 * 1.25 = 25.625 to 26; 8 pixels of radius are 10,
        // the height of 10 pixels limits them to 5.
        let rounded = ClipInstance::new(&clip(Corners::all(8.0)), 1.25);
        assert_eq!(rounded.rounded, [3.0, 5.0, 26.0, 15.0]);
        assert_eq!(rounded.radii, [5.0; 4]);
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
