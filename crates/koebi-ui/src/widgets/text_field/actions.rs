//! The keyboard actions of a text field being edited, in the [`TEXT`] context. The field
//! grabs the whole keyboard while it edits ([`Ui::grab`](crate::Ui::grab)):
//! the shortcuts of other contexts wait until the edit ends.
//!
//! Defaults follow each platform: the primary modifier (Command on Apple platforms, Ctrl
//! elsewhere) for the clipboard, undo and select all; Option on Apple platforms and Ctrl
//! elsewhere to move and delete by word.

use koebi_core::input::{Key, ModifierKey};
use koebi_i18n::n_;
use koebi_input::{
    ActionDef, ActionGroup, ActionKind, Context, Hint, ModifierRule, ModifierState, Shortcut,
};

use crate::widgets::repeating;

/// A text field editing text: its keys apply wherever the pointer is.
pub const TEXT: Context = Context::new("text");

/// Whether the platform is Apple's, whose text shortcuts differ.
const APPLE: bool = cfg!(any(target_os = "macos", target_os = "ios"));

/// The modifier that moves and deletes by word.
const WORD: ModifierKey = if APPLE {
    ModifierKey::Alt
} else {
    ModifierKey::Ctrl
};

/// `key`, repeating while held, with Shift held or not: deleting while typing capitals.
const fn deleting(key: Key) -> Shortcut {
    repeating(key).modifiers(ModifierRule::NONE.with(ModifierKey::Shift, ModifierState::Any))
}

/// `true` when one of `defaults` repeats while its key is held.
const fn repeats_by_default(defaults: &[Shortcut]) -> bool {
    let mut index = 0;
    while index < defaults.len() {
        if defaults[index].repeats() {
            return true;
        }
        index += 1;
    }
    false
}

/// The hint of a text action: none, or the one given.
macro_rules! hint {
    () => {
        None
    };
    ($hint:expr) => {
        Some($hint)
    };
}

