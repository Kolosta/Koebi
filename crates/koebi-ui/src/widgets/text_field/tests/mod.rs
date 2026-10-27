//! Text field tests: the harness, the behaviours ([`basics`]) and one test per bug found in
//! review ([`regressions`]).

mod basics;
mod regressions;

use koebi_core::{Rect, Vec2};
use koebi_render::Scene;
use koebi_text::Fonts;

use super::geometry::{caret_x, offset_at, scroll_to};
use super::*;
use crate::{Input, Styles, UiState};

const NAME: WidgetId = WidgetId::new("name");
const OTHER: WidgetId = WidgetId::new("other");

fn field() -> Rect {
    Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::new(160.0, 22.0))
}

fn other() -> Rect {
    Rect::from_origin_size(Vec2::new(10.0, 40.0), Vec2::new(160.0, 22.0))
}

/// Text fields, their UI state and input, one frame at a time.
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

    /// One frame running `show`.
    fn frame<R>(&mut self, show: impl FnOnce(&mut Ui<'_>) -> R) -> (R, Scene) {
        let mut scene = Scene::new();
        let result = {
            let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
            show(&mut ui)
        };
        self.input.end_frame();
        (result, scene)
    }

    /// One frame showing the field `NAME` editing `text` in [`field`].
    fn show(&mut self, text: &mut String) -> TextFieldResponse {
        self.frame(|ui| TextField::new(NAME, text).show(ui, field()))
            .0
    }

    fn click(&mut self, at: Vec2) {
        self.input.primary_button(true, at);
        self.input.primary_button(false, at);
    }

    /// Lays the field out, then clicks it: the edit starts with the text selected.
    fn start(&mut self, text: &mut String) -> TextFieldResponse {
        self.show(text);
        self.click(field().center());
        self.show(text)
    }
}
