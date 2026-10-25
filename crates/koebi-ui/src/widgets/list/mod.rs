//! Lists and trees: one component for every list of the application (outliner, layers, color
//! usage, messages), with every behaviour optional ([`ListOptions`]).
//!
//! The application gives its rows in display order, depth first: a row is the parent of the
//! deeper rows that follow it. Rows are identified by the application's own stable keys, so
//! that folding and selection survive edits. [`ListState`] keeps what outlives a frame: the
//! folded rows and their animations, the selection and the active row, the drag in progress
//! and the scroll position.
//!
//! - **Rows**: one interface unit high (taller when a list asks), zebra colors, a fold arrow
//!   slot and an indentation per level, an icon or a color, a label, and items aligned at the
//!   end that the [`ListDelegate`] draws.
//! - **Folding**: a click on the arrow folds or unfolds a row (with Shift, its whole subtree)
//!   with an animation; a folded row shows how many rows it hides.
//! - **Selection**: a click selects a row and makes it active, Ctrl (Command on Apple
//!   platforms) adds or removes one, Shift selects a range; a click on the empty space clears
//!   the selection. The keyboard moves the active row, folds and unfolds.
//! - **Drag and drop**: dragged rows go before, after or into a row; below the last child of
//!   a subtree, the pointer's indentation picks the level. The list refuses places inside the
//!   dragged rows, the delegate decides the rest, and a line or a frame previews the drop.
//!   The application moves its own data when [`ListResponse::dropped`] says so.

mod draw;
mod interact;
mod layout;
mod state;

#[cfg(test)]
mod tests;

use core::time::Duration;

use koebi_core::input::{Key, ModifierKey, Modifiers, MouseButton};
use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_input::{ActionDef, ActionGroup, ButtonTrigger, Context, ModifierRule, Shortcut};
use koebi_tokens::{component, semantic};

use crate::icon::Icon;
use crate::id::WidgetId;
use crate::ui::Ui;
use crate::widgets::repeating;
use crate::widgets::scroll::{self, ScrollAxes};
pub use state::ListState;

/// A list: its keys apply where the pointer is over it.
pub const LIST: Context = Context::new("list");

/// Rows being dragged: their keys apply wherever the pointer is.
pub const LIST_DRAG: Context = Context::new("list.drag");

/// Selects the row below the active one.
pub static LIST_NEXT: ActionDef = ActionDef {
    id: "list.next",
    label: "Next row",
    description: "Selects the row below the active one and makes it active.",
    context: LIST,
    defaults: &[repeating(Key::Down)],
};

/// Selects the row above the active one.
pub static LIST_PREVIOUS: ActionDef = ActionDef {
    id: "list.previous",
    label: "Previous row",
    description: "Selects the row above the active one and makes it active.",
    context: LIST,
    defaults: &[repeating(Key::Up)],
};

/// Extends the selection to the row below the active one.
pub static LIST_EXTEND_NEXT: ActionDef = ActionDef {
    id: "list.extend-next",
    label: "Extend selection down",
    description: "Selects the rows from the first one selected to the row below the active one.",
    context: LIST,
    defaults: &[repeating(Key::Down).with(ModifierKey::Shift)],
};

/// Extends the selection to the row above the active one.
pub static LIST_EXTEND_PREVIOUS: ActionDef = ActionDef {
    id: "list.extend-previous",
    label: "Extend selection up",
    description: "Selects the rows from the first one selected to the row above the active one.",
    context: LIST,
    defaults: &[repeating(Key::Up).with(ModifierKey::Shift)],
};

/// Unfolds the active row, or goes to its first child.
pub static LIST_UNFOLD: ActionDef = ActionDef {
    id: "list.unfold",
    label: "Unfold",
    description: "Unfolds the active row, or goes to its first child when it is unfolded.",
    context: LIST,
    defaults: &[Shortcut::key(Key::Right)],
};

/// Folds the active row, or goes to its parent.
pub static LIST_FOLD: ActionDef = ActionDef {
    id: "list.fold",
    label: "Fold",
    description: "Folds the active row, or goes to its parent when it is folded or has no \
                  children.",
    context: LIST,
    defaults: &[Shortcut::key(Key::Left)],
};

/// Selects every row.
pub static LIST_SELECT_ALL: ActionDef = ActionDef {
    id: "list.select-all",
    label: "Select all",
    description: "Selects every row of the list, folded ones included.",
    context: LIST,
    defaults: &[Shortcut::key(Key::A).primary()],
};

/// Cancels the drag in progress.
pub static LIST_DRAG_CANCEL: ActionDef = ActionDef {
    id: "list.cancel-drag",
    label: "Cancel drag",
    description: "Cancels the drag in progress: nothing moves.",
    context: LIST_DRAG,
    defaults: &[
        Shortcut::key(Key::Escape),
        Shortcut::key(Key::Delete),
        Shortcut::button(MouseButton::Right, ButtonTrigger::Press),
    ],
};

