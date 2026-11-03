//! [`Keymap`]: the shortcuts of every action, and the engine that turns input into actions.
//!
//! - **Resolution**: each input event becomes gestures (press, release, click, double click,
//!   drag, wheel, pinch); each gesture is looked up in the chain of contexts the caller gives
//!   (the zones under the pointer, innermost first); the first context with a matching
//!   shortcut wins. A button or key held keeps the chain of its press: a drag that starts on
//!   a gutter stays the gutter's, wherever it goes.
//! - **Conflicts**: two actions of one context never share a shortcut that one input could
//!   trigger twice ([`Shortcut::overlaps`]); every change is checked. A shortcut turned off
//!   ([`Binding`]) triggers nothing and conflicts with nothing: an editor keeps there what
//!   conflicts until the user changes it. [`crate::Zones`] tells the conflicts across contexts.
//! - **Keys held as modifiers**: a shortcut holding a key (`Space+G`) triggers only while it is
//!   down, and wins over the same shortcut without it in its context.
//! - **Overrides**: the shortcuts of an action can be replaced; an action is *modified* when
//!   its shortcuts differ from its defaults, whatever the edits that led there.

use std::collections::HashMap;
use std::fmt;
use std::time::Instant;

use koebi_core::Vec2;
use koebi_core::input::{Key, Modifiers, MouseButton};

use crate::action::{ActionDef, ActionGroup, Context};
use crate::gesture::{Gesture, GestureKind, InputEvent, InputSettings};
use crate::shortcut::{
    Binding, ButtonTrigger, DoubleClick, Drag, DragDirection, KeyTrigger, PinchDirection, Shortcut,
    Trigger, WheelDirection,
};

/// A keymap error.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeymapError {
    /// Two actions have the same identifier.
    DuplicateAction(&'static str),
    /// No action has this identifier.
    UnknownAction(String),
    /// No group has this name.
    UnknownGroup(String),
    /// The shortcut is already used by another action of the same context.
    Conflict {
        /// The action being changed.
        action: &'static str,
        /// The shortcut in conflict.
        shortcut: Shortcut,
        /// The action that has it.
        with: &'static str,
    },
    /// A default shortcut of an action that does not repeat repeats.
    Repeats(&'static str),
}

impl fmt::Display for KeymapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateAction(id) => write!(f, "two actions are named {id}"),
            Self::UnknownAction(id) => write!(f, "unknown action {id}"),
            Self::UnknownGroup(name) => write!(f, "unknown action group {name}"),
            Self::Conflict {
                action,
                shortcut,
                with,
            } => write!(f, "{shortcut} of {action} is already a shortcut of {with}"),
            Self::Repeats(id) => write!(f, "{id} does not repeat, but a default shortcut does"),
        }
    }
}

impl std::error::Error for KeymapError {}

/// An action triggered by a gesture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triggered {
    /// The action.
    pub action: &'static ActionDef,
    /// What triggered it, and where.
    pub gesture: Gesture,
}

/// The actions one input event triggered: a release may also be a click.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Triggers {
    items: [Option<Triggered>; Self::CAPACITY],
    len: usize,
}

impl Triggers {
    /// A release that is also a click, each for actions that share their shortcut.
    const CAPACITY: usize = 8;

    fn push(&mut self, triggered: Triggered) {
        if let Some(slot) = self.items.get_mut(self.len) {
            *slot = Some(triggered);
            self.len += 1;
        }
    }

    /// The actions triggered, in order.
    pub fn iter(&self) -> impl Iterator<Item = &Triggered> {
        self.items[..self.len].iter().flatten()
    }

    /// `true` when nothing was triggered.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// The shortcuts of an action.
#[derive(Debug)]
struct Entry {
    action: &'static ActionDef,
    /// Its shortcuts on and off, in the order the user sees them.
    bindings: Vec<Binding>,
    /// Those that are on: what triggers the action.
    shortcuts: Vec<Shortcut>,
}

impl Entry {
    fn is_modified(&self) -> bool {
        self.bindings.len() != self.action.defaults.len()
            || self.bindings.iter().any(|binding| {
                !binding.enabled || !self.action.defaults.contains(&binding.shortcut)
            })
    }

