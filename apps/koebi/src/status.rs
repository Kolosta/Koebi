//! What the status bar of a window shows: the shortcuts that apply where the pointer is, as
//! Blender does, and the version.
//!
//! - **Which**: the actions of the contexts under the pointer (an action in progress's alone,
//!   [`koebi_ui::UiState::chain_at`]) that have a hint ([`koebi_input::ActionDef::hint`]),
//!   each by its first shortcut that is on; a shortcut an inner zone takes is left out of the
//!   outer one's.
//! - **In what order**: by kind (confirm, navigate, fold, select, edit, view, window), so that
//!   similar shortcuts keep their places; by priority within a kind, which also tells the bar
//!   which to keep when room is short.

use koebi_core::Vec2;
use koebi_i18n::tr;
use koebi_input::{ActionKind, Context, Keymap};
use koebi_ui::{StatusEntry, UiState};

/// The text at the end of the bar.
pub(crate) const VERSION: &str = concat!("Koebi ", env!("CARGO_PKG_VERSION"));

/// The shortcuts a window's status bar shows, worked out each frame in buffers kept between
/// frames.
#[derive(Debug, Default)]
pub(crate) struct StatusShortcuts {
    chain: Vec<Context>,
    /// Each shortcut with its kind, priority and how deep its context lies (innermost 0).
    found: Vec<(ActionKind, u8, usize, StatusEntry<'static>)>,
    entries: Vec<StatusEntry<'static>>,
}

impl StatusShortcuts {
    /// The shortcuts that apply at `pointer` in a window of `state`, by kind.
    pub(crate) fn refresh(
        &mut self,
        state: &UiState,
        pointer: Option<Vec2>,
        keymap: &Keymap,
    ) -> &[StatusEntry<'static>] {
        state.chain_at(pointer, &mut self.chain);
        self.found.clear();
        for (depth, context) in self.chain.iter().enumerate() {
            for (action, bindings) in keymap.actions() {
                let Some(hint) = action.hint.filter(|_| action.context == *context) else {
                    continue;
                };
                let Some(shortcut) = bindings
                    .iter()
                    .find(|binding| binding.enabled)
                    .map(|binding| binding.shortcut)
                else {
                    continue;
                };
                // An inner zone takes that input: the outer action does not apply here.
                let shadowed = self.found.iter().any(|(_, _, inner, entry)| {
                    *inner < depth && entry.shortcut.overlaps(shortcut)
                });
                if shadowed {
                    continue;
                }
                let entry = StatusEntry {
                    shortcut,
                    label: tr(action.label),
                    priority: hint.priority,
                };
                self.found.push((hint.kind, hint.priority, depth, entry));
            }
        }
        self.found.sort_by_key(|(kind, priority, depth, _)| {
            (*kind, core::cmp::Reverse(*priority), *depth)
        });
        self.entries.clear();
        self.entries
            .extend(self.found.iter().map(|(_, _, _, entry)| *entry));
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use koebi_ui::{Input, Styles, Ui};

    use super::*;
    use crate::actions;

    #[test]
    fn the_shortcuts_of_the_zones_under_the_pointer_show_by_kind() {
        let keymap = Keymap::new(&actions::GROUPS).unwrap();
        let styles =
            Styles::resolve(&koebi_design::DesignSystem::new(&koebi_tokens::KOEBI).unwrap());
        let (mut state, input, mut scene) =
            (UiState::new(), Input::new(), koebi_render::Scene::new());
        // A frame with the outliner inside the application, under the pointer.
        let window = koebi_core::Rect::from_origin_size(Vec2::ZERO, Vec2::new(400.0, 300.0));
        {
            let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
            ui.zone(window, actions::APP, |ui| {
                ui.zone(window, crate::editors::outliner::OUTLINER, |_| {});
            });
        }
        let mut status = StatusShortcuts::default();
        let entries = status.refresh(&state, Some(Vec2::new(100.0, 100.0)), &keymap);
        let labels: Vec<&str> = entries.iter().map(|entry| entry.label).collect();
        // The outliner's edits and folds, then the application's window actions, after them.
        assert!(
            labels.contains(&"Delete") && labels.contains(&"Fold all"),
            "{labels:?}"
        );
        let fold = labels
            .iter()
            .position(|label| *label == "Fold all")
            .unwrap();
        let delete = labels.iter().position(|label| *label == "Delete").unwrap();
        let settings = labels
            .iter()
            .position(|label| *label == "Settings")
            .unwrap();
        assert!(fold < delete && delete < settings, "{labels:?}");
        // Nowhere: nothing.
        assert_eq!(status.refresh(&state, None, &keymap), []);
    }
}
