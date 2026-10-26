//! [`Input`]: the pointer, the modifiers and the triggered actions as components see them
//! during one frame.

use core::time::Duration;

use koebi_core::Vec2;
use koebi_core::input::Modifiers;
use koebi_input::ActionDef;

/// A pointer button components react to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerButton {
    /// The left mouse button, or the first finger.
    Primary,
    /// The right mouse button: context menus.
    Secondary,
    /// The middle mouse button: panning.
    Middle,
}

impl PointerButton {
    const ALL: [Self; 3] = [Self::Primary, Self::Secondary, Self::Middle];

    fn index(self) -> usize {
        match self {
            Self::Primary => 0,
            Self::Secondary => 1,
            Self::Middle => 2,
        }
    }
}

/// One button's state during a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ButtonState {
    down: bool,
    pressed_at: Option<Vec2>,
    released: bool,
}

/// Actions triggered during a frame, in order.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Actions {
    items: [Option<&'static ActionDef>; Actions::CAPACITY],
    len: usize,
}

impl Actions {
    /// More actions between two frames are dropped: a person cannot trigger that many.
    const CAPACITY: usize = 16;

    fn push(&mut self, action: &'static ActionDef) {
        if let Some(slot) = self.items.get_mut(self.len) {
            *slot = Some(action);
            self.len += 1;
        } else {
            koebi_core::debug!("action {} dropped: too many in one frame", action.id);
        }
    }

    fn iter(&self) -> impl Iterator<Item = &'static ActionDef> + '_ {
        self.items[..self.len].iter().flatten().copied()
    }
}

/// The input of one frame, fed by the application from platform events and from the keymap.
///
/// Positions are in logical pixels. Call [`Input::end_frame`] after each frame: presses,
/// releases, wheel turns and actions are reported to the next frame only. A press and its
/// release may both happen between two frames (a quick click); components still see the
/// click. Touch screens work through the mouse events the platform synthesizes from the first
/// finger.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Input {
    pointer: Option<Vec2>,
    buttons: [ButtonState; 3],
    wheel: Vec2,
    modifiers: Modifiers,
    actions: Actions,
    /// When the frame is drawn, since a moment the application chose.
    time: Duration,
}

impl Input {
    /// No pointer, nothing pressed.
    pub fn new() -> Self {
        Self::default()
    }

    /// The pointer moved to `position`.
    pub fn pointer_moved(&mut self, position: Vec2) {
        self.pointer = Some(position);
    }

    /// The pointer left the window: nothing is under it any more.
    pub fn pointer_left(&mut self) {
        self.pointer = None;
    }

    /// `button` went down or up at `position`.
    ///
    /// A release without a press seen first (pressed outside the window) is ignored.
    pub fn button(&mut self, button: PointerButton, down: bool, position: Vec2) {
        self.pointer = Some(position);
        let state = &mut self.buttons[button.index()];
        if down && !state.down {
            state.pressed_at = Some(position);
        }
        if !down && state.down {
            state.released = true;
        }
        state.down = down;
    }

    /// The primary button went down or up at `position` (see [`Input::button`]).
    pub fn primary_button(&mut self, down: bool, position: Vec2) {
        self.button(PointerButton::Primary, down, position);
    }

    /// The wheel turned by `delta` notches (fractions for touchpads): positive `y` away from
    /// the user, positive `x` to the right. Turns add up until the frame ends.
    pub fn scrolled(&mut self, delta: Vec2) {
        self.wheel += delta;
    }

    /// The modifier keys held now.
    pub fn set_modifiers(&mut self, modifiers: Modifiers) {
        self.modifiers = modifiers;
    }

    /// The keymap triggered `action`.
    pub fn trigger(&mut self, action: &'static ActionDef) {
        self.actions.push(action);
    }

    /// Sets the time of the frame about to be drawn, measured from a fixed moment (the
    /// application's start): delays and animations count from it.
    pub fn set_time(&mut self, time: Duration) {
        self.time = time;
    }

    /// The time of this frame (see [`Input::set_time`]).
    pub fn time(&self) -> Duration {
        self.time
    }

    /// Forgets the presses, releases, wheel turns and actions reported to the frame that just
    /// ended.
    pub fn end_frame(&mut self) {
        *self = self.settled();
    }

    /// The same pointer, buttons held and modifiers, without this frame's presses, releases,
    /// wheel turns and actions.
    ///
    /// When a frame changes what components look like (a slider edits a token, a card
    /// switches the theme), draw it again with this input: components show the new state in
    /// the same frame, without handling anything twice.
    #[must_use]
    pub fn settled(&self) -> Self {
        let mut settled = Self {
            wheel: Vec2::ZERO,
            actions: Actions::default(),
            ..*self
        };
        for state in &mut settled.buttons {
            state.pressed_at = None;
            state.released = false;
        }
        settled
    }

    /// Where the pointer is, if it is over the window.
    pub fn pointer(&self) -> Option<Vec2> {
        self.pointer
    }

    /// `true` while `button` is down.
    pub fn is_button_down(&self, button: PointerButton) -> bool {
        self.buttons[button.index()].down
    }

    /// Where `button` went down since the last frame, if it did.
    pub fn button_pressed_at(&self, button: PointerButton) -> Option<Vec2> {
        self.buttons[button.index()].pressed_at
    }

