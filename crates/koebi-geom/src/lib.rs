//! Koebi's document geometry, in f64 and on the CPU ([ADR 0029]).
//!
//! - [`Point`], [`Vector`]: positions and displacements of the document plane (y downward).
//! - [`Affine`], [`Transform`]: transforms, and an object's location, rotation and scale.
//!   [`Affine::decompose`] gives a transform back with the skew it cannot hold.
//! - [`Rect`], for bounds.
//! - [`Segment`]: lines, quadratic and cubic Béziers, and rational Béziers up to degree 5,
//!   evaluated exactly in homogeneous coordinates: NURBS spans, true circles.
//! - [`Path`]: what the user draws and edits: [`Subpath`]s of [`Anchor`]s with handles
//!   ([`HandleKind`]), as Bézier, NURBS or polyline [`Spline`]s. Builders give rectangles, an
//!   editable ellipse, an exact one, regular polygons.
//! - [`Contours`]: a path's segments, built once per edit, then measured (bounds, lengths,
//!   points at a length, nearest points) and hit-tested ([`FillRule`]) without allocating.
//! - [`outline`] and [`Stroker`]: a stroke ([`StrokeStyle`]) turned into contours to fill, so
//!   that strokes are drawn with the coverage of fills. Every align, cap, join and dash, and
//!   a stroke over itself united rather than darkened.
//! - [`boolean`] ([`BooleanOp`]), [`simplify`], [`offset_region`]: union, intersection,
//!   difference and exclusive or of filled outlines under their fill rules, a region made of
//!   loops that neither overlap nor cross, a region grown or shrunk; as curves.
//! - [`prepare`] ([`Octave`], [`Cell`]): an outline made quadratic and monotone within a
//!   sixteenth of a pixel, cut into cells with f64 origins, for the canvas at any zoom.
//!
//! ```
//! use koebi_geom::{FillRule, Path, Point, Vector};
//!
//! // An exact circle of radius 10: a closed quadratic NURBS.
//! let circle = Path::exact_ellipse(Point::new(0.0, 0.0), Vector::new(10.0, 10.0)).contours();
//! assert!(circle.contains(Point::new(7.0, 7.0), FillRule::NonZero));
//! assert!(!circle.contains(Point::new(7.1, 7.1), FillRule::NonZero));
//! let length = circle.contours()[0].length();
//! assert!((length - 20.0 * core::f64::consts::PI).abs() < 1e-8);
//! ```
//!
//! The crate depends on nothing but `koebi-core`.
//!
//! [ADR 0029]: https://github.com/Kolosta/Koebi/blob/develop/docs/adr/0029-canvas-rendering-from-curves.md

#![allow(
    clippy::many_single_char_names,
    reason = "geometry follows the notation of its formulas: Bézier points, matrix entries"
)]
#![allow(
    clippy::float_cmp,
    reason = "exact comparisons where they are meant: sign changes, degenerate inputs"
)]

mod affine;
mod boolean;
mod cells;
mod contour;
mod nurbs;
mod path;
mod point;
mod prepare;
mod rect;
mod segment;
mod solve;
mod stroke;

pub use affine::{Affine, Decomposed, Transform};
pub use boolean::{BooleanOp, boolean, offset_region, simplify};
pub use cells::{Edge, FillCell, fill_cells};
pub use contour::{Contour, Contours, FillRule, Nearest};
pub use nurbs::NurbsKnots;
pub use path::{Anchor, HandleKind, Path, Spline, Subpath};
pub use point::{Point, Vector};
pub use prepare::{Cell, Octave, Quadratic, cells, prepare, quadratics};
pub use rect::Rect;
pub use segment::{MAX_DEGREE, RationalBez, Segment};
pub use stroke::{
    Align, Cap, DashAnchor, DashCenter, DashFit, Dashes, Join, StrokeOffset, StrokeStyle, Stroker,
    outline,
};
