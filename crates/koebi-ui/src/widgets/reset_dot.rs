//! Reset dots: the colored dot beside a value changed from its default, which brings the default
//! back.
//!
//! The caller shows one only where something was changed: beside an input whose value differs
//! from its default, or in the header of a group when any of its values does. A click brings
//! the default back; a press of the secondary button lets the caller offer a choice of what to
//! bring back. Under the pointer the dot grows and a tooltip says what it does.

use koebi_core::{Color, Rect};
use koebi_design::Resolve;
use koebi_render::RectPrimitive;
use koebi_tokens::component;

use crate::id::WidgetId;
use crate::ui::{Response, Ui};

/// Color and sizes of reset dots, from the `component.panel.override-badge.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResetDotStyle {
    /// Color of the dot.
    pub color: Color,
    /// Its side at rest.
    pub size: f32,
    /// Its side under the pointer.
    pub size_hovered: f32,
}

impl ResetDotStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        Self {
            color: tokens.get(c::PANEL_OVERRIDE_BADGE_COLOR_DEFAULT),
            size: tokens.get(c::PANEL_OVERRIDE_BADGE_SIZE_DEFAULT),
            size_hovered: tokens.get(c::PANEL_OVERRIDE_BADGE_SIZE_HOVER),
        }
    }

    /// The side of the dot in `area`: larger under the pointer, never out of `area`.
    fn side(&self, area: Rect, hovered: bool) -> f32 {
        let side = if hovered {
            self.size_hovered
        } else {
            self.size
        };
        side.min(area.width()).min(area.height()).max(0.0)
    }
}

impl Ui<'_> {
    /// A reset dot in the middle of `area`, which takes the pointer over all of it: `clicked`
    /// asks to bring the default back, `secondary_pressed` to offer a choice of what to bring
    /// back. Under the pointer it grows and `hint` shows in a tooltip.
    pub fn reset_dot(&mut self, id: WidgetId, area: Rect, hint: &str) -> Response {
        let response = self.interact(id, area);
        let style = self.styles().reset_dot;
        let side = style.side(area, response.hovered || response.held);
        let dot = self.pixel_centered(area, side);
        self.scene
            .push_rect(&RectPrimitive::new(dot, style.color).with_corner_radius(side * 0.5));
        self.tooltip(id, hint, area);
        response
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::Vec2;
    use koebi_render::Scene;

    use super::*;
    use crate::{Input, UiState};

    const DOT: WidgetId = WidgetId::new("dot");

    fn area() -> Rect {
        Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::splat(20.0))
    }

    /// One frame with the pointer at `pointer`, pressed with `press` when given: the scene
    /// drawn and the dot's response.
    fn frame(
        state: &mut UiState,
        input: &mut Input,
        pointer: Vec2,
        press: Option<bool>,
    ) -> (Scene, Response) {
        let styles = crate::test_styles();
        input.pointer_moved(pointer);
        if let Some(down) = press {
            input.primary_button(down, pointer);
        }
        let mut scene = Scene::new();
        let response = Ui::new(&mut scene, &styles, input, state).reset_dot(DOT, area(), "Reset");
        input.end_frame();
        (scene, response)
    }

    #[test]
    fn a_dot_grows_under_the_pointer_and_a_click_asks_for_the_default() {
        let styles = crate::test_styles();
        let style = styles.reset_dot;
        assert!(style.size_hovered > style.size);
        let (mut state, mut input) = (UiState::new(), Input::new());
        let away = Vec2::new(100.0, 100.0);
        let (scene, response) = frame(&mut state, &mut input, away, None);
        assert_eq!(scene.rect_count(), 1);
        assert!(!response.hovered && !response.clicked);
        // Under the pointer (hit testing reads the frame before), then pressed and released.
        let center = area().center();
        frame(&mut state, &mut input, center, None);
        let (_, response) = frame(&mut state, &mut input, center, None);
        assert!(response.hovered);
        frame(&mut state, &mut input, center, Some(true));
        let (_, response) = frame(&mut state, &mut input, center, Some(false));
        assert!(response.clicked);
    }

    #[test]
    fn a_dot_never_grows_out_of_its_area() {
        let style = ResetDotStyle {
            color: Color::WHITE,
            size: 6.0,
            size_hovered: 9.0,
        };
        assert_eq!(style.side(area(), false), 6.0);
        assert_eq!(style.side(area(), true), 9.0);
        let thin = Rect::from_origin_size(Vec2::ZERO, Vec2::new(30.0, 4.0));
        assert_eq!(style.side(thin, true), 4.0);
        assert_eq!(style.side(Rect::ZERO, false), 0.0);
    }
}
