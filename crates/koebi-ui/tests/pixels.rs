//! Pixel tests: token values reach the screen through the components.
//!
//! Each test changes the design system the way a user would (an override, a theme switch,
//! a color vision mode), resolves the styles again, renders components offscreen and reads
//! back pixels whose expected color is the token's value.

mod common;

use common::{assert_pixel, over, render};
use koebi_core::{Color, Extent, Rect, Vec2};
use koebi_design::{ColorVision, Deficiency, DesignSystem, Scope};
use koebi_render::RectPrimitive;
use koebi_render::Scene;
use koebi_testkit::gpu_or_skip;
use koebi_text::Fonts;
use koebi_tokens::{KOEBI, component, semantic};
use koebi_ui::{
    BarMapping, Button, Checkbox, ControlState, Dropdown, Input, ListItem, ListOptions, ListState,
    MenuItem, MenuState, NumberField, PanelLevel, Styles, ThemePreview, Tone, TreeGuide, Ui,
    UiState, WidgetId,
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

fn button(ui: &mut Ui<'_>) {
    Button::new(WidgetId::new("button")).show(ui, rect(20.0, 20.0, 80.0, 40.0));
}

fn accent_button(ui: &mut Ui<'_>) {
    Button::new(WidgetId::new("button"))
        .tone(Tone::ACCENT)
        .show(ui, rect(20.0, 20.0, 80.0, 40.0));
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
    // component.button.accent.background.color.default -> semantic.background.accent.default
    let image = render(&gpu, SIZE, &draw(&design, accent_button));
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
    // A colored component: the neutral grays look the same to every vision.
    let normal = design.get(component::BUTTON_ACCENT_BACKGROUND_COLOR_DEFAULT);
    design
        .set_color_vision(ColorVision::Simulate {
            deficiency: Deficiency::Tritan,
            severity: 1.0,
        })
        .unwrap();
    let simulated = design.get(component::BUTTON_ACCENT_BACKGROUND_COLOR_DEFAULT);
    assert_ne!(simulated.to_rgba8(), normal.to_rgba8());
    let image = render(&gpu, SIZE, &draw(&design, accent_button));
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
        Checkbox::new(WidgetId::new("check"), &mut checked).show(ui, rect(8.0, 8.0, 120.0, 16.0));
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
        let mut half = 0.5;
        NumberField::new(WidgetId::new("drag"), &mut half)
            .range(0.0, 1.0)
            .bar(BarMapping::Linear)
            .show(ui, rect(0.0, 50.0, 200.0, 24.0));
    });
    let image = render(&gpu, SIZE, &scene);
    // Half of the number field's bar is filled.
    let fill = design.get(component::NUMBER_FIELD_FILL_COLOR_DEFAULT);
    let empty = design.get(component::NUMBER_FIELD_BACKGROUND_COLOR_DEFAULT);
    assert_pixel(&image, 60, 62, fill.to_rgba8());
    assert_pixel(&image, 140, 62, empty.to_rgba8());
    gpu.assert_no_validation_errors();
}

