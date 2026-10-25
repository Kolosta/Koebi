//! Raw input in, gestures out: what the keymap matches shortcuts against.
//!
//! The application feeds [`InputEvent`]s (translated from the platform's events); the keymap
//! turns them into [`Gesture`]s (presses, releases, clicks, double clicks, drags, wheel and
//! pinch steps) and looks up the shortcuts that match.

use core::time::Duration;

use koebi_core::Vec2;
use koebi_core::input::{Key, Modifiers, MouseButton};

use crate::shortcut::{
    ButtonTrigger, DragDirection, KeyTrigger, PinchDirection, Shortcut, Trigger, WheelDirection,
};

/// Input preferences, from design tokens (`semantic.input.*`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputSettings {
    /// How far the pointer moves, in logical pixels, before a press becomes a drag.
    pub drag_threshold: f32,
    /// Longest time between the two presses of a key's double click (the platform counts
    /// mouse double clicks with the system's settings).
    pub double_click_time: Duration,
}

impl Default for InputSettings {
    fn default() -> Self {
        Self {
            drag_threshold: 4.0,
            double_click_time: Duration::from_millis(500),
        }
    }
}

/// Raw input, as the platform reports it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputEvent {
    /// A key went down or up.
    Key {
        /// The key.
        key: Key,
        /// `true` when it went down.
        down: bool,
        /// `true` for presses repeated while the key stays down.
        repeat: bool,
        /// Modifiers held.
        modifiers: Modifiers,
    },
    /// A mouse button went down or up.
    Button {
        /// The button.
        button: MouseButton,
        /// `true` when it went down.
        down: bool,
        /// Consecutive clicks counted by the platform (2 for a double click).
        clicks: u8,
        /// Pointer position in logical pixels.
        position: Vec2,
        /// Modifiers held.
        modifiers: Modifiers,
    },
    /// The pointer moved.
    PointerMove {
        /// New position in logical pixels.
        position: Vec2,
        /// Modifiers held.
        modifiers: Modifiers,
    },
    /// The wheel turned, or two fingers scrolled.
    Wheel {
        /// Notches (fractions for touchpads): positive `y` away from the user, positive `x`
        /// to the right.
        delta: Vec2,
        /// Pointer position in logical pixels.
        position: Vec2,
        /// Modifiers held.
        modifiers: Modifiers,
    },
    /// Two fingers pinched.
    Pinch {
        /// Scale change: above 1 zooms in.
        scale: f32,
        /// Pointer position in logical pixels.
        position: Vec2,
        /// Modifiers held.
        modifiers: Modifiers,
    },
    /// The window lost the keyboard focus: keys and buttons held are forgotten.
    FocusLost,
}

/// What the user did, in the terms of shortcuts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GestureKind {
    /// A key event.
    Key {
        /// The key.
        key: Key,
        /// What happened to it.
        when: KeyTrigger,
        /// `true` for a repeated press.
        repeat: bool,
    },
    /// A mouse button event; drags carry their actual direction and no distance.
    Button {
        /// The button.
        button: MouseButton,
        /// What happened to it.
        when: ButtonTrigger,
    },
    /// The wheel turned this much in this direction.
    Wheel {
        /// The direction.
        direction: WheelDirection,
        /// Notches, fractions for touchpads.
        amount: f32,
    },
    /// Fingers pinched by this factor.
    Pinch {
        /// The direction.
        direction: PinchDirection,
        /// The scale change.
        scale: f32,
    },
    /// The pointer moved.
    PointerMove,
}

/// A gesture with where and how it happened.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gesture {
    /// What happened.
    pub kind: GestureKind,
    /// The pointer position now.
    pub position: Vec2,
    /// Where the button or key went down: the start of a drag or a click.
    pub origin: Vec2,
    /// Modifiers held: for button gestures, those held when the button went down.
    pub modifiers: Modifiers,
}

impl Shortcut {
    /// `true` when `gesture` triggers this shortcut. Drags are matched separately, since
    /// their distance and direction are only known once the pointer moves.
    pub fn matches(&self, gesture: &Gesture) -> bool {
        let modifiers = match gesture.kind {
            // The modifier key that triggers does not count against the rule.
            GestureKind::Key { key, .. } if key.is_modifier() => {
                gesture.modifiers.without(modifier_of(key))
            }
            _ => gesture.modifiers,
        };
        if !self.modifiers.accepts(modifiers) {
            return false;
        }
        match (self.trigger, gesture.kind) {
            (
                Trigger::Key { key, when, repeat },
                GestureKind::Key {
                    key: pressed,
                    when: happened,
                    repeat: repeated,
                },
            ) => key == pressed && when == happened && (repeat || !repeated),
            (
                Trigger::Button { button, when },
                GestureKind::Button {
                    button: used,
                    when: happened,
                },
            ) => button == used && when == happened && !matches!(when, ButtonTrigger::Drag(_)),
            (
                Trigger::Wheel(direction),
                GestureKind::Wheel {
                    direction: turned, ..
                },
            ) => direction == turned,
            (
                Trigger::Pinch(direction),
                GestureKind::Pinch {
                    direction: pinched, ..
                },
            ) => direction == pinched,
            (Trigger::PointerMove, GestureKind::PointerMove) => true,
            _ => false,
        }
    }

