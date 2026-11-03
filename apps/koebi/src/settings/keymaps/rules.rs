//! The rules of editing a shortcut, as pure functions: its device and what changing it keeps,
//! its event and what follows (only presses repeat, drags keep their direction), the modifiers
//! by family and by side (both sides mean either key, one side never goes alone), and the
//! lists the editor offers.

use koebi_core::input::{Key, ModifierKey, MouseButton};
use koebi_input::{
    ButtonTrigger, Drag, DragDirection, KeyTrigger, ModifierRule, ModifierState, PinchDirection,
    Side, Trigger, WheelDirection,
};

/// Where a shortcut comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Device {
    Keyboard,
    Mouse,
    Wheel,
    Touchpad,
    Pointer,
}

impl Device {
    pub(super) const ALL: [Self; 5] = [
        Self::Keyboard,
        Self::Mouse,
        Self::Wheel,
        Self::Touchpad,
        Self::Pointer,
    ];

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Keyboard => "Keyboard",
            Self::Mouse => "Mouse",
            Self::Wheel => "Wheel",
            Self::Touchpad => "Touchpad",
            Self::Pointer => "Pointer move",
        }
    }

    pub(super) const fn of(trigger: Trigger) -> Self {
        match trigger {
            Trigger::Key { .. } => Self::Keyboard,
            Trigger::Button { .. } => Self::Mouse,
            Trigger::Wheel(_) => Self::Wheel,
            Trigger::Pinch(_) => Self::Touchpad,
            Trigger::PointerMove => Self::Pointer,
        }
    }

    /// A trigger of this device: `from` when it is one already, else its first input.
    pub(super) const fn trigger(self, from: Trigger) -> Trigger {
        if matches!(
            (self, from),
            (Self::Keyboard, Trigger::Key { .. })
                | (Self::Mouse, Trigger::Button { .. })
                | (Self::Wheel, Trigger::Wheel(_))
                | (Self::Touchpad, Trigger::Pinch(_))
                | (Self::Pointer, Trigger::PointerMove)
        ) {
            return from;
        }
        match self {
            Self::Keyboard => Trigger::key(Key::A),
            Self::Mouse => Trigger::button(MouseButton::Left),
            Self::Wheel => Trigger::Wheel(WheelDirection::Up),
            Self::Touchpad => Trigger::Pinch(PinchDirection::In),
            Self::Pointer => Trigger::PointerMove,
        }
    }
}

/// When a key or a button triggers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Event {
    Press,
    Release,
    Click,
    DoubleClick,
    Drag,
}

impl Event {
    const KEYS: [Self; 4] = [Self::Press, Self::Release, Self::Click, Self::DoubleClick];
    pub(super) const BUTTONS: [Self; 5] = [
        Self::Press,
        Self::Release,
        Self::Click,
        Self::DoubleClick,
        Self::Drag,
    ];

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Press => "Press",
            Self::Release => "Release",
            Self::Click => "Click",
            Self::DoubleClick => "Double click",
            Self::Drag => "Click and drag",
        }
    }

    /// The events `device` has: keys and buttons only.
    pub(super) const fn of_device(device: Device) -> &'static [Self] {
        match device {
            Device::Keyboard => &Self::KEYS,
            Device::Mouse => &Self::BUTTONS,
            Device::Wheel | Device::Touchpad | Device::Pointer => &[],
        }
    }

    pub(super) const fn of(trigger: Trigger) -> Option<Self> {
        match trigger {
            Trigger::Key { when, .. } => Some(match when {
                KeyTrigger::Press => Self::Press,
                KeyTrigger::Release => Self::Release,
                KeyTrigger::Click => Self::Click,
                KeyTrigger::DoubleClick => Self::DoubleClick,
            }),
            Trigger::Button { when, .. } => Some(match when {
                ButtonTrigger::Press => Self::Press,
                ButtonTrigger::Release => Self::Release,
                ButtonTrigger::Click => Self::Click,
                ButtonTrigger::DoubleClick => Self::DoubleClick,
                ButtonTrigger::Drag(_) => Self::Drag,
            }),
            Trigger::Wheel(_) | Trigger::Pinch(_) | Trigger::PointerMove => None,
        }
    }

    /// `trigger` happening on this event instead: a key repeats on presses only.
    pub(super) const fn apply(self, trigger: Trigger) -> Trigger {
        match trigger {
            Trigger::Key { key, repeat, .. } => {
                let when = match self {
                    Self::Press | Self::Drag => KeyTrigger::Press,
                    Self::Release => KeyTrigger::Release,
                    Self::Click => KeyTrigger::Click,
                    Self::DoubleClick => KeyTrigger::DoubleClick,
                };
                Trigger::Key {
                    key,
                    when,
                    repeat: repeat && matches!(when, KeyTrigger::Press),
                }
            }
            Trigger::Button { button, when } => Trigger::Button {
                button,
                when: match (self, when) {
                    (Self::Press, _) => ButtonTrigger::Press,
                    (Self::Release, _) => ButtonTrigger::Release,
                    (Self::Click, _) => ButtonTrigger::Click,
                    (Self::DoubleClick, _) => ButtonTrigger::DoubleClick,
                    (Self::Drag, ButtonTrigger::Drag(drag)) => ButtonTrigger::Drag(drag),
                    (Self::Drag, _) => ButtonTrigger::Drag(Drag::ANY),
                },
            },
            other => other,
        }
    }
}