    /// Replaces the bindings, keeping the first of each shortcut. An action that does not
    /// repeat keeps none that repeats.
    fn set(&mut self, bindings: impl IntoIterator<Item = Binding>) {
        self.bindings.clear();
        let repeats = self.action.repeats;
        for binding in bindings {
            let binding = if repeats {
                binding
            } else {
                Binding {
                    shortcut: once(binding.shortcut),
                    ..binding
                }
            };
            if !self
                .bindings
                .iter()
                .any(|kept| kept.shortcut == binding.shortcut)
            {
                self.bindings.push(binding);
            }
        }
        self.shortcuts.clear();
        self.shortcuts.extend(
            self.bindings
                .iter()
                .filter(|binding| binding.enabled)
                .map(|binding| binding.shortcut),
        );
    }

    /// Back to the defaults, all on.
    fn restore(&mut self) {
        let defaults = self.action.defaults;
        self.set(defaults.iter().copied().map(Binding::on));
    }
}

/// A mouse button held down.
#[derive(Debug)]
struct ButtonPress {
    button: MouseButton,
    origin: Vec2,
    modifiers: Modifiers,
    /// The key held as a modifier when it went down.
    held: Option<Key>,
    /// Moved beyond the drag threshold: its release is not a click.
    moved: bool,
    /// Started a drag shortcut: no other drag starts until it is released.
    dragged: bool,
    chain: Vec<Context>,
}

/// A key held down.
#[derive(Debug)]
struct KeyPress {
    key: Key,
    chain: Vec<Context>,
}

/// The shortcuts of every action, and the state of the keys and buttons held.
#[derive(Debug)]
pub struct Keymap {
    groups: Vec<&'static ActionGroup>,
    entries: Vec<Entry>,
    by_id: HashMap<&'static str, usize>,
    by_context: HashMap<Context, Vec<usize>>,
    buttons: Vec<ButtonPress>,
    keys: Vec<KeyPress>,
    last_key_press: Option<(Key, Instant)>,
    /// The last press of a mouse button, when and where: the first half of a double click.
    last_button_press: Option<(MouseButton, Instant, Vec2)>,
    pointer: Vec2,
    /// Counts the changes of shortcuts: what an editor compares to know it is up to date.
    revision: u64,
}

impl Keymap {
    /// The keymap of `groups`, with the default shortcuts.
    ///
    /// # Errors
    ///
    /// [`KeymapError::DuplicateAction`] when two actions share an identifier,
    /// [`KeymapError::Conflict`] when two defaults of one context overlap.
    pub fn new(groups: &[&'static ActionGroup]) -> Result<Self, KeymapError> {
        let mut keymap = Self {
            groups: groups.to_vec(),
            entries: Vec::new(),
            by_id: HashMap::new(),
            by_context: HashMap::new(),
            buttons: Vec::new(),
            keys: Vec::new(),
            last_key_press: None,
            last_button_press: None,
            pointer: Vec2::ZERO,
            revision: 0,
        };
        for action in groups
            .iter()
            .flat_map(|group| group.actions.iter().copied())
        {
            let index = keymap.entries.len();
            if keymap.by_id.insert(action.id, index).is_some() {
                return Err(KeymapError::DuplicateAction(action.id));
            }
            keymap
                .by_context
                .entry(action.context)
                .or_default()
                .push(index);
            keymap.entries.push(Entry {
                action,
                bindings: Vec::new(),
                shortcuts: Vec::new(),
            });
        }
        for index in 0..keymap.entries.len() {
            let action = keymap.entries[index].action;
            if !action.repeats && action.defaults.iter().any(|shortcut| shortcut.repeats()) {
                return Err(KeymapError::Repeats(action.id));
            }
            keymap.assign(index, &on(action.defaults))?;
        }
        Ok(keymap)
    }

    /// How many times shortcuts changed: what an editor compares to know it is up to date.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    // --- Actions and their shortcuts -------------------------------------------------------

    /// The groups, in registration order.
    pub fn groups(&self) -> &[&'static ActionGroup] {
        &self.groups
    }

    /// The action with identifier `id`.
    pub fn action(&self, id: &str) -> Option<&'static ActionDef> {
        self.by_id.get(id).map(|&index| self.entries[index].action)
    }

