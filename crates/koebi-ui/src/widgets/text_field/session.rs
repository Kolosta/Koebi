//! What text fields keep between frames, in [`UiState`](crate::UiState): the one field editing
//! text and its [`TextEdit`], the order fields were shown in (for Tab), the field asked to
//! edit next, and the exchanges with the clipboard.
//!
//! One field edits at a time. Its text changes as the user types; the session keeps the text
//! it started with, so that cancelling restores it. A session ends when its field confirms or
//! cancels, when another field starts editing, or when a frame no longer shows its field; a
//! session that ended away from its field tells the field on its next frame.

use core::time::Duration;

use super::edit::TextEdit;
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

/// How an edit ended away from its field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Ended {
    /// The field.
    pub(crate) id: WidgetId,
}

/// The text fields' state between frames.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TextState {
    /// The field editing text.
    pub(crate) session: Option<Session>,
    /// A field whose edit ended (kept) while it was not drawing: its next frame says so.
    pub(crate) ended: Option<Ended>,
    /// Fields in the order the last frame showed them.
    pub(crate) order: Vec<WidgetId>,
    /// Fields in the order this frame shows them.
    pub(crate) shown: Vec<WidgetId>,
    /// The field to edit when it is next shown (Tab, [`Ui::edit_text_field`]).
    ///
    /// [`Ui::edit_text_field`]: crate::Ui::edit_text_field
    pub(crate) focus: Option<WidgetId>,
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
        self.shown.clear();
        self.copied = None;
        self.paste_requested = false;
        if let Some(session) = &mut self.session {
            session.seen = false;
        }
    }

    /// A frame ends: a session whose field it did not show ends, keeping the text.
    pub(crate) fn end_frame(&mut self) {
        if self.session.as_ref().is_some_and(|session| !session.seen) {
            self.end_away();
        }
        // Pasted text nobody took is stale.
        self.pasted = None;
        core::mem::swap(&mut self.order, &mut self.shown);
    }

    /// Ends the session away from its field, keeping its text (written as it changed).
    pub(crate) fn end_away(&mut self) {
        if let Some(session) = self.session.take() {
            self.ended = Some(Ended { id: session.id });
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
    fn a_session_whose_field_is_not_shown_ends() {
        let mut state = TextState {
            session: Some(Session {
                id: A,
                edit: TextEdit::new("Cube"),
                original: "Cube".to_owned(),
                scroll: 0.0,
                dragging: false,
                select_after_threshold: false,
                since: Duration::ZERO,
                seen: false,
            }),
            ..TextState::default()
        };
        state.begin_frame();
        state.end_frame();
        assert_eq!(state.session, None);
        assert_eq!(state.ended, Some(Ended { id: A }));
    }
}
