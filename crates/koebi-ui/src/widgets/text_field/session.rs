//! What text fields keep between frames, in [`UiState`](crate::UiState): the one field editing
//! text and its [`TextEdit`], the order fields were shown in (for Tab), the field asked to
//! edit next, the edits that ended away from their field, and the exchanges with the
//! clipboard.
//!
//! One field edits at a time. Its text changes as the user types; the session keeps the text
//! it started with, so that cancelling restores it. A session ends when its field confirms or
//! cancels, when another field starts editing, or when a frame no longer shows its field. An
//! edit that ended away from its field tells the field during the next frame, then is
//! forgotten; so is a request to edit a field that no frame showed since.

use core::time::Duration;

use super::edit::TextEdit;
use super::{EditEnd, OutsidePress};
use crate::id::WidgetId;

/// The edit of one field.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Session {
    /// The field.
    pub(crate) id: WidgetId,
    /// The text, caret, selection and undo steps.
    pub(crate) edit: TextEdit,
    /// The text when the edit started, restored on cancel.
    pub(crate) original: String,
    /// What a press outside the field does, as the field last said.
    pub(crate) outside: OutsidePress,
    /// How far the text scrolls left to keep the caret in view.
    pub(crate) scroll: f32,
    /// The primary button, pressed on the field, selects as the pointer moves.
    pub(crate) dragging: bool,
    /// The press that started the edit selected everything: dragging selects from the press
    /// only past the drag threshold.
    pub(crate) select_after_threshold: bool,
    /// When the caret last moved or the text changed: the caret blinks from there.
    pub(crate) since: Duration,
    /// The field was shown this frame.
    pub(crate) seen: bool,
}

impl Session {
    /// The edit of `text` by the field `id`, starting at `now`.
    pub(crate) fn new(id: WidgetId, text: &str, now: Duration, outside: OutsidePress) -> Self {
        Self {
            id,
            edit: TextEdit::new(text),
            original: text.to_owned(),
            outside,
            scroll: 0.0,
            dragging: false,
            select_after_threshold: false,
            since: now,
            seen: true,
        }
    }
}

/// An edit that ended away from its field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Ended {
    /// The field.
    pub(crate) id: WidgetId,
    /// How it ended.
    pub(crate) end: EditEnd,
    /// The text to put back, when the edit was cancelled.
    pub(crate) restore: Option<String>,
    /// The frame it ended in.
    frame: u64,
}

/// Why a session ends away from its field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Away {
    /// A press on another field: the field's [`OutsidePress`] applies.
    Pressed,
    /// Another field starts editing without a press (Tab, a request): the text stays.
    Replaced,
    /// The field was not shown: the text stays.
    Hidden,
}

/// The text fields' state between frames.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TextState {
    /// The field editing text.
    pub(crate) session: Option<Session>,
    /// Edits that ended away from their field, until the field's next frame.
    ended: Vec<Ended>,
    /// Enabled fields in the order the last frame showed them.
    pub(crate) order: Vec<WidgetId>,
    /// Enabled fields in the order this frame shows them.
    pub(crate) shown: Vec<WidgetId>,
    /// The field to edit when it is next shown (Tab, [`Ui::edit_text_field`]), and the frame
    /// that asked.
    ///
    /// [`Ui::edit_text_field`]: crate::Ui::edit_text_field
    focus: Option<(WidgetId, u64)>,
    /// Frames since the state was created.
    frame: u64,
    /// A field took this frame's typed text.
    pub(crate) typed_taken: bool,
    /// Text copied during the last frame, for the application's clipboard.
    pub(crate) copied: Option<String>,
    /// The last frame asked for the clipboard's text.
    pub(crate) paste_requested: bool,
    /// The clipboard's text, given by the application, until the editing field takes it.
    pub(crate) pasted: Option<String>,
}

impl TextState {
    /// A frame starts: what the last one asked of the application was answered.
    pub(crate) fn begin_frame(&mut self) {
        self.frame += 1;
        self.shown.clear();
        self.copied = None;
        self.paste_requested = false;
        self.typed_taken = false;
        if let Some(session) = &mut self.session {
            session.seen = false;
        }
    }

    /// A frame ends: a session whose field it did not show ends, keeping the text; notices
    /// and requests older than the frame are forgotten.
    pub(crate) fn end_frame(&mut self) {
        if self.session.as_ref().is_some_and(|session| !session.seen) {
            self.end_away(Away::Hidden);
        }
        // Pasted text nobody took is stale.
        self.pasted = None;
        let frame = self.frame;
        self.ended.retain(|ended| ended.frame >= frame);
        if self.focus.is_some_and(|(_, asked)| asked < frame) {
            self.focus = None;
        }
        core::mem::swap(&mut self.order, &mut self.shown);
    }

