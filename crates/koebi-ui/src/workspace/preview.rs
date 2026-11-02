//! Previews of splits and joins, and the operations the area menu starts: the preview
//! follows the pointer and a press applies it.

use koebi_core::input::CursorShape;
use koebi_core::{Color, Rect};
use koebi_render::{Corners, RectPrimitive};
use koebi_workspace::{AreaId, Fill, GridRect, JoinPlan, Layout, Orientation, Side, Workspace};

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

/// The arrow pointing the way the join goes: from the kept area, which grows, towards the
/// removed one.
pub(super) fn join_cursor(layout: &Layout, plan: &JoinPlan) -> CursorShape {
    let (Some(kept), Some(removed)) = (layout.rect(plan.kept), layout.rect(plan.removed)) else {
        return CursorShape::NotAllowed;
    };
    if removed.x0 == kept.x1 {
        CursorShape::ArrowRight
    } else if removed.x1 == kept.x0 {
        CursorShape::ArrowLeft
    } else if removed.y0 == kept.y1 {
        CursorShape::ArrowDown
    } else {
        CursorShape::ArrowUp
    }
}

/// `true` when `b` lies against `side` of `a`, along a stretch of some length.
fn touches(a: GridRect, b: GridRect, side: Side) -> bool {
    let along_y = a.y0 < b.y1 && b.y0 < a.y1;
    let along_x = a.x0 < b.x1 && b.x0 < a.x1;
    match side {
        Side::Left => a.x0 == b.x1 && along_y,
        Side::Right => a.x1 == b.x0 && along_y,
        Side::Top => a.y0 == b.y1 && along_x,
        Side::Bottom => a.y1 == b.y0 && along_x,
    }
}

/// `rect` reaching `by` further on `side`.
fn grown(rect: Rect, side: Side, by: f32) -> Rect {
    let mut grown = rect;
    match side {
        Side::Left => grown.min.x -= by,
        Side::Right => grown.max.x += by,
        Side::Top => grown.min.y -= by,
        Side::Bottom => grown.max.y += by,
    }
    grown
}

/// A corner of a rectangle.
#[derive(Clone, Copy)]
struct Corner {
    right: bool,
    bottom: bool,
}

impl Corner {
    const ALL: [Self; 4] = [
        Self {
            right: false,
            bottom: false,
        },
        Self {
            right: true,
            bottom: false,
        },
        Self {
            right: true,
            bottom: true,
        },
        Self {
            right: false,
            bottom: true,
        },
    ];

    /// `radius` at this corner, the others square.
    fn radii(self, radius: f32) -> Corners {
        let at = |right: bool, bottom: bool| {
            if self.right == right && self.bottom == bottom {
                radius
            } else {
                0.0
            }
        };
        Corners {
            top_left: at(false, false),
            top_right: at(true, false),
            bottom_right: at(true, true),
            bottom_left: at(false, true),
        }
    }

    /// Where this corner of `rect` lies on the grid.
    fn of(self, rect: GridRect) -> (u32, u32) {
        (
            if self.right { rect.x1 } else { rect.x0 },
            if self.bottom { rect.y1 } else { rect.y0 },
        )
    }

    /// `true` when `other` lies against one of the two sides of `rect` meeting at this corner,
    /// and reaches this corner along that side.
    fn is_against(self, rect: GridRect, other: GridRect) -> bool {
        let (x, y) = self.of(rect);
        let within = |low: u32, high: u32, at: u32, end: bool| {
            if end {
                low < at && at <= high
            } else {
                low <= at && at < high
            }
        };
        let beside = touches(
            rect,
            other,
            if self.right { Side::Right } else { Side::Left },
        ) && within(other.y0, other.y1, y, self.bottom);
        let over = touches(
            rect,
            other,
            if self.bottom { Side::Bottom } else { Side::Top },
        ) && within(other.x0, other.x1, x, self.right);
        beside || over
    }
}

