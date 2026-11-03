//! What the shortcut editor knows of the keymap, worked out again only when it changes: the
//! conflicts of every shortcut with what each says, the gravest of each action and of each zone
//! (its sub-zones included), and what a search finds.
//!
//! Soft conflicts between two default shortcuts are left out: the application designed them
//! together (a click that selects and a double click that opens). Those the user makes show.
//! A shortcut shared with an action that never applies at the same time is said, as an
//! information, and counts as no conflict.

use std::collections::HashMap;

use koebi_input::{Conflict, ConflictKind, Context, Keymap, Shortcut, Zones};

/// The conflicts of one shortcut of an action.
#[derive(Debug, Default)]
pub(super) struct BindingReport {
    /// The gravest.
    pub(super) worst: Option<ConflictKind>,
    /// What each says, the gravest first.
    pub(super) messages: Vec<(ConflictKind, String)>,
}

/// The conflicts of an action's shortcuts.
#[derive(Debug, Default)]
pub(super) struct ActionReport {
    /// One per shortcut, in order.
    pub(super) bindings: Vec<BindingReport>,
    /// The gravest of them.
    pub(super) worst: Option<ConflictKind>,
}

/// The conflicts of the whole keymap.
#[derive(Debug, Default)]
pub(super) struct Analysis {
    /// The revision of the keymap they were worked out from.
    revision: Option<u64>,
    actions: HashMap<&'static str, ActionReport>,
    /// The gravest conflict in each zone and the zones inside it.
    zones: HashMap<Context, ConflictKind>,
    scratch: Vec<Conflict>,
}

impl Analysis {
    /// Works the conflicts out again if `keymap` changed since.
    pub(super) fn refresh(&mut self, keymap: &Keymap, zones: &Zones) {
        if self.revision == Some(keymap.revision()) {
            return;
        }
        self.revision = Some(keymap.revision());
        self.actions.clear();
        self.zones.clear();
        for (action, bindings) in keymap.actions() {
            let mut report = ActionReport::default();
            for binding in bindings {
                keymap.conflicts_in(zones, action.id, binding.shortcut, &mut self.scratch);
                let designed = action.defaults.contains(&binding.shortcut);
                self.scratch.retain(|conflict| {
                    conflict.kind.is_total()
                        || !designed
                        || !conflict.other.defaults.contains(&conflict.shortcut)
                });
                let messages = self
                    .scratch
                    .iter()
                    .map(|conflict| {
                        let text = message(zones, action.context, binding.shortcut, conflict);
                        (conflict.kind, text)
                    })
                    .collect();
                let worst = self
                    .scratch
                    .iter()
                    .map(|conflict| conflict.kind)
                    .find(|kind| kind.is_conflict());
                report.worst = report.worst.max(worst);
                report.bindings.push(BindingReport { worst, messages });
            }
            if let Some(worst) = report.worst {
                // The zone and every zone holding it show it.
                let mut context = Some(action.context);
                for _ in 0..=zones.infos().len() {
                    let Some(current) = context else {
                        break;
                    };
                    let shown = self.zones.entry(current).or_insert(worst);
                    *shown = (*shown).max(worst);
                    context = zones.parent(current);
                }
            }
            self.actions.insert(action.id, report);
        }
    }

    /// The conflicts of the action `id`.
    pub(super) fn action(&self, id: &str) -> Option<&ActionReport> {
        self.actions.get(id)
    }

    /// The gravest conflict in `context` and the zones inside it.
    pub(super) fn zone(&self, context: Context) -> Option<ConflictKind> {
        self.zones.get(&context).copied()
    }
}

/// What a conflict of `shortcut`, a shortcut of an action in `context`, says.
fn message(zones: &Zones, context: Context, shortcut: Shortcut, conflict: &Conflict) -> String {
    let other = conflict.other;
    let place = zones.label(other.context);
    let here = zones.label(context);
    match conflict.kind {
        ConflictKind::Total => format!(
            "{shortcut} is already a shortcut of {} in {place}: it stays off until one of them \
             changes.",
            other.label
        ),
        ConflictKind::Shadowed => format!(
            "In {place}, inside {here}, {shortcut} does {} instead.",
            other.label
        ),
        ConflictKind::Shadows => format!(
            "In {here}, {shortcut} wins over {} of {place}, which it lies inside.",
            other.label
        ),
        ConflictKind::Similar => format!(
            "{} does {} in {place}: one movement does both, or one instead of the other.",
            conflict.shortcut, other.label
        ),
        ConflictKind::Shared => format!(
            "Also {} {}: the two never apply at once.",
            other.label,
            other.when.map_or("", |when| when.state)
        ),
    }
}

/// `true` when `text` holds `needle`, letters compared without their case. Searches run when
/// what they look for changes, not at every frame.
pub(super) fn contains_folded(text: &str, needle: &str) -> bool {
    text.to_lowercase().contains(&needle.to_lowercase())
}

/// `true` when a search for `shortcut` finds `bound`: the same input, or a similar one.
pub(super) fn finds(shortcut: Shortcut, bound: Shortcut) -> bool {
    shortcut.overlaps(bound) || shortcut.is_similar(bound) || shortcut == bound
}

#[cfg(test)]
mod tests {
    use koebi_core::input::{Key, ModifierKey};
    use koebi_input::Binding;

    use super::*;
    use crate::actions;

    #[test]
    fn searches_fold_case_and_find_similar_shortcuts() {
        assert!(contains_folded("Frame times", "frame"));
        assert!(contains_folded("Frame times", "IME"));
        assert!(!contains_folded("Quit", "quiet"));
        assert!(contains_folded("anything", ""));
        let home = Shortcut::key(Key::Home);
        assert!(finds(home, home));
        assert!(!finds(home, home.with(ModifierKey::Ctrl)));
    }

    #[test]
    fn conflicts_are_worked_out_once_per_change_and_reach_the_zones_holding_them() {
        let mut keymap = Keymap::new(&actions::GROUPS).unwrap();
        let mut analysis = Analysis::default();
        analysis.refresh(&keymap, &actions::ZONES);
        // The defaults, designed together, show no conflict.
        assert!(keymap.actions().all(|(action, _)| {
            analysis
                .action(action.id)
                .is_some_and(|report| report.worst.is_none())
        }));
        // Quit's shortcut, off, given to Fullscreen: a total conflict, in the application.
        let quit = keymap.shortcuts(actions::QUIT.id).unwrap()[0];
        keymap
            .set_bindings(actions::FULLSCREEN.id, &[Binding::off(quit)])
            .unwrap();
        analysis.refresh(&keymap, &actions::ZONES);
        let report = analysis.action(actions::FULLSCREEN.id).unwrap();
        assert_eq!(report.worst, Some(ConflictKind::Total));
        let (kind, text) = &report.bindings[0].messages[0];
        assert_eq!(*kind, ConflictKind::Total);
        assert!(
            text.contains("Quit") && text.contains("Application"),
            "{text}"
        );
        assert_eq!(analysis.zone(actions::APP), Some(ConflictKind::Total));
        // Nothing changed: the same answers, not worked out again.
        let revision = analysis.revision;
        analysis.refresh(&keymap, &actions::ZONES);
        assert_eq!(analysis.revision, revision);
    }
}
