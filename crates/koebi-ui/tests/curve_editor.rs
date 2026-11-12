//! The curve editor: keys added by a double click and the button, dragged and brought back by
//! the operation's cancel, removed by the secondary button and the button, the ends kept; and its golden image.

mod common;

use common::render;
use koebi_core::{Extent, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_design::value::{CubicBezier, Curve, CurveKey};
use koebi_render::Scene;
use koebi_testkit::{Tolerance, assert_golden, gpu_or_skip};
use koebi_text::Fonts;
use koebi_tokens::KOEBI;
use koebi_ui::{
    CurveEditor, CurveEditorState, Input, OPERATION_CANCEL, PointerButton, Styles, Ui, UiState,
    WidgetId,
};

/// Anti-aliased edges may differ by a few units between drivers.
const CROSS_DRIVER: Tolerance = Tolerance::new(3, 8);

const EDITOR: WidgetId = WidgetId::new("curve");

/// The editor's top left corner and width.
const ORIGIN: Vec2 = Vec2::new(10.0, 10.0);
const WIDTH: f32 = 320.0;

/// Frames of one editor, its curve and state kept between them.
struct Harness {
    styles: Styles,
    state: UiState,
    input: Input,
    curve: Curve,
    editor: CurveEditorState,
}

impl Harness {
    fn new(curve: Curve) -> Self {
        let mut state = UiState::new();
        state.set_fonts(Fonts::with_embedded());
        Self {
            styles: Styles::resolve(&DesignSystem::new(&KOEBI).expect("Koebi's schema is valid")),
            state,
            input: Input::new(),
            curve,
            editor: CurveEditorState::new(),
        }
    }

    /// One frame of the editor. Returns whether the curve changed.
    fn frame(&mut self) -> (bool, Scene) {
        let mut scene = Scene::new();
        let changed = {
            let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
            let height = CurveEditor::height(&ui);
            let rect = Rect::from_origin_size(ORIGIN, Vec2::new(WIDTH, height));
            CurveEditor::new(EDITOR, &mut self.curve, &mut self.editor).show(&mut ui, rect)
        };
        self.input.end_frame();
        (changed, scene)
    }

    /// Where `(x, y)` of a curve within 0..1 lies on the graph: the graph shows values from
    /// -0.08 to 1.08 then (its margin of 8%).
    fn point(&self, x: f32, y: f32) -> Vec2 {
        let style = self.styles.curve_editor;
        let plot = Rect::from_origin_size(ORIGIN, Vec2::new(WIDTH, style.graph_height))
            .inset(style.padding);
        let (low, high) = (-0.08, 1.08);
        Vec2::new(
            plot.min.x + x * plot.width(),
            plot.max.y - (y - low) / (high - low) * plot.height(),
        )
    }

    /// The middle of the button at `index` of the row below the graph: "Add key", then
    /// "Remove key".
    /// The middle of the button at `index`: two squares the row's height wide, add then
    /// remove.
    fn button(&self, index: usize) -> Vec2 {
        let style = self.styles.curve_editor;
        let row = self.styles.number_field.height;
        let y = ORIGIN.y + style.graph_height + self.styles.property_group.row_gap + row * 0.5;
        Vec2::new(ORIGIN.x + row * (0.5 + index as f32), y)
    }

    fn press(&mut self, at: Vec2) -> bool {
        self.input.pointer_moved(at);
        self.frame();
        self.input.primary_button(true, at);
        let (changed, _) = self.frame();
        changed
    }

    fn release(&mut self, at: Vec2) -> bool {
        self.input.primary_button(false, at);
        let (changed, _) = self.frame();
        changed
    }

    /// A press and its release: what changed on either (a button acts on the release).
    fn click(&mut self, at: Vec2) -> bool {
        let pressed = self.press(at);
        let released = self.release(at);
        pressed || released
    }

    fn key(&self, index: usize) -> (f32, f32) {
        let key = self.curve.keys()[index];
        (key.x, key.y)
    }
}

fn close(a: (f32, f32), b: (f32, f32)) -> bool {
    (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01
}

#[test]
fn a_double_click_adds_a_key_that_drags_and_escape_brings_back() {
    let mut harness = Harness::new(Curve::LINEAR);
    harness.frame();
    let at = harness.point(0.3, 0.7);
    assert!(!harness.click(at));
    assert!(
        harness.click(at),
        "the second press of a double click adds a key"
    );
    assert_eq!(harness.curve.keys().len(), 3);
    assert!(close(harness.key(1), (0.3, 0.7)), "{:?}", harness.key(1));
    assert_eq!(harness.editor.selected(), Some(1));
    // Dragged, it follows the pointer.
    harness.press(at);
    let to = harness.point(0.6, 0.2);
    harness.input.pointer_moved(to);
    let (changed, _) = harness.frame();
    assert!(changed);
    assert!(close(harness.key(1), (0.6, 0.2)), "{:?}", harness.key(1));
    // Past the end in time: kept before it.
    harness.input.pointer_moved(harness.point(1.4, 0.2));
    harness.frame();
    assert!(harness.key(1).0 < 1.0);
    assert!(harness.curve.is_valid());
    // Escape brings it back where the drag started.
    harness.input.trigger(&OPERATION_CANCEL);
    let (changed, _) = harness.frame();
    assert!(changed);
    assert!(close(harness.key(1), (0.3, 0.7)), "{:?}", harness.key(1));
    harness.release(at);
}

#[test]
fn the_secondary_button_removes_inner_keys_and_the_ends_stay() {
    let mut harness = Harness::new(Curve::new(&[
        CurveKey::new(0.0, 0.0, CubicBezier::LINEAR),
        CurveKey::new(0.5, 0.5, CubicBezier::LINEAR),
        CurveKey::new(1.0, 1.0, CubicBezier::LINEAR),
    ]));
    harness.frame();
    // The ends neither go nor move.
    let start = harness.point(0.0, 0.0);
    harness.input.pointer_moved(start);
    harness.frame();
    harness.input.button(PointerButton::Secondary, true, start);
    harness.frame();
    harness.input.button(PointerButton::Secondary, false, start);
    harness.frame();
    assert_eq!(harness.curve.keys().len(), 3);
    harness.press(start);
    harness.input.pointer_moved(harness.point(0.2, 0.4));
    harness.frame();
    assert_eq!(harness.key(0), (0.0, 0.0));
    harness.release(start);
    // The inner key goes.
    let middle = harness.point(0.5, 0.5);
    harness.input.pointer_moved(middle);
    harness.frame();
    harness.input.button(PointerButton::Secondary, true, middle);
    let (changed, _) = harness.frame();
    harness
        .input
        .button(PointerButton::Secondary, false, middle);
    harness.frame();
    assert!(changed);
    assert_eq!(harness.curve, Curve::LINEAR);
}

#[test]
fn the_buttons_add_a_key_in_the_widest_segment_and_remove_the_selected_one() {
    let mut harness = Harness::new(Curve::LINEAR);
    harness.frame();
    assert!(harness.click(harness.button(0)));
    assert!(close(harness.key(1), (0.5, 0.5)), "{:?}", harness.key(1));
    assert_eq!(harness.editor.selected(), Some(1));
    assert!(harness.click(harness.button(1)));
    assert_eq!(harness.curve, Curve::LINEAR);
    // Nothing selected: nothing to remove.
    assert_eq!(harness.editor.selected(), None);
    assert!(!harness.click(harness.button(1)));
}

#[test]
fn the_curve_editor_matches_golden() {
    let Some(gpu) = gpu_or_skip("the_curve_editor_matches_golden") else {
        return;
    };
    // An overshoot that settles, its peak selected.
    let mut harness = Harness::new(Curve::new(&[
        CurveKey::new(0.0, 0.0, CubicBezier::new(0.3, 0.0, 0.7, 1.0)),
        CurveKey::new(0.45, 1.2, CubicBezier::new(0.4, 0.0, 0.6, 1.0)),
        CurveKey::new(0.75, 0.9, CubicBezier::LINEAR),
        CurveKey::new(1.0, 1.0, CubicBezier::LINEAR),
    ]));
    harness.editor.select(Some(1));
    let (_, mut scene) = harness.frame();
    scene.set_clear_color(harness.styles.background);
    // The editor, its graph 160 high and two rows of fields, with a margin of 10 around.
    let image = render(&gpu, Extent::new(340, 244), &scene);
    assert_golden!(&image, "curve_editor_dark", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}
