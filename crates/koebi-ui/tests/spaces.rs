//! Components in zoomable spaces (ADR 0030): drawn as they would be at the matching scale
//! factor, hit where they show, dragged at the speed of the hand, typed into, clipped in their
//! own units, their menus open at the window's scale, in spaces zoomed in, zoomed out and
//! nested, with no code of their own.

mod common;

use common::render_scaled;
use koebi_core::{Extent, Rect, Space, Vec2};
use koebi_design::{DesignSystem, Scope};
use koebi_render::Scene;
use koebi_testkit::{Image, gpu_or_skip};
use koebi_text::Fonts;
use koebi_tokens::{KOEBI, semantic};
use koebi_ui::{
    Button, Checkbox, Dropdown, Input, Layer, MenuChoice, MenuItem, MenuState, NumberField,
    Response, Styles, TextField, Ui, UiState, WidgetId,
};

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Vec2::new(x, y), Vec2::new(width, height))
}

fn window() -> Rect {
    rect(0.0, 0.0, 800.0, 600.0)
}

/// The spaces the tests run in, an outer one and one inside it: zoomed in, zoomed out, and
/// a space zoomed out inside one zoomed in. None maps a point to itself.
fn spaces() -> [(Space, Space); 4] {
    [
        (Space::new(4.0, Vec2::new(30.0, 20.0)), Space::IDENTITY),
        (Space::new(0.5, Vec2::new(10.0, 10.0)), Space::IDENTITY),
        (Space::new(0.25, Vec2::new(200.0, 150.0)), Space::IDENTITY),
        (
            Space::new(3.0, Vec2::new(5.0, 5.0)),
            Space::new(0.25, Vec2::new(40.0, 8.0)),
        ),
    ]
}

/// Frames of an interface, its state and input kept between them.
struct Harness {
    state: UiState,
    input: Input,
    styles: Styles,
    scene: Scene,
}

impl Harness {
    fn new() -> Self {
        let design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
        let mut state = UiState::new();
        state.set_fonts(Fonts::with_embedded());
        state.set_window(window());
        Self {
            state,
            input: Input::new(),
            styles: Styles::resolve(&design),
            scene: Scene::new(),
        }
    }

