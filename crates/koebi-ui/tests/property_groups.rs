//! A property group on screen: labels right-aligned on their share of the width, a text field,
//! vectors whose label goes with their first component, a checkbox and a disabled row,
//! matching its golden image.

mod common;

use common::render;
use koebi_core::expr::{DEGREE, MILLIMETER};
use koebi_core::{Extent, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_render::Scene;
use koebi_testkit::{Tolerance, assert_golden, gpu_or_skip};
use koebi_text::Fonts;
use koebi_tokens::KOEBI;
use koebi_ui::{
    Checkbox, Input, NumberField, PanelLevel, PropertyGroup, Styles, TextField, Ui, UiState,
    WidgetId,
};

/// Anti-aliased edges may differ by a few units between drivers.
const CROSS_DRIVER: Tolerance = Tolerance::new(3, 8);

fn property_group_sheet(design: &DesignSystem) -> Scene {
    let styles = Styles::resolve(design);
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let input = Input::new();
    let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
    let id = WidgetId::new;
    let panel = Rect::from_origin_size(Vec2::new(8.0, 8.0), Vec2::new(304.0, 284.0));
    let content = ui.panel(panel, PanelLevel::First);
    let mut group = PropertyGroup::new(&ui, content);
    let mut name = String::from("Cube");
    let row = group.row(&mut ui, "Name");
    TextField::new(id("name"), &mut name).show(&mut ui, row);
    let mut location = [1.0, -2.5, 0.0];
    let rects = group.vector(&mut ui, "Location", ["X", "Y", "Z"]);
    for ((value, rect), key) in location.iter_mut().zip(rects).zip(0u64..) {
        NumberField::new(id("location").with(key), value)
            .unit(&MILLIMETER)
            .show(&mut ui, rect);
    }
    let mut rotation = [0.0, 45.0, 90.0];
    let rects = group.vector(&mut ui, "Rotation", ["X", "Y", "Z"]);
    for ((value, rect), key) in rotation.iter_mut().zip(rects).zip(0u64..) {
        NumberField::new(id("rotation").with(key), value)
            .unit(&DEGREE)
            .decimals(0)
            .show(&mut ui, rect);
    }
    let mut visible = true;
    Checkbox::new(id("visible"), &mut visible)
        .label("Visible in renders")
        .show(&mut ui, group.unlabeled());
    group.group();
    group.set_enabled(false);
    let mut locked = 2.0;
    let row = group.row(&mut ui, "Locked");
    NumberField::new(id("locked"), &mut locked)
        .enabled(false)
        .show(&mut ui, row);
    drop(ui);
    scene
}

#[test]
fn property_groups_match_golden() {
    let Some(gpu) = gpu_or_skip("property_groups_match_golden") else {
        return;
    };
    let design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
    let image = render(&gpu, Extent::new(320, 300), &property_group_sheet(&design));
    assert_golden!(image, "property_groups_dark", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}
