//! [`FillPrimitive`]: a shape whose color the fragment shader computes for each pixel, for what
//! a single color cannot show: the gradients of color pickers (a saturation and value square,
//! strips of hues, a hue wheel, a ring, a triangle) and the checkerboard behind transparency.
//!
//! Every fill but [`Fill::Corners`] computes its colors from a color model in the shader
//! (`shaders/fill.frag`); what a picker reads back from a position is computed the same way on
//! the CPU, from the same formulas ([`koebi_core::Hsv`], [`koebi_core::Hsl`],
//! [`koebi_core::Oklch`]). Angles follow one convention: hue 0 (red) at the top, growing
//! clockwise.

use koebi_core::{Color, Rect};
use koebi_gpu::Pod;

use crate::scene::{Corners, Edges};

/// The color model of a wheel, a ring or a strip of hues.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HueModel {
    /// Hue, saturation and value: pure hues at full saturation, the classic wheel. Its
    /// secondary colors (yellow, cyan, magenta) look much brighter than its primaries: they
    /// show as spokes.
    Hsv,
    /// Hue, saturation and lightness: the pure hues halfway between black and white.
    Hsl,
    /// OKLCH: every hue at one lightness and chroma, perceptually even, without spokes, as
    /// the wheel of a picker made for the eye.
    Oklch,
}

impl HueModel {
    const fn code(self) -> f32 {
        match self {
            Self::Hsv => 0.0,
            Self::Hsl => 1.0,
            Self::Oklch => 2.0,
        }
    }
}

/// The side a triangle points toward.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pointing {
    /// Toward the left.
    Left,
    /// Toward the right.
    Right,
    /// Up.
    Up,
    /// Down.
    Down,
}

impl Pointing {
    const fn code(self) -> f32 {
        match self {
            Self::Left => 0.0,
            Self::Right => 1.0,
            Self::Up => 2.0,
            Self::Down => 3.0,
        }
    }
}

/// How a [`FillPrimitive`] colors its shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fill {
    /// The four corner colors blended across the shape, component by component as they are
    /// encoded (straight alpha): exactly the saturation and value square of a hue (white,
    /// the hue, black, black), and the strips of value, lightness or opacity.
    Corners {
        /// The top left corner's color.
        top_left: Color,
        /// The top right corner's.
        top_right: Color,
        /// The bottom right corner's.
        bottom_right: Color,
        /// The bottom left corner's.
        bottom_left: Color,
    },
    /// A checkerboard of squares `size` logical pixels wide, `light` at the top left: what
    /// shows through a color's transparency.
    Checker {
        /// The side of a square.
        size: f32,
        /// The color of every other square, from the top left.
        light: Color,
        /// The color of the others.
        dark: Color,
    },
    /// Every hue along the shape's length (from the top or the left), at full saturation, or
    /// at `lightness` and `chroma` for [`HueModel::Oklch`].
    Hues {
        /// The model of the hues.
        model: HueModel,
        /// Along its height, or along its width.
        vertical: bool,
        /// The lightness of OKLCH hues.
        lightness: f32,
        /// The chroma of OKLCH hues.
        chroma: f32,
    },
    /// Every hue along the width and every saturation along the height (full at the top, gray
    /// at the bottom), at one HSV `value`.
    HueSaturation {
        /// The value.
        value: f32,
    },
    /// A disc of every hue around (red at the top, clockwise) and every saturation outward,
    /// at a `level`: the value in HSV, the lightness in HSL and OKLCH, whose chroma grows
    /// outward up to `chroma`.
    Wheel {
        /// The model.
        model: HueModel,
        /// The third component: value or lightness.
        level: f32,
        /// The chroma at the edge, in OKLCH.
        chroma: f32,
    },
    /// A ring of every hue around (red at the top, clockwise), at full saturation, or at
    /// `lightness` and `chroma` in OKLCH; `inner` its inner radius over its outer one.
    Ring {
        /// The model.
        model: HueModel,
        /// The inner radius over the outer one.
        inner: f32,
        /// The lightness of OKLCH hues.
        lightness: f32,
        /// The chroma of OKLCH hues.
        chroma: f32,
    },
    /// The triangle of one HSV `hue` inscribed in the circle of the shape: the pure hue at
    /// its corner pointing at the hue's angle, white and black at the next ones clockwise.
    Triangle {
        /// The hue, in turns.
        hue: f32,
    },
    /// A triangle filling the shape, its tip in the middle of the side it points toward, in
    /// `color` and outlined in `outline`: the marks of a picker's strips.
    Arrow {
        /// Where its tip points.
        pointing: Pointing,
        /// Inside.
        color: Color,
        /// Along its edges, inside them.
        outline: Color,
        /// The outline's width, in logical pixels.
        outline_width: f32,
    },
}

