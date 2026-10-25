//! The workspace view: areas drawn as rounded panels with gutters between them, gutters that
//! move every aligned edge, an area menu on a right click, and previews of splits and joins.
//!
//! [`WorkspaceView`] keeps what outlives a frame: the gutter under the pointer and since when,
//! the operation in progress, the area menu. [`Ui::workspace`] draws a [`Workspace`] and
//! edits it. Editors belong to the application: an [`EditorHost`] draws them and learns when
//! a split copies one or a join removes some.
//!
//! Operations follow the rules of modal operations: Escape, Delete or a right click cancel
//! and restore the exact previous state; Shift makes a resize ten times finer and Ctrl snaps
//! it, both as soon as they are pressed.

mod geometry;

use core::time::Duration;

use koebi_core::input::{CursorShape, Key, MouseButton};
use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_input::{ActionDef, ActionGroup, ButtonTrigger, Context, Shortcut};
use koebi_render::RectPrimitive;
use koebi_tokens::{component, semantic};
use koebi_workspace::{
    AreaId, EditorId, GRID, GridRect, JoinPlan, Layout, Line, Orientation, Workspace,
};

use crate::id::WidgetId;
use crate::routing::Layer;
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

const GUTTER: WidgetId = WidgetId::new("workspace-gutter");
const OPERATION: WidgetId = WidgetId::new("workspace-operation");
const AREA_MENU: WidgetId = WidgetId::new("workspace-area-menu");

/// Ctrl snaps a resized line to multiples of this fraction of the window.
const SNAP_STEPS: u32 = 48;

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
    /// Smallest width and height of an area.
    pub min_size: f32,
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
            min_size: tokens.get(component::ZONE_MIN_SIZE_DEFAULT),
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
    /// Previewing a split of the area under the pointer.
    Split(Orientation),
    /// Previewing the join of two areas: the one under the pointer goes.
    Join { first: AreaId, second: AreaId },
}

/// What the workspace view remembers between frames. Keep one per workspace.
#[derive(Debug, Default)]
pub struct WorkspaceView {
    /// The layout `segments` were found in.
    layout: Option<Layout>,
    segments: Vec<Segment>,
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

/// The widget id of the gutter between `before` and `after`, stable while they exist.
fn gutter_id(segment: &Segment) -> WidgetId {
    GUTTER.with((u64::from(segment.before.get()) << 32) | u64::from(segment.after.get()))
}

/// The cursor of a gutter on a line of `orientation`.
fn resize_cursor(orientation: Orientation) -> CursorShape {
    match orientation {
        Orientation::Vertical => CursorShape::ResizeHorizontal,
        Orientation::Horizontal => CursorShape::ResizeVertical,
    }
}

/// The part of `a` inside `b`, if any.
fn intersection(a: GridRect, b: GridRect) -> Option<GridRect> {
    let rect = GridRect::new(
        a.x0.max(b.x0),
        a.y0.max(b.y0),
        a.x1.min(b.x1),
        a.y1.min(b.y1),
    );
    (!rect.is_empty()).then_some(rect)
}

/// The arrow pointing from the removed area towards the kept one.
fn join_cursor(layout: &Layout, plan: &JoinPlan) -> CursorShape {
    let (Some(kept), Some(removed)) = (layout.rect(plan.kept), layout.rect(plan.removed)) else {
        return CursorShape::NotAllowed;
    };
    if removed.x0 == kept.x1 {
        CursorShape::ArrowLeft
    } else if removed.x1 == kept.x0 {
        CursorShape::ArrowRight
    } else if removed.y0 == kept.y1 {
        CursorShape::ArrowUp
    } else {
        CursorShape::ArrowDown
    }
}

/// Where a resized line goes for a grid position, snapped by Ctrl.
fn resize_target(position: f64, snap: bool) -> u32 {
    let position = position.clamp(0.0, f64::from(GRID));
    let target = if snap {
        let step = f64::from(GRID / SNAP_STEPS);
        (position / step).round() * step
    } else {
        position.round()
    };
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=GRID first"
    )]
    let target = target.clamp(0.0, f64::from(GRID)) as u32;
    target
}

