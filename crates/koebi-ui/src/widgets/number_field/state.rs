//! What number fields keep between frames, in [`UiState`](crate::UiState): the one field being
//! dragged and its [`DragState`], the one field whose value is typed as text, and where the
//! pointer comes back when a drag ends. A drag or an edit whose field a frame no longer shows
//! ends.

use core::time::Duration;

use koebi_core::Vec2;

use super::math::DragState;
use crate::id::WidgetId;

/// The drag of one field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NumberDrag {
    /// The field.
    pub(crate) id: WidgetId,
    /// The values the travel gives.
    pub(crate) state: DragState,
    /// Pointer travel along the field since the press, in logical pixels.
    pub(crate) travel: f32,
    /// Pointer travel in both directions, for the drag threshold.
    pub(crate) moved: Vec2,
    /// The travel went past the drag threshold: releasing keeps the value, not typing.
    pub(crate) dragging: bool,
    /// Where the press was: where the pointer comes back.
    pub(crate) press: Vec2,
    /// The field was shown this frame.
    pub(crate) seen: bool,
}

/// The value of one field typed as text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NumberEdit {
    /// The field.
    pub(crate) id: WidgetId,
    /// The text being typed.
    pub(crate) text: String,
    /// The last confirmed text was not a valid expression.
    pub(crate) error: bool,
    /// The edit started this frame: its text field is asked to edit.
    pub(crate) starting: bool,
    /// The field was shown this frame.
    pub(crate) seen: bool,
}

/// The number fields' state between frames.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct NumberState {
    /// The field being dragged.
    pub(crate) drag: Option<NumberDrag>,
    /// The field whose value is typed.
    pub(crate) edit: Option<NumberEdit>,
    /// An edit started while another field's edit was still open: it takes over when the
    /// frame ends, after the other field has seen the press that ends its edit.
    pub(crate) pending: Option<NumberEdit>,
    /// Where the pointer comes back after the drag that ended at the given input time: every
    /// pass drawn with that input asks for it, so that a frame drawn again with settled input
    /// does not lose it.
    pub(crate) release: Option<(Vec2, Duration)>,
}

impl NumberState {
    /// A frame starts.
    pub(crate) fn begin_frame(&mut self) {
        if let Some(drag) = &mut self.drag {
            drag.seen = false;
        }
        if let Some(edit) = &mut self.edit {
            edit.seen = false;
        }
    }

    /// A frame ends: a drag or an edit whose field was not shown ends.
    pub(crate) fn end_frame(&mut self) {
        if self.drag.is_some_and(|drag| !drag.seen) {
            self.drag = None;
        }
        if self.edit.as_ref().is_some_and(|edit| !edit.seen) {
            self.edit = None;
        }
        if let Some(pending) = self.pending.take() {
            self.edit = Some(pending);
        }
    }
}
