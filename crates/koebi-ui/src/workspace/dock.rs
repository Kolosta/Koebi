//! Docking: an editor dragged by its tab or by the background of its top bar drops into the
//! tabs of an area (over its tab bar or top bar, or in its middle) or beside it (near the
//! edges of its content), which splits the area in half. Its former area closes when left
//! empty.

use koebi_core::input::CursorShape;
use koebi_core::{Rect, Vec2};
use koebi_render::RectPrimitive;
use koebi_workspace::{AreaId, EditorId, Side, Workspace};

use super::area::{AreaParts, TabPart};
use super::geometry::Frame;
use super::{EditorHost, Operation, WorkspaceStyle, WorkspaceView};
use crate::text::Align;
use crate::ui::Ui;

/// Near the edges, within this fraction of the content's size, an editor docks beside the
/// area rather than into its tabs.
const EDGE: f32 = 0.25;

/// Where a dragged editor would go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DockTarget {
    /// Into the tabs of `area`, at `index`.
    Tabs { area: AreaId, index: usize },
    /// Beside `area`, on `side`: the area splits in half.
    Side { area: AreaId, side: Side },
}

/// Where `editor`, dragged from `from`, would go with the pointer at `pointer`, given the
/// areas and tabs of the last frame.
pub(super) fn dock_target(
    parts: &[AreaParts],
    tabs: &[TabPart],
    workspace: &Workspace,
    editor: EditorId,
    from: AreaId,
    pointer: Vec2,
) -> Option<DockTarget> {
    let part = parts.iter().find(|part| part.panel.contains(pointer))?;
    let area = part.area;
    let count = workspace.stack(area)?.len();
    if part.top().contains(pointer) {
        let index = if part.tabs.is_some() {
            // Before the first tab whose middle lies right of the pointer.
            tabs.iter()
                .filter(|tab| tab.area == area && tab.editor != editor)
                .take_while(|tab| tab.rect.center().x < pointer.x)
                .count()
        } else {
            usize::from(pointer.x >= part.top().center().x)
        };
        return Some(DockTarget::Tabs { area, index });
    }
    let content = part.content;
    if !content.contains(pointer) || content.width() <= 0.0 || content.height() <= 0.0 {
        return None;
    }
    let x = (pointer.x - content.min.x) / content.width();
    let y = (pointer.y - content.min.y) / content.height();
    let (distance, side) = [
        (x, Side::Left),
        (1.0 - x, Side::Right),
        (y, Side::Top),
        (1.0 - y, Side::Bottom),
    ]
    .into_iter()
    .min_by(|a, b| a.0.total_cmp(&b.0))?;
    if distance < EDGE {
        // An editor alone in its area cannot dock beside it.
        (area != from || count > 1).then_some(DockTarget::Side { area, side })
    } else {
        let others = if area == from { count - 1 } else { count };
        Some(DockTarget::Tabs {
            area,
            index: others,
        })
    }
}

/// Applies a drop: tabs reorder within an area, editors move between areas.
fn apply(
    workspace: &mut Workspace,
    editor: EditorId,
    from: AreaId,
    target: DockTarget,
    frame: Frame,
    style: &WorkspaceStyle,
) {
    let result = match target {
        DockTarget::Tabs { area, index } if area == from => {
            let Some(position) = workspace
                .stack(area)
                .and_then(|stack| stack.editors().iter().position(|e| *e == editor))
            else {
                return;
            };
            workspace.reorder(area, position, index)
        }
        DockTarget::Tabs { area, index } => workspace.dock_in_tabs(editor, area, index).map(|_| ()),
        DockTarget::Side { area, side } => {
            let min = frame.grid_length(style.min_size, side.orientation());
            workspace.dock_at_side(editor, area, side, min).map(|_| ())
        }
    };
    if let Err(error) = result {
        koebi_core::debug!("the editor cannot dock there: {error}");
    }
}