    /// One frame running `show` inside `outer`, then `inner` inside it.
    fn frame<R>(
        &mut self,
        (outer, inner): (Space, Space),
        show: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> R {
        self.scene.clear();
        let result = {
            let mut ui = Ui::new(&mut self.scene, &self.styles, &self.input, &mut self.state);
            ui.space(outer, |ui| ui.space(inner, show))
        };
        self.input.end_frame();
        result
    }

    fn click(&mut self, at: Vec2) {
        self.input.pointer_moved(at);
        self.input.primary_button(true, at);
        self.input.primary_button(false, at);
    }
}

const BUTTON: WidgetId = WidgetId::new("button");

fn button() -> Rect {
    rect(20.0, 10.0, 60.0, 22.0)
}

fn show_button(ui: &mut Ui<'_>) -> Response {
    Button::new(BUTTON).label("Apply").show(ui, button())
}

#[track_caller]
fn assert_near(actual: Vec2, expected: Vec2) {
    assert!(
        (actual - expected).length() < 1e-3,
        "{actual:?} is not {expected:?}"
    );
}

#[test]
fn components_are_hit_where_they_show_in_every_space() {
    for pair in spaces() {
        let space = pair.0.then(pair.1);
        let mut harness = Harness::new();
        harness.frame(pair, show_button);
        let shown = space.map_point(button().center());
        // Where the button would be unscaled, nothing.
        harness.input.pointer_moved(button().center());
        assert!(!harness.frame(pair, show_button).hovered, "{space:?}");
        // Where it shows, the button, and the pointer in its units.
        harness.input.pointer_moved(shown);
        let (response, pointer) = harness.frame(pair, |ui| (show_button(ui), ui.pointer()));
        assert!(response.hovered, "{space:?}");
        assert_near(pointer.expect("over the window"), button().center());
        harness.click(shown);
        let (response, press) = harness.frame(pair, |ui| (show_button(ui), ui.press_position()));
        assert!(response.pressed && response.clicked, "{space:?}");
        assert_near(press.expect("a press"), button().center());
    }
}

#[test]
fn drags_move_at_the_speed_of_the_hand() {
    for pair in spaces() {
        let space = pair.0.then(pair.1);
        let mut harness = Harness::new();
        let grip = |ui: &mut Ui<'_>| ui.interact(BUTTON, button());
        harness.frame(pair, grip);
        let start = space.map_point(button().center());
        harness.input.pointer_moved(start);
        harness.input.primary_button(true, start);
        assert!(harness.frame(pair, grip).pressed);
        // The hand moves 12 pixels right and 6 up: what is dragged moves as far on screen.
        let hand = Vec2::new(12.0, -6.0);
        harness.input.pointer_moved(start + hand);
        let response = harness.frame(pair, grip);
        assert!(response.held);
        assert_near(response.drag_delta, hand / space.scale);
        // A locked pointer's motion too.
        harness.input.pointer_moved_by(hand);
        let delta = harness.frame(pair, |ui| ui.pointer_delta());
        assert_near(delta, hand / space.scale);
    }
}

#[test]
fn text_fields_in_a_space_take_typing_and_map_the_input_method() {
    let field = rect(10.0, 40.0, 160.0, 22.0);
    for pair in spaces() {
        let space = pair.0.then(pair.1);
        let mut harness = Harness::new();
        let mut text = String::from("old");
        harness.frame(pair, |ui| TextField::new(BUTTON, &mut text).show(ui, field));
        harness.click(space.map_point(field.center()));
        let started = harness.frame(pair, |ui| TextField::new(BUTTON, &mut text).show(ui, field));
        assert!(started.editing, "{space:?}");
        // The input method shows beside the field where it is on screen.
        let area = harness
            .state
            .text_input_area()
            .expect("text input asked for");
        let shown = space.map_rect(field);
        assert!(
            area.intersection(&shown) == Some(area),
            "{area:?} outside {shown:?}"
        );
        assert!(area.height() <= shown.height() + 1e-3 && area.height() > 0.5 * shown.height());
        // Everything was selected: typing replaces it.
        harness.input.typed("new");
        harness.frame(pair, |ui| TextField::new(BUTTON, &mut text).show(ui, field));
        assert_eq!(text, "new", "{space:?}");
    }
}

const DROPDOWN: WidgetId = WidgetId::new("dropdown");

const MODES: [MenuItem<'static>; 3] = [
    MenuItem::action("Solid"),
    MenuItem::action("Wireframe"),
    MenuItem::action("Rendered"),
];

fn control() -> Rect {
    rect(20.0, 20.0, 120.0, 22.0)
}

/// A frame of a dropdown in `control` in `pair`, its menu kept inside the window.
fn show_dropdown(
    harness: &mut Harness,
    pair: (Space, Space),
    control: Rect,
    menu: &mut MenuState,
) -> Option<MenuChoice> {
    harness.frame(pair, |ui| {
        Dropdown::new(DROPDOWN, menu, &MODES)
            .label("Solid")
            .show(ui, control, window())
    })
}

#[test]
fn menus_open_at_the_window_scale_beside_their_control() {
    let unscaled = (Space::IDENTITY, Space::IDENTITY);
    for pair in spaces() {
        let space = pair.0.then(pair.1);
        // The dropdown in the space, and the same one unscaled where the first one shows.
        let shown = space.map_rect(control());
        let (mut zoomed, mut zoomed_menu) = (Harness::new(), MenuState::new());
        let (mut plain, mut plain_menu) = (Harness::new(), MenuState::new());
        show_dropdown(&mut zoomed, pair, control(), &mut zoomed_menu);
        show_dropdown(&mut plain, unscaled, shown, &mut plain_menu);
        zoomed.click(shown.center());
        plain.click(shown.center());
        show_dropdown(&mut zoomed, pair, control(), &mut zoomed_menu);
        show_dropdown(&mut plain, unscaled, shown, &mut plain_menu);
        // A frame with the menus drawn.
        show_dropdown(&mut zoomed, pair, control(), &mut zoomed_menu);
        show_dropdown(&mut plain, unscaled, shown, &mut plain_menu);
        // Read in the dropdown's units, the menu lies on screen where the unscaled one does:
        // as large, below the control.
        let menu = space.map_rect(zoomed_menu.rect().expect("open"));
        let reference = plain_menu.rect().expect("open");
        assert_near(menu.min, reference.min);
        assert_near(menu.max, reference.max);
        assert!((menu.min.y - shown.max.y).abs() < 1.0, "{space:?}");
        // Its items are where they show: a click on the middle one chooses it.
        let middle = menu.center();
        zoomed.click(middle);
        let choice = show_dropdown(&mut zoomed, pair, control(), &mut zoomed_menu);
        assert_eq!(choice.map(|choice| choice.index()), Some(1), "{space:?}");
        assert!(!zoomed_menu.is_open());
    }
}

#[test]
fn floating_layers_draw_at_the_window_scale() {
    for pair in spaces() {
        let space = pair.0.then(pair.1);
        let mut harness = Harness::new();
        let (inside, floating, overlay, base, after) = harness.frame(pair, |ui| {
            let inside = ui.window_space();
            let floating = ui.on_layer(Layer::FLOATING, |ui| ui.window_space());
            let overlay = ui.on_layer(Layer::OVERLAY, |ui| ui.window_space());
            // A layer below the floating ones keeps the space.
            let base = ui.on_layer(Layer(5), |ui| ui.window_space());
            (inside, floating, overlay, base, ui.window_space())
        });
        assert_eq!(inside, space);
        assert_eq!(floating, Space::IDENTITY);
        assert_eq!(overlay, Space::IDENTITY);
        assert_eq!(base, space);
        assert_eq!(after, space);
    }
}

#[test]
fn clips_and_hidden_parts_are_in_local_units() {
    for pair in spaces() {
        let space = pair.0.then(pair.1);
        let mut harness = Harness::new();
        let view = rect(0.0, 0.0, 100.0, 50.0);
        let (clip, below, inside) = harness.frame(pair, |ui| {
            ui.clipped(view, |ui| {
                (
                    ui.clip_rect(),
                    ui.is_hidden(rect(0.0, 60.0, 100.0, 20.0)),
                    ui.is_hidden(rect(0.0, 40.0, 100.0, 20.0)),
                )
            })
        });
        let clip = clip.expect("a clip");
        assert_near(clip.min, view.min);
        assert_near(clip.max, view.max);
        assert!(below && !inside, "{space:?}");
    }
}

#[test]
fn detail_drops_where_text_cannot_be_read_on_screen() {
    let mut harness = Harness::new();
    let readable = harness.styles.zoom.readable_size;
    let text = 14.0;
    let at =
        |harness: &mut Harness, pair| harness.frame(pair, |ui| (ui.scale(), ui.readable(text)));
    let unscaled = at(&mut harness, (Space::IDENTITY, Space::IDENTITY));
    assert_eq!(unscaled, (1.0, true));
    let limit = readable / text;
    let just = Space::new(limit * 1.01, Vec2::ZERO);
    assert!(at(&mut harness, (just, Space::IDENTITY)).1);
    let under = Space::new(limit * 0.99, Vec2::ZERO);
    assert!(!at(&mut harness, (under, Space::IDENTITY)).1);
    // Nested spaces multiply their scales.
    let (scale, read) = at(
        &mut harness,
        (
            Space::new(2.0, Vec2::ZERO),
            Space::new(0.4 * limit, Vec2::ZERO),
        ),
    );
    assert!((scale - 0.8 * limit).abs() < 1e-6 && !read);
}

#[test]
fn a_space_that_cannot_be_drawn_receives_nothing() {
    let collapsed = (Space::new(0.0, Vec2::new(30.0, 30.0)), Space::IDENTITY);
    let mut harness = Harness::new();
    harness.frame(collapsed, show_button);
    harness.click(Vec2::new(30.0, 30.0));
    let (response, pointer, press) = harness.frame(collapsed, |ui| {
        (show_button(ui), ui.pointer(), ui.press_position())
    });
    assert!(!response.hovered && !response.pressed);
    assert_eq!((pointer, press), (None, None));
    assert!(harness.scene.is_empty());
}

/// The size of the sheet of components, in logical pixels.
const SHEET: Vec2 = Vec2::new(160.0, 96.0);

/// One frame of a sheet of components with borders, labels, icons and fields, at the origin of
/// `space`.
fn sheet(space: Space) -> Scene {
    let mut design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
    // Outlined controls: the base theme draws no borders, which scaling must keep crisp.
    design
        .set_alias(
            Scope::Global,
            semantic::BORDER_WIDTH_CONTROL,
            semantic::BORDER_WIDTH_THIN,
        )
        .expect("a valid alias");
    let styles = Styles::resolve(&design);
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let input = Input::new();
    let (mut checked, mut value, mut text) = (true, 0.25_f32, String::from("Mix"));
    let mut menu = MenuState::new();
    {
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        ui.space(space, |ui| {
            Button::new(WidgetId::new("add"))
                .icon(koebi_icons::interface::PLUS)
                .label("Add")
                .show(ui, rect(8.0, 8.0, 72.0, 22.0));
            Checkbox::new(WidgetId::new("check"), &mut checked)
                .show(ui, rect(88.0, 11.0, 64.0, 16.0));
            NumberField::new(WidgetId::new("factor"), &mut value)
                .show(ui, rect(8.0, 38.0, 144.0, 22.0));
            TextField::new(WidgetId::new("name"), &mut text).show(ui, rect(8.0, 66.0, 70.0, 22.0));
            Dropdown::new(WidgetId::new("mode"), &mut menu, &MODES)
                .label("Solid")
                .show(ui, rect(82.0, 66.0, 70.0, 22.0), window());
        });
    }
    scene
}

/// `image` shrunk by `factor` on both sides, each pixel the mean of the square it covers.
fn shrunk(image: &Image, factor: u32) -> Image {
    let (width, height) = (image.width() / factor, image.height() / factor);
    let mut small = Image::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let mut sum = [0_u32; 4];
            for dy in 0..factor {
                for dx in 0..factor {
                    let pixel = image.pixel(x * factor + dx, y * factor + dy);
                    for (total, value) in sum.iter_mut().zip(pixel) {
                        *total += u32::from(value);
                    }
                }
            }
            let count = factor * factor;
            let mean = sum
                .map(|total| u8::try_from((total + count / 2) / count).expect("a mean of bytes"));
            small.set_pixel(x, y, mean);
        }
    }
    small
}

