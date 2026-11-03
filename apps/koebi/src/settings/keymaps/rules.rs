//! The rules of editing a shortcut, as pure functions: its device and what changing it keeps,
//! the events each device has and what follows (only presses of actions that repeat repeat,
//! drags keep their direction), the modifiers by family and by side (both sides mean either
//! key, one side never goes alone), and the lists the editor offers, keys by section.

use koebi_core::input::{Key, ModifierKey, MouseButton};
use koebi_input::{
    ActionDef, ButtonTrigger, Drag, DragDirection, KeyTrigger, ModifierRule, ModifierState,
    PinchDirection, Side, Trigger, WheelDirection,
};

/// Where a shortcut comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Device {
    Keyboard,
    /// The mouse: its buttons and its wheel.
    Mouse,
    Touchpad,
    Pointer,
}

impl Device {
    pub(super) const ALL: [Self; 4] = [Self::Keyboard, Self::Mouse, Self::Touchpad, Self::Pointer];

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Keyboard => "Keyboard",
            Self::Mouse => "Mouse",
            Self::Touchpad => "Touchpad",
            Self::Pointer => "Pointer move",
        }
    }

    pub(super) const fn of(trigger: Trigger) -> Self {
        match trigger {
            Trigger::Key { .. } => Self::Keyboard,
            Trigger::Button { .. } | Trigger::Wheel(_) => Self::Mouse,
            Trigger::Pinch(_) => Self::Touchpad,
            Trigger::PointerMove => Self::Pointer,
        }
    }

    /// A trigger of this device: `from` when it is one already, else its first input.
    pub(super) const fn trigger(self, from: Trigger) -> Trigger {
        if matches!(
            (self, from),
            (Self::Keyboard, Trigger::Key { .. })
                | (Self::Mouse, Trigger::Button { .. } | Trigger::Wheel(_))
                | (Self::Touchpad, Trigger::Pinch(_))
                | (Self::Pointer, Trigger::PointerMove)
        ) {
            return from;
        }
        match self {
            Self::Keyboard => Trigger::key(Key::A),
            Self::Mouse => Trigger::button(MouseButton::Left),
            Self::Touchpad => Trigger::Pinch(PinchDirection::In),
            Self::Pointer => Trigger::PointerMove,
        }
    }

    /// What a field recording this device's shortcuts takes: its keys or its buttons and
    /// wheel; nothing for the others, chosen from a list.
    pub(super) const fn records(self) -> bool {
        matches!(self, Self::Keyboard | Self::Mouse)
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
    /// A key goes down and up: a click or a double click of a key means nothing more.
    const KEYS: [Self; 2] = [Self::Press, Self::Release];
    /// A mouse button also clicks, double clicks and drags.
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

    /// The events `trigger` can happen on: those of its key or its button; none for the
    /// wheel, pinches and pointer moves, which only happen.
    pub(super) const fn of_trigger(trigger: Trigger) -> &'static [Self] {
        match trigger {
            Trigger::Key { .. } => &Self::KEYS,
            Trigger::Button { .. } => &Self::BUTTONS,
            Trigger::Wheel(_) | Trigger::Pinch(_) | Trigger::PointerMove => &[],
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
                ButtonTrigger::DoubleClick(_) => Self::DoubleClick,
                ButtonTrigger::Drag(_) => Self::Drag,
            }),
            Trigger::Wheel(_) | Trigger::Pinch(_) | Trigger::PointerMove => None,
        }
    }

    /// `trigger` happening on this event instead: a key repeats on presses only; a drag and
    /// a double click keep what they had of their own.
    pub(super) const fn apply(self, trigger: Trigger) -> Trigger {
        match trigger {
            Trigger::Key { key, repeat, .. } => {
                let when = match self {
                    Self::Press | Self::Drag | Self::Click | Self::DoubleClick => KeyTrigger::Press,
                    Self::Release => KeyTrigger::Release,
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
                    (Self::DoubleClick, ButtonTrigger::DoubleClick(double)) => {
                        ButtonTrigger::DoubleClick(double)
                    }
                    (Self::DoubleClick, _) => ButtonTrigger::DOUBLE_CLICK,
                    (Self::Drag, ButtonTrigger::Drag(drag)) => ButtonTrigger::Drag(drag),
                    (Self::Drag, _) => ButtonTrigger::Drag(Drag::ANY),
                },
            },
            other => other,
        }
    }
}

/// `true` when `trigger` of `action` may repeat while held: a key press, of an action that
/// repeats (moving through a list, not folding or quitting).
pub(super) const fn can_repeat(action: &ActionDef, trigger: Trigger) -> bool {
    action.repeats
        && matches!(
            trigger,
            Trigger::Key {
                when: KeyTrigger::Press,
                ..
            }
        )
}