#[test]
fn vertical_bars_fill_from_the_bottom_and_disabled_bars_dim() {
    let Some(gpu) = gpu_or_skip("vertical_bars_fill_from_the_bottom_and_disabled_bars_dim") else {
        return;
    };
    let design = design();
    let scene = draw(&design, |ui| {
        let (mut quarter, mut half) = (0.25, 0.5);
        NumberField::new(WidgetId::new("tall"), &mut quarter)
            .range(0.0, 1.0)
            .bar(BarMapping::Linear)
            .vertical()
            .show(ui, rect(10.0, 4.0, 24.0, 92.0));
        NumberField::new(WidgetId::new("locked"), &mut half)
            .range(0.0, 1.0)
            .bar(BarMapping::Linear)
            .enabled(false)
            .show(ui, rect(60.0, 10.0, 120.0, 24.0));
    });
    let image = render(&gpu, SIZE, &scene);
    let fill = design.get(component::NUMBER_FIELD_FILL_COLOR_DEFAULT);
    let empty = design.get(component::NUMBER_FIELD_BACKGROUND_COLOR_DEFAULT);
    // A quarter: the bottom quarter is filled, the top is not.
    assert_pixel(&image, 22, 88, fill.to_rgba8());
    assert_pixel(&image, 22, 20, empty.to_rgba8());
    // A disabled field fills its bar in the disabled color.
    let disabled = design.get(component::NUMBER_FIELD_FILL_COLOR_DISABLED);
    assert_pixel(&image, 70, 22, disabled.to_rgba8());
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

/// A field showing a bar at (0, 0), with the pointer over its left end, step buttons shown.
fn stepped_bar(design: &DesignSystem, buttons: Option<bool>, pointer: Vec2) -> Scene {
    let styles = Styles::resolve(design);
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let mut input = Input::new();
    let mut value = 0.5;
    let mut last = Scene::new();
    for frame in 0..3 {
        if frame == 1 {
            input.pointer_moved(pointer);
        }
        last = Scene::new();
        last.set_clear_color(styles.background);
        let mut ui = Ui::new(&mut last, &styles, &input, &mut state);
        let field = NumberField::new(WidgetId::new("level"), &mut value)
            .range(0.0, 1.0)
            .bar(BarMapping::Linear);
        let field = match buttons {
            Some(shown) => field.step_buttons(shown),
            None => field,
        };
        field.show(&mut ui, rect(0.0, 0.0, 200.0, 24.0));
        drop(ui);
        input.end_frame();
    }
    last
}

#[test]
fn step_buttons_lie_inside_the_border_and_let_the_bar_show_through() {
    let Some(gpu) = gpu_or_skip("step_buttons_lie_inside_the_border_and_let_the_bar_show_through")
    else {
        return;
    };
    let design = design();
    let styles = Styles::resolve(&design);
    let style = styles.number_field;
    let pointer = Vec2::new(8.0, 12.0);
    // A bar has none by default: the pointer there drags, the bar in its hovered color.
    let image = render(&gpu, SIZE, &stepped_bar(&design, None, pointer));
    let hovered = design.get(component::NUMBER_FIELD_FILL_COLOR_HOVER);
    assert_pixel(&image, 3, 12, hovered.to_rgba8());
    // Asked for: the bar shows through the button's translucent background.
    let image = render(&gpu, SIZE, &stepped_bar(&design, Some(true), pointer));
    let blended = over(style.step_background, hovered);
    assert!(style.step_background.a < 1.0);
    assert_pixel(&image, 3, 3, blended.to_rgba8());
    // The border stays whole around it.
    let border = style.border.get(ControlState::Hovered);
    assert_pixel(&image, 8, 0, border.to_rgba8());
    assert_pixel(&image, 0, 12, border.to_rgba8());
    gpu.assert_no_validation_errors();
}

#[test]
fn tree_lines_run_along_the_children_in_three_looks() {
    let Some(gpu) = gpu_or_skip("tree_lines_run_along_the_children_in_three_looks") else {
        return;
    };
    let design = design();
    let tag = Color::from_rgba8(200, 30, 90, 255);
    let group = |id: u64, guide: TreeGuide| {
        [
            ListItem::new(id, "").with_guide(guide),
            ListItem::new(id + 1, "").with_depth(1),
            ListItem::new(id + 2, "").with_depth(1),
        ]
    };
    let items: Vec<ListItem<'_>> = group(0, TreeGuide::Line)
        .into_iter()
        .chain(group(3, TreeGuide::Color(tag)))
        .chain(group(6, TreeGuide::Dotted))
        .collect();
    let options = ListOptions {
        zebra: false,
        ..ListOptions::TREE
    };
    let mut list = ListState::new();
    let scene = draw(&design, |ui| {
        ui.list(
            WidgetId::new("tree"),
            rect(0.0, 0.0, 200.0, 180.0),
            &mut list,
            &items,
            &options,
            &mut (),
        );
    });
    let image = render(&gpu, Extent::new(200, 180), &scene);
    let styles = Styles::resolve(&design);
    let style = styles.list;
    let background = styles.background.to_rgba8();
    // On the middle of the chevron's slot: 4 pixels of padding, then half the 16 pixels of the
    // slot, the line one pixel wide.
    let x = 12;
    // A group's rows are 20 pixels high; the line starts and ends 4 pixels inside its children.
    assert_pixel(&image, x, 23, background);
    assert_pixel(&image, x, 24, style.line.to_rgba8());
    assert_pixel(&image, x, 55, style.line.to_rgba8());
    assert_pixel(&image, x, 56, background);
    assert_pixel(&image, x, 83, background);
    assert_pixel(&image, x, 84, tag.to_rgba8());
    assert_pixel(&image, x, 115, tag.to_rgba8());
    assert_pixel(&image, x, 116, background);
    // Dashes of 2 and gaps of 2, continuing across the rows.
    let dotted = style.line_dotted.to_rgba8();
    assert_pixel(&image, x, 143, background);
    assert_pixel(&image, x, 144, dotted);
    assert_pixel(&image, x, 145, dotted);
    assert_pixel(&image, x, 146, background);
    assert_pixel(&image, x, 147, background);
    assert_pixel(&image, x, 160, dotted);
    assert_pixel(&image, x, 162, background);
    assert_pixel(&image, x, 176, background);
    gpu.assert_no_validation_errors();
}

#[test]
fn list_highlights_lie_inside_their_rows() {
    let Some(gpu) = gpu_or_skip("list_highlights_lie_inside_their_rows") else {
        return;
    };
    let design = design();
    let items: Vec<ListItem<'_>> = (0..3).map(|id| ListItem::new(id, "")).collect();
    let mut list = ListState::new();
    list.set_active(Some(1));
    let options = ListOptions {
        zebra: false,
        ..ListOptions::LIST
    };
    let scene = draw(&design, |ui| {
        ui.list(
            WidgetId::new("list"),
            rect(0.0, 0.0, 200.0, 100.0),
            &mut list,
            &items,
            &options,
            &mut (),
        );
    });
    let image = render(&gpu, SIZE, &scene);
    let styles = Styles::resolve(&design);
    let active = styles.list.active.to_rgba8();
    let background = styles.background.to_rgba8();
    // The row lies at 20 to 40: its highlight a pixel inside, 2 pixels from the sides, and over
    // the place of a scrollbar: the list keeps its whole width.
    assert_pixel(&image, 50, 20, background);
    assert_pixel(&image, 50, 21, active);
    assert_pixel(&image, 50, 38, active);
    assert_pixel(&image, 50, 39, background);
    assert_pixel(&image, 1, 30, background);
    assert_pixel(&image, 2, 30, active);
    assert_pixel(&image, 197, 30, active);
    assert_pixel(&image, 198, 30, background);
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

/// A menu of three unnamed items at (10, 10), the second one the current choice, drawn with the
/// pointer at `pointer`.
fn menu_scene(design: &DesignSystem, pointer: Vec2) -> Scene {
    let styles = Styles::resolve(design);
    let mut menu = MenuState::new();
    menu.open_at(Vec2::new(10.0, 10.0));
    let items = [
        MenuItem::action(""),
        MenuItem::action("").selected(true),
        MenuItem::action(""),
    ];
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let mut input = Input::new();
    let bounds = rect(0.0, 0.0, 200.0, 100.0);
    // The pointer is resolved against the last frame's layout, and takes the highlight when it
    // moves.
    let mut last = Scene::new();
    for frame in 0..3 {
        if frame == 1 {
            input.pointer_moved(pointer);
        }
        last = Scene::new();
        last.set_clear_color(styles.background);
        let mut ui = Ui::new(&mut last, &styles, &input, &mut state);
        ui.menu(WidgetId::new("menu"), &mut menu, &items, bounds);
        drop(ui);
        input.end_frame();
    }
    last
}

#[test]
fn menu_items_show_their_highlight_inside_their_row() {
    let Some(gpu) = gpu_or_skip("menu_items_show_their_highlight_inside_their_row") else {
        return;
    };
    let design = design();
    let background = design
        .get(component::MENU_BACKGROUND_COLOR_DEFAULT)
        .to_rgba8();
    let hover = design
        .get(component::MENU_ITEM_BACKGROUND_COLOR_HOVER)
        .to_rgba8();
    let selected = design
        .get(component::MENU_ITEM_BACKGROUND_COLOR_SELECTED)
        .to_rgba8();
    let selected_hover = design
        .get(component::MENU_ITEM_BACKGROUND_COLOR_SELECTED_HOVER)
        .to_rgba8();
    assert_ne!(selected, selected_hover);
    // The border is 1 pixel and the padding 6, counted to the first highlight: the rows lie at
    // 16 to 36, 36 to 56 and 56 to 76, their highlights a pixel inside them, 2 from the sides.
    let image = render(&gpu, SIZE, &menu_scene(&design, Vec2::new(20.0, 26.0)));
    assert_pixel(&image, 20, 26, hover);
    assert_pixel(&image, 20, 46, selected);
    // Between two highlights, and at the side of one: the panel.
    assert_pixel(&image, 20, 16, background);
    assert_pixel(&image, 20, 17, hover);
    assert_pixel(&image, 20, 34, hover);
    assert_pixel(&image, 20, 35, background);
    assert_pixel(&image, 20, 36, background);
    assert_pixel(&image, 20, 37, selected);
    assert_pixel(&image, 12, 26, background);
    assert_pixel(&image, 13, 26, hover);
    // The current choice under the pointer: a color of its own.
    let image = render(&gpu, SIZE, &menu_scene(&design, Vec2::new(20.0, 46.0)));
    assert_pixel(&image, 20, 46, selected_hover);
    assert_pixel(&image, 20, 26, background);
    gpu.assert_no_validation_errors();
}

#[test]
fn a_dropdowns_chevron_sits_at_the_end_of_its_control() {
    let Some(gpu) = gpu_or_skip("a_dropdowns_chevron_sits_at_the_end_of_its_control") else {
        return;
    };
    let design = design();
    let items = [MenuItem::action("One")];
    let bounds = rect(0.0, 0.0, 200.0, 100.0);
    let control = rect(10.0, 10.0, 180.0, 20.0);
    let mut menu = MenuState::new();
    let scene = draw(&design, |ui| {
        Dropdown::new(WidgetId::new("dropdown"), &mut menu, &items)
            .label("A")
            .show(ui, control, bounds);
    });
    let image = render(&gpu, SIZE, &scene);
    let fill = design
        .get(component::DROPDOWN_BACKGROUND_COLOR_DEFAULT)
        .to_rgba8();
    let drawn_in =
        |from: u32, to: u32| (from..to).any(|x| (12..28).any(|y| image.pixel(x, y) != fill));
    // Nothing in the middle, the chevron near the end.
    assert!(!drawn_in(60, 160));
    assert!(drawn_in(165, 188));
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
