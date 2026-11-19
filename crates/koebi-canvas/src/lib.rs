//! Koebi's canvas renderer: the programs the document evaluates
//! ([`koebi_doc::eval::Program`]) drawn into the target of each view, exact at any zoom and on
//! any driver (ADR 0029).
//!
//! - **Coverage from curves:** an outline is cut into the grid cells of the zoom's octave
//!   ([`koebi_geom::fill_cells`]), each cell holding its quadratic curves relative to its f64
//!   origin and what lies beyond its right and bottom sides folded onto them. One instance per
//!   cell covers the cell's own pixels, within a pixel of its outline's bounds; the fragment
//!   shader counts each pixel's winding by two rays over the cell's bands, as glyphs and icons
//!   are drawn. There is no tessellation, no MSAA and no zoom limit: a kilometre-wide shape seen
//!   at 10⁹ prepares a few cells whose curves stay within 2¹¹ pixels of their origins.
//! - **Kept from render to render:** cells are prepared once per outline and octave, around
//!   the view, and their records live in one texture that receives only what was added. An edit
//!   prepares the outlines it changed; a pan or a zoom within an octave prepares nothing but
//!   what it reveals ([`CanvasStats`] counts it). A view whose program revision and camera did
//!   not change records nothing: the interface shows its last image.
//! - **Views** ([`View`]): a camera in f64 (the document point at the target's center and a
//!   zoom), the target's size in physical pixels. Each view renders into its own `Rgba16Float`
//!   target ([`ViewTarget`]) in linear light, premultiplied, which the interface shows as an
//!   image (`koebi_render::ImagePrimitive`).
//! - **Strokes in pixels** (hairlines, `koebi_doc::StrokeUnits::Viewport`) are outlined by the
//!   canvas at each view's zoom, per quarter octave with hysteresis, and kept: their width on
//!   screen stays within 14% of the one asked for at any zoom.
//! - **Picking** ([`ViewTarget::pick`]) is exact, on the CPU: the topmost object a view drew
//!   under a point, found through an index of the bounds of what it drew.
//! - **Paints:** solid colors, in linear light, and patterns: a motif prepared once in its own
//!   space and drawn at every point of a lattice that shows, its cells moved there.
//! - **Compositing:** the program's merges, blends and masks run as groups, each in a target of
//!   its own pooled by the view and within the area it draws, composited onto the target below
//!   by the W3C formulas (the sixteen modes) or as Erase. A blend reaches only what lies below
//!   it in its own merge, and an Erase at the page's root reveals the page.
//! - **The CPU reference** ([`reference`](mod@reference)) computes the shader's coverage and
//!   the compositing in the same arithmetic, for the tests.

mod blend;
mod canvas;
mod cells;
mod compose;
mod frame;
mod pick;
pub mod reference;
mod store;
mod strokes;
mod view;

pub use canvas::{Canvas, TARGET_FORMAT, ViewTarget};
pub use store::CanvasStats;
pub use view::View;
