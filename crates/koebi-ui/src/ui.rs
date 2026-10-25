//! [`Ui`]: one frame of immediate-mode interface, and the interaction state between frames.

use core::time::Duration;

use koebi_core::input::CursorShape;
use koebi_core::{Rect, Vec2};
use koebi_input::{ActionDef, Context};
use koebi_render::Scene;
use koebi_text::{Fonts, TextLine};

use crate::id::WidgetId;
use crate::input::{Input, PointerButton};
use crate::routing::{Layer, Region, Regions};
use crate::style::Styles;

/// What must outlive a frame: the widget that holds the pointer, where it took it, and the
/// regions of the last frame, which decide what is under the pointer.
///
/// Keep one per window and pass it to every [`Ui`].
#[derive(Clone, Debug, Default)]
pub struct UiState {
    active: Option<WidgetId>,
    /// Pointer position when the active widget was pressed.
    press_origin: Vec2,
    /// The active widget's value when it was pressed: drag values move relative to it.
    press_value: f32,
    /// Whether the active widget took part in the current frame.
    active_seen: bool,
    /// The regions of the last complete frame, and of the frame being built.
    previous: Regions,
    current: Regions,
    /// The widget on top under the pointer, and under the press, resolved when a frame starts.
    hovered: Option<WidgetId>,
    press_target: Option<WidgetId>,
    /// The widget on top where the secondary button went down.
    secondary_target: Option<WidgetId>,
    cursor: CursorShape,
    /// Contexts that take the keyboard whatever the pointer is over (an open menu), from the
    /// last complete frame and from the frame being built, innermost last.
    grabs: Vec<Context>,
    grabbing: Vec<Context>,
    /// The fonts labels are laid out with, and a line reused for each label.
    pub(crate) fonts: Fonts,
    pub(crate) line: TextLine,
    /// When the last frame wants to be drawn again, if it does.
    redraw_at: Option<Duration>,
}

impl UiState {
    /// Nothing holds the pointer.
    pub fn new() -> Self {
        Self::default()
    }

    /// When the last frame asked to be drawn again (an animation, a delay running out), on the
    /// time scale of [`Input::time`]: the application redraws then, even without input.
    pub fn redraw_at(&self) -> Option<Duration> {
        self.redraw_at
    }

    /// Sets the fonts labels are laid out with: the collection the renderer rasterizes
    /// glyphs from. Without fonts, labels draw nothing.
    pub fn set_fonts(&mut self, fonts: Fonts) {
        self.fonts = fonts;
    }

    /// The widget that holds the pointer: pressed, and not released yet.
    pub fn active(&self) -> Option<WidgetId> {
        self.active
    }

    /// The widget on top under the pointer when the frame started.
    pub fn hovered(&self) -> Option<WidgetId> {
        self.hovered
    }

    /// The cursor the interface asks for: the one of the widget that holds the pointer, or of
    /// what is on top under it.
    pub fn cursor(&self) -> CursorShape {
        self.cursor
    }

    /// The shortcut contexts under `point` in the last complete frame, innermost first,
    /// replacing the content of `chain` (pass the same vector every time: no allocation).
    pub fn contexts_at(&self, point: Vec2, chain: &mut Vec<Context>) {
        self.previous.contexts_at(point, chain);
    }

