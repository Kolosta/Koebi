//! [`Canvas`]: draws programs into views' targets, one instance per cell of each outline, from
//! cells kept from render to render ([`crate::store`]).
//!
//! - **Data:** one texture of records for every view, updated by range: a render sends the
//!   records it added and the header, and everything only once records moved or the texture
//!   grew. Copies wait for earlier frames' draws, which are ordered before them on the queue.
//! - **Instances** are per view: each [`ViewTarget`] keeps its own, rebuilt when its program or
//!   view changes, and written into its buffer for the frame in flight.
//! - **Steady state:** a target that shows the same program revision through the same view
//!   is not drawn again, and nothing is recorded: the interface shows the last image.

use koebi_core::Extent;
use koebi_doc::eval::Program;
use koebi_gpu::{
    AddressMode, BindGroup, BindGroupDesc, BindGroupLayout, BindGroupLayoutDesc, BindingResource,
    BindingType, BlendMode, Buffer, BufferDesc, BufferTexels, BufferUsage, ColorTargetDesc,
    CommandEncoder, Device, Error, FRAMES_IN_FLIGHT, FilterMode, LoadOp, MemoryLocation, Pod,
    PrimitiveTopology, RenderPipeline, RenderPipelineDesc, Result, Sampler, SamplerDesc,
    ShaderStage, Texture, TextureDesc, TextureFormat, TextureRegion, TextureState, TextureUsage,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};

use crate::cells::DATA_WIDTH;
use crate::frame::{CellInstance, Frame};
use crate::store::{CanvasStats, CellStore};
use crate::view::View;

static CANVAS_VERT: &[u32] =
    koebi_gpu::include_spirv!(include_bytes!("../shaders/spirv/canvas.vert.spv"));
static CANVAS_FRAG: &[u32] =
    koebi_gpu::include_spirv!(include_bytes!("../shaders/spirv/canvas.frag.spv"));

/// The format of every view's target: linear light, premultiplied, in halves.
pub const TARGET_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

/// Rows of the data texture at the least: grows by doubling.
const MIN_ROWS: u32 = 16;
/// Rows of the data texture at the most: the store's capacity.
const MAX_ROWS: u32 = crate::store::ROWS;
/// Bytes of a texel of the data texture.
const TEXEL: u64 = 16;
/// The smallest buffer a stream makes.
const MIN_STREAM: u64 = 64 * 1024;

/// A host-visible buffer per frame in flight, written at a cursor. A frame slot is free again
/// once a later submission records into it (the presenter waits for its last one), so the
/// first write of a submission starts at the beginning; the writes of one submission follow
/// one another.
#[derive(Debug)]
struct Stream {
    label: &'static str,
    usage: BufferUsage,
    slots: Vec<StreamSlot>,
}

#[derive(Debug, Default)]
struct StreamSlot {
    buffer: Option<Buffer>,
    /// Where the next write starts.
    cursor: u64,
    /// The submission whose writes the slot holds.
    submission: u64,
}

impl Stream {
    fn new(label: &'static str, usage: BufferUsage) -> Self {
        Self {
            label,
            usage,
            slots: (0..FRAMES_IN_FLIGHT)
                .map(|_| StreamSlot::default())
                .collect(),
        }
    }

    /// Writes `parts` one after another into the buffer of `frame_index` for `submission`,
    /// made anew when too small (or far too large), and returns it with where they start.
    fn write<T: Pod>(
        &mut self,
        device: &Device,
        submission: u64,
        frame_index: usize,
        parts: &[&[T]],
    ) -> Result<(&Buffer, u64)> {
        let slot = self
            .slots
            .get_mut(frame_index)
            .ok_or_else(|| Error::InvalidArgument(format!("frame index {frame_index}")))?;
        let bytes: u64 = parts.iter().map(|part| size_of_val(*part) as u64).sum();
        if slot.submission != submission {
            slot.submission = submission;
            slot.cursor = 0;
        }
        let size = slot.buffer.as_ref().map_or(0, Buffer::size);
        let wanted = (slot.cursor + bytes).next_power_of_two().max(MIN_STREAM);
        // Too small, or a large upload's buffer kept for small ones.
        if slot.cursor + bytes > size || (slot.cursor == 0 && size > 4 * wanted) {
            // What this submission wrote before stays in the old buffer, alive until done.
            slot.buffer = Some(
                device.create_buffer(&BufferDesc {
                    label: self.label,
                    size: bytes
                        .next_power_of_two()
                        .max(MIN_STREAM)
                        .max(size.min(2 * wanted)),
                    usage: self.usage,
                    location: MemoryLocation::CpuToGpu,
                })?,
            );
            slot.cursor = 0;
        }
        let start = slot.cursor;
        let buffer = slot
            .buffer
            .as_mut()
            .ok_or_else(|| Error::InvalidArgument(format!("{} missing", self.label)))?;
        let mut at = start;
        for part in parts {
            buffer.write(at, part)?;
            at += size_of_val(*part) as u64;
        }
        // Texels and instances are 16-byte aligned: so are the writes that follow.
        slot.cursor = at.next_multiple_of(TEXEL);
        Ok((buffer, start))
    }
}

