//! [`TextEdit`]: a line of text being edited: its content, the caret and the selection, moves
//! by character and by word, deletion, and the steps to undo and redo.
//!
//! Positions are byte offsets on character boundaries. The caret moves over characters (code
//! points): a character made of several (an accent typed apart) takes several moves.

use std::collections::VecDeque;

/// What a move does to the selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Select {
    /// The caret moves alone: the selection collapses.
    Collapse,
    /// The selection follows the caret from its anchor.
    Extend,
}

/// The unit of a move or a deletion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// One character.
    Character,
    /// To the edge of a word.
    Word,
    /// To the start or the end of the line.
    Line,
}

/// A state to come back to.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Snapshot {
    text: String,
    caret: usize,
    anchor: usize,
}

/// How the last edit can merge with the next one into one undo step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Last {
    /// Nothing merges with the next edit.
    Nothing,
    /// Typing ended at this caret position: more typing there continues the step.
    Typing(usize),
}

/// The most steps kept to undo.
const UNDO_LIMIT: usize = 100;

/// A line of text being edited.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TextEdit {
    text: String,
    caret: usize,
    anchor: usize,
    undo: VecDeque<Snapshot>,
    redo: Vec<Snapshot>,
    last: Last,
}

impl TextEdit {
    /// Editing `text`, the caret at its end.
    pub(crate) fn new(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            caret: text.len(),
            anchor: text.len(),
            undo: VecDeque::new(),
            redo: Vec::new(),
            last: Last::Nothing,
        }
    }

    /// The text.
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// The caret, in bytes.
    pub(crate) fn caret(&self) -> usize {
        self.caret
    }

    /// The selected bytes, empty when nothing is selected.
    pub(crate) fn selection(&self) -> core::ops::Range<usize> {
        self.caret.min(self.anchor)..self.caret.max(self.anchor)
    }

    /// The selected text.
    pub(crate) fn selected(&self) -> &str {
        &self.text[self.selection()]
    }

    /// Selects everything.
    pub(crate) fn select_all(&mut self) {
        self.anchor = 0;
        self.caret = self.text.len();
        self.last = Last::Nothing;
    }

    /// Puts the caret at `position` (clamped to a character boundary), extending the
    /// selection or not.
    pub(crate) fn place(&mut self, position: usize, select: Select) {
        self.caret = self.boundary(position);
        if select == Select::Collapse {
            self.anchor = self.caret;
        }
        self.last = Last::Nothing;
    }

    /// Selects the word around `position` (or the run of other characters there).
    pub(crate) fn select_word(&mut self, position: usize) {
        let position = self.boundary(position);
        let class = self.text[position..]
            .chars()
            .next()
            .or_else(|| self.text[..position].chars().next_back())
            .map(class);
        let Some(class_here) = class else {
            return;
        };
        let start = self.text[..position]
            .char_indices()
            .rev()
            .take_while(|(_, c)| self::class(*c) == class_here)
            .last()
            .map_or(position, |(index, _)| index);
        let end = self.text[position..]
            .char_indices()
            .find(|(_, c)| self::class(*c) != class_here)
            .map_or(self.text.len(), |(index, _)| position + index);
        self.anchor = start;
        self.caret = end;
        self.last = Last::Nothing;
    }

    /// Moves the caret left (`forward` false) or right by `step`. Without extending, a
    /// selection collapses to its edge on that side first.
    pub(crate) fn step(&mut self, forward: bool, step: Step, select: Select) {
        let selection = self.selection();
        if select == Select::Collapse && !selection.is_empty() && step == Step::Character {
            self.caret = if forward {
                selection.end
            } else {
                selection.start
            };
            self.anchor = self.caret;
            self.last = Last::Nothing;
            return;
        }
        let target = self.target(self.caret, forward, step);
        self.place(target, select);
    }

    /// Deletes the selection, or `step` before the caret (`forward` false: Backspace) or
    /// after it (Delete). Returns `true` when something was deleted.
    pub(crate) fn delete(&mut self, forward: bool, step: Step) -> bool {
        let mut range = self.selection();
        if range.is_empty() {
            let target = self.target(self.caret, forward, step);
            range = self.caret.min(target)..self.caret.max(target);
        }
        if range.is_empty() {
            return false;
        }
        self.save();
        self.text.replace_range(range.clone(), "");
        self.caret = range.start;
        self.anchor = range.start;
        true
    }

    /// Replaces the selection with `text` (typing, pasting). Line breaks become spaces: the
    /// text is one line. Returns `true` when the text changed.
    pub(crate) fn insert(&mut self, text: &str) -> bool {
        let range = self.selection();
        // Line breaks and tabs become spaces; other control characters vanish.
        let kept = |c: char| matches!(c, '\n' | '\r' | '\t') || !c.is_control();
        if range.is_empty() && !text.chars().any(kept) {
            return false;
        }
        // Consecutive typing makes one undo step; a word boundary starts a new one.
        let continues = range.is_empty()
            && self.last == Last::Typing(self.caret)
            && !text.starts_with(char::is_whitespace);
        if !continues {
            self.save();
        }
        let start = range.start;
        let caret = if text.chars().all(|c| !c.is_control()) {
            // The usual case, typing or pasting plain text: one replacement.
            self.text.replace_range(range, text);
            start + text.len()
        } else {
            let line: String = text
                .chars()
                .filter(|c| kept(*c))
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect();
            self.text.replace_range(range, &line);
            start + line.len()
        };
        self.caret = caret;
        self.anchor = caret;
        self.last = Last::Typing(caret);
        true
    }

    /// Replaces the selection with pasted `text`: one undo step of its own, which typing
    /// after it does not continue. Returns `true` when the text changed.
    pub(crate) fn paste(&mut self, text: &str) -> bool {
        self.last = Last::Nothing;
        let changed = self.insert(text);
        self.last = Last::Nothing;
        changed
    }

    /// Replaces the whole text (an outside change), keeping it undoable.
    pub(crate) fn set_text(&mut self, text: &str) {
        if text == self.text {
            return;
        }
        self.save();
        self.text.clear();
        self.text.push_str(text);
        self.caret = self.text.len();
        self.anchor = self.caret;
    }

    /// Goes back one step. Returns `true` when there was one.
    pub(crate) fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop_back() else {
            return false;
        };
        let current = self.snapshot();
        self.redo.push(current);
        self.restore(previous);
        true
    }

    /// Goes forward one undone step. Returns `true` when there was one.
    pub(crate) fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        let current = self.snapshot();
        self.undo.push_back(current);
        self.restore(next);
        true
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text.clone(),
            caret: self.caret,
            anchor: self.anchor,
        }
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.text = snapshot.text;
        self.caret = snapshot.caret;
        self.anchor = snapshot.anchor;
        self.last = Last::Nothing;
    }

    /// Saves the current state as a step to undo before an edit.
    fn save(&mut self) {
        if self.undo.len() == UNDO_LIMIT {
            self.undo.pop_front();
        }
        let current = self.snapshot();
        self.undo.push_back(current);
        self.redo.clear();
        self.last = Last::Nothing;
    }

    /// `position` moved back to the character boundary at or before it, within the text.
    fn boundary(&self, position: usize) -> usize {
        let mut position = position.min(self.text.len());
        while !self.text.is_char_boundary(position) {
            position -= 1;
        }
        position
    }

    /// Where a move of `step` from `from` lands.
    fn target(&self, from: usize, forward: bool, step: Step) -> usize {
        match (step, forward) {
            (Step::Line, false) => 0,
            (Step::Line, true) => self.text.len(),
            (Step::Character, false) => self.text[..from]
                .char_indices()
                .next_back()
                .map_or(0, |(index, _)| index),
            (Step::Character, true) => self.text[from..]
                .chars()
                .next()
                .map_or(from, |c| from + c.len_utf8()),
            // Words: skip the spaces, then the run of the same class.
            (Step::Word, false) => {
                let before = &self.text[..from];
                let trimmed = before.trim_end();
                let class_here = trimmed.chars().next_back().map(class);
                trimmed
                    .char_indices()
                    .rev()
                    .take_while(|(_, c)| Some(class(*c)) == class_here)
                    .last()
                    .map_or(0, |(index, _)| index)
            }
            (Step::Word, true) => {
                let after = &self.text[from..];
                let start = after.len() - after.trim_start().len();
                let class_here = after[start..].chars().next().map(class);
                let end = after[start..]
                    .char_indices()
                    .find(|(_, c)| Some(class(*c)) != class_here)
                    .map_or(after.len(), |(index, _)| start + index);
                from + end
            }
        }
    }
}

