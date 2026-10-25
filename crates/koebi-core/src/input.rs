//! Input vocabulary shared by the layer that produces input (`koebi-platform`) and the layers
//! that consume it (shortcuts, components): keys, modifier keys and mouse buttons.
//!
//! Keys are identified by what they produce in the current keyboard layout, so that a
//! shortcut written `Ctrl+Z` follows the key labelled Z on QWERTY and AZERTY keyboards alike.
//! Names (`Key::name`, [`Key::from_name`]) are stable: saved shortcuts use them.

use core::fmt;

macro_rules! keys {
    ($($(#[doc = $doc:literal])* $variant:ident => $name:literal,)*) => {
        /// A key of the keyboard, as the current layout labels it.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum Key {
            $($(#[doc = $doc])* $variant,)*
            /// Another key producing this character in the current layout (`é`, `²`...).
            Character(char),
            /// A key Koebi has no name for, by platform key code.
            Unknown(u32),
        }

        impl Key {
            /// Every named key with its stable name.
            const NAMED: &'static [(Self, &'static str)] = &[$((Self::$variant, $name),)*];
        }
    };
}

keys! {
    /// Letter A.
    A => "A",
    /// Letter B.
    B => "B",
    /// Letter C.
    C => "C",
    /// Letter D.
    D => "D",
    /// Letter E.
    E => "E",
    /// Letter F.
    F => "F",
    /// Letter G.
    G => "G",
    /// Letter H.
    H => "H",
    /// Letter I.
    I => "I",
    /// Letter J.
    J => "J",
    /// Letter K.
    K => "K",
    /// Letter L.
    L => "L",
    /// Letter M.
    M => "M",
    /// Letter N.
    N => "N",
    /// Letter O.
    O => "O",
    /// Letter P.
    P => "P",
    /// Letter Q.
    Q => "Q",
    /// Letter R.
    R => "R",
    /// Letter S.
    S => "S",
    /// Letter T.
    T => "T",
    /// Letter U.
    U => "U",
    /// Letter V.
    V => "V",
    /// Letter W.
    W => "W",
    /// Letter X.
    X => "X",
    /// Letter Y.
    Y => "Y",
    /// Letter Z.
    Z => "Z",
    /// Digit 0 of the main block.
    Digit0 => "0",
    /// Digit 1 of the main block.
    Digit1 => "1",
    /// Digit 2 of the main block.
    Digit2 => "2",
    /// Digit 3 of the main block.
    Digit3 => "3",
    /// Digit 4 of the main block.
    Digit4 => "4",
    /// Digit 5 of the main block.
    Digit5 => "5",
    /// Digit 6 of the main block.
    Digit6 => "6",
    /// Digit 7 of the main block.
    Digit7 => "7",
    /// Digit 8 of the main block.
    Digit8 => "8",
    /// Digit 9 of the main block.
    Digit9 => "9",
    /// Function key 1.
    F1 => "F1",
    /// Function key 2.
    F2 => "F2",
    /// Function key 3.
    F3 => "F3",
    /// Function key 4.
    F4 => "F4",
    /// Function key 5.
    F5 => "F5",
    /// Function key 6.
    F6 => "F6",
    /// Function key 7.
    F7 => "F7",
    /// Function key 8.
    F8 => "F8",
    /// Function key 9.
    F9 => "F9",
    /// Function key 10.
    F10 => "F10",
    /// Function key 11.
    F11 => "F11",
    /// Function key 12.
    F12 => "F12",
    /// Function key 13.
    F13 => "F13",
    /// Function key 14.
    F14 => "F14",
    /// Function key 15.
    F15 => "F15",
    /// Function key 16.
    F16 => "F16",
    /// Function key 17.
    F17 => "F17",
    /// Function key 18.
    F18 => "F18",
    /// Function key 19.
    F19 => "F19",
    /// Function key 20.
    F20 => "F20",
    /// Function key 21.
    F21 => "F21",
    /// Function key 22.
    F22 => "F22",
    /// Function key 23.
    F23 => "F23",
    /// Function key 24.
    F24 => "F24",
    /// Escape.
    Escape => "Escape",
    /// Enter (Return) of the main block.
    Enter => "Enter",
    /// Tab.
    Tab => "Tab",
    /// Backspace.
    Backspace => "Backspace",
    /// Delete.
    Delete => "Delete",
    /// Insert.
    Insert => "Insert",
    /// Home.
    Home => "Home",
    /// End.
    End => "End",
    /// Page up.
    PageUp => "PageUp",
    /// Page down.
    PageDown => "PageDown",
    /// Arrow up.
    Up => "Up",
    /// Arrow down.
    Down => "Down",
    /// Arrow left.
    Left => "Left",
    /// Arrow right.
    Right => "Right",
    /// Space bar.
    Space => "Space",
    /// The key producing `-`.
    Minus => "Minus",
    /// The key producing `=`.
    Equals => "Equals",
    /// The key producing `[`.
    LeftBracket => "LeftBracket",
    /// The key producing `]`.
    RightBracket => "RightBracket",
    /// The key producing `\`.
    Backslash => "Backslash",
    /// The key producing `;`.
    Semicolon => "Semicolon",
    /// The key producing `'`.
    Apostrophe => "Apostrophe",
    /// The key producing `` ` ``.
    Grave => "Grave",
    /// The key producing `,`.
    Comma => "Comma",
    /// The key producing `.`.
    Period => "Period",
    /// The key producing `/`.
    Slash => "Slash",
    /// Caps lock.
    CapsLock => "CapsLock",
    /// Print screen.
    PrintScreen => "PrintScreen",
    /// Scroll lock.
    ScrollLock => "ScrollLock",
    /// Pause.
    Pause => "Pause",
    /// The context menu (application) key.
    Menu => "Menu",
    /// Num lock.
    NumLock => "NumLock",
    /// Keypad 0.
    Keypad0 => "Keypad0",
    /// Keypad 1.
    Keypad1 => "Keypad1",
    /// Keypad 2.
    Keypad2 => "Keypad2",
    /// Keypad 3.
    Keypad3 => "Keypad3",
    /// Keypad 4.
    Keypad4 => "Keypad4",
    /// Keypad 5.
    Keypad5 => "Keypad5",
    /// Keypad 6.
    Keypad6 => "Keypad6",
    /// Keypad 7.
    Keypad7 => "Keypad7",
    /// Keypad 8.
    Keypad8 => "Keypad8",
    /// Keypad 9.
    Keypad9 => "Keypad9",
    /// Keypad `/`.
    KeypadDivide => "KeypadDivide",
    /// Keypad `*`.
    KeypadMultiply => "KeypadMultiply",
    /// Keypad `-`.
    KeypadMinus => "KeypadMinus",
    /// Keypad `+`.
    KeypadPlus => "KeypadPlus",
    /// Keypad Enter.
    KeypadEnter => "KeypadEnter",
    /// Keypad `.`.
    KeypadPeriod => "KeypadPeriod",
    /// Left Shift.
    LeftShift => "LeftShift",
    /// Right Shift.
    RightShift => "RightShift",
    /// Left Ctrl.
    LeftCtrl => "LeftCtrl",
    /// Right Ctrl.
    RightCtrl => "RightCtrl",
    /// Left Alt (Option on Apple keyboards).
    LeftAlt => "LeftAlt",
    /// Right Alt (Option, `AltGr`).
    RightAlt => "RightAlt",
    /// Left GUI key: Windows, Command or Super.
    LeftGui => "LeftGui",
    /// Right GUI key: Windows, Command or Super.
    RightGui => "RightGui",
}

impl Key {
    /// The key's stable name: `A`, `F12`, `KeypadPlus`, the character itself for
    /// [`Key::Character`], `#` and the code for [`Key::Unknown`].
    pub fn name(self) -> KeyName {
        match self {
            Self::Character(character) => KeyName::Character(character),
            Self::Unknown(code) => KeyName::Code(code),
            named => KeyName::Static(
                Self::NAMED
                    .iter()
                    .find(|(key, _)| *key == named)
                    .map_or("?", |(_, name)| name),
            ),
        }
    }

    /// The key named `name` (see [`Key::name`]); a single character that is not a named key
    /// is a [`Key::Character`].
    pub fn from_name(name: &str) -> Option<Self> {
        if let Some((key, _)) = Self::NAMED.iter().find(|(_, known)| *known == name) {
            return Some(*key);
        }
        if let Some(code) = name.strip_prefix('#') {
            return code.parse().ok().map(Self::Unknown);
        }
        let mut characters = name.chars();
        match (characters.next(), characters.next()) {
            (Some(character), None) => Some(Self::Character(character)),
            _ => None,
        }
    }

    /// `true` for the Shift, Ctrl, Alt and GUI keys themselves.
    pub fn is_modifier(self) -> bool {
        matches!(
            self,
            Self::LeftShift
                | Self::RightShift
                | Self::LeftCtrl
                | Self::RightCtrl
                | Self::LeftAlt
                | Self::RightAlt
                | Self::LeftGui
                | Self::RightGui
        )
    }
}

/// The name of a [`Key`], displayed without allocating.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyName {
    /// A named key.
    Static(&'static str),
    /// A character key.
    Character(char),
    /// A key known only by its platform code.
    Code(u32),
}

impl fmt::Display for KeyName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Static(name) => f.write_str(name),
            Self::Character(character) => write!(f, "{character}"),
            Self::Code(code) => write!(f, "#{code}"),
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name().fmt(f)
    }
}