/// Declares the text actions: `NAME, "id", "label", "description", [defaults]` and, for those
/// the status bar shows, `, hint`; then `;`.
macro_rules! text_actions {
    ($($(#[$doc:meta])* $name:ident, $id:literal, $label:expr, $description:expr,
        $defaults:expr $(, $hint:expr)?;)*) => {
        $(
            $(#[$doc])*
            pub static $name: ActionDef = {
                const DEFAULTS: &[Shortcut] = $defaults;
                ActionDef {
                    id: $id,
                    label: $label,
                    description: $description,
                    context: TEXT,
                    defaults: DEFAULTS,
                    // Moving and deleting repeat while their key is held, as their defaults do.
                    repeats: repeats_by_default(DEFAULTS),
                    when: None,
                    hint: hint!($($hint)?),
                }
            };
        )*

        /// The actions of text fields, for the keymap.
        pub static TEXT_ACTIONS: ActionGroup = ActionGroup {
            name: n_!("Text editing"),
            actions: &[$(&$name),*],
        };
    };
}

text_actions! {
    /// Ends the edit, keeping the text.
    TEXT_CONFIRM, "text.confirm", n_!("Confirm"), n_!("Ends the edit and keeps the text."),
        &[Shortcut::key(Key::Enter), Shortcut::key(Key::KeypadEnter)],
        Hint::new(ActionKind::Confirm, 9);
    /// Ends the edit, restoring the text it started with.
    TEXT_CANCEL, "text.cancel", n_!("Cancel"), n_!("Ends the edit and restores the text it started with."),
        &[Shortcut::key(Key::Escape)],
        Hint::new(ActionKind::Confirm, 8);
    /// Keeps the text and edits the next field.
    TEXT_NEXT_FIELD, "text.next-field", n_!("Next field"),
        n_!("Keeps the text and edits the next field."), &[Shortcut::key(Key::Tab)],
        Hint::new(ActionKind::Navigate, 4);
    /// Keeps the text and edits the previous field.
    TEXT_PREVIOUS_FIELD, "text.previous-field", n_!("Previous field"),
        n_!("Keeps the text and edits the previous field."),
        &[Shortcut::key(Key::Tab).with(ModifierKey::Shift)],
        Hint::new(ActionKind::Navigate, 3);
    /// Moves the caret one character left.
    TEXT_LEFT, "text.left", n_!("Left"), n_!("Moves the caret one character left."),
        &[repeating(Key::Left)];
    /// Moves the caret one character right.
    TEXT_RIGHT, "text.right", n_!("Right"), n_!("Moves the caret one character right."),
        &[repeating(Key::Right)];
    /// Moves the caret to the start of the previous word.
    TEXT_WORD_LEFT, "text.word-left", n_!("Word left"),
        n_!("Moves the caret to the start of the word before it."),
        &[repeating(Key::Left).with(WORD)];
    /// Moves the caret to the end of the next word.
    TEXT_WORD_RIGHT, "text.word-right", n_!("Word right"),
        n_!("Moves the caret to the end of the word after it."),
        &[repeating(Key::Right).with(WORD)];
    /// Moves the caret to the start of the text.
    TEXT_START, "text.start", n_!("Start"), n_!("Moves the caret to the start of the text."),
        if APPLE {
            &[Shortcut::key(Key::Home), Shortcut::key(Key::Left).primary()]
        } else {
            &[Shortcut::key(Key::Home)]
        };
    /// Moves the caret to the end of the text.
    TEXT_END, "text.end", n_!("End"), n_!("Moves the caret to the end of the text."),
        if APPLE {
            &[Shortcut::key(Key::End), Shortcut::key(Key::Right).primary()]
        } else {
            &[Shortcut::key(Key::End)]
        };
    /// Extends the selection one character left.
    TEXT_SELECT_LEFT, "text.select-left", n_!("Select left"),
        n_!("Extends the selection one character left."),
        &[repeating(Key::Left).with(ModifierKey::Shift)];
    /// Extends the selection one character right.
    TEXT_SELECT_RIGHT, "text.select-right", n_!("Select right"),
        n_!("Extends the selection one character right."),
        &[repeating(Key::Right).with(ModifierKey::Shift)];
    /// Extends the selection to the start of the previous word.
    TEXT_SELECT_WORD_LEFT, "text.select-word-left", n_!("Select word left"),
        n_!("Extends the selection to the start of the word before the caret."),
        &[repeating(Key::Left).with(WORD).with(ModifierKey::Shift)];
    /// Extends the selection to the end of the next word.
    TEXT_SELECT_WORD_RIGHT, "text.select-word-right", n_!("Select word right"),
        n_!("Extends the selection to the end of the word after the caret."),
        &[repeating(Key::Right).with(WORD).with(ModifierKey::Shift)];
    /// Extends the selection to the start of the text.
    TEXT_SELECT_START, "text.select-start", n_!("Select to start"),
        n_!("Extends the selection to the start of the text."),
        if APPLE {
            &[
                Shortcut::key(Key::Home).with(ModifierKey::Shift),
                Shortcut::key(Key::Left).primary().with(ModifierKey::Shift),
            ]
        } else {
            &[Shortcut::key(Key::Home).with(ModifierKey::Shift)]
        };
    /// Extends the selection to the end of the text.
    TEXT_SELECT_END, "text.select-end", n_!("Select to end"),
        n_!("Extends the selection to the end of the text."),
        if APPLE {
            &[
                Shortcut::key(Key::End).with(ModifierKey::Shift),
                Shortcut::key(Key::Right).primary().with(ModifierKey::Shift),
            ]
        } else {
            &[Shortcut::key(Key::End).with(ModifierKey::Shift)]
        };
    /// Selects the whole text.
    TEXT_SELECT_ALL, "text.select-all", n_!("Select all"), n_!("Selects the whole text."),
        &[Shortcut::key(Key::A).primary()],
        Hint::new(ActionKind::Select, 5);
    /// Deletes the selection or the character before the caret.
    TEXT_DELETE_BACKWARD, "text.delete-backward", n_!("Delete backward"),
        n_!("Deletes the selection, or the character before the caret."),
        &[deleting(Key::Backspace)];
    /// Deletes the selection or the character after the caret.
    TEXT_DELETE_FORWARD, "text.delete-forward", n_!("Delete forward"),
        n_!("Deletes the selection, or the character after the caret."),
        &[deleting(Key::Delete)];
    /// Deletes the selection or the word before the caret.
    TEXT_DELETE_WORD_BACKWARD, "text.delete-word-backward", n_!("Delete word backward"),
        n_!("Deletes the selection, or the word before the caret."),
        &[repeating(Key::Backspace).with(WORD)];
    /// Deletes the selection or the word after the caret.
    TEXT_DELETE_WORD_FORWARD, "text.delete-word-forward", n_!("Delete word forward"),
        n_!("Deletes the selection, or the word after the caret."),
        &[repeating(Key::Delete).with(WORD)];
    /// Copies the selection to the clipboard.
    TEXT_COPY, "text.copy", n_!("Copy"), n_!("Copies the selected text to the clipboard."),
        &[Shortcut::key(Key::C).primary()],
        Hint::new(ActionKind::Edit, 6);
    /// Copies the selection to the clipboard and deletes it.
    TEXT_CUT, "text.cut", n_!("Cut"), n_!("Copies the selected text to the clipboard and deletes it."),
        &[Shortcut::key(Key::X).primary()],
        Hint::new(ActionKind::Edit, 5);
    /// Replaces the selection with the clipboard's text.
    TEXT_PASTE, "text.paste", n_!("Paste"), n_!("Replaces the selection with the clipboard's text."),
        &[Shortcut::key(Key::V).primary()],
        Hint::new(ActionKind::Edit, 6);
    /// Undoes the last edit of the field.
    TEXT_UNDO, "text.undo", n_!("Undo"), n_!("Undoes the last change to the text being edited."),
        &[repeating(Key::Z).primary()],
        Hint::new(ActionKind::Edit, 7);
    /// Redoes the last undone edit of the field.
    TEXT_REDO, "text.redo", n_!("Redo"), n_!("Redoes the last change undone in the text being edited."),
        if APPLE {
            &[repeating(Key::Z).primary().with(ModifierKey::Shift)]
        } else {
            &[
                repeating(Key::Z).primary().with(ModifierKey::Shift),
                repeating(Key::Y).primary(),
            ]
        },
        Hint::new(ActionKind::Edit, 4);
}
