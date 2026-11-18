//! Koebi's canvas renderer: the programs the document evaluates
//! ([`koebi_doc::eval::Program`]) drawn into the target of each view, exact at any zoom and on
//! any driver (ADR 0029).
//!
//! - **Coverage from curves:** an outline is cut into the grid cells of the zoom's octave
//!   ([`koebi_geom::fill_cells`]), each cell holding its quadratic curves relative to its f64
//!   origin and what lies beyond its right and bottom sides folded onto them. One instance per
//!   cell covers the cell's own pixels; the fragment shader counts each pixel's winding by two
//!   rays over the cell's bands, as glyphs and icons are drawn. There is no tessellation, no
//!   MSAA and no zoom limit: a kilometre-wide shape seen at 10⁹ prepares a few cells whose
//!   curves stay within 2¹¹ pixels of their origins.
//! - **Views** ([`View`]): a camera in f64 (the document point at the target's center and a
//!   zoom), the target's size in physical pixels. Each view renders into its own `Rgba16Float`
//!   target ([`ViewTarget`]) in linear light, premultiplied, which the interface shows as an
//!   image (`koebi_render::ImagePrimitive`).
//! - **Paints:** solid colors, in linear light.
//! - **The CPU reference** ([`reference`](mod@reference)) computes the shader's coverage in the
//!   same arithmetic, for the tests.
//!
//! Compositing (blend modes, Erase, masks) comes with the next part of the renderer: until then,
//! every layer of a program draws over what lies below it, at its opacity.

mod canvas;
mod cells;
mod frame;
pub mod reference;
mod view;

pub use canvas::{Canvas, TARGET_FORMAT, ViewTarget};
pub use view::View;
