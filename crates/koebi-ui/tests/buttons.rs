//! Buttons, checkboxes and text fields on screen: every tone shows its tokens' colors in every
//! state, components stay inside their rectangles whatever the values of their tokens, and
//! sheets of buttons, groups and checkboxes match their golden images.

mod common;

use common::{assert_pixel, over, render};
use koebi_core::{Color, Extent, Rect, Vec2};
use koebi_design::kind::KindId;
use koebi_design::value::{Definition, Range, Value};
use koebi_design::{DesignSystem, Scope};
use koebi_render::Scene;
use koebi_testkit::{Tolerance, assert_golden, gpu_or_skip};
use koebi_text::Fonts;
use koebi_tokens::KOEBI;
use koebi_ui::{
    Align, Button, ButtonGroup, Checkbox, ControlState, GroupCell, Hue, Icon, Input, Styles,
    TextField, Tone, Ui, UiState, Variant, WidgetId,
};

/// Anti-aliased edges may differ by a few units between drivers.
const CROSS_DRIVER: Tolerance = Tolerance::new(3, 8);

/// The size of the button sheet.
const SHEET: Extent = Extent::new(320, 240);

/// The rows of the color families: filled variants, then the default variant.
const FILLED_ROW: f32 = 168.0;
const SUBTLE_ROW: f32 = 192.0;

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
        reason = "coordinates inside a small image"
    )]
    let value = value.max(0.0) as u32;
    value
}

/// A count of cells as a coordinate factor.
fn factor(index: usize) -> f32 {
    f32::from(u8::try_from(index).expect("a few cells"))
}

/// Where the sheet puts the button of `variant` in `state`: one row per variant, one column
/// per state.
fn state_cell(variant: Variant, state: ControlState) -> Rect {
    rect(
        8.0 + 40.0 * factor(state.index()),
        8.0 + 22.0 * factor(variant.index()),
        36.0,
        20.0,
    )
}

/// Where the sheet puts the button of `hue` in the row at `top`.
fn hue_cell(hue: Hue, top: f32) -> Rect {
    rect(8.0 + 38.0 * factor(hue.index()), top, 36.0, 20.0)
}

/// The state the sheet shows the default-variant button of `hue` in: every other one
/// selected.
fn subtle_state(hue: Hue) -> ControlState {
    if hue.index().is_multiple_of(2) {
        ControlState::Normal
    } else {
        ControlState::Selected
    }
}

/// Shows the button `build` makes in `rect`, in `state`, into `scene`: a frame lays it out
/// on its own UI state, then a frame draws it with the pointer placed (and pressed) as the
/// state needs.
fn show_in_state(
    scene: &mut Scene,
    styles: &Styles,
    fonts: &Fonts,
    rect: Rect,
    state: ControlState,
    build: impl Fn() -> Button<'static>,
) {
    let pointer = match state {
        ControlState::Hovered | ControlState::Pressed | ControlState::SelectedHovered => {
            rect.center()
        }
        _ => Vec2::new(-100.0, -100.0),
    };
    let selected = matches!(
        state,
        ControlState::Selected | ControlState::SelectedHovered
    );
    let enabled = state != ControlState::Disabled;
    let mut input = Input::new();
    input.pointer_moved(pointer);
    let mut ui_state = UiState::new();
    ui_state.set_fonts(fonts.clone());
    let mut layout = Scene::new();
    build().selected(selected).enabled(enabled).show(
        &mut Ui::new(&mut layout, styles, &input, &mut ui_state),
        rect,
    );
    input.end_frame();
    if state == ControlState::Pressed {
        input.primary_button(true, pointer);
    }
    build()
        .selected(selected)
        .enabled(enabled)
        .show(&mut Ui::new(scene, styles, &input, &mut ui_state), rect);
}

