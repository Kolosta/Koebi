//! Number field tests: the harness, the behaviours ([`basics`]) and one test per bug found in
//! review ([`regressions`]).

mod basics;
mod regressions;

use koebi_core::{Rect, Vec2};
use koebi_render::Scene;
use koebi_text::Fonts;

use super::*;
use crate::{Input, Styles, UiState};

const ID: WidgetId = WidgetId::new("size");

fn field() -> Rect {
    Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::new(160.0, 22.0))
}

/// Number fields, their UI state and input, one frame at a time.
struct Harness {
    state: UiState,
    input: Input,
    styles: Styles,
}

impl Harness {
    fn new() -> Self {
        let mut state = UiState::new();
        state.set_fonts(Fonts::with_embedded());
        Self {
            state,
            input: Input::new(),
            styles: crate::test_styles(),
        }
    }

    /// Moves the clock to the next frame, 16 ms later, as the application does.
    fn tick(&mut self) {
        let next = self.input.time() + core::time::Duration::from_millis(16);
        self.input.set_time(next);
    }

    /// One frame running `show`.
    fn frame<R>(&mut self, show: impl FnOnce(&mut Ui<'_>) -> R) -> (R, Scene) {
        self.tick();
        let mut scene = Scene::new();
        let result = {
            let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
            show(&mut ui)
        };
        self.input.end_frame();
        (result, scene)
    }

    /// One frame drawn twice, as the application does after a change of the design system:
    /// with the frame's input, then with the same input settled.
    fn frame_twice<R>(&mut self, mut show: impl FnMut(&mut Ui<'_>) -> R) -> R {
        self.tick();
        let mut scene = Scene::new();
        let first = {
            let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
            show(&mut ui)
        };
        let settled = self.input.settled();
        let mut scene = Scene::new();
        {
            let mut ui = Ui::new(&mut scene, &self.styles, &settled, &mut self.state);
            show(&mut ui);
        }
        self.input.end_frame();
        first
    }

    /// One frame of a plain field of `value` in [`field`].
    fn show(&mut self, value: &mut f64) -> NumberFieldResponse {
        self.frame(|ui| NumberField::new(ID, value).show(ui, field()))
            .0
    }

    /// The travel of `steps` steps of an unbounded field.
    fn steps(&self, steps: f32) -> Vec2 {
        Vec2::new(self.styles.number_field.step_distance * steps, 0.0)
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}
