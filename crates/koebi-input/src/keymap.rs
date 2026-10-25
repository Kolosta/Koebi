//! [`Keymap`]: the shortcuts of every action, and the engine that turns input into actions.
//!
//! - **Resolution**: each input event becomes gestures (press, release, click, double click,
//!   drag, wheel, pinch); each gesture is looked up in the chain of contexts the caller gives
//!   (the zones under the pointer, innermost first); the first context with a matching
//!   shortcut wins. A button or key held keeps the chain of its press: a drag that starts on
//!   a gutter stays the gutter's, wherever it goes.
//! - **Conflicts**: two actions of one context never share a shortcut that one input could
//!   trigger twice ([`Shortcut::overlaps`]); every change is checked.
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
    ButtonTrigger, Drag, DragDirection, KeyTrigger, PinchDirection, Shortcut, WheelDirection,
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
    const CAPACITY: usize = 4;

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
    shortcuts: Vec<Shortcut>,
}

impl Entry {
    fn is_modified(&self) -> bool {
        self.shortcuts.len() != self.action.defaults.len()
            || self
                .shortcuts
                .iter()
                .any(|shortcut| !self.action.defaults.contains(shortcut))
    }
}

/// A mouse button held down.
#[derive(Debug)]
struct ButtonPress {
    button: MouseButton,
    origin: Vec2,
    modifiers: Modifiers,
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
    pointer: Vec2,
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
            pointer: Vec2::ZERO,
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
                shortcuts: Vec::new(),
            });
        }
        for index in 0..keymap.entries.len() {
            let action = keymap.entries[index].action;
            keymap.assign(index, action.defaults)?;
        }
        Ok(keymap)
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

    /// The current shortcuts of the action `id`.
    pub fn shortcuts(&self, id: &str) -> Option<&[Shortcut]> {
        self.by_id
            .get(id)
            .map(|&index| self.entries[index].shortcuts.as_slice())
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
                self.entries[other]
                    .shortcuts
                    .iter()
                    .any(|existing| existing.overlaps(shortcut))
            })
            .map(|&other| self.entries[other].action)
            .collect())
    }

    /// Checks `shortcuts` for the action at `index` against the other actions of its
    /// context, then assigns them, without duplicates.
    fn assign(&mut self, index: usize, shortcuts: &[Shortcut]) -> Result<(), KeymapError> {
        let action = self.entries[index].action;
        let mut unique: Vec<Shortcut> = Vec::with_capacity(shortcuts.len());
        for shortcut in shortcuts {
            if unique.contains(shortcut) {
                continue;
            }
            if let Some(&other) = self.by_context[&action.context].iter().find(|&&other| {
                other != index
                    && self.entries[other]
                        .shortcuts
                        .iter()
                        .any(|existing| existing.overlaps(*shortcut))
            }) {
                return Err(KeymapError::Conflict {
                    action: action.id,
                    shortcut: *shortcut,
                    with: self.entries[other].action.id,
                });
            }
            unique.push(*shortcut);
        }
        self.entries[index].shortcuts = unique;
        Ok(())
    }

    /// Replaces the shortcuts of the action `id`.
    ///
    /// # Errors
    ///
    /// [`KeymapError::UnknownAction`], or [`KeymapError::Conflict`] when a shortcut is used
    /// by another action of the same context; nothing changes then.
    pub fn set_shortcuts(&mut self, id: &str, shortcuts: &[Shortcut]) -> Result<(), KeymapError> {
        let index = self.index(id)?;
        self.assign(index, shortcuts)
    }

    /// Adds a shortcut to the action `id`.
    ///
    /// # Errors
    ///
    /// See [`Self::set_shortcuts`].
    pub fn add_shortcut(&mut self, id: &str, shortcut: Shortcut) -> Result<(), KeymapError> {
        let index = self.index(id)?;
        let mut shortcuts = self.entries[index].shortcuts.clone();
        shortcuts.push(shortcut);
        self.assign(index, &shortcuts)
    }

    /// Removes a shortcut from the action `id`. Returns whether it had it.
    ///
    /// # Errors
    ///
    /// [`KeymapError::UnknownAction`].
    pub fn remove_shortcut(&mut self, id: &str, shortcut: Shortcut) -> Result<bool, KeymapError> {
        let index = self.index(id)?;
        let shortcuts = &mut self.entries[index].shortcuts;
        let before = shortcuts.len();
        shortcuts.retain(|existing| *existing != shortcut);
        Ok(shortcuts.len() != before)
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
        let saved: Vec<Vec<Shortcut>> = indices
            .iter()
            .map(|&index| core::mem::take(&mut self.entries[index].shortcuts))
            .collect();
        for &index in indices {
            let defaults = self.entries[index].action.defaults;
            if let Err(error) = self.assign(index, defaults) {
                for (&index, shortcuts) in indices.iter().zip(saved) {
                    self.entries[index].shortcuts = shortcuts;
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
            entry.shortcuts = entry.action.defaults.to_vec();
        }
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
                clicks,
                position,
                modifiers,
            } => {
                self.pointer = position;
                self.buttons.retain(|press| press.button != button);
                self.buttons.push(ButtonPress {
                    button,
                    origin: position,
                    modifiers,
                    moved: false,
                    dragged: false,
                    chain: chain.to_vec(),
                });
                let whens = [
                    (true, ButtonTrigger::Press),
                    (clicks >= 2, ButtonTrigger::DoubleClick),
                ];
                for (_, when) in whens.into_iter().filter(|(happened, _)| *happened) {
                    let kind = GestureKind::Button { button, when };
                    self.resolve(self.gesture(kind, modifiers), chain, &mut triggers);
                }
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
                let (origin, held, moved) = pressed
                    .as_ref()
                    .map_or((position, modifiers, true), |press| {
                        (press.origin, press.modifiers, press.moved)
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
                    },
                });
            }
        }
    }

    fn gesture(&self, kind: GestureKind, modifiers: Modifiers) -> Gesture {
        Gesture {
            kind,
            position: self.pointer,
            origin: self.pointer,
            modifiers,
        }
    }

    /// Triggers the first action of the chain with a shortcut matching `gesture`.
    fn resolve(&self, gesture: Gesture, chain: &[Context], triggers: &mut Triggers) {
        let found = chain.iter().find_map(|context| {
            self.by_context.get(context)?.iter().find_map(|&index| {
                let entry = &self.entries[index];
                entry
                    .shortcuts
                    .iter()
                    .any(|shortcut| shortcut.matches(&gesture))
                    .then_some(entry.action)
            })
        });
        if let Some(action) = found {
            triggers.push(Triggered { action, gesture });
        }
    }

    // --- Saving ---------------------------------------------------------------------------

    pub(crate) fn entries(&self) -> impl Iterator<Item = (&'static ActionDef, &[Shortcut])> {
        self.entries
            .iter()
            .map(|entry| (entry.action, entry.shortcuts.as_slice()))
    }

    /// Assigns shortcuts without checking conflicts (loading), returning the index.
    pub(crate) fn assign_unchecked(&mut self, id: &str, shortcuts: Vec<Shortcut>) -> Option<usize> {
        let index = *self.by_id.get(id)?;
        let mut unique = Vec::with_capacity(shortcuts.len());
        for shortcut in shortcuts {
            if !unique.contains(&shortcut) {
                unique.push(shortcut);
            }
        }
        self.entries[index].shortcuts = unique;
        Some(index)
    }

    /// The first pair of actions whose shortcuts conflict, as `(earlier, later)` indices.
    pub(crate) fn first_conflict(&self) -> Option<(usize, usize, Shortcut)> {
        self.by_context.values().find_map(|indices| {
            indices.iter().enumerate().find_map(|(position, &a)| {
                indices[position + 1..].iter().find_map(|&b| {
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
        self.entries[index].shortcuts = self.entries[index].action.defaults.to_vec();
    }

    /// The action at `index`.
    pub(crate) fn action_at(&self, index: usize) -> &'static ActionDef {
        self.entries[index].action
    }
}
