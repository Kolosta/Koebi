//! Bind groups: the resources a pipeline's shaders read, as descriptor sets.
//!
//! A [`BindGroupLayout`] declares the bindings of one descriptor set (`layout(set = S,
//! binding = N)` in GLSL, binding `N` being the layout's entry `N`); a pipeline lists the
//! layouts of its sets; a [`BindGroup`] fills a layout with resources and is bound in a render
//! pass ([`crate::RenderPass::set_bind_group`]).
//!
//! Each bind group owns a descriptor pool sized for its one set, so creating and dropping
//! groups never contends on a shared pool, and keeps its resources alive: dropping a texture
//! that a bind group still references cannot free it under the GPU. Groups are immutable;
//! when a resource is replaced (an atlas that grows), create a new group.

use std::fmt;
use std::sync::Arc;

use ash::vk;

use crate::debug::set_object_name;
use crate::device::DeviceShared;
use crate::error::VkResultExt as _;
use crate::garbage::Garbage;
use crate::sampler::{Sampler, SamplerShared};
use crate::texture::{Texture, TextureShared, TextureUsage};
use crate::{Error, Result};

/// The kind of one binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BindingType {
    /// A texture read through a sampler by the fragment stage (`sampler2D`).
    SampledTexture,
}

impl BindingType {
    fn descriptor_type(self) -> vk::DescriptorType {
        match self {
            Self::SampledTexture => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        }
    }

    fn stages(self) -> vk::ShaderStageFlags {
        match self {
            Self::SampledTexture => vk::ShaderStageFlags::FRAGMENT,
        }
    }
}

/// Parameters of [`crate::Device::create_bind_group_layout`].
#[derive(Clone, Copy, Debug)]
pub struct BindGroupLayoutDesc<'a> {
    /// Debug name.
    pub label: &'a str,
    /// The bindings, by binding number (at least one).
    pub entries: &'a [BindingType],
}

/// The bindings of one descriptor set.
///
/// Two layouts with the same entries are interchangeable: a bind group of either can be bound
/// to a pipeline built with the other.
pub struct BindGroupLayout {
    shared: Arc<BindGroupLayoutShared>,
}

pub(crate) struct BindGroupLayoutShared {
    pub(crate) device: Arc<DeviceShared>,
    pub(crate) raw: vk::DescriptorSetLayout,
    pub(crate) entries: Vec<BindingType>,
}

impl BindGroupLayout {
    pub(crate) fn new(device: &Arc<DeviceShared>, desc: &BindGroupLayoutDesc<'_>) -> Result<Self> {
        if desc.entries.is_empty() {
            return Err(Error::InvalidArgument(format!(
                "bind group layout {:?} has no binding",
                desc.label
            )));
        }
        let bindings: Vec<_> = desc
            .entries
            .iter()
            .zip(0_u32..)
            .map(|(entry, binding)| {
                vk::DescriptorSetLayoutBinding::default()
                    .binding(binding)
                    .descriptor_type(entry.descriptor_type())
                    .descriptor_count(1)
                    .stage_flags(entry.stages())
            })
            .collect();
        let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
        // SAFETY: valid device and create info.
        let raw = unsafe { device.raw.create_descriptor_set_layout(&info, None) }
            .ctx("vkCreateDescriptorSetLayout")?;
        set_object_name(device, raw, desc.label);
        Ok(Self {
            shared: Arc::new(BindGroupLayoutShared {
                device: Arc::clone(device),
                raw,
                entries: desc.entries.to_vec(),
            }),
        })
    }

    /// The bindings, by binding number.
    pub fn entries(&self) -> &[BindingType] {
        &self.shared.entries
    }

    /// The raw descriptor set layout handle.
    pub fn handle(&self) -> vk::DescriptorSetLayout {
        self.shared.raw
    }

    pub(crate) fn shared(&self) -> &Arc<BindGroupLayoutShared> {
        &self.shared
    }
}

impl fmt::Debug for BindGroupLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BindGroupLayout")
            .field("handle", &self.shared.raw)
            .field("entries", &self.shared.entries)
            .finish_non_exhaustive()
    }
}

impl Drop for BindGroupLayoutShared {
    fn drop(&mut self) {
        // Descriptor sets allocated with a layout may access it when they are bound, so the
        // layout waits for the GPU like the sets themselves.
        self.device
            .defer_destroy(Garbage::BindGroupLayout { layout: self.raw });
    }
}

/// The resource bound to one binding of a [`BindGroup`].
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum BindingResource<'a> {
    /// For [`BindingType::SampledTexture`]: a texture created with
    /// [`TextureUsage::SAMPLED`], read through `sampler`. Shaders read it in
    /// [`crate::TextureState::ShaderRead`].
    SampledTexture {
        /// The texture.
        texture: &'a Texture,
        /// How it is read.
        sampler: &'a Sampler,
    },
}

/// Parameters of [`crate::Device::create_bind_group`].
#[derive(Clone, Copy, Debug)]
pub struct BindGroupDesc<'a> {
    /// Debug name.
    pub label: &'a str,
    /// The layout to fill.
    pub layout: &'a BindGroupLayout,
    /// One resource per entry of the layout, in binding order.
    pub entries: &'a [BindingResource<'a>],
}

/// The resources a bind group keeps alive.
#[derive(Default)]
struct Retained {
    textures: Vec<Arc<TextureShared>>,
    samplers: Vec<Arc<SamplerShared>>,
}

/// A descriptor set filled with resources, bound to pipelines in render passes.
///
/// Keeps its layout and resources alive; dropping is always safe (deferred destruction).
pub struct BindGroup {
    device: Arc<DeviceShared>,
    pool: vk::DescriptorPool,
    raw: vk::DescriptorSet,
    layout: Arc<BindGroupLayoutShared>,
    /// Held so that the GPU objects outlive every use of the set.
    resources: Retained,
}

