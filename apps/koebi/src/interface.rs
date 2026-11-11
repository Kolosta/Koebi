//! What every window of the application shares: the design system and its saved state, what
//! the editors show, the keymap, and what is saved.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use koebi_design::DesignSystem;
use koebi_design::persist::Store;
use koebi_input::{Context, InputSettings, Keymap};
use koebi_platform::{AppContext, AppError, exit_code};
use koebi_text::Fonts;
use koebi_tokens::semantic;
use koebi_ui::StyleCache;
use koebi_workspace::Workspace;

use crate::actions;
use crate::chrome::ShortcutLabels;
use crate::design_options;
use crate::editors::{self, Content, Editors};
use crate::language::LanguageSetting;
use crate::options::Options;
use crate::overlay::OverlaySwitch;
use crate::preferences::Preferences;

/// The interface shared by the windows.
pub(crate) struct Interface {
    pub(crate) design: DesignSystem,
    /// `None` without a place to save to: changes then last until the application quits.
    store: Option<Store>,
    /// The saved workspace and shortcuts, `None` without a place to save to.
    preferences: Option<Preferences>,
    pub(crate) styles: StyleCache,
    /// What the editors show: the gallery, the document, the messages.
    pub(crate) content: Content,
    /// Whether the frame-time overlay is shown.
    pub(crate) overlay: OverlaySwitch,
    /// The fonts text is laid out with; the renderers draw glyphs from the same ones.
    pub(crate) fonts: Fonts,
    /// How many times fonts were added since start-up: windows and renderers take the new
    /// collection when it changes.
    fonts_revision: u64,
    /// The text of the shortcuts the menus show.
    pub(crate) labels: ShortcutLabels,
    /// The keymap's revision the labels were read from.
    labels_revision: u64,
    /// Turns input events into actions, through the contexts under the pointer.
    pub(crate) keymap: Keymap,
    pub(crate) input_settings: InputSettings,
    /// Reused for the chain of contexts of each event.
    pub(crate) chain: Vec<Context>,
    /// The origin of the interface's time (`Input::set_time`).
    pub(crate) started: Instant,
    /// The language of the interface, and where it comes from.
    pub(crate) language: LanguageSetting,
}

impl Interface {
    /// Restores the saved design state, then applies the command line's design changes. Returns
    /// the interface and the workspace of the main window with its editors.
    pub(crate) fn new(
        context: &AppContext,
        options: &Options,
        fonts: Fonts,
    ) -> Result<(Self, Workspace, Editors), AppError> {
        let mut design = DesignSystem::new(&koebi_tokens::KOEBI).map_err(|error| {
            AppError::new(exit_code::FAILURE, format!("design system: {error}"))
        })?;
        let directory = data_directory(context, options);
        let mut store = directory
            .as_deref()
            .map(|directory| open_store(directory, &mut design));
        design_options::apply(&mut design, options)
            .map_err(|message| AppError::new(exit_code::USAGE, message))?;
        koebi_core::info!("{}", design_options::summary(&design));
        // Changes asked on the command line, an imported theme above all, are kept at once.
        if let Some(store) = &mut store {
            let result = store.save(&design);
            report_save(store, result);
        }
        design_options::export(&design, options)
            .map_err(|message| AppError::new(exit_code::FAILURE, message))?;
        let mut keymap = Keymap::new(&actions::GROUPS)
            .map_err(|error| AppError::new(exit_code::FAILURE, format!("shortcuts: {error}")))?;
        let mut preferences = directory
            .as_deref()
            .map(|directory| Preferences::new(directory, !options.untitled));
        let (workspace, editors) = preferences
            .as_mut()
            .and_then(Preferences::load_workspace)
            .unwrap_or_else(editors::default_workspace);
        if let Some(preferences) = &mut preferences {
            preferences.load_shortcuts(&mut keymap);
        }
        let language = LanguageSetting::new(
            directory.as_deref(),
            context.preferred_locales(),
            options.language.as_deref().and_then(koebi_i18n::find),
        );
        language.apply();
        let labels = ShortcutLabels::read(&keymap);
        let interface = Self {
            styles: StyleCache::new(&design),
            design,
            store,
            preferences,
            content: Content::new(),
            overlay: OverlaySwitch::new(options.shows_frame_times()),
            fonts,
            fonts_revision: 0,
            labels,
            labels_revision: keymap.revision(),
            keymap,
            input_settings: InputSettings::default(),
            chain: Vec::new(),
            started: Instant::now(),
            language,
        };
        Ok((interface, workspace, editors))
    }

