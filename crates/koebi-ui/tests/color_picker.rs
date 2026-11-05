//! Color fields and pickers: what a drag in the area and on the strips gives, Escape bringing
//! the color back, the eyedropper picking from the screen, the field opening its picker below
//! it; and their golden images, every shape and every way of showing transparency.

mod common;

use common::render;
use koebi_core::input::{Key, Modifiers};
use koebi_core::{Color, Extent, Hsv, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_render::Scene;
use koebi_testkit::{Tolerance, assert_golden, gpu_or_skip};
use koebi_text::Fonts;
use koebi_tokens::KOEBI;
use koebi_ui::{
    ColorField, ColorFieldState, ColorPicker, ColorPickerState, Input, MENU_CLOSE,
    OPERATION_CANCEL, OPERATION_CONFIRM, PickerShape, ScreenPixels, Styles, Transparency, Ui,
    UiState, WidgetId,
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
}

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
        }
    }

    /// The area of the picker, at the top left of its rectangle.
    fn area(&self) -> Rect {
        Rect::from_origin_size(
            Vec2::new(10.0, 10.0),
            Vec2::splat(self.styles.color_picker.area_size),
        )
    }

    /// One frame: the picker in a window of 500 by 400. Returns whether the color changed.
    fn frame(&mut self) -> (bool, Scene) {
        let mut scene = Scene::new();
        let changed = {
            let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
            let window = Rect::from_origin_size(Vec2::ZERO, Vec2::new(500.0, 400.0));
            let picker = ColorPicker::new(PICKER, &mut self.picker, &mut self.color)
                .shape(self.shape)
                .eyedropper(window);
            let size = picker.size(&ui);
            picker.show(&mut ui, Rect::from_origin_size(Vec2::new(10.0, 10.0), size))
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

/// 11 by 11 pixels read around (400, 300): orange, the right column red.
fn read_back() -> ScreenPixels {
    let mut pixels = vec![[230, 120, 20, 255]; 121];
    for row in 0..11 {
        pixels[row * 11 + 10] = [200, 0, 0, 255];
    }
    ScreenPixels {
        origin: (395, 295),
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
fn the_eyedropper_picks_the_pixel_under_the_pointer() {
    let mut harness = Harness::new(PickerShape::SquareSv);
    harness.frame();
    harness.click_eyedropper();
    // It asks for the screen around the pointer.
    let at = Vec2::new(400.0, 300.0);
    harness.input.pointer_moved(at);
    harness.frame();
    assert_eq!(harness.state.take_pixels_wanted(), Some(at));
    // The application gives the pixels back: a press picks the one under the pointer.
    let orange = [230, 120, 20, 255];
    harness.state.set_screen_pixels(ScreenPixels {
        origin: (395, 295),
        width: 11,
        height: 11,
        pixels: vec![orange; 121],
    });
    harness.frame();
    harness.input.primary_button(true, at);
    let (changed, _) = harness.frame();
    assert!(changed);
    assert_eq!(harness.color.to_rgba8(), orange);
    // Picked: it stops asking.
    harness.input.primary_button(false, at);
    harness.frame();
    harness.state.take_pixels_wanted();
    harness.frame();
    assert_eq!(harness.state.take_pixels_wanted(), None);
}

#[test]
fn the_eyedropper_picks_the_nearest_pixel_read_when_the_pointer_outran_them() {
    let mut harness = Harness::new(PickerShape::SquareSv);
    harness.frame();
    harness.click_eyedropper();
    // The pixels come from a frame shown earlier, around (400, 300); the pointer has gone on
    // far to the right since: the nearest pixel read, in the right column, is picked.
    harness.state.set_screen_pixels(read_back());
    assert!(harness.press_at(Vec2::new(460.0, 300.0)));
    assert_eq!(harness.color.to_rgba8(), [200, 0, 0, 255]);
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
