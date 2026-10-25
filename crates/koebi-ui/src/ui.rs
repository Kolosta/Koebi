//! [`Ui`]: one frame of immediate-mode interface, and the interaction state between frames.

use koebi_core::{Rect, Vec2};
use koebi_render::Scene;

use crate::id::WidgetId;
use crate::input::Input;
use crate::style::Styles;

/// What must outlive a frame: the widget that holds the pointer, and where it took it.
///
/// Keep one per window and pass it to every [`Ui`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiState {
    active: Option<WidgetId>,
    /// Pointer position when the active widget was pressed.
    press_origin: Vec2,
    /// The active widget's value when it was pressed: drag values move relative to it.
    press_value: f32,
    /// Whether the active widget took part in the current frame.
    active_seen: bool,
}

impl UiState {
    /// Nothing holds the pointer.
    pub fn new() -> Self {
        Self::default()
    }

    /// The widget that holds the pointer: pressed, and not released yet.
    pub fn active(&self) -> Option<WidgetId> {
        self.active
    }
}

/// The visual state of a component.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetState {
    /// At rest.
    #[default]
    Normal,
    /// Under the pointer.
    Hovered,
    /// Held down by the pointer.
    Pressed,
}

/// What happened to a component this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent facts that callers test one by one, as `response.clicked`"
)]
pub struct Response {
    /// The pointer is over the component, and no other component holds it.
    pub hovered: bool,
    /// The primary button went down on the component this frame.
    pub pressed: bool,
    /// The component holds the pointer: it was pressed and is not released yet.
    pub held: bool,
    /// The pointer was released over the component after pressing it.
    pub clicked: bool,
    /// The component let go of the pointer this frame: a click or the end of a drag.
    pub released: bool,
    /// The component changed its value this frame.
    pub changed: bool,
    /// How far the pointer moved since the press, while the component holds it.
    pub drag_delta: Vec2,
}

impl Response {
    /// The visual state of a push button: pressed while held under the pointer (moving away
    /// before releasing cancels the click), hovered, or normal.
    pub fn state(&self) -> WidgetState {
        match (self.held, self.hovered) {
            (true, true) => WidgetState::Pressed,
            (_, true) => WidgetState::Hovered,
            _ => WidgetState::Normal,
        }
    }

    /// The visual state of a dragged component: pressed for the whole drag, wherever the
    /// pointer goes.
    pub fn drag_state(&self) -> WidgetState {
        if self.held {
            WidgetState::Pressed
        } else {
            self.state()
        }
    }

    /// Combines the responses of the parts of one component.
    #[must_use]
    pub(crate) fn merge(self, part: Self) -> Self {
        Self {
            hovered: self.hovered || part.hovered,
            pressed: self.pressed || part.pressed,
            held: self.held || part.held,
            clicked: self.clicked || part.clicked,
            released: self.released || part.released,
            changed: self.changed || part.changed,
            drag_delta: if self.held {
                self.drag_delta
            } else {
                part.drag_delta
            },
        }
    }
}

/// One frame of interface: components draw into a [`Scene`] and react to the [`Input`].
///
/// Create one per frame, call components on it, then drop it: dropping ends the frame for
/// the [`UiState`]. Components are hit-tested in drawing order and must not overlap: where
/// they do, the first one drawn takes the press.
#[derive(Debug)]
pub struct Ui<'a> {
    pub(crate) scene: &'a mut Scene,
    styles: &'a Styles,
    input: &'a Input,
    state: &'a mut UiState,
}

impl<'a> Ui<'a> {
    /// Starts a frame. Clear the scene first, as components only add to it.
    pub fn new(
        scene: &'a mut Scene,
        styles: &'a Styles,
        input: &'a Input,
        state: &'a mut UiState,
    ) -> Self {
        state.active_seen = false;
        Self {
            scene,
            styles,
            input,
            state,
        }
    }

    /// The styles components draw with.
    pub fn styles(&self) -> &'a Styles {
        self.styles
    }

    /// The pointer state of this frame.
    pub fn input(&self) -> &'a Input {
        self.input
    }

    /// The scene, for custom drawing between components.
    pub fn scene(&mut self) -> &mut Scene {
        self.scene
    }

    /// Hit-tests `rect` for the widget `id` and tracks which widget holds the pointer: the
    /// building block of every interactive component.
    pub fn interact(&mut self, id: WidgetId, rect: Rect) -> Response {
        let contains = |point: Option<Vec2>| point.is_some_and(|point| rect.contains(point));
        let mut response = Response::default();
        if self.state.active.is_none() && contains(self.input.pressed_at()) {
            self.state.active = Some(id);
            self.state.press_origin = self.input.pressed_at().unwrap_or_default();
            response.pressed = true;
        }
        let pointer_over = contains(self.input.pointer());
        let active = self.state.active == Some(id);
        response.hovered = pointer_over && (active || self.state.active.is_none());
        if active {
            self.state.active_seen = true;
            response.held = self.input.is_down();
            response.drag_delta = self
                .input
                .pointer()
                .map_or(Vec2::ZERO, |pointer| pointer - self.state.press_origin);
            if self.input.released() {
                response.released = true;
                response.clicked = pointer_over;
                self.state.active = None;
            }
        }
        response
    }

    /// The value recorded by [`Ui::set_press_value`] when the active widget was pressed.
    pub(crate) fn press_value(&self) -> f32 {
        self.state.press_value
    }

    /// Records the value of the widget pressed this frame.
    pub(crate) fn set_press_value(&mut self, value: f32) {
        self.state.press_value = value;
    }
}

