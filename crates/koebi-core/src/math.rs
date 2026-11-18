//! Geometry primitives and unit conventions.
//!
//! Koebi distinguishes two units, and the type tells you which one you hold:
//!
//! - **Logical pixels** (`f32`, [`Vec2`], [`Rect`]): layout and scene coordinates, independent
//!   of display density. The origin is the top-left corner, `x` grows to the right and `y`
//!   grows downwards.
//! - **Physical pixels** (`u32`, [`Extent`]): framebuffer, swapchain and texture sizes.
//!
//! Conversion is always `physical = logical * scale_factor`, where the scale factor comes
//! from the platform layer (display density times user content scale).
//!
//! A [`Space`] maps the local units of a zoomable part of the interface (a node graph, a
//! preview) to the units around it: a uniform scale, then a translation.

use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// A 2D vector or point, in logical pixels unless stated otherwise.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Vec2 {
    /// Horizontal component, growing to the right.
    pub x: f32,
    /// Vertical component, growing downwards.
    pub y: f32,
}

impl Vec2 {
    /// `(0, 0)`.
    pub const ZERO: Self = Self::new(0.0, 0.0);
    /// `(1, 1)`.
    pub const ONE: Self = Self::splat(1.0);

    /// Creates a vector from its components.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Creates a vector with both components set to `value`.
    pub const fn splat(value: f32) -> Self {
        Self::new(value, value)
    }

    /// Component-wise minimum.
    #[must_use]
    pub fn min(self, other: Self) -> Self {
        Self::new(self.x.min(other.x), self.y.min(other.y))
    }

    /// Component-wise maximum.
    #[must_use]
    pub fn max(self, other: Self) -> Self {
        Self::new(self.x.max(other.x), self.y.max(other.y))
    }

    /// Component-wise absolute value.
    #[must_use]
    pub fn abs(self) -> Self {
        Self::new(self.x.abs(), self.y.abs())
    }

    /// Euclidean length.
    #[must_use]
    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }

    /// Dot product.
    #[must_use]
    pub fn dot(self, other: Self) -> f32 {
        self.x.mul_add(other.x, self.y * other.y)
    }

    /// Returns `[x, y]`, the layout expected by GPU buffers.
    #[must_use]
    pub const fn to_array(self) -> [f32; 2] {
        [self.x, self.y]
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl Mul for Vec2 {
    type Output = Self;
    /// Component-wise product.
    fn mul(self, rhs: Self) -> Self {
        Self::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;
    fn div(self, rhs: f32) -> Self {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

/// An axis-aligned rectangle stored as its minimum (top-left) and maximum (bottom-right)
/// corners, in logical pixels unless stated otherwise.
///
/// A rectangle whose `max` is not strictly greater than its `min` on both axes is empty.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Rect {
    /// Top-left corner (inclusive).
    pub min: Vec2,
    /// Bottom-right corner (exclusive).
    pub max: Vec2,
}

impl Rect {
    /// The empty rectangle at the origin.
    pub const ZERO: Self = Self::from_min_max(Vec2::ZERO, Vec2::ZERO);

    /// Creates a rectangle from its corners.
    pub const fn from_min_max(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    /// Creates a rectangle from its top-left corner and size.
    #[must_use]
    pub fn from_origin_size(origin: Vec2, size: Vec2) -> Self {
        Self::from_min_max(origin, origin + size)
    }

    /// Creates a rectangle centered on `center`.
    #[must_use]
    pub fn from_center_size(center: Vec2, size: Vec2) -> Self {
        let half = size * 0.5;
        Self::from_min_max(center - half, center + half)
    }

    /// Width (`max.x - min.x`).
    #[must_use]
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    /// Height (`max.y - min.y`).
    #[must_use]
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    /// Size as a vector.
    #[must_use]
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    /// Center point.
    #[must_use]
    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    /// Returns `true` when the rectangle has no area.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.max.x <= self.min.x || self.max.y <= self.min.y
    }

    /// Returns `true` if `point` lies inside (min inclusive, max exclusive).
    #[must_use]
    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.y >= self.min.y
            && point.x < self.max.x
            && point.y < self.max.y
    }

    /// Returns the overlapping area, or `None` if the rectangles do not overlap.
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let result = Self::from_min_max(self.min.max(other.min), self.max.min(other.max));
        (!result.is_empty()).then_some(result)
    }

    /// The smallest rectangle holding both.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        Self::from_min_max(self.min.min(other.min), self.max.max(other.max))
    }

    /// Shrinks the rectangle by `amount` on every side (grows it when negative).
    #[must_use]
    pub fn inset(&self, amount: f32) -> Self {
        Self::from_min_max(
            self.min + Vec2::splat(amount),
            self.max - Vec2::splat(amount),
        )
    }

    /// Moves the rectangle by `offset`.
    #[must_use]
    pub fn translate(&self, offset: Vec2) -> Self {
        Self::from_min_max(self.min + offset, self.max + offset)
    }

    /// Scales both corners by `factor` (e.g. logical to physical pixels).
    #[must_use]
    pub fn scale(&self, factor: f32) -> Self {
        Self::from_min_max(self.min * factor, self.max * factor)
    }
}

