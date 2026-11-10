//! Theme cards: a miniature of a theme, painted with that theme's own tokens: a mock
//! interface of shapes (never real components, which would seem to answer the pointer), with
//! the colors where themes and color vision modes differ the most: surfaces, text, the accent,
//! the colors of states and a chart.

use koebi_core::{Color, Rect, Vec2};
use koebi_design::{DesignSystem, Layers, Resolve, ThemeId};
use koebi_render::{Corners, RectPrimitive};
use koebi_tokens::{component, semantic};

use crate::icon::{Mark, draw_mark};
use crate::id::WidgetId;
use crate::text::{Align, FontStyle};
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

/// What a card shows of a theme, resolved in that theme (or as a color vision mode shows it):
/// its surfaces, text, controls, the colors of states and of charts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemePreview {
    /// Window background.
    pub background: Color,
    /// Background of the application bar.
    pub app_bar: Color,
    /// Background of the status bar.
    pub status_bar: Color,
    /// Panel background.
    pub panel: Color,
    /// Panel border.
    pub panel_border: Color,
    /// Background of a selected row.
    pub selection: Color,
    /// Primary text.
    pub text_primary: Color,
    /// Secondary text.
    pub text_secondary: Color,
    /// Background of the accent button.
    pub accent: Color,
    /// Label of the accent button.
    pub accent_label: Color,
    /// Background of a plain button.
    pub button: Color,
    /// Label of a plain button.
    pub button_label: Color,
    /// Background of a field.
    pub field: Color,
    /// The bar of a field's value.
    pub field_fill: Color,
    /// Background of a checked checkbox.
    pub check_background: Color,
    /// Check mark.
    pub check_mark: Color,
    /// The colors of states: positive, negative, notice and info buttons.
    pub tones: [Color; 4],
    /// Their labels.
    pub tone_labels: [Color; 4],
    /// The categorical colors of charts.
    pub chart: [Color; 8],
    /// Corner radius of panels and controls.
    pub corner_radius: f32,
}

impl ThemePreview {
    /// Reads a preview from `tokens`, usually a [`View`](koebi_design::View) of the theme.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        use semantic as s;
        Self {
            background: tokens.get(s::BACKGROUND_BASE_DEFAULT),
            app_bar: tokens.get(c::TITLE_BAR_BACKGROUND_COLOR_DEFAULT),
            status_bar: tokens.get(c::STATUS_BAR_BACKGROUND_COLOR_DEFAULT),
            panel: tokens.get(c::PANEL_BODY_BACKGROUND_COLOR_LEVEL_1),
            panel_border: tokens.get(c::PANEL_BORDER_COLOR_DEFAULT),
            selection: tokens.get(c::OUTLINER_ROW_BACKGROUND_SELECTED),
            text_primary: tokens.get(s::TEXT_COLOR_PRIMARY_DEFAULT),
            text_secondary: tokens.get(s::TEXT_COLOR_SECONDARY_DEFAULT),
            accent: tokens.get(c::BUTTON_ACCENT_BACKGROUND_COLOR_DEFAULT),
            accent_label: tokens.get(c::BUTTON_ACCENT_LABEL_COLOR_DEFAULT),
            button: tokens.get(c::BUTTON_BACKGROUND_COLOR_DEFAULT),
            button_label: tokens.get(c::BUTTON_LABEL_COLOR_DEFAULT),
            field: tokens.get(c::NUMBER_FIELD_BACKGROUND_COLOR_DEFAULT),
            field_fill: tokens.get(c::NUMBER_FIELD_FILL_COLOR_DEFAULT),
            check_background: tokens.get(c::CHECKBOX_BACKGROUND_COLOR_SELECTED),
            check_mark: tokens.get(c::CHECKBOX_MARK_COLOR_DEFAULT),
            tones: [
                tokens.get(c::BUTTON_POSITIVE_BACKGROUND_COLOR_DEFAULT),
                tokens.get(c::BUTTON_NEGATIVE_BACKGROUND_COLOR_DEFAULT),
                tokens.get(c::BUTTON_NOTICE_BACKGROUND_COLOR_DEFAULT),
                tokens.get(c::BUTTON_INFO_BACKGROUND_COLOR_DEFAULT),
            ],
            tone_labels: [
                tokens.get(c::BUTTON_POSITIVE_LABEL_COLOR_DEFAULT),
                tokens.get(c::BUTTON_NEGATIVE_LABEL_COLOR_DEFAULT),
                tokens.get(c::BUTTON_NOTICE_LABEL_COLOR_DEFAULT),
                tokens.get(c::BUTTON_INFO_LABEL_COLOR_DEFAULT),
            ],
            chart: [
                tokens.get(s::DATA_VIZ_CATEGORICAL_SHORT_1),
                tokens.get(s::DATA_VIZ_CATEGORICAL_SHORT_2),
                tokens.get(s::DATA_VIZ_CATEGORICAL_SHORT_3),
                tokens.get(s::DATA_VIZ_CATEGORICAL_SHORT_4),
                tokens.get(s::DATA_VIZ_CATEGORICAL_SHORT_5),
                tokens.get(s::DATA_VIZ_CATEGORICAL_SHORT_6),
                tokens.get(s::DATA_VIZ_CATEGORICAL_EXTENDED_7),
                tokens.get(s::DATA_VIZ_CATEGORICAL_EXTENDED_8),
            ],
            corner_radius: tokens.get(c::PANEL_CORNER_RADIUS_DEFAULT),
        }
    }

    /// The preview of `theme` as the user sees it: with its overrides, the global
    /// overrides and the color vision setting. `None` for a removed theme.
    pub fn of(design: &DesignSystem, theme: ThemeId) -> Option<Self> {
        design
            .view(theme, Layers::ALL)
            .map(|view| Self::resolve(&view))
    }
}