/// The target a view renders into: `Rgba16Float`, linear light and premultiplied, sampled by
/// the interface as an image once drawn. It keeps the view's instances and what it shows.
#[derive(Debug)]
pub struct ViewTarget {
    texture: Texture,
    /// The layers and instances of the last render.
    frame: Frame,
    instances: Stream,
    /// The program revision and the view it shows.
    shown: Option<(u64, View)>,
    /// Whether it was ever drawn: interface frames in flight may still sample it.
    drawn: bool,
}

impl ViewTarget {
    /// A target of `extent` physical pixels.
    ///
    /// # Errors
    ///
    /// Fails for an empty extent or when the device cannot create the texture.
    pub fn new(device: &Device, extent: Extent) -> Result<Self> {
        Ok(Self {
            texture: device.create_texture(&TextureDesc {
                label: "view target",
                extent,
                format: TARGET_FORMAT,
                usage: TextureUsage::COLOR_ATTACHMENT
                    | TextureUsage::SAMPLED
                    | TextureUsage::TRANSFER_SRC,
            })?,
            frame: Frame::default(),
            instances: Stream::new("canvas instances", BufferUsage::VERTEX),
            shown: None,
            drawn: false,
        })
    }

    /// The texture: in [`TextureState::ShaderRead`] once [`Canvas::render`] has drawn it.
    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    /// Its size, in physical pixels.
    pub fn extent(&self) -> Extent {
        self.texture.extent()
    }

    /// The revision of the program it shows and the view it shows it through, once drawn.
    pub fn shown(&self) -> Option<(u64, View)> {
        self.shown
    }
}

/// Push constants of the canvas pipeline. Must match `Constants` in `shaders/canvas.vert`.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct Constants {
    target_px: [f32; 2],
    padding: [f32; 2],
}

// SAFETY: `repr(C)` with four `f32`: 16 bytes, no padding, any bit pattern is valid.
unsafe impl Pod for Constants {}

/// Draws programs into views' targets ([`Canvas::render`]). Keep one per device for every view;
/// reuse it every frame: its cells and storage stay.
pub struct Canvas {
    device: Device,
    pipeline: RenderPipeline,
    layout: BindGroupLayout,
    sampler: Sampler,
    /// The cells of the outlines views showed, and their records.
    store: CellStore,
    /// The texture of records, and its bind group.
    data: Option<(Texture, BindGroup)>,
    /// Whether the texture was ever written: its first copy comes from `Undefined`.
    written: bool,
    staging: Stream,
    /// The copies of an upload, and where their texels lie in the staging buffer: reused.
    regions: Vec<(TextureRegion, u64)>,
}

impl std::fmt::Debug for Canvas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Canvas")
            .field("texels", &self.store.data().len())
            .field("stats", &self.store.stats)
            .finish_non_exhaustive()
    }
}