/// The actions of lists, for the keymap.
pub static LIST_ACTIONS: ActionGroup = ActionGroup {
    name: "Lists",
    actions: &[
        &LIST_NEXT,
        &LIST_PREVIOUS,
        &LIST_EXTEND_NEXT,
        &LIST_EXTEND_PREVIOUS,
        &LIST_UNFOLD,
        &LIST_FOLD,
        &LIST_SELECT_ALL,
        &LIST_DRAG_CANCEL,
    ],
};

/// How fast rows scroll while dragged rows press against the top or bottom edge, in rows per
/// second at the edge.
const AUTO_SCROLL_ROWS_PER_SECOND: f32 = 12.0;

/// Child ids of a list's parts, away from its scroll area's own (`id.with(1)`, `id.with(2)`).
const ROWS: u64 = 10;
const FOLDS: u64 = 11;
const BACKGROUND: u64 = 12;

/// What a row shows before its label.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ItemMark {
    /// Nothing: the label starts after the fold arrow's slot.
    #[default]
    None,
    /// A line icon.
    Icon(Icon),
    /// A color: a layer's, a color in use.
    Color(Color),
}

/// A row of a list, as the application gives it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ListItem<'a> {
    /// The application's stable key for the row: folding, selection and drops refer to it.
    pub id: u64,
    /// Nesting level, 0 at the top: a row is the parent of the deeper rows that follow it.
    pub depth: u16,
    /// Text of the row.
    pub label: &'a str,
    /// Icon or color before the label.
    pub mark: ItemMark,
}

impl<'a> ListItem<'a> {
    /// A top-level row with a label and no mark.
    pub const fn new(id: u64, label: &'a str) -> Self {
        Self {
            id,
            depth: 0,
            label,
            mark: ItemMark::None,
        }
    }

    /// The same row, at nesting level `depth`.
    #[must_use]
    pub const fn with_depth(mut self, depth: u16) -> Self {
        self.depth = depth;
        self
    }

    /// The same row, showing `icon` before its label.
    #[must_use]
    pub const fn with_icon(mut self, icon: Icon) -> Self {
        self.mark = ItemMark::Icon(icon);
        self
    }

    /// The same row, showing `color` before its label.
    #[must_use]
    pub const fn with_color(mut self, color: Color) -> Self {
        self.mark = ItemMark::Color(color);
        self
    }
}

/// How rows are selected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ListSelection {
    /// Rows cannot be selected.
    None,
    /// One row at a time.
    Single,
    /// Any rows: Ctrl (Command) adds or removes one, Shift selects a range.
    #[default]
    Multiple,
}

/// What a list does: every behaviour is optional.
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent switches, one per behaviour of the list"
)]
pub struct ListOptions {
    /// Height of a row; `None` takes the style's, one interface unit.
    pub row_height: Option<f32>,
    /// Alternate row backgrounds.
    pub zebra: bool,
    /// Fold arrows and indentation: rows followed by deeper rows fold. Without it, every row
    /// is a top-level row.
    pub tree: bool,
    /// Lines from parents along their children.
    pub lines: bool,
    /// The number of rows a folded row hides, after its label.
    pub counts: bool,
    /// How rows are selected.
    pub selection: ListSelection,
    /// Rows can be dragged before or after others (and into them in a tree).
    pub drag: bool,
}

impl ListOptions {
    /// A flat list: zebra rows, multiple selection, no dragging.
    pub const LIST: Self = Self {
        row_height: None,
        zebra: true,
        tree: false,
        lines: false,
        counts: false,
        selection: ListSelection::Multiple,
        drag: false,
    };

    /// A tree: folding with lines and counts, multiple selection, drag and drop.
    pub const TREE: Self = Self {
        row_height: None,
        zebra: true,
        tree: true,
        lines: true,
        counts: true,
        selection: ListSelection::Multiple,
        drag: true,
    };
}

impl Default for ListOptions {
    fn default() -> Self {
        Self::LIST
    }
}

/// Where dragged rows would go, relative to a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DropPlace {
    /// Just before the row, at its level.
    Before(u64),
    /// Just after the row and its children, at its level.
    After(u64),
    /// Inside the row, after its children.
    Into(u64),
}

/// A secondary press on a list: open a context menu.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ListContextMenu {
    /// Where the press happened.
    pub at: Vec2,
    /// The row pressed (now selected), or `None` for the empty space.
    pub row: Option<u64>,
}

