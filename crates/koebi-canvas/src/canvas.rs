//! [`Canvas`]: draws a program into a view's target, one instance per cell of each outline.

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
use crate::view::View;

static CANVAS_VERT: &[u32] =
    koebi_gpu::include_spirv!(include_bytes!("../shaders/spirv/canvas.vert.spv"));
static CANVAS_FRAG: &[u32] =
    koebi_gpu::include_spirv!(include_bytes!("../shaders/spirv/canvas.frag.spv"));

/// The format of every view's target: linear light, premultiplied, in halves.
pub const TARGET_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

/// Rows of the data texture at the least: grows by doubling.
const MIN_ROWS: u32 = 4;
/// Rows of the data texture at the most: 2^24 texels, far above a frame's cells.
const MAX_ROWS: u32 = 4096;

/// The target a view renders into: `Rgba16Float`, linear light and premultiplied, sampled by
/// the interface as an image once drawn.
#[derive(Debug)]
pub struct ViewTarget {
    texture: Texture,
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

/// What one frame in flight owns: its data texture and the buffers filling it.
#[derive(Default)]
struct InFlight {
    data: Option<(Texture, BindGroup)>,
    /// Whether the data texture was ever written: its first copy comes from `Undefined`.
    written: bool,
    staging: Option<Buffer>,
    instances: Option<Buffer>,
}

/// Draws programs into views' targets ([`Canvas::render`]). Keep one per device; reuse it every
/// frame: its storage stays.
pub struct Canvas {
    device: Device,
    pipeline: RenderPipeline,
    layout: BindGroupLayout,
    sampler: Sampler,
    frames: Vec<InFlight>,
    /// The frame being drawn: its layers, records and instances.
    frame: Frame,
}

impl std::fmt::Debug for Canvas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Canvas")
            .field("instances", &self.frame.instances.len())
            .field("texels", &self.frame.data.len())
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
                stride: 64,
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
            frames: (0..FRAMES_IN_FLIGHT).map(|_| InFlight::default()).collect(),
            frame: Frame::default(),
        })
    }

    /// Records into `encoder` the drawing of `program` as `view` shows it into `target`: the
    /// upload of its cells, a pass clearing the target to transparent and drawing every layer
    /// over the ones below, and the target's transition to
    /// [`TextureState::ShaderRead`], ready to show as an image. `frame_index` must not be in
    /// use by the GPU, as the presenter guarantees.
    ///
    /// Until compositing comes, layers blended in another mode, masked or isolated draw as
    /// layers over the ones below, at their opacity.
    ///
    /// # Errors
    ///
    /// Fails if the target's size is not the view's, if `frame_index` is not below
    /// [`FRAMES_IN_FLIGHT`], or if the device cannot hold the frame's cells.
    pub fn render(
        &mut self,
        encoder: &mut CommandEncoder<'_>,
        target: &ViewTarget,
        program: &Program,
        view: &View,
        frame_index: usize,
    ) -> Result<()> {
        if target.extent() != view.extent {
            return Err(Error::InvalidArgument(format!(
                "a view of {:?} cannot draw into a target of {:?}",
                view.extent,
                target.extent()
            )));
        }
        if frame_index >= self.frames.len() {
            return Err(Error::InvalidArgument(format!("frame index {frame_index}")));
        }
        self.frame.build(program, view);
        self.upload(encoder, frame_index)?;
        let frame = &self.frames[frame_index];
        encoder.texture_barrier(
            &target.texture,
            TextureState::Undefined,
            TextureState::ColorAttachment,
        );
        {
            let mut pass =
                encoder.begin_render_pass(target.texture.render_target()?, LoadOp::Clear([0.0; 4]));
            if let (Some((_, bind_group)), Some(instances), false) = (
                &frame.data,
                &frame.instances,
                self.frame.instances.is_empty(),
            ) {
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
                pass.set_vertex_buffer(0, instances, 0);
                let count = u32::try_from(self.frame.instances.len())
                    .map_err(|_| Error::InvalidArgument("too many cells".into()))?;
                pass.draw(0..4, 0..count);
            }
        }
        encoder.texture_barrier(
            &target.texture,
            TextureState::ColorAttachment,
            TextureState::ShaderRead,
        );
        Ok(())
    }

    /// Uploads the records and instances for `frame_index`, growing what is too small.
    fn upload(&mut self, encoder: &mut CommandEncoder<'_>, frame_index: usize) -> Result<()> {
        let width = DATA_WIDTH as usize;
        let data = &mut self.frame.data;
        let rows_needed = data.len().div_ceil(width).max(1);
        // Whole rows: the copy fills them.
        data.resize(rows_needed * width, [0.0; 4]);
        let frame = &mut self.frames[frame_index];
        let rows_held = frame
            .data
            .as_ref()
            .map_or(0, |(texture, _)| texture.extent().height);
        if (rows_held as usize) < rows_needed {
            let mut rows = rows_held.max(MIN_ROWS);
            while (rows as usize) < rows_needed && rows < MAX_ROWS {
                rows *= 2;
            }
            if (rows as usize) < rows_needed {
                return Err(Error::InvalidArgument(
                    "the view's cells exceed the data texture".into(),
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
            // Frames in flight keep the old texture alive through their bind group.
            frame.data = Some((texture, bind_group));
            frame.written = false;
        }
        let bytes = size_of_val(self.frame.data.as_slice()) as u64;
        grow(
            &self.device,
            &mut frame.staging,
            "canvas staging",
            bytes,
            BufferUsage::TRANSFER_SRC,
        )?;
        let instance_bytes = (size_of::<CellInstance>() * self.frame.instances.len().max(1)) as u64;
        grow(
            &self.device,
            &mut frame.instances,
            "canvas instances",
            instance_bytes,
            BufferUsage::VERTEX,
        )?;
        let (Some(staging), Some(instances), Some((texture, _))) = (
            frame.staging.as_mut(),
            frame.instances.as_mut(),
            frame.data.as_ref(),
        ) else {
            return Ok(());
        };
        staging.write(0, &self.frame.data)?;
        if !self.frame.instances.is_empty() {
            instances.write(0, &self.frame.instances)?;
        }
        let from = if frame.written {
            TextureState::ShaderRead
        } else {
            TextureState::Undefined
        };
        encoder.texture_barrier(texture, from, TextureState::TransferDst);
        let rows = u32::try_from(rows_needed).map_err(|_| Error::InvalidArgument("rows".into()))?;
        encoder.copy_buffer_to_texture(
            staging,
            BufferTexels {
                offset: 0,
                bytes_per_row: DATA_WIDTH * 16,
            },
            texture,
            TextureRegion::new(0, 0, DATA_WIDTH, rows),
        )?;
        encoder.texture_barrier(texture, TextureState::TransferDst, TextureState::ShaderRead);
        frame.written = true;
        Ok(())
    }
}

/// `slot` grown to hold `bytes` at least: doubled, never shrunk.
fn grow(
    device: &Device,
    slot: &mut Option<Buffer>,
    label: &'static str,
    bytes: u64,
    usage: BufferUsage,
) -> Result<()> {
    if slot.as_ref().is_some_and(|buffer| buffer.size() >= bytes) {
        return Ok(());
    }
    *slot = Some(device.create_buffer(&BufferDesc {
        label,
        size: bytes.next_power_of_two().max(4096),
        usage,
        location: MemoryLocation::CpuToGpu,
    })?);
    Ok(())
}
