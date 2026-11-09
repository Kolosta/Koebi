//! The application bar of a window: its menus, and what they ask of the application.
//!
//! The bar is drawn with the frame; what it asks for is applied after the frame
//! ([`AppCommand`]), since changing a window sends events and may draw again. After each frame,
//! the system learns where the bar is and what lies on it ([`AppChrome::update_regions`]).

use koebi_core::{Rect, Vec2};
use koebi_input::Keymap;
use koebi_platform::{ChromeInsets, MaximizeButtonState, Window, WindowRegions, WindowState};
use koebi_ui::{
    AppBar, BarLayout, MenuBarEntry, MenuBarState, MenuItem, Ui, UiState, WidgetId, WindowCommand,
};

use crate::actions;

/// Mobile platforms show one window.
const MULTIPLE_WINDOWS: bool = !cfg!(any(target_os = "android", target_os = "ios"));

const BAR: WidgetId = WidgetId::new("app-bar");
const MENUS: WidgetId = WidgetId::new("app-bar-menus");

/// What a window shows of itself on its bar this frame.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WindowView<'a> {
    /// Its title.
    pub(crate) title: &'a str,
    /// What it shows as.
    pub(crate) state: WindowState,
    /// Whether its bar holds the application's menus.
    pub(crate) has_menus: bool,
    /// What the system's own title bar takes.
    pub(crate) insets: ChromeInsets,
    /// What the system reports of the pointer on the maximize button.
    pub(crate) maximize: MaximizeButtonState,
}

/// What the bar asks of the application, applied after the frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum AppCommand {
    /// A window button.
    Window(WindowCommand),
    /// Enters or leaves fullscreen.
    ToggleFullscreen,
    /// Quits.
    Quit,
    /// Opens the system's window menu there.
    SystemMenu(Vec2),
    /// Opens a floating window of the same document, which closes with its main window.
    NewWindow,
    /// Opens another main window of the same document.
    NewMainWindow,
    /// Starts another instance of the application, on a document of its own.
    NewInstance,
    /// Opens the settings window, or brings it to the front.
    OpenSettings,
}

/// The text of the shortcuts the menus show, read when the keymap changes rather than at every
/// frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ShortcutLabels {
    quit: String,
    fullscreen: String,
    settings: String,
}

impl ShortcutLabels {
    /// The first shortcut of each action, as `keymap` holds them.
    pub(crate) fn read(keymap: &Keymap) -> Self {
        let first = |id: &str| {
            keymap
                .shortcuts(id)
                .and_then(<[_]>::first)
                .map_or_else(String::new, |shortcut| readable(&shortcut.to_string()))
        };
        Self {
            quit: first(actions::QUIT.id),
            fullscreen: first(actions::FULLSCREEN.id),
            settings: first(actions::SETTINGS.id),
        }
    }
}

/// The text of a shortcut as a menu shows it: punctuation as itself rather than by its name.
fn readable(shortcut: &str) -> String {
    const SIGNS: [(&str, &str); 8] = [
        ("Comma", ","),
        ("Period", "."),
        ("Slash", "/"),
        ("Semicolon", ";"),
        ("Apostrophe", "'"),
        ("Minus", "-"),
        ("Equals", "="),
        ("Grave", "`"),
    ];
    SIGNS
        .iter()
        .find_map(|(name, sign)| {
            shortcut
                .strip_suffix(name)
                .map(|rest| format!("{rest}{sign}"))
        })
        .unwrap_or_else(|| shortcut.to_owned())
}

/// `item` showing `shortcut`, when it has one.
const fn with_shortcut<'a>(item: MenuItem<'a>, shortcut: &'a str) -> MenuItem<'a> {
    if shortcut.is_empty() {
        item
    } else {
        item.shortcut(shortcut)
    }
}

