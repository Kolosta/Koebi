//! Capture fields: a shortcut shown as key caps, and the field that records a new one.
//!
//! A click starts recording: the field takes the keyboard ([`CAPTURE`]), so that no shortcut
//! triggers anything meanwhile, and the next key pressed (with the modifiers held) or the next
//! mouse button pressed (with the modifiers and a key held) becomes the shortcut, as
//! [`Capture`] allows. Modifier keys alone wait for the key they modify; a key held as a
//! modifier with a key is set apart, since its own press would be recorded first. Escape
//! cancels; with a keyboard capture, so does a press outside the field.

use koebi_core::input::{Key, ModifierKey, Modifiers};
use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_i18n::tr;
use koebi_input::{
    ButtonTrigger, Context, ModifierRule, ModifierState, Shortcut, Side, Trigger, WheelDirection,
};
use koebi_render::RectPrimitive;
use koebi_tokens::component;

use crate::id::WidgetId;
use crate::input::Input;
use crate::text::Align;
use crate::ui::{Response, Ui};
use crate::widgets::key_caps::ShortcutCaps;
use crate::widgets::parts::{ControlState, States};

/// What a recording field holds: no shortcut applies while it records.
pub const CAPTURE: Context = Context::new("capture");

/// What a capture field records.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Capture {
    /// A key of the keyboard.
    Keyboard,
    /// A mouse button.
    Mouse,
    /// Either.
    #[default]
    Any,
}

impl Capture {
    const fn keys(self) -> bool {
        matches!(self, Self::Keyboard | Self::Any)
    }

    const fn buttons(self) -> bool {
        matches!(self, Self::Mouse | Self::Any)
    }
}

/// Colors and metrics of capture fields, from the `component.capture-field.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaptureFieldStyle {
    /// Background per state, as a field's; active while recording.
    pub background: States<Color>,
    /// Border per state; active while recording.
    pub border: States<Color>,
    /// Border width.
    pub border_width: f32,
    /// Color of the hint shown without a shortcut, and while recording.
    pub hint: Color,
    /// Corner radius.
    pub corner_radius: f32,
    /// Space between the sides and the caps.
    pub padding: f32,
    /// Smallest width the field asks for.
    pub min_width: f32,
}

impl CaptureFieldStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        let get = |token| tokens.get(token);
        let (normal, hovered, pressed) = (
            get(c::CAPTURE_FIELD_BACKGROUND_COLOR_DEFAULT),
            get(c::CAPTURE_FIELD_BACKGROUND_COLOR_HOVER),
            get(c::CAPTURE_FIELD_BACKGROUND_COLOR_PRESSED),
        );
        let border_hover = get(c::CAPTURE_FIELD_BORDER_COLOR_HOVER);
        Self {
            // A field is never selected; recording, it is active.
            background: States {
                normal,
                hovered,
                pressed,
                selected: normal,
                selected_hovered: hovered,
                selected_pressed: pressed,
                active: get(c::CAPTURE_FIELD_BACKGROUND_COLOR_RECORDING),
                disabled: get(c::CAPTURE_FIELD_BACKGROUND_COLOR_DISABLED),
                selected_disabled: get(c::CAPTURE_FIELD_BACKGROUND_COLOR_DISABLED),
            },
            border: States {
                hovered: border_hover,
                pressed: border_hover,
                selected_hovered: border_hover,
                selected_pressed: border_hover,
                active: get(c::CAPTURE_FIELD_BORDER_COLOR_RECORDING),
                disabled: get(c::CAPTURE_FIELD_BORDER_COLOR_DISABLED),
                selected_disabled: get(c::CAPTURE_FIELD_BORDER_COLOR_DISABLED),
                ..States::all(get(c::CAPTURE_FIELD_BORDER_COLOR_DEFAULT))
            },
            border_width: tokens.get(c::KEY_CAP_BORDER_WIDTH_DEFAULT),
            hint: tokens.get(c::CAPTURE_FIELD_LABEL_COLOR_HINT),
            corner_radius: tokens.get(c::CAPTURE_FIELD_CORNER_RADIUS_DEFAULT),
            padding: tokens.get(c::CAPTURE_FIELD_PADDING_HORIZONTAL_DEFAULT),
            min_width: tokens.get(c::CAPTURE_FIELD_SIZE_MIN_WIDTH),
        }
    }
}

/// Whether a capture field records, kept by the caller between frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CaptureState {
    recording: bool,
}

impl CaptureState {
    /// Not recording.
    pub const fn new() -> Self {
        Self { recording: false }
    }

    /// `true` while the field records.
    pub const fn is_recording(self) -> bool {
        self.recording
    }

    /// Starts recording, as a click on the field does.
    pub fn record(&mut self) {
        self.recording = true;
    }
}

