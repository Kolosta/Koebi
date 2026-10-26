//! Theme cards: a miniature of a theme, painted with that theme's own tokens.

use koebi_core::{Color, Rect, Vec2};
use koebi_design::{DesignSystem, Layers, Resolve, ThemeId};
use koebi_render::{RectPrimitive, Scene};
use koebi_tokens::{component, semantic};

use crate::icon::Icon;
use crate::id::WidgetId;
use crate::ui::{Response, Ui};

/// Colors and metrics of the card around a preview, from the `component.theme-card.*`
/// tokens of the active theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeCardStyle {
    /// Corner radius of the card.
    pub corner_radius: f32,
    /// Border color at rest.
    pub border_color: Color,
    /// Border color under the pointer.
    pub border_color_hovered: Color,
    /// Border color of the active theme's card.
    pub border_color_selected: Color,
    /// Border width.
    pub border_width: f32,
    /// Border width of the active theme's card.
    pub border_width_selected: f32,
    /// Space between the border and the preview.
    pub padding: f32,
}

impl ThemeCardStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            corner_radius: tokens.get(component::THEME_CARD_CORNER_RADIUS_DEFAULT),
            border_color: tokens.get(component::THEME_CARD_BORDER_COLOR_DEFAULT),
            border_color_hovered: tokens.get(component::THEME_CARD_BORDER_COLOR_HOVER),
            border_color_selected: tokens.get(component::THEME_CARD_BORDER_COLOR_SELECTED),
            border_width: tokens.get(component::THEME_CARD_BORDER_WIDTH_DEFAULT),
            border_width_selected: tokens.get(component::THEME_CARD_BORDER_WIDTH_SELECTED),
            padding: tokens.get(component::THEME_CARD_PADDING_DEFAULT),
        }
    }
}

/// What a theme card shows of a theme, resolved in that theme: its surfaces, text and accent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemePreview {
    /// Window background.
    pub background: Color,
    /// Panel background.
    pub panel: Color,
    /// Panel border.
    pub panel_border: Color,
    /// Primary text.
    pub text_primary: Color,
    /// Secondary text.
    pub text_secondary: Color,
    /// Background of the accent button.
    pub accent: Color,
    /// Label of the accent button.
    pub accent_label: Color,
    /// Background of a checked checkbox.
    pub check_background: Color,
    /// Check mark.
    pub check_mark: Color,
    /// Corner radius of panels and controls.
    pub corner_radius: f32,
}

