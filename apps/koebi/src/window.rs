//! A window of the application: its native window and what shows frames in it, its interface
//! state, its application bar and what it holds.
//!
//! Each window has its own frames: its input, its scene and its [`UiState`]. What they show comes
//! from the [`Interface`] all windows share.

use std::time::{Duration, Instant};

use koebi_core::{Rect, Vec2};
use koebi_platform::{AppContext, Event, Window, WindowEvent, WindowId};
use koebi_render::Scene;
use koebi_ui::{Input, Ui, UiState, WorkspaceView};
use koebi_workspace::Workspace;

use crate::actions;
use crate::chrome::{AppChrome, AppCommand, WindowView};
use crate::editors::{Editors, Host};
use crate::frame_times::FrameTimes;
use crate::gpu::WindowGpu;
use crate::interface::Interface;
use crate::settings::SettingsView;

/// What a window holds.
#[allow(
    clippy::large_enum_variant,
    reason = "one per window, and most hold editors: no gain in boxing them"
)]
pub(crate) enum WindowContent {
    /// Areas of editors.
    Workspace {
        workspace: Workspace,
        view: WorkspaceView,
        editors: Editors,
    },
    /// The settings.
    Settings(Box<SettingsView>),
}

impl WindowContent {
    /// A window of editors.
    pub(crate) fn workspace(workspace: Workspace, editors: Editors) -> Self {
        Self::Workspace {
            workspace,
            view: WorkspaceView::new(),
            editors,
        }
    }
}

/// What a window is to the application.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    /// A main window: the application quits when its last one closes, and its floating windows
    /// close with it.
    Main,
    /// A floating window of the main window `owner`, which it closes with. The application's
    /// own windows (the settings) belong to none: they stay while a main window does.
    Floating {
        /// The main window it belongs to.
        owner: Option<WindowId>,
    },
}

/// The windows that closing the window at `index` of `roles` closes, the last one first: it,
/// and the floating windows it owns when it is a main window. `None` when it is the last main
/// window: the application quits.
pub(crate) fn closed_with(roles: &[(WindowId, Role)], index: usize) -> Option<Vec<usize>> {
    let (id, role) = *roles.get(index)?;
    if role != Role::Main {
        return Some(vec![index]);
    }
    let mains = roles.iter().filter(|(_, role)| *role == Role::Main).count();
    if mains <= 1 {
        return None;
    }
    let owned = Role::Floating { owner: Some(id) };
    let mut closed: Vec<usize> = (0..roles.len())
        .filter(|&other| other == index || roles[other].1 == owned)
        .collect();
    closed.reverse();
    Some(closed)
}

/// A window of the application.
pub(crate) struct AppWindow {
    /// A main window, or a floating one and the main window it belongs to.
    pub(crate) role: Role,
    pub(crate) state: UiState,
    pub(crate) input: Input,
    pub(crate) scene: Scene,
    pub(crate) chrome: AppChrome,
    pub(crate) content: WindowContent,
    /// What the bar asked for in the last frame.
    pub(crate) command: Option<AppCommand>,
    /// The settings shown over the content, on platforms with one window.
    pub(crate) settings_in_place: Option<Box<SettingsView>>,
    pub(crate) needs_redraw: bool,
    /// The revision of the design system the last frame showed.
    pub(crate) drawn_revision: u64,
    /// While the pointer is locked, motion only moves what holds it: positions stay.
    pointer_locked: bool,
    /// Where the pointer comes back while it is locked.
    locked_at: Option<Vec2>,
    /// Where text is edited while text input is on.
    typing: Option<Rect>,
    // Field order is drop order: GPU objects (surface included) before the native window.
    pub(crate) gpu: WindowGpu,
    pub(crate) window: Window,
}

impl AppWindow {
    /// A window showing `content`, drawn from the next iteration.
    pub(crate) fn new(
        role: Role,
        window: Window,
        gpu: WindowGpu,
        content: WindowContent,
        interface: &Interface,
    ) -> Self {
        let mut state = UiState::new();
        state.set_fonts(interface.fonts.clone());
        Self {
            role,
            state,
            input: Input::new(),
            scene: Scene::new(),
            chrome: AppChrome::default(),
            content,
            command: None,
            settings_in_place: None,
            needs_redraw: true,
            drawn_revision: interface.design.revision(),
            pointer_locked: false,
            locked_at: None,
            typing: None,
            gpu,
            window,
        }
    }

    /// The window's identifier in events.
    pub(crate) fn id(&self) -> WindowId {
        self.window.id()
    }

