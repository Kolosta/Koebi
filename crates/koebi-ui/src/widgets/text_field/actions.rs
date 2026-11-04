//! The keyboard actions of a text field being edited, in the [`TEXT`] context. The field
//! grabs the whole keyboard while it edits ([`Ui::grab`](crate::Ui::grab)):
//! the shortcuts of other contexts wait until the edit ends.
//!
//! Defaults follow each platform: the primary modifier (Command on Apple platforms, Ctrl
//! elsewhere) for the clipboard, undo and select all; Option on Apple platforms and Ctrl
//! elsewhere to move and delete by word.

use koebi_core::input::{Key, ModifierKey};
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
    ($($(#[$doc:meta])* $name:ident, $id:literal, $label:literal, $description:literal,
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
            name: "Text editing",
            actions: &[$(&$name),*],
        };
    };
}

text_actions! {
    /// Ends the edit, keeping the text.
    TEXT_CONFIRM, "text.confirm", "Confirm", "Ends the edit and keeps the text.",
        &[Shortcut::key(Key::Enter), Shortcut::key(Key::KeypadEnter)],
        Hint::new(ActionKind::Confirm, 9);
    /// Ends the edit, restoring the text it started with.
    TEXT_CANCEL, "text.cancel", "Cancel", "Ends the edit and restores the text it started with.",
        &[Shortcut::key(Key::Escape)],
        Hint::new(ActionKind::Confirm, 8);
    /// Keeps the text and edits the next field.
    TEXT_NEXT_FIELD, "text.next-field", "Next field",
        "Keeps the text and edits the next field.", &[Shortcut::key(Key::Tab)],
        Hint::new(ActionKind::Navigate, 4);
    /// Keeps the text and edits the previous field.
    TEXT_PREVIOUS_FIELD, "text.previous-field", "Previous field",
        "Keeps the text and edits the previous field.",
        &[Shortcut::key(Key::Tab).with(ModifierKey::Shift)],
        Hint::new(ActionKind::Navigate, 3);
    /// Moves the caret one character left.
    TEXT_LEFT, "text.left", "Left", "Moves the caret one character left.",
        &[repeating(Key::Left)];
    /// Moves the caret one character right.
    TEXT_RIGHT, "text.right", "Right", "Moves the caret one character right.",
        &[repeating(Key::Right)];
    /// Moves the caret to the start of the previous word.
    TEXT_WORD_LEFT, "text.word-left", "Word left",
        "Moves the caret to the start of the word before it.",
        &[repeating(Key::Left).with(WORD)];
    /// Moves the caret to the end of the next word.
    TEXT_WORD_RIGHT, "text.word-right", "Word right",
        "Moves the caret to the end of the word after it.",
        &[repeating(Key::Right).with(WORD)];
    /// Moves the caret to the start of the text.
    TEXT_START, "text.start", "Start", "Moves the caret to the start of the text.",
        if APPLE {
            &[Shortcut::key(Key::Home), Shortcut::key(Key::Left).primary()]
        } else {
            &[Shortcut::key(Key::Home)]
        };
    /// Moves the caret to the end of the text.
    TEXT_END, "text.end", "End", "Moves the caret to the end of the text.",
        if APPLE {
            &[Shortcut::key(Key::End), Shortcut::key(Key::Right).primary()]
        } else {
            &[Shortcut::key(Key::End)]
        };
    /// Extends the selection one character left.
    TEXT_SELECT_LEFT, "text.select-left", "Select left",
        "Extends the selection one character left.",
        &[repeating(Key::Left).with(ModifierKey::Shift)];
    /// Extends the selection one character right.
    TEXT_SELECT_RIGHT, "text.select-right", "Select right",
        "Extends the selection one character right.",
        &[repeating(Key::Right).with(ModifierKey::Shift)];
    /// Extends the selection to the start of the previous word.
    TEXT_SELECT_WORD_LEFT, "text.select-word-left", "Select word left",
        "Extends the selection to the start of the word before the caret.",
        &[repeating(Key::Left).with(WORD).with(ModifierKey::Shift)];
    /// Extends the selection to the end of the next word.
    TEXT_SELECT_WORD_RIGHT, "text.select-word-right", "Select word right",
        "Extends the selection to the end of the word after the caret.",
        &[repeating(Key::Right).with(WORD).with(ModifierKey::Shift)];
    /// Extends the selection to the start of the text.
    TEXT_SELECT_START, "text.select-start", "Select to start",
        "Extends the selection to the start of the text.",
        if APPLE {
            &[
                Shortcut::key(Key::Home).with(ModifierKey::Shift),
                Shortcut::key(Key::Left).primary().with(ModifierKey::Shift),
            ]
        } else {
            &[Shortcut::key(Key::Home).with(ModifierKey::Shift)]
        };
    /// Extends the selection to the end of the text.
    TEXT_SELECT_END, "text.select-end", "Select to end",
        "Extends the selection to the end of the text.",
        if APPLE {
            &[
                Shortcut::key(Key::End).with(ModifierKey::Shift),
                Shortcut::key(Key::Right).primary().with(ModifierKey::Shift),
            ]
        } else {
            &[Shortcut::key(Key::End).with(ModifierKey::Shift)]
        };
    /// Selects the whole text.
    TEXT_SELECT_ALL, "text.select-all", "Select all", "Selects the whole text.",
        &[Shortcut::key(Key::A).primary()],
        Hint::new(ActionKind::Select, 5);
    /// Deletes the selection or the character before the caret.
    TEXT_DELETE_BACKWARD, "text.delete-backward", "Delete backward",
        "Deletes the selection, or the character before the caret.",
        &[deleting(Key::Backspace)];
    /// Deletes the selection or the character after the caret.
    TEXT_DELETE_FORWARD, "text.delete-forward", "Delete forward",
        "Deletes the selection, or the character after the caret.",
        &[deleting(Key::Delete)];
    /// Deletes the selection or the word before the caret.
    TEXT_DELETE_WORD_BACKWARD, "text.delete-word-backward", "Delete word backward",
        "Deletes the selection, or the word before the caret.",
        &[repeating(Key::Backspace).with(WORD)];
    /// Deletes the selection or the word after the caret.
    TEXT_DELETE_WORD_FORWARD, "text.delete-word-forward", "Delete word forward",
        "Deletes the selection, or the word after the caret.",
        &[repeating(Key::Delete).with(WORD)];
    /// Copies the selection to the clipboard.
    TEXT_COPY, "text.copy", "Copy", "Copies the selected text to the clipboard.",
        &[Shortcut::key(Key::C).primary()],
        Hint::new(ActionKind::Edit, 6);
    /// Copies the selection to the clipboard and deletes it.
    TEXT_CUT, "text.cut", "Cut", "Copies the selected text to the clipboard and deletes it.",
        &[Shortcut::key(Key::X).primary()],
        Hint::new(ActionKind::Edit, 5);
    /// Replaces the selection with the clipboard's text.
    TEXT_PASTE, "text.paste", "Paste", "Replaces the selection with the clipboard's text.",
        &[Shortcut::key(Key::V).primary()],
        Hint::new(ActionKind::Edit, 6);
    /// Undoes the last edit of the field.
    TEXT_UNDO, "text.undo", "Undo", "Undoes the last change to the text being edited.",
        &[repeating(Key::Z).primary()],
        Hint::new(ActionKind::Edit, 7);
    /// Redoes the last undone edit of the field.
    TEXT_REDO, "text.redo", "Redo", "Redoes the last change undone in the text being edited.",
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