/// The text of the miniatures: tiny, but text, set in the label font at `size`.
fn tiny(ui: &Ui<'_>, size: f32) -> FontStyle {
    FontStyle {
        size,
        ..ui.styles().text.label.clone()
    }
}

/// What happened to a card, and the room inside its frame ([`Ui::card`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CardResponse {
    /// The pointer's interaction with the card.
    pub response: Response,
    /// The room inside the frame and its padding.
    pub content: Rect,
    /// The corner radius that follows the frame's inside the room.
    pub corner_radius: f32,
}

impl Ui<'_> {
    /// A card filling `rect`: a frame in the style of theme cards, marked when `selected` and
    /// lighter under the pointer, around what the caller draws in
    /// [`CardResponse::content`]. Clicks are reported.
    pub fn card(&mut self, id: WidgetId, rect: Rect, selected: bool) -> CardResponse {
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
        CardResponse {
            response,
            content: rect.inset(inset),
            corner_radius: (style.corner_radius - inset).max(0.0),
        }
    }

    /// A card filling `rect` that shows `preview` as a theme: a miniature of the application in
    /// it. The card itself uses the active theme's style: `selected` marks the active theme's
    /// card. Clicks are reported.
    pub fn theme_card(
        &mut self,
        id: WidgetId,
        rect: Rect,
        preview: &ThemePreview,
        selected: bool,
    ) -> Response {
        let card = self.card(id, rect, selected);
        if !card.content.is_empty() {
            self.paint_application(preview, card.content, card.corner_radius);
        }
        card.response
    }

    /// A card filling `rect` that shows `preview` as a color vision mode sees it: the contrasts
    /// of text on its surfaces, the colors of states and the colors of charts side by side,
    /// where the mode's changes show. Clicks are reported.
    pub fn vision_card(
        &mut self,
        id: WidgetId,
        rect: Rect,
        preview: &ThemePreview,
        selected: bool,
    ) -> Response {
        let card = self.card(id, rect, selected);
        if !card.content.is_empty() {
            self.paint_contrasts(preview, card.content, card.corner_radius);
        }
        card.response
    }

    /// The miniature of the application in `rect`: its bar, a side panel with a list and its
    /// selected row, a title and text, a field, a checkbox, a button, a chart, the status bar.
    /// Flat shapes and tiny text, which never look like controls to press.
    pub fn paint_application(&mut self, preview: &ThemePreview, rect: Rect, corner_radius: f32) {
        let p = preview;
        let rounded = |rect: Rect, color: Color, radius: f32| {
            RectPrimitive::new(rect, color).with_corner_radius(radius)
        };
        let unit = (rect.height() / 12.0).max(1.0);
        let font = tiny(self, (unit * 1.2).clamp(5.0, 12.0));
        let radius = (p.corner_radius * 0.5).min(unit * 0.5);
        self.scene
            .push_rect(&rounded(rect, p.background, corner_radius));
        self.clipped_rounded(rect, Corners::all(corner_radius), |ui| {
            // The application bar, with the application's name.
            let bar = Rect::from_min_max(rect.min, Vec2::new(rect.max.x, rect.min.y + unit * 1.8));
            ui.scene.push_rect(&RectPrimitive::new(bar, p.app_bar));
            let logo = Rect::from_center_size(
                Vec2::new(bar.min.x + unit * 1.1, bar.center().y),
                Vec2::splat(unit),
            );
            ui.scene.push_rect(&rounded(logo, p.accent, unit * 0.25));
            let name = Rect::from_min_max(
                Vec2::new(logo.max.x + unit * 0.5, bar.min.y),
                Vec2::new(bar.max.x, bar.max.y),
            );
            ui.label("Koebi", name, &font, p.text_secondary, Align::Start);
            // The status bar.
            let status =
                Rect::from_min_max(Vec2::new(rect.min.x, rect.max.y - unit * 1.5), rect.max);
            ui.scene
                .push_rect(&RectPrimitive::new(status, p.status_bar));
            let ready = Rect::from_min_max(
                Vec2::new(status.min.x + unit * 0.6, status.min.y),
                status.max,
            );
            ui.label("Ready", ready, &font, p.text_secondary, Align::Start);
            // A side panel: a list, its second row selected.
            let side = Rect::from_min_max(
                Vec2::new(rect.min.x + unit * 0.5, bar.max.y + unit * 0.5),
                Vec2::new(rect.min.x + rect.width() * 0.36, status.min.y - unit * 0.5),
            );
            ui.scene
                .push_rect(&rounded(side, p.panel, radius).with_border(1.0, p.panel_border));
            for (index, item) in ["Scene", "Layer", "Shape"].into_iter().enumerate() {
                let row = Rect::from_origin_size(
                    Vec2::new(
                        side.min.x + 1.0,
                        side.min.y + unit * 0.5 + unit * 1.7 * index as f32,
                    ),
                    Vec2::new(side.width() - 2.0, unit * 1.5),
                );
                let ink = if index == 1 {
                    ui.scene.push_rect(&rounded(row, p.selection, radius));
                    p.text_primary
                } else {
                    p.text_secondary
                };
                let text =
                    Rect::from_min_max(Vec2::new(row.min.x + unit * 0.6, row.min.y), row.max);
                ui.label(item, text, &font, ink, Align::Start);
            }
            // The editor: a title, a line of text, a field, a checkbox and two buttons.
            let left = side.max.x + unit * 0.8;
            let right = rect.max.x - unit * 0.8;
            let line = |row: f32| side.min.y + unit * 1.6 * row;
            let title = Rect::from_min_max(Vec2::new(left, line(0.0)), Vec2::new(right, line(1.0)));
            let heading = FontStyle {
                size: font.size * 1.35,
                ..font.clone()
            };
            ui.label("Aa Title", title, &heading, p.text_primary, Align::Start);
            let body = Rect::from_min_max(Vec2::new(left, line(1.0)), Vec2::new(right, line(1.9)));
            ui.label(
                "Text over the theme",
                body,
                &font,
                p.text_secondary,
                Align::Start,
            );
            let width = right - left;
            let field = Rect::from_min_max(
                Vec2::new(left, line(2.1)),
                Vec2::new(left + width * 0.62, line(2.1) + unit * 1.3),
            );
            ui.scene.push_rect(&rounded(field, p.field, radius));
            let fill = Rect::from_min_max(
                field.min,
                Vec2::new(field.min.x + field.width() * 0.6, field.max.y),
            );
            ui.scene.push_rect(&rounded(fill, p.field_fill, radius));
            let value = field.inset(unit * 0.3);
            ui.label("0.60", value, &font, p.text_primary, Align::End);
            let check = Rect::from_origin_size(
                Vec2::new(field.max.x + unit * 0.6, field.min.y + unit * 0.15),
                Vec2::splat(unit),
            );
            ui.scene
                .push_rect(&rounded(check, p.check_background, radius * 0.6));
            draw_mark(
                ui.scene,
                check,
                Mark::Check,
                p.check_mark,
                (unit * 0.16).max(0.8),
            );
            let buttons = line(3.6);
            let button = |x: f32| {
                Rect::from_min_max(
                    Vec2::new(x, buttons),
                    Vec2::new(x + width * 0.27, buttons + unit * 1.4),
                )
            };
            let ok = button(left);
            ui.scene.push_rect(&rounded(ok, p.accent, radius));
            ui.label("OK", ok, &font, p.accent_label, Align::Center);
            let cancel = button(ok.max.x + unit * 0.4);
            ui.scene.push_rect(&rounded(cancel, p.button, radius));
            ui.label("No", cancel, &font, p.button_label, Align::Center);
            // A chart beside them: bars of its first colors, standing on the buttons' line.
            let chart_left = cancel.max.x + unit * 0.8;
            let column = ((right - chart_left) / 6.5).max(0.0);
            for (index, (color, share)) in p
                .chart
                .into_iter()
                .take(4)
                .zip([0.5, 0.9, 0.35, 0.7])
                .enumerate()
            {
                let height = unit * 2.4 * share;
                let x = chart_left + column * 1.6 * index as f32;
                let at = Rect::from_min_max(
                    Vec2::new(x, ok.max.y - height),
                    Vec2::new(x + column, ok.max.y),
                );
                ui.scene.push_rect(&rounded(at, color, unit * 0.15));
            }
        });
    }

    /// The board of contrasts in `rect`, for color vision modes: text on the surfaces it lies
    /// on ("Aa" on the window, on a panel, on the accent, on a button), the colors of states
    /// with their labels, and the categorical colors of charts side by side.
    pub fn paint_contrasts(&mut self, preview: &ThemePreview, rect: Rect, corner_radius: f32) {
        let p = preview;
        let unit = (rect.height() / 12.0).max(1.0);
        let gap = (unit * 0.35).max(1.0);
        let radius = (p.corner_radius * 0.5).min(unit * 0.5);
        let big = tiny(self, (unit * 2.4).clamp(6.0, 22.0));
        let small = tiny(self, (unit * 1.15).clamp(5.0, 12.0));
        self.scene
            .push_rect(&RectPrimitive::new(rect, p.background).with_corner_radius(corner_radius));
        let inner = rect.inset(unit * 0.6);
        // Text on its surfaces.
        let samples = [
            (p.background, p.text_primary, false),
            (p.panel, p.text_secondary, true),
            (p.accent, p.accent_label, false),
            (p.button, p.button_label, false),
        ];
        let tile_width = (inner.width() - 3.0 * gap) / 4.0;
        let tile_height = unit * 4.2;
        for (index, (surface, ink, framed)) in samples.into_iter().enumerate() {
            let at = Rect::from_origin_size(
                Vec2::new(inner.min.x + (tile_width + gap) * index as f32, inner.min.y),
                Vec2::new(tile_width, tile_height),
            );
            let mut tile = RectPrimitive::new(at, surface).with_corner_radius(radius);
            if framed || index == 0 {
                tile = tile.with_border(1.0, p.panel_border);
            }
            self.scene.push_rect(&tile);
            self.label("Aa", at, &big, ink, Align::Center);
        }
        // The colors of states, with their labels.
        let tones_top = inner.min.y + tile_height + gap * 2.0;
        let chip_height = unit * 2.0;
        for (index, (color, ink)) in p.tones.into_iter().zip(p.tone_labels).enumerate() {
            let at = Rect::from_origin_size(
                Vec2::new(inner.min.x + (tile_width + gap) * index as f32, tones_top),
                Vec2::new(tile_width, chip_height),
            );
            self.scene
                .push_rect(&RectPrimitive::new(at, color).with_corner_radius(radius));
            let name = ["Done", "Error", "Note", "Info"][index];
            self.label(name, at, &small, ink, Align::Center);
        }
        // The colors of charts, side by side: what a mode confuses shows as neighbours alike.
        let chart_top = tones_top + chip_height + gap * 2.0;
        let chart = Rect::from_min_max(Vec2::new(inner.min.x, chart_top), inner.max);
        let cell = chart.width() / p.chart.len() as f32;
        for (index, color) in p.chart.into_iter().enumerate() {
            let at = Rect::from_min_max(
                Vec2::new(chart.min.x + cell * index as f32, chart.min.y),
                Vec2::new(chart.min.x + cell * (index + 1) as f32, chart.max.y),
            );
            let corners = Corners {
                top_left: if index == 0 { radius } else { 0.0 },
                bottom_left: if index == 0 { radius } else { 0.0 },
                top_right: if index + 1 == p.chart.len() {
                    radius
                } else {
                    0.0
                },
                bottom_right: if index + 1 == p.chart.len() {
                    radius
                } else {
                    0.0
                },
            };
            self.scene
                .push_rect(&RectPrimitive::new(at, color).with_corner_radii(corners));
        }
    }
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;
    use koebi_tokens::KOEBI;

    use crate::{Input, UiState};

    use super::*;

    #[test]
    fn previews_read_the_theme_they_show() {
        let design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
        let light = design.find_theme("snow").expect("built-in theme");
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
        // Frame, background, the two bars, the logo, the side panel and its selected row, a
        // field and its bar, a checkbox, two buttons, four bars of the chart; one check mark.
        assert_eq!((scene.rect_count(), scene.line_count()), (16, 1));
        // A vision card: frame, background, four tiles, four states, eight chart colors.
        let (mut scene, mut state) = (Scene::new(), UiState::new());
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        ui.vision_card(WidgetId::new("vision"), rect, &preview, false);
        drop(ui);
        assert_eq!(scene.rect_count(), 18);
    }
}
