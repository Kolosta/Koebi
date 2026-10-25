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

mod corner;
mod geometry;
mod gutter;
mod preview;

use core::time::Duration;

use koebi_core::input::{CursorShape, Key, MouseButton};
use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_input::{ActionDef, ActionGroup, ButtonTrigger, Context, Shortcut};
use koebi_render::RectPrimitive;
use koebi_tokens::{component, semantic};
use koebi_workspace::{AreaId, EditorId, Layout, Line, Orientation, Workspace};

use crate::id::WidgetId;
use crate::text::Align;
use crate::ui::Ui;
use crate::widgets::{MenuItem, MenuState};
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
    /// The title of `editor`, shown in its top bar.
    fn title(&self, editor: EditorId) -> &str;

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
            ui.workspace_areas(workspace, frame, &style, host);
            ui.workspace_gutters(view, workspace, frame, &style);
            ui.workspace_corners(view, workspace, frame, &style, host);
            ui.workspace_operation(view, workspace, frame, &style, host, area);
            ui.workspace_menu(view, workspace, area);
        });
    }

    /// The panels of the areas: background, top bar with the editor's title, content.
    fn workspace_areas(
        &mut self,
        workspace: &Workspace,
        frame: Frame,
        style: &WorkspaceStyle,
        host: &mut dyn EditorHost,
    ) {
        let font = self.styles().text.label.clone();
        for (id, rect) in workspace.layout().areas() {
            let Some(editor) = workspace.stack(id).map(koebi_workspace::Stack::active) else {
                continue;
            };
            let panel = frame.panel(rect);
            if panel.is_empty() {
                continue;
            }
            self.scene.push_rect(
                &RectPrimitive::new(panel, style.background)
                    .with_corner_radius(style.corner_radius),
            );
            let bar = Rect::from_min_max(
                panel.min,
                Vec2::new(
                    panel.max.x,
                    (panel.min.y + style.top_bar_height).min(panel.max.y),
                ),
            );
            self.scene.push_rect(
                &RectPrimitive::new(bar, style.top_bar).with_corner_radius(style.corner_radius),
            );
            // Square bottom corners: the content continues below the bar.
            let radius = style.corner_radius.min(bar.height() * 0.5);
            self.scene.push_rect(&RectPrimitive::new(
                Rect::from_min_max(Vec2::new(bar.min.x, bar.max.y - radius), bar.max),
                style.top_bar,
            ));
            let title = Rect::from_min_max(
                Vec2::new(bar.min.x + style.top_bar_padding, bar.min.y),
                Vec2::new(bar.max.x - style.top_bar_padding, bar.max.y),
            );
            self.label(host.title(editor), title, &font, style.title, Align::Start);
            let content = Rect::from_min_max(Vec2::new(panel.min.x, bar.max.y), panel.max);
            if !content.is_empty() {
                self.clipped(content, |ui| host.content(ui, editor, content));
            }
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
        match self.menu(AREA_MENU, &mut view.menu, &items, window) {
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
