//! One shortcut of an action in the shortcut editor: its header (on or off, the device, the
//! field recording it, removal) and its details (what it says of its conflicts, the key, the
//! event and what depends on it, the modifiers and their sides, a key held as a modifier).
//!
//! The rules of editing are pure functions ([`super::rules`]), tested on their own; the page
//! applies what the user changed to the keymap.

use koebi_core::input::{Key, ModifierKey};
use koebi_core::{Color, Rect, Vec2};
use koebi_design::{DesignSystem, Scope};
use koebi_input::{
    Binding, ButtonTrigger, ConflictKind, Drag, KeyTrigger, ModifierRule, ModifierState, Shortcut,
    Side, Trigger,
};
use koebi_tokens::{component, semantic};
use koebi_ui::{
    Align, BarMapping, Button, ButtonGroup, Capture, CaptureField, CaptureState, Checkbox,
    CollapsiblePanelState, Dropdown, GroupCell, Icon, MenuItem, MenuState, NumberField, Track, Ui,
    WidgetId,
};

use super::analysis::BindingReport;
use super::rules::{
    BUTTONS, DIRECTIONS, Device, Event, PINCHES, WHEEL, can_repeat, chosen_keys, gui_name, is_any,
    side_on, toggle_any, toggle_family, toggle_side,
};

/// What the user changed in a shortcut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Edit {
    /// It becomes this.
    Set(Binding),
    /// It goes.
    Remove,
}

/// What a shortcut of the editor keeps between frames: its panel, and what edits it.
#[derive(Debug)]
pub(super) struct BindingUi {
    pub(super) panel: CollapsiblePanelState,
    pub(super) editor: Editor,
}

impl BindingUi {
    pub(super) fn new() -> Self {
        Self {
            panel: CollapsiblePanelState::new(false),
            editor: Editor::new(),
        }
    }
}

/// What edits a shortcut: its fields recording, its menus.
#[derive(Debug)]
pub(super) struct Editor {
    capture: CaptureState,
    held: CaptureState,
    device: MenuState,
    input: MenuState,
    event: MenuState,
    direction: MenuState,
    /// Turned off by the editor for a conflict, not by the user: it comes back on once it is
    /// changed to a free shortcut.
    pub(super) auto_off: bool,
}

impl Editor {
    fn new() -> Self {
        Self {
            capture: CaptureState::new(),
            held: CaptureState::new(),
            device: MenuState::new(),
            input: MenuState::new(),
            event: MenuState::new(),
            direction: MenuState::new(),
            auto_off: false,
        }
    }
}

/// The colors of conflicts: soft for those that leave the shortcut on, hard for the others.
pub(super) fn conflict_color(design: &DesignSystem, kind: ConflictKind) -> Color {
    if kind.is_total() {
        design.get(component::SHORTCUT_CONFLICT_COLOR_HARD)
    } else {
        design.get(component::SHORTCUT_CONFLICT_COLOR_SOFT)
    }
}

/// The lists the details choose from, built once.
#[derive(Debug)]
pub(super) struct Choices {
    keys: Vec<MenuItem<'static>>,
    key_list: Vec<Key>,
}

impl Choices {
    pub(super) fn new() -> Self {
        let (key_list, keys) = chosen_keys()
            .map(|(key, name)| (key, MenuItem::action(name)))
            .unzip();
        Self { keys, key_list }
    }
}

