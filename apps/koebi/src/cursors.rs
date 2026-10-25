//! The application's own cursors: direction arrows, drawn at start-up, and the mapping of
//! the interface's cursor shapes to platform cursors.

use koebi_core::Vec2;
use koebi_core::input::CursorShape;
use koebi_platform::{AppContext, Cursor, CursorId, CursorImage, SystemCursor};

/// Side of a cursor image at scale 1, in pixels.
const SIZE: f32 = 32.0;
/// Width of the dark outline around the arrows, in pixels at scale 1.
const OUTLINE: f32 = 1.5;

/// An arrow pointing right on a unit square: shaft, then head, clockwise.
const ARROW: [[f32; 2]; 7] = [
    [0.12, 0.41],
    [0.55, 0.41],
    [0.55, 0.20],
    [0.90, 0.50],
    [0.55, 0.80],
    [0.55, 0.59],
    [0.12, 0.59],
];

/// The four arrow directions, in the order of [`AppCursors::arrows`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    const ALL: [Self; 4] = [Self::Left, Self::Right, Self::Up, Self::Down];

    /// A point of the right-pointing arrow, turned to this direction.
    fn turn(self, [x, y]: [f32; 2]) -> Vec2 {
        match self {
            Self::Right => Vec2::new(x, y),
            Self::Left => Vec2::new(1.0 - x, y),
            Self::Down => Vec2::new(y, x),
            Self::Up => Vec2::new(y, 1.0 - x),
        }
    }
}

/// Cursors the application created; shapes without one fall back to system cursors.
#[derive(Debug, Default)]
pub(crate) struct AppCursors {
    arrows: [Option<CursorId>; 4],
}

impl AppCursors {
    /// Draws and registers the arrows for a display of `scale` (physical pixels per logical
    /// pixel). Failures leave the system's move cursor in their place.
    pub(crate) fn create(context: &mut AppContext, scale: f32) -> Self {
        let size = (SIZE * scale.clamp(1.0, 4.0)).round();
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "between 32 and 128 pixels"
        )]
        let side = size as u32;
        let mut cursors = Self::default();
        for (slot, direction) in cursors.arrows.iter_mut().zip(Direction::ALL) {
            let pixels = arrow_pixels(side, direction, OUTLINE * scale.max(1.0));
            let image = CursorImage {
                width: side,
                height: side,
                pixels: &pixels,
                hotspot: (side / 2, side / 2),
            };
            match context.create_cursor(&image) {
                Ok(id) => *slot = Some(id),
                Err(error) => koebi_core::warn!("cannot create an arrow cursor: {error}"),
            }
        }
        cursors
    }

    /// The platform cursor showing `shape`.
    pub(crate) fn cursor(&self, shape: CursorShape) -> Cursor {
        let arrow = |index: usize| {
            self.arrows[index].map_or(Cursor::System(SystemCursor::Move), Cursor::Custom)
        };
        let system = match shape {
            CursorShape::ArrowLeft => return arrow(0),
            CursorShape::ArrowRight => return arrow(1),
            CursorShape::ArrowUp => return arrow(2),
            CursorShape::ArrowDown => return arrow(3),
            CursorShape::Pointer => SystemCursor::Pointer,
            CursorShape::Text => SystemCursor::Text,
            CursorShape::Crosshair => SystemCursor::Crosshair,
            CursorShape::Move => SystemCursor::Move,
            CursorShape::NotAllowed => SystemCursor::NotAllowed,
            CursorShape::ResizeHorizontal => SystemCursor::ResizeHorizontal,
            CursorShape::ResizeVertical => SystemCursor::ResizeVertical,
            CursorShape::ResizeNwse => SystemCursor::ResizeNwse,
            CursorShape::ResizeNesw => SystemCursor::ResizeNesw,
            CursorShape::Default | CursorShape::Custom(_) => SystemCursor::Default,
        };
        Cursor::System(system)
    }
}

