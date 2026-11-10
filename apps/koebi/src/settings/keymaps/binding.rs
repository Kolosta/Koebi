//! One shortcut of an action in the shortcut editor: its header (on or off, the device, the
//! field recording it, removal) and its details (what it says of its conflicts, the key, the
//! event and what depends on it, the modifiers and their sides, a key held as a modifier).
//!
//! A shortcut being added is a draft until it has a key or a button: its header and its input
//! show, nothing else, and it conflicts with nothing.
//!
//! The rules of editing are pure functions ([`super::rules`]), tested on their own; the page
//! applies what the user changed to the keymap.

use koebi_core::input::{Key, ModifierKey};
use koebi_core::{Color, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_i18n::tr;
use koebi_input::{
    ActionDef, Binding, ButtonTrigger, ConflictKind, DoubleClick, Drag, KeyTrigger, ModifierRule,
    ModifierState, Shortcut, Side, Trigger,
};
use koebi_tokens::{component, semantic};
use koebi_ui::icon_bounds;
use koebi_ui::{
    Align, BarMapping, Button, ButtonGroup, Capture, CaptureField, CaptureState, Checkbox,
    CollapsiblePanelState, Dropdown, GroupCell, IconId, MenuItem, MenuState, NumberField, Track,
    Ui, WidgetId,
};

use super::analysis::BindingReport;
pub(super) use super::rules::Device;
use super::rules::{
    DIRECTIONS, Event, KEY_SECTIONS, MOUSE_SECTIONS, MouseInput, PINCHES, Section, can_repeat,
    is_any, name_in, side_on, toggle_any, toggle_family, toggle_side,
};

/// What the user changed in a shortcut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Edit {
    /// It becomes this.
    Set(Binding),
    /// It goes.
    Remove,
    /// It takes this device: a draft waits for its input, a shortcut takes the device's first
    /// input that no other shortcut of the action has.
    Device(Device),
}

/// A shortcut as the editor shows it: one of the keymap, or a draft without its key yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Shown {
    Binding(Binding),
    Draft(Device),
}

impl Shown {
    fn device(self) -> Device {
        match self {
            Self::Binding(binding) => Device::of(binding.shortcut.trigger),
            Self::Draft(device) => device,
        }
    }
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

    /// The state of a shortcut being added: open, on the keyboard.
    pub(super) fn draft() -> Self {
        Self {
            panel: CollapsiblePanelState::new(true),
            editor: Editor::new(),
        }
    }
}

/// A shortcut being added: its device, until it gets its key or button.
#[derive(Debug)]
pub(super) struct Draft {
    pub(super) device: Device,
    pub(super) ui: BindingUi,
}