/// The controls of a shortcut's header in `rect`: on or off, the device, the field recording
/// it, and the button removing it.
pub(super) fn header(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    binding: Binding,
    state: &mut Editor,
    bounds: Rect,
) -> Option<Edit> {
    let row = rect.height();
    let gap = ui.styles().collapsible_panel.header_gap;
    let mut edit = None;
    // On or off.
    let check = Rect::from_origin_size(rect.min, Vec2::new(row, row));
    let mut enabled = binding.enabled;
    if Checkbox::new(id.with(1), &mut enabled)
        .show(ui, check)
        .changed
    {
        edit = Some(Edit::Set(Binding { enabled, ..binding }));
    }
    // The device.
    let device = Device::of(binding.shortcut.trigger);
    let devices = Device::ALL.map(|device| MenuItem::action(device.label()));
    let width = 110.0_f32.min(rect.width() * 0.3);
    let at = Rect::from_origin_size(
        Vec2::new(check.max.x + gap, rect.min.y),
        Vec2::new(width, row),
    );
    if let Some(choice) = Dropdown::new(id.with(2), &mut state.device, &devices)
        .label(device.label())
        .show(ui, at, bounds)
        && let Some(&chosen) = Device::ALL.get(choice.index())
    {
        let trigger = chosen.trigger(binding.shortcut.trigger);
        edit = Some(Edit::Set(Binding {
            shortcut: Shortcut {
                trigger,
                ..binding.shortcut
            },
            ..binding
        }));
    }
    // The field recording it, then the removal.
    let remove = Rect::from_min_max(Vec2::new(rect.max.x - row, rect.min.y), rect.max);
    let field = Rect::from_min_max(
        Vec2::new(at.max.x + gap, rect.min.y),
        Vec2::new((remove.min.x - gap).max(at.max.x + gap), rect.max.y),
    );
    let capture = match device {
        Device::Mouse => Capture::Mouse,
        _ => Capture::Keyboard,
    };
    let recorded = CaptureField::new(id.with(3), &mut state.capture, Some(binding.shortcut))
        .capture(capture)
        .enabled(matches!(device, Device::Keyboard | Device::Mouse))
        .show(ui, field);
    if let Some(shortcut) = recorded.captured {
        edit = Some(Edit::Set(Binding {
            shortcut: Shortcut {
                // A held key set apart stays.
                held: binding.shortcut.held,
                ..shortcut
            },
            ..binding
        }));
    }
    if Button::new(id.with(4))
        .icon(Icon::Close)
        .show(ui, remove)
        .clicked
    {
        edit = Some(Edit::Remove);
    }
    ui.tooltip(id.with(4), "Remove this shortcut", remove);
    edit
}

