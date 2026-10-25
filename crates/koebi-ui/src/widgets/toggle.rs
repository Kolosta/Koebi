//! Toggle buttons: buttons that stay selected until clicked again.

use koebi_core::{Color, Rect};
use koebi_design::Resolve;
use koebi_render::RectPrimitive;
use koebi_tokens::component;

use crate::icon::Icon;
use crate::id::WidgetId;
use crate::ui::{Response, Ui};

/// Colors and metrics of [`Ui::toggle`], from the `component.toggle.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToggleStyle {
    /// Background when off.
    pub background: Color,
    /// Background when off and hovered.
    pub background_hovered: Color,
    /// Background when on.
    pub background_selected: Color,
    /// Border when off.
    pub border_color: Color,
    /// Border when on.
    pub border_color_selected: Color,
    /// Content color when off.
    pub label: Color,
    /// Content color when on.
    pub label_selected: Color,
    /// Border width.
    pub border_width: f32,
    /// Corner radius.
    pub corner_radius: f32,
}

impl ToggleStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            background: tokens.get(component::TOGGLE_BACKGROUND_COLOR_DEFAULT),
            background_hovered: tokens.get(component::TOGGLE_BACKGROUND_COLOR_HOVER),
            background_selected: tokens.get(component::TOGGLE_BACKGROUND_COLOR_SELECTED),
            border_color: tokens.get(component::TOGGLE_BORDER_COLOR_DEFAULT),
            border_color_selected: tokens.get(component::TOGGLE_BORDER_COLOR_SELECTED),
            label: tokens.get(component::TOGGLE_LABEL_COLOR_DEFAULT),
            label_selected: tokens.get(component::TOGGLE_LABEL_COLOR_SELECTED),
            border_width: tokens.get(component::TOGGLE_BORDER_WIDTH_DEFAULT),
            corner_radius: tokens.get(component::TOGGLE_CORNER_RADIUS_DEFAULT),
        }
    }
}

impl Ui<'_> {
    /// A toggle button filling `rect`, with an optional centered icon. A click flips `on`
    /// and reports `changed`.
    pub fn toggle(
        &mut self,
        id: WidgetId,
        rect: Rect,
        on: &mut bool,
        icon: Option<Icon>,
    ) -> Response {
        let mut response = self.interact(id, rect);
        if response.clicked {
            *on = !*on;
            response.changed = true;
        }
        let styles = self.styles();
        let style = &styles.toggle;
        let (background, border, label) = if *on {
            (
                style.background_selected,
                style.border_color_selected,
                style.label_selected,
            )
        } else if response.hovered {
            (style.background_hovered, style.border_color, style.label)
        } else {
            (style.background, style.border_color, style.label)
        };
        self.scene.push_rect(
            &RectPrimitive::new(rect, background)
                .with_corner_radius(style.corner_radius)
                .with_border(style.border_width, border),
        );
        if let Some(icon) = icon {
            let bounds = Icon::centered(rect, styles.icon.size);
            icon.draw(self.scene, bounds, label, styles.icon.stroke_width);
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
    fn clicks_flip_the_toggle() {
        let styles = crate::test_styles();
        let (mut scene, mut state, mut input) = (Scene::new(), UiState::new(), Input::new());
        let rect = Rect::from_origin_size(Vec2::ZERO, Vec2::new(24.0, 24.0));
        let mut on = false;
        for expected in [true, false] {
            input.primary_button(true, Vec2::new(5.0, 5.0));
            input.primary_button(false, Vec2::new(5.0, 5.0));
            let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
            let response = ui.toggle(WidgetId::new("grid"), rect, &mut on, Some(Icon::Check));
            drop(ui);
            input.end_frame();
            assert!(response.changed);
            assert_eq!(on, expected);
        }
    }
}
