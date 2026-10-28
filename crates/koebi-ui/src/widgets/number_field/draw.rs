//! How a number field looks: its box in the colors of its state, the bar of a bounded field,
//! its label, its value and unit, and on hover its step buttons.

use koebi_core::{Color, Rect, Vec2};
use koebi_render::{Corners, RectPrimitive};

use super::format::ValueText;
use super::{BACKGROUND, BORDER, FILL, NumberField, Numeric};
use crate::icon::Icon;
use crate::text::Align;
use crate::ui::{Response, Ui};
use crate::widgets::parts::ControlState;
use crate::widgets::snap;

impl<N: Numeric> NumberField<'_, N> {
    /// The color of `part` in `state`: the call's, or the style's.
    fn color(&self, ui: &Ui<'_>, part: usize, state: ControlState) -> Color {
        let style = &ui.styles().number_field;
        let fallback = match part {
            BACKGROUND => style.background,
            BORDER => style.border,
            _ => style.fill,
        }
        .get(state);
        Self::prop(ui, self.colors[part][state.index()].as_ref(), fallback)
    }

    /// Draws the field in `rect` in `state`: the box, the bar, the text (clear of the step
    /// buttons when the field `has_steps`), and the step buttons `steps` when they show.
    #[allow(clippy::too_many_arguments, reason = "the parts of one field")]
    pub(super) fn draw(
        &self,
        ui: &mut Ui<'_>,
        rect: Rect,
        body: Rect,
        state: ControlState,
        has_steps: bool,
        steps: Option<[Rect; 2]>,
        step_responses: &[Response; 2],
    ) {
        let style = &ui.styles().number_field;
        let radii = self
            .radii
            .as_ref()
            .map_or(Corners::all(style.corner_radius), |radii| {
                radii.resolve(&ui.styles().tokens)
            });
        let border_width = Self::prop(ui, self.border_width.as_ref(), style.border_width);
        let (background, border) = (
            self.color(ui, BACKGROUND, state),
            self.color(ui, BORDER, state),
        );
        let rect = snap(rect);
        ui.scene().push_rect(
            &RectPrimitive::new(rect, background)
                .with_corner_radii(radii)
                .with_border(border_width, border),
        );
        ui.clipped(rect, |ui| {
            self.draw_bar(ui, rect, radii, border_width, state);
            if let Some(areas) = steps {
                self.draw_steps(ui, areas, step_responses);
            }
            self.draw_text(ui, rect, body, has_steps, state);
        });
    }

    /// The bar of a bounded field, inside the border, filled to where the value lies.
    fn draw_bar(
        &self,
        ui: &mut Ui<'_>,
        rect: Rect,
        radii: Corners,
        border_width: f32,
        state: ControlState,
    ) {
        let Some(mapping) = self.shown_bar() else {
            return;
        };
        #[allow(
            clippy::cast_possible_truncation,
            reason = "a fraction from 0 to 1 fits f32"
        )]
        let fraction = mapping.fraction(self.value.to_f64(), self.min, self.max) as f32;
        if fraction <= 0.0 {
            return;
        }
        let inner = rect.inset(border_width);
        let inset = |radius: f32| (radius - border_width).max(0.0);
        let full = fraction >= 0.999;
        let (fill, corners) = if self.vertical {
            let top = inner.max.y - inner.height() * fraction;
            (
                Rect::from_min_max(Vec2::new(inner.min.x, top), inner.max),
                Corners {
                    top_left: if full { inset(radii.top_left) } else { 0.0 },
                    top_right: if full { inset(radii.top_right) } else { 0.0 },
                    bottom_left: inset(radii.bottom_left),
                    bottom_right: inset(radii.bottom_right),
                },
            )
        } else {
            let right = inner.min.x + inner.width() * fraction;
            (
                Rect::from_min_max(inner.min, Vec2::new(right, inner.max.y)),
                Corners {
                    top_left: inset(radii.top_left),
                    bottom_left: inset(radii.bottom_left),
                    top_right: if full { inset(radii.top_right) } else { 0.0 },
                    bottom_right: if full { inset(radii.bottom_right) } else { 0.0 },
                },
            )
        };
        let color = self.color(ui, FILL, state);
        ui.scene()
            .push_rect(&RectPrimitive::new(fill, color).with_corner_radii(corners));
    }

    /// The step buttons: a chevron each, the hovered one on its own background.
    fn draw_steps(&self, ui: &mut Ui<'_>, areas: [Rect; 2], responses: &[Response; 2]) {
        let style = &ui.styles().number_field;
        let icons = if self.vertical {
            [Icon::ChevronDown, Icon::ChevronUp]
        } else {
            [Icon::ChevronLeft, Icon::ChevronRight]
        };
        let stroke = ui.styles().icon.stroke_width;
        for ((area, icon), response) in areas.into_iter().zip(icons).zip(responses) {
            let hovered = response.hovered || response.held;
            if hovered {
                ui.scene()
                    .push_rect(&RectPrimitive::new(area, style.step_background));
            }
            let color = if hovered {
                style.step_hovered
            } else {
                style.step
            };
            icon.draw(
                ui.scene(),
                Icon::centered(area, style.step_icon_size),
                color,
                stroke,
            );
        }
    }

    /// The label at the start and the value at the end, or the value centred; the unit after
    /// the value in its own color.
    fn draw_text(&self, ui: &mut Ui<'_>, rect: Rect, body: Rect, steps: bool, state: ControlState) {
        let style = &ui.styles().number_field;
        let font = &ui.styles().text.body;
        // The text keeps clear of the step buttons, shown or not, so that it does not move.
        let area = if self.vertical || !steps { rect } else { body };
        let area = Rect::from_min_max(
            Vec2::new(area.min.x + style.padding, area.min.y),
            Vec2::new(
                (area.max.x - style.padding).max(area.min.x + style.padding),
                area.max.y,
            ),
        );
        let disabled = state == ControlState::Disabled;
        let text_color = if disabled {
            style.text_disabled
        } else {
            style.text
        };
        let unit_color = if disabled {
            style.text_disabled
        } else {
            style.unit
        };
        let number = ValueText::shown(self.value.to_f64(), self.decimals_shown(), None);
        let unit = ValueText::unit(self.unit);
        let unit = unit.as_str();
        let number_width = ui.text_width(number.as_str(), font);
        let unit_width = ui.text_width(unit, font);
        let total = number_width + unit_width;
        // A narrow vertical field shows no value rather than a cut one.
        if self.vertical && total > area.width() {
            return;
        }
        let start = match self.label.filter(|_| !self.vertical) {
            Some(label) => {
                let label_color = if disabled {
                    style.text_disabled
                } else {
                    style.label
                };
                let room = Rect::from_min_max(
                    area.min,
                    Vec2::new(
                        (area.max.x - total - style.padding).max(area.min.x),
                        area.max.y,
                    ),
                );
                ui.label(label, room, font, label_color, Align::Start);
                area.max.x - total
            }
            None => area.min.x + (area.width() - total) * 0.5,
        };
        let start = start.max(area.min.x);
        let number_rect = Rect::from_min_max(
            Vec2::new(start, area.min.y),
            Vec2::new((start + number_width).min(area.max.x), area.max.y),
        );
        ui.label(number.as_str(), number_rect, font, text_color, Align::Start);
        if !unit.is_empty() {
            let unit_rect = Rect::from_min_max(
                Vec2::new(number_rect.max.x, area.min.y),
                Vec2::new(area.max.x, area.max.y),
            );
            ui.label(unit, unit_rect, font, unit_color, Align::Start);
        }
    }
}
