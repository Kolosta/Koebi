//! Pixel tests: token values reach the screen through the components.
//!
//! Each test changes the design system the way a user would (an override, a theme switch,
//! a color vision mode), resolves the styles again, renders components offscreen and reads
//! back pixels whose expected color is the token's value.

mod common;

use common::{assert_pixel, render};
use koebi_core::{Color, Extent, Rect, Vec2};
use koebi_design::{ColorVision, Deficiency, DesignSystem, Scope};
use koebi_render::RectPrimitive;
use koebi_render::Scene;
use koebi_testkit::gpu_or_skip;
use koebi_tokens::{KOEBI, component, semantic};
use koebi_ui::{
    Input, ListItem, ListOptions, ListState, MenuItem, MenuState, PanelLevel, Styles, ThemePreview,
    Ui, UiState, WidgetId,
};

const SIZE: Extent = Extent::new(200, 100);

fn design() -> DesignSystem {
    DesignSystem::new(&KOEBI).expect("Koebi's schema is valid")
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Vec2::new(x, y), Vec2::new(width, height))
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

/// `top` over the opaque `bottom`, blended as the renderer does: premultiplied, in the
/// gamma-encoded values of the UNORM target.
fn over(top: Color, bottom: Color) -> Color {
    let mix = |t: f32, b: f32| t * top.a + b * (1.0 - top.a);
    Color::new(
        mix(top.r, bottom.r),
        mix(top.g, bottom.g),
        mix(top.b, bottom.b),
        1.0,
    )
}

fn button(ui: &mut Ui<'_>) {
    ui.button(WidgetId::new("button"), rect(20.0, 20.0, 80.0, 40.0), None);
}

#[test]
fn component_color_tokens_reach_the_screen() {
    let Some(gpu) = gpu_or_skip("component_color_tokens_reach_the_screen") else {
        return;
    };
    let mut design = design();
    let image = render(&gpu, SIZE, &draw(&design, button));
    let expected = design.get(component::BUTTON_BACKGROUND_COLOR_DEFAULT);
    assert_pixel(&image, 60, 40, expected.to_rgba8());

    let red = Color::from_rgba8(200, 30, 90, 255);
    design
        .set_value(
            Scope::Global,
            component::BUTTON_BACKGROUND_COLOR_DEFAULT,
            red,
        )
        .unwrap();
    let image = render(&gpu, SIZE, &draw(&design, button));
    assert_pixel(&image, 60, 40, red.to_rgba8());
    gpu.assert_no_validation_errors();
}

#[test]
fn semantic_edits_reach_components_through_aliases() {
    let Some(gpu) = gpu_or_skip("semantic_edits_reach_components_through_aliases") else {
        return;
    };
    let mut design = design();
    let green = Color::from_rgba8(20, 180, 90, 255);
    design
        .set_value(
            Scope::ActiveTheme,
            semantic::BACKGROUND_ACCENT_DEFAULT,
            green,
        )
        .unwrap();
    // component.button.background.color.default -> semantic.background.accent.default
    let image = render(&gpu, SIZE, &draw(&design, button));
    assert_pixel(&image, 60, 40, green.to_rgba8());
    gpu.assert_no_validation_errors();
}

#[test]
fn theme_switches_restyle_components() {
    let Some(gpu) = gpu_or_skip("theme_switches_restyle_components") else {
        return;
    };
    let mut design = design();
    let panel = |ui: &mut Ui<'_>| {
        ui.panel(rect(10.0, 10.0, 180.0, 80.0), PanelLevel::First);
    };
    let dark = design.get(component::PANEL_BODY_BACKGROUND_COLOR_LEVEL_1);
    let image = render(&gpu, SIZE, &draw(&design, panel));
    assert_pixel(&image, 100, 50, dark.to_rgba8());
    assert_pixel(
        &image,
        2,
        2,
        design
            .get(component::WINDOW_BACKGROUND_COLOR_DEFAULT)
            .to_rgba8(),
    );

    let light = design.find_theme("light").expect("built-in theme");
    design.set_active_theme(light).unwrap();
    let expected = design.get(component::PANEL_BODY_BACKGROUND_COLOR_LEVEL_1);
    assert_ne!(expected, dark);
    let image = render(&gpu, SIZE, &draw(&design, panel));
    assert_pixel(&image, 100, 50, expected.to_rgba8());
    gpu.assert_no_validation_errors();
}