    /// `true` for a main window.
    pub(crate) fn is_main(&self) -> bool {
        self.role == Role::Main
    }

    /// The main window this window belongs to: itself for a main window.
    pub(crate) fn owner(&self) -> Option<WindowId> {
        match self.role {
            Role::Main => Some(self.id()),
            Role::Floating { owner } => owner,
        }
    }

    /// Builds the scene of a frame covering `area`. When the frame changes the design system
    /// (a field edits a token, a card switches the theme), the scene is built again with the
    /// new styles: every component shows the change in this very frame.
    pub(crate) fn build(&mut self, interface: &mut Interface, area: Rect, times: &FrameTimes) {
        self.input.set_time(interface.now());
        if self.input.triggered(&actions::FRAME_TIMES) {
            interface.toggle_frame_times();
        }
        // Thresholds and delays are tokens: a theme or the user may change them.
        interface.update_input_settings();
        if self.input.triggered(&actions::QUIT) {
            self.command = Some(AppCommand::Quit);
        }
        if self.input.triggered(&actions::FULLSCREEN) {
            self.command = Some(AppCommand::ToggleFullscreen);
        }
        if self.input.triggered(&actions::SETTINGS) {
            self.command = Some(AppCommand::OpenSettings);
        }
        let revision = interface.design.revision();
        let input = self.input;
        self.draw(interface, area, &input, times);
        if interface.design.revision() != revision {
            self.draw(interface, area, &input.settled(), times);
        }
        self.input.end_frame();
        self.drawn_revision = interface.design.revision();
    }

    fn draw(&mut self, interface: &mut Interface, area: Rect, input: &Input, times: &FrameTimes) {
        let view = WindowView {
            title: self.title(),
            state: self.window.state(),
            has_menus: !matches!(self.content, WindowContent::Settings(_)),
            insets: self.window.chrome_insets(),
            maximize: self.window.maximize_button(),
        };
        let is_main = self.is_main();
        let Self {
            state,
            scene,
            chrome,
            content,
            command,
            settings_in_place,
            ..
        } = self;
        let Interface {
            design,
            styles,
            content: shown,
            overlay,
            fonts,
            labels,
            ..
        } = interface;
        scene.clear();
        let styles = styles.update(design);
        scene.set_clear_color(styles.background);
        let mut ui = Ui::new(scene, styles, input, state);
        shown.messages.collect();
        ui.zone(area, actions::APP, |ui| {
            let (rest, asked) = chrome.show(ui, area, view, labels);
            if asked.is_some() {
                *command = asked;
            }
            match (settings_in_place.as_mut(), content) {
                (Some(settings), _) | (None, WindowContent::Settings(settings)) => {
                    settings.show(ui, rest, design);
                }
                (
                    None,
                    WindowContent::Workspace {
                        workspace,
                        view,
                        editors,
                    },
                ) => {
                    let mut host = Host {
                        editors,
                        content: shown,
                        design,
                        window: area,
                    };
                    view.set_window(area);
                    ui.workspace(view, workspace, rest, &mut host);
                }
            }
        });
        drop(ui);
        if is_main && let Some(overlay) = overlay {
            overlay.draw(scene, design, fonts, times, area);
        }
    }

    /// Feeds an input event of this window to its interface, and to the keymap through the
    /// contexts under the pointer. Returns `true` when the event concerns them.
    pub(crate) fn handle(&mut self, interface: &mut Interface, event: &Event) -> bool {
        match *event {
            Event::MouseMotion {
                position,
                delta,
                modifiers,
                ..
            } => {
                if !self.pointer_locked {
                    self.input.pointer_moved(position);
                }
                self.input.pointer_moved_by(delta);
                self.input.set_modifiers(modifiers);
            }
            Event::TextInput { text, .. } => {
                self.input.typed(text.as_str());
                return true;
            }
            Event::MouseButton {
                button,
                down,
                position,
                modifiers,
                ..
            } => {
                if let Some(button) = actions::pointer_button(button) {
                    self.input.button(button, down, position);
                }
                self.input.set_modifiers(modifiers);
            }
            Event::MouseWheel {
                delta, modifiers, ..
            } => {
                self.input.scrolled(delta);
                self.input.set_modifiers(modifiers);
            }
            Event::Key { modifiers, .. } => self.input.set_modifiers(modifiers),
            Event::Window {
                event: WindowEvent::PointerLeft,
                ..
            } => {
                self.input.pointer_left();
                return true;
            }
            _ => {}
        }
        let Some((input_event, position)) = actions::input_event(event, self.input.pointer())
        else {
            return false;
        };
        self.state.chain_at(position, &mut interface.chain);
        let triggers = interface.keymap.handle(
            input_event,
            &interface.chain,
            &interface.input_settings,
            Instant::now(),
        );
        for triggered in triggers.iter() {
            self.input.trigger(triggered.action);
        }
        true
    }

