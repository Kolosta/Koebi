//! [`Canvas`]: draws programs into views' targets, one instance per cell of each outline, from
//! cells kept from render to render ([`crate::store`]).
//!
//! - **Data:** one texture of records for every view, updated by range: a render sends the
//!   records it added and the header, and everything only once records moved or the texture
//!   grew. Copies wait for earlier frames' draws, which are ordered before them on the queue.
//! - **Instances** are per view: each [`ViewTarget`] keeps its own, rebuilt when its program or
//!   view changes.
//! - **Buffers the CPU writes** (staging, instances) are written again only once the GPU is
//!   done with them: each is tagged with the submission that reads it.
//! - **Steady state:** a target that shows the same program revision through the same view
//!   is not drawn again, and nothing is recorded: the interface shows the last image.

use koebi_core::Extent;
use koebi_doc::eval::Program;
use koebi_gpu::{
    AddressMode, BindGroup, BindGroupDesc, BindGroupLayout, BindGroupLayoutDesc, BindingResource,
    BindingType, BlendMode, Buffer, BufferDesc, BufferTexels, BufferUsage, ColorTargetDesc,
    CommandEncoder, Device, Error, FilterMode, LoadOp, MemoryLocation, Pod, PrimitiveTopology,
    RenderPipeline, RenderPipelineDesc, Result, Sampler, SamplerDesc, ShaderStage, Texture,
    TextureDesc, TextureFormat, TextureRegion, TextureState, TextureUsage, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexStepMode,
};

use crate::cells::DATA_WIDTH;
use crate::frame::{CellInstance, Frame};
use crate::store::{CanvasStats, CellStore};
use crate::strokes::ViewStrokes;
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

/// Host-visible buffers written at a cursor, each tagged with the submission whose commands
/// read it. A buffer is written again only once the device knows that submission completed:
/// no write lands on bytes the GPU may still read, whatever the windows, presenters and
/// blocking submissions sharing the canvas. The writes of one submission follow one another
/// in its buffer.
#[derive(Debug)]
struct Stream {
    label: &'static str,
    usage: BufferUsage,
    buffers: Vec<Streamed>,
}

#[derive(Debug)]
struct Streamed {
    buffer: Buffer,
    /// Where the next write of its submission starts.
    cursor: u64,
    /// The submission whose commands read it.
    submission: u64,
}

impl Stream {
    fn new(label: &'static str, usage: BufferUsage) -> Self {
        Self {
            label,
            usage,
            buffers: Vec::new(),
        }
    }

