//! Previews of splits and joins, and the operations the area menu starts: the preview
//! follows the pointer and a press applies it.

use koebi_core::input::CursorShape;
use koebi_core::{Color, Rect};
use koebi_render::RectPrimitive;
use koebi_workspace::{AreaId, GridRect, JoinPlan, Layout, Orientation, Side, Workspace};

use super::geometry::Frame;
use super::{
    EditorHost, OPERATION_CONFIRM, Operation, WorkspaceStyle, WorkspaceView, resize_cursor,
};
use crate::id::WidgetId;
use crate::routing::Layer;
use crate::ui::Ui;

const OPERATION: WidgetId = WidgetId::new("workspace-operation");

/// A split about to happen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SplitPreview {
    pub(super) area: AreaId,
    pub(super) rect: GridRect,
    pub(super) orientation: Orientation,
    /// Where the new line goes, in grid units.
    pub(super) at: u32,
}

impl SplitPreview {
    /// The two halves.
    fn halves(self) -> [GridRect; 2] {
        match self.orientation {
            Orientation::Vertical => [
                self.rect.with_edge(Side::Right, self.at),
                self.rect.with_edge(Side::Left, self.at),
            ],
            Orientation::Horizontal => [
                self.rect.with_edge(Side::Bottom, self.at),
                self.rect.with_edge(Side::Top, self.at),
            ],
        }
    }
}

/// The split of `area` along a line of `orientation` through `point` (grid units), moved
/// within the minimum size `min`; `None` when the area is too small to split.
pub(super) fn split_preview(
    layout: &Layout,
    area: AreaId,
    orientation: Orientation,
    point: (u32, u32),
    min: u32,
) -> Option<SplitPreview> {
    let rect = layout.rect(area)?;
    let (start, end, at) = match orientation {
        Orientation::Vertical => (rect.x0, rect.x1, point.0),
        Orientation::Horizontal => (rect.y0, rect.y1, point.1),
    };
    let low = start.checked_add(min)?;
    let high = end.checked_sub(min)?;
    (low <= high).then(|| SplitPreview {
        area,
        rect,
        orientation,
        at: at.clamp(low, high),
    })
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
pub(super) fn join_cursor(layout: &Layout, plan: &JoinPlan) -> CursorShape {
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

impl Ui<'_> {
    /// Two translucent halves a gutter apart, where `preview` would split its area.
    pub(super) fn draw_split(
        &mut self,
        preview: SplitPreview,
        frame: Frame,
        style: &WorkspaceStyle,
    ) {
        for half in preview.halves() {
            self.scene.push_rect(
                &RectPrimitive::new(frame.panel(half), style.split_preview)
                    .with_corner_radius(style.corner_radius),
            );
        }
    }

    /// The preview of a join: the kept area lighter, the part of the removed area it takes
    /// darker, the parts neighbours take darker still, and the joined area outlined.
    pub(super) fn draw_join(
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

    /// The preview of the operation in progress, above everything. For splits and joins of
    /// the area menu, a press applies it (a press where nothing can happen does nothing);
    /// corner drags apply on release ([`Ui::workspace_corners`]).
    pub(super) fn workspace_operation(
        &mut self,
        view: &mut WorkspaceView,
        workspace: &mut Workspace,
        frame: Frame,
        style: &WorkspaceStyle,
        host: &mut dyn EditorHost,
        window: Rect,
    ) {
        let layout = workspace.layout();
        let pointer = self.input().pointer();
        let pointed = pointer
            .and_then(|point| frame.grid_point(point))
            .and_then(|(x, y)| layout.area_at(x, y).zip(Some((x, y))));
        match view.operation {
            Some(Operation::Corner { area, axis, .. }) => {
                match super::corner::corner_preview(layout, area, axis, pointed, frame, style) {
                    super::corner::CornerPreview::Split(preview) => {
                        self.draw_split(preview, frame, style);
                    }
                    super::corner::CornerPreview::Join(plan) => {
                        self.draw_join(layout, &plan, frame, style);
                    }
                    super::corner::CornerPreview::Nothing => {}
                }
            }
            Some(Operation::Split(orientation)) => {
                let min = frame.grid_length(style.min_size, orientation);
                let preview = pointed
                    .and_then(|(area, point)| split_preview(layout, area, orientation, point, min));
                let cursor =
                    preview.map_or(CursorShape::NotAllowed, |_| resize_cursor(orientation));
                let confirmed = self.on_layer(Layer::OVERLAY, |ui| {
                    let catcher = ui.interact_with_cursor(OPERATION, window, cursor);
                    if let Some(preview) = preview {
                        ui.draw_split(preview, frame, style);
                    }
                    (!catcher.secondary_pressed)
                        .then(|| catcher.pressed || ui.triggered(&OPERATION_CONFIRM))
                });
                match (confirmed, preview) {
                    (None, _) => view.operation = None,
                    (Some(true), Some(preview)) => {
                        if let Ok(split) =
                            workspace.split(preview.area, orientation, preview.at, min)
                        {
                            host.copied(split.copy_of, split.editor);
                        }
                        view.operation = None;
                    }
                    _ => {}
                }
            }
            Some(Operation::Join { first, second }) => {
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
                        ui.draw_join(layout, plan, frame, style);
                    }
                    (!catcher.secondary_pressed)
                        .then(|| catcher.pressed || ui.triggered(&OPERATION_CONFIRM))
                });
                match (confirmed, plan) {
                    (None, _) => view.operation = None,
                    (Some(true), Some(plan)) => {
                        if let Ok(removed) = workspace.join(plan.kept, plan.removed) {
                            host.removed(&removed);
                        }
                        view.operation = None;
                    }
                    _ => {}
                }
            }
            Some(Operation::Resize { .. }) | None => {}
        }
    }
}