/// What happened to a capture field this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CaptureResponse {
    /// The pointer's interaction with the field.
    pub response: Response,
    /// The shortcut recorded this frame.
    pub captured: Option<Shortcut>,
    /// Recording stopped without a shortcut.
    pub cancelled: bool,
}

/// A capture field, configured then shown with [`CaptureField::show`].
#[derive(Debug)]
pub struct CaptureField<'a> {
    id: WidgetId,
    state: &'a mut CaptureState,
    shortcut: Option<Shortcut>,
    capture: Capture,
    hint: &'a str,
    recording_hint: &'a str,
    enabled: bool,
}

impl<'a> CaptureField<'a> {
    /// A field showing `shortcut` (none: its hint), recording as `state` says.
    pub fn new(id: WidgetId, state: &'a mut CaptureState, shortcut: Option<Shortcut>) -> Self {
        Self {
            id,
            state,
            shortcut,
            capture: Capture::Any,
            hint: tr("None"),
            recording_hint: tr("Press a key"),
            enabled: true,
        }
    }

    /// What it records.
    #[must_use]
    pub fn capture(mut self, capture: Capture) -> Self {
        self.capture = capture;
        self
    }

    /// What it shows without a shortcut, and while it records.
    #[must_use]
    pub fn hints(mut self, empty: &'a str, recording: &'a str) -> Self {
        self.hint = empty;
        self.recording_hint = recording;
        self
    }

    /// A disabled field shows its shortcut and records nothing.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows the field in `rect`; reports what it recorded.
    pub fn show(self, ui: &mut Ui<'_>, rect: Rect) -> CaptureResponse {
        let response = ui.interact(self.id, rect);
        let mut out = CaptureResponse {
            response,
            ..CaptureResponse::default()
        };
        if !self.enabled {
            self.state.recording = false;
        } else if self.state.recording {
            // The press that started it was a click, released: what comes next is recorded.
            match recorded(ui.input(), self.capture) {
                Some(Recorded::Shortcut(shortcut)) => {
                    out.captured = Some(shortcut);
                    self.state.recording = false;
                }
                Some(Recorded::Cancel) => {
                    out.cancelled = true;
                    self.state.recording = false;
                }
                None => ui.grab(CAPTURE),
            }
        } else if response.clicked {
            self.state.recording = true;
            ui.grab(CAPTURE);
        }
        self.draw(ui, rect, response);
        out
    }

    fn draw(&self, ui: &mut Ui<'_>, rect: Rect, response: Response) {
        let style = ui.styles().capture_field;
        let recording = self.state.recording;
        let state = ControlState::of(&response, false, self.enabled).activated(recording);
        let (background, border) = (style.background.get(state), style.border.get(state));
        ui.scene().push_rect(
            &RectPrimitive::new(rect, background)
                .with_corner_radius(style.corner_radius)
                .with_border(style.border_width, border),
        );
        let inner = Rect::from_min_max(
            Vec2::new(rect.min.x + style.padding, rect.min.y),
            Vec2::new((rect.max.x - style.padding).max(rect.min.x), rect.max.y),
        );
        let font = ui.styles().key_caps.font.clone();
        let held = rule_of(ui.input().modifiers());
        ui.clipped(rect, |ui| match (recording, self.shortcut) {
            // Recording: the modifiers held, as they come and go, then what to press.
            (true, _) if held != ModifierRule::NONE => {
                let caps = ShortcutCaps::of_modifiers(held, ui.keyboard());
                let width = ui.caps_width(&caps);
                let gap = ui.styles().key_caps.gap * 2.0;
                let hint = ui.text_width("+ …", &font);
                let start = inner.center().x - (width + gap + hint) * 0.5;
                let at =
                    Rect::from_min_max(Vec2::new(start.max(inner.min.x), inner.min.y), inner.max);
                let shown = ui.draw_caps(at, &caps, Align::Start);
                let rest = Rect::from_min_max(
                    Vec2::new((at.min.x + shown + gap).min(inner.max.x), inner.min.y),
                    inner.max,
                );
                ui.label("+ …", rest, &font, style.hint, Align::Start);
            }
            (true, _) => {
                ui.label(self.recording_hint, inner, &font, style.hint, Align::Center);
            }
            (false, Some(shortcut)) => {
                ui.key_caps(inner, &shortcut, Align::Center);
            }
            (false, None) => {
                ui.label(self.hint, inner, &font, style.hint, Align::Center);
            }
        });
    }
}

/// What a recording field got from `input`.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Recorded {
    Shortcut(Shortcut),
    Cancel,
}

