//! Separators: thin lines between groups of components.

use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_render::RectPrimitive;
use koebi_tokens::component;

use crate::ui::Ui;

/// Color and thickness of [`Ui::separator`], from the `component.separator.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeparatorStyle {
    /// Line color.
    pub color: Color,
    /// Line thickness.
    pub thickness: f32,
}

impl SeparatorStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            color: tokens.get(component::SEPARATOR_COLOR_DEFAULT),
            thickness: tokens.get(component::SEPARATOR_SIZE_DEFAULT),
        }
    }

    /// The line drawn across `rect`: along its longer side, centered, on whole pixels.
    pub fn line(&self, rect: Rect) -> Rect {
        let thickness = self.thickness;
        if rect.width() >= rect.height() {
            let y = (rect.center().y - thickness * 0.5).round();
            Rect::from_origin_size(Vec2::new(rect.min.x, y), Vec2::new(rect.width(), thickness))
        } else {
            let x = (rect.center().x - thickness * 0.5).round();
            Rect::from_origin_size(
                Vec2::new(x, rect.min.y),
                Vec2::new(thickness, rect.height()),
            )
        }
    }
}

impl Ui<'_> {
    /// A line across the middle of `rect`, horizontal when `rect` is wider than tall.
    pub fn separator(&mut self, rect: Rect) {
        let style = &self.styles().separator;
        self.scene
            .push_rect(&RectPrimitive::new(style.line(rect), style.color));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_follow_the_longer_side() {
        let style = SeparatorStyle {
            color: Color::WHITE,
            thickness: 1.0,
        };
        let row = Rect::from_origin_size(Vec2::new(0.0, 10.0), Vec2::new(100.0, 9.0));
        assert_eq!(
            style.line(row),
            Rect::from_origin_size(Vec2::new(0.0, 14.0), Vec2::new(100.0, 1.0))
        );
        let column = Rect::from_origin_size(Vec2::ZERO, Vec2::new(8.0, 50.0));
        assert_eq!(style.line(column).size(), Vec2::new(1.0, 50.0));
    }
}
