//! The workspace view: areas drawn as rounded panels with gutters between them, gutters that
//! move every aligned edge, an area menu on a right click, corners that split and join, and
//! previews of splits and joins.
//!
//! [`WorkspaceView`] keeps what outlives a frame: the gutter under the pointer and since when,
//! the operation in progress, the area menu. [`Ui::workspace`] draws a [`Workspace`] and
//! edits it. Editors belong to the application: an [`EditorHost`] draws them and learns when
//! a split copies one or a join removes some.
//!
//! Operations follow the rules of modal operations: Escape, Delete or a right click cancel
//! and restore the exact previous state; Shift makes a resize ten times finer and Ctrl snaps
//! it, both as soon as they are pressed.

mod area;
mod corner;
mod dock;
mod geometry;
mod gutter;
mod preview;

use core::time::Duration;

use koebi_core::input::{CursorShape, Key, MouseButton};
use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_input::{ActionDef, ActionGroup, ButtonTrigger, Context, Shortcut};
use koebi_tokens::{component, semantic};
use koebi_workspace::{AreaId, EditorId, Layout, Line, Orientation, Workspace};

use crate::bar::BarLayout;
use crate::id::WidgetId;
use crate::ui::Ui;
use crate::widgets::{MenuItem, MenuState};
use area::{AreaParts, TabPart};
use dock::DockTarget;
use geometry::{Frame, Segment, segments};

/// The workspace: its gutters and areas.
pub const WORKSPACE: Context = Context::new("workspace");

/// A resize, split or join in progress: its keys apply wherever the pointer is.
pub const WORKSPACE_OPERATION: Context = Context::new("workspace.operation");

/// Cancels the operation in progress.
pub static OPERATION_CANCEL: ActionDef = ActionDef {
    id: "workspace.cancel",
    label: "Cancel",
    description: "Cancels the resize, split or join in progress and restores the areas.",
    context: WORKSPACE_OPERATION,
    defaults: &[
        Shortcut::key(Key::Escape),
        Shortcut::key(Key::Delete),
        Shortcut::button(MouseButton::Right, ButtonTrigger::Press),
    ],
};

/// Applies the split or join previewed under the pointer.
pub static OPERATION_CONFIRM: ActionDef = ActionDef {
    id: "workspace.confirm",
    label: "Confirm",
    description: "Applies the split or join previewed under the pointer.",
    context: WORKSPACE_OPERATION,
    defaults: &[Shortcut::key(Key::Enter), Shortcut::key(Key::KeypadEnter)],
};

/// The actions of the workspace, for the keymap.
pub static WORKSPACE_ACTIONS: ActionGroup = ActionGroup {
    name: "Workspace",
    actions: &[&OPERATION_CANCEL, &OPERATION_CONFIRM],
};

const AREA_MENU: WidgetId = WidgetId::new("workspace-area-menu");

/// The application's editors, as the workspace view needs them.
pub trait EditorHost {
    /// The title of `editor`, shown in its tab (and in its top bar when there are no types).
    fn title(&self, editor: EditorId) -> &str;

    /// The editor types the dropdown at the start of each top bar offers; none hides it.
    fn kinds(&self) -> &[&str] {
        &[]
    }

    /// The type of `editor`, as an index into [`EditorHost::kinds`].
    fn kind(&self, editor: EditorId) -> usize {
        let _ = editor;
        0
    }

    /// The dropdown changed the type of `editor`: the editor keeps its state for each type.
    fn set_kind(&mut self, editor: EditorId, kind: usize) {
        let _ = (editor, kind);
    }

    /// Draws the editor's own items in its top bar, after the type dropdown: groups from the
    /// left, from the right and centred (`bar`).
    fn top_bar(&mut self, ui: &mut Ui<'_>, editor: EditorId, bar: BarLayout) {
        let _ = (ui, editor, bar);
    }