#[test]
fn color_vision_transforms_component_colors() {
    let Some(gpu) = gpu_or_skip("color_vision_transforms_component_colors") else {
        return;
    };
    let mut design = design();
    let normal = design.get(component::BUTTON_BACKGROUND_COLOR_DEFAULT);
    design
        .set_color_vision(ColorVision::Simulate {
            deficiency: Deficiency::Tritan,
            severity: 1.0,
        })
        .unwrap();
    let simulated = design.get(component::BUTTON_BACKGROUND_COLOR_DEFAULT);
    assert_ne!(simulated.to_rgba8(), normal.to_rgba8());
    let image = render(&gpu, SIZE, &draw(&design, button));
    assert_pixel(&image, 60, 40, simulated.to_rgba8());
    gpu.assert_no_validation_errors();
}

#[test]
fn dimension_tokens_reach_the_screen() {
    let Some(gpu) = gpu_or_skip("dimension_tokens_reach_the_screen") else {
        return;
    };
    let mut design = design();
    let fill = design
        .get(component::BUTTON_BACKGROUND_COLOR_DEFAULT)
        .to_rgba8();
    let background = design
        .get(component::WINDOW_BACKGROUND_COLOR_DEFAULT)
        .to_rgba8();
    // Square corners: the corner pixel of the button is filled.
    design
        .set_value(Scope::Global, component::BUTTON_CORNER_RADIUS_DEFAULT, 0.0)
        .unwrap();
    let image = render(&gpu, SIZE, &draw(&design, button));
    assert_pixel(&image, 20, 20, fill);
    // A 12 px radius leaves it to the background.
    design
        .set_value(Scope::Global, component::BUTTON_CORNER_RADIUS_DEFAULT, 12.0)
        .unwrap();
    let image = render(&gpu, SIZE, &draw(&design, button));
    assert_pixel(&image, 20, 20, background);
    assert_pixel(&image, 60, 40, fill);
    gpu.assert_no_validation_errors();
}

#[test]
fn check_marks_use_their_tokens() {
    let Some(gpu) = gpu_or_skip("check_marks_use_their_tokens") else {
        return;
    };
    let mut design = design();
    let mark = Color::from_rgba8(250, 210, 40, 255);
    design
        .set_value(Scope::Global, component::CHECKBOX_MARK_COLOR_DEFAULT, mark)
        .unwrap();
    let scene = draw(&design, |ui| {
        let mut checked = true;
        ui.checkbox(
            WidgetId::new("check"),
            rect(8.0, 8.0, 120.0, 16.0),
            &mut checked,
        );
    });
    let image = render(&gpu, SIZE, &scene);
    // The 16 px box sits at (8, 8); the mark's corner is at (6.5, 11.5) in the box.
    assert_pixel(&image, 14, 19, mark.to_rgba8());
    let checked = design.get(component::CHECKBOX_BACKGROUND_COLOR_SELECTED);
    assert_pixel(&image, 20, 10, checked.to_rgba8());
    gpu.assert_no_validation_errors();
}

#[test]
fn value_components_show_their_value() {
    let Some(gpu) = gpu_or_skip("value_components_show_their_value") else {
        return;
    };
    let design = design();
    let scene = draw(&design, |ui| {
        let (mut full, mut half) = (1.0, 0.5);
        ui.slider(
            WidgetId::new("slider"),
            rect(0.0, 10.0, 200.0, 20.0),
            &mut full,
            0.0..=1.0,
        );
        ui.drag_value(
            WidgetId::new("drag"),
            rect(0.0, 50.0, 200.0, 24.0),
            &mut half,
            0.0..=1.0,
            0.0,
        );
    });
    let image = render(&gpu, SIZE, &scene);
    // The grab of a full slider touches the right end, inside the padding.
    let grab = design.get(component::SLIDER_GRAB_COLOR_DEFAULT);
    assert_pixel(&image, 192, 20, grab.to_rgba8());
    let track = design.get(component::SLIDER_TRACK_COLOR_DEFAULT);
    assert_pixel(&image, 100, 20, track.to_rgba8());
    // Half of the drag value is filled.
    let fill = design.get(component::DRAG_VALUE_FILL_COLOR_DEFAULT);
    let empty = design.get(component::DRAG_VALUE_BACKGROUND_COLOR_DEFAULT);
    assert_pixel(&image, 60, 62, fill.to_rgba8());
    assert_pixel(&image, 140, 62, empty.to_rgba8());
    gpu.assert_no_validation_errors();
}