/// What happened to a list this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ListResponse {
    /// The selection or the active row changed.
    pub selection_changed: bool,
    /// The row double-clicked: open it, rename it.
    pub double_clicked: Option<u64>,
    /// A secondary press, for a context menu.
    pub context_menu: Option<ListContextMenu>,
    /// Rows were dropped there: [`ListState::dragged`] lists them, in display order.
    pub dropped: Option<DropPlace>,
}

/// What the application decides about its list. Every method has a default: `&mut ()`
/// accepts every drop and draws nothing at the rows' ends.
pub trait ListDelegate {
    /// Whether the rows `dragged` (in display order) may go at `place`. The list already
    /// refuses places inside the dragged rows themselves.
    fn can_drop(&self, dragged: &[u64], place: DropPlace) -> bool {
        let _ = (dragged, place);
        true
    }

    /// Draws the items at the end of `item`'s row (visibility toggles, values), right-aligned
    /// in `rect`, and returns the width they took: the label ends before them.
    fn row_end(&mut self, ui: &mut Ui<'_>, item: &ListItem<'_>, rect: Rect) -> f32 {
        let _ = (ui, item, rect);
        0.0
    }
}

impl ListDelegate for () {}

/// Colors and metrics of [`Ui::list`], from the `component.list-row.*`, `component.outliner.*`
/// and `component.table.row.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ListStyle {
    /// Height of a row.
    pub row_height: f32,
    /// Indentation of each level, and width of the fold arrow's slot.
    pub indent: f32,
    /// Space between the rows' sides and their content.
    pub padding: f32,
    /// Space between the parts of a row.
    pub gap: f32,
    /// Backgrounds of even and odd rows.
    pub zebra: [Color; 2],
    /// Background of the row under the pointer.
    pub hover: Color,
    /// Background of selected rows.
    pub selected: Color,
    /// Background of a selected row under the pointer.
    pub selected_hover: Color,
    /// Background of the active row.
    pub active: Color,
    /// Background of the active row under the pointer.
    pub active_hover: Color,
    /// Color of labels and icons.
    pub label: Color,
    /// Side of the fold arrow.
    pub fold_size: f32,
    /// Color of the fold arrow.
    pub fold_color: Color,
    /// How long a fold takes to open.
    pub expand: Duration,
    /// How long a fold takes to close.
    pub collapse: Duration,
    /// Side of a color mark.
    pub swatch_size: f32,
    /// Corner radius of a color mark.
    pub swatch_radius: f32,
    /// Color of the number of rows a folded row hides.
    pub count: Color,
    /// Color of the lines from parents along their children.
    pub line: Color,
    /// Width of those lines.
    pub line_width: f32,
    /// Space those lines leave below the parent and after the last child.
    pub line_inset: f32,
    /// The line where dragged rows would be inserted.
    pub drop_line: Color,
    /// Width of that line.
    pub drop_line_width: f32,
    /// The frame around a row dragged rows would go into.
    pub drop_frame: Color,
    /// Width of that frame.
    pub drop_frame_width: f32,
    /// Corner radius of that frame, and of the dragged rows' label.
    pub drop_frame_radius: f32,
    /// Background of the dragged rows' label, near the pointer.
    pub drag_background: Color,
    /// Opacity of that label, and of the dragged rows' labels in the list.
    pub drag_opacity: f32,
    /// How far the pointer moves before a press drags rows.
    pub drag_threshold: f32,
    /// Longest time between the two clicks of a double click.
    pub double_click: Duration,
}