/// A uniform scale then a translation: how the local units of a zoomable space map to the
/// units around it (ADR 0030). The point `p` of the space lies at `p * scale + offset` around
/// it.
///
/// A space that cannot be drawn, its scale not positive and finite or its offset not finite,
/// composes into [`Space::COLLAPSED`]: everything in it lies at one point, without size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Space {
    /// How many units around the space one of its units spans: 2 zooms in, 0.5 out.
    pub scale: f32,
    /// Where the space's origin lies, in the units around it.
    pub offset: Vec2,
}

impl Default for Space {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Space {
    /// The space whose units are those around it.
    pub const IDENTITY: Self = Self::new(1.0, Vec2::ZERO);

    /// The space where everything lies at the origin, without size: what a space that cannot
    /// be drawn becomes.
    pub const COLLAPSED: Self = Self::new(0.0, Vec2::ZERO);

    /// Units of `scale`, the origin at `offset`.
    pub const fn new(scale: f32, offset: Vec2) -> Self {
        Self { scale, offset }
    }

    /// `true` when the space can be drawn: its scale positive and finite, its offset finite.
    pub fn is_valid(self) -> bool {
        self.scale > 0.0
            && self.scale.is_finite()
            && self.offset.x.is_finite()
            && self.offset.y.is_finite()
    }

    /// `inner`, opened inside this space, as a space of the units around this one: its points
    /// map through `inner`, then through `self`. [`Space::COLLAPSED`] when either cannot be
    /// drawn, or their composition cannot (a scale that overflows).
    #[must_use]
    pub fn then(self, inner: Self) -> Self {
        let composed = Self::new(self.scale * inner.scale, self.map_point(inner.offset));
        if self.is_valid() && inner.is_valid() && composed.is_valid() {
            composed
        } else {
            Self::COLLAPSED
        }
    }

    /// Where the point `point` of the space lies around it.
    #[must_use]
    pub fn map_point(self, point: Vec2) -> Vec2 {
        point * self.scale + self.offset
    }

    /// The vector `vector` of the space, around it.
    #[must_use]
    pub fn map_vector(self, vector: Vec2) -> Vec2 {
        vector * self.scale
    }

    /// The length `length` of the space, around it.
    #[must_use]
    pub fn map_length(self, length: f32) -> f32 {
        length * self.scale
    }

    /// Where the rectangle `rect` of the space lies around it.
    #[must_use]
    pub fn map_rect(self, rect: Rect) -> Rect {
        Rect::from_min_max(self.map_point(rect.min), self.map_point(rect.max))
    }

    /// The point of the space at `point` around it; the origin in a collapsed space, where
    /// every point lies at once.
    #[must_use]
    pub fn unmap_point(self, point: Vec2) -> Vec2 {
        if self.is_valid() {
            (point - self.offset) / self.scale
        } else {
            Vec2::ZERO
        }
    }