    /// Reads the input settings from their tokens: a theme or the user may change them.
    pub(crate) fn update_input_settings(&mut self) {
        self.input_settings = InputSettings {
            drag_threshold: self.design.get(semantic::INPUT_DRAG_THRESHOLD),
            double_click_time: self.design.get(semantic::INPUT_DOUBLE_CLICK_TIME),
        };
    }

    /// Reads the menus' shortcut labels again once the keymap changed: the shortcut editor
    /// changes it.
    pub(crate) fn refresh_labels(&mut self) {
        if self.labels_revision != self.keymap.revision() {
            self.labels = ShortcutLabels::read(&self.keymap);
            self.labels_revision = self.keymap.revision();
        }
    }

    /// Shows or hides the frame-time overlay.
    pub(crate) fn toggle_frame_times(&mut self) {
        self.overlay.toggle();
    }

    /// The revision of what every window shows: the design system and the shared state drawn
    /// over it (the frame-time overlay). A window drawn at another revision draws again, even
    /// when the change came from another window. Both counts only grow, so their sum changes
    /// whenever either does.
    pub(crate) fn revision(&self) -> u64 {
        self.design.revision()
            + self.overlay.changes()
            + self.fonts_revision
            + koebi_i18n::revision()
    }

    /// Adds `font` to `family`, as a fallback for the characters other fonts lack when
    /// `fallback`; every window and renderer takes the new collection on its next frame.
    pub(crate) fn add_font(&mut self, family: &str, font: koebi_text::Font, fallback: bool) {
        let id = self.fonts.add(family, font);
        if fallback {
            self.fonts.add_fallback(id);
        }
        self.fonts_revision += 1;
    }

    /// The revision of the fonts ([`Self::add_font`]).
    pub(crate) fn fonts_revision(&self) -> u64 {
        self.fonts_revision
    }

    /// The time since the interface started: the time scale of `Input::set_time`.
    pub(crate) fn now(&self) -> Duration {
        self.started.elapsed()
    }

    /// `true` while changes wait for [`Self::save_if_due`].
    pub(crate) fn has_unsaved_changes(&self, workspace: &Workspace, editors: &Editors) -> bool {
        self.store
            .as_ref()
            .is_some_and(|store| store.is_dirty(&self.design))
            || self.preferences.as_ref().is_some_and(|preferences| {
                preferences.is_dirty(workspace, editors)
                    || preferences.shortcuts_changed(&self.keymap)
            })
    }

    /// Saves the design state once changes have settled: one write per pause while a field
    /// drags, not one per frame.
    pub(crate) fn save_if_due(&mut self, workspace: &Workspace, editors: &Editors) {
        if let Some(store) = &mut self.store {
            let result = store.save_if_due(&self.design, Instant::now());
            report_save(store, result);
        }
        if let Some(preferences) = &mut self.preferences {
            preferences.save_if_due(workspace, editors, Instant::now());
            preferences.save_shortcuts_if_changed(&self.keymap);
        }
    }

    /// Saves now if anything changed: before quitting or going to the background.
    pub(crate) fn save(&mut self, workspace: &Workspace, editors: &Editors) {
        if let Some(store) = &mut self.store {
            let result = store.save(&self.design);
            report_save(store, result);
        }
        if let Some(preferences) = &mut self.preferences {
            preferences.save(workspace, editors, &self.keymap);
        }
    }
}

/// Where the design state, the workspace and the shortcuts are saved: the command line's
/// directory, else the platform's preferences directory; `None` when there is none.
fn data_directory(context: &AppContext, options: &Options) -> Option<PathBuf> {
    match &options.data_dir {
        Some(directory) => Some(directory.clone()),
        None => match context.preferences_dir() {
            Ok(directory) => Some(directory),
            Err(error) => {
                koebi_core::warn!("no preferences directory ({error}): changes will not be saved");
                None
            }
        },
    }
}

/// Opens the store in `directory` and restores the saved state into `design`.
fn open_store(directory: &Path, design: &mut DesignSystem) -> Store {
    let mut store = Store::new(directory);
    match store.load(design) {
        Ok(Some(report)) => {
            for skipped in &report.skipped {
                koebi_core::warn!("design: {skipped}");
            }
            koebi_core::info!("design restored from {}", store.path().display());
        }
        Ok(None) => koebi_core::info!("design: nothing saved yet in {}", store.path().display()),
        Err(error) => koebi_core::warn!(
            "design: cannot restore {} ({error}); starting from the built-in themes",
            store.path().display()
        ),
    }
    store
}

fn report_save(store: &Store, result: koebi_design::Result<bool>) {
    match result {
        Ok(true) => koebi_core::info!("design saved to {}", store.path().display()),
        Ok(false) => {}
        Err(error) => koebi_core::warn!("cannot save the design: {error}"),
    }
}
