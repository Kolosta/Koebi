//! [`Affine`]: the transforms of the document plane, and [`Transform`]: an object's location,
//! rotation and scale.
//!
//! An affine maps `(x, y)` to `(a·x + c·y + e, b·x + d·y + f)`, the order of SVG's
//! `matrix(a, b, c, d, e, f)`. `a * b` applies `b` first, then `a`, as matrices compose. Every
//! value is f64: world transforms are composed from chains of parents and groups at every
//! evaluation (ADR 0028), never stored and patched, and stay exact at any zoom.

use core::ops::Mul;

use crate::{Point, Vector};

/// An affine transform of the plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    /// `[a, b, c, d, e, f]`: the columns `(a, b)` and `(c, d)` are the images of the unit
    /// vectors, `(e, f)` the translation.
    pub coefficients: [f64; 6],
}

impl Default for Affine {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Affine {
    /// The transform that changes nothing.
    pub const IDENTITY: Self = Self::new([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    /// The transform of SVG's `matrix(a, b, c, d, e, f)`.
    pub const fn new(coefficients: [f64; 6]) -> Self {
        Self { coefficients }
    }

    /// A translation by `offset`.
    pub const fn translate(offset: Vector) -> Self {
        Self::new([1.0, 0.0, 0.0, 1.0, offset.x, offset.y])
    }

    /// A rotation by `angle` radians about the origin, from x toward y.
    pub fn rotate(angle: f64) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self::new([cos, sin, -sin, cos, 0.0, 0.0])
    }

    /// A scale by `x` and `y` about the origin.
    pub const fn scale(x: f64, y: f64) -> Self {
        Self::new([x, 0.0, 0.0, y, 0.0, 0.0])
    }

    /// The point `point` maps to.
    pub fn apply(self, point: Point) -> Point {
        let [a, b, c, d, e, f] = self.coefficients;
        Point::new(a * point.x + c * point.y + e, b * point.x + d * point.y + f)
    }

    /// The vector `vector` maps to: the translation does not apply.
    pub fn apply_vector(self, vector: Vector) -> Vector {
        let [a, b, c, d, _, _] = self.coefficients;
        Vector::new(a * vector.x + c * vector.y, b * vector.x + d * vector.y)
    }

    /// The translation part.
    pub const fn translation(self) -> Vector {
        Vector::new(self.coefficients[4], self.coefficients[5])
    }

    /// The determinant of the linear part: the factor areas are scaled by, negative when the
    /// transform mirrors.
    pub fn determinant(self) -> f64 {
        let [a, b, c, d, _, _] = self.coefficients;
        a * d - b * c
    }

    /// The transform that undoes this one, or `None` when it collapses the plane (a zero
    /// scale) or holds a non-finite value.
    pub fn inverse(self) -> Option<Self> {
        let [a, b, c, d, e, f] = self.coefficients;
        let det = self.determinant();
        if !det.is_finite() || det.abs() <= f64::MIN_POSITIVE || !self.is_finite() {
            return None;
        }
        let inv = 1.0 / det;
        let (ia, ib, ic, id) = (d * inv, -b * inv, -c * inv, a * inv);
        Some(Self::new([
            ia,
            ib,
            ic,
            id,
            -(ia * e + ic * f),
            -(ib * e + id * f),
        ]))
    }

    /// Whether every coefficient is finite.
    pub fn is_finite(self) -> bool {
        self.coefficients.iter().all(|value| value.is_finite())
    }

    /// The location, rotation and scale this transform is made of, with the skew left over: a
    /// non-uniform scale under a rotation, composed through a parent, shears, and a location,
    /// rotation and scale cannot hold a shear. `skew` is the tangent of the shear angle (0 when
    /// there is none). Mirrors appear as a negative y scale.
    pub fn decompose(self) -> Decomposed {
        let [a, b, c, d, e, f] = self.coefficients;
        let scale_x = a.hypot(b);
        let rotation = b.atan2(a);
        let det = self.determinant();
        let (scale_y, skew) = if scale_x > f64::MIN_POSITIVE {
            (det / scale_x, (a * c + b * d) / (scale_x * scale_x))
        } else {
            (c.hypot(d), 0.0)
        };
        Decomposed {
            transform: Transform {
                location: Vector::new(e, f),
                rotation,
                scale: Vector::new(scale_x, scale_y),
            },
            skew,
        }
    }
}

impl Mul for Affine {
    type Output = Self;

