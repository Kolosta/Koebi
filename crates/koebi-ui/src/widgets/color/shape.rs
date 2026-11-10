//! The shapes a color picker can take, and how a position in their area maps to a color and
//! back: pure functions, the same formulas as the fills that paint them
//! ([`koebi_render::Fill`]).
//!
//! Positions are in the area's unit square: `(0, 0)` its top left, `(1, 1)` its bottom right.
//! Wheels and rings put hue 0 (red) at the top, growing clockwise.

use core::f32::consts::TAU;
use koebi_i18n::tr;

use koebi_core::{Hsl, Hsv, Oklch, Vec2};
use koebi_render::HueModel;

/// The chroma at the edge of the OKLCH wheel: past it, most hues leave the sRGB gamut.
pub const OKLCH_CHROMA: f32 = 0.2;

/// The inner radius of rings over their outer one.
pub const RING_INNER: f32 = 0.8;

/// How a color picker shows the color: an area of two components, and a strip of the third
/// when the area does not hold it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PickerShape {
    /// Saturation across and value up in a square, hue in a strip: Photoshop's and
    /// `DaVinci Resolve`'s. The square is exactly the HSV slice of a hue.
    #[default]
    SquareSv,
    /// Hue across and saturation up in a square, value in a strip: Blender's "HS + V".
    SquareHs,
    /// Hue around and saturation outward in a disc, value in a strip: Blender's HSV circle.
    /// Its secondary colors look brighter than its primaries: they show as spokes.
    WheelHsv,
    /// The same disc in HSL, lightness in a strip: Blender's HSL circle, the pure hues
    /// halfway between black and white.
    WheelHsl,
    /// Hue around and chroma outward at one OKLCH lightness, lightness in a strip: every hue
    /// as light as the others, a disc without spokes, made for the eye.
    WheelOklch,
    /// Hue on a ring, saturation and value in the triangle it holds: GIMP's and Krita's.
    RingTriangle,
    /// Hue on a ring, saturation and value in the square it holds.
    RingSquare,
}

impl PickerShape {
    /// Every shape, in the order a list offers them.
    pub const ALL: [Self; 7] = [
        Self::SquareSv,
        Self::SquareHs,
        Self::WheelHsv,
        Self::WheelHsl,
        Self::WheelOklch,
        Self::RingTriangle,
        Self::RingSquare,
    ];

    /// Its name, for a list.
    pub fn label(self) -> &'static str {
        match self {
            Self::SquareSv => tr("Square: saturation and value"),
            Self::SquareHs => tr("Square: hue and saturation"),
            Self::WheelHsv => tr("Wheel: HSV"),
            Self::WheelHsl => tr("Wheel: HSL"),
            Self::WheelOklch => tr("Wheel: OKLCH, perceptual"),
            Self::RingTriangle => tr("Ring and triangle"),
            Self::RingSquare => tr("Ring and square"),
        }
    }

    /// `true` for the shapes whose area holds the hue on a ring around another shape.
    pub const fn has_ring(self) -> bool {
        matches!(self, Self::RingTriangle | Self::RingSquare)
    }

    /// `true` for the shapes with a strip of their third component next to the area (every
    /// one but the rings, whose ring is that component).
    pub const fn has_strip(self) -> bool {
        !self.has_ring()
    }

    /// The model of its hues, for the fills.
    pub const fn hue_model(self) -> HueModel {
        match self {
            Self::WheelHsl => HueModel::Hsl,
            Self::WheelOklch => HueModel::Oklch,
            _ => HueModel::Hsv,
        }
    }
}

/// What a color picker edits: the components of its shape, kept apart so that the hue stays
/// where it was when the saturation or the value reaches zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Components {
    /// HSV, for every shape but the OKLCH wheel.
    pub(crate) hsv: Hsv,
    /// HSL lightness and saturation, for the HSL wheel.
    pub(crate) hsl: Hsl,
    /// OKLCH, for the OKLCH wheel.
    pub(crate) oklch: Oklch,
}