/// The class of a character for word moves: word characters, spaces, and the rest.
fn class(c: char) -> u8 {
    if c.is_alphanumeric() || c == '_' {
        0
    } else if c.is_whitespace() {
        1
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_inserts_at_the_caret_and_replaces_the_selection() {
        let mut edit = TextEdit::new("Cube");
        assert!(edit.insert(".001"));
        assert_eq!(edit.text(), "Cube.001");
        edit.select_all();
        assert_eq!(edit.selected(), "Cube.001");
        edit.insert("Sph\u{e8}re");
        assert_eq!((edit.text(), edit.caret()), ("Sph\u{e8}re", 7));
        // Line breaks become spaces; control characters vanish.
        edit.insert("\na\u{7}b");
        assert_eq!(edit.text(), "Sph\u{e8}re ab");
        assert!(!edit.insert(""));
    }

    #[test]
    fn the_caret_moves_by_character_word_and_line() {
        let mut edit = TextEdit::new("snap to grid");
        edit.step(false, Step::Word, Select::Collapse);
        assert_eq!(edit.caret(), 8);
        edit.step(false, Step::Word, Select::Collapse);
        assert_eq!(edit.caret(), 5);
        edit.step(false, Step::Character, Select::Collapse);
        assert_eq!(edit.caret(), 4);
        edit.step(true, Step::Word, Select::Collapse);
        assert_eq!(edit.caret(), 7);
        edit.step(false, Step::Line, Select::Collapse);
        assert_eq!(edit.caret(), 0);
        edit.step(true, Step::Line, Select::Extend);
        assert_eq!(edit.selected(), "snap to grid");
        // A collapsing move puts the caret at the selection's edge.
        edit.step(false, Step::Character, Select::Collapse);
        assert_eq!((edit.caret(), edit.selected()), (0, ""));
        // Multi-byte characters move as one.
        let mut accents = TextEdit::new("\u{e9}t\u{e9}");
        accents.step(false, Step::Character, Select::Collapse);
        assert_eq!(accents.caret(), 3);
    }

    #[test]
    fn deletion_removes_characters_words_or_the_selection() {
        let mut edit = TextEdit::new("snap to grid");
        assert!(edit.delete(false, Step::Character));
        assert_eq!(edit.text(), "snap to gri");
        assert!(edit.delete(false, Step::Word));
        assert_eq!(edit.text(), "snap to ");
        edit.place(0, Select::Collapse);
        assert!(edit.delete(true, Step::Word));
        assert_eq!(edit.text(), " to ");
        edit.select_all();
        assert!(edit.delete(true, Step::Character));
        assert_eq!(edit.text(), "");
        assert!(!edit.delete(false, Step::Character));
    }

    #[test]
    fn words_are_selected_where_the_pointer_is() {
        let mut edit = TextEdit::new("snap to_grid, now");
        edit.select_word(10);
        assert_eq!(edit.selected(), "to_grid");
        edit.select_word(12);
        assert_eq!(edit.selected(), ",");
        edit.select_word(100);
        assert_eq!(edit.selected(), "now");
    }

    #[test]
    fn typing_undoes_by_word_and_redoes() {
        let mut edit = TextEdit::new("");
        for c in ["s", "n", "a", "p", " ", "o", "n"] {
            edit.insert(c);
        }
        assert_eq!(edit.text(), "snap on");
        assert!(edit.undo());
        assert_eq!(edit.text(), "snap");
        assert!(edit.undo());
        assert_eq!(edit.text(), "");
        assert!(!edit.undo());
        assert!(edit.redo());
        assert!(edit.redo());
        assert_eq!(edit.text(), "snap on");
        assert!(!edit.redo());
        // A new edit drops what was undone.
        edit.undo();
        edit.delete(false, Step::Character);
        assert!(!edit.redo());
        assert_eq!(edit.text(), "sna");
    }

    #[test]
    fn only_control_characters_change_nothing_and_old_steps_are_dropped() {
        let mut edit = TextEdit::new("Cube");
        assert!(!edit.insert("\u{7}\u{1b}"));
        assert!(!edit.undo());
        for _ in 0..UNDO_LIMIT + 10 {
            edit.select_all();
            edit.insert("x");
        }
        let mut steps = 0;
        while edit.undo() {
            steps += 1;
        }
        assert_eq!(steps, UNDO_LIMIT);
    }

    #[test]
    fn words_skip_punctuation_and_multi_byte_text_as_wholes() {
        let mut edit = TextEdit::new("\u{e9}t\u{e9}, d\u{e9}j\u{e0}");
        edit.step(false, Step::Word, Select::Collapse);
        assert_eq!(&edit.text()[edit.caret()..], "d\u{e9}j\u{e0}");
        edit.step(false, Step::Word, Select::Collapse);
        assert_eq!(&edit.text()[edit.caret()..], ", d\u{e9}j\u{e0}");
        edit.step(false, Step::Word, Select::Collapse);
        assert_eq!(edit.caret(), 0);
        edit.step(true, Step::Word, Select::Extend);
        assert_eq!(edit.selected(), "\u{e9}t\u{e9}");
    }

    #[test]
    fn positions_snap_to_character_boundaries() {
        let mut edit = TextEdit::new("\u{e9}t\u{e9}");
        edit.place(1, Select::Collapse);
        assert_eq!(edit.caret(), 0);
        edit.place(99, Select::Collapse);
        assert_eq!(edit.caret(), 5);
        edit.set_text("abc");
        assert_eq!((edit.text(), edit.caret()), ("abc", 3));
        assert!(edit.undo());
        assert_eq!(edit.text(), "\u{e9}t\u{e9}");
    }
}
