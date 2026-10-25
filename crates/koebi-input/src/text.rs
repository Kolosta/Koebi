//! The text form of shortcuts, used by saved keymaps and documentation: `Ctrl+Shift+Z`,
//! `LCtrl+LeftMouse drag north`, `Z double`, `Any+WheelUp`.
//!
//! - Modifiers come first, each followed by `+`: `Ctrl` (either side), `LCtrl`, `RCtrl`,
//!   `Ctrl?` (ignored), and likewise `Shift`, `Alt` and `Gui`. Families not written must not
//!   be held, unless `Any+` makes them ignored.
//! - Then the trigger: a key name ([`Key::name`]), `LeftMouse`, `MiddleMouse`, `RightMouse`,
//!   `BackMouse`, `ForwardMouse`, `Mouse6`..., `WheelUp`, `WheelDown`, `WheelLeft`,
//!   `WheelRight`, `PinchIn`, `PinchOut` or `PointerMove`.
//! - Then qualifiers, separated by spaces: `release`, `click`, `double` and, for keys,
//!   `repeat`; for buttons, `drag`, optionally followed by a direction (`north`, `north-east`,
//!   `east`... `north-west`) and a distance in logical pixels.

use core::fmt;
use core::str::FromStr;

use koebi_core::input::{Key, ModifierKey, MouseButton};

use crate::shortcut::{
    ButtonTrigger, Drag, DragDirection, KeyTrigger, ModifierRule, ModifierState, PinchDirection,
    Shortcut, Side, Trigger, WheelDirection,
};

/// A shortcut whose text form is invalid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseShortcutError {
    /// The text.
    pub text: String,
    /// What is wrong.
    pub reason: String,
}

impl fmt::Display for ParseShortcutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid shortcut {:?}: {}", self.text, self.reason)
    }
}

impl std::error::Error for ParseShortcutError {}

const DIRECTIONS: [(DragDirection, &str); 8] = [
    (DragDirection::North, "north"),
    (DragDirection::NorthEast, "north-east"),
    (DragDirection::East, "east"),
    (DragDirection::SouthEast, "south-east"),
    (DragDirection::South, "south"),
    (DragDirection::SouthWest, "south-west"),
    (DragDirection::West, "west"),
    (DragDirection::NorthWest, "north-west"),
];

const NAMED_TRIGGERS: [(Trigger, &str); 7] = [
    (Trigger::Wheel(WheelDirection::Up), "WheelUp"),
    (Trigger::Wheel(WheelDirection::Down), "WheelDown"),
    (Trigger::Wheel(WheelDirection::Left), "WheelLeft"),
    (Trigger::Wheel(WheelDirection::Right), "WheelRight"),
    (Trigger::Pinch(PinchDirection::In), "PinchIn"),
    (Trigger::Pinch(PinchDirection::Out), "PinchOut"),
    (Trigger::PointerMove, "PointerMove"),
];

fn button_name(button: MouseButton) -> MouseName {
    match button {
        MouseButton::Left => MouseName::Static("LeftMouse"),
        MouseButton::Middle => MouseName::Static("MiddleMouse"),
        MouseButton::Right => MouseName::Static("RightMouse"),
        MouseButton::Back => MouseName::Static("BackMouse"),
        MouseButton::Forward => MouseName::Static("ForwardMouse"),
        MouseButton::Other(number) => MouseName::Numbered(number),
    }
}