/// The point of the area of `shape` that `components` lie at.
pub(crate) fn area_point(shape: PickerShape, components: &Components) -> Vec2 {
    let Components { hsv, hsl, oklch } = *components;
    match shape {
        PickerShape::SquareSv => Vec2::new(hsv.s, 1.0 - hsv.v),
        PickerShape::SquareHs => Vec2::new(hsv.h, 1.0 - hsv.s),
        PickerShape::WheelHsv => polar(hsv.h, hsv.s),
        PickerShape::WheelHsl => polar(hsl.h, hsl.s),
        PickerShape::WheelOklch => polar(oklch.h / 360.0, oklch.c / OKLCH_CHROMA),
        PickerShape::RingTriangle => triangle_point(hsv),
        PickerShape::RingSquare => {
            let half = RING_INNER * core::f32::consts::FRAC_1_SQRT_2 * 0.5;
            Vec2::new(
                0.5 - half + 2.0 * half * hsv.s,
                0.5 + half - 2.0 * half * hsv.v,
            )
        }
    }
}

/// The point of a ring at `hue`, halfway through its thickness.
pub(crate) fn ring_point(hue: f32) -> Vec2 {
    polar(hue, f32::midpoint(1.0, RING_INNER))
}

/// `true` when `point` of the area of a ring shape lies on its ring.
pub(crate) fn on_ring(point: Vec2) -> bool {
    let from = (point - Vec2::splat(0.5)) * 2.0;
    from.length() >= RING_INNER
}

/// The hue (turns) of the direction of `point` from the area's center: 0 at the top,
/// clockwise.
pub(crate) fn hue_at(point: Vec2) -> f32 {
    let from = point - Vec2::splat(0.5);
    (from.x.atan2(-from.y) / TAU).rem_euclid(1.0)
}

/// `components` with the two of the area of `shape` taken from `point` (clamped to the
/// area: its corners included, past a rounded frame).
pub(crate) fn from_area(shape: PickerShape, components: &Components, point: Vec2) -> Components {
    let point = Vec2::new(point.x.clamp(0.0, 1.0), point.y.clamp(0.0, 1.0));
    let mut next = *components;
    match shape {
        PickerShape::SquareSv => {
            next.hsv.s = point.x;
            next.hsv.v = 1.0 - point.y;
        }
        PickerShape::SquareHs => {
            next.hsv.h = point.x.min(0.9999);
            next.hsv.s = 1.0 - point.y;
        }
        PickerShape::WheelHsv => {
            next.hsv.h = hue_at(point);
            next.hsv.s = radius_at(point);
        }
        PickerShape::WheelHsl => {
            next.hsl.h = hue_at(point);
            next.hsl.s = radius_at(point);
        }
        PickerShape::WheelOklch => {
            next.oklch.h = hue_at(point) * 360.0;
            next.oklch.c = radius_at(point) * OKLCH_CHROMA;
        }
        PickerShape::RingTriangle => {
            let (s, v) = triangle_sv(next.hsv.h, point);
            next.hsv.s = s;
            next.hsv.v = v;
        }
        PickerShape::RingSquare => {
            let half = RING_INNER * core::f32::consts::FRAC_1_SQRT_2 * 0.5;
            let low = 0.5 - half;
            next.hsv.s = ((point.x - low) / (2.0 * half)).clamp(0.0, 1.0);
            next.hsv.v = (1.0 - (point.y - low) / (2.0 * half)).clamp(0.0, 1.0);
        }
    }
    next
}

/// The point at `hue` (turns) and `radius` (0 the center, 1 the edge) of a disc.
fn polar(hue: f32, radius: f32) -> Vec2 {
    let (sin, cos) = (hue * TAU).sin_cos();
    let radius = radius.clamp(0.0, 1.0) * 0.5;
    Vec2::new(0.5 + radius * sin, 0.5 - radius * cos)
}

/// How far `point` lies from the center of a disc, 1 at its edge, at most 1.
fn radius_at(point: Vec2) -> f32 {
    ((point - Vec2::splat(0.5)) * 2.0).length().min(1.0)
}

/// The corners of the triangle of `hue` inside a ring: the pure hue, white, black.
fn triangle_corners(hue: f32) -> [Vec2; 3] {
    let radius = RING_INNER * 0.5;
    [0.0, 1.0 / 3.0, -1.0 / 3.0].map(|turn| {
        let (sin, cos) = ((hue + turn) * TAU).sin_cos();
        Vec2::new(0.5 + radius * sin, 0.5 - radius * cos)
    })
}

