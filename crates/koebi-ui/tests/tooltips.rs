//! Tooltips standing out: below a button on a panel and below one on a menu's background, their
//! inverted background marking them on both; and one that would leave the window, kept inside.

mod common;

use common::render;
use koebi_core::{Extent, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_render::{RectPrimitive, Scene};
use koebi_testkit::{Tolerance, assert_golden, gpu_or_skip};
use koebi_text::Fonts;
use koebi_tokens::KOEBI;
use koebi_ui::{Button, Input, PanelLevel, Styles, Ui, UiState, WidgetId};

/// Anti-aliased edges may differ by a few units between drivers.
const CROSS_DRIVER: Tolerance = Tolerance::new(3, 8);

/// Draws a panel and a menu's background, each with a button and its tooltip, the pointer on
/// `hovered`.
fn frame(state: &mut UiState, input: &mut Input, styles: &Styles) -> Scene {
    let mut scene = Scene::new();
    {
        let mut ui = Ui::new(&mut scene, styles, input, state);
        let panel = Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::new(240.0, 120.0));
        ui.panel(panel, PanelLevel::First);
        let first = Rect::from_origin_size(Vec2::new(30.0, 30.0), Vec2::new(80.0, 24.0));
        Button::new(WidgetId::new("on-panel"))
            .label("Panel")
            .show(&mut ui, first);
        ui.tooltip(WidgetId::new("on-panel"), "Stands out on panels", first);
        let menu = Rect::from_origin_size(Vec2::new(260.0, 10.0), Vec2::new(200.0, 120.0));
        let style = ui.styles().menu;
        ui.scene().push_rect(
            &RectPrimitive::new(menu, style.background)
                .with_corner_radius(style.corner_radius)
                .with_border(style.border_width, style.border_color),
        );
        let second = Rect::from_origin_size(Vec2::new(380.0, 30.0), Vec2::new(70.0, 24.0));
        Button::new(WidgetId::new("on-menu"))
            .label("Menu")
            .show(&mut ui, second);
        ui.tooltip(
            WidgetId::new("on-menu"),
            "And on menus, kept in the window",
            second,
        );
    }
    input.end_frame();
    scene
}

#[test]
fn tooltips_stand_out_and_stay_inside_the_window_golden() {
    let Some(gpu) = gpu_or_skip("tooltips_stand_out_and_stay_inside_the_window_golden") else {
        return;
    };
    let styles = Styles::resolve(&DesignSystem::new(&KOEBI).expect("Koebi's schema is valid"));
    let window = Rect::from_origin_size(Vec2::ZERO, Vec2::new(470.0, 140.0));
    // Two windows' worth of state: one tooltip at a time, each rested on its own.
    let mut scenes = Vec::new();
    for at in [Vec2::new(40.0, 40.0), Vec2::new(420.0, 40.0)] {
        let mut state = UiState::new();
        state.set_fonts(Fonts::with_embedded());
        state.set_window(window);
        let mut input = Input::new();
        frame(&mut state, &mut input, &styles);
        input.pointer_moved(at);
        frame(&mut state, &mut input, &styles);
        frame(&mut state, &mut input, &styles);
        input.set_time(styles.input.tooltip_delay);
        scenes.push(frame(&mut state, &mut input, &styles));
    }
    for (index, mut scene) in scenes.into_iter().enumerate() {
        scene.set_clear_color(styles.background);
        let image = render(&gpu, Extent::new(470, 140), &scene);
        assert_golden!(&image, &format!("tooltips_dark_{index}"), CROSS_DRIVER);
    }
    gpu.assert_no_validation_errors();
}