impl Ui<'_> {
    /// Advances a docking drag before the areas are drawn: past the threshold it starts, the
    /// target follows the pointer, and the release drops the editor there.
    pub(super) fn workspace_docking(
        &mut self,
        view: &mut WorkspaceView,
        workspace: &mut Workspace,
        frame: Frame,
        style: &WorkspaceStyle,
    ) {
        let Some(Operation::Dock {
            editor,
            from,
            origin,
            started,
        }) = &mut view.operation
        else {
            view.dock_target = None;
            return;
        };
        let (editor, from) = (*editor, *from);
        let Some(pointer) = self.input().pointer() else {
            return;
        };
        if !*started && (pointer - *origin).length() >= style.tab_drag_threshold {
            *started = true;
        }
        if !*started {
            // Released before the threshold: a click, which the tab handles while drawing.
            return;
        }
        view.dock_target = dock_target(&view.parts, &view.tabs, workspace, editor, from, pointer);
        if self.input().released() {
            if let Some(target) = view.dock_target {
                apply(workspace, editor, from, target, frame, style);
            }
            view.operation = None;
            view.dock_target = None;
        }
    }

    /// The preview of a docking drag, above everything: where the editor would go, and its
    /// tab under the pointer.
    pub(super) fn dock_preview(
        &mut self,
        view: &WorkspaceView,
        style: &WorkspaceStyle,
        host: &mut dyn EditorHost,
    ) {
        let Some(Operation::Dock {
            editor,
            started: true,
            ..
        }) = view.operation
        else {
            return;
        };
        let Some(pointer) = self.input().pointer() else {
            return;
        };
        let radius = style.corner_radius;
        match view.dock_target {
            Some(DockTarget::Side { area, side }) => {
                let Some(part) = view.parts.iter().find(|part| part.area == area) else {
                    return;
                };
                let (panel, half) = (part.panel, style.gap * 0.5);
                let center = panel.center();
                let rect = match side {
                    Side::Left => {
                        Rect::from_min_max(panel.min, Vec2::new(center.x - half, panel.max.y))
                    }
                    Side::Right => {
                        Rect::from_min_max(Vec2::new(center.x + half, panel.min.y), panel.max)
                    }
                    Side::Top => {
                        Rect::from_min_max(panel.min, Vec2::new(panel.max.x, center.y - half))
                    }
                    Side::Bottom => {
                        Rect::from_min_max(Vec2::new(panel.min.x, center.y + half), panel.max)
                    }
                };
                self.scene.push_rect(
                    &RectPrimitive::new(rect, style.drop_preview).with_corner_radius(radius),
                );
            }
            Some(DockTarget::Tabs { area, index }) => {
                let Some(part) = view.parts.iter().find(|part| part.area == area) else {
                    return;
                };
                if part.top().contains(pointer) {
                    // A line where the tab would go, between the other tabs.
                    let mut others = view
                        .tabs
                        .iter()
                        .filter(|tab| tab.area == area && tab.editor != editor);
                    let before = index.checked_sub(1).and_then(|i| others.clone().nth(i));
                    let after = others.nth(index);
                    let strip = part.tabs.unwrap_or(part.bar);
                    let x = match (before, after) {
                        (Some(before), _) => before.rect.max.x + style.tab_gap * 0.5,
                        (None, Some(after)) => after.rect.min.x - style.tab_gap * 0.5,
                        (None, None) => strip.min.x + style.tab_gap,
                    };
                    let half = style.insert_line_width * 0.5;
                    self.scene.push_rect(&RectPrimitive::new(
                        Rect::from_min_max(
                            Vec2::new(x - half, strip.min.y),
                            Vec2::new(x + half, strip.max.y),
                        ),
                        style.insert_line,
                    ));
                } else {
                    let rect = part.panel.inset(style.drop_center_inset);
                    self.scene.push_rect(
                        &RectPrimitive::new(rect, style.drop_preview).with_corner_radius(radius),
                    );
                }
            }
            None => {}
        }
        // The dragged tab, under the pointer.
        let font = self.styles().text.label.clone();
        let title = host.title(editor);
        let width = self.text_width(title, &font) + 2.0 * style.tab_padding;
        let ghost = Rect::from_center_size(
            pointer + Vec2::new(0.0, style.tab_bar_height),
            Vec2::new(width, style.tab_bar_height),
        );
        self.scene.push_rect(
            &RectPrimitive::new(ghost, style.tab_active.with_alpha(0.85))
                .with_corner_radius(radius),
        );
        self.label(title, ghost, &font, style.tab_label_active, Align::Center);
    }
}

/// The cursor of the tab or top bar of `editor` while it is dragged to dock: moving where it
/// can drop, "not allowed" elsewhere. `None` when it is not being docked.
pub(super) fn dock_cursor(view: &WorkspaceView, editor: EditorId) -> Option<CursorShape> {
    match view.operation {
        Some(Operation::Dock {
            editor: dragged,
            started: true,
            ..
        }) if dragged == editor => Some(if view.dock_target.is_some() {
            CursorShape::Move
        } else {
            CursorShape::NotAllowed
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use koebi_workspace::{GRID, Orientation};

    use super::*;

    fn parts(workspace: &Workspace) -> Vec<AreaParts> {
        // Two halves of a 800 x 600 window, without tabs: top bars 32 pixels high.
        workspace
            .layout()
            .areas()
            .map(|(area, rect)| {
                let x0 = if rect.x0 == 0 { 0.0 } else { 400.0 };
                let panel = Rect::from_min_max(Vec2::new(x0, 0.0), Vec2::new(x0 + 400.0, 600.0));
                let bar = Rect::from_min_max(panel.min, Vec2::new(panel.max.x, 32.0));
                let content = Rect::from_min_max(Vec2::new(panel.min.x, 32.0), panel.max);
                AreaParts {
                    area,
                    panel,
                    tabs: None,
                    bar,
                    content,
                }
            })
            .collect()
    }

    #[test]
    fn edges_dock_beside_and_the_middle_and_top_dock_into_tabs() {
        let mut workspace = Workspace::new();
        let left = workspace.layout().areas().next().unwrap().0;
        let split = workspace
            .split(left, Orientation::Vertical, GRID / 2, 1)
            .unwrap();
        let right = split.area;
        let editor = workspace.stack(left).unwrap().active();
        let parts = parts(&workspace);
        let target =
            |x: f32, y: f32| dock_target(&parts, &[], &workspace, editor, left, Vec2::new(x, y));
        assert_eq!(
            target(790.0, 300.0),
            Some(DockTarget::Side {
                area: right,
                side: Side::Right
            })
        );
        assert_eq!(
            target(600.0, 40.0),
            Some(DockTarget::Side {
                area: right,
                side: Side::Top
            })
        );
        assert_eq!(
            target(600.0, 320.0),
            Some(DockTarget::Tabs {
                area: right,
                index: 1
            })
        );
        // Over the top bar: before or after the single editor.
        assert_eq!(
            target(450.0, 16.0),
            Some(DockTarget::Tabs {
                area: right,
                index: 0
            })
        );
        assert_eq!(
            target(750.0, 16.0),
            Some(DockTarget::Tabs {
                area: right,
                index: 1
            })
        );
        // Alone in its area, the editor cannot dock beside it.
        assert_eq!(target(10.0, 300.0), None);
        assert_eq!(target(-5.0, 300.0), None);
    }
}
