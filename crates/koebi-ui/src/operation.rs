//! The operation in progress: a drag (a number field's value, a color's mark, a curve's key, rows
//! of a list), a resize, split or join of the workspace, a pick of the eyedropper. Every one
//! shares the same keys, which apply wherever the pointer is while it lasts: its cancel (Escape,
//! Delete, the secondary button) and its confirm (Enter). They are the keymap's: what the user
//! binds to them applies to every operation.
//!
//! A component keeps its operation going with [`Ui::operation`] every frame it lasts, and reads
//! from it how the user ended it.

use koebi_core::input::{Key, MouseButton};
use koebi_i18n::n_;
use koebi_input::{ActionDef, ActionGroup, ActionKind, ButtonTrigger, Context, Hint, Shortcut};

use crate::ui::Ui;

/// An operation in progress: its keys apply wherever the pointer is, and nothing under the
/// pointer receives a shortcut meanwhile.
pub const OPERATION: Context = Context::new("operation");

/// Cancels the operation in progress, bringing back what it changed.
pub static OPERATION_CANCEL: ActionDef = ActionDef {
    id: "operation.cancel",
    label: n_!("Cancel"),
    description: "Cancels the operation in progress (a drag, a resize, a pick) and brings back \
        what it changed.",
    context: OPERATION,
    defaults: &[
        Shortcut::key(Key::Escape),
        Shortcut::key(Key::Delete),
        Shortcut::button(MouseButton::Right, ButtonTrigger::Press),
    ],
    repeats: false,
    when: None,
    hint: Some(Hint::new(ActionKind::Confirm, 8)),
};

/// Ends the operation in progress, keeping what it does.
pub static OPERATION_CONFIRM: ActionDef = ActionDef {
    id: "operation.confirm",
    label: n_!("Confirm"),
    description: "Ends the operation in progress and keeps what it does: the value dragged, the \
        split or join previewed, the color under the pointer.",
    context: OPERATION,
    defaults: &[Shortcut::key(Key::Enter), Shortcut::key(Key::KeypadEnter)],
    repeats: false,
    when: None,
    hint: Some(Hint::new(ActionKind::Confirm, 9)),
};

/// The actions of operations, for the keymap.
pub static OPERATION_ACTIONS: ActionGroup = ActionGroup {
    name: n_!("Operations"),
    actions: &[&OPERATION_CANCEL, &OPERATION_CONFIRM],
};

/// How the user ended an operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationEnd {
    /// Its cancel: what it changed comes back.
    Cancelled,
    /// Its confirm: what it does stays.
    Confirmed,
}

impl Ui<'_> {
    /// Keeps an operation going: until the next frame, the operation's keys apply wherever the
    /// pointer is, and nothing under the pointer receives a shortcut. Returns how the user ended
    /// it this frame, if they did, its action consumed; the operation then takes nothing more.
    /// Call it every frame the operation lasts.
    pub fn operation(&mut self) -> Option<OperationEnd> {
        if self.triggered(&OPERATION_CANCEL) {
            self.consume(&OPERATION_CANCEL);
            return Some(OperationEnd::Cancelled);
        }
        if self.triggered(&OPERATION_CONFIRM) {
            self.consume(&OPERATION_CONFIRM);
            return Some(OperationEnd::Confirmed);
        }
        self.grab(OPERATION);
        None
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::{Rect, Vec2};
    use koebi_render::Scene;

    use super::*;
    use crate::{Input, UiState};

    #[test]
    fn an_operation_takes_its_keys_until_the_user_ends_it() {
        let styles = crate::test_styles();
        let (mut scene, mut state, mut input) = (Scene::new(), UiState::new(), Input::new());
        let area = Rect::from_origin_size(Vec2::ZERO, Vec2::new(100.0, 100.0));
        let mut frame = |input: &mut Input, state: &mut UiState| {
            let mut ui = Ui::new(&mut scene, &styles, input, state);
            let end = ui.zone(area, Context::new("app"), Ui::operation);
            drop(ui);
            let mut chain = Vec::new();
            state.chain_at(Some(Vec2::new(50.0, 50.0)), &mut chain);
            input.end_frame();
            (end, chain)
        };
        // Going on: its context comes first in the chain of the next frame.
        let (end, chain) = frame(&mut input, &mut state);
        assert_eq!(end, None);
        assert_eq!(chain.first(), Some(&OPERATION));
        input.trigger(&OPERATION_CANCEL);
        assert_eq!(
            frame(&mut input, &mut state).0,
            Some(OperationEnd::Cancelled)
        );
        input.trigger(&OPERATION_CONFIRM);
        assert_eq!(
            frame(&mut input, &mut state).0,
            Some(OperationEnd::Confirmed)
        );
        // Every default of the cancel lies in the operation's context: one place to bind them.
        assert!(
            OPERATION_ACTIONS
                .actions
                .iter()
                .all(|action| action.context == OPERATION)
        );
    }
}
