//! Panels: surfaces that group components.

use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_render::RectPrimitive;
use koebi_tokens::component;

use crate::ui::Ui;

/// How deep a panel is nested; each level has its own background.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PanelLevel {
    /// A panel on the window background.
    #[default]
    First,
    /// A panel inside a first-level panel.
    Second,
    /// A panel inside a second-level panel.
    Third,
}

/// Colors and metrics of [`Ui::panel`], from the `component.panel.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelStyle {
    /// Background of each [`PanelLevel`], first level first.
    pub backgrounds: [Color; 3],
    /// Border color.
    pub border_color: Color,
    /// Border width.
    pub border_width: f32,
    /// Corner radius.
    pub corner_radius: f32,
    /// Space between the border and the content.
    pub padding: f32,
}

impl PanelStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            backgrounds: [
                tokens.get(component::PANEL_BODY_BACKGROUND_COLOR_LEVEL_1),
                tokens.get(component::PANEL_BODY_BACKGROUND_COLOR_LEVEL_2),
                tokens.get(component::PANEL_BODY_BACKGROUND_COLOR_LEVEL_3),
            ],
            border_color: tokens.get(component::PANEL_BORDER_COLOR_DEFAULT),
            border_width: tokens.get(component::PANEL_BORDER_WIDTH_DEFAULT),
            corner_radius: tokens.get(component::PANEL_CORNER_RADIUS_DEFAULT),
            padding: tokens.get(component::PANEL_PADDING_DEFAULT),
        }
    }

    /// The content area of a panel occupying `rect`: inside its border and padding.
    pub fn content(&self, rect: Rect) -> Rect {
        let content = rect.inset(self.border_width + self.padding);
        if content.is_empty() {
            Rect::from_center_size(rect.center(), Vec2::ZERO)
        } else {
            content
        }
    }
}

impl Ui<'_> {
    /// A panel filling `rect`. Returns its content area, inside the border and padding.
    pub fn panel(&mut self, rect: Rect, level: PanelLevel) -> Rect {
        let style = &self.styles().panel;
        let background = match level {
            PanelLevel::First => style.backgrounds[0],
            PanelLevel::Second => style.backgrounds[1],
            PanelLevel::Third => style.backgrounds[2],
        };
        self.scene.push_rect(
            &RectPrimitive::new(rect, background)
                .with_corner_radius(style.corner_radius)
                .with_border(style.border_width, style.border_color),
        );
        style.content(rect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_sits_inside_the_border_and_padding() {
        let style = crate::test_styles().panel;
        let rect = Rect::from_origin_size(Vec2::ZERO, Vec2::new(100.0, 60.0));
        let inset = style.border_width + style.padding;
        assert_eq!(style.content(rect).min, Vec2::splat(inset));
        let tiny = Rect::from_origin_size(Vec2::ZERO, Vec2::splat(4.0));
        assert_eq!(style.content(tiny).size(), Vec2::ZERO);
    }
}