    /// The chain of contexts an input event at `point` goes through, innermost first: the
    /// contexts grabbing the keyboard ([`Ui::grab`]), then those under the point.
    pub fn chain_at(&self, point: Option<Vec2>, chain: &mut Vec<Context>) {
        match point {
            Some(point) => self.previous.contexts_at(point, chain),
            None => chain.clear(),
        }
        for (index, context) in self.grabs.iter().rev().enumerate() {
            chain.insert(index, *context);
        }
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
    /// The component is on top under the pointer, and no other component holds the pointer.
    pub hovered: bool,
    /// The primary button went down on the component this frame.
    pub pressed: bool,
    /// The component holds the pointer: it was pressed and is not released yet.
    pub held: bool,
    /// The pointer was released over the component after pressing it.
    pub clicked: bool,
    /// The secondary button went down on the component this frame: open its context menu.
    pub secondary_pressed: bool,
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
            secondary_pressed: self.secondary_pressed || part.secondary_pressed,
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
/// the [`UiState`]. What is on top under the pointer receives it: the highest [`Layer`], then
/// the component drawn last (see [`crate::Layer`]); nothing under it is hovered or clicked.
#[derive(Debug)]
pub struct Ui<'a> {
    pub(crate) scene: &'a mut Scene,
    styles: &'a Styles,
    input: &'a Input,
    pub(crate) state: &'a mut UiState,
    layer: Layer,
    /// The innermost zone being drawn, by region index.
    zone: Option<u32>,
    /// Regions are clipped to this rectangle.
    clip: Option<Rect>,
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
        state.current.clear();
        let topmost = |point: Option<Vec2>| {
            point
                .and_then(|point| state.previous.topmost(point))
                .and_then(|(_, region)| region.widget)
        };
        state.hovered = topmost(input.pointer());
        state.press_target = topmost(input.pressed_at());
        state.secondary_target = topmost(input.button_pressed_at(PointerButton::Secondary));
        state.grabbing.clear();
        state.redraw_at = None;
        Self {
            scene,
            styles,
            input,
            state,
            layer: Layer::BASE,
            zone: None,
            clip: None,
        }
    }

