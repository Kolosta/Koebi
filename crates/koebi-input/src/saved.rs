//! The text form of a keymap's overrides: a header line, then one line per modified action,
//! `action.id = shortcut, shortcut` (nothing after `=` removes every shortcut).
//!
//! ```text
//! # Koebi shortcuts 1
//! area.split-vertical = Ctrl+Alt+V, MiddleMouse double
//! viewport.view-all =
//! ```
//!
//! Only actions that differ from their defaults are written, so files stay short and keep
//! following the defaults of every action the user never changed.

use std::fmt::Write as _;

use crate::keymap::Keymap;
use crate::shortcut::Shortcut;

/// The first line of a saved keymap.
pub const HEADER: &str = "# Koebi shortcuts 1";

/// What loading overrides had to skip, one sentence each.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoadReport {
    /// Lines skipped and conflicts resolved.
    pub skipped: Vec<String>,
}

impl LoadReport {
    /// `true` when everything was applied.
    pub fn is_complete(&self) -> bool {
        self.skipped.is_empty()
    }
}

impl Keymap {
    /// The shortcuts that differ from the defaults, as text (see the module documentation).
    pub fn overrides_text(&self) -> String {
        let mut text = String::from(HEADER);
        text.push('\n');
        for (action, shortcuts) in self.entries() {
            if !self.is_modified(action.id) {
                continue;
            }
            text.push_str(action.id);
            text.push_str(" =");
            for (index, shortcut) in shortcuts.iter().enumerate() {
                let separator = if index == 0 { " " } else { ", " };
                // Writing into a `String` cannot fail.
                let _infallible = write!(text, "{separator}{shortcut}");
            }
            text.push('\n');
        }
        text
    }

    /// Replaces every override with those of `text`. Lines that cannot be applied are
    /// skipped and reported: unknown actions, invalid shortcuts, and conflicts, where the line
    /// that comes later keeps its defaults. A text without the header applies nothing.
    pub fn load_overrides(&mut self, text: &str) -> LoadReport {
        let mut report = LoadReport::default();
        let mut lines = text
            .lines()
            .enumerate()
            .map(|(index, line)| (index + 1, line.trim()))
            .filter(|(_, line)| !line.is_empty());
        if lines.next().map(|(_, line)| line) != Some(HEADER) {
            report.skipped.push(format!(
                "not a saved keymap: the first line must be {HEADER:?}"
            ));
            return report;
        }
        self.reset_all();
        // File order decides which override loses a conflict.
        let mut applied: Vec<usize> = Vec::new();
        for (number, line) in lines {
            if line.starts_with('#') {
                continue;
            }
            let Some((id, list)) = line.split_once('=') else {
                report.skipped.push(format!("line {number}: no `=`"));
                continue;
            };
            let id = id.trim();
            let parsed: Result<Vec<Shortcut>, _> = list
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::parse)
                .collect();
            let shortcuts = match parsed {
                Ok(shortcuts) => shortcuts,
                Err(error) => {
                    report.skipped.push(format!("line {number}: {error}"));
                    continue;
                }
            };
            match self.assign_unchecked(id, shortcuts) {
                Some(index) => {
                    applied.retain(|&earlier| earlier != index);
                    applied.push(index);
                }
                None => report
                    .skipped
                    .push(format!("line {number}: unknown action {id}")),
            }
        }
        // Each round restores one action's defaults: the number of overrides bounds it.
        for _ in 0..=applied.len() {
            let Some((a, b, shortcut)) = self.first_conflict() else {
                break;
            };
            let rank = |index: usize| applied.iter().position(|&other| other == index);
            let (loser, winner) = if rank(a) > rank(b) { (a, b) } else { (b, a) };
            self.restore_unchecked(loser);
            applied.retain(|&index| index != loser);
            report.skipped.push(format!(
                "{}: {shortcut} is already a shortcut of {}; defaults kept",
                self.action_at(loser).id,
                self.action_at(winner).id
            ));
        }
        report
    }
}