impl Draft {
    pub(super) fn new() -> Self {
        Self {
            device: Device::Keyboard,
            ui: BindingUi::draft(),
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
    /// The modifiers Any replaced: shown disabled meanwhile, given back when it goes off.
    before_any: Option<ModifierRule>,
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
            before_any: None,
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

/// The lists the details choose from, built once: the items of each section, each with the
/// caps of its key or button.
#[derive(Debug)]
pub(super) struct Choices {
    keys: Vec<Vec<MenuItem<'static>>>,
    mouse: Vec<Vec<MenuItem<'static>>>,
}

impl Choices {
    pub(super) fn new() -> Self {
        let keys = KEY_SECTIONS
            .iter()
            .map(|section| {
                section
                    .inputs
                    .iter()
                    .map(|(key, name)| MenuItem::action(name).caps(Shortcut::key(*key)))
                    .collect()
            })
            .collect();
        let mouse = MOUSE_SECTIONS
            .iter()
            .map(|section| {
                section
                    .inputs
                    .iter()
                    .map(|(input, name)| {
                        let trigger =
                            input.apply(Trigger::button(koebi_core::input::MouseButton::Left));
                        MenuItem::action(name).caps(Shortcut::new(trigger))
                    })
                    .collect()
            })
            .collect();
        Self { keys, mouse }
    }
}

/// The controls of a shortcut's header in `rect`: on or off, the device, the field recording
/// it, and the button removing it. A draft's is off until it has its key.
pub(super) fn header(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    shown: Shown,
    state: &mut Editor,
    bounds: Rect,
) -> Option<Edit> {
    let row = rect.height();
    let gap = ui.styles().collapsible_panel.header_gap;
    let mut edit = None;
    // On or off; a draft cannot be on yet.
    let check = Rect::from_origin_size(rect.min, Vec2::new(row, row));
    let mut enabled = matches!(shown, Shown::Binding(binding) if binding.enabled);
    let toggled = Checkbox::new(id.with(1), &mut enabled)
        .enabled(matches!(shown, Shown::Binding(_)))
        .show(ui, check)
        .changed;
    if toggled && let Shown::Binding(binding) = shown {
        edit = Some(Edit::Set(Binding { enabled, ..binding }));
    }
    // The device.
    let device = shown.device();
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
        edit = Some(Edit::Device(chosen));
    }
    // The field recording it, then the removal.
    let remove = Rect::from_min_max(Vec2::new(rect.max.x - row, rect.min.y), rect.max);
    let field = Rect::from_min_max(
        Vec2::new(at.max.x + gap, rect.min.y),
        Vec2::new((remove.min.x - gap).max(at.max.x + gap), rect.max.y),
    );
    let (capture, empty) = match device {
        Device::Mouse => (Capture::Mouse, tr("Click to record a button")),
        _ if device.records() => (Capture::Keyboard, tr("Click to record a key")),
        _ => (Capture::Keyboard, tr("Choose it below")),
    };
    let current = match shown {
        Shown::Binding(binding) => Some(binding.shortcut),
        Shown::Draft(_) => None,
    };
    let recorded = CaptureField::new(id.with(3), &mut state.capture, current)
        .capture(capture)
        .hints(empty, tr("Hold modifiers, then press"))
        .enabled(device.records())
        .show(ui, field);
    if let Some(shortcut) = recorded.captured {
        edit = Some(Edit::Set(match shown {
            // A held key set apart stays, and on or off too.
            Shown::Binding(binding) => Binding {
                shortcut: Shortcut {
                    held: binding.shortcut.held,
                    ..shortcut
                },
                ..binding
            },
            Shown::Draft(_) => Binding::on(shortcut),
        }));
    }
    if Button::new(id.with(4))
        .icon(koebi_icons::interface::CLOSE)
        .show(ui, remove)
        .clicked
    {
        edit = Some(Edit::Remove);
    }
    ui.tooltip(id.with(4), tr("Remove this shortcut"), remove);
    edit
}

/// The rows of a shortcut's details: each a label and its control.
struct Rows {
    left: f32,
    right: f32,
    label_width: f32,
    row: f32,
    gap: f32,
    y: f32,
}

impl Rows {
    fn new(ui: &Ui<'_>, rect: Rect, design: &DesignSystem) -> Self {
        let padding = ui.styles().panel.padding;
        let (left, right) = (rect.min.x + padding, rect.max.x - padding);
        Self {
            left,
            right,
            label_width: 110.0_f32.min((right - left) * 0.35),
            row: ui.styles().button.height,
            gap: ui.whole_pixels(design.get(semantic::SPACING_EDITOR_INSET) * 0.5),
            y: rect.min.y,
        }
    }

    /// A row labeled `label`, `rows` controls high: the room of its control.
    fn labeled(&mut self, ui: &mut Ui<'_>, label: &str, rows: f32) -> Rect {
        let font = ui.styles().text.label.clone();
        let secondary = ui.styles().text.secondary;
        let title = Rect::from_origin_size(
            Vec2::new(self.left, self.y),
            Vec2::new(self.label_width, self.row),
        );
        ui.label(label, title, &font, secondary, Align::Start);
        let room = Rect::from_min_max(
            Vec2::new(self.left + self.label_width, self.y),
            Vec2::new(self.right, self.y + rows * self.row),
        );
        self.y += rows * self.row + self.gap;
        room
    }

    /// A line of text, after `icon` in `color` when given.
    fn line(&mut self, ui: &mut Ui<'_>, icon: Option<(IconId, Color)>, text: &str) {
        let line = Rect::from_min_max(
            Vec2::new(self.left, self.y),
            Vec2::new(self.right, self.y + self.row),
        );
        let icons = ui.styles().icon;
        let mut start = line.min.x;
        if let Some((icon, color)) = icon {
            let slot = Rect::from_origin_size(line.min, Vec2::new(icons.size, self.row));
            ui.icon(icon_bounds(slot, icons.size), icon, color);
            start = slot.max.x + self.gap;
        }
        let detail = ui.styles().text.detail.clone();
        let secondary = ui.styles().text.secondary;
        let text_rect = Rect::from_min_max(Vec2::new(start, line.min.y), line.max);
        ui.label(text, text_rect, &detail, secondary, Align::Start);
        self.y += self.row;
    }
}

/// The details of a draft in `rect`: what it waits for, and its input to choose. Returns the
/// height used, and the shortcut it becomes in `edit` once its input is chosen.
#[allow(clippy::too_many_arguments, reason = "the parts of one shortcut")]
pub(super) fn draft_details(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    device: Device,
    state: &mut Editor,
    choices: &Choices,
    design: &DesignSystem,
    bounds: Rect,
    edit: &mut Option<Edit>,
) -> f32 {
    let mut rows = Rows::new(ui, rect, design);
    rows.line(
        ui,
        None,
        tr("Record its key or button above, or choose it: the shortcut works from then on."),
    );
    rows.y += rows.gap;
    let input = rows.labeled(ui, input_title(device), 1.0);
    let trigger = device.trigger(Trigger::key(Key::A));
    if let Some(trigger) = input_choice(
        ui,
        input,
        id.with(10),
        trigger,
        true,
        state,
        choices,
        bounds,
    ) {
        *edit = Some(Edit::Set(Binding::on(Shortcut::new(trigger))));
    }
    rows.y - rect.min.y
}

/// The details of a shortcut of `action` in `rect`: its conflicts, its key, its event and what
/// depends on it, its modifiers. Returns the height used, and what the user changed in `edit`.
#[allow(clippy::too_many_arguments, reason = "the parts of one shortcut")]
pub(super) fn details(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    action: &ActionDef,
    binding: Binding,
    report: Option<&BindingReport>,
    state: &mut Editor,
    choices: &Choices,
    design: &DesignSystem,
    bounds: Rect,
    edit: &mut Option<Edit>,
) -> f32 {
    let mut rows = Rows::new(ui, rect, design);
    let shortcut = binding.shortcut;
    let mut set = |shortcut: Shortcut| {
        *edit = Some(Edit::Set(Binding {
            shortcut,
            ..binding
        }));
    };

    // What its conflicts say, and the actions it is shared with.
    let messages = report.map_or(&[][..], |report| report.messages.as_slice());
    for (kind, text) in messages {
        let icon = if kind.is_conflict() {
            (
                koebi_icons::interface::WARNING,
                conflict_color(design, *kind),
            )
        } else {
            (koebi_icons::interface::INFO, ui.styles().text.secondary)
        };
        rows.line(ui, Some(icon), text);
    }
    if !messages.is_empty() {
        rows.y += rows.gap;
    }

    // The key, the button, the direction.
    let device = Device::of(shortcut.trigger);
    let input = rows.labeled(ui, input_title(device), 1.0);
    if let Some(trigger) = input_choice(
        ui,
        input,
        id.with(10),
        shortcut.trigger,
        false,
        state,
        choices,
        bounds,
    ) {
        set(Shortcut {
            trigger,
            ..shortcut
        });
    }

    // The event, and what depends on it.
    let events = Event::of_trigger(shortcut.trigger);
    if let Some(current) = Event::of(shortcut.trigger)
        && !events.is_empty()
    {
        let at = rows.labeled(ui, tr("Event"), 1.0);
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
    }
    match shortcut.trigger {
        Trigger::Key { key, repeat, .. } if can_repeat(action, shortcut.trigger) => {
            let at = rows.labeled(ui, tr("Repeat"), 1.0);
            let mut on = repeat;
            if Checkbox::new(id.with(12), &mut on)
                .label(tr("While the key is held"))
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
        }
        Trigger::Button {
            button,
            when: ButtonTrigger::Drag(drag),
        } => {
            let at = rows.labeled(ui, tr("Direction"), 1.0);
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
            let at = rows.labeled(ui, tr("Distance"), 1.0);
            let settings = design.get(semantic::INPUT_DRAG_THRESHOLD);
            let value = drag.distance.map(f64::from);
            if let Some(distance) = own_value(
                ui,
                at,
                id.with(14),
                value,
                f64::from(settings),
                (1.0, 200.0),
                &koebi_core::expr::PIXEL,
            ) {
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a distance from 1 to 200 pixels"
                )]
                let pixels = distance
                    .value()
                    .map(|distance| distance.round().clamp(1.0, 200.0) as u16);
                set(Shortcut {
                    trigger: Trigger::Button {
                        button,
                        when: ButtonTrigger::Drag(Drag {
                            distance: pixels,
                            ..drag
                        }),
                    },
                    ..shortcut
                });
            }
        }
        Trigger::Button {
            button,
            when: ButtonTrigger::DoubleClick(double),
        } => {
            // The application's time, unless the shortcut has its own.
            let at = rows.labeled(ui, tr("Double click time"), 1.0);
            let settings = design.get(semantic::INPUT_DOUBLE_CLICK_TIME);
            let (shortest, longest) = DoubleClick::TIMES;
            if let Some(time) = own_value(
                ui,
                at,
                id.with(16),
                double.time.map(f64::from),
                settings.as_secs_f64() * 1000.0,
                (f64::from(shortest), f64::from(longest)),
                &koebi_core::expr::MILLISECOND,
            ) {
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a time within the double click's limits, in milliseconds"
                )]
                let millis = time
                    .value()
                    .map(|time| time.round().clamp(f64::from(shortest), f64::from(longest)) as u16);
                set(Shortcut {
                    trigger: Trigger::Button {
                        button,
                        when: ButtonTrigger::DoubleClick(DoubleClick { time: millis }),
                    },
                    ..shortcut
                });
            }
        }
        _ => {}
    }

    // The modifiers: one group per family with its sides below, any, and a key held.
    if !matches!(device, Device::Pointer) {
        let grid = rows.labeled(ui, tr("Modifiers"), 2.0);
        if let Some(rule) = modifiers(ui, grid, id.with(20), shortcut.modifiers, state) {
            set(Shortcut {
                modifiers: rule,
                ..shortcut
            });
        }
        let at = rows.labeled(ui, tr("Key held"), 1.0);
        let row = rows.row;
        let has_held = shortcut.held.is_some();
        // A button clearing it, once there is one.
        let (field, clear) = if has_held {
            let clear = Rect::from_min_max(Vec2::new(at.max.x - row, at.min.y), at.max);
            let field = Rect::from_min_max(at.min, Vec2::new(clear.min.x - rows.gap, at.max.y));
            (field, Some(clear))
        } else {
            (at, None)
        };
        let held = CaptureField::new(
            id.with(30),
            &mut state.held,
            shortcut.held.map(Shortcut::key),
        )
        .capture(Capture::Keyboard)
        .hints(tr("None"), tr("Press the key to hold"))
        .show(ui, field);
        if let Some(captured) = held.captured
            && let Trigger::Key { key, .. } = captured.trigger
        {
            set(Shortcut {
                held: Some(key),
                ..shortcut
            });
        }
        ui.tooltip(
            id.with(30),
            tr("A key held as a modifier: the shortcut triggers only while it is down"),
            field,
        );
        if let Some(clear) = clear
            && Button::new(id.with(31))
                .icon(koebi_icons::interface::CLOSE)
                .show(ui, clear)
                .clicked
        {
            set(Shortcut {
                held: None,
                ..shortcut
            });
        }
    }
    rows.y - rect.min.y
}

