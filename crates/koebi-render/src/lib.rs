//! Koebi's 2D renderer: a retained [`Scene`] turned into batched, instanced GPU draws.
//!
//! Every primitive type (rectangles with a radius per corner and a border width per side,
//! line segments and glyphs today; paths and images next) has one pipeline and one GPU
//! instance layout. Consecutive primitives of the same type become a single instanced draw
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
//! - **Text**: glyphs of `koebi_text` lines are rasterized on the CPU at their physical size,
//!   cached in a glyph atlas texture and drawn one texel per pixel
//!   (`docs/adr/0014-text-basics.md`).
//! - **Shaders**: GLSL in `shaders/`, compiled ahead of time to `shaders/spirv/` by
//!   `cargo xtask shaders` and embedded in the binary.

mod atlas;
mod renderer;
mod scene;
mod shaders;

pub use renderer::{Renderer2D, Viewport};
pub use scene::{Corners, GlyphPrimitive, LinePrimitive, RectPrimitive, Scene, Sides};
