//! The demo scene, as a pure function of the window size (unit-testable without a GPU).

use koebi_core::{Rect, Vec2};
use koebi_render::{RectPrimitive, Scene};

use crate::theme::Theme;

/// Rebuilds `scene` for a window of `logical_size` (logical pixels).
pub(crate) fn build(scene: &mut Scene, logical_size: Vec2, theme: &Theme) {
    scene.clear();
    scene.set_clear_color(theme.background);
    scene.push_rect(
        &RectPrimitive::new(card_bounds(logical_size), theme.accent)
            .with_corner_radius(theme.corner_radius)
            .with_border(theme.border_width, theme.accent_border),
    );
}

/// A centered card: half the window width and 40 % of its height, within sane limits, and
/// never larger than 90 % of the window.
pub(crate) fn card_bounds(logical_size: Vec2) -> Rect {
    let preferred = Vec2::new(
        (logical_size.x * 0.5).clamp(160.0, 640.0),
        (logical_size.y * 0.4).clamp(100.0, 400.0),
    );
    let size = preferred.min(logical_size * 0.9);
    // Snap to whole logical pixels so edges stay crisp at integer scale factors.
    let min = (logical_size * 0.5 - size * 0.5).round_to_pixels();
    Rect::from_origin_size(min, size.round_to_pixels())
}

trait RoundToPixels {
    fn round_to_pixels(self) -> Self;
}

impl RoundToPixels for Vec2 {
    fn round_to_pixels(self) -> Self {
        Self::new(self.x.round(), self.y.round())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_is_centered_and_pixel_aligned() {
        let window = Vec2::new(1280.0, 800.0);
        let card = card_bounds(window);
        assert_eq!(card.size(), Vec2::new(640.0, 320.0));
        assert_eq!(card.center(), window * 0.5);
        assert_eq!(card.min, Vec2::new(card.min.x.round(), card.min.y.round()));
    }

    #[test]
    fn card_fits_tiny_windows() {
        let window = Vec2::new(100.0, 60.0);
        let card = card_bounds(window);
        assert!(card.width() <= 90.0 && card.height() <= 54.0);
        assert!(card.min.x >= 0.0 && card.min.y >= 0.0);
    }

    #[test]
    fn scene_contains_the_themed_card() {
        let theme = Theme::dark();
        let mut scene = Scene::new();
        build(&mut scene, Vec2::new(800.0, 600.0), &theme);
        assert_eq!(scene.rect_count(), 1);
        assert_eq!(scene.clear_color(), theme.background);
        build(&mut scene, Vec2::new(800.0, 600.0), &theme);
        assert_eq!(
            scene.rect_count(),
            1,
            "rebuilding must not accumulate primitives"
        );
    }
}