impl Drop for Ui<'_> {
    fn drop(&mut self) {
        // A widget that was not drawn this frame cannot keep the pointer, and a release
        // always frees it (even when the holder skipped its `interact` call).
        if !self.state.active_seen || self.input.released() {
            self.state.active = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: WidgetId = WidgetId::new("a");
    const B: WidgetId = WidgetId::new("b");

    fn rect_a() -> Rect {
        Rect::from_origin_size(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0))
    }

    fn rect_b() -> Rect {
        Rect::from_origin_size(Vec2::new(20.0, 0.0), Vec2::new(10.0, 10.0))
    }

    /// Runs one frame with widgets A and B, then ends the frame for the input.
    fn frame(state: &mut UiState, input: &mut Input) -> (Response, Response) {
        let styles = crate::test_styles();
        let mut scene = Scene::new();
        let responses = {
            let mut ui = Ui::new(&mut scene, &styles, input, state);
            (ui.interact(A, rect_a()), ui.interact(B, rect_b()))
        };
        input.end_frame();
        responses
    }

    #[test]
    fn hover_follows_the_pointer() {
        let (mut state, mut input) = (UiState::new(), Input::new());
        input.pointer_moved(Vec2::new(5.0, 5.0));
        let (a, b) = frame(&mut state, &mut input);
        assert!(a.hovered && !b.hovered);
        assert_eq!(a.state(), WidgetState::Hovered);
    }

    #[test]
    fn press_then_release_inside_clicks() {
        let (mut state, mut input) = (UiState::new(), Input::new());
        input.primary_button(true, Vec2::new(5.0, 5.0));
        let (a, _) = frame(&mut state, &mut input);
        assert!(a.pressed && a.held && !a.clicked);
        assert_eq!(a.state(), WidgetState::Pressed);
        assert_eq!(state.active(), Some(A));

        let (a, _) = frame(&mut state, &mut input);
        assert!(!a.pressed && a.held);

        input.primary_button(false, Vec2::new(6.0, 6.0));
        let (a, _) = frame(&mut state, &mut input);
        assert!(a.clicked && a.released && !a.held);
        assert_eq!(state.active(), None);
    }

    #[test]
    fn releasing_elsewhere_cancels_the_click() {
        let (mut state, mut input) = (UiState::new(), Input::new());
        input.primary_button(true, Vec2::new(5.0, 5.0));
        frame(&mut state, &mut input);
        input.pointer_moved(Vec2::new(25.0, 5.0));
        let (a, b) = frame(&mut state, &mut input);
        // A keeps the pointer: B is not hovered, and A shows as hovered no more.
        assert!(a.held && !a.hovered && !b.hovered);
        assert_eq!(a.state(), WidgetState::Normal);
        assert_eq!(a.drag_state(), WidgetState::Pressed);
        assert_eq!(a.drag_delta, Vec2::new(20.0, 0.0));
        input.primary_button(false, Vec2::new(25.0, 5.0));
        let (a, b) = frame(&mut state, &mut input);
        assert!(a.released && !a.clicked && !b.clicked);
    }

    #[test]
    fn a_quick_click_between_two_frames_still_clicks() {
        let (mut state, mut input) = (UiState::new(), Input::new());
        input.primary_button(true, Vec2::new(25.0, 5.0));
        input.primary_button(false, Vec2::new(25.0, 5.0));
        let (a, b) = frame(&mut state, &mut input);
        assert!(!a.pressed && b.pressed && b.clicked);
        assert_eq!(state.active(), None);
    }

    #[test]
    fn presses_on_empty_space_hold_nothing() {
        let (mut state, mut input) = (UiState::new(), Input::new());
        input.primary_button(true, Vec2::new(15.0, 5.0));
        frame(&mut state, &mut input);
        assert_eq!(state.active(), None);
        // Dragging over a widget with the button down does not press it.
        input.pointer_moved(Vec2::new(5.0, 5.0));
        let (a, _) = frame(&mut state, &mut input);
        assert!(a.hovered && !a.pressed && !a.held);
    }

    #[test]
    fn a_vanished_widget_lets_go_of_the_pointer() {
        let (mut state, mut input) = (UiState::new(), Input::new());
        input.primary_button(true, Vec2::new(5.0, 5.0));
        frame(&mut state, &mut input);
        assert_eq!(state.active(), Some(A));
        // A frame without widget A.
        let styles = crate::test_styles();
        let mut scene = Scene::new();
        drop(Ui::new(&mut scene, &styles, &input, &mut state));
        assert_eq!(state.active(), None);
    }

    #[test]
    fn merged_responses_combine_parts() {
        let body = Response {
            held: true,
            drag_delta: Vec2::new(3.0, 0.0),
            ..Response::default()
        };
        let step = Response {
            hovered: true,
            changed: true,
            ..Response::default()
        };
        let merged = body.merge(step);
        assert!(merged.held && merged.hovered && merged.changed);
        assert_eq!(merged.drag_delta, Vec2::new(3.0, 0.0));
    }
}
