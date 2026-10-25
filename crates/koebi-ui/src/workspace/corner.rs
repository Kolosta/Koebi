//! Corners: dragging one inwards splits its area along the axis the drag starts on (a
//! horizontal drag makes a vertical line); dragging it into a neighbour joins that neighbour
//! into the area. The preview follows the pointer and the release applies it.

use koebi_core::input::CursorShape;
use koebi_core::{Rect, Vec2};
use koebi_workspace::{AreaId, JoinPlan, Layout, Orientation, Workspace};

use super::geometry::Frame;
use super::preview::{SplitPreview, join_cursor, split_preview};
use super::{EditorHost, Operation, WorkspaceStyle, WorkspaceView, resize_cursor};
use crate::id::WidgetId;
use crate::ui::Ui;

const CORNER: WidgetId = WidgetId::new("workspace-corner");

/// What releasing a corner drag would do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum CornerPreview {
    /// Nothing: the pointer is outside the workspace, the drag has not started, the area is
    /// too small to split, or the neighbour cannot join.
    Nothing,
    /// Split the area.
    Split(SplitPreview),
    /// Join the neighbour under the pointer into the area.
    Join(JoinPlan),
}

/// What a drag from a corner of `area` would do with the pointer over `pointed` (an area and
/// the grid point); `axis` is the split axis, once the drag has started.
pub(super) fn corner_preview(
    layout: &Layout,
    area: AreaId,
    axis: Option<Orientation>,
    pointed: Option<(AreaId, (u32, u32))>,
    frame: Frame,
    style: &WorkspaceStyle,
) -> CornerPreview {
    match pointed {
        Some((under, point)) if under == area => axis
            .and_then(|axis| {
                let min = frame.grid_length(style.min_size, axis);
                split_preview(layout, area, axis, point, min)
            })
            .map_or(CornerPreview::Nothing, CornerPreview::Split),
        Some((under, _)) => layout
            .plan_join(area, under)
            .map_or(CornerPreview::Nothing, CornerPreview::Join),
        None => CornerPreview::Nothing,
    }
}

/// What releasing the corner drag of `area` in progress would do now.
fn current(
    view: &WorkspaceView,
    workspace: &Workspace,
    area: AreaId,
    pointer: Option<Vec2>,
    frame: Frame,
    style: &WorkspaceStyle,
) -> CornerPreview {
    let Some(Operation::Corner { axis, .. }) = view.operation else {
        return CornerPreview::Nothing;
    };
    let layout = workspace.layout();
    let pointed = pointer
        .and_then(|point| frame.grid_point(point))
        .and_then(|(x, y)| layout.area_at(x, y).zip(Some((x, y))));
    corner_preview(layout, area, axis, pointed, frame, style)
}

/// The axis of the split a drag of `delta` makes, once it moved `threshold` pixels: a
/// horizontal drag makes a vertical line.
fn drag_axis(delta: Vec2, threshold: f32) -> Option<Orientation> {
    (delta.length() >= threshold).then(|| {
        if delta.x.abs() >= delta.y.abs() {
            Orientation::Vertical
        } else {
            Orientation::Horizontal
        }
    })
}

/// The squares of side `size` at the corners of `panel`: top-left, top-right, bottom-left,
/// bottom-right.
fn corners(panel: Rect, size: f32) -> [Rect; 4] {
    let at = |x: f32, y: f32| Rect::from_origin_size(Vec2::new(x, y), Vec2::splat(size));
    let (right, bottom) = (panel.max.x - size, panel.max.y - size);
    [
        at(panel.min.x, panel.min.y),
        at(right, panel.min.y),
        at(panel.min.x, bottom),
        at(right, bottom),
    ]
}