impl Canvas {
    /// Compiles the canvas's pipeline.
    ///
    /// # Errors
    ///
    /// Fails if the shader modules, the pipeline or the sampler cannot be created.
    pub fn new(device: &Device) -> Result<Self> {
        let layout = device.create_bind_group_layout(&BindGroupLayoutDesc {
            label: "canvas cells",
            entries: &[BindingType::SampledTexture],
        })?;
        let vertex = device.create_shader_module("canvas.vert", CANVAS_VERT)?;
        let fragment = device.create_shader_module("canvas.frag", CANVAS_FRAG)?;
        let attributes: Vec<_> = (0..4)
            .map(|location| VertexAttribute {
                location,
                format: VertexFormat::Float32x4,
                offset: location * 16,
            })
            .collect();
        let pipeline = device.create_render_pipeline(&RenderPipelineDesc {
            label: "canvas",
            vertex: ShaderStage {
                module: &vertex,
                entry_point: c"main",
            },
            fragment: ShaderStage {
                module: &fragment,
                entry_point: c"main",
            },
            vertex_buffers: &[VertexBufferLayout {
                #[allow(clippy::cast_possible_truncation, reason = "64 bytes")]
                stride: size_of::<CellInstance>() as u32,
                step_mode: VertexStepMode::Instance,
                attributes: &attributes,
            }],
            topology: PrimitiveTopology::TriangleStrip,
            color_targets: &[ColorTargetDesc {
                format: TARGET_FORMAT,
                blend: BlendMode::PremultipliedAlpha,
            }],
            push_constant_size: 16,
            bind_group_layouts: &[&layout],
        })?;
        let sampler = device.create_sampler(&SamplerDesc {
            label: "canvas cells",
            // Read with texelFetch, never filtered.
            filter: FilterMode::Nearest,
            address_mode: AddressMode::ClampToEdge,
        })?;
        Ok(Self {
            device: device.clone(),
            pipeline,
            layout,
            sampler,
            store: CellStore::default(),
            data: None,
            written: false,
            staging: Stream::new("canvas staging", BufferUsage::TRANSFER_SRC),
            regions: Vec::with_capacity(4),
        })
    }

    /// What the canvas did since [`Self::reset_stats`].
    pub fn stats(&self) -> CanvasStats {
        self.store.stats
    }

    /// Forgets the counts of [`Self::stats`].
    pub fn reset_stats(&mut self) {
        self.store.stats = CanvasStats::default();
    }

    /// Records into `encoder` the drawing of `program` as `view` shows it into `target`, and
    /// returns whether it recorded anything:
    /// - `false` when the target already shows this program revision through this view: the
    ///   steady state, which records nothing and allocates nothing;
    /// - else the upload of the records the view needs that the GPU lacks, a pass clearing the
    ///   target to transparent and drawing every layer over the ones below, and the target's
    ///   transition to [`TextureState::ShaderRead`], ready to show as an image.
    ///
    /// `frame_index` must not be in use by the GPU, as the presenter guarantees. Several views
    /// may render in one frame.
    ///
    /// Until compositing comes, layers blended in another mode, masked or isolated draw as
    /// layers over the ones below, at their opacity.
    ///
    /// # Errors
    ///
    /// Fails if the target's size is not the view's, if `frame_index` is not below
    /// [`FRAMES_IN_FLIGHT`], if the view shows more outlines than the canvas holds, or if the
    /// device cannot make its buffers.
    pub fn render(
        &mut self,
        encoder: &mut CommandEncoder<'_>,
        target: &mut ViewTarget,
        program: &Program,
        view: &View,
        frame_index: usize,
    ) -> Result<bool> {
        if target.extent() != view.extent {
            return Err(Error::InvalidArgument(format!(
                "a view of {:?} cannot draw into a target of {:?}",
                view.extent,
                target.extent()
            )));
        }
        if frame_index >= FRAMES_IN_FLIGHT {
            return Err(Error::InvalidArgument(format!("frame index {frame_index}")));
        }
        if target.shown == Some((program.revision(), *view)) {
            return Ok(false);
        }
        self.store.begin();
        if target.frame.build(program, view, &mut self.store).is_err() {
            // Room for this view alone: what others kept is prepared again when they change.
            self.store.clear();
            if target.frame.build(program, view, &mut self.store).is_err() {
                return Err(Error::InvalidArgument(
                    "the view shows more outlines than the canvas holds".into(),
                ));
            }
        }
        self.upload(encoder, frame_index)?;
        let submission = encoder.submission();
        let instances = if target.frame.instances.is_empty() {
            None
        } else {
            Some(target.instances.write(
                &self.device,
                submission,
                frame_index,
                &[target.frame.instances.as_slice()],
            )?)
        };
        // Interface frames in flight may still sample what it showed: the pass waits for them.
        let from = if target.drawn {
            TextureState::ShaderRead
        } else {
            TextureState::Undefined
        };
        encoder.texture_barrier(&target.texture, from, TextureState::ColorAttachment);
        {
            let mut pass =
                encoder.begin_render_pass(target.texture.render_target()?, LoadOp::Clear([0.0; 4]));
            if let (Some((_, bind_group)), Some((buffer, offset))) = (&self.data, instances) {
                let extent = view.extent;
                pass.set_scissor(0, 0, extent.width, extent.height);
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(&self.pipeline, 0, bind_group)?;
                #[allow(clippy::cast_precision_loss, reason = "a target's size")]
                pass.set_push_constants(
                    &self.pipeline,
                    &Constants {
                        target_px: [extent.width as f32, extent.height as f32],
                        padding: [0.0; 2],
                    },
                )?;
                pass.set_vertex_buffer(0, buffer, offset);
                let count = u32::try_from(target.frame.instances.len())
                    .map_err(|_| Error::InvalidArgument("too many cells".into()))?;
                pass.draw(0..4, 0..count);
            }
        }
        encoder.texture_barrier(
            &target.texture,
            TextureState::ColorAttachment,
            TextureState::ShaderRead,
        );
        target.shown = Some((program.revision(), *view));
        target.drawn = true;
        self.store.stats.rendered += 1;
        Ok(true)
    }