    fn index(&self, id: &str) -> Result<usize, KeymapError> {
        self.by_id
            .get(id)
            .copied()
            .ok_or_else(|| KeymapError::UnknownAction(id.to_owned()))
    }

    /// The shortcuts of the action `id` that are on.
    pub fn shortcuts(&self, id: &str) -> Option<&[Shortcut]> {
        self.by_id
            .get(id)
            .map(|&index| self.entries[index].shortcuts.as_slice())
    }

    /// The shortcuts of the action `id`, on and off, in order.
    pub fn bindings(&self, id: &str) -> Option<&[Binding]> {
        self.by_id
            .get(id)
            .map(|&index| self.entries[index].bindings.as_slice())
    }

    /// Every action and its bindings, in registration order.
    pub fn actions(&self) -> impl Iterator<Item = (&'static ActionDef, &[Binding])> {
        self.entries
            .iter()
            .map(|entry| (entry.action, entry.bindings.as_slice()))
    }

    /// The other actions of `id`'s context that `shortcut` would conflict with.
    ///
    /// # Errors
    ///
    /// [`KeymapError::UnknownAction`].
    pub fn conflicts(
        &self,
        id: &str,
        shortcut: Shortcut,
    ) -> Result<Vec<&'static ActionDef>, KeymapError> {
        let index = self.index(id)?;
        let context = self.entries[index].action.context;
        Ok(self.by_context[&context]
            .iter()
            .filter(|&&other| other != index)
            .filter(|&&other| {
                let entry = &self.entries[other];
                !entry.action.excludes(self.entries[index].action)
                    && entry
                        .shortcuts
                        .iter()
                        .any(|existing| existing.overlaps(shortcut))
            })
            .map(|&other| self.entries[other].action)
            .collect())
    }

    /// Checks the bindings that are on for the action at `index` against the other actions
    /// of its context, then assigns them all, without duplicates.
    fn assign(&mut self, index: usize, bindings: &[Binding]) -> Result<(), KeymapError> {
        let action = self.entries[index].action;
        for binding in bindings.iter().filter(|binding| binding.enabled) {
            let shortcut = binding.shortcut;
            if let Some(&other) = self.by_context[&action.context].iter().find(|&&other| {
                other != index
                    && !self.entries[other].action.excludes(action)
                    && self.entries[other]
                        .shortcuts
                        .iter()
                        .any(|existing| existing.overlaps(shortcut))
            }) {
                return Err(KeymapError::Conflict {
                    action: action.id,
                    shortcut,
                    with: self.entries[other].action.id,
                });
            }
        }
        self.entries[index].set(bindings.iter().copied());
        self.revision += 1;
        Ok(())
    }

    /// Replaces the shortcuts of the action `id`, all on.
    ///
    /// # Errors
    ///
    /// [`KeymapError::UnknownAction`], or [`KeymapError::Conflict`] when a shortcut is used
    /// by another action of the same context; nothing changes then.
    pub fn set_shortcuts(&mut self, id: &str, shortcuts: &[Shortcut]) -> Result<(), KeymapError> {
        let index = self.index(id)?;
        self.assign(index, &on(shortcuts))
    }

    /// Replaces the shortcuts of the action `id`, each on or off: only those on are checked
    /// against the other actions of its context.
    ///
    /// # Errors
    ///
    /// See [`Self::set_shortcuts`].
    pub fn set_bindings(&mut self, id: &str, bindings: &[Binding]) -> Result<(), KeymapError> {
        let index = self.index(id)?;
        self.assign(index, bindings)
    }

