//! What a field does with a frame's input while it edits: a press outside ends the edit; the
//! pointer places the caret and selects; keys and typed text apply in the order they came;
//! pasted text arrives; the text is written back as it changes.
//!
//! Typed text goes to one field a frame: when Tab starts the next field in the same frame,
//! what was typed stays in the field that ended. Every action the field uses is consumed, so
//! that the components after it (the next field, a popup around it) do not see it again.

use koebi_input::ActionDef;

use super::actions::{
    TEXT_CANCEL, TEXT_CONFIRM, TEXT_COPY, TEXT_CUT, TEXT_DELETE_BACKWARD, TEXT_DELETE_FORWARD,
    TEXT_DELETE_WORD_BACKWARD, TEXT_DELETE_WORD_FORWARD, TEXT_END, TEXT_LEFT, TEXT_NEXT_FIELD,
    TEXT_PASTE, TEXT_PREVIOUS_FIELD, TEXT_REDO, TEXT_RIGHT, TEXT_SELECT_ALL, TEXT_SELECT_END,
    TEXT_SELECT_LEFT, TEXT_SELECT_RIGHT, TEXT_SELECT_START, TEXT_SELECT_WORD_LEFT,
    TEXT_SELECT_WORD_RIGHT, TEXT_START, TEXT_UNDO, TEXT_WORD_LEFT, TEXT_WORD_RIGHT,
};
use super::edit::{Select, Step};
use super::session::Session;
use super::{EditEnd, OutsidePress, TEXT, TextField, TextFieldResponse};
use crate::input::{MAX_ACTIONS, PointerButton};
use crate::ui::Ui;
use koebi_core::Rect;

/// How a key ends the edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ending {
    Confirm,
    Cancel,
    /// Keeps the text and edits the next field (`true`) or the previous one.
    Next(bool),
}

/// What a key did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyEffect {
    /// Not a key of text fields.
    Ignored,
    /// Applied; `true` when the text changed.
    Applied(bool),
    /// Ends the edit.
    Ends(Ending),
}

/// Caret moves: the action, forward or not, the step, and whether the selection follows.
const MOVES: [(&ActionDef, bool, Step, Select); 12] = [
    (&TEXT_LEFT, false, Step::Character, Select::Collapse),
    (&TEXT_RIGHT, true, Step::Character, Select::Collapse),
    (&TEXT_WORD_LEFT, false, Step::Word, Select::Collapse),
    (&TEXT_WORD_RIGHT, true, Step::Word, Select::Collapse),
    (&TEXT_START, false, Step::Line, Select::Collapse),
    (&TEXT_END, true, Step::Line, Select::Collapse),
    (&TEXT_SELECT_LEFT, false, Step::Character, Select::Extend),
    (&TEXT_SELECT_RIGHT, true, Step::Character, Select::Extend),
    (&TEXT_SELECT_WORD_LEFT, false, Step::Word, Select::Extend),
    (&TEXT_SELECT_WORD_RIGHT, true, Step::Word, Select::Extend),
    (&TEXT_SELECT_START, false, Step::Line, Select::Extend),
    (&TEXT_SELECT_END, true, Step::Line, Select::Extend),
];

/// Deletions: the action, forward or not, and the step.
const DELETES: [(&ActionDef, bool, Step); 4] = [
    (&TEXT_DELETE_BACKWARD, false, Step::Character),
    (&TEXT_DELETE_FORWARD, true, Step::Character),
    (&TEXT_DELETE_WORD_BACKWARD, false, Step::Word),
    (&TEXT_DELETE_WORD_FORWARD, true, Step::Word),
];