    /// Uploads the header and the records the GPU lacks, growing the texture first if needed.
    fn upload(&mut self, encoder: &mut CommandEncoder<'_>, frame_index: usize) -> Result<()> {
        let length = self.store.data().len();
        let rows_needed = u32::try_from(length.div_ceil(DATA_WIDTH as usize).max(1))
            .map_err(|_| Error::InvalidArgument("too many records".into()))?;
        let rows_held = self
            .data
            .as_ref()
            .map_or(0, |(texture, _)| texture.extent().height);
        let all = rows_held < rows_needed;
        if all {
            let mut rows = rows_held.max(MIN_ROWS);
            while rows < rows_needed && rows < MAX_ROWS {
                rows *= 2;
            }
            if rows < rows_needed {
                return Err(Error::InvalidArgument(
                    "the views' cells exceed the data texture".into(),
                ));
            }
            let texture = self.device.create_texture(&TextureDesc {
                label: "canvas cells",
                extent: Extent::new(DATA_WIDTH, rows),
                format: TextureFormat::Rgba32Float,
                usage: TextureUsage::SAMPLED | TextureUsage::TRANSFER_DST,
            })?;
            let bind_group = self.device.create_bind_group(&BindGroupDesc {
                label: "canvas cells",
                layout: &self.layout,
                entries: &[BindingResource::SampledTexture {
                    texture: &texture,
                    sampler: &self.sampler,
                }],
            })?;
            // Frames in flight keep the old texture alive: its destruction waits for them.
            self.data = Some((texture, bind_group));
            self.written = false;
        }
        let pending = if all { 1..length } else { self.store.pending() };
        if pending.is_empty() && !all {
            return Ok(());
        }
        let data = self.store.data();
        let (buffer, offset) = self.staging.write(
            &self.device,
            encoder.submission(),
            frame_index,
            &[&data[..1], &data[pending.clone()]],
        )?;
        self.regions.clear();
        self.regions.push((TextureRegion::new(0, 0, 1, 1), offset));
        TextureRegion::spans(
            pending.start,
            pending.end,
            DATA_WIDTH,
            offset + TEXEL,
            TEXEL,
            &mut self.regions,
        );
        let Some((texture, _)) = self.data.as_ref() else {
            return Ok(());
        };
        let from = if self.written {
            TextureState::ShaderRead
        } else {
            TextureState::Undefined
        };
        encoder.texture_barrier(texture, from, TextureState::TransferDst);
        let mut copied = Ok(());
        for &(region, at) in &self.regions {
            if copied.is_ok() {
                copied = encoder.copy_buffer_to_texture(
                    buffer,
                    BufferTexels {
                        offset: at,
                        bytes_per_row: DATA_WIDTH * 16,
                    },
                    texture,
                    region,
                );
            }
        }
        // The texture goes back to being sampled, whatever happened.
        encoder.texture_barrier(texture, TextureState::TransferDst, TextureState::ShaderRead);
        self.written = true;
        copied?;
        self.store.uploaded(all);
        Ok(())
    }
}
