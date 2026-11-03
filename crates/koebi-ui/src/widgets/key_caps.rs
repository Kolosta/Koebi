//! Key caps: a shortcut shown as the keys to press, each in a cap that fits its label, joined by
//! `+` outside the caps, then what to do with the last one (`release`, `double click`,
//! `drag up`).
//!
//! Modifiers show as the platform names them: Command and the Windows key with their icon,
//! Super on Linux, Option on Apple keyboards; a side shows before the name (`L Ctrl`). Mouse
//! buttons show as a mouse with the button marked.

use core::fmt::Write as _;

use koebi_core::input::{Key, KeyName, ModifierKey, MouseButton};
use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_input::{
    ButtonTrigger, DragDirection, KeyTrigger, ModifierState, PinchDirection, Shortcut, Side,
    Trigger, WheelDirection,
};
use koebi_render::RectPrimitive;
use koebi_tokens::component;

use crate::icon::Icon;
use crate::text::{Align, FontStyle};
use crate::ui::Ui;

/// Colors and metrics of key caps, from the `component.key-cap.*` tokens.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyCapStyle {
    /// Background of a cap.
    pub background: Color,
    /// Border of a cap.
    pub border: Color,
    /// Border width.
    pub border_width: f32,
    /// Label and icon color.
    pub label: Color,
    /// Color of the `+` between caps and of what follows them.
    pub separator: Color,
    /// Corner radius.
    pub corner_radius: f32,
    /// Space between a cap's sides and its label.
    pub padding: f32,
    /// Space above and below a cap in its row.
    pub inset: f32,
    /// Space around the `+` between caps.
    pub gap: f32,
    /// Side of the icons in caps.
    pub icon_size: f32,
    /// Stroke width of the icons.
    pub stroke_width: f32,
    /// Font of the labels: the label font, scaled.
    pub font: FontStyle,
}

impl KeyCapStyle {
    /// Reads the style from `tokens`, the labels in `label` scaled by `key-cap.font-scale`.
    pub fn resolve(tokens: &impl Resolve, label: &FontStyle) -> Self {
        use component as c;
        let scale = tokens.get(c::KEY_CAP_FONT_SCALE_DEFAULT);
        Self {
            background: tokens.get(c::KEY_CAP_BACKGROUND_COLOR_DEFAULT),
            border: tokens.get(c::KEY_CAP_BORDER_COLOR_DEFAULT),
            border_width: tokens.get(c::KEY_CAP_BORDER_WIDTH_DEFAULT),
            label: tokens.get(c::KEY_CAP_LABEL_COLOR_DEFAULT),
            separator: tokens.get(c::KEY_CAP_SEPARATOR_COLOR_DEFAULT),
            corner_radius: tokens.get(c::KEY_CAP_CORNER_RADIUS_DEFAULT),
            padding: tokens.get(c::KEY_CAP_PADDING_HORIZONTAL_DEFAULT),
            inset: tokens.get(c::KEY_CAP_PADDING_VERTICAL_DEFAULT),
            gap: tokens.get(c::KEY_CAP_GAP_DEFAULT),
            icon_size: tokens.get(c::KEY_CAP_ICON_SIZE_DEFAULT),
            stroke_width: tokens.get(c::ICON_STROKE_WIDTH_CHEVRON),
            font: FontStyle {
                size: label.size * scale,
                ..label.clone()
            },
        }
    }
}

/// Whose keyboard keys are named for: Command and Option on Apple keyboards, the Windows key on
/// Windows, Super elsewhere. The platform's by default ([`Keyboard::CURRENT`]); tests and
/// previews may name another's ([`crate::UiState::set_keyboard`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keyboard {
    /// macOS and iOS.
    Apple,
    /// Windows.
    Windows,
    /// Linux, Android and the others.
    Other,
}

impl Default for Keyboard {
    /// The platform's.
    fn default() -> Self {
        Self::CURRENT
    }
}

impl Keyboard {
    /// The keyboard of the platform Koebi runs on.
    pub const CURRENT: Self = if cfg!(any(target_os = "macos", target_os = "ios")) {
        Self::Apple
    } else if cfg!(target_os = "windows") {
        Self::Windows
    } else {
        Self::Other
    };