/// What a value a shortcut may have of its own becomes.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Own {
    /// The application's.
    Settings,
    /// Its own.
    Value(f64),
}

impl Own {
    fn value(self) -> Option<f64> {
        match self {
            Self::Settings => None,
            Self::Value(value) => Some(value),
        }
    }
}

/// A value the shortcut may have of its own in `rect`: a checkbox choosing it, and a field
/// showing it, or the application's `settings` disabled. Returns what it becomes when the user
/// changed something.
#[allow(clippy::too_many_arguments, reason = "the parts of one row")]
fn own_value(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    value: Option<f64>,
    settings: f64,
    (min, max): (f64, f64),
    unit: &'static koebi_core::expr::Unit,
) -> Option<Own> {
    let gap = ui.styles().collapsible_panel.header_gap;
    let mut own = value.is_some();
    let check = Rect::from_origin_size(rect.min, Vec2::new(rect.width() * 0.4, rect.height()));
    let toggled = Checkbox::new(id, &mut own)
        .label(tr("Its own"))
        .show(ui, check)
        .changed;
    let field = Rect::from_min_max(Vec2::new(check.max.x + gap, rect.min.y), rect.max);
    let mut shown = value.unwrap_or(settings).clamp(min, max);
    let moved = NumberField::new(id.with(1), &mut shown)
        .range(min, max)
        .decimals(0)
        .unit(unit)
        .bar(BarMapping::Linear)
        .enabled(own)
        .show(ui, field)
        .changed;
    ui.tooltip(
        id.with(1),
        tr("The application's value, unless this shortcut has its own"),
        field,
    );
    (toggled || moved).then_some(if own {
        Own::Value(shown)
    } else {
        Own::Settings
    })
}

