//! Color fields and pickers: what a drag in the area and on the strips gives, Escape bringing
//! the color back, the eyedropper picking from the screen, the field opening its picker below
//! it; and their golden images, every shape and every way of showing transparency.

mod common;

use common::render;
use koebi_core::input::CursorShape;
use koebi_core::input::{Key, Modifiers};
use koebi_core::{Color, Extent, Hsv, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_render::Scene;
use koebi_testkit::{Tolerance, assert_golden, gpu_or_skip};
use koebi_text::Fonts;
use koebi_tokens::KOEBI;
use koebi_ui::{
    Button, ColorField, ColorFieldState, ColorPicker, ColorPickerState, Input, MENU_CLOSE,
    OPERATION_CANCEL, OPERATION_CONFIRM, PickOutcome, PickerShape, Response, ScreenPixels, Styles,
    Transparency, Ui, UiState, WidgetId,
};

/// Anti-aliased edges may differ by a few units between drivers.
const CROSS_DRIVER: Tolerance = Tolerance::new(3, 8);

const PICKER: WidgetId = WidgetId::new("picker");

fn styles() -> Styles {
    Styles::resolve(&DesignSystem::new(&KOEBI).expect("Koebi's schema is valid"))
}

/// Frames of one picker, its color and state kept between them.
struct Harness {
    styles: Styles,
    state: UiState,
    input: Input,
    picker: ColorPickerState,
    color: Color,
    shape: PickerShape,
    /// What the button below the eyedropper's pointer got in the last frame.
    below: Response,
}

/// The window of the harnesses.
fn window() -> Rect {
    Rect::from_origin_size(Vec2::ZERO, Vec2::new(500.0, 400.0))
}

/// A button where the eyedropper picks, which must never react meanwhile.
const BELOW: Rect = Rect::from_min_max(Vec2::new(380.0, 290.0), Vec2::new(440.0, 314.0));

impl Harness {
    fn new(shape: PickerShape) -> Self {
        let mut state = UiState::new();
        state.set_fonts(Fonts::with_embedded());
        Self {
            styles: styles(),
            state,
            input: Input::new(),
            picker: ColorPickerState::new(),
            color: Hsv {
                h: 0.6,
                s: 0.5,
                v: 0.5,
            }
            .to_color(1.0),
            shape,
            below: Response::default(),
        }
    }

    /// The area of the picker, at the top left of its rectangle.
    fn area(&self) -> Rect {
        Rect::from_origin_size(
            Vec2::new(10.0, 10.0),
            Vec2::splat(self.styles.color_picker.area_size),
        )
    }

    /// One frame: the picker in a window of 500 by 400, a button at the bottom right, then the
    /// layer of a session picking on the screen, as the application draws it last. Returns
    /// whether the color changed; the button's response goes to `self.below`.
    fn frame(&mut self) -> (bool, Scene) {
        let mut scene = Scene::new();
        let changed = {
            let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
            let picker = ColorPicker::new(PICKER, &mut self.picker, &mut self.color)
                .shape(self.shape)
                .eyedropper(true);
            let size = picker.size(&ui);
            let changed = picker.show(&mut ui, Rect::from_origin_size(Vec2::new(10.0, 10.0), size));
            self.below = Button::new(WidgetId::new("below"))
                .label("Below")
                .show(&mut ui, BELOW);
            ui.screen_pick_layer(window());
            changed
        };
        self.input.end_frame();
        (changed, scene)
    }

    fn hsv(&self) -> Hsv {
        Hsv::from_color(self.color)
    }

    /// Clicks where the eyedropper's button lies: at the bottom of the column of swatches,
    /// right of the strips.
    fn click_eyedropper(&mut self) {
        let style = self.styles.color_picker;
        let row = self.styles.button.height;
        let area = self.area();
        let column = area.max.x + 3.0 * style.gap + 2.0 * style.strip_width;
        let button = Vec2::new(column + style.swatch_width * 0.5, area.max.y - row * 0.5);
        self.input.pointer_moved(button);
        self.frame();
        self.input.primary_button(true, button);
        self.frame();
        self.input.primary_button(false, button);
        self.frame();
    }

    /// Presses and releases the primary button at `at`. Returns whether the press changed the
    /// color.
    fn press_at(&mut self, at: Vec2) -> bool {
        self.input.pointer_moved(at);
        self.frame();
        self.input.primary_button(true, at);
        let (changed, _) = self.frame();
        self.input.primary_button(false, at);
        self.frame();
        changed
    }
}

/// 11 by 11 pixels read around (400, 300): orange, the pixel at the center red.
fn read_back() -> ScreenPixels {
    let mut pixels = vec![[230, 120, 20, 255]; 121];
    pixels[5 * 11 + 5] = [200, 0, 0, 255];
    ScreenPixels {
        origin: (395, 295),
        center: (400, 300),
        width: 11,
        height: 11,
        pixels,
    }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.02
}

#[test]
fn a_press_in_the_square_sets_saturation_and_value_and_a_drag_follows_hidden() {
    let mut harness = Harness::new(PickerShape::SquareSv);
    harness.frame();
    let area = harness.area();
    let at = area.min + area.size() * Vec2::new(0.25, 0.4);
    harness.input.pointer_moved(at);
    harness.frame();
    harness.input.primary_button(true, at);
    let (changed, _) = harness.frame();
    assert!(changed);
    let hsv = harness.hsv();
    assert!(close(hsv.s, 0.25) && close(hsv.v, 0.6), "{hsv:?}");
    assert!(close(hsv.h, 0.6));
    // The pointer hides while it drags: motions move the mark.
    assert!(harness.state.pointer_lock().is_some());
    harness
        .input
        .pointer_moved_by(area.size() * Vec2::new(0.5, 0.0));
    harness.frame();
    assert!(close(harness.hsv().s, 0.75), "{:?}", harness.hsv());
    // With Shift, a tenth as far.
    harness.input.set_modifiers(Modifiers::LEFT_SHIFT);
    harness
        .input
        .pointer_moved_by(area.size() * Vec2::new(0.0, 0.5));
    harness.frame();
    assert!(close(harness.hsv().v, 0.55), "{:?}", harness.hsv());
    harness.input.set_modifiers(Modifiers::NONE);
    // Past the corner: the real corner, white at full value.
    harness
        .input
        .pointer_moved_by(area.size() * Vec2::new(-3.0, -3.0));
    harness.frame();
    assert_eq!(harness.color.to_rgba8(), [255, 255, 255, 255]);
    // Released: the pointer comes back at the mark.
    harness.input.primary_button(false, at);
    harness.frame();
    harness.frame();
    assert!(harness.state.pointer_lock().is_none());
}

#[test]
fn the_operation_s_cancel_during_a_drag_brings_the_color_back() {
    let mut harness = Harness::new(PickerShape::SquareSv);
    let before = harness.color;
    harness.frame();
    let area = harness.area();
    let at = area.min + area.size() * Vec2::new(0.8, 0.2);
    harness.input.pointer_moved(at);
    harness.frame();
    harness.input.primary_button(true, at);
    harness.frame();
    assert_ne!(harness.color, before);
    // Escape, Delete or the secondary button: the keymap turns them into the cancel.
    harness.input.trigger(&OPERATION_CANCEL);
    let (changed, _) = harness.frame();
    assert!(changed);
    let back = harness.color.to_rgba8();
    assert_eq!(back, before.to_rgba8());
}

#[test]
fn strips_cancel_and_confirm_with_the_operation_s_keys() {
    let mut harness = Harness::new(PickerShape::SquareSv);
    harness.frame();
    let style = harness.styles.color_picker;
    let strip = Rect::from_origin_size(
        Vec2::new(harness.area().max.x + style.gap, harness.area().min.y),
        Vec2::new(style.strip_width, style.area_size),
    );
    let before = harness.hsv().h;
    // A drag of the hue strip, cancelled: the hue comes back.
    let at = Vec2::new(strip.center().x, strip.min.y + strip.height() * 0.25);
    harness.input.pointer_moved(at);
    harness.frame();
    harness.input.primary_button(true, at);
    harness.frame();
    assert!(close(harness.hsv().h, 0.25));
    harness.input.trigger(&OPERATION_CANCEL);
    harness.frame();
    assert!(close(harness.hsv().h, before), "{:?}", harness.hsv());
    harness.input.primary_button(false, at);
    harness.frame();
    // Another drag, confirmed: the hue stays, and moving no longer changes it.
    harness.input.primary_button(true, at);
    harness.frame();
    harness.input.trigger(&OPERATION_CONFIRM);
    harness.frame();
    harness.input.pointer_moved_by(Vec2::new(0.0, 40.0));
    harness.frame();
    assert!(close(harness.hsv().h, 0.25), "{:?}", harness.hsv());
    harness.input.primary_button(false, at);
    harness.frame();
}

#[test]
fn the_hue_strip_and_the_wheels_set_their_components() {
    // The hue strip of the square, at the right of the area.
    let mut harness = Harness::new(PickerShape::SquareSv);
    harness.frame();
    let style = harness.styles.color_picker;
    let strip = Rect::from_origin_size(
        Vec2::new(harness.area().max.x + style.gap, harness.area().min.y),
        Vec2::new(style.strip_width, style.area_size),
    );
    let at = Vec2::new(strip.center().x, strip.min.y + strip.height() * 0.25);
    harness.input.pointer_moved(at);
    harness.frame();
    harness.input.primary_button(true, at);
    harness.frame();
    assert!(close(harness.hsv().h, 0.25), "{:?}", harness.hsv());
    // The HSV wheel: at the right edge, a quarter turn, full saturation.
    let mut wheel = Harness::new(PickerShape::WheelHsv);
    wheel.frame();
    let area = wheel.area();
    let at = Vec2::new(area.max.x - 1.0, area.center().y);
    wheel.input.pointer_moved(at);
    wheel.frame();
    wheel.input.primary_button(true, at);
    wheel.frame();
    let hsv = wheel.hsv();
    assert!(close(hsv.h, 0.25) && hsv.s > 0.95, "{hsv:?}");
}

#[test]
fn the_eyedropper_picks_the_pixel_at_the_center_and_nothing_below_reacts() {
    let mut harness = Harness::new(PickerShape::SquareSv);
    harness.frame();
    harness.click_eyedropper();
    // It asks for the screen around the pointer, which rests over the button: not hovered.
    let at = Vec2::new(400.0, 300.0);
    harness.input.pointer_moved(at);
    harness.frame();
    harness.frame();
    assert_eq!(harness.state.take_pixels_wanted(), Some(at));
    assert!(!harness.below.hovered);
    assert_eq!(harness.state.cursor(), CursorShape::Crosshair);
    // The pixels come back: a press picks the one at the center, and neither it nor its
    // release reaches the button.
    harness.state.set_screen_pixels(read_back());
    harness.frame();
    harness.input.primary_button(true, at);
    harness.frame();
    assert!(!harness.below.pressed);
    let (changed, _) = harness.frame();
    assert!(changed);
    assert_eq!(harness.color.to_rgba8()[..3], [200, 0, 0]);
    harness.input.primary_button(false, at);
    harness.frame();
    assert!(!harness.below.clicked && !harness.below.hovered);
    // Picked: it stops asking, and the button answers again.
    harness.frame();
    harness.state.take_pixels_wanted();
    harness.frame();
    assert_eq!(harness.state.take_pixels_wanted(), None);
    harness.frame();
    assert!(harness.below.hovered);
}

#[test]
fn the_operation_s_keys_give_up_or_pick_under_the_pointer() {
    let mut harness = Harness::new(PickerShape::SquareSv);
    let before = harness.color;
    harness.frame();
    harness.click_eyedropper();
    let at = Vec2::new(400.0, 300.0);
    harness.input.pointer_moved(at);
    harness.frame();
    harness.state.set_screen_pixels(read_back());
    // The cancel (Escape, Delete or the secondary button, through the keymap): nothing picked.
    harness.input.trigger(&OPERATION_CANCEL);
    harness.frame();
    harness.frame();
    assert_eq!(harness.color, before);
    assert!(!harness.state.picks_here());
    // Again, then the confirm (Enter): the pixel under the pointer.
    harness.click_eyedropper();
    harness.input.pointer_moved(at);
    harness.frame();
    harness.state.set_screen_pixels(read_back());
    harness.input.trigger(&OPERATION_CONFIRM);
    harness.frame();
    harness.frame();
    assert_eq!(harness.color.to_rgba8()[..3], [200, 0, 0]);
}

#[test]
fn a_press_ends_the_eyedropper_even_before_pixels_come_back() {
    let mut harness = Harness::new(PickerShape::SquareSv);
    let before = harness.color;
    harness.frame();
    // Pixels of another time, read before it starts, are forgotten when it does.
    harness.state.set_screen_pixels(read_back());
    harness.click_eyedropper();
    assert!(!harness.press_at(Vec2::new(400.0, 300.0)));
    harness.frame();
    assert_eq!(harness.color, before);
    // It no longer picks: it stops asking for the screen.
    harness.state.take_pixels_wanted();
    harness.input.pointer_moved(Vec2::new(410.0, 310.0));
    harness.frame();
    assert_eq!(harness.state.take_pixels_wanted(), None);
}

#[test]
fn the_eyedropper_hides_when_frames_cannot_be_read() {
    let mut harness = Harness::new(PickerShape::SquareSv);
    harness.state.set_screen_readable(false);
    harness.frame();
    harness.click_eyedropper();
    harness.input.pointer_moved(Vec2::new(400.0, 300.0));
    harness.frame();
    assert_eq!(harness.state.take_pixels_wanted(), None);
}

#[test]
fn another_window_picks_for_the_one_that_started_and_its_title_bar_too() {
    let mut owner = Harness::new(PickerShape::SquareSv);
    owner.frame();
    owner.click_eyedropper();
    assert!(owner.state.picks_here());
    // The other window learns of the session, as the application tells it.
    let styles = owner.styles.clone();
    let mut other = UiState::new();
    let mut input = Input::new();
    assert!(other.set_pick_elsewhere(true));
    let frame = |other: &mut UiState, input: &mut Input| {
        let mut scene = Scene::new();
        let pressed = {
            let mut ui = Ui::new(&mut scene, &styles, input, other);
            // Its title bar, which must not take the press.
            let bar = Rect::from_origin_size(Vec2::ZERO, Vec2::new(500.0, 30.0));
            let response = ui.interact(WidgetId::new("title-bar"), bar);
            ui.screen_pick_layer(window());
            response.pressed
        };
        input.end_frame();
        pressed
    };
    let at = Vec2::new(250.0, 15.0);
    input.pointer_moved(at);
    frame(&mut other, &mut input);
    assert_eq!(other.take_pixels_wanted(), Some(at));
    other.set_screen_pixels(ScreenPixels {
        origin: (245, 10),
        center: (250, 15),
        ..read_back()
    });
    frame(&mut other, &mut input);
    input.primary_button(true, at);
    assert!(
        !frame(&mut other, &mut input),
        "the title bar took the press"
    );
    // What it picked goes to the window that started the session.
    let picked = other.take_foreign_pick().expect("a pick");
    assert_eq!(picked, PickOutcome::Picked([200, 0, 0, 255]));
    owner.state.deliver_pick(picked);
    let (changed, _) = owner.frame();
    assert!(changed);
    assert_eq!(owner.color.to_rgba8()[..3], [200, 0, 0]);
    assert!(!owner.state.picks_here());
    assert!(other.set_pick_elsewhere(false));
}

#[test]
fn a_field_opens_its_picker_below_it_and_closes_it_again() {
    let styles = styles();
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let mut input = Input::new();
    let mut field = ColorFieldState::new();
    let mut color = Color::from_hex(0x33_66_CC);
    let rect = Rect::from_origin_size(Vec2::new(20.0, 20.0), Vec2::new(120.0, 24.0));
    let window = Rect::from_origin_size(Vec2::ZERO, Vec2::new(500.0, 400.0));
    let mut frame = |input: &mut Input, field: &mut ColorFieldState, color: &mut Color| {
        let mut scene = Scene::new();
        {
            let mut ui = Ui::new(&mut scene, &styles, input, &mut state);
            ColorField::new(WidgetId::new("field"), color, field).show(&mut ui, rect, window);
        }
        input.end_frame();
    };
    input.pointer_moved(rect.center());
    frame(&mut input, &mut field, &mut color);
    input.primary_button(true, rect.center());
    frame(&mut input, &mut field, &mut color);
    input.primary_button(false, rect.center());
    frame(&mut input, &mut field, &mut color);
    assert!(field.is_open());
    frame(&mut input, &mut field, &mut color);
    // Escape closes it, the color kept.
    input.key(Key::Escape, true);
    input.trigger(&MENU_CLOSE);
    frame(&mut input, &mut field, &mut color);
    assert!(!field.is_open());
    assert_eq!(color, Color::from_hex(0x33_66_CC));
}

#[test]
fn pickers_and_fields_match_golden() {
    let Some(gpu) = gpu_or_skip("pickers_and_fields_match_golden") else {
        return;
    };
    let styles = styles();
    let mut state = UiState::new();
    state.set_fonts(Fonts::with_embedded());
    let input = Input::new();
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    let color = Hsv {
        h: 0.58,
        s: 0.7,
        v: 0.8,
    }
    .to_color(0.6);
    {
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        // A field per way of showing transparency, then the default halves.
        let mut y = 8.0;
        for (index, transparency) in (0_u64..).zip(Transparency::ALL) {
            let mut field = ColorFieldState::new();
            let mut shown = color;
            let rect = Rect::from_origin_size(Vec2::new(8.0, y), Vec2::new(96.0, 20.0));
            ColorField::new(WidgetId::new("field").with(index), &mut shown, &mut field)
                .halves(transparency, transparency)
                .show(&mut ui, rect, Rect::ZERO);
            y += 26.0;
        }
        let mut field = ColorFieldState::new();
        let mut shown = color;
        ColorField::new(WidgetId::new("default"), &mut shown, &mut field).show(
            &mut ui,
            Rect::from_origin_size(Vec2::new(8.0, y), Vec2::new(96.0, 20.0)),
            Rect::ZERO,
        );
        // Every shape's picker.
        for (index, shape) in PickerShape::ALL.into_iter().enumerate() {
            let mut picker = ColorPickerState::new();
            let mut shown = color;
            let at = Vec2::new(
                120.0 + (index % 4) as f32 * 300.0,
                8.0 + (index / 4) as f32 * 345.0,
            );
            let id = WidgetId::new("shape").with(index as u64);
            let picker = ColorPicker::new(id, &mut picker, &mut shown)
                .shape(shape)
                .title(shape.label())
                .original(Color::from_hex(0x2E_8B_57));
            let size = picker.size(&ui);
            picker.show(&mut ui, Rect::from_origin_size(at, size));
        }
    }
    let image = render(&gpu, Extent::new(1330, 700), &scene);
    assert_golden!(image, "color_pickers_dark", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}

#[test]
fn the_round_magnifier_matches_golden() {
    let Some(gpu) = gpu_or_skip("the_round_magnifier_matches_golden") else {
        return;
    };
    let mut harness = Harness::new(PickerShape::SquareSv);
    harness.frame();
    harness.click_eyedropper();
    let at = Vec2::new(400.0, 300.0);
    harness.input.pointer_moved(at);
    harness.frame();
    // A checkered square, the picked pixel red at its center.
    let mut pixels = read_back();
    for (index, pixel) in pixels.pixels.iter_mut().enumerate() {
        if index != 5 * 11 + 5 {
            *pixel = if (index / 11 + index % 11) % 2 == 0 {
                [40, 110, 200, 255]
            } else {
                [235, 235, 235, 255]
            };
        }
    }
    harness.state.set_screen_pixels(pixels);
    let (_, mut scene) = harness.frame();
    scene.set_clear_color(harness.styles.background);
    let image = render(&gpu, Extent::new(500, 400), &scene);
    assert_golden!(&image, "eyedropper_magnifier_dark", CROSS_DRIVER);
    gpu.assert_no_validation_errors();
}
