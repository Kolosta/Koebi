//! The Keymaps page: the shortcuts of every action, by zone, as nested expandable panels.
//!
//! - **Zones**: the application holds the workspace, which holds the editors; lists lie in any
//!   zone; menus, fields being edited and operations take the input while they last
//!   ([`crate::actions::CONTEXTS`]). Each zone shows its actions, by feature when it holds
//!   several, then the zones inside it.
//! - **Actions**: the header shows the action's first shortcut and, once its shortcuts differ
//!   from the defaults, the dot bringing them back. The panel holds each shortcut, then a field
//!   adding one.
//! - **Shortcuts**: the header turns the shortcut on or off, chooses its device and records it;
//!   the panel tells its conflicts and edits it in detail ([`binding`]).
//! - **Conflicts**, checked as soon as a shortcut changes ([`analysis`]): the same shortcut in
//!   one zone is a hard conflict, and the newcomer stays off until one of them changes; a zone
//!   inside another taking the shortcut, or a similar shortcut, is a soft one. A shortcut in
//!   conflict tints its header, and every panel holding it shows the warning in the conflict's
//!   color.
//! - **Search** by name, or by pressing a shortcut: only the actions found show.

mod analysis;
mod binding;
mod rules;

use std::collections::HashMap;

use koebi_core::{Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_input::{ActionDef, Binding, Conflict, Context, Keymap, Shortcut, Within};
use koebi_tokens::semantic;
use koebi_ui::{
    Align, Button, Capture, CaptureField, CaptureState, CollapsiblePanel, CollapsiblePanelState,
    Icon, TextField, Ui, WidgetId,
};

use self::analysis::{Analysis, contains_folded, finds};
use self::binding::{BindingUi, Choices, Edit, conflict_color};
use super::Stack;
use crate::actions::{CONTEXTS, ZONES};

const ID: WidgetId = WidgetId::new("settings-keymaps");

/// How strong a conflict's tint is over a header.
const TINT: f32 = 0.22;

/// The room the header of an action keeps for its first shortcut and its reset dot.
const ACTION_TRAILING: f32 = 220.0;

/// The room the header of a shortcut gives its controls: all of it after the chevron.
const BINDING_TRAILING: f32 = 1.0e6;

/// What an action of the page keeps between frames.
#[derive(Debug)]
struct ActionUi {
    panel: CollapsiblePanelState,
    bindings: Vec<BindingUi>,
    add: CaptureState,
}

impl ActionUi {
    fn new() -> Self {
        Self {
            panel: CollapsiblePanelState::new(false),
            bindings: Vec::new(),
            add: CaptureState::new(),
        }
    }
}

/// What the search found, and what it looked for.
#[derive(Debug, Default)]
struct Found {
    text: String,
    shortcut: Option<Shortcut>,
    revision: Option<u64>,
    /// The actions found; `None` without a search.
    actions: Option<Vec<&'static str>>,
}

impl Found {
    /// `true` when the action `id` shows.
    fn shows(&self, id: &str) -> bool {
        self.actions
            .as_ref()
            .is_none_or(|actions| actions.contains(&id))
    }
}

/// The Keymaps page's state.
#[derive(Debug)]
pub(super) struct KeymapsPage {
    search: String,
    search_capture: CaptureState,
    search_shortcut: Option<Shortcut>,
    found: Found,
    zones: HashMap<Context, CollapsiblePanelState>,
    features: HashMap<(Context, &'static str), CollapsiblePanelState>,
    actions: HashMap<&'static str, ActionUi>,
    analysis: Analysis,
    choices: Choices,
    /// Reused to check an edit.
    conflicts: Vec<Conflict>,
}

impl KeymapsPage {
    pub(super) fn new() -> Self {
        Self {
            search: String::new(),
            search_capture: CaptureState::new(),
            search_shortcut: None,
            found: Found::default(),
            zones: HashMap::new(),
            features: HashMap::new(),
            actions: HashMap::new(),
            analysis: Analysis::default(),
            choices: Choices::new(),
            conflicts: Vec::new(),
        }
    }

    pub(super) fn show(
        &mut self,
        ui: &mut Ui<'_>,
        stack: &mut Stack,
        design: &mut DesignSystem,
        keymap: &mut Keymap,
    ) {
        self.analysis.refresh(keymap, &ZONES);
        self.search_row(ui, stack, design);
        self.refresh_search(keymap);
        let bounds = stack.bounds;
        let roots = CONTEXTS
            .iter()
            .filter(|info| matches!(info.within, Within::Root | Within::AnyZone));
        for info in roots {
            let context = info.context;
            if !self.has(keymap, context) {
                continue;
            }
            // Out of the page while the panel holds it: its content works on the rest.
            let mut state = self
                .zones
                .remove(&context)
                .unwrap_or_else(|| CollapsiblePanelState::new(context == crate::actions::APP));
            let mut panel =
                CollapsiblePanel::new(ID.with(context_key(context)), &mut state, info.label);
            if let Some(worst) = self.analysis.zone(context) {
                panel = panel.status(Icon::Warning, conflict_color(design, worst));
            }
            stack.panel(ui, panel, |ui, rect| {
                self.zone_content(ui, rect, context, design, keymap, bounds)
            });
            self.zones.insert(context, state);
        }
    }

    /// Opens the panel of the action `id` and those of its shortcuts at `bindings`.
    #[cfg(test)]
    pub(super) fn open_action(&mut self, id: &'static str, bindings: &[usize]) {
        let state = self.actions.entry(id).or_insert_with(ActionUi::new);
        state.panel.set_open(true);
        for &index in bindings {
            if state.bindings.len() <= index {
                state.bindings.resize_with(index + 1, BindingUi::new);
            }
            state.bindings[index].panel.set_open(true);
        }
    }

    /// The search: by name, by shortcut, and what clears them.
    fn search_row(&mut self, ui: &mut Ui<'_>, stack: &mut Stack, design: &DesignSystem) {
        let row = design.get(semantic::SIZE_CONTROL_HEIGHT);
        let gap = design.get(semantic::SPACING_EDITOR_INSET) * 0.5;
        let rect = Rect::from_min_max(
            Vec2::new(stack.left, stack.y),
            Vec2::new(stack.right, stack.y + row),
        );
        let clear = Rect::from_min_max(Vec2::new(rect.max.x - row, rect.min.y), rect.max);
        let half = ((clear.min.x - gap - rect.min.x - gap) * 0.5).max(0.0);
        let name = Rect::from_origin_size(rect.min, Vec2::new(half, row));
        TextField::new(ID.with(1), &mut self.search)
            .placeholder("Search by name")
            .show(ui, name);
        let by_shortcut = Rect::from_origin_size(
            Vec2::new(name.max.x + gap, rect.min.y),
            Vec2::new(half, row),
        );
        let recorded =
            CaptureField::new(ID.with(2), &mut self.search_capture, self.search_shortcut)
                .capture(Capture::Any)
                .hints("Search by shortcut", "Press the shortcut to find")
                .show(ui, by_shortcut);
        if let Some(shortcut) = recorded.captured {
            self.search_shortcut = Some(shortcut);
        }
        let searching = !self.search.is_empty() || self.search_shortcut.is_some();
        if searching
            && Button::new(ID.with(3))
                .icon(Icon::Close)
                .show(ui, clear)
                .clicked
        {
            self.search.clear();
            self.search_shortcut = None;
        }
        ui.tooltip(ID.with(3), "Clear the search", clear);
        stack.y += row + stack.gap;
    }

    /// Finds the actions again when the search or the keymap changed: those found open their
    /// zones.
    fn refresh_search(&mut self, keymap: &Keymap) {
        let found = &self.found;
        if found.text == self.search
            && found.shortcut == self.search_shortcut
            && found.revision == Some(keymap.revision())
        {
            return;
        }
        let changed = found.text != self.search || found.shortcut != self.search_shortcut;
        self.found.text.clone_from(&self.search);
        self.found.shortcut = self.search_shortcut;
        self.found.revision = Some(keymap.revision());
        if self.search.is_empty() && self.search_shortcut.is_none() {
            self.found.actions = None;
            return;
        }
        let text = self.search.trim();
        let actions: Vec<&'static str> = keymap
            .actions()
            .filter(|(action, bindings)| {
                let named = text.is_empty()
                    || contains_folded(action.label, text)
                    || contains_folded(action.id, text)
                    || contains_folded(action.description, text);
                let pressed = self.search_shortcut.is_none_or(|shortcut| {
                    bindings
                        .iter()
                        .any(|binding| finds(shortcut, binding.shortcut))
                });
                named && pressed
            })
            .map(|(action, _)| action.id)
            .collect();
        if changed {
            // What was found shows: its zones and its feature open.
            for id in &actions {
                let Some(action) = keymap.action(id) else {
                    continue;
                };
                let mut context = Some(action.context);
                while let Some(current) = context {
                    self.zones
                        .entry(current)
                        .or_insert_with(|| CollapsiblePanelState::new(true))
                        .set_open(true);
                    context = ZONES.parent(current);
                }
                for state in self
                    .features
                    .iter_mut()
                    .filter(|((context, _), _)| *context == action.context)
                    .map(|(_, state)| state)
                {
                    state.set_open(true);
                }
            }
        }
        self.found.actions = Some(actions);
    }

    /// `true` when `context` or a zone inside it shows an action.
    fn has(&self, keymap: &Keymap, context: Context) -> bool {
        keymap
            .actions()
            .any(|(action, _)| action.context == context && self.found.shows(action.id))
            || CONTEXTS
                .iter()
                .filter(|info| info.within == Within::Zone(context))
                .any(|info| self.has(keymap, info.context))
    }

    /// What a zone holds: its actions, by feature when there are several, then the zones
    /// inside it. Returns the height used.
    fn zone_content(
        &mut self,
        ui: &mut Ui<'_>,
        rect: Rect,
        context: Context,
        design: &mut DesignSystem,
        keymap: &mut Keymap,
        bounds: Rect,
    ) -> f32 {
        let mut y = rect.min.y;
        let gap = ui.styles().collapsible_panel.gap;
        let groups = keymap.groups();
        let features: Vec<&'static koebi_input::ActionGroup> = groups
            .iter()
            .copied()
            .filter(|group| {
                group
                    .actions
                    .iter()
                    .any(|action| action.context == context && self.found.shows(action.id))
            })
            .collect();
        let several = features.len() > 1;
        for group in features {
            let actions = group
                .actions
                .iter()
                .copied()
                .filter(|action| action.context == context && self.found.shows(action.id));
            let at = Rect::from_min_max(Vec2::new(rect.min.x, y), rect.max);
            if several {
                let key = (context, group.name);
                let mut state = self
                    .features
                    .remove(&key)
                    .unwrap_or_else(|| CollapsiblePanelState::new(true));
                let id = ID.with(context_key(context)).with(name_key(group.name));
                let actions: Vec<&'static ActionDef> = actions.collect();
                let shown =
                    CollapsiblePanel::new(id, &mut state, group.name).show(ui, at, |ui, rect| {
                        self.actions_in(ui, rect, &actions, design, keymap, bounds)
                    });
                self.features.insert(key, state);
                y += shown.height + gap;
            } else {
                let actions: Vec<&'static ActionDef> = actions.collect();
                y += self.actions_in(ui, at, &actions, design, keymap, bounds);
            }
        }
        for info in CONTEXTS
            .iter()
            .filter(|info| info.within == Within::Zone(context))
        {
            if !self.has(keymap, info.context) {
                continue;
            }
            let at = Rect::from_min_max(Vec2::new(rect.min.x, y), rect.max);
            let mut state = self
                .zones
                .remove(&info.context)
                .unwrap_or_else(|| CollapsiblePanelState::new(false));
            let mut panel =
                CollapsiblePanel::new(ID.with(context_key(info.context)), &mut state, info.label);
            if let Some(worst) = self.analysis.zone(info.context) {
                panel = panel.status(Icon::Warning, conflict_color(design, worst));
            }
            let inner = info.context;
            let shown = panel.show(ui, at, |ui, rect| {
                self.zone_content(ui, rect, inner, design, keymap, bounds)
            });
            self.zones.insert(info.context, state);
            y += shown.height + gap;
        }
        (y - rect.min.y - gap).max(0.0)
    }

    /// The panels of `actions`, one below the other. Returns the height used.
    fn actions_in(
        &mut self,
        ui: &mut Ui<'_>,
        rect: Rect,
        actions: &[&'static ActionDef],
        design: &mut DesignSystem,
        keymap: &mut Keymap,
        bounds: Rect,
    ) -> f32 {
        let mut y = rect.min.y;
        for action in actions {
            let at = Rect::from_min_max(Vec2::new(rect.min.x, y), rect.max);
            y += self.action(ui, at, action, design, keymap, bounds);
        }
        y - rect.min.y
    }

    /// The panel of `action` at the top of `rect`. Returns its height.
    fn action(
        &mut self,
        ui: &mut Ui<'_>,
        rect: Rect,
        action: &'static ActionDef,
        design: &mut DesignSystem,
        keymap: &mut Keymap,
        bounds: Rect,
    ) -> f32 {
        let id = ID.with(name_key(action.id));
        let mut state = self.actions.remove(action.id).unwrap_or_else(ActionUi::new);
        let count = keymap.bindings(action.id).map_or(0, <[Binding]>::len);
        state.bindings.resize_with(count, BindingUi::new);
        let worst = self
            .analysis
            .action(action.id)
            .and_then(|report| report.worst);
        let ActionUi {
            panel: panel_state,
            bindings,
            add,
        } = &mut state;
        let mut panel =
            CollapsiblePanel::new(id, panel_state, action.label).trailing(ACTION_TRAILING);
        if let Some(worst) = worst {
            panel = panel.status(Icon::Warning, conflict_color(design, worst));
        }
        let mut edits: Vec<(Option<usize>, Edit)> = Vec::new();
        let shown = panel.show(ui, rect, |ui, inner| {
            self.action_body(
                ui, inner, action, bindings, add, design, keymap, bounds, &mut edits,
            )
        });
        // At the end of the header: its first shortcut on, and what brings the defaults back.
        if let Some(trailing) = shown.trailing {
            let slot = design.get(semantic::SIZE_CONTROL_HEIGHT);
            let dot = Rect::from_min_max(
                Vec2::new(trailing.max.x - slot, trailing.min.y),
                trailing.max,
            );
            let caps = Rect::from_min_max(trailing.min, Vec2::new(dot.min.x, trailing.max.y));
            if let Some(first) = keymap
                .shortcuts(action.id)
                .and_then(|shortcuts| shortcuts.first())
            {
                ui.clipped(caps, |ui| ui.key_caps(caps, first, Align::End));
            }
            if keymap.is_modified(action.id)
                && ui
                    .reset_dot(id.with(9), dot, "Bring back the default shortcuts")
                    .clicked
                && let Err(error) = keymap.reset(action.id)
            {
                koebi_core::warn!(
                    "cannot bring back the shortcuts of {}: {error}",
                    action.label
                );
            }
        }
        for (index, edit) in edits {
            self.apply(keymap, action, index, edit, &mut state);
        }
        self.actions.insert(action.id, state);
        shown.height
    }

    /// What an action's panel holds: what it does, its shortcuts, and the field adding one.
    #[allow(clippy::too_many_arguments, reason = "the parts of one action")]
    fn action_body(
        &self,
        ui: &mut Ui<'_>,
        rect: Rect,
        action: &'static ActionDef,
        bindings: &mut [BindingUi],
        add: &mut CaptureState,
        design: &mut DesignSystem,
        keymap: &Keymap,
        bounds: Rect,
        edits: &mut Vec<(Option<usize>, Edit)>,
    ) -> f32 {
        let padding = ui.styles().panel.padding;
        let row = design.get(semantic::SIZE_CONTROL_HEIGHT);
        let gap = ui.styles().collapsible_panel.gap;
        let detail = ui.styles().text.detail.clone();
        let secondary = ui.styles().text.secondary;
        let mut y = rect.min.y;
        let line = Rect::from_min_max(
            Vec2::new(rect.min.x + padding, y),
            Vec2::new(rect.max.x - padding, y + row),
        );
        ui.label(action.description, line, &detail, secondary, Align::Start);
        y += row + gap;
        let shown = keymap.bindings(action.id).unwrap_or(&[]);
        let report = self.analysis.action(action.id);
        for (index, (binding, state)) in shown.iter().zip(bindings.iter_mut()).enumerate() {
            let id = ID.with(name_key(action.id)).with(100 + index as u64);
            let at = Rect::from_min_max(Vec2::new(rect.min.x, y), rect.max);
            let binding_report = report.and_then(|report| report.bindings.get(index));
            let mut panel =
                CollapsiblePanel::new(id, &mut state.panel, "").trailing(BINDING_TRAILING);
            if let Some(worst) = binding_report.and_then(|report| report.worst) {
                let color = conflict_color(design, worst);
                panel = panel.tint(color.with_alpha(color.a * TINT));
            }
            let mut edit = None;
            let shown = panel.show(ui, at, |ui, inner| {
                binding::details(
                    ui,
                    inner,
                    id,
                    *binding,
                    binding_report,
                    &mut state.editor,
                    &self.choices,
                    design,
                    bounds,
                    &mut edit,
                )
            });
            // The header's controls, over the header: after it, so that they take the pointer.
            if let Some(trailing) = shown.trailing
                && let Some(header) =
                    binding::header(ui, trailing, id, *binding, &mut state.editor, bounds)
            {
                edit = Some(header);
            }
            if let Some(edit) = edit {
                edits.push((Some(index), edit));
            }
            y += shown.height + gap;
        }
        // The field adding a shortcut.
        let at = Rect::from_min_max(
            Vec2::new(rect.min.x + padding, y),
            Vec2::new(rect.max.x - padding, y + row),
        );
        let recorded = CaptureField::new(ID.with(name_key(action.id)).with(99), add, None)
            .capture(Capture::Any)
            .hints("Add a shortcut", "Press the new shortcut")
            .show(ui, at);
        if let Some(shortcut) = recorded.captured {
            edits.push((None, Edit::Set(Binding::on(shortcut))));
        }
        y += row;
        y - rect.min.y
    }

    /// Applies what the user changed in the shortcut at `index` of `action` (a new one for
    /// `None`). A shortcut that would conflict in its own zone stays off until it changes to a
    /// free one; one the user turns on against such a conflict stays off.
    fn apply(
        &mut self,
        keymap: &mut Keymap,
        action: &'static ActionDef,
        index: Option<usize>,
        edit: Edit,
        state: &mut ActionUi,
    ) {
        let mut bindings = keymap.bindings(action.id).unwrap_or(&[]).to_vec();
        // One state per shortcut, in step with the keymap.
        state.bindings.resize_with(bindings.len(), BindingUi::new);
        match edit {
            Edit::Remove => {
                if let Some(index) = index.filter(|&index| index < bindings.len()) {
                    bindings.remove(index);
                    if index < state.bindings.len() {
                        state.bindings.remove(index);
                    }
                }
            }
            Edit::Set(wanted) => {
                keymap.conflicts_in(&ZONES, action.id, wanted.shortcut, &mut self.conflicts);
                let taken = self
                    .conflicts
                    .iter()
                    .any(|conflict| conflict.kind.is_total());
                let previous = index.and_then(|index| bindings.get(index).copied());
                let mut auto_off = index
                    .and_then(|index| state.bindings.get(index))
                    .is_some_and(|ui| ui.editor.auto_off);
                let toggled = previous.is_some_and(|previous| previous.shortcut == wanted.shortcut);
                let enabled = if toggled {
                    // The user's choice, unless it cannot be on: then it comes on once it
                    // changes to a free shortcut.
                    auto_off = wanted.enabled && taken;
                    wanted.enabled && !taken
                } else if taken {
                    auto_off = auto_off || wanted.enabled;
                    false
                } else if auto_off {
                    auto_off = false;
                    true
                } else {
                    wanted.enabled
                };
                let binding = Binding {
                    shortcut: wanted.shortcut,
                    enabled,
                };
                if let Some(index) = index.filter(|&index| index < bindings.len()) {
                    bindings[index] = binding;
                } else {
                    bindings.push(binding);
                    state.bindings.push(BindingUi::new());
                }
                let position = index.unwrap_or(bindings.len() - 1);
                if let Some(ui) = state.bindings.get_mut(position) {
                    ui.editor.auto_off = auto_off;
                }
            }
        }
        if let Err(error) = keymap.set_bindings(action.id, &bindings) {
            koebi_core::warn!("cannot change the shortcuts of {}: {error}", action.label);
        }
    }
}

/// A number standing for `context` in widget identifiers.
fn context_key(context: Context) -> u64 {
    name_key(context.name())
}

/// A number standing for `name` in widget identifiers.
fn name_key(name: &str) -> u64 {
    // FNV-1a: stable from run to run, unlike the standard hasher.
    name.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests;