impl ThemePreview {
    /// Reads a preview from `tokens`, usually a [`View`](koebi_design::View) of the theme.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            background: tokens.get(semantic::BACKGROUND_BASE_DEFAULT),
            panel: tokens.get(component::PANEL_BODY_BACKGROUND_COLOR_LEVEL_1),
            panel_border: tokens.get(component::PANEL_BORDER_COLOR_DEFAULT),
            text_primary: tokens.get(semantic::TEXT_COLOR_PRIMARY_DEFAULT),
            text_secondary: tokens.get(semantic::TEXT_COLOR_SECONDARY_DEFAULT),
            accent: tokens.get(component::BUTTON_ACCENT_BACKGROUND_COLOR_DEFAULT),
            accent_label: tokens.get(component::BUTTON_ACCENT_LABEL_COLOR_DEFAULT),
            check_background: tokens.get(component::CHECKBOX_BACKGROUND_COLOR_SELECTED),
            check_mark: tokens.get(component::CHECKBOX_MARK_COLOR_DEFAULT),
            corner_radius: tokens.get(component::PANEL_CORNER_RADIUS_DEFAULT),
        }
    }

    /// The preview of `theme` as the user sees it: with its overrides, the global
    /// overrides and the color vision setting. `None` for a removed theme.
    pub fn of(design: &DesignSystem, theme: ThemeId) -> Option<Self> {
        design
            .view(theme, Layers::ALL)
            .map(|view| Self::resolve(&view))
    }

    /// Paints the miniature in `rect`: a panel with text and a checked checkbox, a heading,
    /// a paragraph and a button. Text is drawn as bars until Koebi renders text.
    pub fn paint(&self, scene: &mut Scene, rect: Rect, corner_radius: f32) {
        let rounded = |rect: Rect, color: Color, radius: f32| {
            RectPrimitive::new(rect, color).with_corner_radius(radius)
        };
        // A bar of text: a pill `width` long and `unit` tall.
        let bar = |x: f32, y: f32, width: f32, unit: f32| {
            Rect::from_origin_size(Vec2::new(x, y), Vec2::new(width, unit * 0.6))
        };
        scene.push_rect(&rounded(rect, self.background, corner_radius));
        let unit = (rect.height() / 10.0).max(1.0);
        let (x0, y0, width) = (rect.min.x, rect.min.y, rect.width());
        let pill = unit * 0.3;

        // Panel on the left, with two lines of text and a checked checkbox.
        let panel = Rect::from_min_max(
            Vec2::new(x0 + unit, y0 + unit),
            Vec2::new(x0 + width * 0.45, rect.max.y - unit),
        );
        scene.push_rect(
            &rounded(panel, self.panel, self.corner_radius).with_border(1.0, self.panel_border),
        );
        let inner_x = panel.min.x + unit;
        let inner_width = (panel.width() - 2.0 * unit).max(0.0);
        let line = |row: f32| panel.min.y + unit * (1.0 + 1.5 * row);
        scene.push_rect(&rounded(
            bar(inner_x, line(0.0), inner_width * 0.8, unit),
            self.text_primary,
            pill,
        ));
        scene.push_rect(&rounded(
            bar(inner_x, line(1.0), inner_width * 0.6, unit),
            self.text_secondary,
            pill,
        ));
        let check = Rect::from_origin_size(Vec2::new(inner_x, line(2.0)), Vec2::splat(unit * 1.2));
        scene.push_rect(&rounded(
            check,
            self.check_background,
            self.corner_radius * 0.5,
        ));
        Icon::Check.draw(scene, check, self.check_mark, (unit * 0.2).max(1.0));
        scene.push_rect(&rounded(
            bar(
                check.max.x + unit * 0.5,
                line(2.0) + unit * 0.3,
                inner_width * 0.4,
                unit,
            ),
            self.text_secondary,
            pill,
        ));

        // On the right: a heading, a paragraph and a button.
        let right_x = panel.max.x + unit;
        let right_width = (rect.max.x - unit - right_x).max(0.0);
        let heading = Rect::from_origin_size(
            Vec2::new(right_x, y0 + unit * 1.2),
            Vec2::new(right_width * 0.8, unit),
        );
        scene.push_rect(&rounded(heading, self.text_primary, unit * 0.5));
        for (row, length) in [0.95, 0.7, 0.85].into_iter().enumerate() {
            let y = heading.max.y + unit * (0.9 + 1.1 * row as f32);
            scene.push_rect(&rounded(
                bar(right_x, y, right_width * length, unit),
                self.text_secondary,
                pill,
            ));
        }
        let button = Rect::from_min_max(
            Vec2::new(right_x, rect.max.y - unit * 3.2),
            Vec2::new(right_x + right_width * 0.6, rect.max.y - unit),
        );
        scene.push_rect(&rounded(button, self.accent, self.corner_radius));
        let label =
            Rect::from_center_size(button.center(), Vec2::new(button.width() * 0.5, unit * 0.6));
        scene.push_rect(&rounded(label, self.accent_label, pill));
    }
}

impl Ui<'_> {
    /// A card filling `rect` that shows `preview`. The card itself uses the active theme's
    /// style: `selected` marks the active theme's card. Clicks are reported.
    pub fn theme_card(
        &mut self,
        id: WidgetId,
        rect: Rect,
        preview: &ThemePreview,
        selected: bool,
    ) -> Response {
        let response = self.interact(id, rect);
        let style = &self.styles().theme_card;
        let (border, width) = if selected {
            (style.border_color_selected, style.border_width_selected)
        } else if response.hovered {
            (style.border_color_hovered, style.border_width)
        } else {
            (style.border_color, style.border_width)
        };
        self.scene.push_rect(
            &RectPrimitive::new(rect, Color::TRANSPARENT)
                .with_corner_radius(style.corner_radius)
                .with_border(width, border),
        );
        let inset = width + style.padding;
        let content = rect.inset(inset);
        if !content.is_empty() {
            let radius = (style.corner_radius - inset).max(0.0);
            preview.paint(self.scene, content, radius);
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use koebi_tokens::KOEBI;

    use crate::{Input, UiState};

    use super::*;

    #[test]
    fn previews_read_the_theme_they_show() {
        let design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
        let light = design.find_theme("light").expect("built-in theme");
        let preview = ThemePreview::of(&design, light).expect("live theme");
        let view = design.view(light, Layers::ALL).expect("live theme");
        assert_eq!(
            preview.background,
            view.get(semantic::BACKGROUND_BASE_DEFAULT)
        );
        assert_ne!(
            preview.background,
            design.get(semantic::BACKGROUND_BASE_DEFAULT)
        );
    }

    #[test]
    fn cards_draw_a_frame_and_the_miniature() {
        let styles = crate::test_styles();
        let design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
        let preview = ThemePreview::resolve(&design);
        let (mut scene, mut state, input) = (Scene::new(), UiState::new(), Input::new());
        let rect = Rect::from_origin_size(Vec2::ZERO, Vec2::new(160.0, 100.0));
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        ui.theme_card(WidgetId::new("dark"), rect, &preview, true);
        drop(ui);
        // Frame, background, panel, 2 lines, checkbox, its label, heading, 3 lines, button
        // and its label; the check mark is two segments.
        assert_eq!((scene.rect_count(), scene.line_count()), (13, 2));
    }
}
