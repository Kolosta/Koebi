//! Tests of the Keymaps page: edits against conflicts, searches, identifiers.

use koebi_core::input::{Key, ModifierKey};

use super::*;
use crate::actions;

fn keymap() -> Keymap {
    Keymap::new(&actions::GROUPS).unwrap()
}

#[test]
fn a_shortcut_taken_in_its_zone_stays_off_until_it_changes_to_a_free_one() {
    let (mut page, mut keymap) = (KeymapsPage::new(), keymap());
    let mut state = ActionUi::new();
    let id = actions::FULLSCREEN.id;
    let quit = keymap.shortcuts(actions::QUIT.id).unwrap()[0];
    // Added: Quit's shortcut, in the same zone: kept, off.
    state.drafts.push(Draft::new());
    page.apply(
        &mut keymap,
        &actions::FULLSCREEN,
        Target::Draft(0),
        Edit::Set(Binding::on(quit)),
        &mut state,
    );
    assert!(state.drafts.is_empty());
    let bindings = keymap.bindings(id).unwrap();
    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[1], Binding::off(quit));
    assert!(state.bindings[1].editor.auto_off);
    // Turned on by hand against the conflict: it stays off.
    page.apply(
        &mut keymap,
        &actions::FULLSCREEN,
        Target::Binding(1),
        Edit::Set(Binding::on(quit)),
        &mut state,
    );
    assert_eq!(keymap.bindings(id).unwrap()[1], Binding::off(quit));
    // Changed to a free shortcut: it comes back on by itself.
    let free = Shortcut::key(Key::F10).with(ModifierKey::Shift);
    page.apply(
        &mut keymap,
        &actions::FULLSCREEN,
        Target::Binding(1),
        Edit::Set(Binding::off(free)),
        &mut state,
    );
    assert_eq!(keymap.bindings(id).unwrap()[1], Binding::on(free));
    assert!(!state.bindings[1].editor.auto_off);
    // Turned off by hand, it stays off when it changes.
    page.apply(
        &mut keymap,
        &actions::FULLSCREEN,
        Target::Binding(1),
        Edit::Set(Binding::off(free)),
        &mut state,
    );
    let other = Shortcut::key(Key::F9).with(ModifierKey::Shift);
    page.apply(
        &mut keymap,
        &actions::FULLSCREEN,
        Target::Binding(1),
        Edit::Set(Binding::off(other)),
        &mut state,
    );
    assert_eq!(keymap.bindings(id).unwrap()[1], Binding::off(other));
    // Removed.
    page.apply(
        &mut keymap,
        &actions::FULLSCREEN,
        Target::Binding(1),
        Edit::Remove,
        &mut state,
    );
    assert_eq!(keymap.bindings(id).unwrap().len(), 1);
    assert_eq!(state.bindings.len(), 1);
}

#[test]
fn a_draft_waits_for_its_key_quietly_then_becomes_a_shortcut() {
    let (mut page, mut keymap) = (KeymapsPage::new(), keymap());
    let id = actions::FULLSCREEN.id;
    let before = keymap.bindings(id).unwrap().len();
    page.add_draft(id);
    page.add_draft(id);
    assert_eq!(page.drafts(id), 2);
    // Nothing in the keymap yet: no conflict to show.
    assert_eq!(keymap.bindings(id).unwrap().len(), before);
    page.analysis.refresh(&keymap, &ZONES);
    assert!(
        page.analysis
            .action(id)
            .is_some_and(|report| report.worst.is_none())
    );
    // Another device, then removed: still nothing in the keymap.
    page.edit_draft(&mut keymap, id, 1, Edit::Device(binding::Device::Mouse));
    page.edit_draft(&mut keymap, id, 1, Edit::Remove);
    assert_eq!(page.drafts(id), 1);
    assert_eq!(keymap.bindings(id).unwrap().len(), before);
    // Its key chosen: a shortcut of the action, its open panel kept.
    let key = Shortcut::key(Key::F7);
    page.edit_draft(&mut keymap, id, 0, Edit::Set(Binding::on(key)));
    assert_eq!(page.drafts(id), 0);
    let bindings = keymap.bindings(id).unwrap();
    assert_eq!(bindings.len(), before + 1);
    assert_eq!(bindings[before], Binding::on(key));
    let state = page.actions.get(id).unwrap();
    assert!(state.bindings[before].panel.is_open());
}

#[test]
fn searches_find_actions_by_name_and_by_shortcut() {
    let (mut page, keymap) = (KeymapsPage::new(), keymap());
    page.refresh_search(&keymap);
    assert!(page.found.actions.is_none());
    assert!(page.found.shows(actions::QUIT.id));
    page.search.push_str("QUIT");
    page.refresh_search(&keymap);
    assert!(page.found.shows(actions::QUIT.id));
    assert!(!page.found.shows(actions::FULLSCREEN.id));
    // What was found opens its zone.
    assert!(
        page.zones
            .get(&actions::APP)
            .is_some_and(CollapsiblePanelState::is_open)
    );
    page.search.clear();
    page.search_shortcut = keymap
        .shortcuts(actions::FULLSCREEN.id)
        .map(|shortcuts| shortcuts[0]);
    page.refresh_search(&keymap);
    assert!(page.found.shows(actions::FULLSCREEN.id));
    assert!(!page.found.shows(actions::QUIT.id));
    // A zone shows while it holds an action found.
    assert!(page.has(&keymap, actions::APP));
    assert!(!page.has(&keymap, koebi_ui::MENU));
}

#[test]
fn identifiers_of_names_are_stable_and_distinct() {
    assert_eq!(name_key("app.quit"), name_key("app.quit"));
    assert_ne!(name_key("app.quit"), name_key("app.fullscreen"));
}
