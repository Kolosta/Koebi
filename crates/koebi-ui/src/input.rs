//! [`Input`]: the pointer as components see it during one frame.

use koebi_core::Vec2;

/// The pointer state of one frame, fed by the application from platform events.
///
/// Positions are in logical pixels. Call [`Input::end_frame`] after each frame: a press or a
/// release is reported to the next frame only. A press and its release may both happen
/// between two frames (a quick click); components still see the click. Touch screens work
/// through the mouse events the platform synthesizes from the first finger.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Input {
    pointer: Option<Vec2>,
    down: bool,
    pressed_at: Option<Vec2>,
    released: bool,
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

    /// The primary button (left mouse button, first finger) went down or up at `position`.
    ///
    /// A release without a press seen first (pressed outside the window) is ignored.
    pub fn primary_button(&mut self, down: bool, position: Vec2) {
        self.pointer = Some(position);
        if down && !self.down {
            self.pressed_at = Some(position);
        }
        if !down && self.down {
            self.released = true;
        }
        self.down = down;
    }

    /// Forgets the presses and releases reported to the frame that just ended.
    pub fn end_frame(&mut self) {
        self.pressed_at = None;
        self.released = false;
    }

    /// The same pointer without this frame's press and release.
    ///
    /// When a frame changes what components look like (a slider edits a token, a card
    /// switches the theme), draw it again with this input: components show the new state in
    /// the same frame, without handling the press or the release twice.
    #[must_use]
    pub fn settled(&self) -> Self {
        Self {
            pressed_at: None,
            released: false,
            ..*self
        }
    }

    /// Where the pointer is, if it is over the window.
    pub fn pointer(&self) -> Option<Vec2> {
        self.pointer
    }

    /// `true` while the primary button is down.
    pub fn is_down(&self) -> bool {
        self.down
    }

    /// Where the primary button went down since the last frame, if it did.
    pub fn pressed_at(&self) -> Option<Vec2> {
        self.pressed_at
    }

    /// `true` when the primary button went up since the last frame.
    pub fn released(&self) -> bool {
        self.released
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT: Vec2 = Vec2::new(4.0, 2.0);

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
    fn settled_input_keeps_the_pointer_but_not_the_transitions() {
        let mut input = Input::new();
        input.primary_button(true, AT);
        let settled = input.settled();
        assert_eq!((settled.pressed_at(), settled.is_down()), (None, true));
        assert_eq!(settled.pointer(), Some(AT));
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
}