/// `rule` with `family` switched on (either side) or off.
pub(super) fn toggle_family(rule: ModifierRule, family: ModifierKey) -> ModifierRule {
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

/// Any modifiers on, keeping `rule` to give back once it goes off; off, the rule kept (none
/// when nothing was).
pub(super) fn toggle_any(rule: ModifierRule, kept: Option<ModifierRule>) -> ModifierRule {
    if is_any(rule) {
        kept.filter(|kept| !is_any(*kept))
            .unwrap_or(ModifierRule::NONE)
    } else {
        ModifierRule::ANY
    }
}

/// A list of inputs of a device, by section.
pub(super) struct Section<T: 'static> {
    pub(super) name: &'static str,
    pub(super) inputs: &'static [(T, &'static str)],
}

/// The keys a shortcut can be chosen from, by section; every named key but the modifiers lies
/// in one.
pub(super) const KEY_SECTIONS: [Section<Key>; 8] = [
    Section {
        name: "Letters",
        inputs: &[
            (Key::A, "A"),
            (Key::B, "B"),
            (Key::C, "C"),
            (Key::D, "D"),
            (Key::E, "E"),
            (Key::F, "F"),
            (Key::G, "G"),
            (Key::H, "H"),
            (Key::I, "I"),
            (Key::J, "J"),
            (Key::K, "K"),
            (Key::L, "L"),
            (Key::M, "M"),
            (Key::N, "N"),
            (Key::O, "O"),
            (Key::P, "P"),
            (Key::Q, "Q"),
            (Key::R, "R"),
            (Key::S, "S"),
            (Key::T, "T"),
            (Key::U, "U"),
            (Key::V, "V"),
            (Key::W, "W"),
            (Key::X, "X"),
            (Key::Y, "Y"),
            (Key::Z, "Z"),
        ],
    },
    Section {
        name: "Digits",
        inputs: &[
            (Key::Digit0, "0"),
            (Key::Digit1, "1"),
            (Key::Digit2, "2"),
            (Key::Digit3, "3"),
            (Key::Digit4, "4"),
            (Key::Digit5, "5"),
            (Key::Digit6, "6"),
            (Key::Digit7, "7"),
            (Key::Digit8, "8"),
            (Key::Digit9, "9"),
        ],
    },
    Section {
        name: "Function keys",
        inputs: &[
            (Key::F1, "F1"),
            (Key::F2, "F2"),
            (Key::F3, "F3"),
            (Key::F4, "F4"),
            (Key::F5, "F5"),
            (Key::F6, "F6"),
            (Key::F7, "F7"),
            (Key::F8, "F8"),
            (Key::F9, "F9"),
            (Key::F10, "F10"),
            (Key::F11, "F11"),
            (Key::F12, "F12"),
            (Key::F13, "F13"),
            (Key::F14, "F14"),
            (Key::F15, "F15"),
            (Key::F16, "F16"),
            (Key::F17, "F17"),
            (Key::F18, "F18"),
            (Key::F19, "F19"),
            (Key::F20, "F20"),
            (Key::F21, "F21"),
            (Key::F22, "F22"),
            (Key::F23, "F23"),
            (Key::F24, "F24"),
        ],
    },
    Section {
        name: "Navigation",
        inputs: &[
            (Key::Up, "Up"),
            (Key::Down, "Down"),
            (Key::Left, "Left"),
            (Key::Right, "Right"),
            (Key::Home, "Home"),
            (Key::End, "End"),
            (Key::PageUp, "Page up"),
            (Key::PageDown, "Page down"),
        ],
    },
    Section {
        name: "Editing",
        inputs: &[
            (Key::Space, "Space"),
            (Key::Enter, "Enter"),
            (Key::Tab, "Tab"),
            (Key::Backspace, "Backspace"),
            (Key::Delete, "Delete"),
            (Key::Insert, "Insert"),
            (Key::Escape, "Escape"),
        ],
    },
    Section {
        name: "Punctuation",
        inputs: &[
            (Key::Minus, "Minus"),
            (Key::Equals, "Equals"),
            (Key::LeftBracket, "Left bracket"),
            (Key::RightBracket, "Right bracket"),
            (Key::Backslash, "Backslash"),
            (Key::Semicolon, "Semicolon"),
            (Key::Apostrophe, "Apostrophe"),
            (Key::Grave, "Grave accent"),
            (Key::Comma, "Comma"),
            (Key::Period, "Period"),
            (Key::Slash, "Slash"),
        ],
    },
    Section {
        name: "Numeric keypad",
        inputs: &[
            (Key::Keypad0, "Keypad 0"),
            (Key::Keypad1, "Keypad 1"),
            (Key::Keypad2, "Keypad 2"),
            (Key::Keypad3, "Keypad 3"),
            (Key::Keypad4, "Keypad 4"),
            (Key::Keypad5, "Keypad 5"),
            (Key::Keypad6, "Keypad 6"),
            (Key::Keypad7, "Keypad 7"),
            (Key::Keypad8, "Keypad 8"),
            (Key::Keypad9, "Keypad 9"),
            (Key::KeypadDivide, "Keypad divide"),
            (Key::KeypadMultiply, "Keypad multiply"),
            (Key::KeypadMinus, "Keypad minus"),
            (Key::KeypadPlus, "Keypad plus"),
            (Key::KeypadEnter, "Keypad enter"),
            (Key::KeypadPeriod, "Keypad period"),
            (Key::NumLock, "Num lock"),
        ],
    },
    Section {
        name: "System",
        inputs: &[
            (Key::CapsLock, "Caps lock"),
            (Key::PrintScreen, "Print screen"),
            (Key::ScrollLock, "Scroll lock"),
            (Key::Pause, "Pause"),
            (Key::Menu, "Menu"),
        ],
    },
];

