//! Running a view's script on the GPU (ADR 0029): layers drawn over the current target, groups
//! in targets of their own, composited onto the target below.
//!
//! - **Targets:** the view's own at the bottom; above it, group targets of the view's size,
//!   `Rgba16Float`, linear and premultiplied, pooled by the view ([`Groups`]): taken at a
//!   group's start, given back at its end, kept from frame to frame. A frame of a program it
//!   drew before makes no target and no bind group.
//! - **Areas:** a group renders, clears and composites only the pixels it may draw (the bounds
//!   of its layers and of its groups, [`crate::frame`]): a small group costs its size, not the
//!   view's.
//! - **Compositing:** Normal and Erase by fixed-function blending (over, destination-out) onto
//!   the target below. The other modes by `shaders/composite.frag`: the backdrop's pixels under
//!   the group are copied aside, and the result, read from the copy and the group, is written
//!   over them. A mask's content is drawn over the target below where its coverage is.
//! - **Empty groups** are skipped: a transparent source changes nothing in any mode, and a
//!   transparent coverage shows nothing, or all of its content when inverted.

use std::ops::Range;

use koebi_core::Extent;
use koebi_core::hash::IdMap;
use koebi_doc::BlendMode;
use koebi_gpu::{
    AddressMode, BindGroup, BindGroupDesc, BindGroupLayout, BindGroupLayoutDesc, BindingResource,
    BindingType, BlendMode as Blending, Buffer, ColorTargetDesc, CommandEncoder, Device, Error,
    FilterMode, LoadOp, Pod, PrimitiveTopology, RenderPass, RenderPipeline, RenderPipelineDesc,
    Result, Sampler, SamplerDesc, ShaderStage, Texture, TextureDesc, TextureRegion, TextureState,
    TextureUsage,
};

use crate::canvas::TARGET_FORMAT;
use crate::frame::{EMPTY, Op, intersection};

static COMPOSITE_VERT: &[u32] =
    koebi_gpu::include_spirv!(include_bytes!("../shaders/spirv/composite.vert.spv"));
static COMPOSITE_FRAG: &[u32] =
    koebi_gpu::include_spirv!(include_bytes!("../shaders/spirv/composite.frag.spv"));

/// Push constants of the composite pipelines. Must match `Constants` in
/// `shaders/composite.frag`.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct Constants {
    mode: u32,
    opacity: f32,
    flags: u32,
    padding: u32,
}

// SAFETY: `repr(C)` with four 4-byte fields: 16 bytes, no padding, any bit pattern is valid.
unsafe impl Pod for Constants {}

/// Push constants of the canvas pipeline. Must match `Constants` in `shaders/canvas.vert`.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct LayerConstants {
    pub(crate) target_px: [f32; 2],
    pub(crate) padding: [f32; 2],
}

// SAFETY: `repr(C)` with four `f32`: 16 bytes, no padding, any bit pattern is valid.
unsafe impl Pod for LayerConstants {}

/// Blend the source with the backdrop it reads (the modes other than Normal and Erase).
const BLEND: u32 = 1;
/// The source shows where the mask's alpha is.
const MASKED: u32 = 2;
/// The mask inverted.
const INVERTED: u32 = 4;

/// The composite pipelines, one per way the result is written.
pub(crate) struct Compositor {
    layout: BindGroupLayout,
    sampler: Sampler,
    /// The source over the target: Normal, and masks.
    over: RenderPipeline,
    /// What the source covers erased: Erase.
    erase: RenderPipeline,
    /// The result written as is, into a target of its own: the other modes.
    replace: RenderPipeline,
}

