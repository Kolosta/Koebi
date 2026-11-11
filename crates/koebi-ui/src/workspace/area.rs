//! The panels of the areas: a tab bar when an area stacks several editors, the top bar with
//! the editor type dropdown and the editor's own items, and the content.

use koebi_core::{Color, Rect, Vec2};
use koebi_render::{Corners, RectPrimitive};
use koebi_workspace::{AreaId, EditorId, Workspace};

use super::dock::dock_cursor;
use super::geometry::Frame;
use super::{EditorHost, Operation, WorkspaceStyle, WorkspaceView};
use crate::bar::BarLayout;
use crate::icon::icon_bounds;
use crate::id::WidgetId;
use crate::text::Align;
use crate::ui::Ui;
use crate::widgets::{Dropdown, MenuItem, MenuState};

const TAB: WidgetId = WidgetId::new("workspace-tab");
const TOP_BAR: WidgetId = WidgetId::new("workspace-top-bar");
const EDITOR_TYPE: WidgetId = WidgetId::new("workspace-editor-type");

/// The editor type dropdown lists at most this many types.
const MAX_KINDS: usize = 16;

/// Where the parts of an area lie this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct AreaParts {
    pub(super) area: AreaId,
    pub(super) panel: Rect,
    pub(super) tabs: Option<Rect>,
    pub(super) bar: Rect,
    pub(super) content: Rect,
}

/// A tab drawn this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TabPart {
    pub(super) area: AreaId,
    pub(super) editor: EditorId,
    pub(super) rect: Rect,
}

/// `rect` with rounded top corners and square bottom corners.
fn rounded_top(ui: &mut Ui<'_>, rect: Rect, color: Color, radius: f32) {
    ui.scene
        .push_rect(&RectPrimitive::new(rect, color).with_corner_radius(radius));
    let radius = radius.min(rect.height() * 0.5);
    ui.scene.push_rect(&RectPrimitive::new(
        Rect::from_min_max(Vec2::new(rect.min.x, rect.max.y - radius), rect.max),
        color,
    ));
}

