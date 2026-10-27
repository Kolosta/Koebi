//! Menus and dropdowns on screen: a context menu with an open submenu and open dropdowns in
//! every placement match their golden images, and a dropdown squares the corners where it
//! meets its menu.

mod common;

use common::{assert_pixel, render};
use koebi_core::{Extent, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_render::Scene;
use koebi_testkit::{Tolerance, assert_golden, gpu_or_skip};
use koebi_text::Fonts;
use koebi_tokens::KOEBI;
use koebi_ui::{
    Button, ButtonGroup, Dropdown, GroupCell, Icon, Input, MENU_OPEN_SUBMENU, MENU_PREVIOUS,
    MenuItem, MenuState, Placement, Styles, Ui, UiState, WidgetId,
};

/// Anti-aliased edges may differ by a few units between drivers.
const CROSS_DRIVER: Tolerance = Tolerance::new(3, 8);

const SHEET: Extent = Extent::new(320, 240);

fn design() -> DesignSystem {
    DesignSystem::new(&KOEBI).expect("Koebi's schema is valid")
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Vec2::new(x, y), Vec2::new(width, height))
}

fn window() -> Rect {
    rect(0.0, 0.0, 320.0, 240.0)
}

const ADD: [MenuItem<'static>; 3] = [
    MenuItem::action("Cube"),
    MenuItem::action("Sphere"),
    MenuItem::action("Plane"),
];

/// A context menu with every kind of item; its submenu opened through the keyboard.
fn menu_sheet(design: &DesignSystem) -> Scene {
    let styles = Styles::resolve(design);
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let items = [
        MenuItem::Title("Object"),
        MenuItem::action("Duplicate")
            .icon(Icon::Plus)
            .shortcut("Shift+D"),
        MenuItem::action("Delete").shortcut("X").enabled(false),
        MenuItem::Separator,
        MenuItem::action("Snap").checked(true),
        MenuItem::action("Global").radio(true),
        MenuItem::action("Local").radio(false),
        MenuItem::Separator,
        MenuItem::submenu("Add", &ADD).icon(Icon::Plus),
    ];
    let mut menu = MenuState::new();
    menu.open_at(Vec2::new(8.0, 8.0));
    let mut input = Input::new();
    // The last item, then its submenu; a frame for each, then the frame drawn.
    for key in [Some(&MENU_PREVIOUS), Some(&MENU_OPEN_SUBMENU), None] {
        if let Some(key) = key {
            input.trigger(key);
        }
        scene.clear();
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        ui.menu(WidgetId::new("context"), &mut menu, &items, window());
        drop(ui);
        input.end_frame();
    }
    assert_eq!(menu.depth(), 2, "the submenu is open");
    scene
}

#[test]
fn menus_match_golden() {
    let Some(gpu) = gpu_or_skip("menus_match_golden") else {
        return;
    };
    let image = render(&gpu, SHEET, &menu_sheet(&design()));
    assert_golden!(image, "menus_dark", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}

const CHOICES: [MenuItem<'static>; 3] = [
    MenuItem::action("Solid").selected(true),
    MenuItem::action("Wireframe"),
    MenuItem::action("Rendered"),
];

/// Open dropdowns: below, flipped above near the bottom, vertical (beside), and a menu button
/// fused in a group.
fn dropdown_sheet(design: &DesignSystem) -> Scene {
    let styles = Styles::resolve(design);
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let input = Input::new();
    let below = rect(8.0, 8.0, 96.0, 20.0);
    let above = rect(8.0, 212.0, 96.0, 20.0);
    let beside = rect(120.0, 150.0, 80.0, 20.0);
    let fused = rect(120.0, 8.0, 132.0, 20.0);
    let mut menus: [MenuState; 4] = core::array::from_fn(|_| MenuState::new());
    menus[0].open_for(below, Placement::BELOW);
    menus[1].open_for(above, Placement::BELOW);
    menus[2].open_for(beside, Placement::RIGHT);
    menus[3].open_for(fused, Placement::BELOW);
    // A frame places the menus, the next one draws the controls squared where they meet.
    for _ in 0..2 {
        scene.clear();
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        let [first, second, third, fourth] = &mut menus;
        Dropdown::new(WidgetId::new("below"), first, &CHOICES)
            .label("Solid")
            .show(&mut ui, below, window());
        Dropdown::new(WidgetId::new("above"), second, &CHOICES)
            .label("Solid")
            .show(&mut ui, above, window());
        Dropdown::new(WidgetId::new("beside"), third, &CHOICES)
            .icon(Icon::Plus)
            .placement(Placement::RIGHT)
            .show(&mut ui, beside, window());
        let mut cells = [
            GroupCell::new(Button::new(WidgetId::new("back")).icon(Icon::ChevronLeft)),
            GroupCell::new(
                Button::new(WidgetId::new("mode"))
                    .label("Mode")
                    .trailing_icon(Icon::ChevronDown)
                    .menu(fourth, Placement::BELOW),
            ),
            GroupCell::new(Button::new(WidgetId::new("next")).icon(Icon::ChevronRight)),
        ];
        ButtonGroup::horizontal().show(&mut ui, fused, &mut cells);
        ui.menu(WidgetId::new("fused"), fourth, &CHOICES, window());
        drop(ui);
    }
    scene
}

#[test]
fn dropdowns_match_golden() {
    let Some(gpu) = gpu_or_skip("dropdowns_match_golden") else {
        return;
    };
    let image = render(&gpu, SHEET, &dropdown_sheet(&design()));
    assert_golden!(image, "dropdowns_dark", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}

#[test]
fn a_dropdown_and_its_menu_square_the_corners_where_they_meet() {
    let Some(gpu) = gpu_or_skip("a_dropdown_and_its_menu_square_the_corners_where_they_meet")
    else {
        return;
    };
    let design = design();
    let styles = Styles::resolve(&design);
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let input = Input::new();
    let control = rect(20.0, 20.0, 100.0, 20.0);
    let mut menu = MenuState::new();
    menu.open_for(control, Placement::BELOW);
    for _ in 0..2 {
        scene.clear();
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        Dropdown::new(WidgetId::new("view"), &mut menu, &CHOICES)
            .label("Solid")
            .corner_radius(6.0)
            .show(&mut ui, control, window());
        drop(ui);
    }
    let image = render(&gpu, Extent::new(160, 160), &scene);
    let placed = menu.placed().expect("open below");
    assert_eq!(placed.rect.min.y, control.max.y);
    let border = styles.dropdown.colors.border.selected.to_rgba8();
    let clear = styles.background.to_rgba8();
    // The control: rounded on top, square at the bottom where the menu starts.
    assert_pixel(&image, 20, 20, clear);
    assert_pixel(&image, 20, 39, border);
    // The menu: square at its top left, under the control; rounded at the bottom.
    let menu_border = styles.menu.border_color.to_rgba8();
    assert_pixel(&image, 20, 40, menu_border);
    // The last row of the menu, at its left edge: outside the rounded corner.
    let bottom = (0..160)
        .rev()
        .find(|y| f32::from(u16::try_from(*y).unwrap()) < placed.rect.max.y)
        .unwrap();
    assert_pixel(&image, 20, bottom, clear);
    gpu.assert_no_validation_errors();
}
