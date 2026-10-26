//! The workspace layout and the shortcut overrides in the data directory, next to the design
//! state: restored at start, saved once changes settle and when the application leaves.
//!
//! Files are keyed by stable names (editor kinds, action identifiers), so that they survive
//! code changes: what a newer or older Koebi does not know is skipped, with a warning.

use std::path::Path;
use std::time::{Duration, Instant};

use koebi_input::Keymap;
use koebi_workspace::Workspace;

use crate::editors::{self, Editors};
use crate::saved::SavedFile;

/// How long the workspace stays unchanged before it is saved: one write per pause while a
/// gutter drags.
const DELAY: Duration = Duration::from_secs(1);

/// The saved workspace and shortcuts.
#[derive(Debug)]
pub(crate) struct Preferences {
    workspace_file: SavedFile,
    shortcuts_file: SavedFile,
    /// The workspace and its editors' kinds as last saved or restored.
    saved: Option<(Workspace, Editors)>,
    /// The shortcut overrides as last saved or restored.
    saved_shortcuts: Option<String>,
    /// Since when the workspace differs from the saved one.
    dirty_since: Option<Instant>,
}

impl Preferences {
    /// The files in `dir`.
    pub(crate) fn new(dir: &Path) -> Self {
        Self {
            workspace_file: SavedFile::new(dir, "workspace.txt"),
            shortcuts_file: SavedFile::new(dir, "shortcuts.txt"),
            saved: None,
            saved_shortcuts: None,
            dirty_since: None,
        }
    }

    /// The saved workspace and its editors, if one can be restored.
    pub(crate) fn load_workspace(&mut self) -> Option<(Workspace, Editors)> {
        let path = self.workspace_file.path().display().to_string();
        let texts = match self.workspace_file.read() {
            Ok(texts) => texts,
            Err(error) => {
                koebi_core::warn!("workspace: cannot read {path} ({error})");
                return None;
            }
        };
        for (index, text) in texts.iter().enumerate() {
            match editors::saved::decode(text) {
                Ok((workspace, editors, skipped)) => {
                    for line in skipped {
                        koebi_core::warn!("workspace: skipped {line}");
                    }
                    if index > 0 {
                        koebi_core::warn!("workspace: {path} was damaged: restored the backup");
                    }
                    koebi_core::info!(
                        "workspace restored from {path} ({} areas)",
                        workspace.layout().len()
                    );
                    self.saved = Some((workspace.clone(), editors.clone()));
                    return Some((workspace, editors));
                }
                Err(error) => koebi_core::warn!("workspace: cannot restore {path}: {error}"),
            }
        }
        None
    }

    /// Applies the saved shortcut overrides to `keymap`.
    pub(crate) fn load_shortcuts(&mut self, keymap: &mut Keymap) {
        let path = self.shortcuts_file.path().display().to_string();
        match self.shortcuts_file.read() {
            Ok(texts) => {
                if let Some(text) = texts.first() {
                    let report = keymap.load_overrides(text);
                    for skipped in report.skipped {
                        koebi_core::warn!("shortcuts: {skipped}");
                    }
                    koebi_core::info!("shortcuts restored from {path}");
                }
            }
            Err(error) => koebi_core::warn!("shortcuts: cannot read {path} ({error})"),
        }
        self.saved_shortcuts = Some(keymap.overrides_text());
    }

    /// `true` when the workspace differs from the saved one.
    pub(crate) fn is_dirty(&self, workspace: &Workspace, editors: &Editors) -> bool {
        self.saved.as_ref().is_none_or(|(saved, kinds)| {
            saved != workspace || !editors::saved::same_kinds(workspace, editors, kinds)
        })
    }

    /// Saves the workspace once it has stayed changed for a second.
    pub(crate) fn save_if_due(&mut self, workspace: &Workspace, editors: &Editors, now: Instant) {
        if !self.is_dirty(workspace, editors) {
            self.dirty_since = None;
            return;
        }
        let since = *self.dirty_since.get_or_insert(now);
        if now.duration_since(since) >= DELAY {
            self.save_workspace(workspace, editors);
        }
    }