/// What the row of a device's input is called.
fn input_title(device: Device) -> &'static str {
    match device {
        Device::Keyboard => tr("Key"),
        Device::Mouse => tr("Button"),
        Device::Touchpad => tr("Gesture"),
        Device::Pointer => tr("Input"),
    }
}

/// The dropdown choosing the key, button or direction of `trigger` in `rect` (nothing chosen
/// yet for a draft); the trigger it becomes when one is chosen.
#[allow(clippy::too_many_arguments, reason = "the parts of one row")]
fn input_choice(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    trigger: Trigger,
    draft: bool,
    state: &mut Editor,
    choices: &Choices,
    bounds: Rect,
) -> Option<Trigger> {
    let menu = &mut state.input;
    match trigger {
        Trigger::Key { key, when, repeat } => {
            let current = if draft {
                tr("Choose a key")
            } else {
                name_in(&KEY_SECTIONS, key).unwrap_or(tr("Other key"))
            };
            let key = sectioned(
                ui,
                rect,
                id,
                menu,
                &KEY_SECTIONS,
                &choices.keys,
                current,
                bounds,
            )?;
            Some(Trigger::Key { key, when, repeat })
        }
        Trigger::Button { .. } | Trigger::Wheel(_) => {
            let current = if draft {
                tr("Choose a button")
            } else {
                MouseInput::of(trigger)
                    .and_then(|input| name_in(&MOUSE_SECTIONS, input))
                    .unwrap_or(tr("Other button"))
            };
            let input = sectioned(
                ui,
                rect,
                id,
                menu,
                &MOUSE_SECTIONS,
                &choices.mouse,
                current,
                bounds,
            )?;
            Some(input.apply(trigger))
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
            ui.label(tr("Any movement"), rect, &font, secondary, Align::Start);
            None
        }
    }
}

