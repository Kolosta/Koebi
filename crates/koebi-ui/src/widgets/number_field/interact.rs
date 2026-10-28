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
use crate::widgets::text_field::{EditEnd, SelectOnFocus, TEXT_CONFIRM, TextField};

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
        let track = self.track(ui, rect);
        let scale = self.scale(ui, track);
        let mut step_responses = [Response::default(); 2];
        if let Some(areas) = steps {
            for (index, area) in areas.into_iter().enumerate() {
                step_responses[index] = ui.interact(id.with(index as u64), area);
            }
        }
        if self.enabled {
            self.drag(ui, body, track, &scale, &mut out);
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
        self.draw(
            ui,
            rect,
            body,
            state,
            steps.is_some(),
            shown_steps,
            &step_responses,
        );
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
    fn drag(
        &mut self,
        ui: &mut Ui<'_>,
        body: Rect,
        track: Rect,
        scale: &Scale,
        out: &mut NumberFieldResponse,
    ) {
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
        if !out.response.pressed {
            let motion = ui.input().motion();
            drag.moved += motion;
            drag.travel += if self.vertical { -motion.y } else { motion.x };
        }
        let threshold = ui.styles().input.drag_threshold;
        if !drag.dragging && drag.moved.length() >= threshold {
            drag.dragging = true;
        }
        let right_click = ui
            .input()
            .button_pressed_at(PointerButton::Secondary)
            .is_some();
        if ui.triggered(&NUMBER_CANCEL) || right_click {
            ui.consume(&NUMBER_CANCEL);
            self.set(drag.state.start, out);
            out.ended = Some(EditEnd::Cancelled);
            // The right click cancels; it does not also ask for a context menu.
            out.response.secondary_pressed = false;
            Self::release(ui, drag.press);
            return;
        }
        if drag.dragging {
            let precision = Precision::of(ui.input().modifiers());
            let value = drag.state.value(drag.travel, precision, scale);
            self.set(value, out);
            out.dragging = true;
        }
        let release_at = self.release_point(track, drag.press);
        let released = !out.response.held || out.response.released;
        if ui.triggered(&NUMBER_CONFIRM) || (released && drag.dragging) {
            ui.consume(&NUMBER_CONFIRM);
            out.ended = Some(EditEnd::Confirmed);
            Self::release(ui, release_at);
            return;
        }
        if released {
            // A press without a drag: the value is typed.
            self.start_edit(ui);
            Self::release(ui, drag.press);
            return;
        }
        // The keys of the drag, as long as it lasts.
        ui.grab_keyboard(NUMBER);
        ui.lock_pointer(release_at);
        ui.state.number.drag = Some(drag);
    }

    /// Ends a drag: the pointer comes back at `at`, also when the application draws the frame
    /// again with settled input.
    fn release(ui: &mut Ui<'_>, at: Vec2) {
        ui.lock_pointer(at);
        let time = ui.input().time();
        ui.state.number.release = Some((at, time));
    }

    /// Starts typing the value, after the edit of another field if one is still open.
    fn start_edit(&self, ui: &mut Ui<'_>) {
        let text = ValueText::typed(self.value.to_f64(), self.decimals_shown(), self.unit)
            .as_str()
            .to_owned();
        let edit = NumberEdit {
            id: self.id,
            text,
            error: false,
            starting: true,
            seen: true,
        };
        let number = &mut ui.state.number;
        if number.edit.as_ref().is_some_and(|open| open.id != self.id) {
            // The other field sees the press that ends its edit, then this one starts.
            number.pending = Some(edit);
        } else {
            number.edit = Some(edit);
        }
    }

    /// Where the pointer comes back when the drag ends: at the end of a bounded field's bar
    /// on `track`, else where the press was.
    fn release_point(&self, track: Rect, press: Vec2) -> Vec2 {
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
            let y = track.max.y - fraction * track.height();
            Vec2::new(track.center().x, inside(track.min.y, track.max.y, y))
        } else {
            let x = track.min.x + fraction * track.width();
            Vec2::new(inside(track.min.x, track.max.x, x), track.center().y)
        }
    }

    /// The value `text` gives, if it is a valid expression in the field's unit.
    fn evaluate(&self, text: &str) -> Option<f64> {
        let context = Context {
            unit: self.unit,
            ..Context::default()
        };
        expr::evaluate(text, &context)
            .ok()
            .filter(|value| value.is_finite())
    }

    /// Where the value is typed: over the field, or across a vertical field in a box one
    /// control high and wide enough.
    fn edit_rect(&self, ui: &Ui<'_>, rect: Rect) -> Rect {
        if !self.vertical {
            return rect;
        }
        let style = &ui.styles().number_field;
        let size = Vec2::new(rect.width().max(style.min_width), style.height);
        Rect::from_center_size(rect.center(), size)
    }

    /// Shows the value being typed: a text field over the number field. Keeping the edit
    /// evaluates the expression; Enter on an invalid one keeps the edit open, as an error,
    /// with the text and its caret as they are.
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
        // Enter on an invalid expression: the edit stays open.
        if ui.triggered(&TEXT_CONFIRM) && self.evaluate(&edit.text).is_none() {
            ui.consume(&TEXT_CONFIRM);
            edit.error = true;
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
        let typed = field.show(ui, self.edit_rect(ui, rect));
        let mut out = NumberFieldResponse {
            response: typed.response,
            editing: true,
            ..NumberFieldResponse::default()
        };
        out.response.changed = false;
        match typed.ended {
            Some(EditEnd::Confirmed) => {
                if let Some(value) = self.evaluate(&edit.text) {
                    self.set(value, &mut out);
                    out.ended = Some(EditEnd::Confirmed);
                } else {
                    // Ended away (a press outside, Tab) on an invalid expression: the value
                    // stays.
                    out.ended = Some(EditEnd::Cancelled);
                }
                out.editing = false;
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
