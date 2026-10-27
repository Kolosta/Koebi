//! Number fields on screen: plain, with a label and a unit, with linear and logarithmic bars,
//! of integers, hovered with their step buttons, dragged, vertical, and disabled, matching
//! their golden image.

mod common;

use common::render;
use koebi_core::expr::MILLIMETER;
use koebi_core::{Extent, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_render::Scene;
use koebi_testkit::{Tolerance, assert_golden, gpu_or_skip};
use koebi_text::Fonts;
use koebi_tokens::KOEBI;
use koebi_ui::{BarMapping, Input, NumberField, Styles, Ui, UiState, WidgetId};

/// Anti-aliased edges may differ by a few units between drivers.
const CROSS_DRIVER: Tolerance = Tolerance::new(3, 8);

fn row(index: u16) -> Rect {
    Rect::from_origin_size(
        Vec2::new(8.0, 8.0 + f32::from(index) * 28.0),
        Vec2::new(184.0, 22.0),
    )
}

fn tall() -> Rect {
    Rect::from_origin_size(Vec2::new(200.0, 8.0), Vec2::new(22.0, 190.0))
}

/// The values of the sheet.
struct Values {
    plain: f64,
    width: f64,
    opacity: f64,
    frequency: f64,
    samples: i32,
    hovered: f64,
    dragged: f64,
    tall: f64,
    locked: f64,
}

/// One frame of the sheet.
fn frame(state: &mut UiState, styles: &Styles, input: &Input, values: &mut Values) -> Scene {
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    let mut ui = Ui::new(&mut scene, styles, input, state);
    let id = WidgetId::new;
    NumberField::new(id("plain"), &mut values.plain).show(&mut ui, row(0));
    NumberField::new(id("width"), &mut values.width)
        .label("Width")
        .unit(&MILLIMETER)
        .show(&mut ui, row(1));
    NumberField::new(id("opacity"), &mut values.opacity)
        .label("Opacity")
        .range(0.0, 1.0)
        .bar(BarMapping::Linear)
        .show(&mut ui, row(2));
    NumberField::new(id("frequency"), &mut values.frequency)
        .label("Frequency")
        .range(20.0, 20_000.0)
        .decimals(0)
        .bar(BarMapping::Logarithmic)
        .show(&mut ui, row(3));
    NumberField::new(id("samples"), &mut values.samples)
        .label("Samples")
        .range(1, 256)
        .show(&mut ui, row(4));
    NumberField::new(id("hovered"), &mut values.hovered)
        .range(0.0, 10.0)
        .bar(BarMapping::Linear)
        .show(&mut ui, row(5));
    NumberField::new(id("dragged"), &mut values.dragged)
        .label("Dragged")
        .show(&mut ui, row(6));
    NumberField::new(id("tall"), &mut values.tall)
        .range(0.0, 1.0)
        .bar(BarMapping::Linear)
        .vertical()
        .show(&mut ui, tall());
    NumberField::new(id("locked"), &mut values.locked)
        .label("Locked")
        .enabled(false)
        .show(&mut ui, row(7));
    drop(ui);
    scene
}

/// The sheet with the pointer over the sixth field (its step buttons show) after dragging
/// the seventh.
fn number_field_sheet(design: &DesignSystem) -> Scene {
    let styles = Styles::resolve(design);
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let mut input = Input::new();
    let mut values = Values {
        plain: 1.5,
        width: 12.5,
        opacity: 0.3,
        frequency: 440.0,
        samples: 64,
        hovered: 7.5,
        dragged: 2.0,
        tall: 0.6,
        locked: 1.0,
    };
    frame(&mut state, &styles, &input, &mut values);
    // A drag of the seventh field: pressed, moved by three steps, released.
    let press = row(6).center();
    input.primary_button(true, press);
    frame(&mut state, &styles, &input, &mut values);
    input.end_frame();
    input.pointer_moved_by(Vec2::new(3.0 * styles.number_field.step_distance, 0.0));
    frame(&mut state, &styles, &input, &mut values);
    input.end_frame();
    input.primary_button(false, press);
    frame(&mut state, &styles, &input, &mut values);
    input.end_frame();
    // The pointer comes back, then rests over the sixth field.
    input.pointer_moved(row(5).center());
    frame(&mut state, &styles, &input, &mut values);
    input.end_frame();
    assert!(
        (values.dragged - 2.3).abs() < 1e-9,
        "dragged to {}",
        values.dragged
    );
    frame(&mut state, &styles, &input, &mut values)
}

#[test]
fn number_fields_match_golden() {
    let Some(gpu) = gpu_or_skip("number_fields_match_golden") else {
        return;
    };
    let design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
    let image = render(&gpu, Extent::new(232, 236), &number_field_sheet(&design));
    assert_golden!(image, "number_fields_dark", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}