#[test]
fn theme_cards_paint_other_themes() {
    let Some(gpu) = gpu_or_skip("theme_cards_paint_other_themes") else {
        return;
    };
    let design = design();
    let light = design.find_theme("light").expect("built-in theme");
    let preview = ThemePreview::of(&design, light).expect("live theme");
    let scene = draw(&design, |ui| {
        ui.theme_card(
            WidgetId::new("light"),
            rect(0.0, 0.0, 160.0, 100.0),
            &preview,
            false,
        );
    });
    let image = render(&gpu, SIZE, &scene);
    // The preview background, between the heading and the card's right edge, is the light
    // theme's; the card's frame is the active (dark) theme's.
    assert_pixel(&image, 145, 10, preview.background.to_rgba8());
    let frame = design.get(component::THEME_CARD_BORDER_COLOR_DEFAULT);
    assert_pixel(&image, 80, 0, frame.to_rgba8());
    gpu.assert_no_validation_errors();
}

#[test]
fn list_rows_alternate_zebra_colors_and_show_the_selection() {
    let Some(gpu) = gpu_or_skip("list_rows_alternate_zebra_colors_and_show_the_selection") else {
        return;
    };
    let mut design = design();
    let items: Vec<ListItem<'_>> = (0..5).map(|id| ListItem::new(id, "")).collect();
    let mut list = ListState::new();
    list.set_selected(2, true);
    list.set_active(Some(3));
    let scene = |design: &DesignSystem, list: &mut ListState| {
        draw(design, |ui| {
            ui.list(
                WidgetId::new("list"),
                rect(0.0, 0.0, 200.0, 100.0),
                list,
                &items,
                &ListOptions::LIST,
                &mut (),
            );
        })
    };
    let image = render(&gpu, SIZE, &scene(&design, &mut list));
    let styles = Styles::resolve(&design);
    let (style, background) = (styles.list, styles.background);
    let even = over(style.zebra[0], background);
    let odd = over(style.zebra[1], background);
    // Rows are one unit (20 pixels) high: even and odd rows alternate under the selected
    // row and the active one.
    let rows = [
        even,
        odd,
        over(style.selected, even),
        over(style.active, odd),
        even,
    ];
    for (row, expected) in (0_u32..).zip(rows) {
        assert_pixel(&image, 150, 10 + 20 * row, expected.to_rgba8());
    }
    // The zebra follows its token.
    let red = Color::from_rgba8(200, 30, 90, 255);
    design
        .set_value(
            Scope::Global,
            component::TABLE_ROW_BACKGROUND_COLOR_ALT,
            red,
        )
        .unwrap();
    let image = render(&gpu, SIZE, &scene(&design, &mut list));
    assert_pixel(&image, 150, 30, red.to_rgba8());
    assert_pixel(&image, 150, 10, even.to_rgba8());
    gpu.assert_no_validation_errors();
}

#[test]
fn menus_cover_what_is_drawn_after_them() {
    let Some(gpu) = gpu_or_skip("menus_cover_what_is_drawn_after_them") else {
        return;
    };
    // Regression test for #102: a menu opened from a top bar used to draw under the areas
    // drawn after it.
    let design = design();
    let mut menu = MenuState::new();
    menu.open_at(Vec2::new(10.0, 10.0));
    let items = [MenuItem::action(""), MenuItem::action("")];
    let red = Color::from_rgba8(200, 30, 90, 255);
    let scene = draw(&design, |ui| {
        ui.menu(
            WidgetId::new("menu"),
            &mut menu,
            &items,
            rect(0.0, 0.0, 200.0, 100.0),
        );
        // An area drawn after the menu, over the whole window.
        ui.scene()
            .push_rect(&RectPrimitive::new(rect(0.0, 0.0, 200.0, 100.0), red));
    });
    let image = render(&gpu, SIZE, &scene);
    // The menu has no labels: it is narrow, from x = 10 to about 36.
    let background = design.get(component::MENU_BACKGROUND_COLOR_DEFAULT);
    assert_pixel(&image, 18, 24, background.to_rgba8());
    assert_pixel(&image, 150, 80, red.to_rgba8());
    gpu.assert_no_validation_errors();
}
