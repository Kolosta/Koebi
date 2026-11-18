//! [`ImagePrimitive`]: a texture drawn in a rectangle: the target a view of the canvas renders
//! into, the preview of a node, the vignette of an object, the preview of a file (ADR 0029).
//!
//! The renderer keeps the images a scene may draw ([`Renderer2D::add_image`]), each a texture
//! and the encoding of its colors ([`ImageContent`]); a scene names them by their
//! [`ImageId`]. Images of one texture draw together in one instanced draw, like other
//! primitives (ADR 0025); each texture takes a draw of its own.
//!
//! The texture is read with bilinear filtering, clamped at its edges: drawn one texel per
//! physical pixel, on whole pixels (the default [`Edges::Snapped`]), it shows its texels
//! exactly.
//!
//! [`Renderer2D::add_image`]: crate::Renderer2D::add_image

use koebi_core::{Rect, Vec2};
use koebi_gpu::{
    AddressMode, BindGroup, BindGroupDesc, BindGroupLayout, BindGroupLayoutDesc, BindingResource,
    BindingType, Device, Error, FilterMode, Pod, Result, Sampler, SamplerDesc, Texture,
    TextureUsage,
};

use crate::scene::{Corners, Edges};

/// An image the renderer keeps ([`crate::Renderer2D::add_image`]), named by the scenes that
/// draw it. Numbers are never reused: an image removed draws nothing, even if another one is
/// added after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImageId(u32);

impl ImageId {
    /// Its number, the key of its batches.
    pub(crate) const fn key(self) -> u32 {
        self.0
    }
}

/// How the texels of an image encode their colors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ImageContent {
    /// Linear light, premultiplied alpha: what the canvas renders into its `Rgba16Float`
    /// targets. Encoded to the window's sRGB when drawn.
    #[default]
    Linear,
    /// Gamma-encoded sRGB, premultiplied alpha, as the window's own pixels: drawn as they are.
    Encoded,
}

/// A rectangle showing an image, or a part of it, with a radius per corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImagePrimitive {
    /// The image.
    pub image: ImageId,
    /// Where it shows, in logical pixels or in the space in force
    /// ([`Scene::push_space`](crate::Scene::push_space)).
    pub rect: Rect,
    /// The part of the image shown, in its normalized coordinates: 0 to 1 from its top left
    /// corner. A part reaching past them shows the image's edge texels.
    pub uv: Rect,
    /// Multiplies the image's alpha, 0 to 1.
    pub opacity: f32,
    /// Corner radii, each clamped to half the smallest side.
    pub corner_radii: Corners,
    /// How its edges meet the pixel grid: on whole pixels by default, where an image drawn one
    /// texel per pixel shows its texels exactly.
    pub edges: Edges,
}

impl ImagePrimitive {
    /// The whole of `image` in `rect`, opaque, square corners, edges on the pixel grid.
    pub const fn new(image: ImageId, rect: Rect) -> Self {
        Self {
            image,
            rect,
            uv: Rect::from_min_max(Vec2::ZERO, Vec2::splat(1.0)),
            opacity: 1.0,
            corner_radii: Corners::ZERO,
            edges: Edges::Snapped,
        }
    }

    /// The same image, showing the part `uv` of it.
    #[must_use]
    pub const fn with_uv(mut self, uv: Rect) -> Self {
        self.uv = uv;
        self
    }

    /// The same image, its alpha multiplied by `opacity`.
    #[must_use]
    pub const fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    /// The same image, the same radius at every corner.
    #[must_use]
    pub const fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radii = Corners::all(radius);
        self
    }

    /// The same image, a radius per corner.
    #[must_use]
    pub const fn with_corner_radii(mut self, radii: Corners) -> Self {
        self.corner_radii = radii;
        self
    }

    /// The same image, its edges meeting the pixel grid as `edges` says.
    #[must_use]
    pub const fn with_edges(mut self, edges: Edges) -> Self {
        self.edges = edges;
        self
    }
}

/// GPU layout of one image (vertex buffer, per-instance step).
///
/// Must match the inputs of `shaders/image.vert`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub(crate) struct ImageInstance {
    /// `x, y, width, height` in logical pixels.
    pub(crate) rect: [f32; 4],
    /// The part shown: `u0, v0, u1, v1`.
    pub(crate) uv: [f32; 4],
    /// Corner radii in logical pixels: top-left, top-right, bottom-right, bottom-left.
    pub(crate) radii: [f32; 4],
    /// The opacity, 1 for exact edges, then unused.
    pub(crate) params: [f32; 4],
}

// SAFETY: `repr(C)` struct of four `[f32; 4]` arrays: 64 bytes with no padding, and every bit
// pattern is a valid value.
unsafe impl Pod for ImageInstance {}
const _: () = assert!(size_of::<ImageInstance>() == 64);

impl ImageInstance {
    pub(crate) const STRIDE: u32 = 64;

