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
pub(crate) static GROUPS: [&ActionGroup; 1] = [&GROUP];

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