enum MouseName {
    Static(&'static str),
    Numbered(u8),
}

fn button_from_name(name: &str) -> Option<MouseButton> {
    let button = match name {
        "LeftMouse" => MouseButton::Left,
        "MiddleMouse" => MouseButton::Middle,
        "RightMouse" => MouseButton::Right,
        "BackMouse" => MouseButton::Back,
        "ForwardMouse" => MouseButton::Forward,
        other => {
            let number: u8 = other.strip_prefix("Mouse")?.parse().ok()?;
            return (number >= 6).then_some(MouseButton::Other(number));
        }
    };
    Some(button)
}

fn write_modifiers(f: &mut fmt::Formatter<'_>, rule: ModifierRule) -> fmt::Result {
    let all_others_any = ModifierKey::ALL
        .into_iter()
        .filter(|family| !matches!(rule.get(*family), ModifierState::Down(_)))
        .all(|family| rule.get(family) == ModifierState::Any);
    let any_family_any = ModifierKey::ALL
        .into_iter()
        .any(|family| rule.get(family) == ModifierState::Any);
    if any_family_any && all_others_any {
        f.write_str("Any+")?;
    }
    for family in ModifierKey::ALL {
        let name = family.name();
        match rule.get(family) {
            ModifierState::Down(Side::Either) => write!(f, "{name}+")?,
            ModifierState::Down(Side::Left) => write!(f, "L{name}+")?,
            ModifierState::Down(Side::Right) => write!(f, "R{name}+")?,
            ModifierState::Any if !all_others_any => write!(f, "{name}?+")?,
            // Up is the default, and `Any+` already covers the ignored families.
            ModifierState::Up | ModifierState::Any => {}
        }
    }
    Ok(())
}

impl fmt::Display for Shortcut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_modifiers(f, self.modifiers)?;
        match self.trigger {
            Trigger::Key { key, when, repeat } => {
                write!(f, "{}", key.name())?;
                match when {
                    KeyTrigger::Press if repeat => f.write_str(" repeat"),
                    KeyTrigger::Press => Ok(()),
                    KeyTrigger::Release => f.write_str(" release"),
                    KeyTrigger::Click => f.write_str(" click"),
                    KeyTrigger::DoubleClick => f.write_str(" double"),
                }
            }
            Trigger::Button { button, when } => {
                match button_name(button) {
                    MouseName::Static(name) => f.write_str(name)?,
                    MouseName::Numbered(number) => write!(f, "Mouse{number}")?,
                }
                match when {
                    ButtonTrigger::Press => Ok(()),
                    ButtonTrigger::Release => f.write_str(" release"),
                    ButtonTrigger::Click => f.write_str(" click"),
                    ButtonTrigger::DoubleClick => f.write_str(" double"),
                    ButtonTrigger::Drag(drag) => {
                        f.write_str(" drag")?;
                        if let Some((_, name)) = DIRECTIONS
                            .iter()
                            .find(|(direction, _)| *direction == drag.direction)
                        {
                            write!(f, " {name}")?;
                        }
                        match drag.distance {
                            Some(distance) => write!(f, " {distance}"),
                            None => Ok(()),
                        }
                    }
                }
            }
            other => {
                let name = NAMED_TRIGGERS
                    .iter()
                    .find(|(trigger, _)| *trigger == other)
                    .map_or("?", |(_, name)| name);
                f.write_str(name)
            }
        }
    }
}

/// Reads one modifier token (without its `+`).
fn modifier_token(token: &str) -> Option<(ModifierKey, ModifierState)> {
    ModifierKey::ALL.into_iter().find_map(|family| {
        let name = family.name();
        let state = if token == name {
            ModifierState::Down(Side::Either)
        } else if token.strip_prefix('L') == Some(name) {
            ModifierState::Down(Side::Left)
        } else if token.strip_prefix('R') == Some(name) {
            ModifierState::Down(Side::Right)
        } else if token.strip_suffix('?') == Some(name) {
            ModifierState::Any
        } else {
            return None;
        };
        Some((family, state))
    })
}

impl FromStr for Shortcut {
    type Err = ParseShortcutError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let error = |reason: &str| ParseShortcutError {
            text: text.to_owned(),
            reason: reason.to_owned(),
        };
        let mut words = text.split_whitespace();
        let head = words.next().ok_or_else(|| error("empty"))?;

        // Modifiers: tokens before a `+`, as long as they name modifiers.
        let mut rule = ModifierRule::NONE;
        let mut written = [false; 4];
        let mut any = false;
        let mut rest = head;
        while let Some((token, after)) = rest.split_once('+') {
            if after.is_empty() {
                // `Ctrl++`: the last `+` is the key itself.
                break;
            }
            if token == "Any" {
                any = true;
            } else if let Some((family, state)) = modifier_token(token) {
                let index = ModifierKey::ALL
                    .iter()
                    .position(|candidate| *candidate == family)
                    .unwrap_or(0);
                if written[index] {
                    return Err(error("a modifier is written twice"));
                }
                written[index] = true;
                rule = rule.with(family, state);
            } else {
                break;
            }
            rest = after;
        }
        if any {
            for (family, written) in ModifierKey::ALL.into_iter().zip(written) {
                if !written {
                    rule = rule.with(family, ModifierState::Any);
                }
            }
        }