/// The details of a shortcut in `rect`: its conflicts, its key, its event and what depends on
/// it, its modifiers. Returns the height used, and what the user changed in `edit`.
#[allow(clippy::too_many_arguments, reason = "the parts of one shortcut")]
pub(super) fn details(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    binding: Binding,
    report: Option<&BindingReport>,
    state: &mut Editor,
    choices: &Choices,
    design: &mut DesignSystem,
    bounds: Rect,
    edit: &mut Option<Edit>,
) -> f32 {
    let padding = ui.styles().panel.padding;
    let row = design.get(semantic::SIZE_CONTROL_HEIGHT);
    let gap = design.get(semantic::SPACING_EDITOR_INSET) * 0.5;
    let font = ui.styles().text.label.clone();
    let detail = ui.styles().text.detail.clone();
    let secondary = ui.styles().text.secondary;
    let (left, right) = (rect.min.x + padding, rect.max.x - padding);
    let label_width = 110.0_f32.min((right - left) * 0.35);
    let mut y = rect.min.y;
    let shortcut = binding.shortcut;
    let mut set = |shortcut: Shortcut| {
        *edit = Some(Edit::Set(Binding {
            shortcut,
            ..binding
        }));
    };

    // What its conflicts say.
    for (kind, text) in report.map_or(&[][..], |report| report.messages.as_slice()) {
        let color = conflict_color(design, *kind);
        let line = Rect::from_min_max(Vec2::new(left, y), Vec2::new(right, y + row));
        let icons = ui.styles().icon;
        let slot = Rect::from_origin_size(line.min, Vec2::new(icons.size, row));
        Icon::Warning.draw(
            ui.scene(),
            Icon::centered(slot, icons.size),
            color,
            icons.stroke_width,
        );
        let text_rect = Rect::from_min_max(Vec2::new(slot.max.x + gap, line.min.y), line.max);
        ui.label(text, text_rect, &detail, secondary, Align::Start);
        y += row;
    }
    if report.is_some_and(|report| !report.messages.is_empty()) {
        y += gap;
    }

    // A labeled row: its label, then the room of its control.
    let labeled = |ui: &mut Ui<'_>, y: f32, label: &str| {
        let title = Rect::from_origin_size(Vec2::new(left, y), Vec2::new(label_width, row));
        ui.label(label, title, &font, secondary, Align::Start);
        Rect::from_min_max(Vec2::new(left + label_width, y), Vec2::new(right, y + row))
    };

    // The key, the button, the direction.
    let device = Device::of(shortcut.trigger);
    let input = labeled(ui, y, input_title(device));
    let changed = input_choice(
        ui,
        input,
        id.with(10),
        shortcut.trigger,
        state,
        choices,
        bounds,
    );
    if let Some(trigger) = changed {
        set(Shortcut {
            trigger,
            ..shortcut
        });
    }
    y += row + gap;

    // The event, and what depends on it.
    let events = Event::of_device(device);
    if let Some(current) = Event::of(shortcut.trigger) {
        let at = labeled(ui, y, "Event");
        let items: [MenuItem<'_>; 5] = core::array::from_fn(|index| {
            MenuItem::action(events.get(index).map_or("", |event| event.label()))
        });
        if let Some(choice) = Dropdown::new(id.with(11), &mut state.event, &items[..events.len()])
            .label(current.label())
            .show(ui, at, bounds)
            && let Some(event) = events.get(choice.index())
        {
            set(Shortcut {
                trigger: event.apply(shortcut.trigger),
                ..shortcut
            });
        }
        y += row + gap;
    }
    match shortcut.trigger {
        Trigger::Key { key, repeat, .. } if can_repeat(shortcut.trigger) => {
            let at = labeled(ui, y, "Repeat");
            let mut on = repeat;
            if Checkbox::new(id.with(12), &mut on)
                .label("While the key is held")
                .show(ui, at)
                .changed
            {
                set(Shortcut {
                    trigger: Trigger::Key {
                        key,
                        when: KeyTrigger::Press,
                        repeat: on,
                    },
                    ..shortcut
                });
            }
            y += row + gap;
        }
        Trigger::Button {
            button,
            when: ButtonTrigger::Drag(drag),
        } => {
            let at = labeled(ui, y, "Direction");
            let items = DIRECTIONS.map(|(_, name)| MenuItem::action(name));
            let current = DIRECTIONS
                .iter()
                .find(|(direction, _)| *direction == drag.direction)
                .map_or("", |(_, name)| name);
            if let Some(choice) = Dropdown::new(id.with(13), &mut state.direction, &items)
                .label(current)
                .show(ui, at, bounds)
                && let Some(&(direction, _)) = DIRECTIONS.get(choice.index())
            {
                set(Shortcut {
                    trigger: Trigger::Button {
                        button,
                        when: ButtonTrigger::Drag(Drag { direction, ..drag }),
                    },
                    ..shortcut
                });
            }
            y += row + gap;
            let at = labeled(ui, y, "Distance");
            let mut own = drag.distance.is_some();
            let check = Rect::from_origin_size(at.min, Vec2::new(at.width() * 0.4, row));
            let mut distance = f64::from(drag.distance.unwrap_or(8));
            let toggled = Checkbox::new(id.with(14), &mut own)
                .label("Its own")
                .show(ui, check)
                .changed;
            let field = Rect::from_min_max(Vec2::new(check.max.x + gap, at.min.y), at.max);
            let moved = NumberField::new(id.with(15), &mut distance)
                .range(1.0, 200.0)
                .decimals(0)
                .unit(&koebi_core::expr::PIXEL)
                .bar(BarMapping::Linear)
                .enabled(own)
                .show(ui, field)
                .changed;
            if toggled || moved {
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a distance from 1 to 200 pixels"
                )]
                let pixels = distance.round().clamp(1.0, 200.0) as u16;
                set(Shortcut {
                    trigger: Trigger::Button {
                        button,
                        when: ButtonTrigger::Drag(Drag {
                            distance: own.then_some(pixels),
                            ..drag
                        }),
                    },
                    ..shortcut
                });
            }
            y += row + gap;
        }
        _ => {}
    }
    if matches!(Event::of(shortcut.trigger), Some(Event::DoubleClick)) {
        // The time is the application's, for every double click.
        let at = labeled(ui, y, "Double click time");
        let token = semantic::INPUT_DOUBLE_CLICK_TIME;
        let mut millis = design.get(token).as_secs_f64() * 1000.0;
        if NumberField::new(id.with(16), &mut millis)
            .range(100.0, 2000.0)
            .decimals(0)
            .unit(&koebi_core::expr::MILLISECOND)
            .bar(BarMapping::Linear)
            .show(ui, at)
            .changed
        {
            let time = core::time::Duration::from_secs_f64(millis.max(0.0) / 1000.0);
            if let Err(error) = design.set_value(Scope::Global, token, time) {
                koebi_core::warn!("cannot change the double click time: {error}");
            }
        }
        y += row + gap;
    }

    // The modifiers: one per family with its sides below, any, and a key held.
    if !matches!(device, Device::Pointer) {
        let at = labeled(ui, y, "Modifiers");
        let grid = Rect::from_min_max(at.min, Vec2::new(at.max.x, at.min.y + 2.0 * row));
        if let Some(rule) = modifiers(ui, grid, id.with(20), shortcut.modifiers) {
            set(Shortcut {
                modifiers: rule,
                ..shortcut
            });
        }
        y += 2.0 * row + gap;
        let at = labeled(ui, y, "Key held");
        let held = CaptureField::new(
            id.with(30),
            &mut state.held,
            shortcut.held.map(Shortcut::key),
        )
        .capture(Capture::Keyboard)
        .hints("None", "Press the key to hold, Backspace for none")
        .show(ui, at);
        if let Some(captured) = held.captured
            && let Trigger::Key { key, .. } = captured.trigger
        {
            let held = (!matches!(key, Key::Backspace | Key::Delete)).then_some(key);
            set(Shortcut { held, ..shortcut });
        }
        ui.tooltip(
            id.with(30),
            "A key held as a modifier: the shortcut triggers only while it is down",
            at,
        );
        y += row + gap;
    }
    y - rect.min.y
}

