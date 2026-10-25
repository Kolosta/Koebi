//! [`Shortcut`]: what the user does to trigger an action, and which modifier keys must be held.
//!
//! Shortcuts compare by value; two shortcuts *overlap* when one input could trigger both,
//! which is what conflicts are made of ([`Shortcut::overlaps`]).

use koebi_core::input::{Key, ModifierKey, Modifiers, MouseButton, Sides};

/// When a key triggers its shortcut.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyTrigger {
    /// When the key goes down.
    Press,
    /// When the key goes up.
    Release,
    /// When the key goes up after going down.
    Click,
    /// When the key goes down a second time within the double-click time.
    DoubleClick,
}

/// The eight directions of a drag, and any of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DragDirection {
    /// Any direction.
    Any,
    /// Up.
    North,
    /// Up and right.
    NorthEast,
    /// Right.
    East,
    /// Down and right.
    SouthEast,
    /// Down.
    South,
    /// Down and left.
    SouthWest,
    /// Left.
    West,
    /// Up and left.
    NorthWest,
}

impl DragDirection {
    /// The eight directions, clockwise from north.
    pub const COMPASS: [Self; 8] = [
        Self::North,
        Self::NorthEast,
        Self::East,
        Self::SouthEast,
        Self::South,
        Self::SouthWest,
        Self::West,
        Self::NorthWest,
    ];

    /// The compass direction of a movement of `dx`, `dy` logical pixels (`y` grows downward):
    /// each direction covers 45 degrees around its axis.
    pub fn of(dx: f32, dy: f32) -> Self {
        if dx == 0.0 && dy == 0.0 {
            return Self::Any;
        }
        // Angle clockwise from north, in eighths of a turn.
        let angle = dx.atan2(-dy).to_degrees().rem_euclid(360.0);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the angle is in 0..360, so the sector is in 0..=8"
        )]
        let sector = ((angle + 22.5) / 45.0) as usize % 8;
        Self::COMPASS[sector]
    }

    /// `true` when a drag going `actual` satisfies this direction.
    pub fn accepts(self, actual: Self) -> bool {
        self == Self::Any || self == actual
    }

    /// `true` when some drag satisfies both directions.
    pub fn overlaps(self, other: Self) -> bool {
        self == Self::Any || other == Self::Any || self == other
    }
}

/// A drag of a mouse button: a press followed by a movement beyond a distance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Drag {
    /// The direction of the movement.
    pub direction: DragDirection,
    /// The distance in logical pixels before the drag starts, when it differs from the
    /// drag threshold of the input settings.
    pub distance: Option<u16>,
}

impl Drag {
    /// A drag in any direction, from the drag threshold.
    pub const ANY: Self = Self {
        direction: DragDirection::Any,
        distance: None,
    };
}

/// When a mouse button triggers its shortcut.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonTrigger {
    /// When the button goes down.
    Press,
    /// When the button goes up.
    Release,
    /// When the button goes up without the pointer having been dragged.
    Click,
    /// When the button goes down a second time quickly (the platform counts clicks).
    DoubleClick,
    /// When the pointer moves far enough with the button down.
    Drag(Drag),
}

impl ButtonTrigger {
    fn overlaps(self, other: Self) -> bool {
        match (self, other) {
            (Self::Drag(a), Self::Drag(b)) => a.direction.overlaps(b.direction),
            (a, b) => a == b,
        }
    }
}

/// A direction of the wheel (or of a two-finger scroll on a touchpad).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WheelDirection {
    /// Away from the user.
    Up,
    /// Toward the user.
    Down,
    /// Left.
    Left,
    /// Right.
    Right,
}

/// A direction of a pinch gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PinchDirection {
    /// Fingers moving apart: zoom in.
    In,
    /// Fingers moving together: zoom out.
    Out,
}

/// The input part of a shortcut.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Trigger {
    /// A key of the keyboard.
    Key {
        /// The key.
        key: Key,
        /// When it triggers.
        when: KeyTrigger,
        /// Whether a key held down triggers again at the system's repeat rate (presses only).
        repeat: bool,
    },
    /// A mouse button.
    Button {
        /// The button.
        button: MouseButton,
        /// When it triggers.
        when: ButtonTrigger,
    },
    /// The wheel, or a two-finger scroll.
    Wheel(WheelDirection),
    /// A two-finger pinch.
    Pinch(PinchDirection),
    /// Any pointer movement.
    PointerMove,
}

impl Trigger {
    /// The press of `key`.
    pub const fn key(key: Key) -> Self {
        Self::Key {
            key,
            when: KeyTrigger::Press,
            repeat: false,
        }
    }

    /// The press of `button`.
    pub const fn button(button: MouseButton) -> Self {
        Self::Button {
            button,
            when: ButtonTrigger::Press,
        }
    }