/// Every variant in every state, the color families, groups and content, styled by the
/// active theme of `design`.
fn button_sheet(design: &DesignSystem) -> Scene {
    let styles = Styles::resolve(design);
    let fonts = Fonts::with_embedded();
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    let id = WidgetId::new;
    for variant in Variant::ALL {
        for state in ControlState::ALL {
            let tone = Tone::new(variant);
            let cell = state_cell(variant, state);
            show_in_state(&mut scene, &styles, &fonts, cell, state, || {
                Button::new(id("state")).icon(Icon::Plus).tone(tone)
            });
        }
    }
    for hue in Hue::ALL {
        let filled = hue_cell(hue, FILLED_ROW);
        show_in_state(
            &mut scene,
            &styles,
            &fonts,
            filled,
            ControlState::Normal,
            || {
                Button::new(id("filled"))
                    .label("Aa")
                    .tone(Tone::ACCENT.with_hue(hue))
            },
        );
        let subtle = hue_cell(hue, SUBTLE_ROW);
        show_in_state(
            &mut scene,
            &styles,
            &fonts,
            subtle,
            subtle_state(hue),
            || {
                Button::new(id("subtle"))
                    .label("Aa")
                    .tone(Tone::DEFAULT.with_hue(hue))
            },
        );
    }

    // Groups and content, at rest.
    let mut state = UiState::new();
    state.set_fonts(fonts);
    let input = Input::new();
    let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
    let mut stack = [
        GroupCell::new(Button::new(id("up")).icon(Icon::ChevronUp)),
        GroupCell::new(Button::new(id("check")).icon(Icon::Check).selected(true)),
        GroupCell::new(Button::new(id("down")).icon(Icon::ChevronDown)),
    ];
    ButtonGroup::vertical().show(&mut ui, rect(252.0, 8.0, 60.0, 60.0), &mut stack);
    let mut modifiers = [
        GroupCell::new(Button::new(id("ctrl")).label("Ctrl"))
            .at(0, 0)
            .span(2, 1),
        GroupCell::new(Button::new(id("left")).label("L").selected(true)).at(0, 1),
        GroupCell::new(Button::new(id("right")).label("R")).at(1, 1),
    ];
    ButtonGroup::horizontal().show(&mut ui, rect(252.0, 76.0, 60.0, 40.0), &mut modifiers);
    let mut grid = [
        GroupCell::new(Button::new(id("plus")).icon(Icon::Plus)).at(0, 0),
        GroupCell::new(Button::new(id("minus")).icon(Icon::Minus)).at(1, 0),
        GroupCell::new(Button::new(id("close")).icon(Icon::Close).enabled(false)).at(0, 1),
        GroupCell::new(
            Button::new(id("accept"))
                .icon(Icon::Check)
                .tone(Tone::ACCENT),
        )
        .at(1, 1),
    ];
    ButtonGroup::horizontal().show(&mut ui, rect(252.0, 124.0, 60.0, 40.0), &mut grid);

    let mut x = 8.0;
    let mut place = |ui: &mut Ui<'_>, button: Button<'_>| {
        let size = button.size(ui);
        button.show(ui, Rect::from_origin_size(Vec2::new(x, 216.0), size));
        x += size.x + 6.0;
    };
    place(
        &mut ui,
        Button::new(id("full"))
            .leading_icon(Icon::Plus)
            .label("Add")
            .status_dot(Color::from_hex(0x27_AE_60))
            .trailing_icon(Icon::ChevronDown),
    );
    place(&mut ui, Button::new(id("off")).label("Off").enabled(false));
    place(
        &mut ui,
        Button::new(id("start"))
            .label("Start")
            .align(Align::Start)
            .width(64.0),
    );
    place(
        &mut ui,
        Button::new(id("cut"))
            .label("A label too long")
            .max_width(60.0),
    );
    drop(ui);
    scene
}