/// What the row of a device's input is called.
const fn input_title(device: Device) -> &'static str {
    match device {
        Device::Keyboard => "Key",
        Device::Mouse => "Button",
        Device::Wheel => "Direction",
        Device::Touchpad => "Gesture",
        Device::Pointer => "Input",
    }
}

/// The dropdown choosing the key, button or direction of `trigger` in `rect`; the trigger it
/// becomes when one is chosen.
fn input_choice(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    trigger: Trigger,
    state: &mut Editor,
    choices: &Choices,
    bounds: Rect,
) -> Option<Trigger> {
    let menu = &mut state.input;
    match trigger {
        Trigger::Key { key, when, repeat } => {
            let current = chosen_keys()
                .find(|(chosen, _)| *chosen == key)
                .map_or("Other key", |(_, name)| name);
            let choice = Dropdown::new(id, menu, &choices.keys)
                .label(current)
                .show(ui, rect, bounds)?;
            let key = *choices.key_list.get(choice.index())?;
            Some(Trigger::Key { key, when, repeat })
        }
        Trigger::Button { button, when } => {
            let items = BUTTONS.map(|(_, name)| MenuItem::action(name));
            let current = BUTTONS
                .iter()
                .find(|(chosen, _)| *chosen == button)
                .map_or("Other button", |(_, name)| name);
            let choice = Dropdown::new(id, menu, &items)
                .label(current)
                .show(ui, rect, bounds)?;
            let (button, _) = *BUTTONS.get(choice.index())?;
            Some(Trigger::Button { button, when })
        }
        Trigger::Wheel(direction) => {
            let items = WHEEL.map(|(_, name)| MenuItem::action(name));
            let current = WHEEL
                .iter()
                .find(|(chosen, _)| *chosen == direction)
                .map_or("", |(_, name)| name);
            let choice = Dropdown::new(id, menu, &items)
                .label(current)
                .show(ui, rect, bounds)?;
            Some(Trigger::Wheel(WHEEL.get(choice.index())?.0))
        }
        Trigger::Pinch(direction) => {
            let items = PINCHES.map(|(_, name)| MenuItem::action(name));
            let current = PINCHES
                .iter()
                .find(|(chosen, _)| *chosen == direction)
                .map_or("", |(_, name)| name);
            let choice = Dropdown::new(id, menu, &items)
                .label(current)
                .show(ui, rect, bounds)?;
            Some(Trigger::Pinch(PINCHES.get(choice.index())?.0))
        }
        Trigger::PointerMove => {
            let secondary = ui.styles().text.secondary;
            let font = ui.styles().text.label.clone();
            ui.label("Any movement", rect, &font, secondary, Align::Start);
            None
        }
    }
}

