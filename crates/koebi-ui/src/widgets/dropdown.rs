//! Dropdowns: a control showing the current choice, opening a menu of choices below it.

use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_render::RectPrimitive;
use koebi_tokens::component;

use crate::icon::Icon;
use crate::id::WidgetId;
use crate::style::StateColors;
use crate::text::Align;
use crate::ui::Ui;
use crate::widgets::menu::{MenuItem, MenuState};

/// Colors and metrics of [`Ui::dropdown`], from the `component.dropdown.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropdownStyle {
    /// Background per state.
    pub background: StateColors,
    /// Background while the menu is open.
    pub open: Color,
    /// Label color.
    pub label: Color,
    /// Chevron color.
    pub icon: Color,
    /// Border color, at rest and under the pointer or open.
    pub border: Color,
    /// Border color under the pointer or while open.
    pub border_hover: Color,
    /// Border width.
    pub border_width: f32,
    /// Corner radius.
    pub corner_radius: f32,
    /// Space between the sides and the label or chevron.
    pub padding: f32,
    /// Size of the chevron.
    pub chevron_size: f32,
}

impl DropdownStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            background: StateColors {
                normal: tokens.get(component::DROPDOWN_BACKGROUND_COLOR_DEFAULT),
                hovered: tokens.get(component::DROPDOWN_BACKGROUND_COLOR_HOVER),
                pressed: tokens.get(component::DROPDOWN_BACKGROUND_COLOR_PRESSED),
            },
            open: tokens.get(component::DROPDOWN_BACKGROUND_COLOR_OPEN),
            label: tokens.get(component::DROPDOWN_LABEL_COLOR_DEFAULT),
            icon: tokens.get(component::DROPDOWN_ICON_COLOR_DEFAULT),
            border: tokens.get(component::DROPDOWN_BORDER_COLOR_DEFAULT),
            border_hover: tokens.get(component::DROPDOWN_BORDER_COLOR_HOVER),
            border_width: tokens.get(component::DROPDOWN_BORDER_WIDTH_DEFAULT),
            corner_radius: tokens.get(component::DROPDOWN_CORNER_RADIUS_DEFAULT),
            padding: tokens.get(component::DROPDOWN_PADDING_HORIZONTAL_DEFAULT),
            chevron_size: tokens.get(component::DROPDOWN_CHEVRON_SIZE),
        }
    }
}

impl Ui<'_> {
    /// A dropdown filling `rect` and showing `current`. A click opens `items` below it, kept
    /// inside `bounds` (the window); returns the item chosen this frame, by index (see
    /// [`Ui::menu`]). Mark the current item with [`MenuItem::selected`].
    pub fn dropdown(
        &mut self,
        id: WidgetId,
        rect: Rect,
        state: &mut MenuState,
        current: &str,
        items: &[MenuItem<'_>],
        bounds: Rect,
    ) -> Option<usize> {
        let response = self.interact(id, rect);
        // While open, the menu catches presses outside it, this control included: a click
        // here closes it rather than opening it again.
        if response.clicked && !state.is_open() {
            state.open_below(rect);
        }
        let styles = self.styles();
        let style = styles.dropdown;
        let open = state.is_open();
        let background = if open {
            style.open
        } else {
            style.background.pick(response.state())
        };
        let border = if open || response.hovered {
            style.border_hover
        } else {
            style.border
        };
        self.scene.push_rect(
            &RectPrimitive::new(rect, background)
                .with_corner_radius(style.corner_radius)
                .with_border(style.border_width, border),
        );
        let chevron = Rect::from_center_size(
            Vec2::new(
                rect.max.x - style.padding - style.chevron_size * 0.5,
                rect.center().y,
            ),
            Vec2::splat(style.chevron_size),
        );
        let icon = if open {
            Icon::ChevronUp
        } else {
            Icon::ChevronDown
        };
        icon.draw(self.scene, chevron, style.icon, styles.icon.stroke_width);
        let label = Rect::from_min_max(
            Vec2::new(rect.min.x + style.padding, rect.min.y),
            Vec2::new(chevron.min.x - style.padding * 0.5, rect.max.y),
        );
        let font = styles.text.label.clone();
        self.label(current, label, &font, style.label, Align::Start);
        self.menu(id.with(0), state, items, bounds)
    }
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;
    use koebi_text::Fonts;

    use super::*;
    use crate::{Input, UiState};

    const ID: WidgetId = WidgetId::new("editor type");

    fn bounds() -> Rect {
        Rect::from_origin_size(Vec2::ZERO, Vec2::new(400.0, 300.0))
    }

    fn control() -> Rect {
        Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::new(120.0, 20.0))
    }

    fn items() -> [MenuItem<'static>; 3] {
        [
            MenuItem::action("Viewport").selected(true),
            MenuItem::action("Outliner"),
            MenuItem::action("Timeline"),
        ]
    }

    fn frame(
        state: &mut UiState,
        menu: &mut MenuState,
        input: &mut Input,
    ) -> (Option<usize>, Scene) {
        let styles = crate::test_styles();
        let mut scene = Scene::new();
        let chosen = {
            let mut ui = Ui::new(&mut scene, &styles, input, state);
            ui.dropdown(ID, control(), menu, "Viewport", &items(), bounds())
        };
        input.end_frame();
        (chosen, scene)
    }

    fn click(input: &mut Input, at: Vec2) {
        input.primary_button(true, at);
        input.primary_button(false, at);
    }

    #[test]
    fn a_click_opens_the_menu_below_and_a_choice_closes_it() {
        let (mut state, mut menu, mut input) = (UiState::new(), MenuState::new(), Input::new());
        state.set_fonts(Fonts::with_embedded());
        let (_, closed) = frame(&mut state, &mut menu, &mut input);
        // The control, its chevron and its label.
        assert_eq!((closed.rect_count(), closed.line_count()), (1, 2));
        assert!(closed.glyph_count() >= 8);
        click(&mut input, Vec2::new(50.0, 20.0));
        frame(&mut state, &mut menu, &mut input);
        assert!(menu.is_open());
        // The menu lays out below the control: the second item ("Outliner").
        let (_, open) = frame(&mut state, &mut menu, &mut input);
        assert!(open.rect_count() > closed.rect_count());
        let styles = crate::test_styles();
        let top = control().max.y + styles.menu.border_width + styles.menu.padding_vertical;
        let second = Vec2::new(50.0, top + styles.menu.item_height * 1.5);
        click(&mut input, second);
        let (chosen, _) = frame(&mut state, &mut menu, &mut input);
        assert_eq!(chosen, Some(1));
        assert!(!menu.is_open());
    }

    #[test]
    fn clicking_the_control_again_closes_the_menu() {
        let (mut state, mut menu, mut input) = (UiState::new(), MenuState::new(), Input::new());
        frame(&mut state, &mut menu, &mut input);
        click(&mut input, Vec2::new(50.0, 20.0));
        frame(&mut state, &mut menu, &mut input);
        frame(&mut state, &mut menu, &mut input);
        assert!(menu.is_open());
        click(&mut input, Vec2::new(50.0, 20.0));
        frame(&mut state, &mut menu, &mut input);
        frame(&mut state, &mut menu, &mut input);
        assert!(!menu.is_open());
    }
}
