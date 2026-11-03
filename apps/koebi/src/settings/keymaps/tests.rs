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
    page.apply(
        &mut keymap,
        &actions::FULLSCREEN,
        None,
        Edit::Set(Binding::on(quit)),
        &mut state,
    );
    let bindings = keymap.bindings(id).unwrap();
    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[1], Binding::off(quit));
    assert!(state.bindings[1].editor.auto_off);
    // Turned on by hand against the conflict: it stays off.
    page.apply(
        &mut keymap,
        &actions::FULLSCREEN,
        Some(1),
        Edit::Set(Binding::on(quit)),
        &mut state,
    );
    assert_eq!(keymap.bindings(id).unwrap()[1], Binding::off(quit));
    // Changed to a free shortcut: it comes back on by itself.
    let free = Shortcut::key(Key::F10).with(ModifierKey::Shift);
    page.apply(
        &mut keymap,
        &actions::FULLSCREEN,
        Some(1),
        Edit::Set(Binding::off(free)),
        &mut state,
    );
    assert_eq!(keymap.bindings(id).unwrap()[1], Binding::on(free));
    assert!(!state.bindings[1].editor.auto_off);
    // Turned off by hand, it stays off when it changes.
    page.apply(
        &mut keymap,
        &actions::FULLSCREEN,
        Some(1),
        Edit::Set(Binding::off(free)),
        &mut state,
    );
    let other = Shortcut::key(Key::F9).with(ModifierKey::Shift);
    page.apply(
        &mut keymap,
        &actions::FULLSCREEN,
        Some(1),
        Edit::Set(Binding::off(other)),
        &mut state,
    );
    assert_eq!(keymap.bindings(id).unwrap()[1], Binding::off(other));
    // Removed.
    page.apply(
        &mut keymap,
        &actions::FULLSCREEN,
        Some(1),
        Edit::Remove,
        &mut state,
    );
    assert_eq!(keymap.bindings(id).unwrap().len(), 1);
    assert_eq!(state.bindings.len(), 1);
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
