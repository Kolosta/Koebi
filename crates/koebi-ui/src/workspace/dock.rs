//! Docking: an editor dragged by its tab or by the background of its top bar drops into the
//! tabs of an area (over its tab bar, or in its middle) or beside it (around the middle), which
//! splits the area in half. Its former area closes when left empty.
//!
//! A lone editor has no tab bar to drop into: when the drag stays over its top for a moment,
//! the bar slides in ([`Reveal`]). The preview of where the editor would go glides from one
//! place to the next within an area ([`Glide`]), and jumps to another area.

use core::time::Duration;

use koebi_core::input::CursorShape;
use koebi_core::{Rect, Vec2};
use koebi_render::RectPrimitive;
use koebi_workspace::{AreaId, EditorId, Side, Workspace};

use super::area::{AreaParts, TabPart};
use super::geometry::Frame;
use super::{EditorHost, Operation, WorkspaceStyle, WorkspaceView};
use crate::text::Align;
use crate::ui::Ui;

/// Around the middle of an area, within this share of its size on each side of its center, an
/// editor docks into its tabs; further out, beside it.
const MIDDLE: f32 = 0.2;

/// How open a sliding tab bar must be for its tabs to take a drop: it has stopped moving.
const SETTLED: f32 = 0.95;

/// How open a sliding tab bar must be for a drop over the top of its area to go into its tabs.
const HALF_OPEN: f32 = 0.5;

/// The longest step an animation takes in one frame: a frame drawn after a pause starts the
/// animation instead of finishing it.
const MAX_STEP: Duration = Duration::from_millis(33);

/// How soon an animating frame asks for the next.
const FRAME: Duration = Duration::from_millis(16);

/// Where a dragged editor would go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DockTarget {
    /// Into the tabs of `area`, at `index`: `in_bar` when the pointer is over its tab bar, which
    /// shows where the tab would go.
    Tabs {
        area: AreaId,
        index: usize,
        in_bar: bool,
    },
    /// Beside `area`, on `side`: the area splits in half.
    Side { area: AreaId, side: Side },
}

/// The tab bar of a lone editor, opening or closing for a docking drag.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Reveal {
    pub(super) area: AreaId,
    /// The drag is over the top of the area.
    hovered: bool,
    /// When `hovered` last changed: the delays run from there.
    since: Duration,
    /// Whether the bar goes open or closed.
    open: bool,
    /// How open it is, from 0 to 1.
    pub(super) amount: f32,
}

/// The preview of a docking drag on its way to where it goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Glide {
    area: AreaId,
    pub(super) rect: Rect,
}

/// The share of the way an exponential glide with time constant `constant` covers in `elapsed`.
fn glide_share(elapsed: Duration, constant: Duration) -> f32 {
    let constant = constant.as_secs_f32();
    if constant > 0.0 {
        1.0 - (-elapsed.as_secs_f32() / constant).exp()
    } else {
        1.0
    }
}

impl WorkspaceView {
    /// Advances the animations' clock to `now`.
    pub(super) fn tick(&mut self, now: Duration) {
        self.elapsed = now.saturating_sub(self.clock).min(MAX_STEP);
        self.clock = now;
    }
}

