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
use koebi_text::Fonts;
use koebi_tokens::KOEBI;
use koebi_ui::{
    BarMapping, Button, Checkbox, Icon, Input, ListItem, ListOptions, ListState, NumberField,
    PanelLevel, Styles, ThemePreview, Ui, UiState, WidgetId,
};

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
    // Hit testing uses the previous frame: lay the sheet out once, then draw it.
    for _ in 0..2 {
        scene.clear();
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        draw_sheet(&mut ui, design);
    }
    scene
}

/// Every component of the sheet.
fn draw_sheet(ui: &mut Ui<'_>, design: &DesignSystem) {
    let id = WidgetId::new;

    // Buttons, toggles and checkboxes.
    Button::new(id("plain")).show(ui, rect(12.0, 12.0, 64.0, 24.0));
    Button::new(id("hovered"))
        .icon(Icon::Plus)
        .show(ui, rect(84.0, 12.0, 32.0, 24.0));
    Button::new(id("off"))
        .icon(Icon::ChevronDown)
        .toggle(&mut false)
        .show(ui, rect(124.0, 12.0, 24.0, 24.0));
    Button::new(id("on"))
        .icon(Icon::Check)
        .toggle(&mut true)
        .show(ui, rect(152.0, 12.0, 24.0, 24.0));
    Checkbox::new(id("unchecked"), &mut false).show(ui, rect(188.0, 16.0, 24.0, 16.0));
    Checkbox::new(id("checked"), &mut true).show(ui, rect(212.0, 16.0, 24.0, 16.0));
    for (index, icon) in [Icon::Minus, Icon::Close, Icon::ChevronUp]
        .into_iter()
        .enumerate()
    {
        let x = 240.0 + 24.0 * index as f32;
        Button::new(id("icons").with(index as u64))
            .icon(icon)
            .show(ui, rect(x, 12.0, 20.0, 24.0));
    }

    // Value components.
    ui.slider(
        id("slider"),
        rect(12.0, 50.0, 136.0, 20.0),
        &mut 0.3,
        0.0..=1.0,
    );
    NumberField::new(id("drag"), &mut 0.6)
        .range(0.0, 1.0)
        .bar(BarMapping::Linear)
        .show(ui, rect(160.0, 48.0, 148.0, 24.0));

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

/// The rows of the lists' golden images: a scene tree with icons, and colored layers.
fn list_items() -> [ListItem<'static>; 9] {
    [
        ListItem::new(1, "Scene"),
        ListItem::new(2, "Camera")
            .with_depth(1)
            .with_icon(Icon::Plus),
        ListItem::new(3, "Lights")
            .with_depth(1)
            .with_icon(Icon::Check),
        ListItem::new(4, "Sun").with_depth(2),
        ListItem::new(5, "Fill light with a long name").with_depth(2),
        ListItem::new(6, "Props")
            .with_depth(1)
            .with_icon(Icon::Close),
        ListItem::new(7, "Table").with_depth(2),
        ListItem::new(8, "Background").with_color(Color::from_hex(0x2D_9C_DB)),
        ListItem::new(9, "Notes").with_color(Color::from_hex(0xE0_4F_5F)),
    ]
}

/// The tree of [`list_items`] in a 200 x 160 window, Props folded, Camera and Sun selected
/// with Sun active, after `input` reached it: the frames before it lay the list out and
/// play its pointer moves.
fn list_scene(design: &DesignSystem, inputs: &[Input]) -> Scene {
    let styles = Styles::resolve(design);
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let mut list = ListState::new();
    list.set_folded(6, true);
    list.set_selected(2, true);
    list.set_active(Some(4));
    let items = list_items();
    for input in inputs {
        scene.clear();
        let mut ui = Ui::new(&mut scene, &styles, input, &mut state);
        ui.list(
            WidgetId::new("outliner"),
            rect(0.0, 0.0, 200.0, 160.0),
            &mut list,
            &items,
            &ListOptions::TREE,
            &mut (),
        );
    }
    scene
}

#[test]
fn trees_match_golden() {
    let Some(gpu) = gpu_or_skip("trees_match_golden") else {
        return;
    };
    let design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
    // The pointer over Lights.
    let mut hover = Input::new();
    hover.pointer_moved(Vec2::new(150.0, 50.0));
    let image = render(
        &gpu,
        Extent::new(200, 160),
        &list_scene(&design, &[hover, hover]),
    );
    assert_golden!(image, "tree", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}

#[test]
fn dragged_rows_match_golden() {
    let Some(gpu) = gpu_or_skip("dragged_rows_match_golden") else {
        return;
    };
    let design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
    // Background pressed, then dragged into the middle of Lights, then below Fill at the
    // pointer's level: after Lights.
    let mut frames = [Input::new(); 4];
    frames[0].pointer_moved(Vec2::new(150.0, 130.0));
    frames[1] = frames[0];
    frames[1].end_frame();
    frames[1].primary_button(true, Vec2::new(150.0, 130.0));
    frames[2] = frames[1];
    frames[2].end_frame();
    frames[2].pointer_moved(Vec2::new(150.0, 50.0));
    let image = render(
        &gpu,
        Extent::new(200, 160),
        &list_scene(&design, &frames[..3]),
    );
    assert_golden!(image, "tree_drag_into", CROSS_DRIVER);
    frames[3] = frames[2];
    frames[3].end_frame();
    frames[3].pointer_moved(Vec2::new(10.0, 97.0));
    let image = render(&gpu, Extent::new(200, 160), &list_scene(&design, &frames));
    assert_golden!(image, "tree_drag_after", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}