/// The point of the triangle of `hsv`'s hue at its saturation and value: weights of the pure
/// hue `s·v`, of white `(1 − s)·v`, of black `1 − v`.
fn triangle_point(hsv: Hsv) -> Vec2 {
    let [hue, white, black] = triangle_corners(hsv.h);
    let (s, v) = (hsv.s.clamp(0.0, 1.0), hsv.v.clamp(0.0, 1.0));
    hue * (s * v) + white * ((1.0 - s) * v) + black * (1.0 - v)
}

/// The saturation and value at `point` of the triangle of `hue`, the point taken back inside
/// the triangle.
fn triangle_sv(hue: f32, point: Vec2) -> (f32, f32) {
    let [pure_at, white_at, black_at] = triangle_corners(hue);
    let (edge, other, from) = (white_at - pure_at, black_at - pure_at, point - pure_at);
    let den = edge.x * other.y - other.x * edge.y;
    let wb = (from.x * other.y - other.x * from.y) / den;
    let wc = (edge.x * from.y - from.x * edge.y) / den;
    let weights = [1.0 - wb - wc, wb, wc].map(|weight: f32| weight.max(0.0));
    let total = weights.iter().sum::<f32>().max(1e-6);
    let [pure, white, _] = weights.map(|weight| weight / total);
    let v = pure + white;
    let s = if v <= 1e-6 { 0.0 } else { pure / v };
    (s.clamp(0.0, 1.0), v.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn components(h: f32, s: f32, v: f32) -> Components {
        let hsv = Hsv { h, s, v };
        let color = hsv.to_color(1.0);
        Components {
            hsv,
            hsl: Hsl::from_color(color),
            oklch: Oklch::new(0.7, 0.1, h * 360.0),
        }
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn every_shape_gives_back_the_components_of_its_point() {
        let start = components(0.3, 0.6, 0.8);
        for shape in PickerShape::ALL {
            let point = area_point(shape, &start);
            let back = from_area(shape, &start, point);
            match shape {
                PickerShape::WheelOklch => {
                    assert!(close(back.oklch.h, start.oklch.h), "{shape:?}");
                    assert!(close(back.oklch.c, start.oklch.c), "{shape:?}");
                }
                PickerShape::WheelHsl => {
                    assert!(close(back.hsl.h, start.hsl.h), "{shape:?}");
                    assert!(close(back.hsl.s, start.hsl.s), "{shape:?}");
                }
                _ => {
                    assert!(close(back.hsv.s, start.hsv.s), "{shape:?} {back:?}");
                    assert!(close(back.hsv.v, start.hsv.v), "{shape:?} {back:?}");
                    assert!(close(back.hsv.h, start.hsv.h), "{shape:?}");
                }
            }
        }
    }

    #[test]
    fn squares_reach_their_real_corners_and_wheels_their_edge() {
        let start = components(0.0, 0.5, 0.5);
        // Past a corner: clamped to the corner itself, not to a rounded frame.
        let corner = from_area(PickerShape::SquareSv, &start, Vec2::new(1.4, -0.3));
        assert_eq!((corner.hsv.s, corner.hsv.v), (1.0, 1.0));
        let black = from_area(PickerShape::SquareSv, &start, Vec2::new(-1.0, 2.0));
        assert_eq!((black.hsv.s, black.hsv.v), (0.0, 0.0));
        // Red at the top, a quarter turn clockwise at the right.
        assert!(close(hue_at(Vec2::new(0.5, 0.0)), 0.0));
        assert!(close(hue_at(Vec2::new(1.0, 0.5)), 0.25));
        let edge = from_area(PickerShape::WheelHsv, &start, Vec2::new(0.5, -0.5));
        assert_eq!(edge.hsv.s, 1.0);
        assert!(on_ring(ring_point(0.7)) && !on_ring(Vec2::splat(0.5)));
    }

    #[test]
    fn the_triangle_has_its_hue_white_and_black_at_its_corners() {
        let [pure, white, black] = triangle_corners(0.0);
        let sv = |point: Vec2| triangle_sv(0.0, point);
        let (s, v) = sv(pure);
        assert!(close(s, 1.0) && close(v, 1.0));
        let (s, v) = sv(white);
        assert!(close(s, 0.0) && close(v, 1.0));
        assert!(close(sv(black).1, 0.0));
        // Outside, taken back to its edge.
        let (s, v) = sv(Vec2::new(0.5, -1.0));
        assert!((0.0..=1.0).contains(&s) && (0.0..=1.0).contains(&v));
    }
}
