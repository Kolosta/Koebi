//! Graphics pipelines for dynamic rendering.

use std::ffi::CStr;
use std::fmt;
use std::sync::Arc;

use ash::vk;

use crate::debug::set_object_name;
use crate::device::DeviceShared;
use crate::error::VkResultExt as _;
use crate::format::{TextureFormat, VertexFormat};
use crate::garbage::Garbage;
use crate::shader::ShaderModule;
use crate::{Error, Result};

/// Push constant budget guaranteed by every Vulkan implementation.
pub const MAX_PUSH_CONSTANT_SIZE: u32 = 128;

/// A shader module and its entry point.
#[derive(Clone, Copy, Debug)]
pub struct ShaderStage<'a> {
    /// Compiled module.
    pub module: &'a ShaderModule,
    /// Entry point name, usually `c"main"`.
    pub entry_point: &'a CStr,
}

/// Whether a vertex buffer advances per vertex or per instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VertexStepMode {
    /// One element per vertex.
    Vertex,
    /// One element per instance (Koebi's 2D primitives).
    Instance,
}

/// One attribute inside a vertex buffer element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VertexAttribute {
    /// `layout(location = N)` in the vertex shader.
    pub location: u32,
    /// Data format.
    pub format: VertexFormat,
    /// Byte offset inside the element.
    pub offset: u32,
}

/// Layout of one vertex buffer binding.
#[derive(Clone, Copy, Debug)]
pub struct VertexBufferLayout<'a> {
    /// Element size in bytes.
    pub stride: u32,
    /// Per-vertex or per-instance stepping.
    pub step_mode: VertexStepMode,
    /// Attributes read from each element.
    pub attributes: &'a [VertexAttribute],
}

/// Primitive assembly mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitiveTopology {
    /// Independent triangles.
    TriangleList,
    /// Strip: every vertex after the second forms a triangle with the previous two.
    TriangleStrip,
}

/// Color blending of a target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendMode {
    /// Overwrite the destination.
    Opaque,
    /// `src + dst * (1 - src.a)` with premultiplied colors: Koebi's default for UI.
    PremultipliedAlpha,
}

/// One color attachment of a pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorTargetDesc {
    /// Attachment format (must match the render target).
    pub format: TextureFormat,
    /// Blending.
    pub blend: BlendMode,
}

/// Parameters of [`crate::Device::create_render_pipeline`].
#[derive(Clone, Copy, Debug)]
pub struct RenderPipelineDesc<'a> {
    /// Debug name.
    pub label: &'a str,
    /// Vertex stage.
    pub vertex: ShaderStage<'a>,
    /// Fragment stage.
    pub fragment: ShaderStage<'a>,
    /// Vertex buffer bindings, by slot.
    pub vertex_buffers: &'a [VertexBufferLayout<'a>],
    /// Primitive topology.
    pub topology: PrimitiveTopology,
    /// Color attachments.
    pub color_targets: &'a [ColorTargetDesc],
    /// Size of the push constant block shared by both stages (multiple of 4, ≤ 128).
    pub push_constant_size: u32,
}

/// A compiled graphics pipeline with its layout.
///
/// Viewport and scissor are dynamic state; rendering uses dynamic rendering (no render pass
/// objects). Dropping is always safe (deferred destruction).
pub struct RenderPipeline {
    device: Arc<DeviceShared>,
    raw: vk::Pipeline,
    layout: vk::PipelineLayout,
    push_constant_size: u32,
}

impl RenderPipeline {
    pub(crate) fn new(device: &Arc<DeviceShared>, desc: &RenderPipelineDesc<'_>) -> Result<Self> {
        validate(desc)?;
        let push_ranges = [vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: desc.push_constant_size,
        }];
        let ranges: &[vk::PushConstantRange] = if desc.push_constant_size > 0 {
            &push_ranges
        } else {
            &[]
        };
        let layout_info = vk::PipelineLayoutCreateInfo::default().push_constant_ranges(ranges);
        // SAFETY: valid device and create info.
        let layout = unsafe { device.raw.create_pipeline_layout(&layout_info, None) }
            .ctx("vkCreatePipelineLayout")?;

