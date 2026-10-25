//! Gutters: dragging one moves its whole line; a right click opens the area menu; after a
//! hover delay, a band shows the line that would move.

use koebi_core::Vec2;
use koebi_render::RectPrimitive;
use koebi_workspace::{GRID, Orientation, Workspace};

use super::geometry::{Frame, Segment};
use super::{Operation, WorkspaceStyle, WorkspaceView, resize_cursor};
use crate::id::WidgetId;
use crate::ui::Ui;

const GUTTER: WidgetId = WidgetId::new("workspace-gutter");

/// Ctrl snaps a resized line to multiples of this fraction of the window.
const SNAP_STEPS: u32 = 48;

/// The widget id of the gutter between `before` and `after`, stable while they exist.
fn gutter_id(segment: &Segment) -> WidgetId {
    GUTTER.with((u64::from(segment.before.get()) << 32) | u64::from(segment.after.get()))
}

/// Where a resized line goes for a grid position, snapped by Ctrl.
fn resize_target(position: f64, snap: bool) -> u32 {
    let position = position.clamp(0.0, f64::from(GRID));
    let target = if snap {
        // Exact fractions of the window: a 48th of the grid is not a whole number of units.
        let steps = f64::from(SNAP_STEPS);
        ((position / f64::from(GRID) * steps).round() * f64::from(GRID) / steps).round()
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
    /// The gutters: resize cursor, drag to move their line, right click for the area menu,
    /// and the band along the line after a hover delay or while resizing.
    pub(super) fn workspace_gutters(
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
        *last = Vec2::new(pointer.x, pointer.y);
        let min = frame.grid_length(style.min_size, line.orientation);
        let target = resize_target(*position, modifiers.ctrl());
        let landed = workspace.move_line(line, target, min);
        if landed != target {
            // Held by a minimum size: the line follows again as soon as the pointer turns.
            *position = f64::from(landed);
        }
        line.position = landed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_snaps_to_a_48th_of_the_window() {
        let step = f64::from(GRID) / f64::from(SNAP_STEPS);
        // 3 * 65536 / 48 = 4096 exactly; 4 * 65536 / 48 = 5461.33, rounded.
        assert_eq!(resize_target(step * 3.4, true), 4096);
        assert_eq!(resize_target(step * 3.6, true), 5461);
        assert_eq!(resize_target(1234.4, false), 1234);
        assert_eq!(resize_target(-50.0, false), 0);
        assert_eq!(resize_target(1e9, true), GRID);
    }
}