    /// Adds a shortcut to the action `id`, on.
    ///
    /// # Errors
    ///
    /// See [`Self::set_shortcuts`].
    pub fn add_shortcut(&mut self, id: &str, shortcut: Shortcut) -> Result<(), KeymapError> {
        let index = self.index(id)?;
        let mut bindings = self.entries[index].bindings.clone();
        bindings.push(Binding::on(shortcut));
        self.assign(index, &bindings)
    }

    /// Removes a shortcut from the action `id`, on or off. Returns whether it had it.
    ///
    /// # Errors
    ///
    /// [`KeymapError::UnknownAction`].
    pub fn remove_shortcut(&mut self, id: &str, shortcut: Shortcut) -> Result<bool, KeymapError> {
        let index = self.index(id)?;
        let entry = &mut self.entries[index];
        let before = entry.bindings.len();
        let kept: Vec<Binding> = entry
            .bindings
            .iter()
            .copied()
            .filter(|binding| binding.shortcut != shortcut)
            .collect();
        entry.set(kept);
        let removed = entry.bindings.len() != before;
        if removed {
            self.revision += 1;
        }
        Ok(removed)
    }

    /// `true` when the shortcuts of the action `id` differ from its defaults.
    pub fn is_modified(&self, id: &str) -> bool {
        self.by_id
            .get(id)
            .is_some_and(|&index| self.entries[index].is_modified())
    }

    /// The actions whose shortcuts differ from their defaults, in registration order.
    pub fn modified(&self) -> impl Iterator<Item = &'static ActionDef> + '_ {
        self.entries
            .iter()
            .filter(|entry| entry.is_modified())
            .map(|entry| entry.action)
    }

    /// Restores the default shortcuts of the actions at `indices`, all or none.
    fn reset_indices(&mut self, indices: &[usize]) -> Result<(), KeymapError> {
        let saved: Vec<Vec<Binding>> = indices
            .iter()
            .map(|&index| {
                let bindings = self.entries[index].bindings.clone();
                self.entries[index].set([]);
                bindings
            })
            .collect();
        for &index in indices {
            let defaults = on(self.entries[index].action.defaults);
            if let Err(error) = self.assign(index, &defaults) {
                for (&index, bindings) in indices.iter().zip(saved) {
                    self.entries[index].set(bindings);
                }
                return Err(error);
            }
        }
        Ok(())
    }

    /// Restores the default shortcuts of the action `id`.
    ///
    /// # Errors
    ///
    /// [`KeymapError::UnknownAction`], or [`KeymapError::Conflict`] when another action of
    /// the context took one of the defaults meanwhile; nothing changes then.
    pub fn reset(&mut self, id: &str) -> Result<(), KeymapError> {
        let index = self.index(id)?;
        self.reset_indices(&[index])
    }

    /// Restores the default shortcuts of every action of the group `name`.
    ///
    /// # Errors
    ///
    /// [`KeymapError::UnknownGroup`], or a [`KeymapError::Conflict`] with an action of
    /// another group; nothing changes then.
    pub fn reset_group(&mut self, name: &str) -> Result<(), KeymapError> {
        let group = self
            .groups
            .iter()
            .find(|group| group.name == name)
            .ok_or_else(|| KeymapError::UnknownGroup(name.to_owned()))?;
        let indices: Vec<usize> = group
            .actions
            .iter()
            .map(|action| self.by_id[action.id])
            .collect();
        self.reset_indices(&indices)
    }

    /// Restores every default shortcut.
    pub fn reset_all(&mut self) {
        for entry in &mut self.entries {
            entry.restore();
        }
        self.revision += 1;
    }

    // --- Input ----------------------------------------------------------------------------

    /// Forgets the keys and buttons held, so that their release triggers nothing: call it
    /// when an action starts an operation that takes over the input (a drag that resizes).
    pub fn cancel_presses(&mut self) {
        self.buttons.clear();
        self.keys.clear();
    }