    /// The vector of the space spanning `vector` around it; zero in a collapsed space.
    #[must_use]
    pub fn unmap_vector(self, vector: Vec2) -> Vec2 {
        if self.is_valid() {
            vector / self.scale
        } else {
            Vec2::ZERO
        }
    }

    /// The rectangle of the space covering `rect` around it; empty in a collapsed space.
    #[must_use]
    pub fn unmap_rect(self, rect: Rect) -> Rect {
        Rect::from_min_max(self.unmap_point(rect.min), self.unmap_point(rect.max))
    }
}

/// A size in physical pixels: framebuffers, swapchain images, textures.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Extent {
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
}

impl Extent {
    /// Creates an extent.
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Returns `true` when either dimension is zero (e.g. a minimized window).
    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Number of pixels.
    pub const fn area(self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Converts to logical pixels for the given scale factor.
    #[must_use]
    pub fn to_logical(self, scale_factor: f32) -> Vec2 {
        Vec2::new(self.width as f32, self.height as f32) / scale_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec2_arithmetic_is_component_wise() {
        let a = Vec2::new(1.0, 2.0);
        let b = Vec2::new(3.0, -4.0);
        assert_eq!(a + b, Vec2::new(4.0, -2.0));
        assert_eq!(a - b, Vec2::new(-2.0, 6.0));
        assert_eq!(a * 2.0, Vec2::new(2.0, 4.0));
        assert_eq!(a * b, Vec2::new(3.0, -8.0));
        assert_eq!(b / 2.0, Vec2::new(1.5, -2.0));
        assert_eq!(-a, Vec2::new(-1.0, -2.0));
        // `hypot` has no specified precision (Miri adds an error of a few units in the last
        // place): compare with a tolerance (#123).
        assert!((Vec2::new(3.0, 4.0).length() - 5.0).abs() < 1e-5);
        assert!((a.dot(b) - (-5.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn rect_constructors_agree() {
        let a = Rect::from_origin_size(Vec2::new(10.0, 20.0), Vec2::new(100.0, 50.0));
        let b = Rect::from_center_size(Vec2::new(60.0, 45.0), Vec2::new(100.0, 50.0));
        assert_eq!(a, b);
        assert_eq!(a.size(), Vec2::new(100.0, 50.0));
        assert_eq!(a.center(), Vec2::new(60.0, 45.0));
    }

    #[test]
    fn rect_contains_is_min_inclusive_max_exclusive() {
        let r = Rect::from_min_max(Vec2::ZERO, Vec2::new(10.0, 10.0));
        assert!(r.contains(Vec2::ZERO));
        assert!(r.contains(Vec2::new(9.99, 9.99)));
        assert!(!r.contains(Vec2::new(10.0, 5.0)));
        assert!(!r.contains(Vec2::new(-0.01, 5.0)));
    }

    #[test]
    fn rect_intersection_handles_overlap_and_disjoint() {
        let a = Rect::from_min_max(Vec2::ZERO, Vec2::new(10.0, 10.0));
        let b = Rect::from_min_max(Vec2::new(5.0, 5.0), Vec2::new(15.0, 15.0));
        let c = Rect::from_min_max(Vec2::new(20.0, 20.0), Vec2::new(30.0, 30.0));
        assert_eq!(
            a.intersection(&b),
            Some(Rect::from_min_max(
                Vec2::new(5.0, 5.0),
                Vec2::new(10.0, 10.0)
            ))
        );
        assert_eq!(a.intersection(&c), None);
    }

    #[test]
    fn rect_inset_and_scale() {
        let r = Rect::from_min_max(Vec2::ZERO, Vec2::new(10.0, 10.0));
        assert_eq!(
            r.inset(2.0),
            Rect::from_min_max(Vec2::splat(2.0), Vec2::splat(8.0))
        );
        assert_eq!(
            r.inset(-1.0),
            Rect::from_min_max(Vec2::splat(-1.0), Vec2::splat(11.0))
        );
        assert!(r.inset(5.0).is_empty());
        assert_eq!(r.scale(2.0).size(), Vec2::splat(20.0));
    }

    #[test]
    fn spaces_map_and_unmap_points_rects_and_lengths() {
        let space = Space::new(2.0, Vec2::new(10.0, -4.0));
        assert_eq!(space.map_point(Vec2::new(3.0, 5.0)), Vec2::new(16.0, 6.0));
        assert_eq!(space.unmap_point(Vec2::new(16.0, 6.0)), Vec2::new(3.0, 5.0));
        assert_eq!(space.map_vector(Vec2::new(3.0, 5.0)), Vec2::new(6.0, 10.0));
        assert_eq!(
            space.unmap_vector(Vec2::new(6.0, 10.0)),
            Vec2::new(3.0, 5.0)
        );
        assert!((space.map_length(1.5) - 3.0).abs() < f32::EPSILON);
        let rect = Rect::from_origin_size(Vec2::new(1.0, 2.0), Vec2::new(4.0, 3.0));
        let mapped = space.map_rect(rect);
        assert_eq!(
            mapped,
            Rect::from_origin_size(Vec2::new(12.0, 0.0), Vec2::new(8.0, 6.0))
        );
        assert_eq!(space.unmap_rect(mapped), rect);
        assert_eq!(Space::default(), Space::IDENTITY);
        assert_eq!(Space::IDENTITY.map_rect(rect), rect);
    }

    #[test]
    fn nested_spaces_compose_inner_first() {
        let outer = Space::new(2.0, Vec2::new(100.0, 50.0));
        let inner = Space::new(0.25, Vec2::new(8.0, 4.0));
        let both = outer.then(inner);
        let point = Vec2::new(12.0, -20.0);
        assert_eq!(
            both.map_point(point),
            outer.map_point(inner.map_point(point))
        );
        assert!((both.scale - 0.5).abs() < f32::EPSILON);
        assert_eq!(Space::IDENTITY.then(inner), inner);
        assert_eq!(inner.then(Space::IDENTITY), inner);
    }

    #[test]
    fn spaces_that_cannot_be_drawn_collapse() {
        let good = Space::new(2.0, Vec2::ZERO);
        for bad in [
            Space::new(0.0, Vec2::ZERO),
            Space::new(-1.0, Vec2::ZERO),
            Space::new(f32::NAN, Vec2::ZERO),
            Space::new(f32::INFINITY, Vec2::ZERO),
            Space::new(1.0, Vec2::new(f32::NAN, 0.0)),
        ] {
            assert!(!bad.is_valid());
            assert_eq!(good.then(bad), Space::COLLAPSED);
            assert_eq!(bad.then(good), Space::COLLAPSED);
        }
        // A scale that overflows collapses too.
        let huge = Space::new(1.0e30, Vec2::ZERO);
        assert_eq!(huge.then(huge), Space::COLLAPSED);
        // Nothing comes back from a collapsed space but its origin.
        let collapsed = Space::COLLAPSED;
        assert_eq!(collapsed.unmap_point(Vec2::new(3.0, 4.0)), Vec2::ZERO);
        assert_eq!(collapsed.unmap_vector(Vec2::new(3.0, 4.0)), Vec2::ZERO);
        assert!(
            collapsed
                .map_rect(Rect::from_origin_size(Vec2::ZERO, Vec2::splat(9.0)))
                .is_empty()
        );
    }

    #[test]
    fn extent_reports_emptiness_and_logical_size() {
        assert!(Extent::new(0, 600).is_empty());
        assert!(!Extent::new(800, 600).is_empty());
        assert_eq!(Extent::new(800, 600).area(), 480_000);
        assert_eq!(
            Extent::new(800, 600).to_logical(2.0),
            Vec2::new(400.0, 300.0)
        );
    }
}