    /// The names of `family`'s keys: either, the left one, the right one, either or none.
    const fn names(self, family: ModifierKey) -> [&'static str; 4] {
        match (family, self) {
            (ModifierKey::Ctrl, _) => ["Ctrl", "L Ctrl", "R Ctrl", "Any Ctrl"],
            (ModifierKey::Shift, _) => ["Shift", "L Shift", "R Shift", "Any Shift"],
            (ModifierKey::Alt, Self::Apple) => ["Option", "L Option", "R Option", "Any Option"],
            (ModifierKey::Alt, _) => ["Alt", "L Alt", "R Alt", "Any Alt"],
            (ModifierKey::Gui, Self::Apple) => ["Cmd", "L Cmd", "R Cmd", "Any Cmd"],
            (ModifierKey::Gui, Self::Windows) => ["Win", "L Win", "R Win", "Any Win"],
            (ModifierKey::Gui, Self::Other) => ["Super", "L Super", "R Super", "Any Super"],
        }
    }

    /// The name of `family`'s keys: `Ctrl`, `Option`, `Win`...
    pub const fn modifier_name(self, family: ModifierKey) -> &'static str {
        self.names(family)[0]
    }

    /// The icon of `family`'s keys, where they have one.
    const fn modifier_icon(self, family: ModifierKey) -> Option<Icon> {
        match (family, self) {
            (ModifierKey::Shift, _) => Some(Icon::Shift),
            (ModifierKey::Gui, Self::Apple) => Some(Icon::Command),
            (ModifierKey::Gui, Self::Windows) => Some(Icon::Windows),
            _ => None,
        }
    }
}

/// The punctuation keys and the sign they show as, by key name.
pub const KEY_SIGNS: [(&str, &str); 11] = [
    ("Comma", ","),
    ("Period", "."),
    ("Slash", "/"),
    ("Semicolon", ";"),
    ("Apostrophe", "'"),
    ("Minus", "-"),
    ("Equals", "="),
    ("Grave", "`"),
    ("LeftBracket", "["),
    ("RightBracket", "]"),
    ("Backslash", "\\"),
];

/// What a cap says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapLabel {
    /// A word or a sign.
    Text(&'static str),
    /// A character key.
    Char(char),
    /// A key known only by its platform code.
    Code(u32),
    /// A mouse button by number.
    Button(u8),
}

/// One cap: an icon, a label, or both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cap {
    /// Its icon, before the label.
    pub icon: Option<Icon>,
    /// Its label.
    pub label: Option<CapLabel>,
}

impl Cap {
    const fn text(text: &'static str) -> Self {
        Self {
            icon: None,
            label: Some(CapLabel::Text(text)),
        }
    }

    const fn icon(icon: Icon, text: Option<&'static str>) -> Self {
        Self {
            icon: Some(icon),
            label: match text {
                Some(text) => Some(CapLabel::Text(text)),
                None => None,
            },
        }
    }
}

/// The caps of a shortcut, and what follows them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShortcutCaps {
    caps: [Cap; Self::CAPACITY],
    len: usize,
    /// What to do with the last key: `release`, `double click`, `drag up`...; nothing for a
    /// press.
    pub phase: Option<&'static str>,
}

impl ShortcutCaps {
    /// Room for every modifier, a held key and the trigger.
    const CAPACITY: usize = 6;

    /// The caps of `shortcut`, as `keyboard` names its keys.
    pub fn of(shortcut: &Shortcut, keyboard: Keyboard) -> Self {
        let mut caps = Self::default();
        let rule = shortcut.modifiers;
        let families = ModifierKey::ALL.map(|family| (family, rule.get(family)));
        if families
            .iter()
            .all(|(_, state)| *state == ModifierState::Any)
        {
            caps.push(Cap::text("Any"));
        } else {
            for (family, state) in families {
                if let Some(cap) = modifier_cap(keyboard, family, state) {
                    caps.push(cap);
                }
            }
        }
        if let Some(key) = shortcut.held {
            caps.push(key_cap(key));
        }
        let (cap, phase) = trigger_cap(shortcut.trigger);
        caps.push(cap);
        caps.phase = phase;
        caps
    }

    fn push(&mut self, cap: Cap) {
        if let Some(slot) = self.caps.get_mut(self.len) {
            *slot = cap;
            self.len += 1;
        }
    }

    /// The caps, in order.
    pub fn caps(&self) -> &[Cap] {
        &self.caps[..self.len]
    }
}

/// The cap of a modifier family in `state`, as `keyboard` names it, if it shows one.
fn modifier_cap(keyboard: Keyboard, family: ModifierKey, state: ModifierState) -> Option<Cap> {
    let names = keyboard.names(family);
    let name = match state {
        ModifierState::Up => return None,
        ModifierState::Down(Side::Either) => names[0],
        ModifierState::Down(Side::Left) => names[1],
        ModifierState::Down(Side::Right) => names[2],
        ModifierState::Any => names[3],
    };
    Some(match keyboard.modifier_icon(family) {
        Some(icon) => Cap::icon(icon, Some(name)),
        None => Cap::text(name),
    })
}