/// What a join preview draws, in order. Around the future area, everything that goes is dark:
/// the parts of the areas drawn square, across the gutters between them and what stays, and
/// the outsides of the rounded corners of what stays and of the future area.
pub(super) fn join_preview(
    layout: &Layout,
    plan: &JoinPlan,
    frame: Frame,
    style: &WorkspaceStyle,
    emit: &mut impl FnMut(RectPrimitive),
) {
    let radius = style.corner_radius;
    let part_of = |area: AreaId| {
        layout
            .rect(area)
            .and_then(|rect| intersection(rect, plan.result))
    };
    let (kept, removed) = (part_of(plan.kept), part_of(plan.removed));
    let fills: &[Fill] = &plan.fills;
    // What stays, lighter.
    if let Some(kept) = kept {
        emit(RectPrimitive::new(frame.panel(kept), style.join_keep).with_corner_radius(radius));
    }
    // What the kept area takes: square towards it, rounded like the future area at its ends.
    if let Some(part) = removed {
        let mut rect = frame.panel(part);
        if let Some(kept) = kept {
            for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
                if touches(part, kept, side) {
                    rect = grown(rect, side, frame.gap);
                }
            }
        }
        let mut radii = Corners::ZERO;
        for corner in Corner::ALL {
            if corner.of(part) == corner.of(plan.result) {
                let one = corner.radii(radius);
                radii = Corners {
                    top_left: radii.top_left.max(one.top_left),
                    top_right: radii.top_right.max(one.top_right),
                    bottom_right: radii.bottom_right.max(one.bottom_right),
                    bottom_left: radii.bottom_left.max(one.bottom_left),
                };
            }
        }
        emit(RectPrimitive::new(rect, style.join_remove).with_corner_radii(radii));
    }
    // What neighbours take: square, up to what it lies against.
    for fill in fills {
        let mut rect = frame.panel(fill.piece);
        for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
            let against_part = [kept, removed]
                .into_iter()
                .flatten()
                .any(|part| touches(fill.piece, part, side));
            if against_part {
                rect = grown(rect, side, frame.gap);
            } else if fills
                .iter()
                .any(|other| touches(fill.piece, other.piece, side))
            {
                rect = grown(rect, side, frame.gap * 0.5);
            }
        }
        emit(RectPrimitive::new(rect, style.join_residual));
    }
    // The outside of a corner takes the color of what goes next to it.
    let dark_at = |part: GridRect, corner: Corner, own: Option<Color>| {
        let filled = fills
            .iter()
            .any(|fill| corner.is_against(part, fill.piece))
            .then_some(style.join_residual);
        filled.or(own).or_else(|| {
            removed
                .filter(|removed| corner.is_against(part, *removed))
                .map(|_| style.join_remove)
        })
    };
    if let Some(part) = kept {
        let panel = frame.panel(part);
        for corner in Corner::ALL {
            if let Some(color) = dark_at(part, corner, None) {
                emit(
                    RectPrimitive::new(panel, color)
                        .with_corner_radii(corner.radii(radius))
                        .inverted(),
                );
            }
        }
    }
    let result = frame.panel(plan.result);
    if let Some(part) = removed {
        for corner in Corner::ALL {
            if corner.of(part) == corner.of(plan.result)
                && let Some(color) = dark_at(part, corner, Some(style.join_remove))
            {
                emit(
                    RectPrimitive::new(result, color)
                        .with_corner_radii(corner.radii(radius))
                        .inverted(),
                );
            }
        }
    }
    emit(
        RectPrimitive::new(result, Color::TRANSPARENT)
            .with_corner_radius(radius)
            .with_border(style.join_frame_width, style.join_frame),
    );
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

    /// The preview of a join ([`join_preview`]).
    pub(super) fn draw_join(
        &mut self,
        layout: &Layout,
        plan: &JoinPlan,
        frame: Frame,
        style: &WorkspaceStyle,
    ) {
        join_preview(layout, plan, frame, style, &mut |primitive| {
            self.scene.push_rect(&primitive);
        });
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
            Some(Operation::Resize { .. } | Operation::Dock { .. }) | None => {}
        }
    }
}