/// The application bar of one window, kept between frames.
#[derive(Clone, Debug, Default)]
pub(crate) struct AppChrome {
    menus: MenuBarState,
    /// Where the bar lies and what its background is, from the last frame.
    bar: Option<(Rect, WidgetId)>,
    /// The maximize button, from the last frame.
    maximize: Option<(WidgetId, Rect)>,
    /// What lies on the bar: reused from frame to frame.
    covered: Vec<Rect>,
    resize_border: f32,
}

impl AppChrome {
    /// Draws the bar at the top of `area`; returns what the frame has left below it, and what the
    /// bar asks for.
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui<'_>,
        area: Rect,
        view: WindowView<'_>,
        labels: &ShortcutLabels,
    ) -> (Rect, Option<AppCommand>) {
        let style = ui.styles().app_bar;
        let height = style.height().max(view.insets.min_height);
        let bar = Rect::from_min_max(area.min, Vec2::new(area.max.x, area.min.y + height));
        let filled = matches!(view.state, WindowState::Maximized | WindowState::Fullscreen);
        let shown = AppBar::new(BAR, view.title)
            .logo(koebi_icons::app::ICON)
            .filled(filled)
            .window_buttons(view.insets.draws_buttons)
            .leading_inset(view.insets.leading)
            .maximize_state(view.maximize.hovered, view.maximize.pressed)
            .show(ui, bar, |ui, layout| {
                if view.has_menus {
                    self.menus_of(ui, layout, area, view.state, labels)
                } else {
                    None
                }
            });
        self.bar = Some((bar, shown.drag));
        self.maximize = shown.maximize;
        self.resize_border = style.resize_border;
        let command = shown
            .command
            .map(AppCommand::Window)
            .or(shown.items)
            .or(shown.system_menu.map(AppCommand::SystemMenu));
        let rest = Rect::from_min_max(Vec2::new(area.min.x, bar.max.y), area.max);
        (rest, command)
    }

    /// The menus of the bar; returns what the item chosen asks for.
    fn menus_of(
        &mut self,
        ui: &mut Ui<'_>,
        layout: &mut BarLayout,
        area: Rect,
        state: WindowState,
        labels: &ShortcutLabels,
    ) -> Option<AppCommand> {
        // The document model comes later: its commands show what the menu will hold.
        let file = [
            MenuItem::action("New")
                .icon(koebi_icons::actions::NEW)
                .enabled(false),
            MenuItem::action("Open")
                .icon(koebi_icons::actions::OPEN)
                .enabled(false),
            MenuItem::action("Save")
                .icon(koebi_icons::actions::SAVE)
                .enabled(false),
            MenuItem::action("Save As")
                .icon(koebi_icons::actions::SAVE_AS)
                .enabled(false),
            MenuItem::Separator,
            with_shortcut(
                MenuItem::action("Quit").icon(koebi_icons::actions::QUIT),
                &labels.quit,
            ),
        ];
        let edit = [with_shortcut(
            MenuItem::action("Settings").icon(koebi_icons::actions::SETTINGS),
            &labels.settings,
        )];
        let window = window_items(state, labels);
        let entries = [
            MenuBarEntry {
                title: "File",
                items: &file,
            },
            MenuBarEntry {
                title: "Edit",
                items: &edit,
            },
            MenuBarEntry {
                title: "Window",
                items: &window,
            },
        ];
        let (menu, choice) = ui.menu_bar(MENUS, &mut self.menus, &entries, layout, area)?;
        command_of(menu, choice.index())
    }

    /// Tells `window` where the bar was drawn and what lies on it, according to the frame `state`
    /// just ended.
    pub(crate) fn update_regions(&mut self, state: &UiState, window: &mut Window) {
        let Some((bar, drag)) = self.bar else {
            return;
        };
        // The system gets the maximize button only when nothing lies over it: an open menu takes
        // every press.
        let maximize = self.maximize.and_then(|(button, rect)| {
            state.covering(button, &mut self.covered);
            self.covered.is_empty().then_some(rect)
        });
        state.covering(drag, &mut self.covered);
        window.set_regions(&WindowRegions {
            drag: Some(bar),
            blocked: &self.covered,
            resize_border: self.resize_border,
            maximize,
        });
    }
}