/// RGBA pixels (straight alpha) of an arrow pointing `direction`: white with a dark outline,
/// anti-aliased, on a transparent square of `side` pixels.
fn arrow_pixels(side: u32, direction: Direction, outline: f32) -> Vec<u8> {
    let scale = side as f32;
    let polygon: Vec<Vec2> = ARROW
        .iter()
        .map(|point| direction.turn(*point) * scale)
        .collect();
    let mut pixels = Vec::with_capacity((side * side * 4) as usize);
    for y in 0..side {
        for x in 0..side {
            let center = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let distance = signed_distance(&polygon, center);
            // Opaque inside the outline's outer edge, white inside its inner edge.
            let alpha = (0.5 - distance).clamp(0.0, 1.0);
            let white = (0.5 - (distance + outline)).clamp(0.0, 1.0);
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "values clamped to 0..=1, scaled to 0..=255"
            )]
            let byte = |value: f32| (value * 255.0).round() as u8;
            let shade = byte(white);
            pixels.extend_from_slice(&[shade, shade, shade, byte(alpha)]);
        }
    }
    pixels
}

/// Distance from `point` to the edge of `polygon`, negative inside.
fn signed_distance(polygon: &[Vec2], point: Vec2) -> f32 {
    let mut distance = f32::MAX;
    let mut inside = false;
    for (index, &a) in polygon.iter().enumerate() {
        let b = polygon[(index + 1) % polygon.len()];
        let edge = b - a;
        let t = ((point - a).dot(edge) / edge.dot(edge)).clamp(0.0, 1.0);
        distance = distance.min((point - (a + edge * t)).length());
        // Even-odd rule: count the edges a ray to the right of the point crosses.
        if (a.y > point.y) != (b.y > point.y)
            && point.x < a.x + (point.y - a.y) / (b.y - a.y) * edge.x
        {
            inside = !inside;
        }
    }
    if inside { -distance } else { distance }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(pixels: &[u8], side: u32, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * side + x) * 4) as usize;
        [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]]
    }

    #[test]
    fn arrows_are_white_with_an_outline_on_transparency() {
        let side = 32;
        let right = arrow_pixels(side, Direction::Right, OUTLINE);
        assert_eq!(right.len(), 32 * 32 * 4);
        // The middle of the shaft is opaque white; corners are transparent.
        assert_eq!(pixel(&right, side, 10, 16), [255, 255, 255, 255]);
        assert_eq!(pixel(&right, side, 0, 0)[3], 0);
        assert_eq!(pixel(&right, side, 31, 31)[3], 0);
        // Just inside the shaft's top edge: the dark outline.
        let edge = pixel(&right, side, 10, 13);
        assert!(edge[3] > 200 && edge[0] < 60, "{edge:?}");
        // The tip points right: opaque near x = 28, transparent beyond.
        assert!(pixel(&right, side, 26, 16)[3] > 200);
        assert_eq!(pixel(&right, side, 30, 16)[3], 0);
    }

    #[test]
    fn arrows_turn_to_every_direction() {
        let side = 32;
        let left = arrow_pixels(side, Direction::Left, OUTLINE);
        let up = arrow_pixels(side, Direction::Up, OUTLINE);
        let down = arrow_pixels(side, Direction::Down, OUTLINE);
        // Each tip lies on its side, the opposite end is the shaft's square end.
        assert!(pixel(&left, side, 5, 16)[3] > 200 && pixel(&left, side, 1, 16)[3] == 0);
        assert!(pixel(&up, side, 16, 5)[3] > 200 && pixel(&up, side, 16, 1)[3] == 0);
        assert!(pixel(&down, side, 16, 26)[3] > 200 && pixel(&down, side, 16, 30)[3] == 0);
    }

    #[test]
    fn distances_are_negative_inside_polygons() {
        let square = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        assert!((signed_distance(&square, Vec2::new(5.0, 5.0)) + 5.0).abs() < 1e-5);
        assert!((signed_distance(&square, Vec2::new(13.0, 5.0)) - 3.0).abs() < 1e-5);
    }
}