/// What a shortcut of the mouse is made of: a button, or a turn of the wheel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MouseInput {
    Button(MouseButton),
    Wheel(WheelDirection),
}

impl MouseInput {
    /// `trigger` made of this input: a button keeps the event it had.
    pub(super) const fn apply(self, trigger: Trigger) -> Trigger {
        match (self, trigger) {
            (Self::Button(button), Trigger::Button { when, .. }) => {
                Trigger::Button { button, when }
            }
            (Self::Button(button), _) => Trigger::button(button),
            (Self::Wheel(direction), _) => Trigger::Wheel(direction),
        }
    }

    /// The input of `trigger`, when it is the mouse's.
    pub(super) const fn of(trigger: Trigger) -> Option<Self> {
        match trigger {
            Trigger::Button { button, .. } => Some(Self::Button(button)),
            Trigger::Wheel(direction) => Some(Self::Wheel(direction)),
            _ => None,
        }
    }
}

/// The mouse inputs offered, by section: every button, side buttons and numbered ones
/// included, and the wheel turned or tilted.
pub(super) const MOUSE_SECTIONS: [Section<MouseInput>; 2] = [
    Section {
        name: "Buttons",
        inputs: &[
            (MouseInput::Button(MouseButton::Left), "Left button"),
            (MouseInput::Button(MouseButton::Middle), "Middle button"),
            (MouseInput::Button(MouseButton::Right), "Right button"),
            (MouseInput::Button(MouseButton::Back), "Back button"),
            (MouseInput::Button(MouseButton::Forward), "Forward button"),
            (MouseInput::Button(MouseButton::Other(6)), "Button 6"),
            (MouseInput::Button(MouseButton::Other(7)), "Button 7"),
            (MouseInput::Button(MouseButton::Other(8)), "Button 8"),
        ],
    },
    Section {
        name: "Wheel",
        inputs: &[
            (MouseInput::Wheel(WheelDirection::Up), "Wheel up"),
            (MouseInput::Wheel(WheelDirection::Down), "Wheel down"),
            (MouseInput::Wheel(WheelDirection::Left), "Wheel left"),
            (MouseInput::Wheel(WheelDirection::Right), "Wheel right"),
        ],
    },
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

/// The name of `input` in `sections`, if it lies in one.
pub(super) fn name_in<T: PartialEq + Copy>(
    sections: &[Section<T>],
    input: T,
) -> Option<&'static str> {
    sections
        .iter()
        .flat_map(|section| section.inputs.iter())
        .find(|(candidate, _)| *candidate == input)
        .map(|(_, name)| *name)
}