/// The Window menu: windows of the same document, another instance, fullscreen.
fn window_items(state: WindowState, labels: &ShortcutLabels) -> [MenuItem<'_>; 5] {
    let fullscreen = MenuItem::action("Fullscreen")
        .icon(koebi_icons::view::FULLSCREEN)
        .checked(state == WindowState::Fullscreen);
    [
        MenuItem::action("New Window")
            .icon(koebi_icons::actions::NEW_WINDOW)
            .enabled(MULTIPLE_WINDOWS),
        MenuItem::action("New Main Window")
            .icon(koebi_icons::actions::NEW_MAIN_WINDOW)
            .enabled(MULTIPLE_WINDOWS),
        MenuItem::action("New Instance")
            .icon(koebi_icons::actions::NEW_INSTANCE)
            .enabled(MULTIPLE_WINDOWS),
        MenuItem::Separator,
        with_shortcut(fullscreen, &labels.fullscreen),
    ]
}

/// What the item at `index` of the menu at `menu` on the bar asks for.
fn command_of(menu: usize, index: usize) -> Option<AppCommand> {
    match (menu, index) {
        (0, 5) => Some(AppCommand::Quit),
        (1, 0) => Some(AppCommand::OpenSettings),
        (2, 0) => Some(AppCommand::NewWindow),
        (2, 1) => Some(AppCommand::NewMainWindow),
        (2, 2) => Some(AppCommand::NewInstance),
        (2, 4) => Some(AppCommand::ToggleFullscreen),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use koebi_input::Keymap;

    use super::*;

    #[test]
    fn the_menus_show_the_shortcuts_of_the_keymap() {
        let mut keymap = Keymap::new(&actions::GROUPS).unwrap();
        let labels = ShortcutLabels::read(&keymap);
        assert_eq!(labels.fullscreen, "F11");
        assert!(labels.settings.ends_with(','), "{}", labels.settings);
        assert!(labels.quit.ends_with('Q'), "{}", labels.quit);
        // With no shortcut left, the item shows none.
        keymap.set_shortcuts(actions::FULLSCREEN.id, &[]).unwrap();
        assert_eq!(ShortcutLabels::read(&keymap).fullscreen, "");
    }

    #[test]
    fn the_window_menu_opens_windows_of_the_document_and_another_instance() {
        let labels = ShortcutLabels::default();
        let items = window_items(WindowState::Normal, &labels);
        let names: Vec<Option<&str>> = items
            .iter()
            .map(|item| match item {
                MenuItem::Action(action) => Some(action.label),
                _ => None,
            })
            .collect();
        let expected = [
            ("New Window", AppCommand::NewWindow),
            ("New Main Window", AppCommand::NewMainWindow),
            ("New Instance", AppCommand::NewInstance),
            ("Fullscreen", AppCommand::ToggleFullscreen),
        ];
        for (name, command) in expected {
            let index = names
                .iter()
                .position(|label| *label == Some(name))
                .unwrap_or_else(|| panic!("no {name}"));
            assert_eq!(command_of(2, index), Some(command), "{name}");
        }
        // The separator asks for nothing.
        assert_eq!(command_of(2, 3), None);
    }

    #[test]
    fn punctuation_shows_as_itself() {
        assert_eq!(readable("Ctrl+Comma"), "Ctrl+,");
        assert_eq!(readable("Period"), ".");
        assert_eq!(readable("Ctrl+Q"), "Ctrl+Q");
        assert_eq!(readable("F11"), "F11");
    }

    #[test]
    fn items_without_a_shortcut_show_none() {
        let plain = MenuItem::action("Quit");
        assert_eq!(with_shortcut(plain, ""), plain);
        assert_ne!(with_shortcut(plain, "Ctrl+Q"), plain);
    }
}