/// What `input` gives a field recording `capture`. Modifiers alone wait, shown as they are
/// pressed and released; the first other key, mouse button or wheel turn ends the recording
/// with the modifiers held then. Only Escape cancels: Delete, Backspace and the right button
/// are shortcuts like the others. A keyboard capture ignores the mouse, a mouse capture the
/// keys.
fn recorded(input: &Input, capture: Capture) -> Option<Recorded> {
    let modifiers = rule_of(input.modifiers());
    for key in input.keys_pressed() {
        if key == Key::Escape {
            return Some(Recorded::Cancel);
        }
        if key.is_modifier() || !capture.keys() {
            continue;
        }
        return Some(Recorded::Shortcut(Shortcut::key(key).modifiers(modifiers)));
    }
    if !capture.buttons() {
        return None;
    }
    let held = input.keys_held().filter(|key| !key.is_modifier()).last();
    let holding = |shortcut: Shortcut| match held {
        Some(held) => shortcut.holding(held),
        None => shortcut,
    };
    if let Some(button) = input.mouse_buttons_pressed().next() {
        let shortcut = Shortcut::button(button, ButtonTrigger::Press).modifiers(modifiers);
        return Some(Recorded::Shortcut(holding(shortcut)));
    }
    let wheel = input.wheel();
    let direction = if wheel.y > 0.0 {
        Some(WheelDirection::Up)
    } else if wheel.y < 0.0 {
        Some(WheelDirection::Down)
    } else if wheel.x > 0.0 {
        Some(WheelDirection::Right)
    } else if wheel.x < 0.0 {
        Some(WheelDirection::Left)
    } else {
        None
    };
    direction.map(|direction| {
        let shortcut = Shortcut::new(Trigger::Wheel(direction)).modifiers(modifiers);
        Recorded::Shortcut(holding(shortcut))
    })
}

/// The rule of the modifiers held: each family held, on either side, the others up.
fn rule_of(held: Modifiers) -> ModifierRule {
    ModifierKey::ALL
        .into_iter()
        .fold(ModifierRule::NONE, |rule, family| {
            if held.sides(family).any() {
                rule.with(family, ModifierState::Down(Side::Either))
            } else {
                rule
            }
        })
}

#[cfg(test)]
mod tests {
    use koebi_core::input::MouseButton;
    use koebi_input::KeyTrigger;
    use koebi_render::Scene;

    use super::*;
    use crate::UiState;

    const FIELD: WidgetId = WidgetId::new("capture");