/// Where `editor`, dragged from `from`, would go with the pointer at `pointer`, given the
/// areas and tabs of the last frame and the tab bar opening.
#[allow(clippy::too_many_arguments, reason = "what a drop depends on")]
pub(super) fn dock_target(
    parts: &[AreaParts],
    tabs: &[TabPart],
    workspace: &Workspace,
    editor: EditorId,
    from: AreaId,
    pointer: Vec2,
    reveal: Option<Reveal>,
    band: f32,
) -> Option<DockTarget> {
    let part = parts.iter().find(|part| part.panel.contains(pointer))?;
    let area = part.area;
    let count = workspace.stack(area)?.len();
    let others = if area == from { count - 1 } else { count };
    let opening = reveal.filter(|reveal| reveal.area == area);
    // A bar still sliding does not take drops at a place: tabs are moving.
    let settled = opening.is_none_or(|reveal| reveal.amount >= SETTLED);
    if settled && part.tabs.is_some_and(|strip| strip.contains(pointer)) {
        // Before the first tab whose middle lies right of the pointer.
        let index = tabs
            .iter()
            .filter(|tab| tab.area == area && tab.editor != editor)
            .take_while(|tab| tab.rect.center().x < pointer.x)
            .count();
        return Some(DockTarget::Tabs {
            area,
            index,
            in_bar: true,
        });
    }
    if opening.is_some_and(|reveal| reveal.amount > HALF_OPEN)
        && pointer.y < part.panel.min.y + band
    {
        return Some(DockTarget::Tabs {
            area,
            index: others,
            in_bar: false,
        });
    }
    let panel = part.panel;
    if panel.width() <= 0.0 || panel.height() <= 0.0 {
        return None;
    }
    // From the center of the area: -1 at its left or top side, 1 at its right or bottom side.
    let x = (pointer.x - panel.center().x) / (panel.width() * 0.5);
    let y = (pointer.y - panel.center().y) / (panel.height() * 0.5);
    if x.abs() < MIDDLE * 2.0 && y.abs() < MIDDLE * 2.0 {
        return Some(DockTarget::Tabs {
            area,
            index: others,
            in_bar: false,
        });
    }
    let side = if x.abs() >= y.abs() {
        if x < 0.0 { Side::Left } else { Side::Right }
    } else if y < 0.0 {
        Side::Top
    } else {
        Side::Bottom
    };
    // An editor alone in its area cannot dock beside it.
    (area != from || count > 1).then_some(DockTarget::Side { area, side })
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
        DockTarget::Tabs { area, index, .. } if area == from => {
            let Some(position) = workspace
                .stack(area)
                .and_then(|stack| stack.editors().iter().position(|e| *e == editor))
            else {
                return;
            };
            workspace.reorder(area, position, index)
        }
        DockTarget::Tabs { area, index, .. } => {
            workspace.dock_in_tabs(editor, area, index).map(|_| ())
        }
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
    /// Opens the tab bar of a lone editor when a drag stays over its top, closes it when the
    /// drag leaves or ends, and slides it.
    fn reveal_tab_bar(
        &mut self,
        view: &mut WorkspaceView,
        workspace: &Workspace,
        style: &WorkspaceStyle,
    ) {
        let now = self.input().time();
        let dragging = matches!(view.operation, Some(Operation::Dock { started: true, .. }));
        let lone = |area: AreaId| {
            !style.tab_show_solo && workspace.stack(area).is_some_and(|stack| stack.len() == 1)
        };
        let over = self
            .input()
            .pointer()
            .filter(|_| dragging)
            .and_then(|pointer| {
                view.parts.iter().find(|part| {
                    part.panel.contains(pointer)
                        && pointer.y < part.panel.min.y + style.top_bar_height
                })
            })
            .map(|part| part.area)
            .filter(|area| lone(*area));
        let another_open =
            |area: AreaId, reveal: &Reveal| reveal.area != area && reveal.amount > 0.0;
        if let Some(area) = over
            && view
                .reveal
                .is_some_and(|reveal| another_open(area, &reveal))
        {
            // Another area's bar comes first: it slides out, then this one waits its delay.
            if let Some(reveal) = &mut view.reveal {
                reveal.hovered = false;
                reveal.open = false;
            }
        } else if let Some(area) = over {
            let fresh = Reveal {
                area,
                hovered: false,
                since: now,
                open: false,
                amount: 0.0,
            };
            let reveal = view.reveal.get_or_insert(fresh);
            if reveal.area != area {
                *reveal = fresh;
            }
            if !reveal.hovered {
                reveal.hovered = true;
                reveal.since = now;
            }
            let at = reveal.since + style.reveal_delay;
            if now >= at {
                reveal.open = true;
            } else if !reveal.open {
                self.request_redraw_at(at);
            }
        } else if let Some(reveal) = &mut view.reveal {
            if reveal.hovered {
                reveal.hovered = false;
                reveal.since = now;
            }
            let at = reveal.since + style.reveal_grace;
            if !dragging || now >= at {
                reveal.open = false;
            } else if reveal.open {
                self.request_redraw_at(at);
            }
        }
        let share = glide_share(view.elapsed, style.drop_glide);
        if let Some(reveal) = &mut view.reveal {
            let target = if reveal.open { 1.0 } else { 0.0 };
            let before = reveal.amount;
            reveal.amount += (target - reveal.amount) * share;
            if (target - reveal.amount).abs() < 0.01 {
                reveal.amount = target;
            }
            // Until the frame after the last change: the drop target and another area's bar
            // depend on where it ended.
            if (reveal.amount - before).abs() > 0.0 {
                self.request_redraw_after(FRAME);
            }
            if !reveal.open && !reveal.hovered && reveal.amount <= 0.0 {
                view.reveal = None;
            }
        }
    }

    /// Advances a docking drag before the areas are drawn, and the tab bar it opens.
    pub(super) fn workspace_docking(
        &mut self,
        view: &mut WorkspaceView,
        workspace: &mut Workspace,
        frame: Frame,
        style: &WorkspaceStyle,
    ) {
        self.dock_drag(view, workspace, frame, style);
        self.reveal_tab_bar(view, workspace, style);
    }

    /// Past the threshold the drag starts, the target follows the pointer, and the release
    /// drops the editor there.
    fn dock_drag(
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
        view.dock_target = dock_target(
            &view.parts,
            &view.tabs,
            workspace,
            editor,
            from,
            pointer,
            view.reveal,
            style.top_bar_height,
        );
        if self.input().released() {
            if let Some(target) = view.dock_target {
                apply(workspace, editor, from, target, frame, style);
            } else if view.window.is_some_and(|window| !window.contains(pointer)) {
                view.detached = Some(editor);
            }
            view.operation = None;
            view.dock_target = None;
        }
    }

    /// Draws the preview of a drop at `target` in `area`, gliding there from where it was, or
    /// jumping there from another area.
    fn glide_preview(
        &mut self,
        view: &mut WorkspaceView,
        area: AreaId,
        target: Rect,
        style: &WorkspaceStyle,
    ) {
        let rect = match view.glide {
            Some(glide) if glide.area == area => {
                let share = glide_share(view.elapsed, style.drop_glide);
                let min = glide.rect.min + (target.min - glide.rect.min) * share;
                let max = glide.rect.max + (target.max - glide.rect.max) * share;
                if (target.min - min).length() + (target.max - max).length() < 0.5 {
                    target
                } else {
                    self.request_redraw_after(FRAME);
                    Rect::from_min_max(min, max)
                }
            }
            _ => target,
        };
        view.glide = Some(Glide { area, rect });
        self.scene.push_rect(
            &RectPrimitive::new(rect, style.drop_preview).with_corner_radius(style.corner_radius),
        );
    }

    /// The preview of a docking drag, above everything: where the editor would go, and its
    /// tab under the pointer.
    pub(super) fn dock_preview(
        &mut self,
        view: &mut WorkspaceView,
        style: &WorkspaceStyle,
        host: &mut dyn EditorHost,
    ) {
        let Some(Operation::Dock {
            editor,
            started: true,
            ..
        }) = view.operation
        else {
            view.glide = None;
            return;
        };
        let Some(pointer) = self.input().pointer() else {
            return;
        };
        let part_of = |view: &WorkspaceView, area: AreaId| {
            view.parts.iter().find(|part| part.area == area).copied()
        };
        match view.dock_target {
            Some(DockTarget::Side { area, side }) => {
                let Some(part) = part_of(view, area) else {
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
                self.glide_preview(view, area, rect, style);
            }
            Some(DockTarget::Tabs {
                area,
                index,
                in_bar: true,
            }) => {
                view.glide = None;
                let Some(part) = part_of(view, area) else {
                    return;
                };
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
            }
            Some(DockTarget::Tabs { area, .. }) => {
                let Some(part) = part_of(view, area) else {
                    return;
                };
                let rect = part.panel.inset(style.drop_center_inset);
                self.glide_preview(view, area, rect, style);
            }
            None => view.glide = None,
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
                .with_corner_radius(style.corner_radius),
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
        let target = |x: f32, y: f32| {
            dock_target(
                &parts,
                &[],
                &workspace,
                editor,
                left,
                Vec2::new(x, y),
                None,
                32.0,
            )
        };
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
                index: 1,
                in_bar: false,
            })
        );
        // Over the top bar of a lone editor, without its tab bar: beside it, at the top.
        assert_eq!(
            target(600.0, 16.0),
            Some(DockTarget::Side {
                area: right,
                side: Side::Top
            })
        );
        // Alone in its area, the editor cannot dock beside it.
        assert_eq!(target(10.0, 300.0), None);
        assert_eq!(target(-5.0, 300.0), None);
    }
}
