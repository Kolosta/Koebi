//! [`Icon`]: small line icons drawn with round-capped segments.

use koebi_core::{Color, Rect, Vec2};
use koebi_render::{LinePrimitive, Scene};

/// Icons are designed on a 16 x 16 grid, then scaled to the box they are drawn in.
const GRID: f32 = 16.0;

/// A line icon. Strokes keep their width whatever the icon's size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    /// A check mark.
    Check,
    /// A plus sign.
    Plus,
    /// A minus sign.
    Minus,
    /// A cross.
    Close,
    /// A chevron pointing left.
    ChevronLeft,
    /// A chevron pointing right.
    ChevronRight,
    /// A chevron pointing up.
    ChevronUp,
    /// A chevron pointing down.
    ChevronDown,
}

impl Icon {
    /// Every icon, for galleries and tests.
    pub const ALL: [Self; 8] = [
        Self::Check,
        Self::Plus,
        Self::Minus,
        Self::Close,
        Self::ChevronLeft,
        Self::ChevronRight,
        Self::ChevronUp,
        Self::ChevronDown,
    ];

    /// The strokes on the 16 x 16 grid, as `[x0, y0, x1, y1]`. Polylines repeat their
    /// shared points so that consecutive segments join with round corners.
    fn strokes(self) -> &'static [[f32; 4]] {
        match self {
            Self::Check => &[[3.5, 8.5, 6.5, 11.5], [6.5, 11.5, 12.5, 4.5]],
            Self::Plus => &[[8.0, 3.0, 8.0, 13.0], [3.0, 8.0, 13.0, 8.0]],
            Self::Minus => &[[3.0, 8.0, 13.0, 8.0]],
            Self::Close => &[[4.0, 4.0, 12.0, 12.0], [12.0, 4.0, 4.0, 12.0]],
            Self::ChevronLeft => &[[10.0, 3.5, 5.5, 8.0], [5.5, 8.0, 10.0, 12.5]],
            Self::ChevronRight => &[[6.0, 3.5, 10.5, 8.0], [10.5, 8.0, 6.0, 12.5]],
            Self::ChevronUp => &[[3.5, 10.0, 8.0, 5.5], [8.0, 5.5, 12.5, 10.0]],
            Self::ChevronDown => &[[3.5, 6.0, 8.0, 10.5], [8.0, 10.5, 12.5, 6.0]],
        }
    }

    /// Draws the icon scaled to the square `bounds` (use [`Icon::centered`] to place one).
    pub fn draw(self, scene: &mut Scene, bounds: Rect, color: Color, stroke_width: f32) {
        let scale = bounds.width().min(bounds.height()) / GRID;
        let point = |x: f32, y: f32| bounds.min + Vec2::new(x, y) * scale;
        for &[x0, y0, x1, y1] in self.strokes() {
            scene.push_line(&LinePrimitive::new(
                point(x0, y0),
                point(x1, y1),
                stroke_width,
                color,
            ));
        }
    }

    /// Draws the icon like [`Icon::draw`], turned clockwise by `angle` radians around the
    /// center of `bounds`: a fold arrow turning as its fold opens.
    pub(crate) fn draw_rotated(
        self,
        scene: &mut Scene,
        bounds: Rect,
        color: Color,
        stroke_width: f32,
        angle: f32,
    ) {
        let scale = bounds.width().min(bounds.height()) / GRID;
        let center = bounds.center();
        let (sin, cos) = angle.sin_cos();
        let point = |x: f32, y: f32| {
            let local = Vec2::new(x - GRID * 0.5, y - GRID * 0.5) * scale;
            center + Vec2::new(local.x * cos - local.y * sin, local.x * sin + local.y * cos)
        };
        for &[x0, y0, x1, y1] in self.strokes() {
            scene.push_line(&LinePrimitive::new(
                point(x0, y0),
                point(x1, y1),
                stroke_width,
                color,
            ));
        }
    }

    /// The square of side `size` centered in `area`, shrunk to fit it, with its corner on a
    /// whole logical pixel.
    pub fn centered(area: Rect, size: f32) -> Rect {
        let side = size.min(area.width()).min(area.height()).max(0.0);
        let center = area.center();
        let min = Vec2::new(
            (center.x - side * 0.5).round(),
            (center.y - side * 0.5).round(),
        );
        Rect::from_origin_size(min, Vec2::splat(side))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strokes_stay_inside_the_grid() {
        for icon in Icon::ALL {
            for stroke in icon.strokes() {
                assert!(stroke.iter().all(|v| (0.0..=GRID).contains(v)), "{icon:?}");
            }
        }
    }

    #[test]
    fn icons_scale_to_their_bounds() {
        let mut scene = Scene::new();
        let bounds = Rect::from_origin_size(Vec2::new(10.0, 20.0), Vec2::splat(32.0));
        Icon::Plus.draw(&mut scene, bounds, Color::WHITE, 2.0);
        assert_eq!(scene.line_count(), 2);
        assert_eq!(scene.draw_count(), 1);
    }

    #[test]
    fn centered_boxes_fit_and_snap() {
        let area = Rect::from_origin_size(Vec2::new(0.0, 0.0), Vec2::new(41.0, 24.0));
        let icon = Icon::centered(area, 16.0);
        assert_eq!(icon.size(), Vec2::splat(16.0));
        assert_eq!(icon.min, Vec2::new(13.0, 4.0));
        let small = Icon::centered(area, 100.0);
        assert_eq!(small.size(), Vec2::splat(24.0));
    }
}
