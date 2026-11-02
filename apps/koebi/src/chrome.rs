//! The application bar of a window: its menus, and what they ask of the application.
//!
//! The bar is drawn with the frame; what it asks for is applied after the frame
//! ([`AppCommand`]), since changing a window sends events and may draw again. After each frame,
//! the system learns where the bar is and what lies on it ([`AppChrome::update_regions`]).

use koebi_core::{Rect, Vec2};
use koebi_input::Keymap;
use koebi_platform::{ChromeInsets, Window, WindowRegions, WindowState};
use koebi_ui::{
    AppBar, BarLayout, MenuBarEntry, MenuBarState, MenuChoice, MenuItem, Ui, UiState, WidgetId,
    WindowCommand,
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
    /// What the system's own title bar takes.
    pub(crate) insets: ChromeInsets,
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
    /// Opens a floating window.
    NewWindow,
}

/// The text of the shortcuts the menus show, read when the keymap changes rather than at every
/// frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ShortcutLabels {
    quit: String,
    fullscreen: String,
}

impl ShortcutLabels {
    /// The first shortcut of each action, as `keymap` holds them.
    pub(crate) fn read(keymap: &Keymap) -> Self {
        let first = |id: &str| {
            keymap
                .shortcuts(id)
                .and_then(<[_]>::first)
                .map(ToString::to_string)
                .unwrap_or_default()
        };
        Self {
            quit: first(actions::QUIT.id),
            fullscreen: first(actions::FULLSCREEN.id),
        }
    }
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
            .filled(filled)
            .window_buttons(view.insets.draws_buttons)
            .leading_inset(view.insets.leading)
            .show(ui, bar, |ui, layout| {
                self.menus_of(ui, layout, area, view.state, labels)
            });
        self.bar = Some((bar, shown.drag));
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
            MenuItem::action("New").enabled(false),
            MenuItem::action("Open").enabled(false),
            MenuItem::action("Save").enabled(false),
            MenuItem::action("Save As").enabled(false),
            MenuItem::Separator,
            with_shortcut(MenuItem::action("Quit"), &labels.quit),
        ];
        let edit = [MenuItem::action("Settings").enabled(false)];
        let fullscreen = MenuItem::action("Fullscreen").checked(state == WindowState::Fullscreen);
        let window = [
            MenuItem::action("New Window").enabled(MULTIPLE_WINDOWS),
            with_shortcut(fullscreen, &labels.fullscreen),
        ];
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
        command_of(menu, &choice)
    }

    /// Tells `window` where the bar was drawn and what lies on it, according to the frame `state`
    /// just ended.
    pub(crate) fn update_regions(&mut self, state: &UiState, window: &mut Window) {
        let Some((bar, drag)) = self.bar else {
            return;
        };
        state.covering(drag, &mut self.covered);
        window.set_regions(&WindowRegions {
            drag: Some(bar),
            blocked: &self.covered,
            resize_border: self.resize_border,
        });
    }
}

/// What the item `choice` of the menu at `menu` on the bar asks for.
fn command_of(menu: usize, choice: &MenuChoice) -> Option<AppCommand> {
    match (menu, choice.index()) {
        (0, 5) => Some(AppCommand::Quit),
        (2, 0) => Some(AppCommand::NewWindow),
        (2, 1) => Some(AppCommand::ToggleFullscreen),
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
        assert!(labels.quit.ends_with('Q'), "{}", labels.quit);
        // With no shortcut left, the item shows none.
        keymap.set_shortcuts(actions::FULLSCREEN.id, &[]).unwrap();
        assert_eq!(ShortcutLabels::read(&keymap).fullscreen, "");
    }

    #[test]
    fn items_without_a_shortcut_show_none() {
        let plain = MenuItem::action("Quit");
        assert_eq!(with_shortcut(plain, ""), plain);
        assert_ne!(with_shortcut(plain, "Ctrl+Q"), plain);
    }
}