    /// Saves what changed now: before quitting or going to the background.
    pub(crate) fn save(&mut self, workspace: &Workspace, editors: &Editors, keymap: &Keymap) {
        if self.is_dirty(workspace, editors) {
            self.save_workspace(workspace, editors);
        }
        let shortcuts = keymap.overrides_text();
        if self.saved_shortcuts.as_deref() != Some(shortcuts.as_str()) {
            let path = self.shortcuts_file.path().display().to_string();
            match self.shortcuts_file.write(&shortcuts) {
                Ok(()) => {
                    koebi_core::info!("shortcuts saved to {path}");
                    self.saved_shortcuts = Some(shortcuts);
                }
                Err(error) => koebi_core::warn!("cannot save the shortcuts to {path}: {error}"),
            }
        }
    }

    fn save_workspace(&mut self, workspace: &Workspace, editors: &Editors) {
        let path = self.workspace_file.path().display().to_string();
        let text = editors::saved::encode(workspace, editors);
        match self.workspace_file.write(&text) {
            Ok(()) => {
                koebi_core::info!("workspace saved to {path}");
                self.saved = Some((workspace.clone(), editors.clone()));
            }
            Err(error) => koebi_core::warn!("cannot save the workspace to {path}: {error}"),
        }
        self.dirty_since = None;
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use koebi_workspace::Orientation;

    use super::*;
    use crate::editors::{EditorKind, default_workspace};

    fn directory(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("koebi-prefs-{name}-{}", std::process::id()));
        fs::remove_dir_all(&dir).ok();
        dir
    }

    #[test]
    fn workspaces_save_once_changes_settle_and_come_back() {
        let dir = directory("workspace");
        let mut preferences = Preferences::new(&dir);
        assert!(preferences.load_workspace().is_none());
        let (mut workspace, mut editors) = default_workspace();
        let start = Instant::now();
        // Nothing saved yet: the first pause saves.
        preferences.save_if_due(&workspace, &editors, start);
        assert!(!dir.join("workspace.txt").exists());
        preferences.save_if_due(&workspace, &editors, start + DELAY);
        assert!(dir.join("workspace.txt").exists());
        assert!(!preferences.is_dirty(&workspace, &editors));
        // A split and a kind change make it dirty again.
        let area = workspace.layout().areas().next().unwrap().0;
        let split = workspace
            .split(area, Orientation::Vertical, koebi_workspace::GRID / 8, 1)
            .unwrap();
        editors.set(split.editor, EditorKind::Timeline);
        assert!(preferences.is_dirty(&workspace, &editors));
        let keymap = Keymap::new(&crate::actions::GROUPS).unwrap();
        preferences.save(&workspace, &editors, &keymap);
        let (restored, kinds) = Preferences::new(&dir).load_workspace().unwrap();
        assert_eq!(restored, workspace);
        assert_eq!(kinds.kind(split.editor), EditorKind::Timeline);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn shortcut_overrides_save_only_when_they_change() {
        let dir = directory("shortcuts");
        let mut preferences = Preferences::new(&dir);
        let mut keymap = Keymap::new(&crate::actions::GROUPS).unwrap();
        preferences.load_shortcuts(&mut keymap);
        let (workspace, editors) = default_workspace();
        preferences.save(&workspace, &editors, &keymap);
        assert!(!dir.join("shortcuts.txt").exists());
        let text = format!("{}\nviewport.view-all =\n", koebi_input::HEADER);
        keymap.load_overrides(&text);
        preferences.save(&workspace, &editors, &keymap);
        let mut restored = Keymap::new(&crate::actions::GROUPS).unwrap();
        Preferences::new(&dir).load_shortcuts(&mut restored);
        assert!(restored.is_modified("viewport.view-all"));
        fs::remove_dir_all(&dir).ok();
    }
}