        let stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(desc.vertex.module.handle())
                .name(desc.vertex.entry_point),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(desc.fragment.module.handle())
                .name(desc.fragment.entry_point),
        ];
        let bindings: Vec<_> = desc
            .vertex_buffers
            .iter()
            .zip(0_u32..)
            .map(|(layout, binding)| vk::VertexInputBindingDescription {
                binding,
                stride: layout.stride,
                input_rate: match layout.step_mode {
                    VertexStepMode::Vertex => vk::VertexInputRate::VERTEX,
                    VertexStepMode::Instance => vk::VertexInputRate::INSTANCE,
                },
            })
            .collect();
        let attributes: Vec<_> = desc
            .vertex_buffers
            .iter()
            .zip(0_u32..)
            .flat_map(|(layout, binding)| {
                layout
                    .attributes
                    .iter()
                    .map(move |a| vk::VertexInputAttributeDescription {
                        location: a.location,
                        binding,
                        format: a.format.to_vk(),
                        offset: a.offset,
                    })
            })
            .collect();
        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&bindings)
            .vertex_attribute_descriptions(&attributes);
        let input_assembly =
            vk::PipelineInputAssemblyStateCreateInfo::default().topology(match desc.topology {
                PrimitiveTopology::TriangleList => vk::PrimitiveTopology::TRIANGLE_LIST,
                PrimitiveTopology::TriangleStrip => vk::PrimitiveTopology::TRIANGLE_STRIP,
            });
        let viewport = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);
        let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .line_width(1.0);
        let multisample = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);
        let blend_attachments: Vec<_> = desc
            .color_targets
            .iter()
            .map(|target| blend_state(target.blend))
            .collect();
        let color_blend =
            vk::PipelineColorBlendStateCreateInfo::default().attachments(&blend_attachments);
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
        let color_formats: Vec<_> = desc
            .color_targets
            .iter()
            .map(|t| t.format.to_vk())
            .collect();
        let mut rendering =
            vk::PipelineRenderingCreateInfo::default().color_attachment_formats(&color_formats);

        let info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport)
            .rasterization_state(&rasterization)
            .multisample_state(&multisample)
            .color_blend_state(&color_blend)
            .dynamic_state(&dynamic)
            .layout(layout)
            .push_next(&mut rendering);
        // SAFETY: every referenced object is alive; dynamic rendering is enabled on the device.
        let created = unsafe {
            device
                .raw
                .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)
        };
        let raw = match created {
            Ok(pipelines) if !pipelines.is_empty() => pipelines[0],
            failed => {
                // SAFETY: the layout is unused and destroyed once on this error path.
                unsafe { device.raw.destroy_pipeline_layout(layout, None) };
                let result = failed.err().map_or(vk::Result::ERROR_UNKNOWN, |(_, r)| r);
                return Err(Error::from_vk("vkCreateGraphicsPipelines", result));
            }
        };
        set_object_name(device, raw, desc.label);
        set_object_name(device, layout, desc.label);
        Ok(Self {
            device: Arc::clone(device),
            raw,
            layout,
            push_constant_size: desc.push_constant_size,
        })
    }

    /// The raw pipeline handle.
    pub fn handle(&self) -> vk::Pipeline {
        self.raw
    }

    /// The raw pipeline layout handle.
    pub fn layout(&self) -> vk::PipelineLayout {
        self.layout
    }

    /// Size of the push constant block.
    pub fn push_constant_size(&self) -> u32 {
        self.push_constant_size
    }
}

impl fmt::Debug for RenderPipeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RenderPipeline")
            .field("handle", &self.raw)
            .field("push_constant_size", &self.push_constant_size)
            .finish_non_exhaustive()
    }
}

impl Drop for RenderPipeline {
    fn drop(&mut self) {
        self.device.defer_destroy(Garbage::Pipeline {
            pipeline: self.raw,
            layout: self.layout,
        });
    }
}

fn validate(desc: &RenderPipelineDesc<'_>) -> Result<()> {
    let fail = |message: &str| Err(Error::InvalidArgument(format!("{}: {message}", desc.label)));
    if desc.color_targets.is_empty() {
        return fail("at least one color target is required");
    }
    if !desc.push_constant_size.is_multiple_of(4)
        || desc.push_constant_size > MAX_PUSH_CONSTANT_SIZE
    {
        return fail("push constant size must be a multiple of 4 and at most 128 bytes");
    }
    for layout in desc.vertex_buffers {
        for attribute in layout.attributes {
            if attribute.offset + attribute.format.size() > layout.stride {
                return fail("vertex attribute exceeds its buffer stride");
            }
        }
    }
    Ok(())
}

fn blend_state(mode: BlendMode) -> vk::PipelineColorBlendAttachmentState {
    let state = vk::PipelineColorBlendAttachmentState::default()
        .color_write_mask(vk::ColorComponentFlags::RGBA);
    match mode {
        BlendMode::Opaque => state.blend_enable(false),
        BlendMode::PremultipliedAlpha => state
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(vk::BlendOp::ADD),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premultiplied_blend_uses_one_and_one_minus_src_alpha() {
        let state = blend_state(BlendMode::PremultipliedAlpha);
        assert_eq!(state.blend_enable, vk::TRUE);
        assert_eq!(state.src_color_blend_factor, vk::BlendFactor::ONE);
        assert_eq!(
            state.dst_color_blend_factor,
            vk::BlendFactor::ONE_MINUS_SRC_ALPHA
        );
        assert_eq!(blend_state(BlendMode::Opaque).blend_enable, vk::FALSE);
    }
}
