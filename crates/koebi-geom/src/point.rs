//! [`Point`] and [`Vector`]: positions and displacements of the document plane, in f64.
//!
//! The plane is the one of the interface: x grows to the right, y grows downward. A positive
//! angle turns x toward y, so it reads clockwise on screen. Points and vectors are separate
//! types, so that the arithmetic says what it means: a point minus a point is a vector, a point
//! plus a vector is a point, and two points never add.

use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// A position in the document plane, in document units.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    /// Horizontal coordinate, growing to the right.
    pub x: f64,
    /// Vertical coordinate, growing downward.
    pub y: f64,
}

/// A displacement in the document plane: a difference of points, a direction, a handle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector {
    /// Horizontal component.
    pub x: f64,
    /// Vertical component.
    pub y: f64,
}

impl Point {
    /// The origin of the plane.
    pub const ORIGIN: Self = Self::new(0.0, 0.0);

    /// The point at `(x, y)`.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// The displacement from the origin to this point.
    pub const fn to_vector(self) -> Vector {
        Vector::new(self.x, self.y)
    }

    /// The distance to `other`.
    pub fn distance(self, other: Self) -> f64 {
        (other - self).length()
    }

    /// The squared distance to `other`: cheaper, and ordered like the distance.
    pub fn distance_squared(self, other: Self) -> f64 {
        (other - self).length_squared()
    }

    /// The point a share `t` of the way from this point to `other` (`t` = 0 gives this point,
    /// 1 gives `other`).
    #[must_use]
    pub fn lerp(self, other: Self, t: f64) -> Self {
        Self::new(
            self.x + (other.x - self.x) * t,
            self.y + (other.y - self.y) * t,
        )
    }

    /// The point halfway to `other`.
    #[must_use]
    pub fn midpoint(self, other: Self) -> Self {
        self.lerp(other, 0.5)
    }

    /// Whether both coordinates are finite.
    pub const fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    /// This point in the interface's single precision, for drawing overlays.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "screen coordinates fit an f32; precision is kept in f64 until here"
    )]
    pub fn to_vec2(self) -> koebi_core::Vec2 {
        koebi_core::Vec2::new(self.x as f32, self.y as f32)
    }
}

impl From<koebi_core::Vec2> for Point {
    fn from(value: koebi_core::Vec2) -> Self {
        Self::new(f64::from(value.x), f64::from(value.y))
    }
}

impl Vector {
    /// The null displacement.
    pub const ZERO: Self = Self::new(0.0, 0.0);

    /// The vector `(x, y)`.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// The unit vector at `angle` radians from the x axis, toward y.
    pub fn from_angle(angle: f64) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self::new(cos, sin)
    }

    /// Its length: the square root of the squares, several times faster than `hypot`, whose
    /// guard against overflow only matters beyond 1e154, far from any document.
    pub fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// Its squared length.
    pub fn length_squared(self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    /// The dot product with `other`.
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    /// The cross product with `other` (the z of the 3D cross product): positive when `other`
    /// turns from this vector toward y, that is clockwise on screen.
    pub fn cross(self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }

    /// The same direction at length 1, or `None` for a vector too short to have a direction.
    pub fn normalized(self) -> Option<Self> {
        let length = self.length();
        (length > f64::MIN_POSITIVE && length.is_finite()).then(|| self / length)
    }

    /// The vector turned a quarter turn, from x toward y: `(x, y)` becomes `(-y, x)`.
    #[must_use]
    pub const fn perpendicular(self) -> Self {
        Self::new(-self.y, self.x)
    }

    /// Its angle from the x axis, toward y, in radians in `-π..=π`.
    pub fn angle(self) -> f64 {
        self.y.atan2(self.x)
    }

    /// The point at this displacement from the origin.
    pub const fn to_point(self) -> Point {
        Point::new(self.x, self.y)
    }

    /// Whether both components are finite.
    pub const fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl Add<Vector> for Point {
    type Output = Self;
    fn add(self, rhs: Vector) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign<Vector> for Point {
    fn add_assign(&mut self, rhs: Vector) {
        *self = *self + rhs;
    }
}

impl Sub<Vector> for Point {
    type Output = Self;
    fn sub(self, rhs: Vector) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign<Vector> for Point {
    fn sub_assign(&mut self, rhs: Vector) {
        *self = *self - rhs;
    }
}

impl Sub for Point {
    type Output = Vector;
    fn sub(self, rhs: Self) -> Vector {
        Vector::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Add for Vector {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for Vector {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Vector {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for Vector {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<f64> for Vector {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl Mul<Vector> for f64 {
    type Output = Vector;
    fn mul(self, rhs: Vector) -> Vector {
        rhs * self
    }
}

impl Div<f64> for Vector {
    type Output = Self;
    fn div(self, rhs: f64) -> Self {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl Neg for Vector {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_keeps_points_and_vectors_apart() {
        let a = Point::new(1.0, 2.0);
        let b = Point::new(4.0, 6.0);
        let d = b - a;
        assert_eq!(d, Vector::new(3.0, 4.0));
        assert_eq!(a + d, b);
        assert_eq!(b - d, a);
        assert!((a.distance(b) - 5.0).abs() < 1e-15);
        assert_eq!(a.midpoint(b), Point::new(2.5, 4.0));
        assert_eq!(-d * 2.0, Vector::new(-6.0, -8.0));
    }

    #[test]
    fn angles_turn_from_x_toward_y() {
        let quarter = Vector::from_angle(core::f64::consts::FRAC_PI_2);
        assert!((quarter.x).abs() < 1e-15 && (quarter.y - 1.0).abs() < 1e-15);
        assert_eq!(Vector::new(1.0, 0.0).perpendicular(), Vector::new(0.0, 1.0));
        assert!(Vector::new(1.0, 0.0).cross(Vector::new(0.0, 1.0)) > 0.0);
        assert_eq!(Vector::ZERO.normalized(), None);
        let unit = Vector::new(3.0, 4.0).normalized().map(Vector::length);
        assert!(unit.is_some_and(|length| (length - 1.0).abs() < 1e-15));
    }
}