    /// Turns `event` into the actions it triggers. `chain` lists the contexts under the
    /// pointer, innermost first; `now` dates the event (for key double clicks).
    pub fn handle(
        &mut self,
        event: InputEvent,
        chain: &[Context],
        settings: &InputSettings,
        now: Instant,
    ) -> Triggers {
        let mut triggers = Triggers::default();
        match event {
            InputEvent::Key {
                key,
                down: true,
                repeat,
                modifiers,
            } => self.key_down(key, repeat, modifiers, chain, settings, now, &mut triggers),
            InputEvent::Key {
                key,
                down: false,
                modifiers,
                ..
            } => {
                let pressed = self
                    .keys
                    .iter()
                    .position(|press| press.key == key)
                    .map(|index| self.keys.swap_remove(index));
                let chain = pressed
                    .as_ref()
                    .map_or(chain, |press| press.chain.as_slice());
                for when in [KeyTrigger::Release, KeyTrigger::Click] {
                    if when == KeyTrigger::Click && pressed.is_none() {
                        continue;
                    }
                    let kind = GestureKind::Key {
                        key,
                        when,
                        repeat: false,
                    };
                    self.resolve(self.gesture(kind, modifiers), chain, &mut triggers);
                }
            }
            InputEvent::Button {
                button,
                down: true,
                position,
                modifiers,
                ..
            } => {
                self.pointer = position;
                self.buttons.retain(|press| press.button != button);
                let held = self.held_key(None);
                self.buttons.push(ButtonPress {
                    button,
                    origin: position,
                    modifiers,
                    held,
                    moved: false,
                    dragged: false,
                    chain: chain.to_vec(),
                });
                let kind = GestureKind::Button {
                    button,
                    when: ButtonTrigger::Press,
                };
                self.resolve(self.gesture(kind, modifiers), chain, &mut triggers);
                self.double_click(
                    button,
                    position,
                    modifiers,
                    chain,
                    settings,
                    now,
                    &mut triggers,
                );
            }
            InputEvent::Button {
                button,
                down: false,
                position,
                modifiers,
                ..
            } => {
                self.pointer = position;
                let pressed = self
                    .buttons
                    .iter()
                    .position(|press| press.button == button)
                    .map(|index| self.buttons.swap_remove(index));
                let (origin, held, key, moved) = pressed
                    .as_ref()
                    .map_or((position, modifiers, self.held_key(None), true), |press| {
                        (press.origin, press.modifiers, press.held, press.moved)
                    });
                let chain = pressed
                    .as_ref()
                    .map_or(chain, |press| press.chain.as_slice());
                for when in [ButtonTrigger::Release, ButtonTrigger::Click] {
                    if when == ButtonTrigger::Click && moved {
                        continue;
                    }
                    let gesture = Gesture {
                        kind: GestureKind::Button { button, when },
                        position,
                        origin,
                        modifiers: held,
                        held: key,
                    };
                    self.resolve(gesture, chain, &mut triggers);
                }
            }
            InputEvent::PointerMove {
                position,
                modifiers,
            } => {
                self.pointer = position;
                self.drags(position, settings, &mut triggers);
                self.resolve(
                    self.gesture(GestureKind::PointerMove, modifiers),
                    chain,
                    &mut triggers,
                );
            }
            InputEvent::Wheel {
                delta,
                position,
                modifiers,
            } => {
                self.pointer = position;
                let steps = [
                    (delta.y > 0.0, WheelDirection::Up, delta.y),
                    (delta.y < 0.0, WheelDirection::Down, -delta.y),
                    (delta.x > 0.0, WheelDirection::Right, delta.x),
                    (delta.x < 0.0, WheelDirection::Left, -delta.x),
                ];
                for (turned, direction, amount) in steps {
                    if turned {
                        let kind = GestureKind::Wheel { direction, amount };
                        self.resolve(self.gesture(kind, modifiers), chain, &mut triggers);
                    }
                }
            }
            InputEvent::Pinch {
                scale,
                position,
                modifiers,
            } => {
                self.pointer = position;
                let direction = if scale > 1.0 {
                    Some(PinchDirection::In)
                } else if scale < 1.0 {
                    Some(PinchDirection::Out)
                } else {
                    None
                };
                if let Some(direction) = direction {
                    let kind = GestureKind::Pinch { direction, scale };
                    self.resolve(self.gesture(kind, modifiers), chain, &mut triggers);
                }
            }
            InputEvent::FocusLost => {
                self.cancel_presses();
                self.last_key_press = None;
            }
        }
        triggers
    }

