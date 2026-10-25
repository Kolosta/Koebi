//! Push buttons.

use koebi_core::{Color, Rect};
use koebi_design::Resolve;
use koebi_render::RectPrimitive;
use koebi_tokens::component;

use crate::icon::Icon;
use crate::id::WidgetId;
use crate::style::StateColors;
use crate::ui::{Response, Ui};

/// Colors and metrics of [`Ui::button`], from the `component.button.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonStyle {
    /// Background per state.
    pub background: StateColors,
    /// Color of the content (the icon, until buttons have text).
    pub label: Color,
    /// Border color.
    pub border_color: Color,
    /// Border width; `0` draws no border.
    pub border_width: f32,
    /// Corner radius.
    pub corner_radius: f32,
}

impl ButtonStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            background: StateColors {
                normal: tokens.get(component::BUTTON_BACKGROUND_COLOR_DEFAULT),
                hovered: tokens.get(component::BUTTON_BACKGROUND_COLOR_HOVER),
                pressed: tokens.get(component::BUTTON_BACKGROUND_COLOR_PRESSED),
            },
            label: tokens.get(component::BUTTON_LABEL_COLOR_DEFAULT),
            border_color: tokens.get(component::BUTTON_BORDER_COLOR_DEFAULT),
            border_width: tokens.get(component::BUTTON_BORDER_WIDTH_DEFAULT),
            corner_radius: tokens.get(component::BUTTON_CORNER_RADIUS_DEFAULT),
        }
    }
}

impl Ui<'_> {
    /// A push button filling `rect`, with an optional centered icon. It reports `clicked`
    /// when the pointer is released over it after pressing it.
    pub fn button(&mut self, id: WidgetId, rect: Rect, icon: Option<Icon>) -> Response {
        let response = self.interact(id, rect);
        let styles = self.styles();
        let style = &styles.button;
        self.scene.push_rect(
            &RectPrimitive::new(rect, style.background.pick(response.state()))
                .with_corner_radius(style.corner_radius)
                .with_border(style.border_width, style.border_color),
        );
        if let Some(icon) = icon {
            let bounds = Icon::centered(rect, styles.icon.size);
            icon.draw(self.scene, bounds, style.label, styles.icon.stroke_width);
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::Vec2;
    use koebi_render::Scene;

    use crate::{Input, UiState};

    use super::*;

    #[test]
    fn buttons_draw_their_background_and_icon() {
        let styles = crate::test_styles();
        let (mut scene, mut state, input) = (Scene::new(), UiState::new(), Input::new());
        let rect = Rect::from_origin_size(Vec2::ZERO, Vec2::new(60.0, 24.0));
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        ui.button(WidgetId::new("plain"), rect, None);
        ui.button(WidgetId::new("add"), rect, Some(Icon::Plus));
        drop(ui);
        assert_eq!((scene.rect_count(), scene.line_count()), (2, 2));
    }
}