    /// Draws the content of `editor` in `rect`; drawing is clipped to it.
    fn content(&mut self, ui: &mut Ui<'_>, editor: EditorId, rect: Rect);

    /// A split created `editor` as a copy of `source`.
    fn copied(&mut self, source: EditorId, editor: EditorId);

    /// `editors` left the workspace (a join removed their area): their state can go.
    fn removed(&mut self, editors: &[EditorId]);
}

/// Colors and metrics of [`Ui::workspace`], from the `component.zone*` and `component.editor*`
/// tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorkspaceStyle {
    /// Space around and between areas.
    pub gap: f32,
    /// Thickness of the strip over a gutter that grabs the pointer.
    pub hit: f32,
    /// Side of the corners that split and join.
    pub corner: f32,
    /// Smallest width and height of an area.
    pub min_size: f32,
    /// How far a corner moves before its drag counts.
    pub drag_threshold: f32,
    /// The band over the stretch of a gutter between the two hovered areas.
    pub band: Color,
    /// The band along the rest of the line that moves.
    pub band_line: Color,
    /// Background of an area's content.
    pub background: Color,
    /// Background of an area's top bar.
    pub top_bar: Color,
    /// Height of the top bar.
    pub top_bar_height: f32,
    /// Space between the top bar's sides and its content.
    pub top_bar_padding: f32,
    /// Color of the editor's title in the top bar.
    pub title: Color,
    /// Height of the tab bar of an area holding several editors.
    pub tab_bar_height: f32,
    /// Background of the tab bar.
    pub tab_bar_background: Color,
    /// Space between tabs, and above them.
    pub tab_gap: f32,
    /// Space between a tab's sides and its title.
    pub tab_padding: f32,
    /// Background of a tab at rest.
    pub tab_background: Color,
    /// Background of a tab under the pointer.
    pub tab_hover: Color,
    /// Background of the shown editor's tab: the top bar's color.
    pub tab_active: Color,
    /// Title of a tab at rest.
    pub tab_label: Color,
    /// Title of the shown editor's tab.
    pub tab_label_active: Color,
    /// Whether an area with one editor shows its tab bar anyway.
    pub tab_show_solo: bool,
    /// How far a tab or a top bar moves before the drag docks its editor.
    pub tab_drag_threshold: f32,
    /// The line where a dragged tab would go.
    pub insert_line: Color,
    /// Width of that line.
    pub insert_line_width: f32,
    /// Where a dragged editor would go: half an area, or the whole area inset.
    pub drop_preview: Color,
    /// Inset of the preview of a drop into an area's tabs.
    pub drop_center_inset: f32,
    /// Corner radius of the areas.
    pub corner_radius: f32,
    /// How long the pointer stays over a gutter before the band shows.
    pub hover_delay: Duration,
    /// The halves of a split.
    pub split_preview: Color,
    /// The area a join keeps.
    pub join_keep: Color,
    /// The area a join removes.
    pub join_remove: Color,
    /// The parts of the removed area that neighbours take.
    pub join_residual: Color,
    /// The outline of the joined area.
    pub join_frame: Color,
    /// Width of that outline.
    pub join_frame_width: f32,
}