/// The modifiers of `rule` in `rect`: a row of families and any, their sides below. Returns the
/// rule the user made.
fn modifiers(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    rule: ModifierRule,
) -> Option<ModifierRule> {
    let families = [
        (ModifierKey::Ctrl, "Ctrl"),
        (ModifierKey::Shift, "Shift"),
        (
            ModifierKey::Alt,
            if cfg!(target_os = "macos") {
                "Option"
            } else {
                "Alt"
            },
        ),
        (ModifierKey::Gui, gui_name()),
    ];
    let any = is_any(rule);
    // The grid's columns: two per family (its sides below), two for any.
    let column = |index: usize| u16::try_from(index).unwrap_or(u16::MAX);
    let mut cells: [GroupCell<'_>; 13] = core::array::from_fn(|index| {
        let key = id.with(index as u64);
        let button = Button::new(key);
        match index {
            0..4 => {
                let (family, name) = families[index];
                let held = matches!(rule.get(family), ModifierState::Down(_));
                GroupCell::new(button.label(name).selected(held && !any).enabled(!any))
                    .at(2 * column(index), 0)
                    .span(2, 1)
            }
            4 => GroupCell::new(button.label("Any").selected(any))
                .at(8, 0)
                .span(2, 2),
            _ => {
                let family = (index - 5) / 2;
                let side = if (index - 5) % 2 == 0 {
                    Side::Left
                } else {
                    Side::Right
                };
                let state = rule.get(families[family].0);
                let held = matches!(state, ModifierState::Down(_));
                GroupCell::new(
                    button
                        .label(if side == Side::Left { "L" } else { "R" })
                        .selected(held && side_on(state, side))
                        .enabled(held && !any),
                )
                .at(2 * column(family) + u16::from(side == Side::Right), 1)
            }
        }
    });
    let columns = [Track::Share(1.0); 10];
    let clicked = ButtonGroup::horizontal()
        .columns(&columns)
        .show(ui, rect, &mut cells)
        .clicked?;
    Some(match clicked {
        0..4 => toggle_family(rule, families[clicked].0),
        4 => toggle_any(rule),
        _ => {
            let family = families[(clicked - 5) / 2].0;
            let side = if (clicked - 5) % 2 == 0 {
                Side::Left
            } else {
                Side::Right
            };
            toggle_side(rule, family, side)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_to_choose_are_the_named_keys_but_modifiers() {
        let keys: Vec<Key> = chosen_keys().map(|(key, _)| key).collect();
        assert!(keys.contains(&Key::A) && keys.contains(&Key::F12) && keys.contains(&Key::Space));
        assert!(!keys.iter().any(|key| key.is_modifier()));
        let choices = Choices::new();
        assert_eq!(choices.keys.len(), choices.key_list.len());
    }
}