    pub(crate) fn from_primitive(primitive: &ImagePrimitive) -> Self {
        let (r, uv) = (primitive.rect, primitive.uv);
        Self {
            rect: [r.min.x, r.min.y, r.width(), r.height()],
            uv: [uv.min.x, uv.min.y, uv.max.x, uv.max.y],
            radii: primitive
                .corner_radii
                .to_array()
                .map(|radius| radius.max(0.0)),
            params: [
                primitive.opacity.clamp(0.0, 1.0),
                f32::from(u8::from(primitive.edges == Edges::Exact)),
                0.0,
                0.0,
            ],
        }
    }
}

/// One image the renderer keeps: its bind group, which keeps its texture alive, and how its
/// texels encode their colors.
struct Kept {
    id: ImageId,
    bind_group: BindGroup,
    content: ImageContent,
}

/// The images scenes may draw, by number.
pub(crate) struct ImageStore {
    layout: BindGroupLayout,
    sampler: Sampler,
    /// By number, in increasing order: numbers only grow.
    images: Vec<Kept>,
    next: u32,
}

impl ImageStore {
    pub(crate) fn new(device: &Device) -> Result<Self> {
        Ok(Self {
            layout: device.create_bind_group_layout(&BindGroupLayoutDesc {
                label: "image",
                entries: &[BindingType::SampledTexture],
            })?,
            sampler: device.create_sampler(&SamplerDesc {
                label: "images",
                filter: FilterMode::Linear,
                address_mode: AddressMode::ClampToEdge,
            })?,
            images: Vec::new(),
            next: 0,
        })
    }

    /// The layout of an image's bind group, for the image pipeline.
    pub(crate) fn layout(&self) -> &BindGroupLayout {
        &self.layout
    }

    fn bind_group(&self, device: &Device, texture: &Texture) -> Result<BindGroup> {
        if !texture.usage().contains(TextureUsage::SAMPLED) {
            return Err(Error::InvalidArgument(
                "an image's texture needs SAMPLED usage".into(),
            ));
        }
        device.create_bind_group(&BindGroupDesc {
            label: "image",
            layout: &self.layout,
            entries: &[BindingResource::SampledTexture {
                texture,
                sampler: &self.sampler,
            }],
        })
    }

    /// Keeps `texture` as a new image.
    pub(crate) fn add(
        &mut self,
        device: &Device,
        texture: &Texture,
        content: ImageContent,
    ) -> Result<ImageId> {
        let bind_group = self.bind_group(device, texture)?;
        let id = ImageId(self.next);
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(|| Error::InvalidArgument("too many images".into()))?;
        self.images.push(Kept {
            id,
            bind_group,
            content,
        });
        Ok(id)
    }

    /// Shows `texture` for the image `id` from now on.
    pub(crate) fn replace(
        &mut self,
        device: &Device,
        id: ImageId,
        texture: &Texture,
        content: ImageContent,
    ) -> Result<()> {
        let index = self
            .index(id)
            .ok_or_else(|| Error::InvalidArgument(format!("no image {}", id.0)))?;
        let bind_group = self.bind_group(device, texture)?;
        self.images[index] = Kept {
            id,
            bind_group,
            content,
        };
        Ok(())
    }

    /// Forgets the image `id`; `false` when there was none.
    pub(crate) fn remove(&mut self, id: ImageId) -> bool {
        self.index(id)
            .map(|index| self.images.remove(index))
            .is_some()
    }

    fn index(&self, id: ImageId) -> Option<usize> {
        self.images.binary_search_by_key(&id, |kept| kept.id).ok()
    }

    /// The bind group and content of the image numbered `key`, if it is kept.
    pub(crate) fn get(&self, key: u32) -> Option<(&BindGroup, ImageContent)> {
        self.index(ImageId(key))
            .map(|index| (&self.images[index].bind_group, self.images[index].content))
    }

    /// How many images are kept.
    pub(crate) fn len(&self) -> usize {
        self.images.len()
    }
}

impl std::fmt::Debug for ImageStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageStore")
            .field("images", &self.images.len())
            .field("next", &self.next)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The image numbered `number`, for tests of what keys batches.
    pub(crate) const fn id(number: u32) -> ImageId {
        ImageId(number)
    }

    #[test]
    fn instances_carry_their_part_radii_opacity_and_edges() {
        let primitive = ImagePrimitive::new(
            ImageId(3),
            Rect::from_origin_size(Vec2::new(4.0, 6.0), Vec2::new(16.0, 20.0)),
        )
        .with_uv(Rect::from_min_max(Vec2::new(0.5, 0.0), Vec2::new(1.0, 0.5)))
        .with_opacity(2.0)
        .with_corner_radii(Corners {
            top_left: 1.0,
            top_right: -2.0,
            bottom_right: 3.0,
            bottom_left: 4.0,
        })
        .with_edges(Edges::Exact);
        let instance = ImageInstance::from_primitive(&primitive);
        assert_eq!(instance.rect, [4.0, 6.0, 16.0, 20.0]);
        assert_eq!(instance.uv, [0.5, 0.0, 1.0, 0.5]);
        assert_eq!(instance.radii, [1.0, 0.0, 3.0, 4.0]);
        assert_eq!(instance.params, [1.0, 1.0, 0.0, 0.0]);
        let default = ImageInstance::from_primitive(&ImagePrimitive::new(ImageId(0), Rect::ZERO));
        assert_eq!(default.uv, [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(default.params, [1.0, 0.0, 0.0, 0.0]);
    }
}