    /// `self * rhs` applies `rhs` first, then `self`.
    fn mul(self, rhs: Self) -> Self {
        let [a0, b0, c0, d0, e0, f0] = self.coefficients;
        let [a1, b1, c1, d1, e1, f1] = rhs.coefficients;
        Self::new([
            a0 * a1 + c0 * b1,
            b0 * a1 + d0 * b1,
            a0 * c1 + c0 * d1,
            b0 * c1 + d0 * d1,
            a0 * e1 + c0 * f1 + e0,
            b0 * e1 + d0 * f1 + f0,
        ])
    }
}

impl Mul<Point> for Affine {
    type Output = Point;
    fn mul(self, rhs: Point) -> Point {
        self.apply(rhs)
    }
}

/// An object's own placement: a scale, then a rotation, then a location, as in Blender.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    /// Where the object's origin lies.
    pub location: Vector,
    /// Its rotation in radians, from x toward y.
    pub rotation: f64,
    /// Its scale along its own axes.
    pub scale: Vector,
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    /// No move, no rotation, a scale of 1.
    pub const IDENTITY: Self = Self {
        location: Vector::ZERO,
        rotation: 0.0,
        scale: Vector::new(1.0, 1.0),
    };

    /// The affine of this placement: scale, then rotate, then translate.
    pub fn to_affine(self) -> Affine {
        let (sin, cos) = self.rotation.sin_cos();
        let (sx, sy) = (self.scale.x, self.scale.y);
        Affine::new([
            cos * sx,
            sin * sx,
            -sin * sy,
            cos * sy,
            self.location.x,
            self.location.y,
        ])
    }
}

/// What [`Affine::decompose`] finds: a placement, and the skew it could not hold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Decomposed {
    /// Location, rotation and scale.
    pub transform: Transform,
    /// The tangent of the shear angle left over, 0 without shear.
    pub skew: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Affine, b: Affine, tolerance: f64) -> bool {
        a.coefficients
            .iter()
            .zip(b.coefficients)
            .all(|(x, y)| (x - y).abs() <= tolerance)
    }

    #[test]
    fn composition_applies_the_right_operand_first() {
        let move_right = Affine::translate(Vector::new(10.0, 0.0));
        let double = Affine::scale(2.0, 2.0);
        let p = Point::new(1.0, 1.0);
        assert_eq!((move_right * double).apply(p), Point::new(12.0, 2.0));
        assert_eq!((double * move_right).apply(p), Point::new(22.0, 2.0));
    }

    #[test]
    fn inverses_undo_and_degenerate_transforms_have_none() {
        let t = Transform {
            location: Vector::new(3.5, -2.0),
            rotation: 0.7,
            scale: Vector::new(2.0, 0.25),
        }
        .to_affine();
        let inverse = t.inverse().unwrap();
        assert!(close(t * inverse, Affine::IDENTITY, 1e-12));
        assert!(close(inverse * t, Affine::IDENTITY, 1e-12));
        assert_eq!(Affine::scale(0.0, 1.0).inverse(), None);
        assert_eq!(
            Affine::new([f64::NAN, 0.0, 0.0, 1.0, 0.0, 0.0]).inverse(),
            None
        );
    }

    #[test]
    fn a_placement_decomposes_back_into_itself() {
        let placement = Transform {
            location: Vector::new(-7.0, 11.0),
            rotation: -1.2,
            scale: Vector::new(1.5, 3.0),
        };
        let found = placement.to_affine().decompose();
        assert!(found.skew.abs() < 1e-12);
        assert!(close(
            found.transform.to_affine(),
            placement.to_affine(),
            1e-12
        ));
    }

    #[test]
    fn a_non_uniform_scale_under_a_rotation_shears() {
        // A parent scaled 2 × 1 holding a child turned 45°: the child is sheared in the world.
        let parent = Affine::scale(2.0, 1.0);
        let child = Affine::rotate(core::f64::consts::FRAC_PI_4);
        let found = (parent * child).decompose();
        assert!(found.skew.abs() > 0.1, "{found:?}");
    }
}
