//! Forced states: a button, a checkbox and a number field shown in one state whatever the
//! pointer does, as documentation pictures them, look as they do in that state for real and
//! react to nothing.

use koebi_core::{Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_render::Scene;
use koebi_text::Fonts;
use koebi_tokens::KOEBI;
use koebi_ui::{Button, Checkbox, ControlState, Input, NumberField, Styles, Ui, UiState, WidgetId};

const ID: WidgetId = WidgetId::new("forced");

fn rect() -> Rect {
    Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::new(120.0, 24.0))
}

/// Frames of one component, with their own state and input.
struct Harness {
    styles: Styles,
    state: UiState,
    input: Input,
}

impl Harness {
    fn new() -> Self {
        let mut state = UiState::new();
        state.set_fonts(Fonts::with_embedded());
        Self {
            styles: Styles::resolve(&DesignSystem::new(&KOEBI).expect("Koebi's schema is valid")),
            state,
            input: Input::new(),
        }
    }

    fn frame<R>(&mut self, draw: impl FnOnce(&mut Ui<'_>) -> R) -> (R, String) {
        let mut scene = Scene::new();
        let result = {
            let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
            draw(&mut ui)
        };
        self.input.end_frame();
        (result, format!("{scene:?}"))
    }
}

/// What `draw` shows with the pointer resting over it, the second frame (hit testing reads the
/// frame before).
fn hovered<R>(draw: impl Fn(&mut Ui<'_>) -> R) -> String {
    let mut harness = Harness::new();
    harness.input.pointer_moved(rect().center());
    harness.frame(&draw);
    harness.frame(&draw).1
}

/// What `draw` shows with the pointer away.
fn away<R>(draw: impl Fn(&mut Ui<'_>) -> R) -> String {
    let mut harness = Harness::new();
    harness.frame(&draw);
    harness.frame(&draw).1
}

#[test]
fn forced_states_look_as_the_real_ones() {
    let button = |forced: Option<ControlState>| {
        move |ui: &mut Ui<'_>| {
            let button = Button::new(ID).label("Save");
            match forced {
                Some(state) => button.forced(state),
                None => button,
            }
            .show(ui, rect())
        }
    };
    assert_eq!(
        away(button(Some(ControlState::Hovered))),
        hovered(button(None))
    );
    assert_eq!(away(button(Some(ControlState::Normal))), away(button(None)));
    let disabled = |ui: &mut Ui<'_>| {
        Button::new(ID)
            .label("Save")
            .enabled(false)
            .show(ui, rect())
    };
    assert_eq!(away(button(Some(ControlState::Disabled))), away(disabled));

    let checkbox = |forced: Option<ControlState>| {
        move |ui: &mut Ui<'_>| {
            let mut checked = true;
            let checkbox = Checkbox::new(ID, &mut checked).label("Snap");
            match forced {
                Some(state) => checkbox.forced(state),
                None => checkbox,
            }
            .show(ui, rect())
        }
    };
    assert_eq!(
        away(checkbox(Some(ControlState::Hovered))),
        hovered(checkbox(None))
    );

    let field = |forced: Option<ControlState>| {
        move |ui: &mut Ui<'_>| {
            let mut value = 0.5_f64;
            let field = NumberField::new(ID, &mut value)
                .range(0.0, 1.0)
                .label("Opacity");
            match forced {
                Some(state) => field.forced(state),
                None => field,
            }
            .show(ui, rect())
        }
    };
    assert_eq!(
        away(field(Some(ControlState::Hovered))),
        hovered(field(None))
    );
}

#[test]
fn forced_components_react_to_nothing() {
    let mut harness = Harness::new();
    let at = rect().center();
    let mut value = 0.5_f64;
    let mut checked = false;
    let frame = |harness: &mut Harness, value: &mut f64, checked: &mut bool| {
        harness
            .frame(|ui| {
                let button = Button::new(ID.with(1))
                    .label("Save")
                    .forced(ControlState::Pressed)
                    .show(ui, rect());
                let checkbox = Checkbox::new(ID.with(2), checked)
                    .forced(ControlState::Hovered)
                    .show(ui, rect());
                let field = NumberField::new(ID.with(3), value)
                    .range(0.0, 1.0)
                    .forced(ControlState::Active)
                    .show(ui, rect());
                (
                    button.clicked || button.pressed,
                    checkbox.changed,
                    field.changed,
                )
            })
            .0
    };
    harness.input.pointer_moved(at);
    frame(&mut harness, &mut value, &mut checked);
    harness.input.primary_button(true, at);
    let pressed = frame(&mut harness, &mut value, &mut checked);
    harness.input.pointer_moved(at + Vec2::new(30.0, 0.0));
    frame(&mut harness, &mut value, &mut checked);
    harness.input.primary_button(false, at);
    let released = frame(&mut harness, &mut value, &mut checked);
    assert_eq!(pressed, (false, false, false));
    assert_eq!(released, (false, false, false));
    assert!(!checked);
    assert!((value - 0.5).abs() < f64::EPSILON);
}