impl Compositor {
    pub(crate) fn new(device: &Device) -> Result<Self> {
        let layout = device.create_bind_group_layout(&BindGroupLayoutDesc {
            label: "canvas composite",
            entries: &[
                BindingType::SampledTexture,
                BindingType::SampledTexture,
                BindingType::SampledTexture,
            ],
        })?;
        let vertex = device.create_shader_module("composite.vert", COMPOSITE_VERT)?;
        let fragment = device.create_shader_module("composite.frag", COMPOSITE_FRAG)?;
        let pipeline = |label: &str, blend: Blending| {
            device.create_render_pipeline(&RenderPipelineDesc {
                label,
                vertex: ShaderStage {
                    module: &vertex,
                    entry_point: c"main",
                },
                fragment: ShaderStage {
                    module: &fragment,
                    entry_point: c"main",
                },
                vertex_buffers: &[],
                topology: PrimitiveTopology::TriangleList,
                color_targets: &[ColorTargetDesc {
                    format: TARGET_FORMAT,
                    blend,
                }],
                push_constant_size: 16,
                bind_group_layouts: &[&layout],
            })
        };
        Ok(Self {
            over: pipeline("canvas composite over", Blending::PremultipliedAlpha)?,
            erase: pipeline("canvas composite erase", Blending::DestinationOut)?,
            replace: pipeline("canvas composite blend", Blending::Opaque)?,
            sampler: device.create_sampler(&SamplerDesc {
                label: "canvas groups",
                // Read with texelFetch, never filtered.
                filter: FilterMode::Nearest,
                address_mode: AddressMode::ClampToEdge,
            })?,
            layout,
        })
    }
}

/// A target of the stack: the view's own, or group `n`'s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Slot {
    View,
    Group(u8),
}

/// The group targets of one view, pooled (see the module), and the bind groups that read them.
#[derive(Debug, Default)]
pub(crate) struct Groups {
    textures: Vec<Texture>,
    free: Vec<u8>,
    /// Whether each target was drawn since it was taken.
    started: Vec<bool>,
    /// The pixels each target's group may draw, set when it is taken.
    areas: Vec<TextureRegion>,
    /// Bind groups by the targets they read: source, backdrop, mask.
    bindings: IdMap<[u8; 3], BindGroup>,
    /// The stack of targets while a script runs: kept, its storage with it.
    stack: Vec<Slot>,
    /// Whether the view's target was drawn this frame.
    view_started: bool,
}

/// The most groups a view nests: more than any document's depth, against corrupt programs.
const MAX_GROUPS: usize = 64;

impl Groups {
    /// A group target for a group drawing `area`, transparent there once begun: one given
    /// back, else a new one.
    fn take(&mut self, device: &Device, extent: Extent, area: TextureRegion) -> Result<u8> {
        let group = if let Some(group) = self.free.pop() {
            group
        } else {
            if self.textures.len() >= MAX_GROUPS {
                return Err(Error::InvalidArgument(
                    "the view nests more groups than the canvas draws".into(),
                ));
            }
            self.textures.push(device.create_texture(&TextureDesc {
                label: "canvas group",
                extent,
                format: TARGET_FORMAT,
                usage: TextureUsage::COLOR_ATTACHMENT
                    | TextureUsage::SAMPLED
                    | TextureUsage::TRANSFER_SRC
                    | TextureUsage::TRANSFER_DST,
            })?);
            self.started.push(false);
            self.areas.push(area);
            u8::try_from(self.textures.len() - 1)
                .map_err(|_| Error::InvalidArgument("too many groups".into()))?
        };
        self.started[usize::from(group)] = false;
        self.areas[usize::from(group)] = area;
        Ok(group)
    }

    fn give_back(&mut self, slot: Slot) {
        if let Slot::Group(group) = slot {
            self.free.push(group);
        }
    }

    fn started(&self, slot: Slot) -> bool {
        match slot {
            Slot::View => self.view_started,
            Slot::Group(group) => self.started[usize::from(group)],
        }
    }

    fn area(&self, group: u8) -> TextureRegion {
        self.areas[usize::from(group)]
    }

    /// Makes the bind group reading `source`, `backdrop` and `mask`, once.
    fn ensure_bind_group(
        &mut self,
        device: &Device,
        compositor: &Compositor,
        reads: [u8; 3],
    ) -> Result<()> {
        if !self.bindings.contains_key(&reads) {
            let entries = reads.map(|group| BindingResource::SampledTexture {
                texture: &self.textures[usize::from(group)],
                sampler: &compositor.sampler,
            });
            let bind_group = device.create_bind_group(&BindGroupDesc {
                label: "canvas composite",
                layout: &compositor.layout,
                entries: &entries,
            })?;
            self.bindings.insert(reads, bind_group);
        }
        Ok(())
    }
}