    /// The double clicks a press of `button` at `position` makes: the second press near the
    /// first, within each shortcut's double-click time (its own, or the settings'). The
    /// platform's count of clicks plays no part: the time is the application's.
    #[allow(
        clippy::too_many_arguments,
        reason = "a private step of `handle`, which owns all these values"
    )]
    fn double_click(
        &mut self,
        button: MouseButton,
        position: Vec2,
        modifiers: Modifiers,
        chain: &[Context],
        settings: &InputSettings,
        now: Instant,
        triggers: &mut Triggers,
    ) {
        let interval = self
            .last_button_press
            .filter(|(last, _, at)| {
                *last == button && (position - *at).length() <= settings.drag_threshold
            })
            .map(|(_, at, _)| now.saturating_duration_since(at));
        self.last_button_press = Some((button, now, position));
        let Some(interval) = interval else {
            return;
        };
        let millis = u16::try_from(interval.as_millis()).unwrap_or(u16::MAX);
        let kind = GestureKind::Button {
            button,
            when: ButtonTrigger::DoubleClick(DoubleClick { time: Some(millis) }),
        };
        let gesture = self.gesture(kind, modifiers);
        let held = gesture.held;
        let found = self.resolve_with(gesture, chain, triggers, |shortcut| {
            shortcut.matches_double_click(
                button,
                modifiers,
                held,
                interval,
                settings.double_click_time,
            )
        });
        if found {
            // A third press starts again.
            self.last_button_press = None;
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "a private step of `handle`, which owns all these values"
    )]
    fn key_down(
        &mut self,
        key: Key,
        repeat: bool,
        modifiers: Modifiers,
        chain: &[Context],
        settings: &InputSettings,
        now: Instant,
        triggers: &mut Triggers,
    ) {
        if !repeat {
            self.keys.retain(|press| press.key != key);
            self.keys.push(KeyPress {
                key,
                chain: chain.to_vec(),
            });
        }
        let press = GestureKind::Key {
            key,
            when: KeyTrigger::Press,
            repeat,
        };
        self.resolve(self.gesture(press, modifiers), chain, triggers);
        if repeat {
            return;
        }
        let double = self.last_key_press.is_some_and(|(last, at)| {
            last == key && now.saturating_duration_since(at) <= settings.double_click_time
        });
        if double {
            self.last_key_press = None;
            let kind = GestureKind::Key {
                key,
                when: KeyTrigger::DoubleClick,
                repeat: false,
            };
            self.resolve(self.gesture(kind, modifiers), chain, triggers);
        } else {
            self.last_key_press = Some((key, now));
        }
    }

    /// Starts the drags of the buttons held that moved far enough.
    fn drags(&mut self, position: Vec2, settings: &InputSettings, triggers: &mut Triggers) {
        let Self {
            buttons,
            entries,
            by_context,
            ..
        } = self;
        for press in buttons.iter_mut().filter(|press| !press.dragged) {
            let delta = position - press.origin;
            if delta.length() >= settings.drag_threshold {
                press.moved = true;
            }
            let found = press.chain.iter().find_map(|context| {
                by_context.get(context)?.iter().find_map(|&index| {
                    let entry = &entries[index];
                    entry
                        .shortcuts
                        .iter()
                        .any(|shortcut| {
                            shortcut.matches_drag(
                                press.button,
                                press.modifiers,
                                press.held,
                                delta,
                                settings.drag_threshold,
                            )
                        })
                        .then_some(entry.action)
                })
            });
            if let Some(action) = found {
                press.moved = true;
                press.dragged = true;
                triggers.push(Triggered {
                    action,
                    gesture: Gesture {
                        kind: GestureKind::Button {
                            button: press.button,
                            when: ButtonTrigger::Drag(Drag {
                                direction: DragDirection::of(delta.x, delta.y),
                                distance: None,
                            }),
                        },
                        position,
                        origin: press.origin,
                        modifiers: press.modifiers,
                        held: press.held,
                    },
                });
            }
        }
    }

    fn gesture(&self, kind: GestureKind, modifiers: Modifiers) -> Gesture {
        let own = match kind {
            GestureKind::Key { key, .. } => Some(key),
            _ => None,
        };
        Gesture {
            kind,
            position: self.pointer,
            origin: self.pointer,
            modifiers,
            held: self.held_key(own),
        }
    }

    /// The last key pressed and still held that is not a modifier, other than `except`.
    fn held_key(&self, except: Option<Key>) -> Option<Key> {
        self.keys
            .iter()
            .rev()
            .map(|press| press.key)
            .find(|&key| !key.is_modifier() && Some(key) != except)
    }

    /// Triggers the actions of the first context of the chain with a shortcut matching
    /// `gesture`.
    fn resolve(&self, gesture: Gesture, chain: &[Context], triggers: &mut Triggers) {
        self.resolve_with(gesture, chain, triggers, |shortcut| {
            shortcut.matches(&gesture)
        });
    }

    /// Triggers the actions of the first context of the chain with a shortcut `matching`:
    /// in each context, shortcuts holding a key come before those that do not. Several actions
    /// of one context share a shortcut only when their conditions exclude each other: each is
    /// triggered, and acts when its condition holds. Returns whether one was.
    fn resolve_with(
        &self,
        gesture: Gesture,
        chain: &[Context],
        triggers: &mut Triggers,
        matching: impl Fn(&Shortcut) -> bool,
    ) -> bool {
        for context in chain {
            let Some(indices) = self.by_context.get(context) else {
                continue;
            };
            for holding in [true, false] {
                let mut found = false;
                for &index in indices {
                    let entry = &self.entries[index];
                    let matched = entry
                        .shortcuts
                        .iter()
                        .any(|shortcut| shortcut.held.is_some() == holding && matching(shortcut));
                    if matched {
                        triggers.push(Triggered {
                            action: entry.action,
                            gesture,
                        });
                        found = true;
                    }
                }
                if found {
                    return true;
                }
            }
        }
        false
    }

    // --- Saving ---------------------------------------------------------------------------

    /// Assigns bindings without checking conflicts (loading), returning the index.
    pub(crate) fn assign_unchecked(&mut self, id: &str, bindings: Vec<Binding>) -> Option<usize> {
        let index = *self.by_id.get(id)?;
        self.entries[index].set(bindings);
        self.revision += 1;
        Some(index)
    }

    /// The first pair of actions whose shortcuts conflict, as `(earlier, later)` indices.
    pub(crate) fn first_conflict(&self) -> Option<(usize, usize, Shortcut)> {
        self.by_context.values().find_map(|indices| {
            indices.iter().enumerate().find_map(|(position, &a)| {
                indices[position + 1..].iter().find_map(|&b| {
                    if self.entries[a].action.excludes(self.entries[b].action) {
                        return None;
                    }
                    self.entries[a].shortcuts.iter().find_map(|first| {
                        self.entries[b]
                            .shortcuts
                            .iter()
                            .any(|second| first.overlaps(*second))
                            .then_some((a, b, *first))
                    })
                })
            })
        })
    }

    /// Restores the defaults of the action at `index`, without checks (loading).
    pub(crate) fn restore_unchecked(&mut self, index: usize) {
        self.entries[index].restore();
        self.revision += 1;
    }

    /// The action at `index`.
    pub(crate) fn action_at(&self, index: usize) -> &'static ActionDef {
        self.entries[index].action
    }
}

/// `shortcuts`, all on.
fn on(shortcuts: &[Shortcut]) -> Vec<Binding> {
    shortcuts.iter().copied().map(Binding::on).collect()
}

/// `shortcut` without repeating: what an action that does not repeat keeps of it.
const fn once(shortcut: Shortcut) -> Shortcut {
    match shortcut.trigger {
        Trigger::Key { key, when, .. } => Shortcut {
            trigger: Trigger::Key {
                key,
                when,
                repeat: false,
            },
            ..shortcut
        },
        _ => shortcut,
    }
}