/// The modifier keys held down, with their side: left and right Ctrl are told apart.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers(u8);

impl Modifiers {
    /// No modifier.
    pub const NONE: Self = Self(0);
    /// Left Shift.
    pub const LEFT_SHIFT: Self = Self(1);
    /// Right Shift.
    pub const RIGHT_SHIFT: Self = Self(1 << 1);
    /// Left Ctrl.
    pub const LEFT_CTRL: Self = Self(1 << 2);
    /// Right Ctrl.
    pub const RIGHT_CTRL: Self = Self(1 << 3);
    /// Left Alt (Option).
    pub const LEFT_ALT: Self = Self(1 << 4);
    /// Right Alt (Option, `AltGr`).
    pub const RIGHT_ALT: Self = Self(1 << 5);
    /// Left GUI key: Windows, Command or Super.
    pub const LEFT_GUI: Self = Self(1 << 6);
    /// Right GUI key: Windows, Command or Super.
    pub const RIGHT_GUI: Self = Self(1 << 7);

    /// The raw bits, for storage.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Modifiers from raw bits (see [`Modifiers::bits`]).
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    /// `true` when every modifier of `other` is held.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Both sets of modifiers.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// `self` without the modifiers of `other`.
    #[must_use]
    pub const fn without(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    /// `true` when no modifier is held.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The sides of `family` that are held.
    pub const fn sides(self, family: ModifierKey) -> Sides {
        let (left, right) = family.bits();
        Sides {
            left: self.0 & left.0 != 0,
            right: self.0 & right.0 != 0,
        }
    }

    /// `true` when either Shift is held.
    pub const fn shift(self) -> bool {
        self.sides(ModifierKey::Shift).any()
    }

    /// `true` when either Ctrl is held.
    pub const fn ctrl(self) -> bool {
        self.sides(ModifierKey::Ctrl).any()
    }

    /// `true` when either Alt is held.
    pub const fn alt(self) -> bool {
        self.sides(ModifierKey::Alt).any()
    }

    /// `true` when either GUI key is held.
    pub const fn gui(self) -> bool {
        self.sides(ModifierKey::Gui).any()
    }
}

/// A family of modifier keys, each with a left and a right key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModifierKey {
    /// Shift.
    Shift,
    /// Ctrl.
    Ctrl,
    /// Alt (Option on Apple keyboards).
    Alt,
    /// The GUI key: Windows, Command or Super.
    Gui,
}

