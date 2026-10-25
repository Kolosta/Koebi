//! Checkboxes.

use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_render::RectPrimitive;
use koebi_tokens::component;

use crate::icon::Icon;
use crate::id::WidgetId;
use crate::style::StateColors;
use crate::ui::{Response, Ui};
use crate::widgets::snap;

/// Colors and metrics of [`Ui::checkbox`], from the `component.checkbox.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckboxStyle {
    /// Box background per state, unchecked.
    pub background: StateColors,
    /// Box background per state, checked.
    pub background_checked: StateColors,
    /// Box border, unchecked.
    pub border_color: Color,
    /// Box border, checked.
    pub border_color_checked: Color,
    /// Check mark color.
    pub mark: Color,
    /// Check mark stroke width.
    pub mark_width: f32,
    /// Side of the box.
    pub box_size: f32,
    /// Corner radius of the box.
    pub corner_radius: f32,
    /// Border width of the box.
    pub border_width: f32,
}

impl CheckboxStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            background: StateColors {
                normal: tokens.get(component::CHECKBOX_BACKGROUND_COLOR_DEFAULT),
                hovered: tokens.get(component::CHECKBOX_BACKGROUND_COLOR_HOVER),
                pressed: tokens.get(component::CHECKBOX_BACKGROUND_COLOR_PRESSED),
            },
            background_checked: StateColors {
                normal: tokens.get(component::CHECKBOX_BACKGROUND_COLOR_SELECTED),
                hovered: tokens.get(component::CHECKBOX_BACKGROUND_COLOR_SELECTED_HOVER),
                pressed: tokens.get(component::CHECKBOX_BACKGROUND_COLOR_SELECTED_PRESSED),
            },
            border_color: tokens.get(component::CHECKBOX_BORDER_COLOR_DEFAULT),
            border_color_checked: tokens.get(component::CHECKBOX_BORDER_COLOR_SELECTED),
            mark: tokens.get(component::CHECKBOX_MARK_COLOR_DEFAULT),
            mark_width: tokens.get(component::CHECKBOX_MARK_WIDTH_DEFAULT),
            box_size: tokens.get(component::CHECKBOX_BOX_SIZE_DEFAULT),
            corner_radius: tokens.get(component::CHECKBOX_CORNER_RADIUS_DEFAULT),
            border_width: tokens.get(component::CHECKBOX_BORDER_WIDTH_DEFAULT),
        }
    }

    /// The box drawn for a checkbox occupying `rect`: at its left edge, vertically centered.
    pub fn box_rect(&self, rect: Rect) -> Rect {
        let side = self.box_size.min(rect.width()).min(rect.height()).max(0.0);
        let min = Vec2::new(rect.min.x, rect.center().y - side * 0.5);
        snap(Rect::from_origin_size(min, Vec2::splat(side)))
    }
}

impl Ui<'_> {
    /// A checkbox: its box sits at the left of `rect`, and the whole of `rect` (where its
    /// label will go) reacts to the pointer. A click flips `checked` and reports `changed`.
    pub fn checkbox(&mut self, id: WidgetId, rect: Rect, checked: &mut bool) -> Response {
        let mut response = self.interact(id, rect);
        if response.clicked {
            *checked = !*checked;
            response.changed = true;
        }
        let style = &self.styles().checkbox;
        let (background, border) = if *checked {
            (style.background_checked, style.border_color_checked)
        } else {
            (style.background, style.border_color)
        };
        let bounds = style.box_rect(rect);
        self.scene.push_rect(
            &RectPrimitive::new(bounds, background.pick(response.state()))
                .with_corner_radius(style.corner_radius)
                .with_border(style.border_width, border),
        );
        if *checked {
            Icon::Check.draw(self.scene, bounds, style.mark, style.mark_width);
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;

    use crate::{Input, UiState};

    use super::*;

    #[test]
    fn the_mark_shows_once_checked() {
        let styles = crate::test_styles();
        let (mut scene, mut state, mut input) = (Scene::new(), UiState::new(), Input::new());
        let rect = Rect::from_origin_size(Vec2::ZERO, Vec2::new(120.0, 24.0));
        let mut checked = false;
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        ui.checkbox(WidgetId::new("option"), rect, &mut checked);
        drop(ui);
        assert_eq!((scene.rect_count(), scene.line_count()), (1, 0));

        // A click on the label area, far from the box, checks it too.
        scene.clear();
        input.primary_button(true, Vec2::new(100.0, 12.0));
        input.primary_button(false, Vec2::new(100.0, 12.0));
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        let response = ui.checkbox(WidgetId::new("option"), rect, &mut checked);
        drop(ui);
        assert!(checked && response.changed);
        assert_eq!((scene.rect_count(), scene.line_count()), (1, 2));
    }

    #[test]
    fn the_box_is_centered_vertically_on_whole_pixels() {
        let style = crate::test_styles().checkbox;
        let rect = Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::new(100.0, 25.0));
        let bounds = style.box_rect(rect);
        assert_eq!(bounds.size(), Vec2::splat(16.0));
        assert_eq!(bounds.min, Vec2::new(10.0, 15.0));
    }
}