/// `true` when `trigger` can repeat while held: a key on its press.
pub(super) const fn can_repeat(trigger: Trigger) -> bool {
    matches!(
        trigger,
        Trigger::Key {
            when: KeyTrigger::Press,
            ..
        }
    )
}

/// `rule` with `family` switched on (either side) or off. Any modifier: it takes that family
/// alone.
pub(super) fn toggle_family(rule: ModifierRule, family: ModifierKey) -> ModifierRule {
    let rule = if is_any(rule) {
        ModifierRule::NONE
    } else {
        rule
    };
    let state = match rule.get(family) {
        ModifierState::Down(_) => ModifierState::Up,
        ModifierState::Up | ModifierState::Any => ModifierState::Down(Side::Either),
    };
    rule.with(family, state)
}

/// `rule` with the `side` of `family` switched: both sides on mean either key; one side cannot
/// be switched off alone. Nothing changes for a family not held.
pub(super) fn toggle_side(rule: ModifierRule, family: ModifierKey, side: Side) -> ModifierRule {
    let other = match side {
        Side::Left => Side::Right,
        Side::Right | Side::Either => Side::Left,
    };
    let state = match rule.get(family) {
        ModifierState::Down(Side::Either) => ModifierState::Down(other),
        ModifierState::Down(held) if held == side => return rule,
        ModifierState::Down(_) => ModifierState::Down(Side::Either),
        ModifierState::Up | ModifierState::Any => return rule,
    };
    rule.with(family, state)
}

/// `true` when the `side` key of a family in `state` is asked for.
pub(super) fn side_on(state: ModifierState, side: Side) -> bool {
    matches!(state, ModifierState::Down(Side::Either)) || state == ModifierState::Down(side)
}

/// `true` when any modifiers do.
pub(super) fn is_any(rule: ModifierRule) -> bool {
    rule == ModifierRule::ANY
}

/// Any modifiers, or none.
pub(super) fn toggle_any(rule: ModifierRule) -> ModifierRule {
    if is_any(rule) {
        ModifierRule::NONE
    } else {
        ModifierRule::ANY
    }
}

/// The mouse buttons offered, by name.
pub(super) const BUTTONS: [(MouseButton, &str); 5] = [
    (MouseButton::Left, "Left"),
    (MouseButton::Middle, "Middle"),
    (MouseButton::Right, "Right"),
    (MouseButton::Back, "Back"),
    (MouseButton::Forward, "Forward"),
];

/// The wheel directions offered.
pub(super) const WHEEL: [(WheelDirection, &str); 4] = [
    (WheelDirection::Up, "Up"),
    (WheelDirection::Down, "Down"),
    (WheelDirection::Left, "Left"),
    (WheelDirection::Right, "Right"),
];

/// The pinches offered.
pub(super) const PINCHES: [(PinchDirection, &str); 2] = [
    (PinchDirection::In, "Pinch in"),
    (PinchDirection::Out, "Pinch out"),
];