    /// `true` when `button` went up since the last frame.
    pub fn button_released(&self, button: PointerButton) -> bool {
        self.buttons[button.index()].released
    }

    /// `true` while the primary button is down.
    pub fn is_down(&self) -> bool {
        self.is_button_down(PointerButton::Primary)
    }

    /// Where the primary button went down since the last frame, if it did.
    pub fn pressed_at(&self) -> Option<Vec2> {
        self.button_pressed_at(PointerButton::Primary)
    }

    /// `true` when the primary button went up since the last frame.
    pub fn released(&self) -> bool {
        self.button_released(PointerButton::Primary)
    }

    /// `true` when any button went down since the last frame.
    pub fn any_pressed(&self) -> bool {
        PointerButton::ALL
            .iter()
            .any(|button| self.button_pressed_at(*button).is_some())
    }

    /// `true` when something happened since the last frame besides pointer moves: a button
    /// went down or up, the wheel turned, or the keymap triggered an action. Such a frame may
    /// change what parts drawn before the change show.
    pub fn has_events(&self) -> bool {
        self.any_pressed()
            || PointerButton::ALL
                .iter()
                .any(|button| self.button_released(*button))
            || self.wheel != Vec2::ZERO
            || self.actions.len > 0
    }

    /// The wheel turns since the last frame, in notches.
    pub fn wheel(&self) -> Vec2 {
        self.wheel
    }

    /// The modifier keys held.
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// `true` when the keymap triggered `action` since the last frame.
    pub fn triggered(&self, action: &ActionDef) -> bool {
        self.actions.iter().any(|triggered| triggered == action)
    }

    /// The actions triggered since the last frame, in order.
    pub fn actions(&self) -> impl Iterator<Item = &'static ActionDef> + '_ {
        self.actions.iter()
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::input::Key;
    use koebi_input::{Context, Shortcut};

    use super::*;

    const AT: Vec2 = Vec2::new(4.0, 2.0);

    static CLOSE: ActionDef = ActionDef {
        id: "test.close",
        label: "Close",
        description: "Closes.",
        context: Context::new("test"),
        defaults: &[Shortcut::key(Key::Escape)],
    };

    #[test]
    fn presses_and_releases_last_one_frame() {
        let mut input = Input::new();
        input.primary_button(true, AT);
        assert_eq!((input.pressed_at(), input.is_down()), (Some(AT), true));
        input.end_frame();
        assert_eq!((input.pressed_at(), input.is_down()), (None, true));
        input.primary_button(false, AT);
        assert!(input.released() && !input.is_down());
        input.end_frame();
        assert!(!input.released());
    }

    #[test]
    fn a_quick_click_reports_both_transitions() {
        let mut input = Input::new();
        input.primary_button(true, AT);
        input.primary_button(false, AT);
        assert_eq!(input.pressed_at(), Some(AT));
        assert!(input.released() && !input.is_down());
    }

    #[test]
    fn buttons_are_tracked_separately() {
        let mut input = Input::new();
        input.button(PointerButton::Secondary, true, AT);
        assert_eq!(input.button_pressed_at(PointerButton::Secondary), Some(AT));
        assert_eq!(input.pressed_at(), None);
        assert!(input.any_pressed() && !input.is_down());
        input.button(PointerButton::Middle, true, AT);
        input.end_frame();
        assert!(!input.any_pressed());
        assert!(input.is_button_down(PointerButton::Middle));
        input.button(PointerButton::Secondary, false, AT);
        assert!(input.button_released(PointerButton::Secondary));
        assert!(!input.button_released(PointerButton::Middle));
    }

    #[test]
    fn settled_input_keeps_the_pointer_but_not_the_transitions() {
        let mut input = Input::new();
        input.primary_button(true, AT);
        input.scrolled(Vec2::new(0.0, 1.0));
        input.scrolled(Vec2::new(0.5, 1.0));
        input.trigger(&CLOSE);
        input.set_modifiers(Modifiers::LEFT_SHIFT);
        assert_eq!(input.wheel(), Vec2::new(0.5, 2.0));
        assert!(input.triggered(&CLOSE));
        let settled = input.settled();
        assert_eq!((settled.pressed_at(), settled.is_down()), (None, true));
        assert_eq!(settled.pointer(), Some(AT));
        assert_eq!(settled.wheel(), Vec2::ZERO);
        assert!(!settled.triggered(&CLOSE));
        assert_eq!(settled.modifiers(), Modifiers::LEFT_SHIFT);
        input.primary_button(false, AT);
        assert!(!input.settled().released());
    }

    #[test]
    fn a_release_without_a_press_is_ignored() {
        let mut input = Input::new();
        input.primary_button(false, AT);
        assert!(!input.released());
        assert_eq!(input.pointer(), Some(AT));
        input.pointer_left();
        assert_eq!(input.pointer(), None);
    }

    #[test]
    fn actions_beyond_the_capacity_are_dropped() {
        let mut input = Input::new();
        for _ in 0..Actions::CAPACITY + 3 {
            input.trigger(&CLOSE);
        }
        assert_eq!(input.actions().count(), Actions::CAPACITY);
        input.end_frame();
        assert_eq!(input.actions().count(), 0);
    }
}
