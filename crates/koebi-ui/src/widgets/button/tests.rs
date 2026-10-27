//! Tests of buttons and button groups: sizes, activations, repeats, toggles, tones, group
//! layout and borders.

use core::time::Duration;

use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve as _;
use koebi_render::Scene;
use koebi_text::Fonts;

use koebi_render::Corners;

use super::*;
use crate::style::CornerRadii;
use crate::widgets::parts::Hue;
use crate::{Input, Styles, UiState};

const SAVE: WidgetId = WidgetId::new("save");

fn rect() -> Rect {
    Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::new(80.0, 20.0))
}

/// A UI state with fonts, input and styles: one frame at a time.
struct Harness {
    state: UiState,
    input: Input,
    styles: Styles,
}

impl Harness {
    fn new() -> Self {
        let mut state = UiState::new();
        state.set_fonts(Fonts::with_embedded());
        Self {
            state,
            input: Input::new(),
            styles: crate::test_styles(),
        }
    }

    /// Runs one frame with `draw`, then ends the frame for the input.
    fn frame<R>(&mut self, draw: impl FnOnce(&mut Ui<'_>) -> R) -> (R, Scene) {
        let mut scene = Scene::new();
        let result = {
            let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
            draw(&mut ui)
        };
        self.input.end_frame();
        (result, scene)
    }

    /// A plain button over `rect()` for one frame.
    fn button(&mut self, configure: impl FnOnce(Button<'_>) -> Button<'_>) -> Response {
        self.frame(|ui| configure(Button::new(SAVE).label("Save")).show(ui, rect()))
            .0
    }

    fn click(&mut self, at: Vec2) {
        self.input.primary_button(true, at);
        self.input.primary_button(false, at);
    }
}

#[test]
fn buttons_fit_their_content_within_their_limits() {
    let mut harness = Harness::new();
    let style = harness.styles.button.clone();
    let ((label, fit, icon, fixed, capped), _) = harness.frame(|ui| {
        let label = ui.text_width("Save", &ui.styles().text.label);
        (
            label,
            Button::new(SAVE).label("Save").size(ui),
            Button::new(SAVE).icon(Icon::Plus).size(ui),
            Button::new(SAVE).label("Save").width(120.0).size(ui),
            Button::new(SAVE).label("Save").max_width(20.0).size(ui),
        )
    });
    assert_eq!(
        fit,
        Vec2::new((label + 2.0 * style.padding).ceil(), style.height)
    );
    // An icon alone makes a square one unit high (the paddings do not apply).
    assert_eq!(icon, Vec2::splat(style.height.max(style.min_width)));
    assert_eq!(fixed.x, 120.0);
    assert_eq!(capped.x, 20.0);
}

#[test]
fn the_action_fires_on_release_and_cancels_away() {
    let mut harness = Harness::new();
    harness.button(|b| b);
    harness.input.primary_button(true, Vec2::new(20.0, 20.0));
    let pressed = harness.button(|b| b);
    assert!(pressed.pressed && !pressed.clicked);
    harness.input.primary_button(false, Vec2::new(20.0, 20.0));
    assert!(harness.button(|b| b).clicked);
    // Moving away before releasing cancels.
    harness.input.primary_button(true, Vec2::new(20.0, 20.0));
    harness.button(|b| b);
    harness.input.pointer_moved(Vec2::new(300.0, 20.0));
    harness.button(|b| b);
    harness.input.primary_button(false, Vec2::new(300.0, 20.0));
    assert!(!harness.button(|b| b).clicked);
}

fn press(button: Button<'_>) -> Button<'_> {
    button.activation(Activation::Press)
}

fn double(button: Button<'_>) -> Button<'_> {
    button.activation(Activation::DoublePress)
}

fn repeat(button: Button<'_>) -> Button<'_> {
    button.repeat(true)
}

#[test]
fn activations_choose_press_or_double_press() {
    let mut harness = Harness::new();
    harness.button(press);
    harness.input.primary_button(true, Vec2::new(20.0, 20.0));
    assert!(harness.button(press).clicked);
    harness.input.primary_button(false, Vec2::new(20.0, 20.0));
    assert!(!harness.button(press).clicked);

    harness.input.set_time(Duration::from_secs(10));
    harness.click(Vec2::new(20.0, 20.0));
    assert!(!harness.button(double).clicked);
    let quick = harness.styles.input.double_click / 2;
    harness.input.set_time(Duration::from_secs(10) + quick);
    harness.click(Vec2::new(20.0, 20.0));
    assert!(harness.button(double).clicked);
    // A third click starts over.
    harness.input.set_time(Duration::from_secs(10) + quick * 2);
    harness.click(Vec2::new(20.0, 20.0));
    assert!(!harness.button(double).clicked);
}

#[test]
fn repeating_buttons_fire_on_press_then_while_held() {
    let mut harness = Harness::new();
    let input = harness.styles.input;
    harness.button(repeat);
    harness.input.primary_button(true, Vec2::new(20.0, 20.0));
    assert!(harness.button(repeat).clicked);
    // Held, before the delay: nothing, and the next frame is asked for at the delay.
    harness.input.set_time(input.repeat_delay / 2);
    assert!(!harness.button(repeat).clicked);
    assert_eq!(harness.state.redraw_at(), Some(input.repeat_delay));
    harness.input.set_time(input.repeat_delay);
    assert!(harness.button(repeat).clicked);
    harness
        .input
        .set_time(input.repeat_delay + input.repeat_interval / 2);
    assert!(!harness.button(repeat).clicked);
    harness
        .input
        .set_time(input.repeat_delay + input.repeat_interval);
    assert!(harness.button(repeat).clicked);
    harness.input.primary_button(false, Vec2::new(20.0, 20.0));
    assert!(!harness.button(repeat).clicked);
}

#[test]
fn toggles_flip_their_value_and_disabled_buttons_do_nothing() {
    let mut harness = Harness::new();
    let mut on = false;
    harness.frame(|ui| {
        Button::new(SAVE)
            .label("Grid")
            .toggle(&mut on)
            .show(ui, rect())
    });
    harness.click(Vec2::new(20.0, 20.0));
    let (response, _) = harness.frame(|ui| {
        Button::new(SAVE)
            .label("Grid")
            .toggle(&mut on)
            .show(ui, rect())
    });
    assert!(on && response.changed && response.clicked);
    harness.click(Vec2::new(20.0, 20.0));
    let (response, _) = harness.frame(|ui| {
        Button::new(SAVE)
            .label("Grid")
            .toggle(&mut on)
            .enabled(false)
            .show(ui, rect())
    });
    assert!(on && !response.changed && !response.clicked && !response.hovered);
}

#[test]
fn tones_pick_their_colors_and_calls_override_them() {
    let mut harness = Harness::new();
    let style = harness.styles.button.clone();
    let accent = style.colors(Tone::ACCENT);
    let blue = style.colors(Tone::ACCENT.with_hue(Hue::Blue));
    assert_ne!(accent, style.colors(Tone::DEFAULT));
    assert_ne!(accent, blue);
    // A family's default variant uses its subtle background and text color.
    let subtle = style.colors(Tone::DEFAULT.with_hue(Hue::Orange));
    assert_ne!(
        subtle.background.normal,
        style.colors(Tone::DEFAULT).background.normal
    );
    // Overrides take tokens or values, per state.
    let red = Color::from_hex(0xE0_30_30);
    let ((colors, radius), _) = harness.frame(|ui| {
        let button = Button::new(SAVE)
            .background(ControlState::Hovered, red)
            .content_color(
                ControlState::Normal,
                koebi_tokens::semantic::TEXT_COLOR_RED_DEFAULT,
            )
            .corner_radius(0.0);
        (button.colors(ui), button.radii.clone())
    });
    assert_eq!(colors.background.hovered, red);
    assert_eq!(
        colors.background.normal,
        style.colors(Tone::DEFAULT).background.normal
    );
    assert_eq!(
        colors.content.normal,
        harness
            .styles
            .tokens
            .get(koebi_tokens::semantic::TEXT_COLOR_RED_DEFAULT)
    );
    assert_eq!(radius, Some(CornerRadii::all(0.0)));
}

#[test]
fn the_last_radius_call_applies() {
    let corners = Corners {
        top_left: 1.0,
        top_right: 2.0,
        bottom_right: 3.0,
        bottom_left: 4.0,
    };
    let each_then_all = Button::new(SAVE).corner_radii(corners).corner_radius(6.0);
    assert_eq!(each_then_all.radii, Some(CornerRadii::all(6.0)));
    let all_then_each = Button::new(SAVE).corner_radius(6.0).corner_radii(corners);
    assert_eq!(all_then_each.radii, Some(CornerRadii::from(corners)));
}

#[test]
fn toggles_of_the_default_tone_take_the_look_of_toggles() {
    let mut harness = Harness::new();
    let style = harness.styles.button.clone();
    let mut on = false;
    let ((toggle, accent, push), _) = harness.frame(|ui| {
        (
            Button::new(SAVE).toggle(&mut on).colors(ui),
            Button::new(SAVE)
                .tone(Tone::ACCENT)
                .toggle(&mut false)
                .colors(ui),
            Button::new(SAVE).selected(true).colors(ui),
        )
    });
    assert_eq!(toggle, style.toggle);
    // Another tone keeps its colors; a selected push button keeps the button's.
    assert_eq!(accent, *style.colors(Tone::ACCENT));
    assert_eq!(push, *style.colors(Tone::DEFAULT));
}

#[test]
fn repeats_pause_while_the_pointer_is_off_the_button() {
    let mut harness = Harness::new();
    let input = harness.styles.input;
    harness.button(repeat);
    harness.input.primary_button(true, Vec2::new(20.0, 20.0));
    assert!(harness.button(repeat).clicked);
    // Held away from the button past the delay: no repeat (it used to keep firing).
    harness.input.pointer_moved(Vec2::new(300.0, 20.0));
    harness.input.set_time(input.repeat_delay * 2);
    assert!(!harness.button(repeat).clicked);
    // Back over it: the repeats resume at once.
    harness.input.pointer_moved(Vec2::new(20.0, 20.0));
    assert!(harness.button(repeat).clicked);
}

#[test]
fn double_presses_count_from_press_to_press_near_the_same_place() {
    let mut harness = Harness::new();
    let window = harness.styles.input.double_click;
    let at = Vec2::new(20.0, 20.0);
    harness.button(double);
    // A first click held almost the whole window, then a press just after it: the window
    // runs from the first press, so this is not a double press.
    harness.input.set_time(Duration::from_secs(5));
    harness.input.primary_button(true, at);
    harness.button(double);
    harness
        .input
        .set_time(Duration::from_secs(5) + window.saturating_sub(Duration::from_millis(5)));
    harness.input.primary_button(false, at);
    harness.button(double);
    harness
        .input
        .set_time(Duration::from_secs(5) + window + Duration::from_millis(5));
    harness.input.primary_button(true, at);
    assert!(!harness.button(double).clicked);
    harness.input.primary_button(false, at);
    harness.button(double);
    // A second press soon after, on the same place: it fires on the press itself.
    let quick = window / 4;
    harness
        .input
        .set_time(Duration::from_secs(5) + window + Duration::from_millis(5) + quick);
    harness.input.primary_button(true, at);
    let response = harness.button(double);
    assert!(response.clicked && response.pressed);
    harness.input.primary_button(false, at);
    harness.button(double);
    // Two quick presses far apart on the button are not a double press.
    harness.input.set_time(Duration::from_secs(20));
    harness.click(Vec2::new(12.0, 20.0));
    harness.button(double);
    harness.input.set_time(Duration::from_secs(20) + quick);
    harness.click(Vec2::new(85.0, 20.0));
    assert!(!harness.button(double).clicked);
}

/// The modifier group of a shortcut editor: Ctrl across the top, L and R below.
fn modifier_cells<'a>(left: &'a mut bool, right: &'a mut bool) -> [GroupCell<'a>; 3] {
    [
        GroupCell::new(Button::new(WidgetId::new("ctrl")).label("Ctrl"))
            .at(0, 0)
            .span(2, 1),
        GroupCell::new(Button::new(WidgetId::new("left")).label("L").toggle(left)).at(0, 1),
        GroupCell::new(Button::new(WidgetId::new("right")).label("R").toggle(right)).at(1, 1),
    ]
}

#[test]
fn groups_lay_cells_on_a_grid_with_spans() {
    let mut harness = Harness::new();
    let style = harness.styles.button.clone();
    let (mut left, mut right) = (false, false);
    let ((size, ctrl), _) = harness.frame(|ui| {
        let cells = modifier_cells(&mut left, &mut right);
        let group = ButtonGroup::horizontal();
        (
            group.size(ui, &cells),
            Button::new(WidgetId::new("ctrl")).label("Ctrl").size(ui),
        )
    });
    // Two rows one unit high; the columns at least as wide as L and R, and together as wide
    // as Ctrl.
    assert_eq!(size.y, 2.0 * style.height);
    assert!(
        size.x >= ctrl.x && size.x >= 2.0 * style.min_width,
        "{size:?}"
    );
    // A click on R toggles it, and only it.
    let at = Rect::from_origin_size(Vec2::ZERO, Vec2::new(100.0, 40.0));
    harness.frame(|ui| {
        ButtonGroup::horizontal().show(ui, at, &mut modifier_cells(&mut left, &mut right))
    });
    harness.click(Vec2::new(75.0, 30.0));
    let (response, _) = harness.frame(|ui| {
        ButtonGroup::horizontal().show(ui, at, &mut modifier_cells(&mut left, &mut right))
    });
    assert_eq!(response.clicked, Some(2));
    assert!(response.changed && right && !left);
}

#[test]
fn group_tracks_fit_share_or_stay_fixed() {
    let mut harness = Harness::new();
    let mut cells = [
        GroupCell::new(Button::new(WidgetId::new("a")).label("A")),
        GroupCell::new(Button::new(WidgetId::new("b")).label("B")),
        GroupCell::new(Button::new(WidgetId::new("c")).label("C")),
    ];
    let tracks = [Track::Fixed(30.0), Track::Share(1.0), Track::Share(3.0)];
    let at = Rect::from_origin_size(Vec2::ZERO, Vec2::new(150.0, 20.0));
    harness.frame(|ui| {
        ButtonGroup::horizontal()
            .columns(&tracks)
            .show(ui, at, &mut cells)
    });
    // The shares split the 120 pixels left: 30 and 90. A click at 50 is on B, at 100 on C.
    for (x, expected) in [(15.0, 0), (50.0, 1), (100.0, 2)] {
        harness.click(Vec2::new(x, 10.0));
        let (response, _) = harness.frame(|ui| {
            ButtonGroup::horizontal()
                .columns(&tracks)
                .show(ui, at, &mut cells)
        });
        assert_eq!(response.clicked, Some(expected), "at {x}");
    }
}

#[test]
fn groups_draw_a_fill_and_an_outline_per_cell() {
    let mut harness = Harness::new();
    let mut cells = [
        GroupCell::new(Button::new(WidgetId::new("a")).label("A")),
        GroupCell::new(Button::new(WidgetId::new("b")).label("B")),
    ];
    let at = Rect::from_origin_size(Vec2::ZERO, Vec2::new(80.0, 40.0));
    let (_, scene) = harness.frame(|ui| ButtonGroup::vertical().show(ui, at, &mut cells));
    assert_eq!(scene.rect_count(), 4);
}

#[test]
fn groups_shrink_into_rectangles_smaller_than_their_size() {
    let mut harness = Harness::new();
    let mut cells = [
        GroupCell::new(Button::new(WidgetId::new("a")).label("A")),
        GroupCell::new(Button::new(WidgetId::new("b")).label("B")),
    ];
    // Two rows of one unit in 20 pixels: each row gets 10, and a click 15 pixels down lands
    // on the second cell, inside the rectangle (the cells used to overflow it).
    let at = Rect::from_origin_size(Vec2::ZERO, Vec2::new(80.0, 20.0));
    harness.frame(|ui| ButtonGroup::vertical().show(ui, at, &mut cells));
    harness.click(Vec2::new(40.0, 15.0));
    let (response, _) = harness.frame(|ui| ButtonGroup::vertical().show(ui, at, &mut cells));
    assert_eq!(response.clicked, Some(1));
    // Below the rectangle, nothing.
    harness.click(Vec2::new(40.0, 25.0));
    let (response, _) = harness.frame(|ui| ButtonGroup::vertical().show(ui, at, &mut cells));
    assert_eq!(response.clicked, None);
}

#[test]
fn menu_buttons_open_their_menu_and_square_the_corners_it_meets() {
    use crate::widgets::menu::MenuState;
    use crate::widgets::parts::Placement;

    let mut harness = Harness::new();
    let mut menu = MenuState::new();
    harness.frame(|ui| {
        Button::new(SAVE)
            .label("Render")
            .menu(&mut menu, Placement::BELOW)
            .show(ui, rect())
    });
    harness.click(Vec2::new(20.0, 20.0));
    let (response, _) = harness.frame(|ui| {
        Button::new(SAVE)
            .label("Render")
            .menu(&mut menu, Placement::BELOW)
            .show(ui, rect())
    });
    assert!(response.clicked && menu.is_open());
    // Shown with its menu: the menu opens below, and the button squares its bottom corners.
    let window = Rect::from_origin_size(Vec2::ZERO, Vec2::new(300.0, 200.0));
    let (radii, _) = harness.frame(|ui| {
        let button = Button::new(SAVE)
            .label("Render")
            .menu(&mut menu, Placement::BELOW);
        assert!(button.is_selected());
        let (_, state) = button.show_with_menu(ui, rect());
        let state = state.expect("a menu button");
        ui.menu(
            WidgetId::new("menu"),
            state,
            &[crate::widgets::menu::MenuItem::action("Image")],
            window,
        );
        Button::new(SAVE)
            .menu(state, Placement::BELOW)
            .junction(rect(), Corners::all(4.0))
    });
    assert_eq!((radii.bottom_left, radii.bottom_right), (0.0, 0.0));
    assert_eq!((radii.top_left, radii.top_right), (4.0, 4.0));
}

#[test]
fn a_menu_button_in_a_group_opens_its_menu() {
    use crate::widgets::menu::MenuState;
    use crate::widgets::parts::Placement;

    let mut harness = Harness::new();
    let mut menu = MenuState::new();
    let at = Rect::from_origin_size(Vec2::ZERO, Vec2::new(120.0, 20.0));
    let show = |harness: &mut Harness, menu: &mut MenuState| {
        let mut cells = [
            GroupCell::new(Button::new(WidgetId::new("play")).icon(Icon::ChevronRight)),
            GroupCell::new(
                Button::new(WidgetId::new("render"))
                    .label("Render")
                    .menu(menu, Placement::BELOW),
            ),
        ];
        harness
            .frame(|ui| ButtonGroup::horizontal().show(ui, at, &mut cells))
            .0
    };
    show(&mut harness, &mut menu);
    harness.click(Vec2::new(90.0, 10.0));
    let response = show(&mut harness, &mut menu);
    assert_eq!(response.clicked, Some(1));
    assert!(menu.is_open());
}

#[test]
fn the_frame_that_opens_a_menu_squares_the_side_it_opens_on() {
    // Regression test: drawn before its menu was placed, the button kept its round corners in
    // the frame that opened the menu.
    use crate::widgets::menu::MenuState;
    use crate::widgets::parts::Placement;

    let mut menu = MenuState::new();
    menu.open_for(rect(), Placement::RIGHT);
    assert_eq!(menu.placed(), None);
    let radii = Button::new(SAVE)
        .menu(&mut menu, Placement::RIGHT)
        .junction(rect(), Corners::all(4.0));
    assert_eq!((radii.top_right, radii.bottom_right), (0.0, 0.0));
    assert_eq!((radii.top_left, radii.bottom_left), (4.0, 4.0));
}

#[test]
fn toned_dropdowns_keep_the_metrics_of_dropdowns() {
    // Regression test: a tone turned a dropdown into a button, with a button's size.
    let mut harness = Harness::new();
    let dropdown = |tone| {
        Button::new(SAVE)
            .dropdown_look()
            .tone(tone)
            .label("Viewport")
            .trailing_icon(Icon::ChevronDown)
    };
    let ((plain, toned), _) = harness.frame(|ui| {
        let plain = dropdown(Tone::DEFAULT);
        let toned = dropdown(Tone::ACCENT);
        assert_eq!(toned.look(), Look::Dropdown);
        assert_eq!(plain.colors(ui), ui.styles().dropdown.colors);
        assert_eq!(toned.colors(ui), *ui.styles().button.colors(Tone::ACCENT));
        (plain.size(ui), toned.size(ui))
    });
    assert_eq!(plain, toned);
}