/// What drawing layers takes: the canvas pipeline, its records and the view's instances.
pub(crate) struct Layers<'a> {
    pub(crate) pipeline: &'a RenderPipeline,
    pub(crate) records: &'a BindGroup,
    pub(crate) constants: LayerConstants,
    /// The instance buffer and where the view's instances start in it; `None` without any.
    pub(crate) instances: Option<(&'a Buffer, u64)>,
    /// The instances of each layer.
    pub(crate) ranges: &'a [Range<u32>],
    /// The area of each group, in the order the script starts them.
    pub(crate) areas: &'a [TextureRegion],
}

/// Everything a script runs on.
pub(crate) struct Run<'a, 'e> {
    pub(crate) device: &'a Device,
    pub(crate) encoder: &'a mut CommandEncoder<'e>,
    pub(crate) compositor: &'a Compositor,
    pub(crate) groups: &'a mut Groups,
    pub(crate) view: &'a Texture,
    pub(crate) layers: Layers<'a>,
}

impl Run<'_, '_> {
    /// Runs `script`, then leaves the view's target in [`TextureState::ShaderRead`].
    pub(crate) fn script(mut self, script: &[Op]) -> Result<()> {
        self.groups.stack.clear();
        self.groups.stack.push(Slot::View);
        self.groups.view_started = false;
        let mut pending: Option<Range<u32>> = None;
        let mut pushed = 0;
        for &op in script {
            if let Op::Draw(layer) = op {
                let range = self.layers.ranges.get(layer).cloned().unwrap_or(0..0);
                if range.is_empty() {
                    continue;
                }
                pending = match pending {
                    Some(drawn) if drawn.end == range.start => Some(drawn.start..range.end),
                    Some(drawn) => {
                        self.draw(drawn)?;
                        Some(range)
                    }
                    None => Some(range),
                };
                continue;
            }
            if let Some(drawn) = pending.take() {
                self.draw(drawn)?;
            }
            match op {
                Op::Draw(_) => {}
                Op::Push => {
                    let area = self.layers.areas.get(pushed).copied().unwrap_or(EMPTY);
                    pushed += 1;
                    let extent = self.view.extent();
                    let group = self.groups.take(self.device, extent, area)?;
                    self.groups.stack.push(Slot::Group(group));
                }
                Op::Pop { mode, opacity } => self.pop(mode, opacity)?,
                Op::PopMask { invert } => self.pop_mask(invert)?,
            }
        }
        if let Some(drawn) = pending.take() {
            self.draw(drawn)?;
        }
        // Whatever the script left open is dropped, and a view that drew nothing cleared.
        while self.groups.stack.len() > 1 {
            if let Some(slot) = self.groups.stack.pop() {
                self.groups.give_back(slot);
            }
        }
        if !self.groups.view_started {
            let whole = TextureRegion::whole(self.view.extent());
            drop(begin(
                self.encoder,
                self.view,
                &mut self.groups.view_started,
                whole,
                whole,
            )?);
        }
        self.encoder.texture_barrier(
            self.view,
            TextureState::ColorAttachment,
            TextureState::ShaderRead,
        );
        Ok(())
    }

    fn current(&self) -> Slot {
        self.groups.stack.last().copied().unwrap_or(Slot::View)
    }

    /// The pixels `slot` may hold: the whole view, or its group's area.
    fn own_area(&self, slot: Slot) -> TextureRegion {
        match slot {
            Slot::View => TextureRegion::whole(self.view.extent()),
            Slot::Group(group) => self.groups.area(group),
        }
    }

    /// Makes group `group`'s target readable by shaders.
    fn sampled(&mut self, group: u8) {
        self.encoder.texture_barrier(
            &self.groups.textures[usize::from(group)],
            TextureState::ColorAttachment,
            TextureState::ShaderRead,
        );
    }