impl ModifierKey {
    /// Every family, in display order.
    pub const ALL: [Self; 4] = [Self::Ctrl, Self::Shift, Self::Alt, Self::Gui];

    /// The left and right modifiers of the family.
    pub const fn bits(self) -> (Modifiers, Modifiers) {
        match self {
            Self::Shift => (Modifiers::LEFT_SHIFT, Modifiers::RIGHT_SHIFT),
            Self::Ctrl => (Modifiers::LEFT_CTRL, Modifiers::RIGHT_CTRL),
            Self::Alt => (Modifiers::LEFT_ALT, Modifiers::RIGHT_ALT),
            Self::Gui => (Modifiers::LEFT_GUI, Modifiers::RIGHT_GUI),
        }
    }

    /// The family's stable name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Shift => "Shift",
            Self::Ctrl => "Ctrl",
            Self::Alt => "Alt",
            Self::Gui => "Gui",
        }
    }
}

/// Which keys of a modifier family are held.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Sides {
    /// The left key is held.
    pub left: bool,
    /// The right key is held.
    pub right: bool,
}

impl Sides {
    /// `true` when either key is held.
    pub const fn any(self) -> bool {
        self.left || self.right
    }
}

/// A mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Primary button.
    Left,
    /// Wheel button.
    Middle,
    /// Secondary button.
    Right,
    /// First side button, usually "back".
    Back,
    /// Second side button, usually "forward".
    Forward,
    /// Any other button, by number (6 and above).
    Other(u8),
}