/// The mean difference between two images of the same size, over every channel of every
/// pixel, out of 255.
fn mean_difference(a: &Image, b: &Image) -> f64 {
    let total: u64 = a
        .as_rgba8()
        .iter()
        .zip(b.as_rgba8())
        .map(|(a, b)| u64::from(a.abs_diff(*b)))
        .sum();
    total as f64 / a.as_rgba8().len() as f64
}

#[test]
fn components_in_a_space_draw_as_they_would_at_its_scale() {
    let Some(gpu) = gpu_or_skip("components_in_a_space_draw_as_they_would_at_its_scale") else {
        return;
    };
    let extent = |scale: f32| {
        let side = |length: f32| {
            // Whole sheets: 160 and 96 times 4 or a quarter.
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "small positive whole numbers"
            )]
            let pixels = (length * scale) as u32;
            pixels
        };
        Extent::new(side(SHEET.x), side(SHEET.y))
    };
    let unscaled = sheet(Space::IDENTITY);

    // At 1, moved by whole pixels: the same pixels, moved. Glyphs evaluated at other positions
    // may round a channel differently (lavapipe does), within the tolerance of the zoomed case.
    let moved = Space::new(1.0, Vec2::new(16.0, 8.0));
    let reference = render_scaled(&gpu, extent(1.0), &unscaled, 1.0);
    let drawn = render_scaled(&gpu, Extent::new(176, 104), &sheet(moved), 1.0);
    let mut differing = 0;
    for y in 0..reference.height() {
        for x in 0..reference.width() {
            let (a, b) = (drawn.pixel(x + 16, y + 8), reference.pixel(x, y));
            differing += usize::from(a.iter().zip(b).any(|(a, b)| a.abs_diff(b) > 2));
        }
    }
    assert_eq!(differing, 0);

    // At 4: what the window would show at 4 physical pixels per logical pixel.
    let zoomed = render_scaled(&gpu, extent(4.0), &sheet(Space::new(4.0, Vec2::ZERO)), 1.0);
    let reference = render_scaled(&gpu, extent(4.0), &unscaled, 4.0);
    let diff = zoomed.diff(&reference, 2);
    assert_eq!(
        diff.differing_pixels, 0,
        "max delta {}",
        diff.max_channel_delta
    );

    // At a quarter: the unscaled sheet shrunk, but for edges on whole pixels and hairlines
    // faded where the shrunk sheet blurs them over their neighbors (about 5 out of 255 on
    // average). Components misplaced or of the wrong size would differ by far more.
    let small = render_scaled(
        &gpu,
        extent(0.25),
        &sheet(Space::new(0.25, Vec2::ZERO)),
        1.0,
    );
    let reference = shrunk(&render_scaled(&gpu, extent(1.0), &unscaled, 1.0), 4);
    let mean = mean_difference(&small, &reference);
    assert!(mean < 8.0, "mean difference {mean:.2} out of 255");
    gpu.assert_no_validation_errors();
}