    /// Ends the session away from its field: the text, written as it changed, stays, unless
    /// a press on another field cancels it.
    pub(crate) fn end_away(&mut self, why: Away) {
        let Some(session) = self.session.take() else {
            return;
        };
        let cancel = why == Away::Pressed && session.outside == OutsidePress::Cancel;
        self.ended.push(Ended {
            id: session.id,
            end: if cancel {
                EditEnd::Cancelled
            } else {
                EditEnd::Confirmed
            },
            restore: cancel.then_some(session.original),
            frame: self.frame,
        });
    }

    /// Takes the notice of the edit of `id` that ended away from it, if there is one.
    pub(crate) fn take_ended(&mut self, id: WidgetId) -> Option<Ended> {
        let index = self.ended.iter().position(|ended| ended.id == id)?;
        Some(self.ended.swap_remove(index))
    }

    /// Asks the field `id` to edit when it is next shown.
    pub(crate) fn set_focus(&mut self, id: WidgetId) {
        self.focus = Some((id, self.frame));
    }

    /// `true` when the field `id` is asked to edit.
    pub(crate) fn is_focused(&self, id: WidgetId) -> bool {
        self.focus.is_some_and(|(focused, _)| focused == id)
    }

    /// Forgets the request to edit, if it is for `id`.
    pub(crate) fn clear_focus(&mut self, id: WidgetId) {
        if self.is_focused(id) {
            self.focus = None;
        }
    }

    /// The field after (`forward`) or before `id` in the last frame's order, wrapping around.
    pub(crate) fn neighbour(&self, id: WidgetId, forward: bool) -> Option<WidgetId> {
        let index = self.order.iter().position(|field| *field == id)?;
        let count = self.order.len();
        let next = if forward {
            (index + 1) % count
        } else {
            (index + count - 1) % count
        };
        (next != index).then(|| self.order[next])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: WidgetId = WidgetId::new("a");
    const B: WidgetId = WidgetId::new("b");
    const C: WidgetId = WidgetId::new("c");

    fn editing(id: WidgetId, outside: OutsidePress) -> TextState {
        let mut state = TextState::default();
        state.begin_frame();
        state.session = Some(Session::new(id, "Cube", Duration::ZERO, outside));
        state
    }

    #[test]
    fn tab_order_wraps_around_the_fields_of_the_last_frame() {
        let mut state = TextState {
            order: vec![A, B, C],
            ..TextState::default()
        };
        assert_eq!(state.neighbour(A, true), Some(B));
        assert_eq!(state.neighbour(C, true), Some(A));
        assert_eq!(state.neighbour(A, false), Some(C));
        state.order = vec![A];
        assert_eq!(state.neighbour(A, true), None);
        assert_eq!(state.neighbour(B, true), None);
    }

    #[test]
    fn a_session_whose_field_is_not_shown_ends_and_its_notice_lasts_one_frame() {
        let mut state = editing(A, OutsidePress::Confirm);
        if let Some(session) = &mut state.session {
            session.seen = false;
        }
        state.end_frame();
        assert_eq!(state.session, None);
        // The next frame can tell the field.
        state.begin_frame();
        assert!(state.ended.iter().any(|ended| ended.id == A));
        state.end_frame();
        // After it, the notice is gone.
        assert_eq!(state.take_ended(A), None);
    }

    #[test]
    fn notices_of_several_fields_are_kept_apart() {
        let mut state = editing(A, OutsidePress::Cancel);
        state.end_away(Away::Pressed);
        state.session = Some(Session::new(
            B,
            "Sphere",
            Duration::ZERO,
            OutsidePress::Cancel,
        ));
        state.end_away(Away::Replaced);
        let a = state.take_ended(A).expect("A ended");
        assert_eq!(
            (a.end, a.restore.as_deref()),
            (EditEnd::Cancelled, Some("Cube"))
        );
        let b = state.take_ended(B).expect("B ended");
        assert_eq!((b.end, b.restore), (EditEnd::Confirmed, None));
    }

    #[test]
    fn requests_to_edit_expire_after_the_next_frame() {
        let mut state = TextState::default();
        state.begin_frame();
        state.set_focus(A);
        state.end_frame();
        state.begin_frame();
        assert!(state.is_focused(A));
        state.end_frame();
        assert!(!state.is_focused(A));
    }
}