        let qualifiers: Vec<&str> = words.collect();
        let trigger =
            if let Some((trigger, _)) = NAMED_TRIGGERS.iter().find(|(_, name)| *name == rest) {
                if !qualifiers.is_empty() {
                    return Err(error("wheel, pinch and pointer triggers take no qualifier"));
                }
                *trigger
            } else if let Some(button) = button_from_name(rest) {
                Trigger::Button {
                    button,
                    when: button_qualifiers(&qualifiers)
                        .ok_or_else(|| error("invalid button qualifiers"))?,
                }
            } else if let Some(key) = Key::from_name(rest) {
                let (when, repeat) = match qualifiers.as_slice() {
                    [] => (KeyTrigger::Press, false),
                    ["repeat"] => (KeyTrigger::Press, true),
                    ["release"] => (KeyTrigger::Release, false),
                    ["click"] => (KeyTrigger::Click, false),
                    ["double"] => (KeyTrigger::DoubleClick, false),
                    _ => return Err(error("invalid key qualifiers")),
                };
                Trigger::Key { key, when, repeat }
            } else {
                return Err(error("unknown key or trigger"));
            };
        Ok(Self {
            trigger,
            modifiers: rule,
        })
    }
}

fn button_qualifiers(qualifiers: &[&str]) -> Option<ButtonTrigger> {
    Some(match qualifiers {
        [] => ButtonTrigger::Press,
        ["release"] => ButtonTrigger::Release,
        ["click"] => ButtonTrigger::Click,
        ["double"] => ButtonTrigger::DoubleClick,
        ["drag", rest @ ..] => {
            let mut drag = Drag::ANY;
            let mut rest = rest.iter();
            let mut next = rest.next();
            if let Some(word) = next
                && let Some((direction, _)) = DIRECTIONS.iter().find(|(_, name)| name == word)
            {
                drag.direction = *direction;
                next = rest.next();
            }
            if let Some(word) = next {
                drag.distance = Some(word.parse().ok()?);
            }
            if rest.next().is_some() {
                return None;
            }
            ButtonTrigger::Drag(drag)
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(text: &str) -> Shortcut {
        let shortcut: Shortcut = text.parse().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(shortcut.to_string(), text, "canonical form of {text}");
        shortcut
    }

    #[test]
    fn canonical_texts_round_trip() {
        for text in [
            "Z",
            "Ctrl+Z",
            "Ctrl+Shift+Z",
            "LCtrl+RAlt+F12 release",
            "Gui+Q",
            "Space repeat",
            "G double",
            "Escape click",
            "LeftMouse",
            "Shift+LeftMouse click",
            "RightMouse release",
            "MiddleMouse drag",
            "LeftMouse drag north-east",
            "LeftMouse drag west 12",
            "LeftMouse drag 30",
            "Mouse7 double",
            "Any+WheelUp",
            "Ctrl+WheelDown",
            "PinchIn",
            "PointerMove",
            "Ctrl?+Shift+Tab",
            "Any+Ctrl+A",
            "KeypadPlus",
            "é",
            "Ctrl++",
            "#4242",
        ] {
            round_trip(text);
        }
    }

    #[test]
    fn parsed_shortcuts_have_the_expected_meaning() {
        let shortcut = round_trip("LCtrl+LeftMouse drag south 16");
        assert_eq!(
            shortcut.trigger,
            Trigger::Button {
                button: MouseButton::Left,
                when: ButtonTrigger::Drag(Drag {
                    direction: DragDirection::South,
                    distance: Some(16),
                }),
            }
        );
        assert_eq!(shortcut.modifiers.ctrl, ModifierState::Down(Side::Left));
        assert_eq!(shortcut.modifiers.shift, ModifierState::Up);
        let any = round_trip("Any+Ctrl+A");
        assert_eq!(any.modifiers.ctrl, ModifierState::Down(Side::Either));
        assert_eq!(any.modifiers.alt, ModifierState::Any);
        let plus = round_trip("Ctrl++");
        assert_eq!(plus.trigger, Trigger::key(Key::Character('+')));
    }

    #[test]
    fn invalid_texts_are_refused_with_a_reason() {
        for text in [
            "",
            "Ctrl+",
            "Ctrl+Ctrl+Z",
            "NotAKey",
            "Z drag",
            "LeftMouse repeat",
            "LeftMouse drag up",
            "WheelUp click",
            "Mouse3",
            "LeftMouse drag north 12 extra",
        ] {
            assert!(
                text.parse::<Shortcut>().is_err(),
                "{text:?} should be refused"
            );
        }
    }
}
