//! Color swatches: a color to look at or to pick.

use core::f32::consts::FRAC_1_SQRT_2;

use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_render::{RectPrimitive, Scene};
use koebi_tokens::component;

use crate::id::WidgetId;
use crate::ui::{Response, Ui};

/// Colors and metrics of [`Ui::swatch`], from the `component.color-swatch.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwatchStyle {
    /// Corner radius.
    pub corner_radius: f32,
    /// Border color at rest.
    pub border_color: Color,
    /// Border color under the pointer.
    pub border_color_hovered: Color,
    /// Border color of the selected swatch.
    pub border_color_selected: Color,
    /// Border width.
    pub border_width: f32,
    /// Border width of the selected swatch.
    pub border_width_selected: f32,
    /// Light squares of the checkerboard shown behind translucent colors.
    pub checker_light: Color,
    /// Dark squares of the checkerboard.
    pub checker_dark: Color,
    /// Side of a checkerboard square.
    pub checker_size: f32,
}

impl SwatchStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            corner_radius: tokens.get(component::COLOR_SWATCH_CORNER_RADIUS_DEFAULT),
            border_color: tokens.get(component::COLOR_SWATCH_BORDER_COLOR_DEFAULT),
            border_color_hovered: tokens.get(component::COLOR_SWATCH_BORDER_COLOR_HOVER),
            border_color_selected: tokens.get(component::COLOR_SWATCH_BORDER_COLOR_SELECTED),
            border_width: tokens.get(component::COLOR_SWATCH_BORDER_WIDTH_DEFAULT),
            border_width_selected: tokens.get(component::COLOR_SWATCH_BORDER_WIDTH_SELECTED),
            checker_light: tokens.get(component::COLOR_SWATCH_CHECKER_COLOR_LIGHT),
            checker_dark: tokens.get(component::COLOR_SWATCH_CHECKER_COLOR_DARK),
            checker_size: tokens.get(component::COLOR_SWATCH_CHECKER_SIZE_DEFAULT),
        }
    }

    /// Paints the checkerboard of a swatch occupying `rect`. The dark squares stay inside
    /// the rounded corners: the board is inset by the distance between a corner and its arc.
    fn checkerboard(&self, scene: &mut Scene, rect: Rect) {
        scene.push_rect(
            &RectPrimitive::new(rect, self.checker_light).with_corner_radius(self.corner_radius),
        );
        let size = self.checker_size;
        let board = rect.inset(self.corner_radius.max(0.0) * (1.0 - FRAC_1_SQRT_2));
        if size <= 0.0 || board.is_empty() {
            return;
        }
        let mut y = board.min.y;
        let mut row = 0_u32;
        while y < board.max.y {
            let mut x = board.min.x + if row.is_multiple_of(2) { size } else { 0.0 };
            while x < board.max.x {
                let max = Vec2::new((x + size).min(board.max.x), (y + size).min(board.max.y));
                scene.push_rect(&RectPrimitive::new(
                    Rect::from_min_max(Vec2::new(x, y), max),
                    self.checker_dark,
                ));
                x += 2.0 * size;
            }
            y += size;
            row += 1;
        }
    }
}

impl Ui<'_> {
    /// A swatch of `color` filling `rect`, over a checkerboard when the color is
    /// translucent. `selected` thickens and highlights its border; clicks are reported.
    pub fn swatch(&mut self, id: WidgetId, rect: Rect, color: Color, selected: bool) -> Response {
        let response = self.interact(id, rect);
        let style = &self.styles().swatch;
        if color.a < 1.0 {
            style.checkerboard(self.scene, rect);
        }
        let (border, width) = if selected {
            (style.border_color_selected, style.border_width_selected)
        } else if response.hovered {
            (style.border_color_hovered, style.border_width)
        } else {
            (style.border_color, style.border_width)
        };
        self.scene.push_rect(
            &RectPrimitive::new(rect, color)
                .with_corner_radius(style.corner_radius)
                .with_border(width, border),
        );
        response
    }
}

#[cfg(test)]
mod tests {
    use crate::{Input, UiState};

    use super::*;

    fn rects_for(color: Color) -> usize {
        let styles = crate::test_styles();
        let (mut scene, mut state, input) = (Scene::new(), UiState::new(), Input::new());
        let rect = Rect::from_origin_size(Vec2::ZERO, Vec2::splat(24.0));
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        ui.swatch(WidgetId::new("swatch"), rect, color, false);
        drop(ui);
        scene.rect_count()
    }

    #[test]
    fn only_translucent_colors_get_a_checkerboard() {
        assert_eq!(rects_for(Color::from_hex(0x33_66_99)), 1);
        // A 6 px board in a 24 px swatch inset by about 0.6 px: 4 rows of 2 dark squares,
        // plus the light board and the color.
        assert_eq!(rects_for(Color::from_hex(0x33_66_99).with_alpha(0.5)), 10);
    }
}
