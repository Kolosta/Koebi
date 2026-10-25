//! Visual regression tests: a sheet of every component, in two themes, compared with golden
//! PNGs in `tests/golden/`.
//!
//! After an intended visual change, run `cargo xtask bless`, review the updated PNGs in the
//! pull request and commit them with the change.

mod common;

use common::render;
use koebi_core::{Color, Extent, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_render::Scene;
use koebi_testkit::{Tolerance, assert_golden, gpu_or_skip};
use koebi_tokens::KOEBI;
use koebi_ui::{Icon, Input, PanelLevel, Styles, ThemePreview, Ui, UiState, WidgetId};

/// Anti-aliased edges may differ by a few units between drivers.
const CROSS_DRIVER: Tolerance = Tolerance::new(3, 8);

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Vec2::new(x, y), Vec2::new(width, height))
}

/// Every component in its main states, styled by the active theme of `design`, with the
/// pointer over the second button.
fn component_sheet(design: &DesignSystem) -> Scene {
    let styles = Styles::resolve(design);
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    let mut input = Input::new();
    input.pointer_moved(Vec2::new(100.0, 24.0));
    let mut state = UiState::new();
    let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
    let id = WidgetId::new;

    // Buttons, toggles and checkboxes.
    ui.button(id("plain"), rect(12.0, 12.0, 64.0, 24.0), None);
    ui.button(
        id("hovered"),
        rect(84.0, 12.0, 32.0, 24.0),
        Some(Icon::Plus),
    );
    ui.toggle(
        id("off"),
        rect(124.0, 12.0, 24.0, 24.0),
        &mut false,
        Some(Icon::ChevronDown),
    );
    ui.toggle(
        id("on"),
        rect(152.0, 12.0, 24.0, 24.0),
        &mut true,
        Some(Icon::Check),
    );
    ui.checkbox(id("unchecked"), rect(188.0, 16.0, 24.0, 16.0), &mut false);
    ui.checkbox(id("checked"), rect(212.0, 16.0, 24.0, 16.0), &mut true);
    for (index, icon) in [Icon::Minus, Icon::Close, Icon::ChevronUp]
        .into_iter()
        .enumerate()
    {
        let x = 240.0 + 24.0 * index as f32;
        ui.button(
            id("icons").with(index as u64),
            rect(x, 12.0, 20.0, 24.0),
            Some(icon),
        );
    }

    // Value components.
    ui.slider(
        id("slider"),
        rect(12.0, 50.0, 136.0, 20.0),
        &mut 0.3,
        0.0..=1.0,
    );
    ui.drag_value(
        id("drag"),
        rect(160.0, 48.0, 148.0, 24.0),
        &mut 0.6,
        0.0..=1.0,
        0.0,
    );

    // Panels, a separator and swatches.
    let content = ui.panel(rect(12.0, 84.0, 136.0, 80.0), PanelLevel::First);
    ui.panel(
        Rect::from_min_max(content.min, Vec2::new(content.max.x, content.min.y + 28.0)),
        PanelLevel::Second,
    );
    ui.separator(rect(
        content.min.x,
        content.min.y + 32.0,
        content.width(),
        8.0,
    ));
    ui.panel(
        Rect::from_min_max(Vec2::new(content.min.x, content.min.y + 44.0), content.max),
        PanelLevel::Third,
    );
    let swatches = [
        Color::from_hex(0xE0_4F_5F),
        Color::from_hex(0x2D_9C_DB).with_alpha(0.5),
        Color::from_hex(0x27_AE_60),
    ];
    for (index, color) in swatches.into_iter().enumerate() {
        let x = 160.0 + 52.0 * index as f32;
        ui.swatch(
            id("swatch").with(index as u64),
            rect(x, 84.0, 44.0, 32.0),
            color,
            index == 2,
        );
    }

    // Theme cards: the active theme, selected, and the light one.
    let previews = ["dark", "light"].map(|key| {
        let theme = design.find_theme(key).expect("built-in theme");
        ThemePreview::of(design, theme).expect("live theme")
    });
    for (index, preview) in previews.iter().enumerate() {
        let x = 160.0 + 76.0 * index as f32;
        ui.theme_card(
            id("card").with(index as u64),
            rect(x, 124.0, 72.0, 48.0),
            preview,
            index == 0,
        );
    }
    drop(ui);
    scene
}

#[test]
fn components_match_golden_in_the_dark_theme() {
    let Some(gpu) = gpu_or_skip("components_match_golden_in_the_dark_theme") else {
        return;
    };
    let design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
    let image = render(&gpu, Extent::new(320, 184), &component_sheet(&design));
    assert_golden!(image, "components_dark", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}

#[test]
fn components_match_golden_in_the_light_theme() {
    let Some(gpu) = gpu_or_skip("components_match_golden_in_the_light_theme") else {
        return;
    };
    let mut design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
    let light = design.find_theme("light").expect("built-in theme");
    design.set_active_theme(light).expect("built-in theme");
    let image = render(&gpu, Extent::new(320, 184), &component_sheet(&design));
    assert_golden!(image, "components_light", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}
