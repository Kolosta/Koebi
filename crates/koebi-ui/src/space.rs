//! Zoomable spaces (ADR 0030): [`Ui::space`] draws and hit-tests what components inside it
//! draw, scaled and moved, with no code of their own; components read the pointer through
//! [`Ui`], in their own units.
//!
//! - **Local units:** inside a space, every rectangle a component computes is in the space's
//!   units, those it would use unscaled. Styles stay the window's: a button inside a space at
//!   0.5 lays itself out exactly as outside, and is drawn at half its size.
//! - **Drawing:** the scene maps everything pushed through the space ([`Scene::push_space`]),
//!   and edges snap to physical pixels after it: a one-pixel border stays crisp at any zoom,
//!   and a hairline thinner than a pixel keeps one pixel and fades.
//! - **Hit testing:** regions are registered mapped to the window: routing is unchanged
//!   (ADR 0013).
//! - **The pointer:** [`Ui::pointer`], [`Ui::pointer_delta`], [`Ui::press_position`] and
//!   [`Ui::button_press_position`] give it in local units, and drags ([`Response::drag_delta`])
//!   divide by the scale: what is dragged follows the hand at any zoom. The application keeps
//!   [`Input`] in window units; a clippy rule (`disallowed-methods` in `clippy.toml`) forbids
//!   reading positions from it in components.
//! - **Floating layers** (menus, dropdown lists, tooltips, pickers: [`Layer::FLOATING`] and
//!   above) draw at the window's scale, anchored to their control mapped to the window: a
//!   dropdown in a node zoomed out to 0.3 opens a readable list beside it.
//! - **Text input:** the caret, the selection and the place of the input method map through
//!   the space like the rest.
//! - **Detail by scale:** [`Ui::scale`] gives the scale on screen and [`Ui::readable`] tells
//!   whether text of a size can still be read there (`component.zoom.readable-size`), so that
//!   every component drops its labels at the same point.
//!
//! [`Scene::push_space`]: koebi_render::Scene::push_space
//! [`Response::drag_delta`]: crate::Response::drag_delta
//! [`Input`]: crate::Input
//! [`Layer::FLOATING`]: crate::Layer::FLOATING

use koebi_core::{Rect, Space, Vec2};
use koebi_design::Resolve;
use koebi_tokens::component;

use crate::input::PointerButton;
use crate::ui::Ui;

/// How zoomed content drops detail, from the `component.zoom.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoomStyle {
    /// The smallest size on screen, in logical pixels, at which text is still read.
    pub readable_size: f32,
}

impl ZoomStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            readable_size: tokens.get(component::ZOOM_READABLE_SIZE_DEFAULT),
        }
    }
}