    fn rect() -> Rect {
        Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::new(160.0, 20.0))
    }

    struct Harness {
        state: UiState,
        input: Input,
        capture: CaptureState,
    }

    impl Harness {
        fn new() -> Self {
            Self {
                state: UiState::new(),
                input: Input::new(),
                capture: CaptureState::new(),
            }
        }

        fn frame(&mut self, capture: Capture) -> CaptureResponse {
            let styles = crate::test_styles();
            let mut scene = Scene::new();
            let response = {
                let mut ui = Ui::new(&mut scene, &styles, &self.input, &mut self.state);
                CaptureField::new(FIELD, &mut self.capture, None)
                    .capture(capture)
                    .show(&mut ui, rect())
            };
            self.input.end_frame();
            response
        }

        /// Clicks the field: it records from the next frame.
        fn start(&mut self, capture: Capture) {
            let center = rect().center();
            self.input.pointer_moved(center);
            self.frame(capture);
            self.input.primary_button(true, center);
            self.frame(capture);
            self.input.primary_button(false, center);
            self.frame(capture);
            assert!(self.capture.is_recording());
        }
    }

    #[test]
    fn capture_fields_look_like_fields_and_are_active_while_recording() {
        let styles = crate::test_styles();
        let (capture, field) = (styles.capture_field, styles.text_field);
        // At rest and hovered as a field; recording as a field being edited.
        assert_eq!(capture.background.normal, field.background.normal);
        assert_eq!(capture.background.hovered, field.background.hovered);
        assert_eq!(capture.background.active, field.background.active);
        assert_eq!(capture.border.active, field.border.active);
        assert_ne!(capture.background.hovered, Color::BLACK);
    }

    #[test]
    fn a_click_records_the_next_key_with_its_modifiers() {
        let mut harness = Harness::new();
        harness.start(Capture::Keyboard);
        // While it records, the field takes the keyboard: no shortcut applies.
        let mut chain = Vec::new();
        harness.state.chain_at(Some(rect().center()), &mut chain);
        assert_eq!(chain, [CAPTURE]);
        // A modifier alone waits.
        harness.input.set_modifiers(Modifiers::LEFT_CTRL);
        harness.input.key(Key::LeftCtrl, true);
        assert_eq!(harness.frame(Capture::Keyboard).captured, None);
        harness.input.key(Key::G, true);
        let captured = harness.frame(Capture::Keyboard).captured;
        let expected = Shortcut::key(Key::G).with(ModifierKey::Ctrl);
        assert_eq!(captured, Some(expected));
        assert!(!harness.capture.is_recording());
        assert!(matches!(
            expected.trigger,
            Trigger::Key {
                when: KeyTrigger::Press,
                ..
            }
        ));
    }

    #[test]
    fn only_escape_cancels_delete_and_clicks_do_not() {
        let mut harness = Harness::new();
        harness.start(Capture::Keyboard);
        harness.input.key(Key::Escape, true);
        let response = harness.frame(Capture::Keyboard);
        assert!(response.cancelled && response.captured.is_none());
        assert!(!harness.capture.is_recording());
        harness.input.key(Key::Escape, false);
        // A click elsewhere is no key: a keyboard capture goes on.
        harness.start(Capture::Keyboard);
        harness.input.primary_button(true, Vec2::new(400.0, 400.0));
        harness.input.mouse_button(MouseButton::Left, true);
        let response = harness.frame(Capture::Keyboard);
        assert!(!response.cancelled && harness.capture.is_recording());
        // Delete is a shortcut like the others.
        harness.input.key(Key::Delete, true);
        assert_eq!(
            harness.frame(Capture::Keyboard).captured,
            Some(Shortcut::key(Key::Delete))
        );
    }

    #[test]
    fn modifiers_held_show_live_until_a_key_ends_the_recording() {
        let mut harness = Harness::new();
        harness.start(Capture::Any);
        let caps = |harness: &mut Harness| {
            let styles = crate::test_styles();
            let mut scene = Scene::new();
            let mut ui = Ui::new(&mut scene, &styles, &harness.input, &mut harness.state);
            CaptureField::new(FIELD, &mut harness.capture, None).show(&mut ui, rect());
            drop(ui);
            harness.input.end_frame();
            scene.rect_count()
        };
        let waiting = caps(&mut harness);
        // Ctrl and Shift down: their caps show, recording goes on.
        harness
            .input
            .set_modifiers(Modifiers::LEFT_CTRL.union(Modifiers::LEFT_SHIFT));
        harness.input.key(Key::LeftCtrl, true);
        harness.input.key(Key::LeftShift, true);
        let held = caps(&mut harness);
        assert!(held >= waiting + 2, "{held} rectangles, {waiting} before");
        assert!(harness.capture.is_recording());
        // Shift released: one cap less.
        harness.input.set_modifiers(Modifiers::LEFT_CTRL);
        harness.input.key(Key::LeftShift, false);
        assert_eq!(caps(&mut harness), held - 1);
        // A key ends it with the modifiers held then.
        harness.input.key(Key::K, true);
        assert_eq!(
            harness.frame(Capture::Any).captured,
            Some(Shortcut::key(Key::K).with(ModifierKey::Ctrl))
        );
    }

    #[test]
    fn a_mouse_capture_records_any_button_and_the_wheel() {
        let mut harness = Harness::new();
        harness.start(Capture::Mouse);
        // Keys are not for it.
        harness.input.key(Key::A, true);
        assert_eq!(harness.frame(Capture::Mouse).captured, None);
        harness.input.set_modifiers(Modifiers::LEFT_SHIFT);
        harness.input.mouse_button(MouseButton::Middle, true);
        let captured = harness.frame(Capture::Mouse).captured;
        assert_eq!(
            captured,
            Some(
                Shortcut::button(MouseButton::Middle, ButtonTrigger::Press)
                    .with(ModifierKey::Shift)
                    .holding(Key::A)
            )
        );
        harness.input.key(Key::A, false);
        harness.input.set_modifiers(Modifiers::NONE);
        // The right button, a side button and a numbered one are shortcuts too.
        for button in [
            MouseButton::Right,
            MouseButton::Forward,
            MouseButton::Other(7),
        ] {
            harness.start(Capture::Mouse);
            harness.input.mouse_button(button, true);
            assert_eq!(
                harness.frame(Capture::Mouse).captured,
                Some(Shortcut::button(button, ButtonTrigger::Press))
            );
        }
        // The wheel tilted left.
        harness.start(Capture::Mouse);
        harness.input.scrolled(Vec2::new(-1.0, 0.0));
        assert_eq!(
            harness.frame(Capture::Mouse).captured,
            Some(Shortcut::new(Trigger::Wheel(WheelDirection::Left)))
        );
    }

    #[test]
    fn a_disabled_field_records_nothing() {
        let mut harness = Harness::new();
        harness.capture.record();
        let styles = crate::test_styles();
        let mut scene = Scene::new();
        harness.input.key(Key::A, true);
        let response = {
            let mut ui = Ui::new(&mut scene, &styles, &harness.input, &mut harness.state);
            CaptureField::new(FIELD, &mut harness.capture, None)
                .enabled(false)
                .show(&mut ui, rect())
        };
        assert_eq!(response.captured, None);
        assert!(!harness.capture.is_recording());
    }
}