impl ListStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            row_height: tokens.get(component::LIST_ROW_HEIGHT_DEFAULT),
            indent: tokens.get(component::LIST_ROW_INDENT_DEFAULT),
            padding: tokens.get(component::LIST_ROW_PADDING_HORIZONTAL_DEFAULT),
            gap: tokens.get(component::LIST_ROW_GAP_DEFAULT),
            zebra: [
                tokens.get(component::TABLE_ROW_BACKGROUND_COLOR_DEFAULT),
                tokens.get(component::TABLE_ROW_BACKGROUND_COLOR_ALT),
            ],
            hover: tokens.get(component::OUTLINER_ROW_BACKGROUND_HOVER),
            selected: tokens.get(component::OUTLINER_ROW_BACKGROUND_SELECTED),
            selected_hover: tokens.get(component::OUTLINER_ROW_BACKGROUND_SELECTED_HOVER),
            active: tokens.get(component::OUTLINER_ROW_BACKGROUND_ACTIVE),
            active_hover: tokens.get(component::OUTLINER_ROW_BACKGROUND_ACTIVE_HOVER),
            label: tokens.get(component::OUTLINER_ROW_TEXT_DEFAULT),
            fold_size: tokens.get(component::LIST_ROW_FOLD_SIZE_DEFAULT),
            fold_color: tokens.get(component::LIST_ROW_FOLD_COLOR_DEFAULT),
            expand: tokens.get(component::LIST_ROW_EXPAND_DURATION),
            collapse: tokens.get(component::LIST_ROW_COLLAPSE_DURATION),
            swatch_size: tokens.get(component::LIST_ROW_SWATCH_SIZE_DEFAULT),
            swatch_radius: tokens.get(component::LIST_ROW_SWATCH_CORNER_RADIUS_DEFAULT),
            count: tokens.get(component::LIST_ROW_COUNT_COLOR_DEFAULT),
            line: tokens.get(component::OUTLINER_TREE_LINE_COLOR_DEFAULT),
            line_width: tokens.get(component::OUTLINER_TREE_LINE_WIDTH_DEFAULT),
            line_inset: tokens.get(component::OUTLINER_TREE_LINE_INSET_DEFAULT),
            drop_line: tokens.get(component::LIST_ROW_DROP_LINE_COLOR_DEFAULT),
            drop_line_width: tokens.get(component::LIST_ROW_DROP_LINE_WIDTH_DEFAULT),
            drop_frame: tokens.get(component::DROP_TARGET_BORDER_COLOR_DEFAULT),
            drop_frame_width: tokens.get(component::DROP_TARGET_BORDER_WIDTH_DEFAULT),
            drop_frame_radius: tokens.get(component::DROP_TARGET_CORNER_RADIUS_DEFAULT),
            drag_background: tokens.get(component::LIST_ROW_DRAG_BACKGROUND),
            drag_opacity: tokens.get(component::LIST_ROW_DRAG_OPACITY),
            drag_threshold: tokens.get(semantic::INPUT_DRAG_THRESHOLD),
            double_click: tokens.get(semantic::INPUT_DOUBLE_CLICK_TIME),
        }
    }
}

/// Where the parts of a list's rows go this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Metrics {
    row_height: f32,
    /// Indentation per level: none in a flat list.
    indent: f32,
    /// Where the rows' content starts, horizontally.
    start: f32,
}

/// `true` when the platform's primary modifier is held: Ctrl, or Command on Apple platforms.
fn primary_held(modifiers: Modifiers) -> bool {
    match ModifierRule::PRIMARY {
        ModifierKey::Gui => modifiers.gui(),
        _ => modifiers.ctrl(),
    }
}

impl Ui<'_> {
    /// A list or a tree of `items` over `rect`: the rows in display order, depth first (see
    /// [`ListItem::depth`]). `options` turns its behaviours on, `delegate` decides where
    /// dragged rows may go and draws the items at the rows' ends. Rows scroll with the wheel
    /// and an overlay scrollbar; the keyboard acts on the list under the pointer
    /// ([`LIST_ACTIONS`]).
    pub fn list(
        &mut self,
        id: WidgetId,
        rect: Rect,
        state: &mut ListState,
        items: &[ListItem<'_>],
        options: &ListOptions,
        delegate: &mut impl ListDelegate,
    ) -> ListResponse {
        let styles = self.styles();
        let style = styles.list;
        let mut response = ListResponse::default();
        let row_height = options.row_height.unwrap_or(style.row_height).max(1.0);
        if state.lay_out(items, options.tree, row_height, self.input().time(), &style) {
            // A fold is opening or closing: the next frame continues it.
            self.request_redraw_after(Duration::from_millis(16));
        }
        let viewport = scroll::viewport(rect, ScrollAxes::Vertical, &styles.scroll);
        let metrics = Metrics {
            row_height,
            indent: if options.tree { style.indent } else { 0.0 },
            start: viewport.min.x + style.padding,
        };
        if state.drag.is_none() && self.pointer_within(id) {
            response.selection_changed |=
                self.list_keys(state, items, options, viewport.height(), row_height);
        }
        self.list_drag(
            state,
            items,
            options,
            delegate,
            viewport,
            metrics,
            &mut response,
        );

        let mut scroll = state.scroll;
        let content = Vec2::new(0.0, state.height);
        self.zone(rect, LIST, |ui| {
            ui.scroll_area(
                id,
                rect,
                &mut scroll,
                content,
                ScrollAxes::Vertical,
                |ui, view| {
                    ui.list_rows(
                        id,
                        view.origin,
                        view.viewport,
                        state,
                        items,
                        options,
                        delegate,
                        metrics,
                        &mut response,
                    );
                },
            );
        });
        state.scroll = scroll;
        self.list_ghost(state, items, metrics);
        if self.input().released() {
            state.drag = None;
        }
        // Rows drawn before a change of the selection show it on the next frame, and folds
        // changed this frame start animating then.
        if response.selection_changed || !state.folds.is_empty() {
            self.request_redraw_after(Duration::ZERO);
        }
        response
    }
}