impl TextField<'_> {
    /// Applies this frame's input to the edit, and writes the text back when it changed.
    /// Returns `false` when the edit ended.
    pub(super) fn edit(
        &mut self,
        ui: &mut Ui<'_>,
        rect: Rect,
        session: &mut Session,
        out: &mut TextFieldResponse,
    ) -> bool {
        ui.grab_keyboard(TEXT);
        ui.text_input(rect);
        session.outside = self.outside_press;
        // The application changed the text during the edit (an undo of the document): the
        // edit continues from it, and can undo the change.
        if session.edit.text() != self.text.as_str() {
            session.edit.set_text(self.text);
        }
        let before = (session.edit.caret(), session.edit.selection());
        let outside = [
            PointerButton::Primary,
            PointerButton::Secondary,
            PointerButton::Middle,
        ]
        .into_iter()
        .filter_map(|button| ui.input().button_pressed_at(button))
        .any(|at| !rect.contains(at));
        if outside {
            match self.outside_press {
                OutsidePress::Confirm => out.ended = Some(EditEnd::Confirmed),
                OutsidePress::Cancel => self.cancel(session, out),
            }
            return false;
        }
        self.point(ui, rect, session, out);
        let mut changed = self.paste(ui, session);
        let (keys_changed, ending) = self.keys_and_text(ui, session);
        changed |= keys_changed;
        if changed {
            self.text.clear();
            self.text.push_str(session.edit.text());
            out.changed = true;
            out.response.changed = true;
        }
        if changed || before != (session.edit.caret(), session.edit.selection()) {
            session.since = ui.input().time();
        }
        match ending {
            None => true,
            Some(Ending::Cancel) => {
                self.cancel(session, out);
                false
            }
            Some(Ending::Confirm) => {
                out.ended = Some(EditEnd::Confirmed);
                false
            }
            Some(Ending::Next(forward)) => {
                let texts = &mut ui.state.text;
                if let Some(next) = texts.neighbour(self.id, forward) {
                    texts.set_focus(next);
                }
                out.ended = Some(EditEnd::Confirmed);
                false
            }
        }
    }

    /// The pointer: a press places the caret or selects a word, a drag selects.
    fn point(&self, ui: &mut Ui<'_>, rect: Rect, session: &mut Session, out: &TextFieldResponse) {
        let response = &out.response;
        // Where the press happened, not where the pointer went since.
        if response.pressed
            && !out.started
            && let Some(press) = ui.input().pressed_at()
        {
            let at = self.offset_at(ui, rect, session, press.x);
            if response.double_clicked {
                session.edit.select_word(at);
                session.dragging = false;
            } else {
                let select = if ui.input().modifiers().shift() {
                    Select::Extend
                } else {
                    Select::Collapse
                };
                session.edit.place(at, select);
                session.dragging = true;
            }
            session.select_after_threshold = false;
        }
        if !response.held {
            session.dragging = false;
            return;
        }
        let (true, Some(pointer)) = (session.dragging, ui.input().pointer()) else {
            return;
        };
        if session.select_after_threshold {
            if response.drag_delta.length() < ui.styles().input.drag_threshold {
                return;
            }
            // A drag from the press that selected everything selects from the press.
            session.select_after_threshold = false;
            let press = pointer - response.drag_delta;
            let start = self.offset_at(ui, rect, session, press.x);
            session.edit.place(start, Select::Collapse);
        }
        let at = self.offset_at(ui, rect, session, pointer.x);
        session.edit.place(at, Select::Extend);
    }

    /// The keys and the typed text of this frame, in the order they came. Returns whether the
    /// text changed, and how a key ended the edit; what came after that key is dropped.
    fn keys_and_text(&self, ui: &mut Ui<'_>, session: &mut Session) -> (bool, Option<Ending>) {
        let input = ui.input();
        let typed = input.text();
        let take_text = !core::mem::replace(&mut ui.state.text.typed_taken, true);
        let mut done = 0;
        let mut changed = false;
        let mut ending = None;
        let mut used = [None; MAX_ACTIONS];
        for (slot, (action, at)) in used.iter_mut().zip(input.actions_in_text()) {
            if take_text {
                changed |= self.type_text(session, typed.get(done..at).unwrap_or_default());
                done = done.max(at);
            }
            if !ui.triggered(action) {
                continue;
            }
            match Self::key(ui, session, action) {
                KeyEffect::Ignored => continue,
                KeyEffect::Applied(text) => changed |= text,
                KeyEffect::Ends(end) => ending = Some(end),
            }
            *slot = Some(action);
            if ending.is_some() {
                break;
            }
        }
        if take_text && ending.is_none() {
            changed |= self.type_text(session, typed.get(done..).unwrap_or_default());
        }
        // Used here, repeats included: the components after the field do not see them.
        for action in used.into_iter().flatten() {
            ui.consume(action);
        }
        (changed, ending)
    }

    /// What `action` does to the edit.
    fn key(ui: &mut Ui<'_>, session: &mut Session, action: &ActionDef) -> KeyEffect {
        let id = action.id;
        let edit = &mut session.edit;
        if let Some(&(_, forward, step, select)) = MOVES.iter().find(|(a, ..)| a.id == id) {
            edit.step(forward, step, select);
            return KeyEffect::Applied(false);
        }
        if let Some(&(_, forward, step)) = DELETES.iter().find(|(a, ..)| a.id == id) {
            return KeyEffect::Applied(edit.delete(forward, step));
        }
        if id == TEXT_SELECT_ALL.id {
            edit.select_all();
            KeyEffect::Applied(false)
        } else if id == TEXT_COPY.id || id == TEXT_CUT.id {
            let mut changed = false;
            if !edit.selected().is_empty() {
                ui.copy_text(edit.selected());
                if id == TEXT_CUT.id {
                    changed = edit.delete(false, Step::Character);
                }
            }
            KeyEffect::Applied(changed)
        } else if id == TEXT_PASTE.id {
            ui.request_paste();
            KeyEffect::Applied(false)
        } else if id == TEXT_UNDO.id {
            KeyEffect::Applied(edit.undo())
        } else if id == TEXT_REDO.id {
            KeyEffect::Applied(edit.redo())
        } else if id == TEXT_CONFIRM.id {
            KeyEffect::Ends(Ending::Confirm)
        } else if id == TEXT_CANCEL.id {
            KeyEffect::Ends(Ending::Cancel)
        } else if id == TEXT_NEXT_FIELD.id {
            KeyEffect::Ends(Ending::Next(true))
        } else if id == TEXT_PREVIOUS_FIELD.id {
            KeyEffect::Ends(Ending::Next(false))
        } else {
            KeyEffect::Ignored
        }
    }

    /// Inserts `typed`, within the filter and the length limit. Returns `true` when the text
    /// changed.
    fn type_text(&self, session: &mut Session, typed: &str) -> bool {
        let mut changed = false;
        for c in typed.chars() {
            // Tab and line breaks come from keys the field handles as actions.
            if matches!(c, '\t' | '\n' | '\r') || !self.accepts(c) || self.room(session) == 0 {
                continue;
            }
            let mut bytes = [0; 4];
            changed |= session.edit.insert(c.encode_utf8(&mut bytes));
        }
        changed
    }

    /// The clipboard's text, when it arrived, within the filter and the length limit: one
    /// undo step. Nothing left after the filter changes nothing. Returns `true` when the text
    /// changed.
    fn paste(&self, ui: &mut Ui<'_>, session: &mut Session) -> bool {
        let Some(pasted) = ui.state.text.pasted.take() else {
            return false;
        };
        let room = self.room(session);
        let kept: String = pasted
            .chars()
            .map(|c| {
                if matches!(c, '\n' | '\r' | '\t') {
                    ' '
                } else {
                    c
                }
            })
            .filter(|c| self.accepts(*c))
            .take(room)
            .collect();
        !kept.is_empty() && session.edit.paste(&kept)
    }

    /// `true` when the filter keeps `c`.
    fn accepts(&self, c: char) -> bool {
        !c.is_control() && self.filter.is_none_or(|accepts| accepts(c))
    }

    /// How many characters can still be inserted, replacing the selection.
    fn room(&self, session: &Session) -> usize {
        let Some(limit) = self.max_chars else {
            return usize::MAX;
        };
        let kept = session.edit.text().chars().count() - session.edit.selected().chars().count();
        limit.saturating_sub(kept)
    }

    /// Ends the edit restoring the text it started with.
    fn cancel(&mut self, session: &Session, out: &mut TextFieldResponse) {
        if *self.text != session.original {
            self.text.clone_from(&session.original);
            out.changed = true;
            out.response.changed = true;
        }
        out.ended = Some(EditEnd::Cancelled);
    }
}
