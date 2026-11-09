//! The application's own cursors, and the mapping of the interface's cursor shapes to them or to
//! platform cursors.
//!
//! The workspace's cursors are the icons of `koebi_icons::cursors` (those of the previous
//! application): the corners of areas, splits, joins, a refused drop, moves. They are drawn once
//! at start-up for the display's scale, white over a dark outline so that they read on any
//! background. Text, links and resizing keep the system's cursors, which people know.

use koebi_core::input::CursorShape;
use koebi_icons::{IconId, cursors};
use koebi_platform::{AppContext, Cursor, CursorId, CursorImage, SystemCursor};
use koebi_svg::raster::{Colors, render};

/// Side of a cursor image at scale 1, in pixels.
const SIZE: f32 = 32.0;
/// Width of the dark outline around the glyphs, in pixels at scale 1.
const OUTLINE: f32 = 1.5;

/// The shapes drawn from icons, the icon of each, and the system cursor standing in for it when
/// it could not be created.
const CUSTOM: [(CursorShape, IconId, SystemCursor); 12] = [
    (
        CursorShape::ArrowLeft,
        cursors::JOIN_LEFT,
        SystemCursor::Move,
    ),
    (
        CursorShape::ArrowRight,
        cursors::JOIN_RIGHT,
        SystemCursor::Move,
    ),
    (CursorShape::ArrowUp, cursors::JOIN_UP, SystemCursor::Move),
    (
        CursorShape::ArrowDown,
        cursors::JOIN_DOWN,
        SystemCursor::Move,
    ),
    (
        CursorShape::NotAllowed,
        cursors::BLOCK,
        SystemCursor::NotAllowed,
    ),
    (
        CursorShape::Move,
        cursors::MOVE_ALL_DIRECTIONS,
        SystemCursor::Move,
    ),
    (
        CursorShape::CornerTopLeft,
        cursors::CORNER_ACTIONS_ADD_UP_LEFT,
        SystemCursor::Crosshair,
    ),
    (
        CursorShape::CornerTopRight,
        cursors::CORNER_ACTIONS_ADD_UP_RIGHT,
        SystemCursor::Crosshair,
    ),
    (
        CursorShape::CornerBottomLeft,
        cursors::CORNER_ACTIONS_ADD_DOWN_LEFT,
        SystemCursor::Crosshair,
    ),
    (
        CursorShape::CornerBottomRight,
        cursors::CORNER_ACTIONS_ADD_DOWN_RIGHT,
        SystemCursor::Crosshair,
    ),
    (
        CursorShape::SplitVertical,
        cursors::SPLIT_JOIN_VERTICAL,
        SystemCursor::ResizeHorizontal,
    ),
    (
        CursorShape::SplitHorizontal,
        cursors::SPLIT_JOIN_HORIZONTAL,
        SystemCursor::ResizeVertical,
    ),
];

/// Cursors the application created; shapes without one fall back to system cursors.
#[derive(Debug, Default)]
pub(crate) struct AppCursors {
    custom: [Option<CursorId>; CUSTOM.len()],
}

impl AppCursors {
    /// Draws and registers the cursors for a display of `scale` (physical pixels per logical
    /// pixel). A cursor that cannot be created leaves a system cursor in its place.
    pub(crate) fn create(context: &mut AppContext, scale: f32) -> Self {
        let scale = scale.clamp(1.0, 4.0);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "between 32 and 128 pixels"
        )]
        let side = (SIZE * scale).round() as u32;
        let mut created = Self::default();
        for (slot, (_, icon, _)) in created.custom.iter_mut().zip(CUSTOM) {
            let pixels = cursor_pixels(icon, side, OUTLINE * scale);
            let image = CursorImage {
                width: side,
                height: side,
                pixels: &pixels,
                hotspot: (side / 2, side / 2),
            };
            match context.create_cursor(&image) {
                Ok(id) => *slot = Some(id),
                Err(error) => {
                    koebi_core::warn!("cannot create the {} cursor: {error}", icon.name());
                }
            }
        }
        created
    }

    /// The platform cursor showing `shape`.
    pub(crate) fn cursor(&self, shape: CursorShape) -> Cursor {
        if let Some(index) = CUSTOM.iter().position(|(custom, ..)| *custom == shape) {
            let (_, _, fallback) = CUSTOM[index];
            return self.custom[index].map_or(Cursor::System(fallback), Cursor::Custom);
        }
        let system = match shape {
            CursorShape::Pointer => SystemCursor::Pointer,
            CursorShape::Text => SystemCursor::Text,
            CursorShape::Crosshair => SystemCursor::Crosshair,
            CursorShape::ResizeHorizontal => SystemCursor::ResizeHorizontal,
            CursorShape::ResizeVertical => SystemCursor::ResizeVertical,
            CursorShape::ResizeNwse => SystemCursor::ResizeNwse,
            CursorShape::ResizeNesw => SystemCursor::ResizeNesw,
            _ => SystemCursor::Default,
        };
        Cursor::System(system)
    }
}