impl WorkspaceStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        let padding = tokens.get(component::EDITOR_TOP_BAR_PADDING_DEFAULT);
        let continuation = tokens.get(component::ZONE_SEPARATOR_COLOR_CONTINUATION);
        let opacity = tokens.get(component::ZONE_SEPARATOR_CONTINUATION_OPACITY_DEFAULT);
        Self {
            gap: tokens.get(component::ZONE_SEPARATOR_SIZE_DEFAULT),
            hit: tokens.get(component::ZONE_SEPARATOR_HIT_SIZE_DEFAULT),
            corner: tokens.get(component::ZONE_CORNER_SIZE_DEFAULT),
            min_size: tokens.get(component::ZONE_MIN_SIZE_DEFAULT),
            drag_threshold: tokens.get(semantic::INPUT_DRAG_THRESHOLD),
            band: tokens.get(component::ZONE_SEPARATOR_COLOR_DEFAULT),
            band_line: continuation.with_alpha(continuation.a * opacity),
            background: tokens.get(component::EDITOR_BACKGROUND_COLOR_DEFAULT),
            top_bar: tokens.get(component::EDITOR_TOP_BAR_BACKGROUND_COLOR_DEFAULT),
            top_bar_height: tokens.get(semantic::SIZE_CONTROL_HEIGHT) + 2.0 * padding,
            top_bar_padding: padding,
            title: tokens.get(component::ZONE_TAB_LABEL_COLOR_ACTIVE),
            tab_bar_height: tokens.get(semantic::SIZE_CONTROL_HEIGHT)
                + 2.0 * tokens.get(component::ZONE_TAB_PADDING_VERTICAL_DEFAULT),
            tab_bar_background: tokens.get(component::ZONE_TAB_BAR_BACKGROUND_COLOR_DEFAULT),
            tab_gap: tokens.get(component::ZONE_TAB_GAP_DEFAULT),
            tab_padding: tokens.get(component::ZONE_TAB_PADDING_HORIZONTAL_DEFAULT),
            tab_background: tokens.get(component::ZONE_TAB_BACKGROUND_COLOR_DEFAULT),
            tab_hover: tokens.get(component::ZONE_TAB_BACKGROUND_COLOR_HOVER),
            tab_active: tokens.get(component::ZONE_TAB_BACKGROUND_COLOR_ACTIVE),
            tab_label: tokens.get(component::ZONE_TAB_LABEL_COLOR_DEFAULT),
            tab_label_active: tokens.get(component::ZONE_TAB_LABEL_COLOR_ACTIVE),
            tab_show_solo: tokens.get(component::ZONE_TAB_SHOW_SOLO) != 0,
            tab_drag_threshold: tokens.get(component::ZONE_TAB_DRAG_THRESHOLD),
            insert_line: tokens.get(component::ZONE_TAB_INSERT_LINE_COLOR_DEFAULT),
            insert_line_width: tokens.get(component::ZONE_TAB_INSERT_LINE_WIDTH_DEFAULT),
            drop_preview: tokens.get(component::ZONE_TAB_DROP_PREVIEW_COLOR_DEFAULT),
            drop_center_inset: tokens.get(component::ZONE_TAB_DROP_PREVIEW_CENTER_INSET),
            corner_radius: tokens.get(component::EDITOR_CORNER_RADIUS_DEFAULT),
            hover_delay: tokens.get(semantic::INPUT_HOVER_DELAY),
            split_preview: tokens.get(component::ZONE_OVERLAY_SPLIT_PREVIEW),
            join_keep: tokens.get(component::ZONE_OVERLAY_JOIN_KEEP),
            join_remove: tokens.get(component::ZONE_OVERLAY_JOIN_REMOVE),
            join_residual: tokens.get(component::ZONE_OVERLAY_JOIN_RESIDUAL),
            join_frame: tokens.get(component::ZONE_OVERLAY_JOIN_FRAME),
            join_frame_width: tokens.get(component::ZONE_OVERLAY_JOIN_FRAME_WIDTH),
        }
    }
}

/// An operation in progress.
#[derive(Clone, Debug)]
enum Operation {
    /// Dragging a gutter: the line, where it would be without the minimum sizes (fractional
    /// grid units, for precise moves), the pointer last frame, and the workspace to restore.
    Resize {
        segment: Segment,
        line: Line,
        position: f64,
        last: Vec2,
        before: Box<Workspace>,
    },
    /// Previewing a split of the area under the pointer (from the area menu).
    Split(Orientation),
    /// Previewing the join of two areas: the one under the pointer goes (from the area menu).
    Join { first: AreaId, second: AreaId },
    /// Dragging a corner of `area`: inwards it splits along the axis the drag started on,
    /// into a neighbour it joins it.
    Corner {
        area: AreaId,
        corner: u8,
        origin: Vec2,
        axis: Option<Orientation>,
    },
    /// Dragging `editor` out of `from` by its tab or top bar, once past the threshold.
    Dock {
        editor: EditorId,
        from: AreaId,
        origin: Vec2,
        started: bool,
    },
}

/// What the workspace view remembers between frames. Keep one per workspace.
#[derive(Debug, Default)]
pub struct WorkspaceView {
    /// The layout `segments` and `areas` were found in.
    layout: Option<Layout>,
    segments: Vec<Segment>,
    areas: Vec<(AreaId, koebi_workspace::GridRect)>,
    /// The gutter under the pointer, and since when.
    hovered: Option<(Segment, Duration)>,
    operation: Option<Operation>,
    menu: MenuState,
    /// The gutter the area menu was opened on.
    menu_gutter: Option<Segment>,
    /// Where the parts of the areas and their tabs lay in the last frame.
    parts: Vec<AreaParts>,
    tabs: Vec<TabPart>,
    /// Where the editor being docked would go.
    dock_target: Option<DockTarget>,
    /// The editor type menu open, and the editor it belongs to.
    type_menu: MenuState,
    type_menu_editor: Option<EditorId>,
}