impl Ui<'_> {
    /// Draws every area, and records where their parts and tabs lie for docking.
    pub(super) fn workspace_areas(
        &mut self,
        view: &mut WorkspaceView,
        workspace: &mut Workspace,
        frame: Frame,
        style: &WorkspaceStyle,
        host: &mut dyn EditorHost,
        window: Rect,
    ) {
        view.parts.clear();
        view.tabs.clear();
        let mut activate = None;
        let mut change_kind = None;
        for index in 0..view.areas.len() {
            let (area, rect) = view.areas[index];
            let panel = frame.panel(rect);
            let Some(stack) = workspace.stack(area) else {
                continue;
            };
            if panel.is_empty() {
                continue;
            }
            let editor = stack.active();
            // A lone editor's tab bar slides in while a drag waits over its top.
            let permanent = stack.len() > 1 || style.tab_show_solo;
            let opening = view
                .reveal
                .filter(|reveal| reveal.area == area)
                .map_or(0.0, |reveal| reveal.amount);
            let show_tabs = permanent || opening > 0.01;
            let tab_bar_height = if permanent {
                style.tab_bar_height
            } else {
                style.tab_bar_height * opening
            };
            self.scene.push_rect(
                &RectPrimitive::new(panel, style.background)
                    .with_corner_radius(style.corner_radius),
            );
            // The area cuts all it holds at its corners: what it holds does not need to know.
            let radii = Corners::all(style.corner_radius);
            let (tabs, bar, content) = self.clipped_rounded(panel, radii, |ui| {
                let mut top = panel.min.y;
                let tabs = show_tabs.then(|| {
                    let bar = Rect::from_min_max(
                        Vec2::new(panel.min.x, top),
                        Vec2::new(panel.max.x, (top + tab_bar_height).min(panel.max.y)),
                    );
                    top = bar.max.y;
                    bar
                });
                let bar = Rect::from_min_max(
                    Vec2::new(panel.min.x, top),
                    Vec2::new(panel.max.x, (top + style.top_bar_height).min(panel.max.y)),
                );
                let content = Rect::from_min_max(Vec2::new(panel.min.x, bar.max.y), panel.max);

                if let Some(tabs) = tabs {
                    ui.scene
                        .push_rect(&RectPrimitive::new(tabs, style.tab_bar_background));
                    let mut x = tabs.min.x + style.tab_gap;
                    let font = ui.styles().text.label.clone();
                    let icon_size = ui.styles().icon.size;
                    for (position, &tab) in stack.editors().iter().enumerate() {
                        let title = host.title(tab);
                        let icon = host.icon(tab);
                        // The icon, then the title after a gap as wide as the tab's padding.
                        let lead = icon.map_or(0.0, |_| icon_size + style.tab_padding);
                        let width = ui.text_width(title, &font) + 2.0 * style.tab_padding + lead;
                        let rect = Rect::from_min_max(
                            Vec2::new(x, tabs.min.y + style.tab_bar_padding),
                            Vec2::new((x + width).min(tabs.max.x), tabs.max.y),
                        );
                        x += width + style.tab_gap;
                        let id = TAB.with(u64::from(tab.get()));
                        let response = match dock_cursor(view, tab) {
                            Some(cursor) => ui.interact_with_cursor(id, rect, cursor),
                            None => ui.interact(id, rect),
                        };
                        let active = position == stack.active_index();
                        let background = if active {
                            style.tab_active
                        } else if response.hovered {
                            style.tab_hover
                        } else {
                            style.tab_background
                        };
                        rounded_top(ui, rect, background, style.corner_radius);
                        let ink = if active {
                            style.tab_label_active
                        } else {
                            style.tab_label
                        };
                        let text = rect.inset(style.tab_padding.min(rect.width() * 0.5));
                        // One interface unit high: the tab reaches down to the top bar below it.
                        let text = Rect::from_min_max(
                            Vec2::new(text.min.x, rect.min.y),
                            Vec2::new(text.max.x, rect.max.y - style.tab_bar_padding),
                        );
                        ui.clipped(tabs, |ui| {
                            let mut text = text;
                            if let Some(icon) = icon {
                                let slot = Rect::from_min_max(
                                    text.min,
                                    Vec2::new(text.min.x + icon_size, text.max.y),
                                );
                                ui.icon(icon_bounds(slot, icon_size), icon, ink);
                                text.min.x += lead;
                            }
                            ui.label(title, text, &font, ink, Align::Start);
                        });
                        if response.pressed
                            && view.operation.is_none()
                            && let Some(point) = ui.input().pointer()
                        {
                            view.operation = Some(Operation::Dock {
                                editor: tab,
                                from: area,
                                origin: point,
                                started: false,
                            });
                        }
                        if response.clicked
                            && matches!(
                                view.operation,
                                Some(Operation::Dock { started: false, .. })
                            )
                        {
                            activate = Some((area, position));
                            view.operation = None;
                        }
                        view.tabs.push(TabPart {
                            area,
                            editor: tab,
                            rect,
                        });
                    }
                }

                ui.scene.push_rect(&RectPrimitive::new(bar, style.top_bar));
                // The bar's background drags the editor; its items, drawn after, take presses first.
                let id = TOP_BAR.with(u64::from(area.get()));
                let background = match dock_cursor(view, editor) {
                    Some(cursor) => ui.interact_with_cursor(id, bar, cursor),
                    None => ui.interact(id, bar),
                };
                if background.pressed
                    && view.operation.is_none()
                    && let Some(point) = ui.input().pointer()
                {
                    view.operation = Some(Operation::Dock {
                        editor,
                        from: area,
                        origin: point,
                        started: false,
                    });
                }
                let items = Rect::from_min_max(
                    bar.min + Vec2::splat(style.top_bar_padding),
                    bar.max - Vec2::splat(style.top_bar_padding),
                );
                let mut layout = BarLayout::new(items, style.top_bar_gap);
                if let Some(kind) = ui.editor_type(view, host, editor, &mut layout, window) {
                    change_kind = Some((editor, kind));
                }
                host.top_bar(ui, editor, layout);
                if !content.is_empty() {
                    ui.clipped(content, |ui| host.content(ui, editor, content));
                }
                (tabs, bar, content)
            });
            view.parts.push(AreaParts {
                area,
                panel,
                tabs,
                bar,
                content,
            });
        }
        if let Some((area, position)) = activate
            && let Err(error) = workspace.activate(area, position)
        {
            koebi_core::warn!("cannot show the tab: {error}");
        }
        if let Some((editor, kind)) = change_kind {
            host.set_kind(editor, kind);
        }
    }

    /// The editor type dropdown at the start of a top bar, or the editor's title when the
    /// host offers no types. Returns the type chosen this frame.
    fn editor_type(
        &mut self,
        view: &mut WorkspaceView,
        host: &mut dyn EditorHost,
        editor: EditorId,
        layout: &mut BarLayout,
        window: Rect,
    ) -> Option<usize> {
        let styles = self.styles();
        let font = styles.text.label.clone();
        let kinds = host.kinds();
        if kinds.is_empty() {
            let title = layout.remaining();
            let color = styles.workspace.title;
            self.label(host.title(editor), title, &font, color, Align::Start);
            return None;
        }
        let count = kinds.len().min(MAX_KINDS);
        let current = host.kind(editor).min(count - 1);
        let mut items = [MenuItem::Separator; MAX_KINDS];
        let (mut widest, mut widest_label) = (0.0_f32, "");
        let icons = host.kind_icons();
        for (index, (item, label)) in items.iter_mut().zip(kinds).enumerate() {
            *item = MenuItem::action(label).selected(index == current);
            if let Some(&icon) = icons.get(index) {
                *item = item.icon(icon);
            }
            let width = self.text_width(label, &font);
            if width > widest {
                (widest, widest_label) = (width, label);
            }
        }
        let id = EDITOR_TYPE.with(u64::from(editor.get()));
        // As wide as the widest kind: the control keeps its size whatever the choice.
        let width = Dropdown::measure(self, widest_label).x;
        let rect = layout.left(width);
        // One menu opens at a time: the others get a closed one.
        let mut menu = if view.type_menu_editor == Some(editor) {
            core::mem::take(&mut view.type_menu)
        } else {
            MenuState::new()
        };
        let mut dropdown = Dropdown::new(id, &mut menu, &items[..count]).label(kinds[current]);
        if let Some(&icon) = icons.get(current) {
            dropdown = dropdown.icon(icon);
        }
        let chosen = dropdown
            .show(self, rect, window)
            .map(|choice| choice.index());
        if menu.is_open() {
            view.type_menu = menu;
            view.type_menu_editor = Some(editor);
        } else if view.type_menu_editor == Some(editor) {
            view.type_menu_editor = None;
        }
        chosen
    }
}