    /// The styles components draw with.
    pub fn styles(&self) -> &'a Styles {
        self.styles
    }

    /// The input of this frame.
    pub fn input(&self) -> &'a Input {
        self.input
    }

    /// `true` when the keymap triggered `action` since the last frame.
    pub fn triggered(&self, action: &ActionDef) -> bool {
        self.input.triggered(action)
    }

    /// Asks for the frame to be drawn again at `time` ([`Input::time`] scale), even without
    /// input: the end of a delay, the next step of an animation.
    pub fn request_redraw_at(&mut self, time: Duration) {
        let state = &mut *self.state;
        state.redraw_at = Some(state.redraw_at.map_or(time, |at| at.min(time)));
    }

    /// Asks for the frame to be drawn again `delay` after this one.
    pub fn request_redraw_after(&mut self, delay: Duration) {
        self.request_redraw_at(self.input.time() + delay);
    }

    /// Takes the keyboard for the shortcuts of `context` in the next frame, wherever the
    /// pointer is: an open menu or an operation in progress. Call it every frame the grab
    /// lasts; later grabs come first in the chain.
    pub fn grab(&mut self, context: Context) {
        self.state.grabbing.push(context);
    }

    /// The scene, for custom drawing between components.
    pub fn scene(&mut self) -> &mut Scene {
        self.scene
    }

    /// The layer components are drawn on.
    pub fn layer(&self) -> Layer {
        self.layer
    }

    /// Draws with `add` on `layer`, above or below what the current layer holds.
    pub fn on_layer<R>(&mut self, layer: Layer, add: impl FnOnce(&mut Self) -> R) -> R {
        let previous = core::mem::replace(&mut self.layer, layer);
        let result = add(self);
        self.layer = previous;
        result
    }

    /// Draws with `add` clipped to `clip`, and hit tests its components only inside it (a
    /// scrolling list: rows scrolled out of view are neither drawn nor clicked).
    pub fn clipped<R>(&mut self, clip: Rect, add: impl FnOnce(&mut Self) -> R) -> R {
        let clip = match self.clip {
            Some(outer) => outer.intersection(&clip).unwrap_or(Rect::ZERO),
            None => clip,
        };
        let previous = self.clip.replace(clip);
        self.scene.push_clip(clip);
        let result = add(self);
        self.scene.pop_clip();
        self.clip = previous;
        result
    }

    fn register(
        &mut self,
        rect: Rect,
        widget: Option<WidgetId>,
        context: Option<Context>,
        cursor: Option<CursorShape>,
    ) -> u32 {
        let rect = match self.clip {
            Some(clip) => rect.intersection(&clip).unwrap_or(Rect::ZERO),
            None => rect,
        };
        self.state.current.push(Region {
            rect,
            layer: self.layer,
            widget,
            context,
            cursor,
            parent: self.zone,
        })
    }

    /// A zone: a part of the interface where the shortcuts of `context` apply, and that
    /// receives the pointer where no component of `add` does. Zones nest: the chain of zones
    /// under the pointer is the chain of contexts the keymap resolves, innermost first.
    pub fn zone<R>(&mut self, rect: Rect, context: Context, add: impl FnOnce(&mut Self) -> R) -> R {
        let index = self.register(rect, None, Some(context), None);
        let outer = self.zone.replace(index);
        let result = add(self);
        self.zone = outer;
        result
    }

    /// Like [`Self::zone`], showing `cursor` over the zone where nothing inside asks for
    /// another one.
    pub fn zone_with_cursor<R>(
        &mut self,
        rect: Rect,
        context: Context,
        cursor: CursorShape,
        add: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let index = self.register(rect, None, Some(context), Some(cursor));
        let outer = self.zone.replace(index);
        let result = add(self);
        self.zone = outer;
        result
    }

    /// Takes the pointer over `rect` without reacting to it: the background of a floating
    /// panel, so that what lies under the panel receives nothing.
    pub fn block(&mut self, rect: Rect) {
        self.register(rect, None, None, None);
    }

    /// Hit-tests `rect` for the widget `id` and tracks which widget holds the pointer: the
    /// building block of every interactive component.
    pub fn interact(&mut self, id: WidgetId, rect: Rect) -> Response {
        self.interact_inner(id, rect, None)
    }

    /// Like [`Self::interact`], showing `cursor` while the pointer is over the widget or the
    /// widget holds it (a gutter shows a resize cursor during the whole drag).
    pub fn interact_with_cursor(
        &mut self,
        id: WidgetId,
        rect: Rect,
        cursor: CursorShape,
    ) -> Response {
        self.interact_inner(id, rect, Some(cursor))
    }

    fn interact_inner(
        &mut self,
        id: WidgetId,
        rect: Rect,
        cursor: Option<CursorShape>,
    ) -> Response {
        self.register(rect, Some(id), None, cursor);
        let mut response = Response::default();
        if self.state.active.is_none()
            && self.state.press_target == Some(id)
            && let Some(origin) = self.input.pressed_at()
        {
            self.state.active = Some(id);
            self.state.press_origin = origin;
            response.pressed = true;
        }
        response.secondary_pressed = self.state.secondary_target == Some(id)
            && self
                .input
                .button_pressed_at(PointerButton::Secondary)
                .is_some();
        let on_top = self.state.hovered == Some(id);
        let active = self.state.active == Some(id);
        response.hovered = on_top && (active || self.state.active.is_none());
        if active {
            self.state.active_seen = true;
            response.held = self.input.is_down();
            response.drag_delta = self
                .input
                .pointer()
                .map_or(Vec2::ZERO, |pointer| pointer - self.state.press_origin);
            if self.input.released() {
                response.released = true;
                response.clicked = on_top;
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
        // The cursor shown until the next frame comes from what this frame drew: the widget
        // holding the pointer, or what is on top under it.
        let state = &mut *self.state;
        let held = state
            .active
            .and_then(|widget| state.current.of_widget(widget))
            .and_then(|region| region.cursor);
        state.cursor = held
            .or_else(|| {
                self.input
                    .pointer()
                    .and_then(|point| state.current.cursor_at(point))
            })
            .unwrap_or_default();
        // The regions and grabs of this frame decide where the next one's input goes.
        core::mem::swap(&mut state.previous, &mut state.current);
        core::mem::swap(&mut state.grabs, &mut state.grabbing);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: WidgetId = WidgetId::new("a");
    const B: WidgetId = WidgetId::new("b");

    fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect::from_origin_size(Vec2::new(x, y), Vec2::new(width, height))
    }

    fn rect_a() -> Rect {
        rect(0.0, 0.0, 10.0, 10.0)
    }

    fn rect_b() -> Rect {
        rect(20.0, 0.0, 10.0, 10.0)
    }

    /// Runs one frame with widgets A and B, then ends the frame for the input. The first
    /// frame of a state only lays widgets out: hit testing uses the previous frame.
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

    /// A state whose previous frame laid A and B out.
    fn laid_out() -> (UiState, Input) {
        let (mut state, mut input) = (UiState::new(), Input::new());
        frame(&mut state, &mut input);
        (state, input)
    }

    #[test]
    fn secondary_presses_reach_the_widget_on_top_only() {
        let (mut state, mut input) = laid_out();
        input.button(PointerButton::Secondary, true, Vec2::new(5.0, 5.0));
        let (a, b) = frame(&mut state, &mut input);
        assert!(a.secondary_pressed && !b.secondary_pressed);
        // The secondary button never takes the pointer.
        assert!(!a.pressed && state.active().is_none());
        let (a, _) = frame(&mut state, &mut input);
        assert!(!a.secondary_pressed);
    }

    #[test]
    fn grabs_come_first_in_the_chain_for_the_next_frame() {
        let styles = crate::test_styles();
        let (mut scene, mut state, input) = (Scene::new(), UiState::new(), Input::new());
        let (app, menu, modal) = (
            Context::new("app"),
            Context::new("menu"),
            Context::new("modal"),
        );
        {
            let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
            ui.zone(rect(0.0, 0.0, 100.0, 100.0), app, |ui| {
                ui.grab(menu);
                ui.grab(modal);
            });
        }
        let mut chain = Vec::new();
        state.chain_at(Some(Vec2::new(5.0, 5.0)), &mut chain);
        assert_eq!(chain, [modal, menu, app]);
        state.chain_at(None, &mut chain);
        assert_eq!(chain, [modal, menu]);
        // A frame without grabs releases the keyboard.
        drop(Ui::new(&mut scene, &styles, &input, &mut state));
        state.chain_at(Some(Vec2::new(5.0, 5.0)), &mut chain);
        assert_eq!(chain, Vec::<Context>::new());
    }

    #[test]
    fn hover_follows_the_pointer() {
        let (mut state, mut input) = laid_out();
        input.pointer_moved(Vec2::new(5.0, 5.0));
        let (a, b) = frame(&mut state, &mut input);
        assert!(a.hovered && !b.hovered);
        assert_eq!(a.state(), WidgetState::Hovered);
        assert_eq!(state.hovered(), Some(A));
    }

    #[test]
    fn press_then_release_inside_clicks() {
        let (mut state, mut input) = laid_out();
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
        let (mut state, mut input) = laid_out();
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
        let (mut state, mut input) = laid_out();
        input.primary_button(true, Vec2::new(25.0, 5.0));
        input.primary_button(false, Vec2::new(25.0, 5.0));
        let (a, b) = frame(&mut state, &mut input);
        assert!(!a.pressed && b.pressed && b.clicked);
        assert_eq!(state.active(), None);
    }

    #[test]
    fn presses_on_empty_space_hold_nothing() {
        let (mut state, mut input) = laid_out();
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
        let (mut state, mut input) = laid_out();
        input.primary_button(true, Vec2::new(5.0, 5.0));
        frame(&mut state, &mut input);
        assert_eq!(state.active(), Some(A));
        // A frame without widget A.
        let styles = crate::test_styles();
        let mut scene = Scene::new();
        drop(Ui::new(&mut scene, &styles, &input, &mut state));
        assert_eq!(state.active(), None);
    }

    /// Runs `draw` twice: once to lay out, once with `input`.
    fn interact_twice(
        state: &mut UiState,
        input: &mut Input,
        mut draw: impl FnMut(&mut Ui<'_>) -> Vec<Response>,
    ) -> Vec<Response> {
        let styles = crate::test_styles();
        let mut scene = Scene::new();
        let pending = *input;
        let mut idle = pending.settled();
        idle.end_frame();
        drop(draw(&mut Ui::new(&mut scene, &styles, &idle, state)));
        scene.clear();
        let responses = draw(&mut Ui::new(&mut scene, &styles, input, state));
        input.end_frame();
        responses
    }

    #[test]
    fn the_component_drawn_last_is_on_top() {
        let (mut state, mut input) = (UiState::new(), Input::new());
        input.primary_button(true, Vec2::new(5.0, 5.0));
        let responses = interact_twice(&mut state, &mut input, |ui| {
            vec![
                ui.interact(A, rect_a()),
                ui.interact(B, rect(0.0, 0.0, 8.0, 8.0)),
            ]
        });
        assert!(!responses[0].hovered && !responses[0].pressed);
        assert!(responses[1].hovered && responses[1].pressed);
    }

    #[test]
    fn floating_layers_are_above_whatever_the_order() {
        let (mut state, mut input) = (UiState::new(), Input::new());
        input.primary_button(true, Vec2::new(5.0, 5.0));
        let responses = interact_twice(&mut state, &mut input, |ui| {
            let menu = ui.on_layer(Layer::FLOATING, |ui| ui.interact(A, rect_a()));
            vec![menu, ui.interact(B, rect_a())]
        });
        assert!(responses[0].pressed && !responses[1].pressed);
    }

    #[test]
    fn a_blocking_panel_shields_what_lies_under_it() {
        let (mut state, mut input) = (UiState::new(), Input::new());
        input.primary_button(true, Vec2::new(5.0, 5.0));
        let responses = interact_twice(&mut state, &mut input, |ui| {
            let under = ui.interact(A, rect_a());
            ui.on_layer(Layer::FLOATING, |ui| ui.block(rect(0.0, 0.0, 50.0, 50.0)));
            vec![under]
        });
        assert!(!responses[0].hovered && !responses[0].pressed);
        assert_eq!(state.active(), None);
    }

    #[test]
    fn clipped_components_receive_nothing_outside_the_clip() {
        let (mut state, mut input) = (UiState::new(), Input::new());
        input.primary_button(true, Vec2::new(5.0, 15.0));
        let responses = interact_twice(&mut state, &mut input, |ui| {
            ui.clipped(rect(0.0, 0.0, 100.0, 10.0), |ui| {
                // A row scrolled half out of view.
                vec![ui.interact(A, rect(0.0, 5.0, 10.0, 20.0))]
            })
        });
        assert!(!responses[0].pressed);
    }

    #[test]
    fn zones_give_the_context_chain_and_cursor_under_the_pointer() {
        let (mut state, mut input) = (UiState::new(), Input::new());
        input.pointer_moved(Vec2::new(5.0, 5.0));
        interact_twice(&mut state, &mut input, |ui| {
            ui.zone(rect(0.0, 0.0, 100.0, 100.0), Context::new("app"), |ui| {
                ui.zone_with_cursor(
                    rect(0.0, 0.0, 50.0, 50.0),
                    Context::new("area"),
                    CursorShape::Crosshair,
                    |ui| {
                        vec![ui.interact_with_cursor(
                            B,
                            rect(30.0, 30.0, 10.0, 10.0),
                            CursorShape::ResizeHorizontal,
                        )]
                    },
                )
            })
        });
        let mut chain = Vec::new();
        state.contexts_at(Vec2::new(5.0, 5.0), &mut chain);
        assert_eq!(chain, [Context::new("area"), Context::new("app")]);
        state.contexts_at(Vec2::new(35.0, 35.0), &mut chain);
        assert_eq!(chain, [Context::new("area"), Context::new("app")]);
        state.contexts_at(Vec2::new(75.0, 75.0), &mut chain);
        assert_eq!(chain, [Context::new("app")]);
        assert_eq!(state.cursor(), CursorShape::Crosshair);
    }

    #[test]
    fn a_held_widget_keeps_its_cursor_wherever_the_pointer_goes() {
        let (mut state, mut input) = (UiState::new(), Input::new());
        input.primary_button(true, Vec2::new(5.0, 5.0));
        let mut draw = |ui: &mut Ui<'_>| {
            vec![ui.interact_with_cursor(A, rect_a(), CursorShape::ResizeVertical)]
        };
        interact_twice(&mut state, &mut input, &mut draw);
        assert_eq!(state.active(), Some(A));
        input.pointer_moved(Vec2::new(80.0, 80.0));
        interact_twice(&mut state, &mut input, &mut draw);
        assert_eq!(state.cursor(), CursorShape::ResizeVertical);
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