    /// `true` when a press of `button` with `modifiers`, moved by `delta` logical pixels,
    /// starts this drag shortcut. `threshold` is the drag threshold of the settings.
    pub fn matches_drag(
        &self,
        button: MouseButton,
        modifiers: Modifiers,
        delta: Vec2,
        threshold: f32,
    ) -> bool {
        let Trigger::Button {
            button: bound,
            when: ButtonTrigger::Drag(drag),
        } = self.trigger
        else {
            return false;
        };
        let distance = drag.distance.map_or(threshold, f32::from);
        bound == button
            && self.modifiers.accepts(modifiers)
            && delta.length() >= distance
            && drag.direction.accepts(DragDirection::of(delta.x, delta.y))
    }
}

/// The modifier bit of a modifier key.
fn modifier_of(key: Key) -> Modifiers {
    match key {
        Key::LeftShift => Modifiers::LEFT_SHIFT,
        Key::RightShift => Modifiers::RIGHT_SHIFT,
        Key::LeftCtrl => Modifiers::LEFT_CTRL,
        Key::RightCtrl => Modifiers::RIGHT_CTRL,
        Key::LeftAlt => Modifiers::LEFT_ALT,
        Key::RightAlt => Modifiers::RIGHT_ALT,
        Key::LeftGui => Modifiers::LEFT_GUI,
        Key::RightGui => Modifiers::RIGHT_GUI,
        _ => Modifiers::NONE,
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::input::ModifierKey;

    use super::*;
    use crate::shortcut::Drag;

    fn gesture(kind: GestureKind, modifiers: Modifiers) -> Gesture {
        Gesture {
            kind,
            position: Vec2::ZERO,
            origin: Vec2::ZERO,
            modifiers,
        }
    }

    fn key(key: Key, when: KeyTrigger, repeat: bool) -> GestureKind {
        GestureKind::Key { key, when, repeat }
    }

    #[test]
    fn keys_match_with_their_modifiers() {
        let save = Shortcut::key(Key::S).with(ModifierKey::Ctrl);
        assert!(save.matches(&gesture(
            key(Key::S, KeyTrigger::Press, false),
            Modifiers::LEFT_CTRL
        )));
        assert!(!save.matches(&gesture(
            key(Key::S, KeyTrigger::Press, false),
            Modifiers::NONE
        )));
        assert!(!save.matches(&gesture(
            key(Key::S, KeyTrigger::Release, false),
            Modifiers::LEFT_CTRL
        )));
    }

    #[test]
    fn repeats_only_match_repeating_shortcuts() {
        let step = Shortcut::key(Key::Right);
        let repeating = Shortcut::new(Trigger::Key {
            key: Key::Right,
            when: KeyTrigger::Press,
            repeat: true,
        });
        let repeated = gesture(key(Key::Right, KeyTrigger::Press, true), Modifiers::NONE);
        assert!(!step.matches(&repeated));
        assert!(repeating.matches(&repeated));
        assert!(repeating.matches(&gesture(
            key(Key::Right, KeyTrigger::Press, false),
            Modifiers::NONE
        )));
    }

    #[test]
    fn a_modifier_key_triggers_without_counting_itself() {
        let alone = Shortcut::key(Key::LeftCtrl);
        assert!(alone.matches(&gesture(
            key(Key::LeftCtrl, KeyTrigger::Press, false),
            Modifiers::LEFT_CTRL
        )));
        assert!(!alone.matches(&gesture(
            key(Key::LeftCtrl, KeyTrigger::Press, false),
            Modifiers::LEFT_CTRL.union(Modifiers::LEFT_SHIFT)
        )));
    }

    #[test]
    fn drags_need_distance_direction_and_modifiers() {
        let north = Shortcut::button(
            MouseButton::Left,
            ButtonTrigger::Drag(Drag {
                direction: DragDirection::North,
                distance: Some(10),
            }),
        );
        let up = Vec2::new(0.0, -12.0);
        assert!(north.matches_drag(MouseButton::Left, Modifiers::NONE, up, 4.0));
        assert!(!north.matches_drag(
            MouseButton::Left,
            Modifiers::NONE,
            Vec2::new(0.0, -8.0),
            4.0
        ));
        assert!(!north.matches_drag(
            MouseButton::Left,
            Modifiers::NONE,
            Vec2::new(12.0, 0.0),
            4.0
        ));
        assert!(!north.matches_drag(MouseButton::Right, Modifiers::NONE, up, 4.0));
        assert!(!north.matches_drag(MouseButton::Left, Modifiers::LEFT_SHIFT, up, 4.0));
        let any = Shortcut::button(MouseButton::Left, ButtonTrigger::Drag(Drag::ANY));
        assert!(any.matches_drag(MouseButton::Left, Modifiers::NONE, Vec2::new(3.0, 3.0), 4.0));
        // Drags never match as plain gestures.
        assert!(!any.matches(&gesture(
            GestureKind::Button {
                button: MouseButton::Left,
                when: ButtonTrigger::Drag(Drag::ANY),
            },
            Modifiers::NONE
        )));
    }

    #[test]
    fn wheel_pinch_and_moves_match_their_direction() {
        let up = Shortcut::new(Trigger::Wheel(WheelDirection::Up));
        let turned = |direction| GestureKind::Wheel {
            direction,
            amount: 1.0,
        };
        assert!(up.matches(&gesture(turned(WheelDirection::Up), Modifiers::NONE)));
        assert!(!up.matches(&gesture(turned(WheelDirection::Down), Modifiers::NONE)));
        let zoom = Shortcut::new(Trigger::Pinch(PinchDirection::In));
        assert!(zoom.matches(&gesture(
            GestureKind::Pinch {
                direction: PinchDirection::In,
                scale: 1.1,
            },
            Modifiers::NONE
        )));
        assert!(
            Shortcut::new(Trigger::PointerMove)
                .matches(&gesture(GestureKind::PointerMove, Modifiers::NONE))
        );
    }
}
