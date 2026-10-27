//! How a field looks: its box in the colors of its state, its icon, its text cut with an
//! ellipsis or its placeholder while idle, and while it edits, the text scrolled to keep the
//! caret in view, the selection and the caret, blinking when its token says so.

use core::time::Duration;

use koebi_core::{Color, Rect, Vec2};
use koebi_render::{Corners, RectPrimitive};

use super::geometry::{caret_x, scroll_to};
use super::session::Session;
use super::{BACKGROUND, BORDER, ICON, TEXT_PART, TextField, TextFieldResponse};
use crate::icon::Icon;
use crate::ui::Ui;
use crate::widgets::parts::ControlState;
use crate::widgets::snap;

/// Blink intervals shorter than this keep the caret shown: a theme cannot ask for hundreds of
/// frames a second.
const MIN_BLINK: Duration = Duration::from_millis(50);

impl TextField<'_> {
    /// The color of `part` in `state`: the call's, or the style's.
    fn color(&self, ui: &Ui<'_>, part: usize, state: ControlState) -> Color {
        let style = &ui.styles().text_field;
        let disabled = state == ControlState::Disabled;
        let fallback = match part {
            BACKGROUND => style.background.get(state),
            BORDER => style.border.get(state),
            TEXT_PART if disabled => style.text_disabled,
            TEXT_PART => style.text,
            _ if disabled => style.icon_disabled,
            _ => style.icon,
        };
        Self::prop(ui, self.colors[part][state.index()].as_ref(), fallback)
    }

    /// Draws the field: its box, icon and text, and while it edits, the selection and caret.
    pub(super) fn draw(
        &self,
        ui: &mut Ui<'_>,
        rect: Rect,
        session: Option<&mut Session>,
        out: &TextFieldResponse,
    ) {
        let style = &ui.styles().text_field;
        let state = if !self.enabled {
            ControlState::Disabled
        } else if session.is_some() {
            ControlState::Selected
        } else if out.response.hovered || out.response.held {
            ControlState::Hovered
        } else {
            ControlState::Normal
        };
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
        ui.scene().push_rect(
            &RectPrimitive::new(snap(rect), background)
                .with_corner_radii(radii)
                .with_border(border_width, border),
        );
        // What lies inside the box stays inside it, even in a rectangle smaller than asked.
        ui.clipped(rect, |ui| self.draw_content(ui, rect, state, session));
    }

    /// Draws the icon and the text, and while the field edits, the selection and caret.
    fn draw_content(
        &self,
        ui: &mut Ui<'_>,
        rect: Rect,
        state: ControlState,
        session: Option<&mut Session>,
    ) {
        let style = &ui.styles().text_field;
        let area = self.text_area(ui, rect);
        if let Some(icon) = self.icon {
            let color = self.color(ui, ICON, state);
            let slot = Rect::from_origin_size(
                Vec2::new(area.min.x - style.gap - style.icon_size, rect.min.y),
                Vec2::new(style.icon_size, rect.height()),
            );
            let stroke = ui.styles().icon.stroke_width;
            icon.draw(
                ui.scene(),
                Icon::centered(slot, style.icon_size),
                color,
                stroke,
            );
        }
        let font = self.font.unwrap_or(&ui.styles().text.body);
        let text_color = self.color(ui, TEXT_PART, state);
        let placeholder = Self::prop(ui, self.placeholder_color.as_ref(), style.placeholder);
        match session {
            Some(session) => self.draw_editing(ui, area, session, text_color, placeholder),
            None if self.text.is_empty() => {
                if let Some(text) = self.placeholder {
                    ui.label(text, area, font, placeholder, self.align);
                }
            }
            None => {
                ui.label(self.text, area, font, text_color, self.align);
            }
        }
    }

    /// Draws the text being edited, scrolled to keep the caret in view, with its selection
    /// and caret.
    fn draw_editing(
        &self,
        ui: &mut Ui<'_>,
        area: Rect,
        session: &mut Session,
        text_color: Color,
        placeholder: Color,
    ) {
        let style = &ui.styles().text_field;
        let font = self.font.unwrap_or(&ui.styles().text.body);
        let selection_color = Self::prop(ui, self.selection_color.as_ref(), style.selection);
        let caret_color = Self::prop(ui, self.caret_color.as_ref(), style.caret);
        let caret_width = style.caret_width;
        let has_fonts = ui.lay_out(session.edit.text(), font);
        let line = &ui.state.line;
        let width = line.width();
        let caret = caret_x(line, session.edit.caret());
        session.scroll = scroll_to(session.scroll, caret, width, area.width());
        let origin = self.origin(area, width, session.scroll);
        let (baseline, top, bottom) = if has_fonts {
            let baseline = line.centered_baseline(area.min.y, area.height());
            (
                baseline,
                baseline - line.ascent(),
                baseline + line.descent(),
            )
        } else {
            (area.center().y, area.min.y, area.max.y)
        };
        let selection = session.edit.selection();
        let selected = (!selection.is_empty())
            .then(|| (caret_x(line, selection.start), caret_x(line, selection.end)));
        let visible = Self::caret_visible(ui, session);
        // Scrolled text stays out of the padding; the caret shows at the area's edges.
        let clip = Rect::from_min_max(
            Vec2::new(area.min.x - caret_width, area.min.y),
            Vec2::new(area.max.x + caret_width, area.max.y),
        );
        let empty = session.edit.text().is_empty();
        ui.clipped(clip, |ui| {
            if let Some((start, end)) = selected {
                ui.scene().push_rect(&RectPrimitive::new(
                    Rect::from_min_max(
                        Vec2::new(origin + start, top),
                        Vec2::new(origin + end, bottom),
                    ),
                    selection_color,
                ));
            }
            if empty {
                // The placeholder stays where it is while idle.
                if let Some(text) = self.placeholder {
                    ui.label(text, area, font, placeholder, self.align);
                }
            } else {
                ui.scene
                    .push_text(&ui.state.line, Vec2::new(origin, baseline), text_color);
            }
            if visible {
                let x = (origin + caret - caret_width * 0.5).round();
                ui.scene().push_rect(&RectPrimitive::new(
                    Rect::from_min_max(Vec2::new(x, top), Vec2::new(x + caret_width, bottom)),
                    caret_color,
                ));
            }
        });
    }

    /// `true` when the caret shows this frame: always, or in the shown half of its blink,
    /// asking for the frame that turns it.
    fn caret_visible(ui: &mut Ui<'_>, session: &Session) -> bool {
        let blink = ui.styles().text_field.caret_blink;
        if blink < MIN_BLINK {
            return true;
        }
        let elapsed = ui.input().time().saturating_sub(session.since);
        let halves = elapsed.as_nanos() / blink.as_nanos();
        let next = session
            .since
            .saturating_add(blink.saturating_mul(u32::try_from(halves + 1).unwrap_or(u32::MAX)));
        ui.request_redraw_at(next);
        halves.is_multiple_of(2)
    }
}