#[test]
fn every_tone_shows_its_token_colors_in_every_state() {
    let Some(gpu) = gpu_or_skip("every_tone_shows_its_token_colors_in_every_state") else {
        return;
    };
    let design = design();
    let styles = Styles::resolve(&design);
    let image = render(&gpu, SHEET, &button_sheet(&design));
    // Beside the content, away from the rounded corners: the background alone.
    let background = |cell: Rect| (px(cell.min.x + 5.0), px(cell.center().y));
    for variant in Variant::ALL {
        for state in ControlState::ALL {
            let (x, y) = background(state_cell(variant, state));
            let expected = styles
                .button
                .colors(Tone::new(variant))
                .background
                .get(state);
            let shown = over(expected, styles.background).to_rgba8();
            let actual = image.pixel(x, y);
            let close = actual.iter().zip(shown).all(|(a, e)| a.abs_diff(e) <= 1);
            assert!(
                close,
                "{variant:?} {state:?}: pixel ({x}, {y}) is {actual:?}, expected {shown:?}"
            );
        }
    }
    for hue in Hue::ALL {
        let (x, y) = background(hue_cell(hue, FILLED_ROW));
        let filled = styles.button.colors(Tone::ACCENT.with_hue(hue));
        assert_pixel(&image, x, y, filled.background.normal.to_rgba8());
        let (x, y) = background(hue_cell(hue, SUBTLE_ROW));
        let subtle = styles
            .button
            .colors(Tone::DEFAULT.with_hue(hue))
            .background
            .get(subtle_state(hue));
        assert_pixel(&image, x, y, over(subtle, styles.background).to_rgba8());
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn buttons_match_golden() {
    let Some(gpu) = gpu_or_skip("buttons_match_golden") else {
        return;
    };
    let image = render(&gpu, SHEET, &button_sheet(&design()));
    assert_golden!(image, "buttons_dark", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}

/// Checkboxes unchecked, checked, mixed and disabled, with labels and descriptions.
fn checkbox_sheet(design: &DesignSystem) -> Scene {
    let styles = Styles::resolve(design);
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let input = Input::new();
    let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
    let id = WidgetId::new;
    let mut y = 8.0;
    let mut place = |ui: &mut Ui<'_>, checkbox: Checkbox<'_>| {
        let size = checkbox.size(ui);
        checkbox.show(ui, Rect::from_origin_size(Vec2::new(8.0, y), size));
        y += size.y + 4.0;
    };
    let (mut off, mut on, mut mixed, mut disabled) = (false, true, false, true);
    place(
        &mut ui,
        Checkbox::new(id("off"), &mut off).label("Unchecked"),
    );
    place(
        &mut ui,
        Checkbox::new(id("on"), &mut on)
            .label("Snap to grid")
            .description("Moves items by whole cells"),
    );
    place(
        &mut ui,
        Checkbox::new(id("mixed"), &mut mixed)
            .label("Some layers")
            .mixed(true),
    );
    place(
        &mut ui,
        Checkbox::new(id("disabled"), &mut disabled)
            .label("Locked")
            .enabled(false),
    );
    drop(ui);
    scene
}

#[test]
fn checkboxes_match_golden() {
    let Some(gpu) = gpu_or_skip("checkboxes_match_golden") else {
        return;
    };
    let image = render(&gpu, Extent::new(200, 120), &checkbox_sheet(&design()));
    assert_golden!(image, "checkboxes_dark", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}

/// A value inside `range` at its low or high end; open ends take a value far from the other
/// end.
fn extreme(range: Range, high: bool) -> Option<f32> {
    let (min, max) = (range.min(), range.max());
    let candidates = if high {
        [max, min + 64.0, min + 1.0]
    } else {
        [min, min + range.step().max(0.25), max - 64.0]
    };
    #[allow(
        clippy::cast_possible_truncation,
        reason = "token values are f32; the ranges hold them"
    )]
    candidates
        .into_iter()
        .find(|value| value.is_finite() && range.contains(*value))
        .map(|value| value as f32)
}

/// The design system with every numeric button and checkbox token at the low or high end of
/// its range.
fn at_extremes(high: bool) -> DesignSystem {
    let mut design = design();
    let tokens: Vec<_> = KOEBI
        .tokens()
        .filter(|(_, info)| {
            let path = info.path();
            ["button.", "checkbox.", "text-field."]
                .iter()
                .any(|prefix| path.starts_with(prefix))
        })
        .filter_map(|(token, info)| Some((token, info.kind(), design.range(token)?)))
        .collect();
    for (token, kind, range) in tokens {
        let Some(value) = extreme(range, high) else {
            continue;
        };
        let value = match kind {
            KindId::Dimension => Value::Dimension(value),
            KindId::Number => Value::Number(value),
            _ => continue,
        };
        design
            .set_override(Scope::Global, token, Definition::Value(value))
            .expect("a value inside the token's range");
    }
    design
}

/// Buttons, a group and checkboxes at their own sizes, one below the other; returns the scene
/// and the rectangles they were given.
fn fitted_components(design: &DesignSystem) -> (Scene, Vec<Rect>) {
    let styles = Styles::resolve(design);
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let input = Input::new();
    let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
    let id = WidgetId::new;
    let mut rects = Vec::new();
    let mut y = 8.0;
    let mut next = |size: Vec2| {
        let rect = Rect::from_origin_size(Vec2::new(8.0, y), size);
        y += size.y + 8.0;
        rects.push(rect);
        rect
    };
    let buttons = [
        Button::new(id("label")).label("Save"),
        Button::new(id("icon")).icon(Icon::Plus),
        Button::new(id("full"))
            .leading_icon(Icon::Check)
            .label("Render")
            .status_dot(Color::WHITE)
            .trailing_icon(Icon::ChevronDown)
            .tone(Tone::ACCENT),
        Button::new(id("selected")).label("On").selected(true),
    ];
    for button in buttons {
        let size = button.size(&mut ui);
        button.show(&mut ui, next(size));
    }
    let mut modifiers = [
        GroupCell::new(Button::new(id("ctrl")).label("Ctrl"))
            .at(0, 0)
            .span(2, 1),
        GroupCell::new(Button::new(id("left")).label("L").selected(true)).at(0, 1),
        GroupCell::new(Button::new(id("right")).label("R")).at(1, 1),
    ];
    let group = ButtonGroup::horizontal();
    let size = group.size(&mut ui, &modifiers);
    group.show(&mut ui, next(size), &mut modifiers);
    let (mut on, mut mixed) = (true, false);
    let checkboxes = [
        Checkbox::new(id("on"), &mut on)
            .label("Snap")
            .description("Moves items by whole cells"),
        Checkbox::new(id("mixed"), &mut mixed)
            .label("Some")
            .mixed(true),
    ];
    for checkbox in checkboxes {
        let size = checkbox.size(&mut ui);
        checkbox.show(&mut ui, next(size));
    }
    let (mut name, mut search) = (String::from("Cube.001"), String::new());
    let fields = [
        TextField::new(id("name"), &mut name),
        TextField::new(id("search"), &mut search)
            .placeholder("Search")
            .icon(Icon::Search),
    ];
    for field in fields {
        let size = field.size(&mut ui);
        field.show(&mut ui, next(Vec2::new(size.x.max(120.0), size.y)));
    }
    drop(ui);
    (scene, rects)
}

#[test]
fn components_stay_inside_their_rectangles_at_every_token_extreme() {
    let Some(gpu) = gpu_or_skip("components_stay_inside_their_rectangles_at_every_token_extreme")
    else {
        return;
    };
    let extent = Extent::new(400, 900);
    for high in [false, true] {
        let design = at_extremes(high);
        let (scene, rects) = fitted_components(&design);
        let bottom = rects.iter().map(|rect| rect.max.y).fold(0.0, f32::max);
        assert!(bottom < 900.0, "the components fit the image: {bottom}");
        let image = render(&gpu, extent, &scene);
        let clear = scene.clear_color().to_rgba8();
        // Anti-aliasing may reach one pixel past an edge.
        let reach: Vec<Rect> = rects.iter().map(|rect| rect.inset(-1.0)).collect();
        let center = |value: u32| f32::from(u16::try_from(value).unwrap()) + 0.5;
        for y in 0..extent.height {
            for x in 0..extent.width {
                let center = Vec2::new(center(x), center(y));
                if reach.iter().any(|rect| rect.contains(center)) {
                    continue;
                }
                let actual = image.pixel(x, y);
                let close = actual.iter().zip(clear).all(|(a, e)| a.abs_diff(e) <= 1);
                assert!(
                    close,
                    "{} extremes: pixel ({x}, {y}) is {actual:?} outside every component",
                    if high { "high" } else { "low" }
                );
            }
        }
    }
    gpu.assert_no_validation_errors();
}

/// Two cells, "A" selected and "B", in `rect` with outer corners of `radius`, the pointer at
/// `pointer`: a frame lays the group out, the next draws it.
fn pair(styles: &Styles, rect: Rect, pointer: Vec2, radius: f32) -> Scene {
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    let mut layout = Scene::new();
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let mut input = Input::new();
    input.pointer_moved(pointer);
    for target in [&mut layout, &mut scene] {
        let mut ui = Ui::new(target, styles, &input, &mut state);
        let mut cells = [
            GroupCell::new(Button::new(WidgetId::new("a")).label("A").selected(true)),
            GroupCell::new(Button::new(WidgetId::new("b")).label("B")),
        ];
        ButtonGroup::horizontal()
            .corner_radius(radius)
            .show(&mut ui, rect, &mut cells);
        drop(ui);
        input.end_frame();
    }
    scene
}

#[test]
fn shared_borders_take_the_color_of_the_cell_of_highest_priority() {
    let Some(gpu) = gpu_or_skip("shared_borders_take_the_color_of_the_cell_of_highest_priority")
    else {
        return;
    };
    let styles = Styles::resolve(&design());
    let colors = styles.button.colors(Tone::DEFAULT);
    let (hovered, selected) = (colors.border.hovered, colors.border.selected);
    assert_ne!(hovered, selected);
    // Two cells of 40 pixels: the line they share is the column x = 50. "A" is selected; the
    // pointer hovers "B", then "A" itself.
    let group = rect(10.0, 10.0, 80.0, 20.0);
    let image = render(
        &gpu,
        Extent::new(100, 40),
        &pair(&styles, group, Vec2::new(70.0, 20.0), 3.0),
    );
    assert_pixel(&image, 50, 20, hovered.to_rgba8());
    // Selected and hovered ranks above selected alone, and below nothing here.
    let image = render(
        &gpu,
        Extent::new(100, 40),
        &pair(&styles, group, Vec2::new(30.0, 20.0), 3.0),
    );
    assert_pixel(&image, 50, 20, colors.border.selected_hovered.to_rgba8());
    // Away from both, the selected cell owns the line.
    let image = render(
        &gpu,
        Extent::new(100, 40),
        &pair(&styles, group, Vec2::new(-10.0, -10.0), 3.0),
    );
    assert_pixel(&image, 50, 20, selected.to_rgba8());
    gpu.assert_no_validation_errors();
}

#[test]
fn only_the_outer_corners_of_a_group_are_rounded_wherever_it_lies() {
    let Some(gpu) = gpu_or_skip("only_the_outer_corners_of_a_group_are_rounded_wherever_it_lies")
    else {
        return;
    };
    let styles = Styles::resolve(&design());
    // "A" is selected: its outline, drawn last, owns the line the cells share.
    let border = styles
        .button
        .colors(Tone::DEFAULT)
        .border
        .selected
        .to_rgba8();
    let clear = styles.background.to_rgba8();
    // Half a pixel off the grid (a centered group): the cells snap to (11, 11)-(51, 31) and
    // (51, 11)-(91, 31). The outer corners used to lose their radius there.
    let group = rect(10.5, 10.5, 80.0, 20.0);
    let image = render(
        &gpu,
        Extent::new(100, 40),
        &pair(&styles, group, Vec2::new(-10.0, -10.0), 6.0),
    );
    for (x, y) in [(11, 11), (90, 11), (90, 30), (11, 30)] {
        assert_pixel(&image, x, y, clear);
    }
    // Where the two cells meet, the corners are square: the border reaches the edge.
    assert_pixel(&image, 51, 11, border);
    assert_pixel(&image, 51, 30, border);
    gpu.assert_no_validation_errors();
}

#[test]
fn every_tone_shows_its_border_colors_in_every_state() {
    let Some(gpu) = gpu_or_skip("every_tone_shows_its_border_colors_in_every_state") else {
        return;
    };
    // The dark theme draws buttons without borders: give them one.
    let mut design = design();
    design
        .set_value(
            Scope::Global,
            koebi_tokens::component::BUTTON_BORDER_WIDTH_DEFAULT,
            3.0,
        )
        .expect("inside the range");
    let styles = Styles::resolve(&design);
    let image = render(&gpu, SHEET, &button_sheet(&design));
    for variant in Variant::ALL {
        for state in ControlState::ALL {
            let cell = state_cell(variant, state);
            let (x, y) = (px(cell.min.x + 1.0), px(cell.center().y));
            let expected = styles.button.colors(Tone::new(variant)).border.get(state);
            let shown = over(expected, styles.background).to_rgba8();
            let actual = image.pixel(x, y);
            let close = actual.iter().zip(shown).all(|(a, e)| a.abs_diff(e) <= 1);
            assert!(
                close,
                "{variant:?} {state:?}: pixel ({x}, {y}) is {actual:?}, expected {shown:?}"
            );
        }
    }
    gpu.assert_no_validation_errors();
}
