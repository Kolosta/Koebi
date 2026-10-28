//! The application's own actions, and the translation of platform events for the keymap.

use koebi_core::Vec2;
use koebi_core::input::Key;
use koebi_input::{ActionDef, ActionGroup, Context, InputEvent, Shortcut};
use koebi_platform::{Event, GesturePhase, MouseButton, WindowEvent};
use koebi_ui::PointerButton;

/// The whole window: actions available wherever the pointer is.
pub(crate) const APP: Context = Context::new("app");

/// Shows or hides the frame-time overlay.
pub(crate) static FRAME_TIMES: ActionDef = ActionDef {
    id: "app.frame-times",
    label: "Frame times",
    description: "Shows or hides the frame rate and the CPU and GPU time of a frame.",
    context: APP,
    defaults: &[Shortcut::key(Key::F12)],
};

/// The application's actions.
pub(crate) static GROUP: ActionGroup = ActionGroup {
    name: "Application",
    actions: &[&FRAME_TIMES],
};

/// Every action group of the application, for the keymap.
pub(crate) static GROUPS: [&ActionGroup; 10] = [
    &GROUP,
    &koebi_ui::MENU_ACTIONS,
    &koebi_ui::TEXT_ACTIONS,
    &koebi_ui::NUMBER_ACTIONS,
    &koebi_ui::WORKSPACE_ACTIONS,
    &koebi_ui::LIST_ACTIONS,
    &crate::editors::viewport::ACTIONS,
    &crate::editors::outliner::ACTIONS,
    &crate::editors::info::ACTIONS,
    &crate::editors::timeline::ACTIONS,
];

/// The interface's button for a mouse button, if components react to it.
pub(crate) fn pointer_button(button: MouseButton) -> Option<PointerButton> {
    match button {
        MouseButton::Left => Some(PointerButton::Primary),
        MouseButton::Right => Some(PointerButton::Secondary),
        MouseButton::Middle => Some(PointerButton::Middle),
        _ => None,
    }
}

/// `event` for the keymap, and where it happened; `pointer` stands in for events without a
/// position. `None` for events shortcuts never see.
pub(crate) fn input_event(
    event: &Event,
    pointer: Option<Vec2>,
) -> Option<(InputEvent, Option<Vec2>)> {
    Some(match *event {
        Event::Key {
            key,
            down,
            repeat,
            modifiers,
            ..
        } => (
            InputEvent::Key {
                key,
                down,
                repeat,
                modifiers,
            },
            pointer,
        ),
        Event::MouseMotion {
            position,
            modifiers,
            ..
        } => (
            InputEvent::PointerMove {
                position,
                modifiers,
            },
            Some(position),
        ),
        Event::MouseButton {
            button,
            down,
            clicks,
            position,
            modifiers,
            ..
        } => (
            InputEvent::Button {
                button,
                down,
                clicks,
                position,
                modifiers,
            },
            Some(position),
        ),
        Event::MouseWheel {
            delta,
            position,
            modifiers,
            ..
        } => (
            InputEvent::Wheel {
                delta,
                position,
                modifiers,
            },
            Some(position),
        ),
        Event::Pinch {
            phase: GesturePhase::Update,
            scale,
            ..
        } => {
            let position = pointer.unwrap_or_default();
            let modifiers = koebi_core::input::Modifiers::default();
            (
                InputEvent::Pinch {
                    scale,
                    position,
                    modifiers,
                },
                Some(position),
            )
        }
        Event::Window {
            event: WindowEvent::FocusLost,
            ..
        } => (InputEvent::FocusLost, None),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use koebi_core::input::Modifiers;
    use koebi_input::Keymap;
    use koebi_platform::WindowId;

    use super::*;

    #[test]
    fn the_application_keymap_has_no_conflict() {
        let keymap = Keymap::new(&GROUPS).expect("the default shortcuts do not conflict");
        assert_eq!(keymap.action("app.frame-times"), Some(&FRAME_TIMES));
    }

    #[test]
    fn f12_shows_the_frame_times_wherever_the_pointer_is() {
        let mut keymap = Keymap::new(&GROUPS).expect("the default shortcuts do not conflict");
        let press = InputEvent::Key {
            key: Key::F12,
            down: true,
            repeat: false,
            modifiers: Modifiers::default(),
        };
        let settings = koebi_input::InputSettings::default();
        let now = std::time::Instant::now();
        let triggers = keymap.handle(press, &[APP], &settings, now);
        let actions: Vec<_> = triggers.iter().map(|t| t.action.id).collect();
        assert_eq!(actions, ["app.frame-times"]);
    }

    #[test]
    fn list_keys_apply_over_lists_and_during_their_drags() {
        let mut keymap = Keymap::new(&GROUPS).expect("the default shortcuts do not conflict");
        let settings = koebi_input::InputSettings::default();
        let now = std::time::Instant::now();
        let mut actions = |key: Key, modifiers: Modifiers, chain: &[Context]| -> Vec<&str> {
            let press = InputEvent::Key {
                key,
                down: true,
                repeat: false,
                modifiers,
            };
            let triggers = keymap.handle(press, chain, &settings, now);
            triggers.iter().map(|trigger| trigger.action.id).collect()
        };
        // The chain under the pointer over a list: its scroll area, the list, the window.
        let over_list = [koebi_ui::SCROLL, koebi_ui::LIST, APP];
        assert_eq!(
            actions(Key::Down, Modifiers::NONE, &over_list),
            ["list.next"]
        );
        assert_eq!(
            actions(Key::Down, Modifiers::LEFT_SHIFT, &over_list),
            ["list.extend-next"]
        );
        assert_eq!(
            actions(Key::Down, Modifiers::NONE, &[APP]),
            Vec::<&str>::new()
        );
        // A drag grabs the keyboard: Escape cancels it wherever the pointer is.
        let dragging = [koebi_ui::LIST_DRAG, APP];
        assert_eq!(
            actions(Key::Escape, Modifiers::NONE, &dragging),
            ["list.cancel-drag"]
        );
    }

    #[test]
    fn platform_events_reach_the_keymap_with_their_position() {
        let window = WindowId(1);
        let at = Vec2::new(3.0, 4.0);
        let key = Event::Key {
            window,
            key: Key::F12,
            scancode: 0,
            down: true,
            repeat: false,
            modifiers: Modifiers::LEFT_CTRL,
        };
        let (translated, position) = input_event(&key, Some(at)).unwrap();
        assert_eq!(
            translated,
            InputEvent::Key {
                key: Key::F12,
                down: true,
                repeat: false,
                modifiers: Modifiers::LEFT_CTRL,
            }
        );
        // Keys happen where the pointer is.
        assert_eq!(position, Some(at));
        let button = Event::MouseButton {
            window,
            button: MouseButton::Right,
            down: true,
            clicks: 1,
            position: at,
            modifiers: Modifiers::default(),
        };
        assert_eq!(input_event(&button, None).unwrap().1, Some(at));
        assert_eq!(
            pointer_button(MouseButton::Right),
            Some(PointerButton::Secondary)
        );
        assert_eq!(pointer_button(MouseButton::Back), None);
        let begin = Event::Pinch {
            window,
            phase: GesturePhase::Begin,
            scale: 1.0,
        };
        assert_eq!(input_event(&begin, Some(at)), None);
        assert_eq!(input_event(&Event::Quit, Some(at)), None);
    }
}