    /// How long until the window wants to be drawn again without input, if it does: zero when it
    /// is due.
    pub(crate) fn redraw_in(&self, now: Duration) -> Option<Duration> {
        let at = self.state.redraw_at()?;
        Some(at.saturating_sub(now))
    }

    /// Applies what the last frame asked of the platform: a locked pointer (released where the
    /// interface says) and text input.
    pub(crate) fn apply_requests(&mut self, context: &mut AppContext) {
        match (self.locked_at, self.state.pointer_lock()) {
            (None, Some(_)) => {
                if let Err(error) = context.lock_pointer(&self.window, true) {
                    koebi_core::warn!("cannot lock the pointer: {error}");
                }
            }
            (Some(at), None) => {
                if let Err(error) = context.lock_pointer(&self.window, false) {
                    koebi_core::warn!("cannot release the pointer: {error}");
                }
                context.warp_pointer(&self.window, at);
                self.input.pointer_moved(at);
            }
            _ => {}
        }
        self.locked_at = self.state.pointer_lock();
        self.pointer_locked = self.locked_at.is_some();
        let area = self.state.text_input_area();
        match (self.typing, area) {
            (None, Some(_)) => {
                if let Err(error) = context.start_text_input(&self.window, area) {
                    koebi_core::warn!("cannot start text input: {error}");
                }
            }
            (Some(old), Some(new)) if old != new => {
                context.set_text_input_area(&self.window, area);
            }
            (Some(_), None) => context.stop_text_input(&self.window),
            _ => {}
        }
        self.typing = area;
        // The clipboard: what fields copied, and the text a field asked to paste, which it
        // inserts on the next frame (the key that asked already asks for that frame).
        if let Some(text) = self.state.copied_text()
            && let Err(error) = context.set_clipboard_text(text)
        {
            koebi_core::warn!("cannot copy to the clipboard: {error}");
        }
        if self.state.paste_requested()
            && let Some(text) = context.clipboard_text()
        {
            self.state.set_pasted_text(text);
        }
    }

    /// The workspace of the window and its editors, if it holds one.
    pub(crate) fn workspace(&self) -> Option<(&Workspace, &Editors)> {
        match &self.content {
            WindowContent::Workspace {
                workspace, editors, ..
            } => Some((workspace, editors)),
            WindowContent::Settings(_) => None,
        }
    }

    /// The title of the window.
    fn title(&self) -> &'static str {
        match self.content {
            WindowContent::Settings(_) => "Settings",
            WindowContent::Workspace { .. } => "Koebi",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: Role = Role::Main;

    fn of(owner: u32) -> Role {
        Role::Floating {
            owner: Some(WindowId(owner)),
        }
    }

    /// Windows 1 and 4 are main windows; 2 and 5 belong to 1, 6 to 4, 3 is the settings.
    fn roles() -> Vec<(WindowId, Role)> {
        [
            (1, MAIN),
            (2, of(1)),
            (3, Role::Floating { owner: None }),
            (4, MAIN),
            (5, of(1)),
            (6, of(4)),
        ]
        .into_iter()
        .map(|(id, role)| (WindowId(id), role))
        .collect()
    }

    #[test]
    fn a_floating_window_closes_alone() {
        assert_eq!(closed_with(&roles(), 1), Some(vec![1]));
        // The settings belong to no main window.
        assert_eq!(closed_with(&roles(), 2), Some(vec![2]));
    }

    #[test]
    fn a_main_window_closes_with_its_floating_windows_while_another_stays() {
        // The last one first: removing them in this order keeps the others' places.
        assert_eq!(closed_with(&roles(), 0), Some(vec![4, 1, 0]));
        assert_eq!(closed_with(&roles(), 3), Some(vec![5, 3]));
    }

    #[test]
    fn the_last_main_window_closes_the_application() {
        let roles = [
            (WindowId(1), MAIN),
            (WindowId(2), of(1)),
            (WindowId(3), Role::Floating { owner: None }),
        ];
        assert_eq!(closed_with(&roles, 0), None);
        assert_eq!(closed_with(&roles, 1), Some(vec![1]));
        assert_eq!(closed_with(&roles, 9), None);
    }
}