    /// Draws `instances` of the view's layers over the current target.
    fn draw(&mut self, instances: Range<u32>) -> Result<()> {
        let slot = self.current();
        let area = self.own_area(slot);
        let Layers {
            pipeline,
            records,
            constants,
            instances: buffer,
            ..
        } = self.layers;
        let Some((buffer, offset)) = buffer else {
            return Ok(());
        };
        let (texture, started) = target(self.view, self.groups, slot);
        let Some(mut pass) = begin(self.encoder, texture, started, area, area)? else {
            return Ok(());
        };
        pass.set_pipeline(pipeline);
        pass.set_bind_group(pipeline, 0, records)?;
        pass.set_push_constants(pipeline, &constants)?;
        pass.set_vertex_buffer(0, buffer, offset);
        pass.draw(0..4, instances);
        Ok(())
    }

    /// Composites the reads `reads` onto `slot` within `area` through `pipeline`; the targets
    /// read are already sampled.
    fn composite(
        &mut self,
        slot: Slot,
        pipeline: Which,
        reads: [u8; 3],
        constants: Constants,
        area: TextureRegion,
    ) -> Result<()> {
        self.groups
            .ensure_bind_group(self.device, self.compositor, reads)?;
        let own = self.own_area(slot);
        let pipeline = match pipeline {
            Which::Over => &self.compositor.over,
            Which::Erase => &self.compositor.erase,
            Which::Replace => &self.compositor.replace,
        };
        let Groups {
            textures,
            started,
            bindings,
            view_started,
            ..
        } = &mut *self.groups;
        let bind_group = bindings
            .get(&reads)
            .ok_or_else(|| Error::InvalidArgument("composite bind group missing".into()))?;
        let (texture, started) = match slot {
            Slot::View => (self.view, view_started),
            Slot::Group(group) => (
                &textures[usize::from(group)],
                &mut started[usize::from(group)],
            ),
        };
        let Some(mut pass) = begin(self.encoder, texture, started, own, area)? else {
            return Ok(());
        };
        pass.set_pipeline(pipeline);
        pass.set_bind_group(pipeline, 0, bind_group)?;
        pass.set_push_constants(pipeline, &constants)?;
        pass.draw(0..3, 0..1);
        Ok(())
    }

    /// Ends a group: composites it onto the target below in `mode` at `opacity`, within the
    /// group's area.
    fn pop(&mut self, mode: BlendMode, opacity: f32) -> Result<()> {
        let Some(Slot::Group(source)) = self.groups.stack.pop() else {
            return Err(Error::InvalidArgument("a group ended twice".into()));
        };
        let below = self.current();
        let area = self.groups.area(source);
        let constants = Constants {
            mode: mode as u32,
            opacity,
            flags: 0,
            padding: 0,
        };
        // An empty group changes nothing; Erase of or onto nothing neither.
        let skip = !self.groups.started(Slot::Group(source))
            || (mode == BlendMode::Erase && !self.groups.started(below));
        if !skip {
            self.sampled(source);
            match (mode, below) {
                (BlendMode::Normal, _) => {
                    self.composite(below, Which::Over, [source; 3], constants, area)?;
                }
                (BlendMode::Erase, _) => {
                    self.composite(below, Which::Erase, [source; 3], constants, area)?;
                }
                // Over nothing yet, a blend is the source over transparent.
                (_, _) if !self.groups.started(below) => {
                    self.composite(below, Which::Over, [source; 3], constants, area)?;
                }
                (_, Slot::Group(backdrop)) => {
                    self.blend(source, backdrop, area, constants)?;
                }
                (_, Slot::View) => {
                    return Err(Error::InvalidArgument(
                        "a blend onto the view's own target".into(),
                    ));
                }
            }
        }
        self.groups.give_back(Slot::Group(source));
        Ok(())
    }