#[expect(
    clippy::disallowed_methods,
    reason = "this is where the pointer in window units becomes the pointer in local units"
)]
impl Ui<'_> {
    /// Draws and hit-tests what `add` draws in `space`, opened in the space in force: what it
    /// gives in its local units lands at `point * space.scale + space.offset` in the units
    /// around it. Spaces nest. A space that cannot be drawn (a scale that is not positive and
    /// finite) draws nothing and receives nothing.
    ///
    /// What lies inside starts afresh: a collapsible panel in a space is not nested in the
    /// panel holding the space, whose levels and backgrounds are in other units.
    pub fn space<R>(&mut self, space: Space, add: impl FnOnce(&mut Self) -> R) -> R {
        let depth = core::mem::take(&mut self.panel_depth);
        let span = self.panel_span.take();
        self.scene.push_space(space);
        let result = add(self);
        self.scene.pop_space();
        self.panel_depth = depth;
        self.panel_span = span;
        result
    }

    /// The space components are drawn in now: how their local units map to the window's
    /// logical pixels. [`Space::IDENTITY`] outside every space and on floating layers.
    pub fn window_space(&self) -> Space {
        self.scene.space()
    }

    /// How many logical pixels of the window one local unit spans now: 1 outside every space,
    /// 0 in a space that cannot be drawn.
    pub fn scale(&self) -> f32 {
        self.window_space().scale
    }

    /// `true` when text `size` local units high is still read on screen at the scale in force
    /// (`component.zoom.readable-size`): below it, components drop their labels and details,
    /// all at the same point.
    pub fn readable(&self, size: f32) -> bool {
        size * self.scale() >= self.styles().zoom.readable_size
    }

    /// Where the point `point`, in local units, lies in the window.
    pub fn window_point(&self, point: Vec2) -> Vec2 {
        self.window_space().map_point(point)
    }

    /// Where the rectangle `rect`, in local units, lies in the window.
    pub fn window_rect(&self, rect: Rect) -> Rect {
        self.window_space().map_rect(rect)
    }

    /// The pointer, in local units; `None` outside the window, or in a space that cannot be
    /// drawn.
    pub fn pointer(&self) -> Option<Vec2> {
        let space = self.window_space();
        self.input()
            .pointer()
            .filter(|_| space.is_valid())
            .map(|point| space.unmap_point(point))
    }

    /// How far the pointer moved since the last frame, in local units: its relative motion
    /// while it is locked ([`Ui::lock_pointer`]).
    pub fn pointer_delta(&self) -> Vec2 {
        self.window_space().unmap_vector(self.input().motion())
    }

    /// Where the primary button went down this frame, in local units.
    pub fn press_position(&self) -> Option<Vec2> {
        self.button_press_position(PointerButton::Primary)
    }

    /// Where `button` went down this frame, in local units.
    pub fn button_press_position(&self, button: PointerButton) -> Option<Vec2> {
        let space = self.window_space();
        self.input()
            .button_pressed_at(button)
            .filter(|_| space.is_valid())
            .map(|point| space.unmap_point(point))
    }

    /// The pointer in the window's logical pixels, whatever the space: what the interface
    /// itself compares with the regions and menus it keeps in window units.
    pub(crate) fn window_pointer(&self) -> Option<Vec2> {
        self.input().pointer()
    }

    /// Where `button` went down this frame in the window's logical pixels, whatever the space:
    /// what goes to the platform (the window's system menu).
    pub(crate) fn window_press_position(&self, button: PointerButton) -> Option<Vec2> {
        self.input().button_pressed_at(button)
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::Color;
    use koebi_render::Scene;
    use koebi_text::Fonts;

    use super::*;
    use crate::text::Align;
    use crate::{Button, Dropdown, Input, MenuItem, MenuState, TextField, UiState, WidgetId};

    fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect::from_origin_size(Vec2::new(x, y), Vec2::new(width, height))
    }

    /// Regression test: a label cut short was known by its text and its local place, so the
    /// same label in two nodes was one: its tooltip showed below the other node.
    #[test]
    fn labels_cut_short_in_two_spaces_show_their_own_tooltips() {
        let styles = crate::test_styles();
        let mut state = UiState::new();
        state.set_fonts(Fonts::with_embedded());
        state.set_window(rect(0.0, 0.0, 800.0, 600.0));
        let (mut scene, mut input) = (Scene::new(), Input::new());
        let label = rect(4.0, 30.0, 40.0, 20.0);
        let nodes = [
            Space::new(2.0, Vec2::new(10.0, 0.0)),
            Space::new(2.0, Vec2::new(10.0, 200.0)),
        ];
        let mut frame = |input: &mut Input| {
            scene.clear();
            let mut ui = Ui::new(&mut scene, &styles, input, &mut state);
            let font = ui.styles().text.label.clone();
            for node in nodes {
                ui.space(node, |ui| {
                    ui.label(
                        "Displacement factor",
                        label,
                        &font,
                        Color::WHITE,
                        Align::Start,
                    );
                });
            }
            drop(ui);
            input.end_frame();
            state.tooltip.shown.map(|shown| shown.rect)
        };
        frame(&mut input);
        let second = nodes[1].map_rect(label);
        input.pointer_moved(second.center());
        frame(&mut input);
        frame(&mut input);
        input.set_time(styles.input.tooltip_delay);
        let tooltip = frame(&mut input).expect("the tooltip shows");
        // Below the label the pointer rests on, not the other one.
        assert!(
            tooltip.min.y >= second.max.y,
            "{tooltip:?} not below {second:?}"
        );
        assert!(
            tooltip.min.y < second.max.y + 20.0,
            "{tooltip:?} far from {second:?}"
        );
    }

    /// A screen of components in a space zoomed in, a space zoomed out inside it, a clip, a
    /// text field being edited and a dropdown with its menu open at the window's scale: once
    /// steady, its frames allocate nothing (ADR 0026).
    #[test]
    fn steady_frames_in_nested_spaces_allocate_nothing() {
        const MODES: [MenuItem<'static>; 3] = [
            MenuItem::action("Solid"),
            MenuItem::action("Wireframe"),
            MenuItem::action("Rendered"),
        ];
        let styles = crate::test_styles();
        let window = rect(0.0, 0.0, 800.0, 600.0);
        let mut state = UiState::new();
        state.set_fonts(Fonts::with_embedded());
        state.set_window(window);
        let (mut scene, mut input) = (Scene::new(), Input::new());
        let mut menu = MenuState::new();
        let mut text = String::from("A node");
        let outer = Space::new(2.0, Vec2::new(40.0, 30.0));
        let inner = Space::new(0.3, Vec2::new(10.0, 60.0));
        let field = rect(10.0, 40.0, 160.0, 22.0);
        let mut frame = |input: &mut Input| {
            scene.clear();
            let mut ui = Ui::new(&mut scene, &styles, input, &mut state);
            let font = ui.styles().text.label.clone();
            ui.space(outer, |ui| {
                Button::new(WidgetId::new("apply"))
                    .label("Apply")
                    .show(ui, rect(10.0, 10.0, 80.0, 22.0));
                TextField::new(WidgetId::new("name"), &mut text).show(ui, field);
                ui.space(inner, |ui| {
                    ui.clipped(rect(0.0, 0.0, 300.0, 200.0), |ui| {
                        ui.label(
                            "Socket",
                            rect(4.0, 4.0, 120.0, 20.0),
                            &font,
                            Color::WHITE,
                            Align::Start,
                        );
                        Dropdown::new(WidgetId::new("mode"), &mut menu, &MODES)
                            .label("Solid")
                            .show(ui, rect(4.0, 30.0, 120.0, 22.0), window);
                    });
                });
            });
            drop(ui);
            input.end_frame();
            menu.is_open()
        };
        frame(&mut input);
        // Edit the field, open the dropdown, the pointer resting on its menu.
        let at = outer.map_point(field.center());
        input.primary_button(true, at);
        input.primary_button(false, at);
        frame(&mut input);
        let dropdown = outer
            .then(inner)
            .map_rect(rect(4.0, 30.0, 120.0, 22.0))
            .center();
        input.pointer_moved(dropdown);
        input.primary_button(true, dropdown);
        input.primary_button(false, dropdown);
        frame(&mut input);
        input.pointer_moved(dropdown + Vec2::new(0.0, 40.0));
        for _ in 0..3 {
            frame(&mut input);
        }
        let (count, open) = koebi_testkit::allocations(|| frame(&mut input));
        assert!(open, "the menu is open");
        assert_eq!(count, 0);
    }
}