impl BindGroup {
    pub(crate) fn new(device: &Arc<DeviceShared>, desc: &BindGroupDesc<'_>) -> Result<Self> {
        let layout = desc.layout.shared();
        let resources = validate(device, desc)?;

        let sampled = desc
            .layout
            .entries()
            .iter()
            .filter(|entry| **entry == BindingType::SampledTexture)
            .count();
        let sizes: Vec<_> = [(
            BindingType::SampledTexture,
            u32::try_from(sampled).unwrap_or(u32::MAX),
        )]
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .map(|(kind, count)| vk::DescriptorPoolSize {
            ty: kind.descriptor_type(),
            descriptor_count: count,
        })
        .collect();
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(1)
            .pool_sizes(&sizes);
        // SAFETY: valid device and create info.
        let pool = unsafe { device.raw.create_descriptor_pool(&pool_info, None) }
            .ctx("vkCreateDescriptorPool")?;
        let destroy_pool = || {
            // SAFETY: the pool and its set are unused and destroyed once on error paths.
            unsafe { device.raw.destroy_descriptor_pool(pool, None) };
        };
        let layouts = [layout.raw];
        let allocate = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(pool)
            .set_layouts(&layouts);
        // SAFETY: the pool was sized for exactly one set of this layout.
        let raw = match unsafe { device.raw.allocate_descriptor_sets(&allocate) } {
            Ok(sets) if !sets.is_empty() => sets[0],
            failed => {
                destroy_pool();
                let result = failed.err().unwrap_or(vk::Result::ERROR_UNKNOWN);
                return Err(Error::from_vk("vkAllocateDescriptorSets", result));
            }
        };

        let images: Vec<_> = desc
            .entries
            .iter()
            .map(|entry| match entry {
                BindingResource::SampledTexture { texture, sampler } => vk::DescriptorImageInfo {
                    sampler: sampler.handle(),
                    image_view: texture.view_handle(),
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                },
            })
            .collect();
        let writes: Vec<_> = images
            .iter()
            .zip(desc.layout.entries())
            .zip(0_u32..)
            .map(|((image, entry), binding)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(raw)
                    .dst_binding(binding)
                    .descriptor_type(entry.descriptor_type())
                    .image_info(core::slice::from_ref(image))
            })
            .collect();
        // SAFETY: the set is not in use by any command buffer (just allocated); every view and
        // sampler is alive, belongs to this device and matches its binding's type (validated).
        unsafe { device.raw.update_descriptor_sets(&writes, &[]) };
        set_object_name(device, raw, desc.label);
        set_object_name(device, pool, desc.label);

        Ok(Self {
            device: Arc::clone(device),
            pool,
            raw,
            layout: Arc::clone(layout),
            resources,
        })
    }

    /// The bindings of the group's layout.
    pub fn entries(&self) -> &[BindingType] {
        &self.layout.entries
    }

    /// The raw descriptor set handle.
    pub fn handle(&self) -> vk::DescriptorSet {
        self.raw
    }

    pub(crate) fn layout(&self) -> &Arc<BindGroupLayoutShared> {
        &self.layout
    }
}

impl fmt::Debug for BindGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BindGroup")
            .field("handle", &self.raw)
            .field("entries", &self.layout.entries)
            .field("textures", &self.resources.textures.len())
            .field("samplers", &self.resources.samplers.len())
            .finish_non_exhaustive()
    }
}

impl Drop for BindGroup {
    fn drop(&mut self) {
        self.device
            .defer_destroy(Garbage::BindGroup { pool: self.pool });
    }
}

/// Checks that `desc` fills its layout with live resources of this device, and returns the
/// resources to keep alive.
fn validate(device: &Arc<DeviceShared>, desc: &BindGroupDesc<'_>) -> Result<Retained> {
    let fail = |message: String| Err(Error::InvalidArgument(format!("{}: {message}", desc.label)));
    let layout = desc.layout.shared();
    if !Arc::ptr_eq(&layout.device, device) {
        return fail("the layout belongs to another device".into());
    }
    if desc.entries.len() != layout.entries.len() {
        return fail(format!(
            "{} resource(s) for a layout of {} binding(s)",
            desc.entries.len(),
            layout.entries.len()
        ));
    }
    let mut retained = Retained::default();
    for (binding, (resource, entry)) in desc.entries.iter().zip(&layout.entries).enumerate() {
        match (resource, entry) {
            (BindingResource::SampledTexture { texture, sampler }, BindingType::SampledTexture) => {
                if !texture.usage().contains(TextureUsage::SAMPLED) {
                    return fail(format!(
                        "binding {binding}: the texture lacks SAMPLED usage"
                    ));
                }
                let (texture, sampler) = (texture.shared(), sampler.shared());
                if !Arc::ptr_eq(&texture.device, device) || !Arc::ptr_eq(&sampler.device, device) {
                    return fail(format!(
                        "binding {binding}: the resource belongs to another device"
                    ));
                }
                retained.textures.push(Arc::clone(texture));
                retained.samplers.push(Arc::clone(sampler));
            }
        }
    }
    Ok(retained)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sampled_textures_are_combined_image_samplers_read_by_fragments() {
        assert_eq!(
            BindingType::SampledTexture.descriptor_type(),
            vk::DescriptorType::COMBINED_IMAGE_SAMPLER
        );
        assert_eq!(
            BindingType::SampledTexture.stages(),
            vk::ShaderStageFlags::FRAGMENT
        );
    }
}