impl Ui<'_> {
    /// Draws `workspace` in `area` (the window) and applies what the user does with its
    /// areas: resizing lines by their gutters, swapping, joining and splitting areas from the
    /// area menu (a right click on a gutter), with previews. `host` draws the editors.
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

    /// The gutters: resize cursor, drag to move their line, right click for the area menu,
    /// and the band along the line after a hover delay or while resizing.
    fn workspace_gutters(
        &mut self,
        view: &mut WorkspaceView,
        workspace: &mut Workspace,
        frame: Frame,
        style: &WorkspaceStyle,
    ) {
        let mut hovered = None;
        for index in 0..view.segments.len() {
            let segment = view.segments[index];
            let rect = frame.strip(&segment.line, segment.start, segment.end, style.hit);
            let response = self.interact_with_cursor(
                gutter_id(&segment),
                rect,
                resize_cursor(segment.line.orientation),
            );
            if response.hovered {
                hovered = Some(segment);
            }
            if response.secondary_pressed
                && view.operation.is_none()
                && let Some(point) = self.input().pointer()
            {
                view.menu.open_at(point);
                view.menu_gutter = Some(segment);
            }
            if response.pressed
                && view.operation.is_none()
                && let Some(point) = self.input().pointer()
            {
                view.operation = Some(Operation::Resize {
                    segment,
                    line: segment.line,
                    position: f64::from(segment.line.position),
                    last: point,
                    before: Box::new(workspace.clone()),
                });
            }
            // The same two areas: the line itself moves while it is dragged.
            let resizing = matches!(
                &view.operation,
                Some(Operation::Resize { segment: held, .. })
                    if held.before == segment.before && held.after == segment.after
            );
            // The release may come with a last movement.
            if resizing && (response.held || response.released) {
                self.resize(view, workspace, frame, style);
            }
            if resizing && response.released {
                view.operation = None;
            }
        }

        let now = self.input().time();
        view.hovered = match (hovered, view.hovered) {
            (Some(segment), Some((previous, since))) if previous == segment => {
                Some((segment, since))
            }
            (Some(segment), _) => Some((segment, now)),
            (None, _) => None,
        };
        let band = match &view.operation {
            Some(Operation::Resize { segment, line, .. }) => Some((*segment, *line)),
            Some(_) => None,
            None => view.hovered.and_then(|(segment, since)| {
                let shown = since + style.hover_delay;
                if now >= shown {
                    Some((segment, segment.line))
                } else {
                    self.request_redraw_at(shown);
                    None
                }
            }),
        };
        if let Some((segment, line)) = band {
            let radius = style.gap * 0.5;
            let whole = frame.strip(&line, line.start, line.end, style.gap);
            self.scene
                .push_rect(&RectPrimitive::new(whole, style.band_line).with_corner_radius(radius));
            let stretch = frame.strip(&line, segment.start, segment.end, style.gap);
            self.scene
                .push_rect(&RectPrimitive::new(stretch, style.band).with_corner_radius(radius));
        }
    }

    /// Moves the line of the resize in progress with the pointer: ten times slower with Shift,
    /// snapped with Ctrl, within the minimum sizes.
    fn resize(
        &mut self,
        view: &mut WorkspaceView,
        workspace: &mut Workspace,
        frame: Frame,
        style: &WorkspaceStyle,
    ) {
        let Some(Operation::Resize {
            line,
            position,
            last,
            ..
        }) = &mut view.operation
        else {
            return;
        };
        let Some(pointer) = self.input().pointer() else {
            return;
        };
        let modifiers = self.input().modifiers();
        let delta = match line.orientation {
            Orientation::Vertical => pointer.x - last.x,
            Orientation::Horizontal => pointer.y - last.y,
        };
        let factor = if modifiers.shift() { 0.1 } else { 1.0 };
        *position += frame.grid_delta(delta * factor, line.orientation);
        *last = pointer;
        let min = frame.grid_length(style.min_size, line.orientation);
        let target = resize_target(*position, modifiers.ctrl());
        let landed = workspace.move_line(line, target, min);
        if landed != target {
            // Held by a minimum size: the line follows again as soon as the pointer turns.
            *position = f64::from(landed);
        }
        line.position = landed;
    }

    /// The preview of a split or join in progress, above everything: a press applies it.
    fn workspace_operation(
        &mut self,
        view: &mut WorkspaceView,
        workspace: &mut Workspace,
        frame: Frame,
        style: &WorkspaceStyle,
        host: &mut dyn EditorHost,
        window: Rect,
    ) {
        let operation = match &view.operation {
            Some(Operation::Split(orientation)) => Operation::Split(*orientation),
            Some(Operation::Join { first, second }) => Operation::Join {
                first: *first,
                second: *second,
            },
            _ => return,
        };
        let layout = workspace.layout();
        let pointed = self
            .input()
            .pointer()
            .and_then(|point| frame.grid_point(point))
            .and_then(|(x, y)| layout.area_at(x, y).zip(Some((x, y))));
        match operation {
            Operation::Split(orientation) => {
                let min = frame.grid_length(style.min_size, orientation);
                let preview = pointed.and_then(|(area, (x, y))| {
                    let rect = layout.rect(area)?;
                    let (start, end, at) = match orientation {
                        Orientation::Vertical => (rect.x0, rect.x1, x),
                        Orientation::Horizontal => (rect.y0, rect.y1, y),
                    };
                    let low = start.checked_add(min)?;
                    let high = end.checked_sub(min)?;
                    (low <= high).then(|| (area, rect, at.clamp(low, high)))
                });
                let cursor =
                    preview.map_or(CursorShape::NotAllowed, |_| resize_cursor(orientation));
                let confirmed = self.on_layer(Layer::OVERLAY, |ui| {
                    let catcher = ui.interact_with_cursor(OPERATION, window, cursor);
                    if let Some((_, rect, at)) = preview {
                        let (first, second) = match orientation {
                            Orientation::Vertical => (
                                rect.with_edge(koebi_workspace::Side::Right, at),
                                rect.with_edge(koebi_workspace::Side::Left, at),
                            ),
                            Orientation::Horizontal => (
                                rect.with_edge(koebi_workspace::Side::Bottom, at),
                                rect.with_edge(koebi_workspace::Side::Top, at),
                            ),
                        };
                        for half in [first, second] {
                            ui.scene.push_rect(
                                &RectPrimitive::new(frame.panel(half), style.split_preview)
                                    .with_corner_radius(style.corner_radius),
                            );
                        }
                    }
                    if catcher.secondary_pressed {
                        return None;
                    }
                    Some(catcher.pressed || ui.triggered(&OPERATION_CONFIRM))
                });
                match (confirmed, preview) {
                    (None, _) => view.operation = None,
                    // Where nothing can split, a press does nothing: the preview goes on.
                    (Some(true), Some((area, _, at))) => {
                        if let Ok(split) = workspace.split(area, orientation, at, min) {
                            host.copied(split.copy_of, split.editor);
                        }
                        view.operation = None;
                    }
                    _ => {}
                }
            }
            Operation::Join { first, second } => {
                let plan = match pointed {
                    Some((area, _)) if area == first => layout.plan_join(second, first).ok(),
                    Some((area, _)) if area == second => layout.plan_join(first, second).ok(),
                    _ => None,
                };
                let cursor = plan
                    .as_ref()
                    .map_or(CursorShape::NotAllowed, |plan| join_cursor(layout, plan));
                let confirmed = self.on_layer(Layer::OVERLAY, |ui| {
                    let catcher = ui.interact_with_cursor(OPERATION, window, cursor);
                    if let Some(plan) = &plan {
                        ui.join_preview(layout, plan, frame, style);
                    }
                    if catcher.secondary_pressed {
                        return None;
                    }
                    Some(catcher.pressed || ui.triggered(&OPERATION_CONFIRM))
                });
                match (confirmed, plan) {
                    (None, _) => view.operation = None,
                    // Where nothing can join, a press does nothing: the preview goes on.
                    (Some(true), Some(plan)) => {
                        if let Ok(removed) = workspace.join(plan.kept, plan.removed) {
                            host.removed(&removed);
                        }
                        view.operation = None;
                    }
                    _ => {}
                }
            }
            Operation::Resize { .. } => {}
        }
    }

    /// The preview of a join: the kept area lighter, the part of the removed area it takes
    /// darker, the parts neighbours take darker still, and the joined area outlined.
    fn join_preview(
        &mut self,
        layout: &Layout,
        plan: &JoinPlan,
        frame: Frame,
        style: &WorkspaceStyle,
    ) {
        let radius = style.corner_radius;
        let mut fill = |rect: GridRect, color: Color| {
            self.scene.push_rect(
                &RectPrimitive::new(frame.panel(rect), color).with_corner_radius(radius),
            );
        };
        if let Some(kept) = layout.rect(plan.kept) {
            fill(kept, style.join_keep);
        }
        if let Some(taken) = layout
            .rect(plan.removed)
            .and_then(|removed| intersection(removed, plan.result))
        {
            fill(taken, style.join_remove);
        }
        for piece in &plan.fills {
            fill(piece.piece, style.join_residual);
        }
        self.scene.push_rect(
            &RectPrimitive::new(frame.panel(plan.result), Color::TRANSPARENT)
                .with_corner_radius(radius)
                .with_border(style.join_frame_width, style.join_frame),
        );
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