    /// Blends group `source` onto group `backdrop` within `area`: the backdrop's pixels there
    /// copied aside, then the result written over them, read from the copy and the source.
    fn blend(
        &mut self,
        source: u8,
        backdrop: u8,
        area: TextureRegion,
        constants: Constants,
    ) -> Result<()> {
        let area = intersection(area, self.groups.area(backdrop));
        if area.is_empty() {
            return Ok(());
        }
        let extent = self.view.extent();
        let copy = self.groups.take(self.device, extent, area)?;
        let (from, to) = (
            &self.groups.textures[usize::from(backdrop)],
            &self.groups.textures[usize::from(copy)],
        );
        self.encoder.texture_barrier(
            from,
            TextureState::ColorAttachment,
            TextureState::TransferSrc,
        );
        self.encoder
            .texture_barrier(to, TextureState::Undefined, TextureState::TransferDst);
        let copied = self.encoder.copy_texture_to_texture(from, to, area);
        // Both go on to their next use, whatever the copy did.
        self.encoder
            .texture_barrier(to, TextureState::TransferDst, TextureState::ShaderRead);
        self.encoder.texture_barrier(
            from,
            TextureState::TransferSrc,
            TextureState::ColorAttachment,
        );
        copied?;
        let blended = self.composite(
            Slot::Group(backdrop),
            Which::Replace,
            [source, copy, source],
            Constants {
                flags: BLEND,
                ..constants
            },
            area,
        );
        self.groups.give_back(Slot::Group(copy));
        blended
    }

    /// Ends a mask: its content composited over the target below where its coverage is.
    fn pop_mask(&mut self, invert: bool) -> Result<()> {
        let (Some(Slot::Group(mask)), Some(Slot::Group(content))) =
            (self.groups.stack.pop(), self.groups.stack.pop())
        else {
            return Err(Error::InvalidArgument("a mask ended outside one".into()));
        };
        let below = self.current();
        let shown = self.groups.started(Slot::Group(content));
        let covered = self.groups.started(Slot::Group(mask));
        if shown && (covered || invert) {
            self.sampled(content);
            let drawn = self.groups.area(content);
            let (reads, flags, area) = if covered {
                self.sampled(mask);
                let area = if invert {
                    drawn
                } else {
                    intersection(drawn, self.groups.area(mask))
                };
                (
                    [content, content, mask],
                    MASKED | if invert { INVERTED } else { 0 },
                    area,
                )
            } else {
                ([content; 3], 0, drawn)
            };
            self.composite(
                below,
                Which::Over,
                reads,
                Constants {
                    mode: 0,
                    opacity: 1.0,
                    flags,
                    padding: 0,
                },
                area,
            )?;
        }
        self.groups.give_back(Slot::Group(mask));
        self.groups.give_back(Slot::Group(content));
        Ok(())
    }
}

/// The texture of `slot` and whether it was drawn this frame.
fn target<'a>(
    view: &'a Texture,
    groups: &'a mut Groups,
    slot: Slot,
) -> (&'a Texture, &'a mut bool) {
    match slot {
        Slot::View => (view, &mut groups.view_started),
        Slot::Group(group) => (
            &groups.textures[usize::from(group)],
            &mut groups.started[usize::from(group)],
        ),
    }
}

/// Begins a pass on `texture` rendering its target's area `own`, drawing within `area` of it:
/// `own` cleared the first time this frame (`started` false), kept after. `None` when there is
/// nothing to draw.
fn begin<'e>(
    encoder: &'e mut CommandEncoder<'_>,
    texture: &Texture,
    started: &mut bool,
    own: TextureRegion,
    area: TextureRegion,
) -> Result<Option<RenderPass<'e>>> {
    let area = intersection(own, area);
    if area.is_empty() {
        return Ok(None);
    }
    // A target taken anew is cleared: discarded. Its last use (sampled, copied, or the
    // interface's image) runs at earlier stages than color output, which the barrier waits for.
    let (from, load) = if *started {
        (TextureState::ColorAttachment, LoadOp::Load)
    } else {
        (TextureState::Undefined, LoadOp::Clear([0.0; 4]))
    };
    encoder.texture_barrier(texture, from, TextureState::ColorAttachment);
    *started = true;
    let mut pass = encoder.begin_render_pass_in(texture.render_target()?, load, own)?;
    #[allow(
        clippy::cast_possible_wrap,
        reason = "an offset inside a target, far below 2^31"
    )]
    pass.set_scissor(area.x as i32, area.y as i32, area.width, area.height);
    Ok(Some(pass))
}

/// Which composite pipeline.
#[derive(Clone, Copy, Debug)]
enum Which {
    Over,
    Erase,
    Replace,
}
