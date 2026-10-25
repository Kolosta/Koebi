//! Glyph rasterization: outlines to 8-bit coverage with exact area anti-aliasing.
//!
//! Each line segment adds, to every pixel it crosses, the signed area it covers to its right
//! in an accumulation buffer; a running sum along each row then gives each pixel's coverage.
//! This is the technique of Raph Levien's `font-rs`: exact for straight edges, with curves
//! flattened finely enough to be indistinguishable. Overlapping contours of the same direction
//! saturate at full coverage, and opposite directions cancel, as the nonzero rule requires.

use koebi_core::Vec2;

use crate::font::{Font, GlyphId};
use crate::outline::{Outline, PathCommand};
use crate::{Error, Result};

/// The largest bitmap side, in pixels: larger glyphs are refused.
pub const MAX_GLYPH_SIZE: u32 = 1024;

/// A rasterized glyph: coverage rows, placed relative to the pen position.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GlyphBitmap {
    /// Horizontal offset of the first column from the pen position, in pixels.
    pub left: i32,
    /// Vertical offset of the first row from the baseline, in pixels, y down (negative above
    /// the baseline).
    pub top: i32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width × height` coverage values, row after row: 0 outside, 255 inside.
    pub coverage: Vec<u8>,
}

impl GlyphBitmap {
    /// `true` when the glyph covers no pixel (spaces).
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// The coverage of pixel (`x`, `y`), 0 outside the bitmap.
    pub fn coverage_at(&self, x: u32, y: u32) -> u8 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        let index = u64::from(y) * u64::from(self.width) + u64::from(x);
        usize::try_from(index)
            .ok()
            .and_then(|index| self.coverage.get(index))
            .copied()
            .unwrap_or(0)
    }
}

/// Rasterizes glyphs. Reuse one: its buffers grow to the largest glyph and stay allocated.
#[derive(Clone, Debug, Default)]
pub struct Rasterizer {
    outline: Outline,
    accumulation: Vec<f32>,
}

impl Rasterizer {
    /// A rasterizer with empty buffers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Rasterizes `glyph` at `size` pixels per em into `bitmap`, its pen position shifted right
    /// by `offset_x` pixels (a fraction, for subpixel positioning).
    ///
    /// # Errors
    ///
    /// Returns the font's errors for broken glyphs, and [`Error::TooComplex`] for bitmaps
    /// larger than [`MAX_GLYPH_SIZE`] or non-finite sizes. The bitmap is left empty on error.
    pub fn rasterize(
        &mut self,
        font: &Font,
        glyph: GlyphId,
        size: f32,
        offset_x: f32,
        bitmap: &mut GlyphBitmap,
    ) -> Result<()> {
        *bitmap = GlyphBitmap {
            coverage: std::mem::take(&mut bitmap.coverage),
            ..GlyphBitmap::default()
        };
        bitmap.coverage.clear();
        if !size.is_finite() || !offset_x.is_finite() || size <= 0.0 {
            return Err(Error::TooComplex);
        }
        self.outline.load(font, glyph)?;
        let Some(bounds) = self.outline.bounds() else {
            return Ok(());
        };
        let scale = size / f32::from(font.units_per_em());
        // Pixel space: x right from the pen, y down from the baseline.
        let left = (bounds.min.x * scale + offset_x).floor();
        let right = (bounds.max.x * scale + offset_x).ceil();
        let top = (-bounds.max.y * scale).floor();
        let bottom = (-bounds.min.y * scale).ceil();
        let (width, height) = (right - left, bottom - top);
        let limit = MAX_GLYPH_SIZE as f32;
        if !(0.0..=limit).contains(&width) || !(0.0..=limit).contains(&height) {
            return Err(Error::TooComplex);
        }
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "whole numbers between 0 and MAX_GLYPH_SIZE, checked above"
        )]
        let (width_px, height_px) = (width as u32, height as u32);
        if width_px == 0 || height_px == 0 {
            return Ok(());
        }
        #[allow(
            clippy::cast_possible_truncation,
            reason = "whole numbers within a glyph's reach of the origin, at most a few thousand"
        )]
        let (left_px, top_px) = (left as i32, top as i32);

        let canvas = Canvas::new(&mut self.accumulation, width_px, height_px);
        let to_pixels =
            |point: Vec2| Vec2::new(point.x * scale + offset_x - left, -point.y * scale - top);
        let mut canvas = canvas;
        let mut current = Vec2::default();
        for command in self.outline.commands() {
            match *command {
                PathCommand::MoveTo(point) => current = to_pixels(point),
                PathCommand::LineTo(point) => {
                    let next = to_pixels(point);
                    canvas.line(current, next);
                    current = next;
                }
                PathCommand::QuadTo(control, point) => {
                    let next = to_pixels(point);
                    canvas.quad(current, to_pixels(control), next);
                    current = next;
                }
                PathCommand::Close => {}
            }
        }
        canvas.resolve(&mut bitmap.coverage);
        bitmap.left = left_px;
        bitmap.top = top_px;
        bitmap.width = width_px;
        bitmap.height = height_px;
        Ok(())
    }
}

/// The accumulation buffer of one glyph.
struct Canvas<'a> {
    cells: &'a mut Vec<f32>,
    width: usize,
    height: usize,
    /// `width` as a signed index step.
    stride: isize,
}

impl<'a> Canvas<'a> {
    fn new(cells: &'a mut Vec<f32>, width: u32, height: u32) -> Self {
        let (width, height) = (width as usize, height as usize);
        cells.clear();
        // One spare row: segments touching the right edge spill into the next cell.
        cells.resize(width * (height + 1) + 1, 0.0);
        Self {
            cells,
            width,
            height,
            // At most MAX_GLYPH_SIZE: always representable.
            stride: isize::try_from(width).unwrap_or(isize::MAX),
        }
    }

    fn add(&mut self, index: isize, value: f32) {
        if let Some(cell) = usize::try_from(index)
            .ok()
            .and_then(|index| self.cells.get_mut(index))
        {
            *cell += value;
        }
    }

    /// Accumulates the signed area right of the segment from `from` to `to`.
    fn line(&mut self, from: Vec2, to: Vec2) {
        let finite = [from.x, from.y, to.x, to.y].iter().all(|v| v.is_finite());
        #[allow(
            clippy::float_cmp,
            reason = "a horizontal segment, exactly, covers no area"
        )]
        let horizontal = from.y == to.y;
        if !finite || horizontal {
            return;
        }
        let (direction, p0, p1) = if from.y < to.y {
            (1.0, from, to)
        } else {
            (-1.0, to, from)
        };
        let slope = (p1.x - p0.x) / (p1.y - p0.y);
        let mut x = p0.x;
        if p0.y < 0.0 {
            x -= p0.y * slope;
        }
        let height = self.height as f32;
        let first_row = p0.y.max(0.0).floor();
        let end_row = p1.y.min(height).ceil();
        let mut row = first_row;
        while row < end_row {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "a row index between 0 and the bitmap height"
            )]
            let row_start = (row as isize) * self.stride;
            let dy = (row + 1.0).min(p1.y) - row.max(p0.y);
            let x_next = x + slope * dy;
            let area = dy * direction;
            let (x0, x1) = if x < x_next { (x, x_next) } else { (x_next, x) };
            let x0_floor = x0.floor();
            let x1_ceil = x1.ceil();
            #[allow(
                clippy::cast_possible_truncation,
                reason = "columns within the bitmap, clamped by `add`"
            )]
            let (x0i, x1i) = (x0_floor as isize, x1_ceil as isize);
            if x1i <= x0i + 1 {
                // Within one column: split by the segment's mean position.
                let mean = f32::midpoint(x, x_next) - x0_floor;
                self.add(row_start + x0i, area - area * mean);
                self.add(row_start + x0i + 1, area * mean);
            } else {
                let inverse = 1.0 / (x1 - x0);
                let x0_fraction = x0 - x0_floor;
                let first = 0.5 * inverse * (1.0 - x0_fraction) * (1.0 - x0_fraction);
                let x1_fraction = x1 - x1_ceil + 1.0;
                let last = 0.5 * inverse * x1_fraction * x1_fraction;
                self.add(row_start + x0i, area * first);
                if x1i == x0i + 2 {
                    self.add(row_start + x0i + 1, area * (1.0 - first - last));
                } else {
                    let second = inverse * (1.5 - x0_fraction);
                    self.add(row_start + x0i + 1, area * (second - first));
                    // Columns past the right edge add nothing a row could show.
                    let right_edge = self.stride + 1;
                    for column in (x0i + 2).max(0)..(x1i - 1).min(right_edge) {
                        self.add(row_start + column, area * inverse);
                    }
                    let before_last = second + (x1i - x0i - 3) as f32 * inverse;
                    self.add(row_start + x1i - 1, area * (1.0 - before_last - last));
                }
                self.add(row_start + x1i, area * last);
            }
            x = x_next;
            row += 1.0;
        }
    }

    /// Accumulates a quadratic curve, flattened into segments.
    fn quad(&mut self, from: Vec2, control: Vec2, to: Vec2) {
        let deviation = from - control * 2.0 + to;
        let deviation = deviation.dot(deviation);
        if deviation < 0.333 {
            self.line(from, to);
            return;
        }
        // Enough segments that the flattening error stays far below a pixel.
        let tolerance = 3.0;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a small positive count: glyphs are at most MAX_GLYPH_SIZE pixels wide"
        )]
        let segments = (1.0 + (tolerance * deviation).sqrt().sqrt().floor()).min(256.0) as u32;
        let mut previous = from;
        for step in 1..=segments {
            let t = step as f32 / segments as f32;
            let u = 1.0 - t;
            let point = from * (u * u) + control * (2.0 * u * t) + to * (t * t);
            self.line(previous, point);
            previous = point;
        }
    }

    /// Turns the accumulated areas into coverage.
    fn resolve(self, coverage: &mut Vec<u8>) {
        let count = self.width * self.height;
        coverage.clear();
        coverage.reserve(count);
        let mut sum = 0.0_f32;
        for cell in &self.cells[..count] {
            sum += cell;
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "rounded and clamped to 0..=255"
            )]
            coverage.push((sum.abs().min(1.0) * 255.0 + 0.5) as u8);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw(width: u32, height: u32, path: &[Vec2]) -> Vec<u8> {
        let mut cells = Vec::new();
        let mut canvas = Canvas::new(&mut cells, width, height);
        for pair in path.windows(2) {
            canvas.line(pair[0], pair[1]);
        }
        let mut coverage = Vec::new();
        canvas.resolve(&mut coverage);
        coverage
    }

    #[test]
    fn pixel_aligned_squares_are_fully_covered() {
        let p = Vec2::new;
        let square = [
            p(1.0, 1.0),
            p(3.0, 1.0),
            p(3.0, 3.0),
            p(1.0, 3.0),
            p(1.0, 1.0),
        ];
        #[rustfmt::skip]
        let expected = [
            0, 0, 0, 0,
            0, 255, 255, 0,
            0, 255, 255, 0,
            0, 0, 0, 0,
        ];
        assert_eq!(draw(4, 4, &square), expected);
        // Either winding direction covers the same pixels.
        let reversed: Vec<_> = square.iter().rev().copied().collect();
        assert_eq!(draw(4, 4, &reversed), expected);
    }

    #[test]
    fn edges_between_pixels_give_partial_coverage() {
        let p = Vec2::new;
        // A rectangle from x = 0.5 to 2: the first column is half covered.
        let rectangle = [
            p(0.5, 0.0),
            p(2.0, 0.0),
            p(2.0, 1.0),
            p(0.5, 1.0),
            p(0.5, 0.0),
        ];
        assert_eq!(draw(2, 1, &rectangle), [128, 255]);
        // A right triangle over one pixel covers half of it.
        let triangle = [p(0.0, 0.0), p(1.0, 1.0), p(0.0, 1.0), p(0.0, 0.0)];
        assert_eq!(draw(1, 1, &triangle), [128]);
    }

    #[test]
    fn holes_cancel_and_overlaps_saturate() {
        let p = Vec2::new;
        let outer = [
            p(0.0, 0.0),
            p(3.0, 0.0),
            p(3.0, 3.0),
            p(0.0, 3.0),
            p(0.0, 0.0),
        ];
        // The inner square runs the other way: a hole.
        let hole = [
            p(1.0, 1.0),
            p(1.0, 2.0),
            p(2.0, 2.0),
            p(2.0, 1.0),
            p(1.0, 1.0),
        ];
        let mut cells = Vec::new();
        let mut canvas = Canvas::new(&mut cells, 3, 3);
        for pair in outer.windows(2).chain(hole.windows(2)) {
            canvas.line(pair[0], pair[1]);
        }
        let mut coverage = Vec::new();
        canvas.resolve(&mut coverage);
        assert_eq!(coverage, [255, 255, 255, 255, 0, 255, 255, 255, 255]);
        // The same square drawn twice in the same direction stays fully covered.
        let doubled: Vec<_> = outer.iter().chain(&outer).copied().collect();
        assert!(draw(3, 3, &doubled).iter().all(|c| *c == 255));
    }

    #[test]
    fn segments_outside_the_bitmap_are_ignored() {
        let p = Vec2::new;
        let wild = [
            p(-100.0, -50.0),
            p(f32::NAN, 2.0),
            p(400.0, 900.0),
            p(-100.0, -50.0),
        ];
        let coverage = draw(4, 4, &wild);
        assert_eq!(coverage.len(), 16);
    }
}
