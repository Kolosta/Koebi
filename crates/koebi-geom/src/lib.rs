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
//! Stroke outlines, booleans and the preparation of curves for the GPU come next (#258, #259).
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
mod contour;
mod nurbs;
mod path;
mod point;
mod rect;
mod segment;
mod solve;

pub use affine::{Affine, Decomposed, Transform};
pub use contour::{Contour, Contours, FillRule, Nearest};
pub use nurbs::NurbsKnots;
pub use path::{Anchor, HandleKind, Path, Spline, Subpath};
pub use point::{Point, Vector};
pub use rect::Rect;
pub use segment::{MAX_DEGREE, RationalBez, Segment};