/// A dropdown of `sections`, a submenu each holding `items`, labeled `current`; the input
/// chosen.
#[allow(clippy::too_many_arguments, reason = "the parts of one dropdown")]
fn sectioned<T: Copy>(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    menu: &mut MenuState,
    sections: &[Section<T>],
    items: &[Vec<MenuItem<'static>>],
    current: &str,
    bounds: Rect,
) -> Option<T> {
    // A submenu per section, on the stack: there are few.
    let mut top: [MenuItem<'_>; 8] = [MenuItem::Separator; 8];
    let count = sections.len().min(top.len());
    for (slot, (section, items)) in top.iter_mut().zip(sections.iter().zip(items)) {
        *slot = MenuItem::submenu(section.name, items);
    }
    let choice = Dropdown::new(id, menu, &top[..count])
        .label(current)
        .show(ui, rect, bounds)?;
    let &[section, index] = choice.path() else {
        return None;
    };
    sections
        .get(section)?
        .inputs
        .get(index)
        .map(|(input, _)| *input)
}

/// The modifiers of `rule` in `rect`: a group per family, its two sides below, then any.
/// While any is on, the families keep the modifiers it replaced, disabled. Returns the rule
/// the user made.
fn modifiers(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    rule: ModifierRule,
    state: &mut Editor,
) -> Option<ModifierRule> {
    let keyboard = ui.keyboard();
    let any = is_any(rule);
    // What the families show: the rule, or under any the one it replaced.
    let shown = if any {
        state.before_any.unwrap_or(ModifierRule::NONE)
    } else {
        rule
    };
    let gap = ui.styles().collapsible_panel.header_gap;
    let any_width = (rect.width() * 0.16).max(rect.height());
    let groups = ModifierKey::ALL.len() as f32;
    let width = ((rect.width() - any_width - groups * gap) / groups).max(0.0);
    let mut changed = None;
    for (index, family) in ModifierKey::ALL.into_iter().enumerate() {
        let x = rect.min.x + index as f32 * (width + gap);
        let at = Rect::from_min_max(Vec2::new(x, rect.min.y), Vec2::new(x + width, rect.max.y));
        let key = id.with(index as u64 * 4);
        let state_of = shown.get(family);
        let held = matches!(state_of, ModifierState::Down(_));
        let side = |side: Side, label: &'static str, key: WidgetId| {
            Button::new(key)
                .label(label)
                .selected(held && side_on(state_of, side))
                .enabled(held && !any)
        };
        let mut cells = [
            GroupCell::new(
                Button::new(key.with(1))
                    .label(keyboard.modifier_name(family))
                    .selected(held)
                    .enabled(!any),
            )
            .at(0, 0)
            .span(2, 1),
            GroupCell::new(side(Side::Left, "L", key.with(2))).at(0, 1),
            GroupCell::new(side(Side::Right, "R", key.with(3))).at(1, 1),
        ];
        let columns = [Track::Share(1.0); 2];
        let clicked = ButtonGroup::horizontal()
            .columns(&columns)
            .show(ui, at, &mut cells)
            .clicked;
        changed = changed.or(match clicked {
            Some(0) => Some(toggle_family(rule, family)),
            Some(1) => Some(toggle_side(rule, family, Side::Left)),
            Some(2) => Some(toggle_side(rule, family, Side::Right)),
            _ => None,
        });
    }
    let any_at = Rect::from_min_max(Vec2::new(rect.max.x - any_width, rect.min.y), rect.max);
    // A group of one cell, so that it looks like the families beside it: their state shows as
    // selected cells with the group's borders, not as a toggle button.
    let mut any_cell = [GroupCell::new(
        Button::new(id.with(40)).label(tr("Any")).selected(any),
    )];
    if ButtonGroup::horizontal()
        .show(ui, any_at, &mut any_cell)
        .clicked
        .is_some()
    {
        let next = toggle_any(rule, state.before_any);
        state.before_any = (!any).then_some(rule);
        changed = Some(next);
    }
    ui.tooltip(
        id.with(40),
        tr("Any modifiers held or not; the others keep their values for when it goes off"),
        any_at,
    );
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_section_has_its_items_with_their_caps() {
        let choices = Choices::new();
        assert_eq!(choices.keys.len(), KEY_SECTIONS.len());
        for (section, items) in KEY_SECTIONS.iter().zip(&choices.keys) {
            assert_eq!(section.inputs.len(), items.len());
            assert!(items.iter().all(|item| matches!(
                item,
                MenuItem::Action(action) if action.caps.is_some()
            )));
        }
        assert_eq!(choices.mouse.len(), MOUSE_SECTIONS.len());
        assert!(KEY_SECTIONS.len() <= 8 && MOUSE_SECTIONS.len() <= 8);
    }
}