    /// Writes `parts` one after another for the commands of `submission`, and returns the
    /// buffer with where they start: after what the submission wrote before when there is
    /// room, else in a buffer the GPU is done with, else in a new one.
    fn write<T: Pod>(
        &mut self,
        device: &Device,
        submission: u64,
        parts: &[&[T]],
    ) -> Result<(&Buffer, u64)> {
        let bytes: u64 = parts.iter().map(|part| size_of_val(*part) as u64).sum();
        let completed = device.completed_submission();
        let wanted = bytes.next_power_of_two().max(MIN_STREAM);
        let free = |streamed: &Streamed| streamed.submission <= completed;
        // Free buffers too small for this, or far too large (kept from a large upload), go.
        self.buffers.retain(|streamed| {
            !free(streamed)
                || (streamed.buffer.size() >= bytes && streamed.buffer.size() <= 16 * wanted)
        });
        let at = self
            .buffers
            .iter()
            .position(|streamed| {
                streamed.submission == submission
                    && streamed.cursor + bytes <= streamed.buffer.size()
            })
            .or_else(|| {
                let at = self.buffers.iter().position(free)?;
                let streamed = &mut self.buffers[at];
                streamed.submission = submission;
                streamed.cursor = 0;
                Some(at)
            });
        let at = if let Some(at) = at {
            at
        } else {
            self.buffers.push(Streamed {
                buffer: device.create_buffer(&BufferDesc {
                    label: self.label,
                    size: wanted,
                    usage: self.usage,
                    location: MemoryLocation::CpuToGpu,
                })?,
                cursor: 0,
                submission,
            });
            self.buffers.len() - 1
        };
        let streamed = &mut self.buffers[at];
        let start = streamed.cursor;
        let mut end = start;
        for part in parts {
            streamed.buffer.write(end, part)?;
            end += size_of_val(*part) as u64;
        }
        // Texels and instances are 16-byte aligned: so are the writes that follow.
        streamed.cursor = end.next_multiple_of(TEXEL);
        Ok((&streamed.buffer, start))
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

    /// The topmost object it shows at `pixel` (physical pixels of the target), exactly: inside
    /// the outline drawn under its fill rule, or within `tolerance` pixels of its edges (8 at
    /// most). `None` over nothing, or before the first render.
    ///
    /// The first pick after a render files what it drew by an index of their bounds over the
    /// view; picks then test only the layers near the point, and allocate nothing once its
    /// storage has grown.
    pub fn pick(&mut self, pixel: [f64; 2], tolerance: f64) -> Option<koebi_doc::NodeId> {
        // Only what a render drew whole: a failed one leaves nothing shown.
        self.shown?;
        self.frame.pick(pixel, tolerance)
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
    /// The outlines of the strokes views showed in pixels.
    strokes: ViewStrokes,
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
            strokes: ViewStrokes::default(),
            data: None,
            written: false,
            staging: Stream::new("canvas staging", BufferUsage::TRANSFER_SRC),
            regions: Vec::with_capacity(4),
        })
    }

    /// What the canvas did since [`Self::reset_stats`].
    pub fn stats(&self) -> CanvasStats {
        CanvasStats {
            outlined: self.strokes.outlined,
            ..self.store.stats
        }
    }

    /// Forgets the counts of [`Self::stats`].
    pub fn reset_stats(&mut self) {
        self.store.stats = CanvasStats::default();
        self.strokes.outlined = 0;
    }

    /// Records into `encoder` the drawing of `program` as `view` shows it into `target`, and
    /// returns whether it recorded anything:
    /// - `false` when the target already shows this program revision through this view: the
    ///   steady state, which records nothing and allocates nothing;
    /// - else the upload of the records the view needs that the GPU lacks, a pass clearing the
    ///   target to transparent and drawing every layer over the ones below, and the target's
    ///   transition to [`TextureState::ShaderRead`], ready to show as an image.
    ///
    /// Several views may render in one encoder, and one canvas may serve every window: what it
    /// writes for the GPU goes into buffers the GPU is done with ([`Device::completed_submission`]).
    /// The encoder must be submitted: the canvas counts what it recorded as done, and one whose
    /// commands were dropped (a failed submission) must be made anew with its targets, as after
    /// a device loss.
    ///
    /// Until compositing comes, layers blended in another mode, masked or isolated draw as
    /// layers over the ones below, at their opacity.
    ///
    /// # Errors
    ///
    /// Fails if the target's size is not the view's, if the view shows more outlines than the
    /// canvas holds, or if the device cannot make its buffers.
    pub fn render(
        &mut self,
        encoder: &mut CommandEncoder<'_>,
        target: &mut ViewTarget,
        program: &Program,
        view: &View,
    ) -> Result<bool> {
        if target.extent() != view.extent {
            return Err(Error::InvalidArgument(format!(
                "a view of {:?} cannot draw into a target of {:?}",
                view.extent,
                target.extent()
            )));
        }
        if target.shown == Some((program.revision(), *view)) {
            return Ok(false);
        }
        self.store.begin();
        self.strokes.begin();
        if target
            .frame
            .build(program, view, &mut self.store, &mut self.strokes)
            .is_err()
        {
            // Room for this view alone: what others kept is prepared again when they change.
            self.store.clear();
            if target
                .frame
                .build(program, view, &mut self.store, &mut self.strokes)
                .is_err()
            {
                return Err(Error::InvalidArgument(
                    "the view shows more outlines than the canvas holds".into(),
                ));
            }
        }
        self.upload(encoder)?;
        let submission = encoder.submission();
        let instances = if target.frame.instances.is_empty() {
            None
        } else {
            Some(target.instances.write(
                &self.device,
                submission,
                &[target.frame.instances.as_slice()],
            )?)
        };
        // What it showed is cleared: discarded. The barrier waits for color output, so for the
        // stages before it too: interface frames still sampling the target finish first.
        encoder.texture_barrier(
            &target.texture,
            TextureState::Undefined,
            TextureState::ColorAttachment,
        );
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
        self.store.stats.rendered += 1;
        Ok(true)
    }

    /// Uploads the header and the records the GPU lacks, growing the texture first if needed.
    fn upload(&mut self, encoder: &mut CommandEncoder<'_>) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stream_writes_again_only_what_the_gpu_is_done_with() {
        let Some(gpu) =
            koebi_testkit::gpu_or_skip("a_stream_writes_again_only_what_the_gpu_is_done_with")
        else {
            return;
        };
        let device = &gpu.device;
        let mut stream = Stream::new("test stream", BufferUsage::VERTEX);
        let texels = [[1.0_f32; 4]; 8];
        let done = device.completed_submission();
        let (first, second) = (done + 1, done + 2);
        // Two writes of one submission follow one another in one buffer.
        assert_eq!(stream.write(device, first, &[&texels[..]]).unwrap().1, 0);
        assert_eq!(
            stream
                .write(device, first, &[&texels[..3], &texels[..2]])
                .unwrap()
                .1,
            128
        );
        assert_eq!(stream.buffers[0].cursor, 208);
        // Another submission while the first may still run: a buffer of its own.
        assert_eq!(stream.write(device, second, &[&texels[..]]).unwrap().1, 0);
        assert_eq!(stream.buffers.len(), 2);
        // Once both completed, their buffers are written again from their start.
        for _ in 0..2 {
            device.submit_and_wait(|_| ()).unwrap();
        }
        assert!(device.completed_submission() >= second);
        let third = device.completed_submission() + 1;
        assert_eq!(stream.write(device, third, &[&texels[..]]).unwrap().1, 0);
        assert_eq!(
            stream.write(device, third + 1, &[&texels[..]]).unwrap().1,
            0
        );
        assert_eq!(stream.buffers.len(), 2);
        // Too large for any free buffer: a new one, the too small ones let go.
        let large = vec![[0.5_f32; 4]; 8192];
        device.submit_and_wait(|_| ()).unwrap();
        device.submit_and_wait(|_| ()).unwrap();
        let later = device.completed_submission() + 1;
        assert_eq!(
            stream.write(device, later, &[large.as_slice()]).unwrap().1,
            0
        );
        assert_eq!(stream.buffers.len(), 1);
        gpu.assert_no_validation_errors();
    }
}