/// `value`, from 0 to 1, as a byte.
fn byte(value: f32) -> u8 {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to a byte"
    )]
    let byte = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    byte
}

/// RGBA pixels (straight alpha) of `icon` on a transparent square of `side` pixels: white,
/// over a dark outline `outline` pixels wide, anti-aliased.
fn cursor_pixels(icon: IconId, side: u32, outline: f32) -> Vec<u8> {
    let compiled = icon.compile();
    let glyph: Vec<f32> = render(&compiled, side, Colors::Original)
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| f32::from(pixel[3]) / 255.0)
        .collect();
    let width = i32::try_from(side).unwrap_or(i32::MAX);
    let at = |x: i32, y: i32| -> f32 {
        let inside = (0..width).contains(&x) && (0..width).contains(&y);
        usize::try_from(y * width + x)
            .ok()
            .filter(|_| inside)
            .map_or(0.0, |index| glyph[index])
    };
    // The outline: the glyph grown by `outline`, each pixel as covered as the most covered
    // pixel near it, less how far past the outline that one lies.
    #[allow(clippy::cast_possible_truncation, reason = "a few pixels")]
    let reach = (outline + 0.5).ceil() as i32;
    let mut pixels = Vec::with_capacity(glyph.len() * 4);
    for y in 0..width {
        for x in 0..width {
            let mut grown = 0.0_f32;
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    let distance = (dx as f32).hypot(dy as f32);
                    let weight = (outline + 0.5 - distance).clamp(0.0, 1.0);
                    grown = grown.max(at(x + dx, y + dy) * weight);
                }
            }
            let ink = at(x, y);
            // White over black: the glyph covers the outline under it.
            let alpha = ink + grown * (1.0 - ink);
            let white = if alpha > 1e-6 { ink / alpha } else { 0.0 };
            pixels.extend_from_slice(&[byte(white), byte(white), byte(white), byte(alpha)]);
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(pixels: &[u8], side: u32, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * side + x) * 4) as usize;
        [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]]
    }

    #[test]
    fn cursors_are_white_glyphs_outlined_on_transparency() {
        let side = 32;
        // The block: a ring crossed by a bar, its center covered.
        let pixels = cursor_pixels(cursors::BLOCK, side, OUTLINE);
        assert_eq!(pixel(&pixels, side, 16, 16), [255, 255, 255, 255]);
        assert_eq!(pixel(&pixels, side, 0, 0)[3], 0);
        // The outline surrounds the glyph: no white pixel touches transparency.
        for y in 1..side - 1 {
            for x in 1..side - 1 {
                if pixel(&pixels, side, x, y) == [255, 255, 255, 255] {
                    for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                        assert!(pixel(&pixels, side, nx, ny)[3] > 0, "({nx}, {ny})");
                    }
                }
            }
        }
        // And it is dark.
        assert!(
            pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[3] == 255 && pixel[0] < 32)
        );
    }

    #[test]
    fn every_shape_drawn_from_an_icon_falls_back_to_a_system_cursor() {
        let none = AppCursors::default();
        for (shape, _, fallback) in CUSTOM {
            assert_eq!(none.cursor(shape), Cursor::System(fallback), "{shape:?}");
        }
        assert_eq!(
            none.cursor(CursorShape::Text),
            Cursor::System(SystemCursor::Text)
        );
        assert_eq!(
            none.cursor(CursorShape::Custom(3)),
            Cursor::System(SystemCursor::Default)
        );
        // Each shape once.
        for (index, (shape, ..)) in CUSTOM.iter().enumerate() {
            assert!(CUSTOM[index + 1..].iter().all(|(other, ..)| other != shape));
        }
    }
}
