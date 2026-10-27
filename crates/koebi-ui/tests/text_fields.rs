//! Text fields on screen: idle, with a placeholder and an icon, centered, editing with a
//! selection, cut with an ellipsis, and disabled, matching their golden image.

mod common;

use common::render;
use koebi_core::{Extent, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_render::Scene;
use koebi_testkit::{Tolerance, assert_golden, gpu_or_skip};
use koebi_text::Fonts;
use koebi_tokens::KOEBI;
use koebi_ui::{
    Align, Icon, Input, Styles, TEXT_END, TEXT_SELECT_LEFT, TextField, Ui, UiState, WidgetId,
};

/// Anti-aliased edges may differ by a few units between drivers.
const CROSS_DRIVER: Tolerance = Tolerance::new(3, 8);

fn row(index: u16) -> Rect {
    Rect::from_origin_size(
        Vec2::new(8.0, 8.0 + f32::from(index) * 30.0),
        Vec2::new(184.0, 22.0),
    )
}

/// The texts of the sheet.
struct Texts {
    name: String,
    search: String,
    centered: String,
    selected: String,
    long: String,
    disabled: String,
}

/// One frame of the sheet.
fn frame(state: &mut UiState, styles: &Styles, input: &Input, texts: &mut Texts) -> Scene {
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    let mut ui = Ui::new(&mut scene, styles, input, state);
    let id = WidgetId::new;
    TextField::new(id("name"), &mut texts.name).show(&mut ui, row(0));
    TextField::new(id("search"), &mut texts.search)
        .placeholder("Search")
        .icon(Icon::Search)
        .show(&mut ui, row(1));
    TextField::new(id("centered"), &mut texts.centered)
        .align(Align::Center)
        .show(&mut ui, row(2));
    TextField::new(id("selected"), &mut texts.selected).show(&mut ui, row(3));
    TextField::new(id("long"), &mut texts.long).show(&mut ui, row(4));
    TextField::new(id("disabled"), &mut texts.disabled)
        .enabled(false)
        .show(&mut ui, row(5));
    drop(ui);
    scene
}

/// The sheet, its fourth field editing with its last word selected (the caret at the start of
/// the selection); the long field shows its text cut with an ellipsis.
fn text_field_sheet(design: &DesignSystem) -> Scene {
    let styles = Styles::resolve(design);
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let mut input = Input::new();
    let mut texts = Texts {
        name: "Cube.001".to_owned(),
        search: String::new(),
        centered: "Centered".to_owned(),
        selected: "Snap to grid".to_owned(),
        long: "A name far too long for its field, cut with an ellipsis".to_owned(),
        disabled: "Disabled".to_owned(),
    };
    frame(&mut state, &styles, &input, &mut texts);
    // Edit the selected field, its last word selected.
    let at = row(3).center();
    input.primary_button(true, at);
    input.primary_button(false, at);
    frame(&mut state, &styles, &input, &mut texts);
    input.end_frame();
    input.trigger(&TEXT_END);
    for _ in 0..4 {
        input.trigger(&TEXT_SELECT_LEFT);
    }
    frame(&mut state, &styles, &input, &mut texts);
    input.end_frame();
    frame(&mut state, &styles, &input, &mut texts)
}

#[test]
fn text_fields_match_golden() {
    let Some(gpu) = gpu_or_skip("text_fields_match_golden") else {
        return;
    };
    let design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
    let image = render(&gpu, Extent::new(200, 190), &text_field_sheet(&design));
    assert_golden!(image, "text_fields_dark", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}