impl WorkspaceView {
    /// Nothing hovered, no operation.
    pub fn new() -> Self {
        Self::default()
    }

    /// `true` while a resize, split or join is in progress.
    pub fn is_operating(&self) -> bool {
        self.operation.is_some()
    }

    /// Finds the gutters again when the layout changed.
    fn refresh(&mut self, layout: &Layout) {
        if self.layout.as_ref() != Some(layout) {
            segments(layout, &mut self.segments);
            self.areas.clear();
            self.areas.extend(layout.areas());
            self.layout = Some(layout.clone());
        }
    }

    /// Ends the operation in progress, restoring the workspace a resize started from.
    fn cancel(&mut self, workspace: &mut Workspace) {
        if let Some(Operation::Resize { before, .. }) = self.operation.take() {
            *workspace = *before;
        }
        self.dock_target = None;
    }
}

/// The cursor of a gutter on a line of `orientation`, or of a split along it.
fn resize_cursor(orientation: Orientation) -> CursorShape {
    match orientation {
        Orientation::Vertical => CursorShape::ResizeHorizontal,
        Orientation::Horizontal => CursorShape::ResizeVertical,
    }
}

impl Ui<'_> {
    /// Draws `workspace` in `area` (the window) and applies what the user does with its
    /// areas: resizing lines by their gutters, splitting and joining by the areas' corners,
    /// swapping, joining and splitting from the area menu (a right click on a gutter), with
    /// previews. `host` draws the editors.
    pub fn workspace(
        &mut self,
        view: &mut WorkspaceView,
        workspace: &mut Workspace,
        area: Rect,
        host: &mut dyn EditorHost,
    ) {
        let style = self.styles().workspace;
        let frame = Frame::new(area, style.gap);
        view.refresh(workspace.layout());
        if view.operation.is_some() {
            self.grab(WORKSPACE_OPERATION);
            if self.triggered(&OPERATION_CANCEL) {
                view.cancel(workspace);
            }
        }
        self.zone(area, WORKSPACE, |ui| {
            ui.workspace_docking(view, workspace, frame, &style);
            ui.workspace_areas(view, workspace, frame, &style, host, area);
            ui.workspace_gutters(view, workspace, frame, &style);
            ui.workspace_corners(view, workspace, frame, &style, host);
            ui.workspace_operation(view, workspace, frame, &style, host, area);
            ui.dock_preview(view, &style, host);
            ui.workspace_menu(view, workspace, area);
        });
        // A press on a top bar released without dragging was a click: nothing to dock.
        if matches!(view.operation, Some(Operation::Dock { started: false, .. }))
            && self.input().released()
        {
            view.operation = None;
        }
    }

    /// The area menu of a right click on a gutter: swap or join the two areas, or start a
    /// split.
    fn workspace_menu(
        &mut self,
        view: &mut WorkspaceView,
        workspace: &mut Workspace,
        window: Rect,
    ) {
        if !view.menu.is_open() {
            return;
        }
        let Some(gutter) = view.menu_gutter else {
            view.menu.close();
            return;
        };
        let layout = workspace.layout();
        let joinable = layout.plan_join(gutter.before, gutter.after).is_ok()
            || layout.plan_join(gutter.after, gutter.before).is_ok();
        let items = [
            MenuItem::action("Swap areas"),
            MenuItem::action("Join areas").enabled(joinable),
            MenuItem::Separator,
            MenuItem::action("Vertical split"),
            MenuItem::action("Horizontal split"),
        ];
        match self
            .menu(AREA_MENU, &mut view.menu, &items, window)
            .map(|choice| choice.index())
        {
            Some(0) => {
                if let Err(error) = workspace.swap(gutter.before, gutter.after) {
                    koebi_core::warn!("cannot swap the areas: {error}");
                }
            }
            Some(1) => {
                view.operation = Some(Operation::Join {
                    first: gutter.before,
                    second: gutter.after,
                });
            }
            Some(3) => view.operation = Some(Operation::Split(Orientation::Vertical)),
            Some(4) => view.operation = Some(Operation::Split(Orientation::Horizontal)),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
