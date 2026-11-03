//! Disabled controls: the disabled background in every theme, a toggle switched on then
//! disabled still showing it is on, a checked box still showing it is checked, and none of them
//! reacting to the pointer.

mod common;

use common::{assert_pixel, over, render};
use koebi_core::{Extent, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_render::Scene;
use koebi_testkit::gpu_or_skip;
use koebi_tokens::{KOEBI, semantic};
use koebi_ui::{
    Button, Checkbox, ControlState, Dropdown, Input, MenuItem, MenuState, NumberField, Styles,
    TextField, Ui, UiState, WidgetId,
};

const SIZE: Extent = Extent::new(200, 100);

fn design() -> DesignSystem {
    DesignSystem::new(&KOEBI).expect("Koebi's schema is valid")
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Vec2::new(x, y), Vec2::new(width, height))
}

/// A pixel coordinate inside the image.
fn px(value: f32) -> u32 {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "positive pixel positions"
    )]
    let pixel = value as u32;
    pixel
}

/// Draws one frame of components with the styles of the active theme.
fn draw(design: &DesignSystem, build: impl FnOnce(&mut Ui<'_>)) -> Scene {
    let styles = Styles::resolve(design);
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    let (mut state, input) = (UiState::new(), Input::new());
    build(&mut Ui::new(&mut scene, &styles, &input, &mut state));
    scene
}

/// Each control, enabled or not, in the row at `top`: a text field, a number field, a
/// dropdown, a button and a checkbox. Returns where each shows its background alone.
fn control_row(ui: &mut Ui<'_>, top: f32, enabled: bool) -> [(u32, u32); 5] {
    let id = |name: &str| WidgetId::new(name).with(u64::from(enabled));
    let (mut text, mut value, mut checked) = (String::new(), 0.5, false);
    let mut menu = MenuState::new();
    let items = [MenuItem::action("One")];
    let cells: [Rect; 5] = core::array::from_fn(|index| {
        let x = 4.0 + 40.0 * f32::from(u8::try_from(index).expect("five cells"));
        rect(x, top, 36.0, 20.0)
    });
    TextField::new(id("text"), &mut text)
        .enabled(enabled)
        .show(ui, cells[0]);
    NumberField::new(id("number"), &mut value)
        .range(0.0, 1.0)
        .enabled(enabled)
        .show(ui, cells[1]);
    Dropdown::new(id("dropdown"), &mut menu, &items)
        .enabled(enabled)
        .show(ui, cells[2], rect(0.0, 0.0, 200.0, 100.0));
    Button::new(id("button"))
        .enabled(enabled)
        .show(ui, cells[3]);
    Checkbox::new(id("checkbox"), &mut checked)
        .enabled(enabled)
        .show(ui, cells[4]);
    let pixel = |x: f32, y: f32| (px(x), px(y));
    let mut places = cells.map(|cell| pixel(cell.min.x + 4.0, cell.center().y));
    // The checkbox's box, at its start.
    let checkbox = ui.styles().checkbox.box_rect(cells[4], ui.scale_factor());
    places[4] = pixel(checkbox.min.x + 3.0, checkbox.min.y + 3.0);
    places
}

#[test]
fn disabled_controls_show_the_disabled_background_in_every_theme() {
    let Some(gpu) = gpu_or_skip("disabled_controls_show_the_disabled_background_in_every_theme")
    else {
        return;
    };
    let mut design = design();
    let themes: Vec<_> = design.themes().map(|(id, _)| id).collect();
    for theme in themes {
        design.set_active_theme(theme).expect("a built-in theme");
        let mut places = ([(0, 0); 5], [(0, 0); 5]);
        let scene = draw(&design, |ui| {
            places = (control_row(ui, 10.0, true), control_row(ui, 50.0, false));
        });
        let image = render(&gpu, SIZE, &scene);
        let disabled = design.get(semantic::BACKGROUND_APP_DISABLED).to_rgba8();
        let controls = [
            "text field",
            "number field",
            "dropdown",
            "button",
            "checkbox",
        ];
        for ((name, enabled), (x, y)) in controls.iter().zip(places.0).zip(places.1) {
            assert_pixel(&image, x, y, disabled);
            let (ex, ey) = enabled;
            assert_ne!(
                image.pixel(ex, ey),
                disabled,
                "{name}: enabled and disabled look alike in {:?}",
                design.active_theme()
            );
        }
    }
    gpu.assert_no_validation_errors();
}

/// Frames of a disabled toggle switched `on` or off in `cell`, the pointer clicking it: hovered,
/// pressed and released at its middle. Returns the last frame, the pointer still over it.
fn clicked_toggle(styles: &Styles, cell: Rect, on: &mut bool) -> Scene {
    let (mut state, mut input) = (UiState::new(), Input::new());
    let center = cell.center();
    let mut scene = Scene::new();
    // Laid out, hovered, pressed, released, then at rest under the pointer.
    for press in [None, None, Some(true), Some(false), None] {
        input.pointer_moved(center);
        if let Some(down) = press {
            input.primary_button(down, center);
        }
        scene.clear();
        scene.set_clear_color(styles.background);
        let mut ui = Ui::new(&mut scene, styles, &input, &mut state);
        Button::new(WidgetId::new("toggle"))
            .label("On")
            .toggle(on)
            .enabled(false)
            .show(&mut ui, cell);
        drop(ui);
        input.end_frame();
    }
    scene
}

#[test]
fn a_toggle_switched_on_then_disabled_shows_it_is_on_and_ignores_the_pointer() {
    let Some(gpu) =
        gpu_or_skip("a_toggle_switched_on_then_disabled_shows_it_is_on_and_ignores_the_pointer")
    else {
        return;
    };
    let styles = Styles::resolve(&design());
    let colors = &styles.button.toggle;
    let shown = |state: ControlState| over(colors.background.get(state), styles.background);
    let cell = rect(20.0, 20.0, 60.0, 20.0);
    for (on, expected) in [
        (true, ControlState::SelectedDisabled),
        (false, ControlState::Disabled),
    ] {
        let mut value = on;
        let scene = clicked_toggle(&styles, cell, &mut value);
        // Clicked, it did not flip; under the pointer, it does not look hovered.
        assert_eq!(value, on);
        let image = render(&gpu, SIZE, &scene);
        let (x, y) = (px(cell.min.x + 5.0), px(cell.center().y));
        assert_pixel(&image, x, y, shown(expected).to_rgba8());
    }
    // Switched on shows, dimmed: neither the plain disabled color nor the selected one.
    let on_disabled = shown(ControlState::SelectedDisabled).to_rgba8();
    assert_ne!(on_disabled, shown(ControlState::Disabled).to_rgba8());
    assert_ne!(on_disabled, shown(ControlState::Selected).to_rgba8());
    gpu.assert_no_validation_errors();
}

#[test]
fn a_checked_box_then_disabled_shows_it_is_checked() {
    let Some(gpu) = gpu_or_skip("a_checked_box_then_disabled_shows_it_is_checked") else {
        return;
    };
    let design = design();
    let styles = Styles::resolve(&design);
    let cells = [rect(20.0, 20.0, 80.0, 20.0), rect(20.0, 50.0, 80.0, 20.0)];
    let scene = draw(&design, |ui| {
        for ((cell, checked), key) in cells.into_iter().zip([true, false]).zip(0_u64..) {
            let mut value = checked;
            Checkbox::new(WidgetId::new("box").with(key), &mut value)
                .enabled(false)
                .show(ui, cell);
        }
    });
    let image = render(&gpu, SIZE, &scene);
    let style = &styles.checkbox;
    // A corner of each box, clear of the mark.
    let corner = |cell: Rect| {
        let bounds = style.box_rect(cell, 1.0);
        (px(bounds.min.x + 3.0), px(bounds.min.y + 3.0))
    };
    let checked = over(style.background_checked.disabled, styles.background).to_rgba8();
    let unchecked = over(style.background.disabled, styles.background).to_rgba8();
    assert_ne!(checked, unchecked);
    let (x, y) = corner(cells[0]);
    assert_pixel(&image, x, y, checked);
    let (x, y) = corner(cells[1]);
    assert_pixel(&image, x, y, unchecked);
    // The checked box keeps a tint of its checked color, dimmed.
    assert_ne!(
        style.background_checked.disabled,
        style.background_checked.normal
    );
    gpu.assert_no_validation_errors();
}