#[cfg(test)]
mod tests {
    use koebi_input::Shortcut;

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
        // The wheel is the mouse's.
        let wheel = Trigger::Wheel(WheelDirection::Up);
        assert_eq!(Device::of(wheel), Device::Mouse);
        assert_eq!(Device::Mouse.trigger(wheel), wheel);
        assert_eq!(Device::Pointer.trigger(key), Trigger::PointerMove);
        for device in Device::ALL {
            assert_eq!(Device::of(device.trigger(key)), device);
        }
        assert!(Device::Keyboard.records() && Device::Mouse.records());
        assert!(!Device::Touchpad.records() && !Device::Pointer.records());
    }

    #[test]
    fn keys_have_presses_and_releases_buttons_clicks_and_drags_too() {
        let key = Trigger::key(Key::F);
        assert_eq!(Event::of_trigger(key), [Event::Press, Event::Release]);
        let button = Trigger::button(MouseButton::Left);
        assert!(Event::of_trigger(button).contains(&Event::DoubleClick));
        assert!(Event::of_trigger(button).contains(&Event::Drag));
        assert_eq!(Event::of_trigger(Trigger::Wheel(WheelDirection::Up)), []);
        assert_eq!(Event::of(Trigger::PointerMove), None);
        // A click of a key, saved by an older version, becomes a press.
        assert_eq!(Event::Click.apply(key), key);
    }

    #[test]
    fn only_presses_of_actions_that_repeat_repeat() {
        let repeating = Trigger::Key {
            key: Key::Right,
            when: KeyTrigger::Press,
            repeat: true,
        };
        let (next, fold) = (&koebi_ui::LIST_NEXT, &koebi_ui::LIST_FOLD);
        assert!(next.repeats && !fold.repeats);
        assert!(can_repeat(next, repeating));
        assert!(!can_repeat(fold, repeating));
        let released = Event::Release.apply(repeating);
        assert_eq!(
            released,
            Trigger::Key {
                key: Key::Right,
                when: KeyTrigger::Release,
                repeat: false
            }
        );
        assert!(!can_repeat(next, released));
        assert_eq!(Event::of(released), Some(Event::Release));
    }

    #[test]
    fn drags_and_double_clicks_keep_what_they_have_of_their_own() {
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
        let own = Trigger::Button {
            button: MouseButton::Left,
            when: ButtonTrigger::DoubleClick(koebi_input::DoubleClick { time: Some(300) }),
        };
        assert_eq!(Event::DoubleClick.apply(own), own);
        assert_eq!(
            Event::DoubleClick.apply(button),
            Trigger::Button {
                button: MouseButton::Left,
                when: ButtonTrigger::DOUBLE_CLICK
            }
        );
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
    }

    #[test]
    fn any_keeps_the_modifiers_it_replaces_and_gives_them_back() {
        let ctrl_shift = Shortcut::key(Key::A)
            .with(ModifierKey::Ctrl)
            .with(ModifierKey::Shift)
            .modifiers;
        let any = toggle_any(ctrl_shift, None);
        assert!(is_any(any));
        // Off again: the modifiers it replaced come back.
        assert_eq!(toggle_any(any, Some(ctrl_shift)), ctrl_shift);
        // Nothing kept: none.
        assert_eq!(toggle_any(any, None), ModifierRule::NONE);
        assert_eq!(toggle_any(any, Some(ModifierRule::ANY)), ModifierRule::NONE);
    }

    #[test]
    fn every_key_lies_in_one_section_and_every_mouse_input_has_a_name() {
        let keys: Vec<Key> = KEY_SECTIONS
            .iter()
            .flat_map(|section| section.inputs.iter().map(|(key, _)| *key))
            .collect();
        for (key, _) in Key::named().iter().filter(|(key, _)| !key.is_modifier()) {
            let count = keys.iter().filter(|listed| *listed == key).count();
            assert_eq!(count, 1, "{key:?} lies in {count} sections");
        }
        assert!(keys.iter().all(|key| !key.is_modifier()));
        assert_eq!(name_in(&KEY_SECTIONS, Key::PageUp), Some("Page up"));
        for direction in [
            WheelDirection::Up,
            WheelDirection::Down,
            WheelDirection::Left,
            WheelDirection::Right,
        ] {
            assert!(name_in(&MOUSE_SECTIONS, MouseInput::Wheel(direction)).is_some());
        }
        assert!(name_in(&MOUSE_SECTIONS, MouseInput::Button(MouseButton::Forward)).is_some());
        // A button keeps its event when another is chosen; the wheel has none.
        let click = Trigger::Button {
            button: MouseButton::Left,
            when: ButtonTrigger::Click,
        };
        assert_eq!(
            MouseInput::Button(MouseButton::Right).apply(click),
            Trigger::Button {
                button: MouseButton::Right,
                when: ButtonTrigger::Click
            }
        );
        let wheel = MouseInput::Wheel(WheelDirection::Left).apply(click);
        assert_eq!(wheel, Trigger::Wheel(WheelDirection::Left));
        assert_eq!(
            MouseInput::of(wheel),
            Some(MouseInput::Wheel(WheelDirection::Left))
        );
        assert_eq!(
            MouseInput::Button(MouseButton::Middle).apply(wheel),
            Trigger::button(MouseButton::Middle)
        );
    }
}