/// The drag directions offered.
pub(super) const DIRECTIONS: [(DragDirection, &str); 9] = [
    (DragDirection::Any, "Any direction"),
    (DragDirection::North, "Up"),
    (DragDirection::NorthEast, "Up right"),
    (DragDirection::East, "Right"),
    (DragDirection::SouthEast, "Down right"),
    (DragDirection::South, "Down"),
    (DragDirection::SouthWest, "Down left"),
    (DragDirection::West, "Left"),
    (DragDirection::NorthWest, "Up left"),
];

/// The keys a shortcut can be chosen from by name: every named key but the modifiers.
pub(super) fn chosen_keys() -> impl Iterator<Item = (Key, &'static str)> {
    Key::named()
        .iter()
        .copied()
        .filter(|(key, _)| !key.is_modifier())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_keeps_its_trigger_and_another_starts_from_its_first_input() {
        let key = Trigger::key(Key::F);
        assert_eq!(Device::of(key), Device::Keyboard);
        assert_eq!(Device::Keyboard.trigger(key), key);
        assert_eq!(
            Device::Mouse.trigger(key),
            Trigger::button(MouseButton::Left)
        );
        assert_eq!(
            Device::Wheel.trigger(key),
            Trigger::Wheel(WheelDirection::Up)
        );
        assert_eq!(Device::Pointer.trigger(key), Trigger::PointerMove);
        for device in Device::ALL {
            assert_eq!(Device::of(device.trigger(key)), device);
        }
    }

    #[test]
    fn events_follow_the_device_and_only_presses_repeat() {
        assert_eq!(Event::of_device(Device::Keyboard).len(), 4);
        assert!(Event::of_device(Device::Mouse).contains(&Event::Drag));
        assert_eq!(Event::of_device(Device::Wheel), []);
        let repeating = Trigger::Key {
            key: Key::Right,
            when: KeyTrigger::Press,
            repeat: true,
        };
        assert!(can_repeat(repeating));
        let released = Event::Release.apply(repeating);
        assert_eq!(
            released,
            Trigger::Key {
                key: Key::Right,
                when: KeyTrigger::Release,
                repeat: false
            }
        );
        assert!(!can_repeat(released));
        assert_eq!(Event::of(released), Some(Event::Release));
        // A drag keeps its direction; leaving it and coming back starts in any direction.
        let button = Trigger::button(MouseButton::Left);
        let drag = Event::Drag.apply(button);
        assert_eq!(Event::of(drag), Some(Event::Drag));
        assert_eq!(
            drag,
            Trigger::Button {
                button: MouseButton::Left,
                when: ButtonTrigger::Drag(Drag::ANY)
            }
        );
        assert_eq!(
            Event::Click.apply(drag),
            Trigger::Button {
                button: MouseButton::Left,
                when: ButtonTrigger::Click
            }
        );
        assert_eq!(Event::of(Trigger::PointerMove), None);
    }

    #[test]
    fn modifiers_switch_by_family_and_by_side_never_to_no_side() {
        let ctrl = toggle_family(ModifierRule::NONE, ModifierKey::Ctrl);
        assert_eq!(ctrl.ctrl, ModifierState::Down(Side::Either));
        assert!(side_on(ctrl.ctrl, Side::Left) && side_on(ctrl.ctrl, Side::Right));
        // Both sides, then the left one off: the right one alone.
        let right = toggle_side(ctrl, ModifierKey::Ctrl, Side::Left);
        assert_eq!(right.ctrl, ModifierState::Down(Side::Right));
        // The last side cannot go: it stays.
        assert_eq!(toggle_side(right, ModifierKey::Ctrl, Side::Right), right);
        // The other side back: either.
        assert_eq!(
            toggle_side(right, ModifierKey::Ctrl, Side::Left).ctrl,
            ModifierState::Down(Side::Either)
        );
        // A family not held has no side to switch.
        assert_eq!(
            toggle_side(ModifierRule::NONE, ModifierKey::Alt, Side::Left),
            ModifierRule::NONE
        );
        assert_eq!(toggle_family(ctrl, ModifierKey::Ctrl), ModifierRule::NONE);
        // Any modifiers, and a family chosen from there.
        let any = toggle_any(ctrl);
        assert!(is_any(any));
        assert_eq!(toggle_any(any), ModifierRule::NONE);
        assert_eq!(
            toggle_family(any, ModifierKey::Shift),
            ModifierRule::NONE.down(ModifierKey::Shift)
        );
    }
}
