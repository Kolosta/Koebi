//! What a number field does with a frame's input: a press starts a drag with the pointer
//! locked; travel past the threshold changes the value; a release before it types the value;
//! the step buttons step; a typed expression is evaluated when the edit is kept.

use koebi_core::expr::{self, Context};
use koebi_core::input::CursorShape;
use koebi_core::{Rect, Vec2};

use super::format::ValueText;
use super::math::{DragState, Precision, Scale, stepped};
use super::state::{NumberDrag, NumberEdit};
use super::{
    EDIT_KEY, NUMBER, NUMBER_CANCEL, NUMBER_CONFIRM, NumberField, NumberFieldResponse, Numeric,
};
use crate::input::PointerButton;
use crate::text::Align;
use crate::ui::{Response, Ui};
use crate::widgets::parts::ControlState;
use crate::widgets::text_field::{EditEnd, SelectOnFocus, TextField};

impl<N: Numeric> NumberField<'_, N> {
    /// Shows the value: dragged, stepped, or starting to be typed.
    pub(super) fn show_value(mut self, ui: &mut Ui<'_>, rect: Rect) -> NumberFieldResponse {
        let id = self.id;
        let (body, steps) = self.parts(ui, rect);
        let cursor = if self.vertical {
            CursorShape::ResizeVertical
        } else {
            CursorShape::ResizeHorizontal
        };
        let response = if self.enabled {
            ui.interact_with_cursor(id, body, cursor)
        } else {
            ui.interact(id, body)
        };
        let mut out = NumberFieldResponse {
            response,
            ..NumberFieldResponse::default()
        };
        let scale = self.scale(ui, body);
        let mut step_responses = [Response::default(); 2];
        if let Some(areas) = steps {
            for (index, area) in areas.into_iter().enumerate() {
                step_responses[index] = ui.interact(id.with(index as u64), area);
            }
        }
        if self.enabled {
            self.drag(ui, body, &scale, &mut out);
            for (response, direction) in step_responses.iter().zip([-1.0, 1.0]) {
                if response.clicked {
                    let precision = Precision::of(ui.input().modifiers());
                    let next = stepped(self.value.to_f64(), direction, precision, &scale);
                    self.set(next, &mut out);
                }
            }
        } else if ui.state.number.drag.is_some_and(|drag| drag.id == id) {
            ui.state.number.drag = None;
        }
        let hovered = out.response.hovered
            || out.response.held
            || step_responses.iter().any(|response| response.hovered);
        let state = if !self.enabled {
            ControlState::Disabled
        } else if out.dragging {
            ControlState::Selected
        } else if out.response.held {
            ControlState::Pressed
        } else if hovered {
            ControlState::Hovered
        } else {
            ControlState::Normal
        };
        let shown_steps = steps.filter(|_| hovered && self.enabled && !out.dragging);
        self.draw(ui, rect, body, state, shown_steps, &step_responses);
        // The whole field's interaction: the body's and the step buttons'.
        for part in step_responses {
            out.response = out.response.merge(part);
        }
        out
    }

    /// Writes `value`, within the bounds, and says whether it changed.
    fn set(&mut self, value: f64, out: &mut NumberFieldResponse) {
        let new = self.bounded(value);
        if new != *self.value {
            *self.value = new;
            out.changed = true;
            out.response.changed = true;
        }
    }

    /// The drag: started by a press, the pointer locked; travel past the threshold changes
    /// the value; Escape, Delete or a right click cancel; Enter or the release keep the value;
    /// a release before the threshold types the value.
    fn drag(&mut self, ui: &mut Ui<'_>, body: Rect, scale: &Scale, out: &mut NumberFieldResponse) {
        let id = self.id;
        if out.response.pressed {
            let press = ui.input().pressed_at().unwrap_or_else(|| body.center());
            ui.state.number.drag = Some(NumberDrag {
                id,
                state: DragState::new(self.value.to_f64(), scale),
                travel: 0.0,
                moved: Vec2::ZERO,
                dragging: false,
                press,
                seen: true,
            });
        }
        let Some(mut drag) = ui.state.number.drag.take_if(|drag| drag.id == id) else {
            return;
        };
        drag.seen = true;
        ui.grab_keyboard(NUMBER);
        if !out.response.pressed {
            let motion = ui.input().motion();
            drag.moved += motion;
            drag.travel += if self.vertical { -motion.y } else { motion.x };
        }
        let threshold = ui.styles().input.drag_threshold;
        if !drag.dragging && drag.moved.length() >= threshold {
            drag.dragging = true;
        }
        let cancel = ui.triggered(&NUMBER_CANCEL)
            || ui
                .input()
                .button_pressed_at(PointerButton::Secondary)
                .is_some();
        if cancel {
            ui.consume(&NUMBER_CANCEL);
            self.set(drag.state.start, out);
            out.ended = Some(EditEnd::Cancelled);
            // One more frame locked, so that the pointer comes back where the press was.
            ui.lock_pointer(drag.press);
            return;
        }
        if drag.dragging {
            let precision = Precision::of(ui.input().modifiers());
            let value = drag.state.value(drag.travel, precision, scale);
            self.set(value, out);
            out.dragging = true;
        }
        let release_at = self.release_point(body, drag.press);
        let released = !out.response.held || out.response.released;
        if ui.triggered(&NUMBER_CONFIRM) || (released && drag.dragging) {
            ui.consume(&NUMBER_CONFIRM);
            out.ended = Some(EditEnd::Confirmed);
            ui.lock_pointer(release_at);
            return;
        }
        if released {
            // A press without a drag: the value is typed.
            let text = ValueText::typed(self.value.to_f64(), self.decimals_shown(), self.unit)
                .as_str()
                .to_owned();
            ui.state.number.edit = Some(NumberEdit {
                id,
                text,
                error: false,
                starting: true,
                seen: true,
            });
            ui.lock_pointer(drag.press);
            return;
        }
        ui.lock_pointer(release_at);
        ui.state.number.drag = Some(drag);
    }

    /// Where the pointer comes back when the drag ends: at the end of a bounded field's bar,
    /// else where the press was.
    fn release_point(&self, body: Rect, press: Vec2) -> Vec2 {
        let Some(mapping) = self.shown_bar() else {
            return press;
        };
        #[allow(
            clippy::cast_possible_truncation,
            reason = "a fraction from 0 to 1 fits f32"
        )]
        let fraction = mapping.fraction(self.value.to_f64(), self.min, self.max) as f32;
        let inside = |low: f32, high: f32, at: f32| at.max(low + 1.0).min(high - 1.0);
        if self.vertical {
            let y = body.max.y - fraction * body.height();
            Vec2::new(body.center().x, inside(body.min.y, body.max.y, y))
        } else {
            let x = body.min.x + fraction * body.width();
            Vec2::new(inside(body.min.x, body.max.x, x), body.center().y)
        }
    }

    /// Shows the value being typed: a text field over the number field; keeping the edit
    /// evaluates the expression, an invalid one keeps the edit open as an error.
    pub(super) fn show_edit(mut self, ui: &mut Ui<'_>, rect: Rect) -> NumberFieldResponse {
        let Some(mut edit) = ui.state.number.edit.take() else {
            return NumberFieldResponse::default();
        };
        edit.seen = true;
        let field_id = self.id.with(EDIT_KEY);
        if edit.starting {
            ui.edit_text_field(field_id);
            edit.starting = false;
        }
        let error = ui.styles().number_field.error;
        let mut field = TextField::new(field_id, &mut edit.text)
            .align(Align::Center)
            .select_on_focus(SelectOnFocus::All);
        if edit.error {
            field = field
                .border_color(ControlState::Normal, error)
                .border_color(ControlState::Selected, error);
        }
        let typed = field.show(ui, rect);
        let mut out = NumberFieldResponse {
            response: typed.response,
            editing: true,
            ..NumberFieldResponse::default()
        };
        out.response.changed = false;
        match typed.ended {
            Some(EditEnd::Confirmed) => {
                let context = Context {
                    unit: self.unit,
                    ..Context::default()
                };
                match expr::evaluate(&edit.text, &context) {
                    Ok(value) if value.is_finite() => {
                        self.set(value, &mut out);
                        out.ended = Some(EditEnd::Confirmed);
                        out.editing = false;
                    }
                    _ => {
                        let outside = ui.input().pressed_at().is_some_and(|at| !rect.contains(at));
                        if outside {
                            // Pressed away: the value stays as it was.
                            out.ended = Some(EditEnd::Cancelled);
                            out.editing = false;
                        } else {
                            edit.error = true;
                            edit.starting = true;
                            ui.state.number.edit = Some(edit);
                        }
                    }
                }
            }
            Some(EditEnd::Cancelled) => {
                out.ended = Some(EditEnd::Cancelled);
                out.editing = false;
            }
            None if typed.editing || typed.started => ui.state.number.edit = Some(edit),
            None => out.editing = false,
        }
        out
    }
}