impl MouseButton {
    /// The button's number: 1 left, 2 middle, 3 right, 4 back, 5 forward.
    pub const fn number(self) -> u8 {
        match self {
            Self::Left => 1,
            Self::Middle => 2,
            Self::Right => 3,
            Self::Back => 4,
            Self::Forward => 5,
            Self::Other(number) => number,
        }
    }
}

impl From<u8> for MouseButton {
    /// The button numbered `number` (see [`MouseButton::number`]).
    fn from(number: u8) -> Self {
        match number {
            1 => Self::Left,
            2 => Self::Middle,
            3 => Self::Right,
            4 => Self::Back,
            5 => Self::Forward,
            other => Self::Other(other),
        }
    }
}

/// The shape of the pointer, as interface components ask for it. The platform layer maps
/// it to the system's cursors; `Custom` cursors are made by the application.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CursorShape {
    /// The default arrow.
    #[default]
    Default,
    /// A clickable element.
    Pointer,
    /// Text insertion.
    Text,
    /// Precise selection.
    Crosshair,
    /// Moving something in any direction.
    Move,
    /// The action under the pointer is not possible.
    NotAllowed,
    /// Resizing horizontally (a vertical gutter).
    ResizeHorizontal,
    /// Resizing vertically (a horizontal gutter).
    ResizeVertical,
    /// Resizing along the top-left to bottom-right diagonal.
    ResizeNwse,
    /// Resizing along the top-right to bottom-left diagonal.
    ResizeNesw,
    /// A cursor the application created, by its own number.
    Custom(u16),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_names_round_trip() {
        for (key, name) in Key::NAMED {
            assert_eq!(key.name().to_string(), *name);
            assert_eq!(Key::from_name(name), Some(*key));
        }
        assert_eq!(Key::from_name("é"), Some(Key::Character('é')));
        assert_eq!(Key::Character('é').to_string(), "é");
        assert_eq!(Key::from_name("#4242"), Some(Key::Unknown(4242)));
        assert_eq!(Key::Unknown(4242).to_string(), "#4242");
        assert_eq!(Key::from_name("NotAKey"), None);
        assert_eq!(Key::from_name(""), None);
    }

    #[test]
    fn named_keys_have_unique_names() {
        for (index, (_, name)) in Key::NAMED.iter().enumerate() {
            assert!(
                !Key::NAMED[..index].iter().any(|(_, other)| other == name),
                "{name}"
            );
        }
    }

    #[test]
    fn modifiers_tell_sides_apart() {
        let held = Modifiers::LEFT_CTRL.union(Modifiers::RIGHT_SHIFT);
        assert!(held.ctrl() && held.shift() && !held.alt() && !held.gui());
        assert_eq!(
            held.sides(ModifierKey::Ctrl),
            Sides {
                left: true,
                right: false
            }
        );
        assert!(held.contains(Modifiers::LEFT_CTRL));
        assert!(!held.contains(Modifiers::RIGHT_CTRL));
        assert_eq!(held.without(Modifiers::LEFT_CTRL), Modifiers::RIGHT_SHIFT);
        assert_eq!(Modifiers::from_bits(held.bits()), held);
        assert!(Modifiers::NONE.is_empty());
    }

    #[test]
    fn mouse_buttons_follow_the_platform_numbering() {
        for number in 1..=8 {
            assert_eq!(MouseButton::from(number).number(), number);
        }
        assert_eq!(MouseButton::from(4), MouseButton::Back);
    }
}