    /// `true` when some input satisfies both triggers.
    pub fn overlaps(self, other: Self) -> bool {
        match (self, other) {
            (
                Self::Key {
                    key: a, when: wa, ..
                },
                Self::Key {
                    key: b, when: wb, ..
                },
            ) => a == b && wa == wb,
            (
                Self::Button {
                    button: a,
                    when: wa,
                },
                Self::Button {
                    button: b,
                    when: wb,
                },
            ) => a == b && wa.overlaps(wb),
            (a, b) => a == b,
        }
    }
}

/// Which keys of a modifier family must be held.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// The left key.
    Left,
    /// The right key.
    Right,
    /// Either key.
    Either,
}

/// What a shortcut requires of one modifier family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModifierState {
    /// Neither key may be held.
    Up,
    /// This side must be held.
    Down(Side),
    /// The family does not matter.
    Any,
}

impl ModifierState {
    /// `true` when the keys held satisfy the state.
    pub fn accepts(self, held: Sides) -> bool {
        match self {
            Self::Up => !held.any(),
            Self::Down(Side::Left) => held.left,
            Self::Down(Side::Right) => held.right,
            Self::Down(Side::Either) => held.any(),
            Self::Any => true,
        }
    }

    /// `true` when some keys held satisfy both states.
    pub fn overlaps(self, other: Self) -> bool {
        [(false, false), (true, false), (false, true), (true, true)]
            .into_iter()
            .map(|(left, right)| Sides { left, right })
            .any(|held| self.accepts(held) && other.accepts(held))
    }
}

/// The modifier keys a shortcut requires: Ctrl, Shift, Alt and GUI, each up, down on a side,
/// or ignored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModifierRule {
    /// Ctrl.
    pub ctrl: ModifierState,
    /// Shift.
    pub shift: ModifierState,
    /// Alt (Option).
    pub alt: ModifierState,
    /// GUI: Windows, Command or Super.
    pub gui: ModifierState,
}

impl ModifierRule {
    /// No modifier may be held.
    pub const NONE: Self = Self {
        ctrl: ModifierState::Up,
        shift: ModifierState::Up,
        alt: ModifierState::Up,
        gui: ModifierState::Up,
    };

    /// Modifiers do not matter.
    pub const ANY: Self = Self {
        ctrl: ModifierState::Any,
        shift: ModifierState::Any,
        alt: ModifierState::Any,
        gui: ModifierState::Any,
    };

    /// The family the platform uses for standard shortcuts (copy, undo...): Command on Apple
    /// platforms, Ctrl elsewhere.
    pub const PRIMARY: ModifierKey = if cfg!(any(target_os = "macos", target_os = "ios")) {
        ModifierKey::Gui
    } else {
        ModifierKey::Ctrl
    };

    /// The state required of `family`.
    pub const fn get(self, family: ModifierKey) -> ModifierState {
        match family {
            ModifierKey::Ctrl => self.ctrl,
            ModifierKey::Shift => self.shift,
            ModifierKey::Alt => self.alt,
            ModifierKey::Gui => self.gui,
        }
    }

    /// The same rule, requiring `state` of `family`.
    #[must_use]
    pub const fn with(mut self, family: ModifierKey, state: ModifierState) -> Self {
        match family {
            ModifierKey::Ctrl => self.ctrl = state,
            ModifierKey::Shift => self.shift = state,
            ModifierKey::Alt => self.alt = state,
            ModifierKey::Gui => self.gui = state,
        }
        self
    }

    /// The same rule, requiring either key of `family`.
    #[must_use]
    pub const fn down(self, family: ModifierKey) -> Self {
        self.with(family, ModifierState::Down(Side::Either))
    }

    /// `true` when the modifiers held satisfy the rule.
    pub fn accepts(self, held: Modifiers) -> bool {
        ModifierKey::ALL
            .into_iter()
            .all(|family| self.get(family).accepts(held.sides(family)))
    }

    /// `true` when some modifiers held satisfy both rules.
    pub fn overlaps(self, other: Self) -> bool {
        ModifierKey::ALL
            .into_iter()
            .all(|family| self.get(family).overlaps(other.get(family)))
    }
}

impl Default for ModifierRule {
    fn default() -> Self {
        Self::NONE
    }
}

/// A trigger and the modifier keys it needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Shortcut {
    /// What the user does.
    pub trigger: Trigger,
    /// Which modifiers must be held, by side.
    pub modifiers: ModifierRule,
}

impl Shortcut {
    /// `trigger` with no modifier held.
    pub const fn new(trigger: Trigger) -> Self {
        Self {
            trigger,
            modifiers: ModifierRule::NONE,
        }
    }

    /// The press of `key` with no modifier.
    pub const fn key(key: Key) -> Self {
        Self::new(Trigger::key(key))
    }

    /// The press of `button` with no modifier.
    pub const fn button(button: MouseButton, when: ButtonTrigger) -> Self {
        Self::new(Trigger::Button { button, when })
    }

    /// The same shortcut, with either key of `family` held.
    #[must_use]
    pub const fn with(mut self, family: ModifierKey) -> Self {
        self.modifiers = self.modifiers.down(family);
        self
    }