/// The cap of a keyboard key.
pub fn key_cap(key: Key) -> Cap {
    let friendly = match key {
        Key::Tab => return Cap::icon(Icon::Tab, Some("Tab")),
        Key::LeftShift => return Cap::icon(Icon::Shift, Some("L Shift")),
        Key::RightShift => return Cap::icon(Icon::Shift, Some("R Shift")),
        Key::Escape => Some("Esc"),
        Key::PageUp => Some("Page Up"),
        Key::PageDown => Some("Page Down"),
        Key::LeftCtrl => Some("L Ctrl"),
        Key::RightCtrl => Some("R Ctrl"),
        Key::LeftAlt => Some("L Alt"),
        Key::RightAlt => Some("R Alt"),
        _ => None,
    };
    if let Some(text) = friendly {
        return Cap::text(text);
    }
    let label = match key.name() {
        KeyName::Static(name) => CapLabel::Text(
            KEY_SIGNS
                .iter()
                .find(|(key, _)| *key == name)
                .map_or(name, |(_, sign)| sign),
        ),
        KeyName::Character(character) => CapLabel::Char(character),
        KeyName::Code(code) => CapLabel::Code(code),
    };
    Cap {
        icon: None,
        label: Some(label),
    }
}

/// The cap of a trigger, and what follows it.
fn trigger_cap(trigger: Trigger) -> (Cap, Option<&'static str>) {
    match trigger {
        Trigger::Key { key, when, repeat } => (
            key_cap(key),
            match when {
                KeyTrigger::Press if repeat => Some("repeat"),
                KeyTrigger::Press => None,
                KeyTrigger::Release => Some("release"),
                KeyTrigger::Click => Some("click"),
                KeyTrigger::DoubleClick => Some("double click"),
            },
        ),
        Trigger::Button { button, when } => {
            let cap = match button {
                MouseButton::Left => Cap::icon(Icon::MouseLeft, None),
                MouseButton::Middle => Cap::icon(Icon::MouseMiddle, None),
                MouseButton::Right => Cap::icon(Icon::MouseRight, None),
                other => Cap {
                    icon: None,
                    label: Some(CapLabel::Button(other.number())),
                },
            };
            let phase = match when {
                ButtonTrigger::Press => None,
                ButtonTrigger::Release => Some("release"),
                ButtonTrigger::Click => Some("click"),
                ButtonTrigger::DoubleClick => Some("double click"),
                ButtonTrigger::Drag(drag) => Some(drag_phase(drag.direction)),
            };
            (cap, phase)
        }
        Trigger::Wheel(direction) => (
            Cap::text(match direction {
                WheelDirection::Up => "Wheel up",
                WheelDirection::Down => "Wheel down",
                WheelDirection::Left => "Wheel left",
                WheelDirection::Right => "Wheel right",
            }),
            None,
        ),
        Trigger::Pinch(direction) => (
            Cap::text(match direction {
                PinchDirection::In => "Pinch in",
                PinchDirection::Out => "Pinch out",
            }),
            None,
        ),
        Trigger::PointerMove => (Cap::text("Move"), None),
    }
}

/// What a drag in `direction` shows after its button.
const fn drag_phase(direction: DragDirection) -> &'static str {
    match direction {
        DragDirection::Any => "drag",
        DragDirection::North => "drag up",
        DragDirection::NorthEast => "drag up right",
        DragDirection::East => "drag right",
        DragDirection::SouthEast => "drag down right",
        DragDirection::South => "drag down",
        DragDirection::SouthWest => "drag down left",
        DragDirection::West => "drag left",
        DragDirection::NorthWest => "drag up left",
    }
}

/// The text of a cap's label, written in `buffer` when it is not a static word.
fn label_text(label: CapLabel, buffer: &mut LabelBuffer) -> &str {
    buffer.len = 0;
    match label {
        CapLabel::Text(text) => return text,
        CapLabel::Char(character) => {
            // Shortcuts show letters as their key, capitalized.
            for upper in character.to_uppercase() {
                let _infallible = buffer.write_char(upper);
            }
        }
        CapLabel::Code(code) => {
            let _infallible = write!(buffer, "#{code}");
        }
        CapLabel::Button(number) => {
            let _infallible = write!(buffer, "Mouse {number}");
        }
    }
    buffer.as_str()
}

/// A few bytes of text on the stack: a cap's label without allocating.
struct LabelBuffer {
    bytes: [u8; 16],
    len: usize,
}

impl LabelBuffer {
    const fn new() -> Self {
        Self {
            bytes: [0; 16],
            len: 0,
        }
    }

