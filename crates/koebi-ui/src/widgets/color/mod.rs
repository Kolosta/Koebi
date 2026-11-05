//! Colors: the color field ([`ColorField`]) and the picker it opens ([`ColorPicker`]), with the
//! strips ([`ColorStrip`]) and the shapes ([`PickerShape`]) it is made of.
//!
//! - **The field**: one interface unit high, outlined in a neutral gray of its own
//!   (`color-field.outline.*`, which stays when the interface's borders go); each half shows
//!   the color's transparency its own way ([`Transparency`]). A click opens the picker below
//!   it, their corners meeting (a dropdown), or as a panel of its own: a menu's panel either
//!   way, kept on screen.
//! - **The picker**: an area of two components (a square, a wheel, a ring holding a triangle
//!   or a square) reaching its real corners under rounded ones, the strip of the third and the
//!   strip of the opacity, each marked by two triangles; the current, previous and original
//!   colors; the switch of the model its fields edit (RGB, HSV, HSL, OKLCH, CMYK, hex) and
//!   those fields, grouped ([`crate::FieldGroup`]); a palette. Its parts, its title and its
//!   palette are the caller's to choose. Dragging hides the pointer, which comes back at the
//!   mark; Shift makes it finer; Escape brings the color back. Every change shows at once.

mod field;
mod picker;
mod shape;
mod strip;

use koebi_core::Color;
use koebi_design::Resolve;
use koebi_tokens::component;

pub use field::{ColorField, ColorFieldResponse, ColorFieldState, Transparency};
pub use picker::{ColorMode, ColorPicker, ColorPickerState, PICKER_SECTIONS, PickerSection};
pub use shape::{OKLCH_CHROMA, PickerShape, RING_INNER};
pub use strip::{ColorStrip, ColorStripState, StripFill, StripResponse};

/// Colors and metrics of color fields, from the `component.color-field.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorFieldStyle {
    /// Outline color.
    pub outline: Color,
    /// Outline color, hovered or open.
    pub outline_hovered: Color,
    /// Outline width.
    pub outline_width: f32,
    /// Corner radius.
    pub corner_radius: f32,
    /// Height.
    pub height: f32,
    /// Side of the squares of the checkerboard.
    pub checker_size: f32,
    /// Light squares.
    pub checker_light: Color,
    /// Dark squares.
    pub checker_dark: Color,
}

impl ColorFieldStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        Self {
            outline: tokens.get(c::COLOR_FIELD_OUTLINE_COLOR_DEFAULT),
            outline_hovered: tokens.get(c::COLOR_FIELD_OUTLINE_COLOR_HOVER),
            outline_width: tokens.get(c::COLOR_FIELD_OUTLINE_WIDTH_DEFAULT),
            corner_radius: tokens.get(c::COLOR_FIELD_CORNER_RADIUS_DEFAULT),
            height: tokens.get(c::COLOR_FIELD_HEIGHT_DEFAULT),
            checker_size: tokens.get(c::COLOR_FIELD_CHECKER_SIZE_DEFAULT),
            checker_light: tokens.get(c::COLOR_FIELD_CHECKER_COLOR_LIGHT),
            checker_dark: tokens.get(c::COLOR_FIELD_CHECKER_COLOR_DARK),
        }
    }
}

/// Colors and metrics of color pickers, from the `component.color-picker.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorPickerStyle {
    /// Side of the area.
    pub area_size: f32,
    /// Width of a strip.
    pub strip_width: f32,
    /// Width of the column of swatches.
    pub swatch_width: f32,
    /// Space between the parts.
    pub gap: f32,
    /// Corner radius of the area, strips and swatches.
    pub corner_radius: f32,
    /// Side of the mark on the area.
    pub marker_size: f32,
    /// Color of the marks.
    pub marker: Color,
    /// Color of the outline around the marks.
    pub marker_outline: Color,
    /// Width of that outline.
    pub outline_width: f32,
    /// Width of the white ring marking the color on the area.
    pub ring_width: f32,
    /// Depth of the triangles of a strip.
    pub arrow_size: f32,
    /// How far the triangles pass the strip's sides.
    pub arrow_overhang: f32,
    /// Share of the pointer's motion a drag takes with Shift.
    pub fine_factor: f32,
}

impl ColorPickerStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        Self {
            area_size: tokens.get(c::COLOR_PICKER_AREA_SIZE_DEFAULT),
            strip_width: tokens.get(c::COLOR_PICKER_STRIP_WIDTH_DEFAULT),
            swatch_width: tokens.get(c::COLOR_PICKER_SWATCH_WIDTH_DEFAULT),
            gap: tokens.get(c::COLOR_PICKER_GAP_DEFAULT),
            corner_radius: tokens.get(c::COLOR_PICKER_CORNER_RADIUS_DEFAULT),
            marker_size: tokens.get(c::COLOR_PICKER_MARKER_SIZE_DEFAULT),
            marker: tokens.get(c::COLOR_PICKER_MARKER_COLOR_DEFAULT),
            marker_outline: tokens.get(c::COLOR_PICKER_MARKER_OUTLINE_COLOR_DEFAULT),
            outline_width: tokens.get(c::COLOR_PICKER_MARKER_OUTLINE_WIDTH_DEFAULT),
            ring_width: tokens.get(c::COLOR_PICKER_MARKER_RING_WIDTH_DEFAULT),
            arrow_size: tokens.get(c::COLOR_PICKER_ARROW_SIZE_DEFAULT),
            arrow_overhang: tokens.get(c::COLOR_PICKER_ARROW_OVERHANG_DEFAULT),
            fine_factor: tokens.get(c::COLOR_PICKER_FINE_FACTOR_DEFAULT),
        }
    }
}
