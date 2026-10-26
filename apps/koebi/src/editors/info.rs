//! The Info template: the application's messages in a flat list, newest last, marked by
//! level. Every Info editor shows the same messages; each keeps its own selection and scroll
//! position.

use koebi_core::input::{Key, ModifierKey};
use koebi_core::log::Level;
use koebi_core::{Color, Rect};
use koebi_design::DesignSystem;
use koebi_input::{ActionDef, ActionGroup, Context, Shortcut};
use koebi_tokens::semantic;
use koebi_ui::{ListItem, ListOptions, ListSelection, ListState, Ui, WidgetId};

use crate::messages::Messages;

/// The Info editor: its keys apply where the pointer is over one.
pub(crate) const INFO: Context = Context::new("info");

/// Deletes the selected messages.
pub(crate) static DELETE: ActionDef = ActionDef {
    id: "info.delete",
    label: "Delete messages",
    description: "Deletes the selected messages.",
    context: INFO,
    defaults: &[Shortcut::key(Key::X), Shortcut::key(Key::Delete)],
};

/// Deletes every message.
pub(crate) static CLEAR: ActionDef = ActionDef {
    id: "info.clear",
    label: "Clear messages",
    description: "Deletes every message.",
    context: INFO,
    defaults: &[Shortcut::key(Key::Delete).with(ModifierKey::Shift)],
};

/// The Info editor's actions, for the keymap.
pub(crate) static ACTIONS: ActionGroup = ActionGroup {
    name: "Info",
    actions: &[&DELETE, &CLEAR],
};

/// The colors marking each level.
struct Marks {
    error: Color,
    warn: Color,
    info: Color,
    other: Color,
}

impl Marks {
    fn resolve(design: &DesignSystem) -> Self {
        Self {
            error: design.get(semantic::ICON_COLOR_NEGATIVE_DEFAULT),
            warn: design.get(semantic::ICON_COLOR_NOTICE_DEFAULT),
            info: design.get(semantic::ICON_COLOR_INFO_DEFAULT),
            other: design.get(semantic::ICON_COLOR_SECONDARY_DEFAULT),
        }
    }

    fn of(&self, level: Level) -> Color {
        match level {
            Level::Error => self.error,
            Level::Warn => self.warn,
            Level::Info => self.info,
            Level::Debug | Level::Trace => self.other,
        }
    }
}

/// Draws an Info editor showing `messages` in `rect`, with its own `state`, and applies its
/// actions. `items` is reused from frame to frame.
pub(crate) fn show(
    ui: &mut Ui<'_>,
    id: WidgetId,
    rect: Rect,
    design: &DesignSystem,
    messages: &mut Messages,
    state: &mut ListState,
    items: &mut Vec<ListItem<'static>>,
) {
    let pointer_over = ui
        .input()
        .pointer()
        .is_some_and(|point| rect.contains(point));
    let marks = Marks::resolve(design);
    let mut rows = super::recycle(core::mem::take(items));
    rows.extend(messages.list().iter().map(|message| {
        ListItem::new(message.id, &message.text).with_color(marks.of(message.level))
    }));
    let options = ListOptions {
        selection: ListSelection::Multiple,
        ..ListOptions::LIST
    };
    ui.zone(rect, INFO, |ui| {
        ui.list(id, rect, state, &rows, &options, &mut ())
    });
    *items = super::recycle(rows);
    if !pointer_over {
        return;
    }
    if ui.triggered(&DELETE) {
        messages.remove(|id| state.is_selected(id));
        state.clear_selection();
    }
    if ui.triggered(&CLEAR) {
        messages.remove(|_| true);
        state.clear_selection();
    }
}