    /// The same shortcut, with the platform's primary modifier held (Command on Apple
    /// platforms, Ctrl elsewhere).
    #[must_use]
    pub const fn primary(self) -> Self {
        self.with(ModifierRule::PRIMARY)
    }

    /// The same shortcut with other modifier requirements.
    #[must_use]
    pub const fn modifiers(mut self, rule: ModifierRule) -> Self {
        self.modifiers = rule;
        self
    }

    /// `true` when one input could trigger both shortcuts.
    pub fn overlaps(self, other: Self) -> bool {
        self.trigger.overlaps(other.trigger) && self.modifiers.overlaps(other.modifiers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_directions_cover_45_degrees_each() {
        assert_eq!(DragDirection::of(0.0, -10.0), DragDirection::North);
        assert_eq!(DragDirection::of(10.0, 0.0), DragDirection::East);
        assert_eq!(DragDirection::of(10.0, 10.0), DragDirection::SouthEast);
        assert_eq!(DragDirection::of(-10.0, 1.0), DragDirection::West);
        // 22 degrees east of north is still north; 23 is north-east.
        let tan22 = 22.0_f32.to_radians().tan();
        assert_eq!(DragDirection::of(tan22, -1.0), DragDirection::North);
        let tan23 = 23.0_f32.to_radians().tan();
        assert_eq!(DragDirection::of(tan23, -1.0), DragDirection::NorthEast);
        assert_eq!(DragDirection::of(0.0, 0.0), DragDirection::Any);
        assert!(DragDirection::Any.accepts(DragDirection::West));
        assert!(!DragDirection::East.accepts(DragDirection::West));
    }

    #[test]
    fn modifier_states_follow_their_side() {
        let left = Sides {
            left: true,
            right: false,
        };
        let right = Sides {
            left: false,
            right: true,
        };
        assert!(ModifierState::Down(Side::Left).accepts(left));
        assert!(!ModifierState::Down(Side::Left).accepts(right));
        assert!(ModifierState::Down(Side::Either).accepts(right));
        assert!(!ModifierState::Up.accepts(left));
        assert!(ModifierState::Any.accepts(Sides::default()));
        assert!(!ModifierState::Down(Side::Left).overlaps(ModifierState::Up));
        assert!(ModifierState::Down(Side::Left).overlaps(ModifierState::Down(Side::Right)));
        assert!(ModifierState::Down(Side::Either).overlaps(ModifierState::Any));
    }

    #[test]
    fn rules_check_every_family() {
        let ctrl = ModifierRule::NONE.down(ModifierKey::Ctrl);
        assert!(ctrl.accepts(Modifiers::RIGHT_CTRL));
        assert!(!ctrl.accepts(Modifiers::RIGHT_CTRL.union(Modifiers::LEFT_SHIFT)));
        assert!(!ctrl.accepts(Modifiers::NONE));
        let left_ctrl = ModifierRule::NONE.with(ModifierKey::Ctrl, ModifierState::Down(Side::Left));
        assert!(!left_ctrl.accepts(Modifiers::RIGHT_CTRL));
        assert!(ModifierRule::ANY.accepts(Modifiers::LEFT_ALT.union(Modifiers::RIGHT_GUI)));
        assert!(ctrl.overlaps(left_ctrl));
        assert!(!ctrl.overlaps(ModifierRule::NONE));
        assert!(ModifierRule::ANY.overlaps(ctrl));
    }

    #[test]
    fn overlapping_shortcuts_are_the_ones_one_input_triggers_twice() {
        let undo = Shortcut::key(Key::Z).with(ModifierKey::Ctrl);
        let left_undo = Shortcut::key(Key::Z)
            .modifiers(ModifierRule::NONE.with(ModifierKey::Ctrl, ModifierState::Down(Side::Left)));
        assert!(undo.overlaps(left_undo));
        assert!(!undo.overlaps(Shortcut::key(Key::Z)));
        let release = Shortcut::new(Trigger::Key {
            key: Key::Z,
            when: KeyTrigger::Release,
            repeat: false,
        })
        .with(ModifierKey::Ctrl);
        assert!(!undo.overlaps(release));

        let drag_any = Shortcut::button(MouseButton::Left, ButtonTrigger::Drag(Drag::ANY));
        let drag_north = Shortcut::button(
            MouseButton::Left,
            ButtonTrigger::Drag(Drag {
                direction: DragDirection::North,
                distance: Some(20),
            }),
        );
        let drag_south = Shortcut::button(
            MouseButton::Left,
            ButtonTrigger::Drag(Drag {
                direction: DragDirection::South,
                distance: None,
            }),
        );
        assert!(drag_any.overlaps(drag_north));
        assert!(!drag_north.overlaps(drag_south));
        assert!(!drag_any.overlaps(Shortcut::button(MouseButton::Left, ButtonTrigger::Click)));
        assert!(
            Shortcut::new(Trigger::Wheel(WheelDirection::Up))
                .overlaps(Shortcut::new(Trigger::Wheel(WheelDirection::Up)))
        );
    }
}
