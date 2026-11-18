//! Koebi's 2D renderer: a retained [`Scene`] turned into batched, instanced GPU draws.
//!
//! Every primitive type (rectangles with a radius per corner and a border width per side,
//! line segments, fills, icons, glyphs and images) has one pipeline and one GPU instance
//! layout. Consecutive primitives of the same type become a single instanced draw
//! call, and runs of different types are drawn in insertion order, so a full UI costs a
//! handful of draws.
//!
//! - **Units**: scenes use logical pixels (`koebi_core::math`); the GPU applies
//!   [`Viewport::scale_factor`] to reach physical pixels, keeping edges pixel-exact.
//! - **Colors**: `koebi_core::Color` (gamma-encoded sRGB, straight alpha) is premultiplied at
//!   upload and blended in gamma space on UNORM targets, matching design tools
//!   (`docs/adr/0004-color-pipeline.md`).
//! - **Layers**: [`Scene::push_layer`] sends what follows to a numbered layer; layers draw in
//!   increasing order, so floating menus and overlays cover the base whatever the order they
//!   are pushed in, and a layer starts outside the clips in force.
//! - **Clipping**: [`Scene::push_clip`] clips what follows to a rectangle, applied as a
//!   scissor per draw; clips nest, and primitives entirely outside are dropped.
//!   [`Scene::push_rounded_clip`] also cuts them at rounded corners, in every fragment shader.
//! - **Spaces**: [`Scene::push_space`] takes what follows in the local units of a zoomable
//!   space (a uniform scale and a translation, nested ones composed) and maps it to logical
//!   pixels as it is pushed; edges then snap after the space, and hairlines thinner than a
//!   pixel keep one and fade (ADR 0030).
//! - **Icons**: vector icons compiled by `koebi-svg` are drawn from their curves in the fragment
//!   shader, sharp at every size and recolored zone by zone per instance; the set lives in a
//!   data texture uploaded once (`docs/adr/0020-vector-icons.md`).
//! - **Text**: glyphs of `koebi_text` lines are drawn from their curves like icons, at any size
//!   and angle: each glyph's outline is filed by bands in a data texture the first time a frame
//!   draws it (`docs/adr/0022-text-on-the-gpu.md`).
//! - **Images**: textures the renderer keeps ([`Renderer2D::add_image`]) drawn in rectangles
//!   ([`ImagePrimitive`]): the targets views of the canvas render into, in linear light, are
//!   encoded to the window's sRGB as they are drawn. Images of one texture draw together.
//! - **Dot grids**: [`DotGrid`] chooses the two levels of dots under a canvas by zoom, drawn by
//!   one fill ([`Fill::Dots`]).
//! - **Shaders**: GLSL in `shaders/`, compiled ahead of time to `shaders/spirv/` by
//!   `cargo xtask shaders` and embedded in the binary.

mod batch;
mod dots;
mod fill;
mod glyphs;
mod icon;
mod image;
mod renderer;
mod scene;
mod shaders;

pub use dots::{DotGrid, DotLevels};
pub use fill::{Fill, FillPrimitive, HueModel, Pointing};
pub use icon::{ICON_ZONES, IconColors, IconPrimitive};
pub use image::{ImageContent, ImageId, ImagePrimitive};
pub use renderer::{Renderer2D, Viewport};
pub use scene::{
    Corners, Edges, GlyphPrimitive, LinePrimitive, RectPrimitive, Scene, Sides, TIE_TOLERANCE,
    pixel_edge,
};
