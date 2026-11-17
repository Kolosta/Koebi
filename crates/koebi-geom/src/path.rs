//! [`Path`]: what the user draws and edits: subpaths of anchors with handles, as Bézier, NURBS
//! or polyline splines, closed or open.
//!
//! A path is the editable form. [`Path::contours`] turns it into [`Contours`]: the segments
//! measures, hit tests and the renderer read. The conversion allocates and is done once per
//! edit; what reads contours does not allocate.

use crate::contour::{Contour, Contours};
use crate::nurbs::{self, NurbsKnots};
use crate::{Point, Rect, Segment, Vector};

/// How an anchor's two handles relate, as in Carto and Blender.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HandleKind {
    /// Independent handles: a sharp corner is possible.
    #[default]
    Corner,
    /// Handles kept on one line through the anchor, each with its own length.
    Smooth,
    /// Handles kept on one line, of equal length.
    Symmetric,
}

/// A point of a subpath, with its handles relative to it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anchor {
    /// Where the anchor lies.
    pub position: Point,
    /// The handle toward the previous anchor, relative to `position`; `None` for no handle.
    pub handle_in: Option<Vector>,
    /// The handle toward the next anchor, relative to `position`.
    pub handle_out: Option<Vector>,
    /// How the handles relate when one is edited.
    pub kind: HandleKind,
    /// The NURBS weight (1 for every other spline).
    pub weight: f64,
}

impl Anchor {
    /// An anchor at `position` with no handles.
    pub const fn corner(position: Point) -> Self {
        Self {
            position,
            handle_in: None,
            handle_out: None,
            kind: HandleKind::Corner,
            weight: 1.0,
        }
    }

    /// An anchor at `position` with symmetric handles: `handle_out`, and its opposite in.
    pub fn symmetric(position: Point, handle_out: Vector) -> Self {
        Self {
            position,
            handle_in: Some(-handle_out),
            handle_out: Some(handle_out),
            kind: HandleKind::Symmetric,
            weight: 1.0,
        }
    }
}

/// The kind of curve a subpath draws through its anchors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Spline {
    /// Cubic Béziers between anchors, shaped by their handles.
    #[default]
    Bezier,
    /// Straight lines between anchors; handles are ignored.
    Poly,
    /// A NURBS curve with the anchors as control points.
    Nurbs {
        /// Its order: degree + 1, from 2 to 6.
        order: u8,
        /// Its knots' layout.
        knots: NurbsKnots,
    },
}

/// A connected run of anchors.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Subpath {
    /// The anchors in order.
    pub anchors: Vec<Anchor>,
    /// Whether the last anchor joins the first.
    pub closed: bool,
    /// The curve through them.
    pub spline: Spline,
}

/// A path: one or several subpaths, filled together.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path {
    /// The subpaths.
    pub subpaths: Vec<Subpath>,
}

/// The handle length that makes four cubic quarters approximate a circle best: 4/3·tan(π/8).
const KAPPA: f64 = 0.552_284_749_830_793_4;

impl Subpath {
    /// The segments this subpath draws, appended to `out`.
    pub fn segments_into(&self, out: &mut Vec<Segment>) {
        let anchors = &self.anchors;
        match self.spline {
            Spline::Bezier | Spline::Poly => {
                let count = anchors.len();
                let pairs = if self.closed {
                    count
                } else {
                    count.saturating_sub(1)
                };
                if count < 2 {
                    return;
                }
                for i in 0..pairs {
                    let (a, b) = (&anchors[i], &anchors[(i + 1) % count]);
                    let handles = if self.spline == Spline::Bezier {
                        (a.handle_out, b.handle_in)
                    } else {
                        (None, None)
                    };
                    out.push(match handles {
                        (None, None) => Segment::line(a.position, b.position),
                        (out_handle, in_handle) => Segment::cubic(
                            a.position,
                            a.position + out_handle.unwrap_or(Vector::ZERO),
                            b.position + in_handle.unwrap_or(Vector::ZERO),
                            b.position,
                        ),
                    });
                }
            }
            Spline::Nurbs { order, knots } => {
                let points: Vec<Point> = anchors.iter().map(|a| a.position).collect();
                let weights: Vec<f64> = anchors.iter().map(|a| a.weight).collect();
                nurbs::segments(
                    &points,
                    &weights,
                    usize::from(order),
                    knots,
                    self.closed,
                    out,
                );
            }
        }
    }
}

impl Path {
    /// The segments of every subpath, as contours: what measures, hit tests and the renderer
    /// read.
    pub fn contours(&self) -> Contours {
        let mut contours = Contours::default();
        self.contours_into(&mut contours);
        contours
    }

    /// [`Self::contours`] into `contours`, reusing its storage.
    pub fn contours_into(&self, contours: &mut Contours) {
        contours.clear();
        for subpath in &self.subpaths {
            let mut segments = Vec::new();
            subpath.segments_into(&mut segments);
            if !segments.is_empty() {
                contours.push(Contour::new(segments, subpath.closed));
            }
        }
    }