impl Ui<'_> {
    /// The corners of the areas: a crosshair cursor, and drags that split or join.
    pub(super) fn workspace_corners(
        &mut self,
        view: &mut WorkspaceView,
        workspace: &mut Workspace,
        frame: Frame,
        style: &WorkspaceStyle,
        host: &mut dyn EditorHost,
    ) {
        let pointer = self.input().pointer();
        for index in 0..view.areas.len() {
            let (area, rect) = view.areas[index];
            let panel = frame.panel(rect);
            // Corners would cover a tiny area entirely.
            if panel.width() < style.corner * 3.0 || panel.height() < style.corner * 3.0 {
                continue;
            }
            for (corner, square) in (0_u8..).zip(corners(panel, style.corner)) {
                let dragging = matches!(
                    view.operation,
                    Some(Operation::Corner { area: held, corner: which, .. })
                        if held == area && which == corner
                );
                // The axis first: the cursor shows the split it decides.
                if dragging
                    && let (Some(Operation::Corner { origin, axis, .. }), Some(point)) =
                        (&mut view.operation, pointer)
                    && axis.is_none()
                {
                    *axis = drag_axis(point - *origin, style.drag_threshold);
                }
                let cursor = if dragging {
                    match current(view, workspace, area, pointer, frame, style) {
                        CornerPreview::Split(split) => resize_cursor(split.orientation),
                        CornerPreview::Join(plan) => join_cursor(workspace.layout(), &plan),
                        CornerPreview::Nothing => CursorShape::Crosshair,
                    }
                } else {
                    CursorShape::Crosshair
                };
                let id = CORNER.with((u64::from(area.get()) << 2) | u64::from(corner));
                let response = self.interact_with_cursor(id, square, cursor);
                if response.pressed
                    && view.operation.is_none()
                    && let Some(point) = pointer
                {
                    view.operation = Some(Operation::Corner {
                        area,
                        corner,
                        origin: point,
                        axis: None,
                    });
                }
                if !dragging || !(response.held || response.released) {
                    continue;
                }
                if response.released {
                    match current(view, workspace, area, pointer, frame, style) {
                        CornerPreview::Split(split) => {
                            let min = frame.grid_length(style.min_size, split.orientation);
                            if let Ok(created) =
                                workspace.split(area, split.orientation, split.at, min)
                            {
                                host.copied(created.copy_of, created.editor);
                            }
                        }
                        CornerPreview::Join(plan) => {
                            if let Ok(removed) = workspace.join(plan.kept, plan.removed) {
                                host.removed(&removed);
                            }
                        }
                        CornerPreview::Nothing => {}
                    }
                    view.operation = None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use koebi_workspace::{GRID, GridRect};

    use super::*;

    #[test]
    fn drags_choose_their_axis_once_past_the_threshold() {
        assert_eq!(drag_axis(Vec2::new(2.0, 1.0), 4.0), None);
        assert_eq!(
            drag_axis(Vec2::new(6.0, 2.0), 4.0),
            Some(Orientation::Vertical)
        );
        assert_eq!(
            drag_axis(Vec2::new(-1.0, -9.0), 4.0),
            Some(Orientation::Horizontal)
        );
    }

    #[test]
    fn corners_sit_inside_the_panel() {
        let panel = Rect::from_min_max(Vec2::new(10.0, 20.0), Vec2::new(110.0, 220.0));
        let [top_left, top_right, bottom_left, bottom_right] = corners(panel, 12.0);
        assert_eq!(top_left.min, panel.min);
        assert_eq!(top_right.max.x, panel.max.x);
        assert_eq!(bottom_left.max.y, panel.max.y);
        assert_eq!(bottom_right.max, panel.max);
    }

    #[test]
    fn corner_drags_split_inside_and_join_neighbours() {
        let layout = Layout::from_areas(&[
            (0, GridRect::new(0, 0, GRID / 2, GRID)),
            (1, GridRect::new(GRID / 2, 0, GRID, GRID)),
        ])
        .unwrap();
        let (left, right) = (AreaId::from_saved(0), AreaId::from_saved(1));
        let frame = Frame::new(
            Rect::from_origin_size(Vec2::ZERO, Vec2::new(804.0, 604.0)),
            4.0,
        );
        let style = crate::test_styles().workspace;
        let inside = Some((left, (GRID / 4, GRID / 2)));
        // Before the drag picks an axis, nothing happens inside the area.
        assert_eq!(
            corner_preview(&layout, left, None, inside, frame, &style),
            CornerPreview::Nothing
        );
        let CornerPreview::Split(split) = corner_preview(
            &layout,
            left,
            Some(Orientation::Vertical),
            inside,
            frame,
            &style,
        ) else {
            panic!("a split inside the area");
        };
        assert_eq!((split.area, split.at), (left, GRID / 4));
        // Over the neighbour: it joins into the area the drag started from.
        let CornerPreview::Join(plan) = corner_preview(
            &layout,
            left,
            Some(Orientation::Vertical),
            Some((right, (3 * GRID / 4, GRID / 2))),
            frame,
            &style,
        ) else {
            panic!("a join into the area");
        };
        assert_eq!((plan.kept, plan.removed), (left, right));
        assert_eq!(
            corner_preview(&layout, left, None, None, frame, &style),
            CornerPreview::Nothing
        );
    }
}
