//! Koebi's 2D renderer: a retained [`Scene`] turned into batched, instanced GPU draws.
//!
//! Every primitive type (rectangles today; text, paths and images next) has one pipeline and
//! one GPU instance layout. Consecutive primitives of the same type become a single
//! instanced draw call, so a full UI costs a handful of draws.
//!
//! - **Units**: scenes use logical pixels (`koebi_core::math`); the GPU applies
//!   [`Viewport::scale_factor`] to reach physical pixels, keeping edges pixel-exact.
//! - **Colors**: `koebi_core::Color` (gamma-encoded sRGB, straight alpha) is premultiplied at
//!   upload and blended in gamma space on UNORM targets, matching design tools
//!   (`docs/adr/0004-color-pipeline.md`).
//! - **Shaders**: GLSL in `shaders/`, compiled ahead of time to `shaders/spirv/` by
//!   `cargo xtask shaders` and embedded in the binary.

mod renderer;
mod scene;
mod shaders;

pub use renderer::{Renderer2D, Viewport};
pub use scene::{RectPrimitive, Scene};