    /// A rectangle: four corners, closed.
    pub fn rectangle(bounds: Rect) -> Self {
        let corners = [
            bounds.min,
            Point::new(bounds.max.x, bounds.min.y),
            bounds.max,
            Point::new(bounds.min.x, bounds.max.y),
        ];
        Self {
            subpaths: vec![Subpath {
                anchors: corners.into_iter().map(Anchor::corner).collect(),
                closed: true,
                spline: Spline::Bezier,
            }],
        }
    }

    /// An ellipse made of four cubic quarters with symmetric handles: editable like Blender's
    /// Bézier circle, within 0.03 % of the true ellipse. [`Self::exact_ellipse`] is exact.
    pub fn ellipse(center: Point, radii: Vector) -> Self {
        let (rx, ry) = (radii.x, radii.y);
        let anchors = vec![
            Anchor::symmetric(center + Vector::new(rx, 0.0), Vector::new(0.0, ry * KAPPA)),
            Anchor::symmetric(center + Vector::new(0.0, ry), Vector::new(-rx * KAPPA, 0.0)),
            Anchor::symmetric(
                center + Vector::new(-rx, 0.0),
                Vector::new(0.0, -ry * KAPPA),
            ),
            Anchor::symmetric(center + Vector::new(0.0, -ry), Vector::new(rx * KAPPA, 0.0)),
        ];
        Self {
            subpaths: vec![Subpath {
                anchors,
                closed: true,
                spline: Spline::Bezier,
            }],
        }
    }

    /// An exact ellipse: a closed quadratic NURBS on the eight points of its bounding box, the
    /// corners weighted √2/2, Carto's NURBS circle.
    pub fn exact_ellipse(center: Point, radii: Vector) -> Self {
        let (rx, ry) = (radii.x, radii.y);
        let corner = core::f64::consts::FRAC_1_SQRT_2;
        let points = [
            (rx, 0.0, 1.0),
            (rx, ry, corner),
            (0.0, ry, 1.0),
            (-rx, ry, corner),
            (-rx, 0.0, 1.0),
            (-rx, -ry, corner),
            (0.0, -ry, 1.0),
            (rx, -ry, corner),
        ];
        let anchors = points
            .into_iter()
            .map(|(x, y, weight)| Anchor {
                weight,
                ..Anchor::corner(center + Vector::new(x, y))
            })
            .collect();
        Self {
            subpaths: vec![Subpath {
                anchors,
                closed: true,
                spline: Spline::Nurbs {
                    order: 3,
                    knots: NurbsKnots {
                        endpoint: true,
                        bezier: true,
                    },
                },
            }],
        }
    }

    /// A regular polygon of `sides` (3 or more), its first corner at `rotation` radians.
    pub fn polygon(center: Point, radius: f64, sides: usize, rotation: f64) -> Self {
        let sides = sides.max(3);
        let anchors = (0..sides)
            .map(|i| {
                let angle = rotation + core::f64::consts::TAU * i as f64 / sides as f64;
                Anchor::corner(center + Vector::from_angle(angle) * radius)
            })
            .collect();
        Self {
            subpaths: vec![Subpath {
                anchors,
                closed: true,
                spline: Spline::Poly,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_exact_ellipse_is_exact_and_an_editable_one_close() {
        let center = Point::new(10.0, -4.0);
        let radii = Vector::new(30.0, 20.0);
        let on_ellipse = |p: Point| {
            let d = p - center;
            (d.x / radii.x).powi(2) + (d.y / radii.y).powi(2)
        };
        let exact = Path::exact_ellipse(center, radii).contours();
        assert_eq!(exact.contours()[0].segments().len(), 4);
        for segment in exact.contours()[0].segments() {
            for i in 0..=50 {
                let value = on_ellipse(segment.eval(f64::from(i) / 50.0));
                assert!((value - 1.0).abs() < 1e-14, "{value}");
            }
        }
        let editable = Path::ellipse(center, radii).contours();
        for segment in editable.contours()[0].segments() {
            for i in 0..=50 {
                let value = on_ellipse(segment.eval(f64::from(i) / 50.0));
                assert!((value - 1.0).abs() < 1e-3, "{value}");
            }
        }
    }

    #[test]
    fn bezier_subpaths_use_their_handles_and_poly_ones_ignore_them() {
        let anchors = vec![
            Anchor::symmetric(Point::new(0.0, 0.0), Vector::new(10.0, 0.0)),
            Anchor::corner(Point::new(20.0, 0.0)),
        ];
        let bezier = Subpath {
            anchors: anchors.clone(),
            closed: false,
            spline: Spline::Bezier,
        };
        let mut out = Vec::new();
        bezier.segments_into(&mut out);
        assert!(matches!(out[0], Segment::Cubic(_)));
        let poly = Subpath {
            anchors,
            closed: true,
            spline: Spline::Poly,
        };
        out.clear();
        poly.segments_into(&mut out);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|s| matches!(s, Segment::Line(_))));
    }
}