/// A shape whose color is computed for each pixel ([`Fill`]), with a radius per corner for the
/// fills that cover their rectangle (corners, checkerboards, hues, planes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FillPrimitive {
    /// Bounds in logical pixels.
    pub rect: Rect,
    /// How it is colored.
    pub fill: Fill,
    /// Corner radii in logical pixels of rectangular fills.
    pub corner_radii: Corners,
    /// How its straight edges meet the pixel grid.
    pub edges: Edges,
}

impl FillPrimitive {
    /// `fill` over `rect`, square corners, edges on the pixel grid.
    pub const fn new(rect: Rect, fill: Fill) -> Self {
        Self {
            rect,
            fill,
            corner_radii: Corners::ZERO,
            edges: Edges::Snapped,
        }
    }

    /// The same fill, a radius per corner.
    #[must_use]
    pub const fn with_corner_radii(mut self, radii: Corners) -> Self {
        self.corner_radii = radii;
        self
    }

    /// The same radius at every corner.
    #[must_use]
    pub const fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radii = Corners::all(radius);
        self
    }

    /// The same fill, its straight edges meeting the pixel grid as `edges` says: exact for
    /// marks that move continuously.
    #[must_use]
    pub const fn with_edges(mut self, edges: Edges) -> Self {
        self.edges = edges;
        self
    }
}

/// GPU layout of one fill (vertex buffer, per-instance step).
///
/// Must match the inputs of `shaders/fill.vert`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub(crate) struct FillInstance {
    /// `x, y, width, height` in logical pixels.
    pub(crate) rect: [f32; 4],
    /// Corner radii in logical pixels: top-left, top-right, bottom-right, bottom-left.
    pub(crate) radii: [f32; 4],
    /// Four colors, gamma-encoded with straight alpha: corners, or the checkerboard's two.
    pub(crate) colors: [[f32; 4]; 4],
    /// The kind of fill, then three of its parameters.
    pub(crate) params: [f32; 4],
    /// 1 for exact edges, then a fourth parameter.
    pub(crate) options: [f32; 4],
}

// SAFETY: `repr(C)` struct of eight `[f32; 4]` arrays: 128 bytes with no padding, and every bit
// pattern is a valid value.
unsafe impl Pod for FillInstance {}
const _: () = assert!(size_of::<FillInstance>() == 128);

impl FillInstance {
    pub(crate) const STRIDE: u32 = 128;

    pub(crate) fn from_primitive(primitive: &FillPrimitive) -> Self {
        let r = primitive.rect;
        let straight = |color: Color| [color.r, color.g, color.b, color.a];
        let none = [0.0; 4];
        let (colors, params, extra) = match primitive.fill {
            Fill::Corners {
                top_left,
                top_right,
                bottom_right,
                bottom_left,
            } => (
                [top_left, top_right, bottom_right, bottom_left].map(straight),
                [0.0, 0.0, 0.0, 0.0],
                0.0,
            ),
            Fill::Checker { size, light, dark } => (
                [straight(light), straight(dark), none, none],
                [1.0, size.max(1.0), 0.0, 0.0],
                0.0,
            ),
            Fill::Hues {
                model,
                vertical,
                lightness,
                chroma,
            } => (
                [none; 4],
                [2.0, model.code(), f32::from(u8::from(vertical)), lightness],
                chroma,
            ),
            Fill::HueSaturation { value } => ([none; 4], [3.0, value, 0.0, 0.0], 0.0),
            Fill::Wheel {
                model,
                level,
                chroma,
            } => ([none; 4], [4.0, model.code(), level, chroma], 0.0),
            Fill::Ring {
                model,
                inner,
                lightness,
                chroma,
            } => (
                [none; 4],
                [5.0, model.code(), inner.clamp(0.0, 1.0), lightness],
                chroma,
            ),
            Fill::Triangle { hue } => ([none; 4], [6.0, hue, 0.0, 0.0], 0.0),
            Fill::Arrow {
                pointing,
                color,
                outline,
                outline_width,
            } => (
                [straight(color), straight(outline), none, none],
                [7.0, pointing.code(), outline_width.max(0.0), 0.0],
                0.0,
            ),
        };
        Self {
            rect: [r.min.x, r.min.y, r.width(), r.height()],
            radii: primitive
                .corner_radii
                .to_array()
                .map(|radius| radius.max(0.0)),
            colors,
            params,
            options: [
                f32::from(u8::from(primitive.edges == Edges::Exact)),
                extra,
                0.0,
                0.0,
            ],
        }
    }
}
