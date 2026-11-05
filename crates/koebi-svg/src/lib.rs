//! Koebi's icon compiler: SVG files, in a subset Koebi checks, compiled into vector icons the
//! renderer draws from their curves at any size, sharp and recolorable per instance, never
//! rasterized ahead of time (`docs/adr/0020-vector-icons.md`).
//!
//! - [`compile`] reads an SVG file into an [`Icon`]: layers of quadratic Béziers in the icon's
//!   unit square (its view box kept whole: an icon drawn against one side keeps the space on
//!   the other), each filled with a color or a gradient whose colors belong to color zones.
//!   Strokes become fills, shapes and arcs curves, transforms are applied. What the subset does
//!   not hold is an error naming the line, never a silent difference (see [`compile`]'s module).
//! - Zones group the icon's colors: every distinct color is one, its role (primary, secondary,
//!   tertiary, accent) read from the `class` of what it colors (`ds-primary`, ...). The
//!   interface recolors an icon zone by zone (from tokens, by state), or shows it with its
//!   original colors.
//! - [`pack`] lays icons out in one array of `vec4` the renderer's shader reads.
//! - [`raster`] computes the same coverage on the CPU: cursors, the application icon, tests.
//!
//! The crate depends on nothing: the icons are compiled by the build script of `koebi-icons`.

#![allow(
    clippy::many_single_char_names,
    reason = "geometry follows the notation of its formulas: Bézier points, matrix entries"
)]

mod color;
mod compile;
mod error;
mod geometry;
mod model;
pub mod pack;
mod path;
pub mod raster;
mod stroke;
mod xml;

pub use color::Rgba;
pub use compile::{MAX_ZONES, compile};
pub use error::{Error, Result};
pub use geometry::{Affine, Point};
pub use model::{FillRule, Icon, Layer, Paint, Role, Stop, Zone};
pub use stroke::{Cap, Join, StrokeStyle};