    fn as_str(&self) -> &str {
        // Only whole characters are written.
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

impl core::fmt::Write for LabelBuffer {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        let end = self.len + text.len();
        let slot = self.bytes.get_mut(self.len..end).ok_or(core::fmt::Error)?;
        slot.copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}

impl Ui<'_> {
    /// The width of `shortcut` as key caps ([`Ui::key_caps`]).
    pub fn key_caps_width(&mut self, shortcut: &Shortcut) -> f32 {
        let caps = ShortcutCaps::of(shortcut, self.state.keyboard());
        self.caps_layout(&caps, None)
    }

    /// Draws `shortcut` as key caps in `rect`, placed by `align`, the caps a little lower than
    /// the row. Returns the width they take.
    pub fn key_caps(&mut self, rect: Rect, shortcut: &Shortcut, align: Align) -> f32 {
        let caps = ShortcutCaps::of(shortcut, self.state.keyboard());
        let width = self.caps_layout(&caps, None);
        let x = match align {
            Align::Start => rect.min.x,
            Align::Center => rect.center().x - width * 0.5,
            Align::End => rect.max.x - width,
        }
        .round();
        self.caps_layout(&caps, Some((rect, x)));
        width
    }

    /// Measures the caps, and draws them from `x` in `rect` when given. Returns their width.
    fn caps_layout(&mut self, caps: &ShortcutCaps, draw: Option<(Rect, f32)>) -> f32 {
        let style = self.styles().key_caps.clone();
        let mut buffer = LabelBuffer::new();
        let mut x = draw.map_or(0.0, |(_, x)| x);
        let start = x;
        let plus = self.text_width("+", &style.font);
        for (index, cap) in caps.caps().iter().enumerate() {
            if index > 0 {
                if let Some((rect, _)) = draw {
                    let slot = Rect::from_min_max(
                        Vec2::new(x, rect.min.y),
                        Vec2::new(x + plus + 2.0 * style.gap, rect.max.y),
                    );
                    self.label("+", slot, &style.font, style.separator, Align::Center);
                }
                x += plus + 2.0 * style.gap;
            }
            let text = cap.label.map(|label| label_text(label, &mut buffer));
            let text_width = text.map_or(0.0, |text| self.text_width(text, &style.font));
            let icon_width = cap.icon.map_or(0.0, |_| style.icon_size);
            let between = if cap.icon.is_some() && text.is_some() {
                style.gap
            } else {
                0.0
            };
            let width = (2.0 * style.padding + icon_width + between + text_width).ceil();
            if let Some((rect, _)) = draw {
                let height = (rect.height() - 2.0 * style.inset).max(0.0);
                let bounds = Rect::from_origin_size(
                    Vec2::new(x, (rect.center().y - height * 0.5).round()),
                    Vec2::new(width, height.round()),
                );
                self.scene.push_rect(
                    &RectPrimitive::new(bounds, style.background)
                        .with_corner_radius(style.corner_radius)
                        .with_border(style.border_width, style.border),
                );
                let mut inner = bounds.min.x + style.padding;
                if let Some(icon) = cap.icon {
                    let slot = Rect::from_origin_size(
                        Vec2::new(inner, bounds.min.y),
                        Vec2::new(style.icon_size, bounds.height()),
                    );
                    icon.draw(
                        self.scene,
                        Icon::centered(slot, style.icon_size),
                        style.label,
                        style.stroke_width,
                    );
                    inner += style.icon_size + between;
                }
                if let Some(text) = text {
                    let slot = Rect::from_min_max(
                        Vec2::new(inner, bounds.min.y),
                        Vec2::new(bounds.max.x - style.padding, bounds.max.y),
                    );
                    self.label(text, slot, &style.font, style.label, Align::Start);
                }
            }
            x += width;
        }
        if let Some(phase) = caps.phase {
            x += style.gap * 2.0;
            let width = self.text_width(phase, &style.font);
            if let Some((rect, _)) = draw {
                let slot =
                    Rect::from_min_max(Vec2::new(x, rect.min.y), Vec2::new(x + width, rect.max.y));
                self.label(phase, slot, &style.font, style.separator, Align::Start);
            }
            x += width;
        }
        x - start
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::input::ModifierKey;
    use koebi_input::{Drag, ModifierRule};
    use koebi_render::Scene;
    use koebi_text::Fonts;

    use super::*;
    use crate::{Input, UiState};

    fn labels(shortcut: &Shortcut) -> Vec<String> {
        labels_on(shortcut, Keyboard::Windows)
    }

    fn labels_on(shortcut: &Shortcut, keyboard: Keyboard) -> Vec<String> {
        let caps = ShortcutCaps::of(shortcut, keyboard);
        let mut buffer = LabelBuffer::new();
        caps.caps()
            .iter()
            .map(|cap| {
                let icon = cap.icon.map_or(String::new(), |icon| format!("{icon:?} "));
                let text = cap.label.map_or("", |label| label_text(label, &mut buffer));
                format!("{icon}{text}").trim().to_owned()
            })
            .collect()
    }

    #[test]
    fn caps_show_modifiers_the_held_key_and_the_trigger_in_order() {
        let shortcut = Shortcut::key(Key::Character('g'))
            .with(ModifierKey::Ctrl)
            .with(ModifierKey::Shift)
            .holding(Key::Space);
        assert_eq!(labels(&shortcut), ["Ctrl", "Shift Shift", "Space", "G"]);
        assert_eq!(ShortcutCaps::of(&shortcut, Keyboard::Windows).phase, None);
        // Each keyboard names its keys.
        let gui = Shortcut::key(Key::Q)
            .with(ModifierKey::Gui)
            .with(ModifierKey::Alt);
        assert_eq!(
            labels_on(&gui, Keyboard::Apple),
            ["Option", "Command Cmd", "Q"]
        );
        assert_eq!(
            labels_on(&gui, Keyboard::Windows),
            ["Alt", "Windows Win", "Q"]
        );
        assert_eq!(labels_on(&gui, Keyboard::Other), ["Alt", "Super", "Q"]);
        assert_eq!(Keyboard::Apple.modifier_name(ModifierKey::Alt), "Option");
        // Sides, any modifiers, punctuation, codes.
        let left = Shortcut::key(Key::Comma)
            .modifiers(ModifierRule::NONE.with(ModifierKey::Ctrl, ModifierState::Down(Side::Left)));
        assert_eq!(labels(&left), ["L Ctrl", ","]);
        let any = Shortcut::key(Key::Escape).modifiers(ModifierRule::ANY);
        assert_eq!(labels(&any), ["Any", "Esc"]);
        assert_eq!(labels(&Shortcut::key(Key::Unknown(42))), ["#42"]);
        assert_eq!(labels(&Shortcut::key(Key::Tab)), ["Tab Tab"]);
    }

    #[test]
    fn caps_say_what_to_do_with_the_last_key() {
        let release = Shortcut::new(Trigger::Key {
            key: Key::F,
            when: KeyTrigger::Release,
            repeat: false,
        });
        assert_eq!(
            ShortcutCaps::of(&release, Keyboard::Other).phase,
            Some("release")
        );
        let drag = Shortcut::button(
            MouseButton::Middle,
            ButtonTrigger::Drag(Drag {
                direction: DragDirection::North,
                distance: None,
            }),
        );
        assert_eq!(labels(&drag), ["MouseMiddle"]);
        assert_eq!(
            ShortcutCaps::of(&drag, Keyboard::Other).phase,
            Some("drag up")
        );
        let back = Shortcut::button(MouseButton::Back, ButtonTrigger::DoubleClick);
        assert_eq!(labels(&back), ["Mouse 4"]);
        assert_eq!(
            ShortcutCaps::of(&back, Keyboard::Other).phase,
            Some("double click")
        );
        let wheel = Shortcut::new(Trigger::Wheel(WheelDirection::Down));
        assert_eq!(labels(&wheel), ["Wheel down"]);
    }

    #[test]
    fn caps_fit_their_labels_and_are_joined_by_plus_signs() {
        let styles = crate::test_styles();
        let (mut scene, mut state, input) = (Scene::new(), UiState::new(), Input::new());
        state.set_fonts(Fonts::with_embedded());
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        let one = ui.key_caps_width(&Shortcut::key(Key::Character('s')));
        let ctrl = ui.key_caps_width(&Shortcut::key(Key::Character('s')).with(ModifierKey::Ctrl));
        let wide = ui.key_caps_width(&Shortcut::key(Key::PageDown));
        // A wider label, a wider cap; a modifier adds its cap and a `+`.
        assert!(wide > one);
        assert!(ctrl > one + 2.0 * styles.key_caps.padding);
        let rect = Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::new(200.0, 20.0));
        let drawn = ui.key_caps(
            rect,
            &Shortcut::key(Key::Character('s')).with(ModifierKey::Ctrl),
            Align::End,
        );
        assert_eq!(drawn, ctrl);
        drop(ui);
        // Two caps; their labels and the `+` are glyphs.
        assert_eq!(scene.rect_count(), 2);
        assert!(scene.glyph_count() >= 4);
    }
}
