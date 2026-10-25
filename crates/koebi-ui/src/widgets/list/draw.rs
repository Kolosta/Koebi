//! Drawing a list: rows with their zebra and state backgrounds, lines, fold arrows, marks,
//! labels and counts; the drop preview; the dragged rows' label near the pointer.

use core::f32::consts::FRAC_PI_2;

use koebi_core::input::CursorShape;
use koebi_core::{Color, Rect, Vec2};
use koebi_render::RectPrimitive;

use super::layout::{Preview, Row, decimal};
use super::state::ListState;
use super::{
    BACKGROUND, FOLDS, ItemMark, ListContextMenu, ListDelegate, ListItem, ListOptions,
    ListResponse, ListSelection, ListStyle, Metrics, ROWS, primary_held,
};
use crate::icon::Icon;
use crate::id::WidgetId;
use crate::input::PointerButton;
use crate::text::Align;
use crate::ui::Ui;

/// `color` at `opacity` of its alpha.
fn fade(color: Color, opacity: f32) -> Color {
    color.with_alpha(color.a * opacity)
}

impl Preview {
    /// The rectangle of the preview in a list's content scrolled to `origin`.
    fn rect(self, origin: Vec2, viewport: Rect, metrics: Metrics, style: &ListStyle) -> Rect {
        match self {
            Self::Line { y, depth } => {
                let x = metrics.start + f32::from(depth) * metrics.indent;
                let half = style.drop_line_width * 0.5;
                Rect::from_min_max(
                    Vec2::new(x, origin.y + y - half),
                    Vec2::new(viewport.max.x - style.padding, origin.y + y + half),
                )
            }
            Self::Frame { top } => Rect::from_min_max(
                Vec2::new(viewport.min.x, origin.y + top),
                Vec2::new(viewport.max.x, origin.y + top + metrics.row_height),
            )
            .inset(style.drop_frame_width * 0.5),
        }
    }
}

impl Ui<'_> {
    /// The rows in the viewport of a list scrolled to `origin`, their interaction, and the
    /// drop preview.
    #[allow(clippy::too_many_arguments, reason = "the parts of one list")]
    pub(super) fn list_rows(
        &mut self,
        id: WidgetId,
        origin: Vec2,
        viewport: Rect,
        state: &mut ListState,
        items: &[ListItem<'_>],
        options: &ListOptions,
        delegate: &mut impl ListDelegate,
        metrics: Metrics,
        response: &mut ListResponse,
    ) {
        let style = self.styles().list;
        // The empty space: a press clears the selection, a secondary press asks for a menu.
        let background = self.interact(id.with(BACKGROUND), viewport);
        let modifiers = self.input().modifiers();
        if background.pressed
            && options.selection != ListSelection::None
            && !primary_held(modifiers)
            && !modifiers.shift()
            && (!state.selected.is_empty() || state.active.is_some())
        {
            state.clear_selection();
            response.selection_changed = true;
        }
        if background.secondary_pressed && state.drag.is_none() {
            response.context_menu = Some(ListContextMenu {
                at: self
                    .input()
                    .button_pressed_at(PointerButton::Secondary)
                    .unwrap_or_default(),
                row: None,
            });
        }

        // The pressed row holds the pointer for the whole drag, even scrolled out of view.
        let holder = state.drag.map(|drag| drag.row);
        let cursor = match state.drag {
            Some(drag) if drag.started && !drag.cancelled && drag.target.is_some() => {
                CursorShape::Move
            }
            Some(drag) if drag.started && !drag.cancelled => CursorShape::NotAllowed,
            _ => CursorShape::Default,
        };
        let mut held = false;
        for position in 0..state.rows.len() {
            let row = state.rows[position];
            let item = &items[row.item];
            let top = origin.y + row.top;
            let bottom = top + metrics.row_height;
            let cut = bottom.min(origin.y + row.clip);
            if cut <= top || cut <= viewport.min.y || top >= viewport.max.y {
                continue;
            }
            let rect = Rect::from_min_max(
                Vec2::new(viewport.min.x, top),
                Vec2::new(viewport.max.x, bottom),
            );
            let shown = Rect::from_min_max(rect.min, Vec2::new(rect.max.x, cut));
            if options.zebra {
                let color = style.zebra[position % 2];
                if color.a > 0.0 {
                    self.scene.push_rect(&RectPrimitive::new(shown, color));
                }
            }
            let mut hovered = false;
            if !row.leaving {
                let row_id = id.with(ROWS).with(item.id);
                let input = if holder == Some(item.id) {
                    held = true;
                    self.interact_with_cursor(row_id, shown, cursor)
                } else {
                    self.interact(row_id, shown)
                };
                hovered = input.hovered;
                self.list_row_input(state, options, item, input, response);
            }
            let selectable = options.selection != ListSelection::None;
            let active = selectable && state.active == Some(item.id);
            let selected = selectable && state.is_selected(item.id);
            let highlight = match (active, selected, hovered) {
                (true, _, false) => Some(style.active),
                (true, _, true) => Some(style.active_hover),
                (false, true, false) => Some(style.selected),
                (false, true, true) => Some(style.selected_hover),
                (false, false, true) => Some(style.hover),
                (false, false, false) => None,
            };
            if let Some(color) = highlight {
                self.scene.push_rect(&RectPrimitive::new(shown, color));
            }
            if cut < bottom {
                // A row of an animating fold: cut at the fold's edge.
                self.clipped(shown, |ui| {
                    ui.list_row_content(
                        id, state, items, options, delegate, metrics, position, rect,
                    );
                });
            } else {
                self.list_row_content(id, state, items, options, delegate, metrics, position, rect);
            }
        }
        if let Some(row) = holder
            && !held
        {
            self.interact_with_cursor(id.with(ROWS).with(row), Rect::ZERO, cursor);
        }

        if let Some(drag) = state.drag
            && drag.started
            && !drag.cancelled
            && let Some((_, preview)) = drag.target
        {
            let rect = preview.rect(origin, viewport, metrics, &style);
            let primitive = match preview {
                Preview::Line { .. } => RectPrimitive::new(rect, style.drop_line)
                    .with_corner_radius(style.drop_line_width * 0.5),
                Preview::Frame { .. } => RectPrimitive::new(rect, Color::TRANSPARENT)
                    .with_border(style.drop_frame_width, style.drop_frame)
                    .with_corner_radius(style.drop_frame_radius),
            };
            self.scene.push_rect(&primitive);
        }
    }

    /// The content of the row at `position` (laid out in `rect`): lines, fold arrow, mark,
    /// end items, label and count.
    #[allow(clippy::too_many_arguments, reason = "the parts of one list")]
    fn list_row_content(
        &mut self,
        id: WidgetId,
        state: &mut ListState,
        items: &[ListItem<'_>],
        options: &ListOptions,
        delegate: &mut impl ListDelegate,
        metrics: Metrics,
        position: usize,
        rect: Rect,
    ) {
        let styles = self.styles();
        let style = styles.list;
        let row: Row = state.rows[position];
        let item = &items[row.item];
        let depth = if options.tree { item.depth } else { 0 };
        let indent = metrics.indent;

        if options.tree && options.lines {
            // A line along the children of each ancestor, through the middle of its arrow's
            // slot, leaving room below the ancestor and after its last child.
            let depth_at =
                |position: usize| state.rows.get(position).map(|row| items[row.item].depth);
            let previous = position.checked_sub(1).and_then(depth_at);
            let next = depth_at(position + 1).unwrap_or(0);
            for level in 0..depth {
                let x = (metrics.start + f32::from(level) * indent + indent * 0.5
                    - style.line_width * 0.5)
                    .round();
                let top = if previous == Some(level) {
                    rect.min.y + style.line_inset
                } else {
                    rect.min.y
                };
                let bottom = if next <= level {
                    rect.max.y - style.line_inset
                } else {
                    rect.max.y
                };
                if bottom > top {
                    self.scene.push_rect(&RectPrimitive::new(
                        Rect::from_min_max(
                            Vec2::new(x, top),
                            Vec2::new(x + style.line_width, bottom),
                        ),
                        style.line,
                    ));
                }
            }
        }

        let slot = metrics.start + f32::from(depth) * indent;
        let mut x = slot + indent;
        if row.parent {
            let slot = Rect::from_min_max(
                Vec2::new(slot, rect.min.y),
                Vec2::new(slot + indent, rect.max.y),
            );
            Icon::ChevronRight.draw_rotated(
                self.scene,
                Icon::centered(slot, style.fold_size),
                style.fold_color,
                styles.icon.stroke_width,
                row.openness * FRAC_PI_2,
            );
            if !row.leaving && self.interact(id.with(FOLDS).with(item.id), slot).clicked {
                let folded = !state.is_folded(item.id);
                if self.input().modifiers().shift() {
                    state.fold_subtree(items, row.item, folded);
                } else {
                    state.fold_animated(item.id, folded);
                }
            }
        }

        let moving = state.is_dragging()
            && state
                .dragged_ranges
                .iter()
                .any(|&(start, end)| (start..end).contains(&row.item));
        let ink = if moving {
            fade(style.label, style.drag_opacity)
        } else {
            style.label
        };
        let icon_size = styles.icon.size;
        let mark_slot = Rect::from_min_max(
            Vec2::new(x, rect.min.y),
            Vec2::new(x + icon_size, rect.max.y),
        );
        match item.mark {
            ItemMark::None => {}
            ItemMark::Icon(icon) => {
                icon.draw(
                    self.scene,
                    Icon::centered(mark_slot, icon_size),
                    ink,
                    styles.icon.stroke_width,
                );
                x += icon_size + style.gap;
            }
            ItemMark::Color(color) => {
                self.scene.push_rect(
                    &RectPrimitive::new(Icon::centered(mark_slot, style.swatch_size), color)
                        .with_corner_radius(style.swatch_radius),
                );
                x += icon_size + style.gap;
            }
        }

        let end = rect.max.x - style.padding;
        let used = delegate.row_end(
            self,
            item,
            Rect::from_min_max(Vec2::new(x, rect.min.y), Vec2::new(end, rect.max.y)),
        );
        let label_end = if used > 0.0 {
            end - used - style.gap
        } else {
            end
        };
        let mut digits = [0; 20];
        let count = (options.counts && row.hidden > 0).then(|| decimal(row.hidden, &mut digits));
        let count_width = count.map_or(0.0, |count| self.text_width(count, &styles.text.detail));
        let room = (label_end
            - x
            - if count.is_some() {
                count_width + style.gap
            } else {
                0.0
            })
        .max(0.0);
        let drawn = self.label(
            item.label,
            Rect::from_min_max(Vec2::new(x, rect.min.y), Vec2::new(x + room, rect.max.y)),
            &styles.text.label,
            ink,
            Align::Start,
        );
        if let Some(count) = count {
            let start = x + drawn + style.gap;
            self.label(
                count,
                Rect::from_min_max(
                    Vec2::new(start, rect.min.y),
                    Vec2::new(start + count_width, rect.max.y),
                ),
                &styles.text.detail,
                style.count,
                Align::Start,
            );
        }
    }

    /// The label of the dragged rows below and right of the pointer (the drop preview stays
    /// in sight), with their number when there are several.
    pub(super) fn list_ghost(
        &mut self,
        state: &ListState,
        items: &[ListItem<'_>],
        metrics: Metrics,
    ) {
        if !state.is_dragging() {
            return;
        }
        let (Some(point), Some(first)) = (self.input().pointer(), state.dragged.first()) else {
            return;
        };
        let Some(label) = items
            .iter()
            .find(|item| item.id == *first)
            .map(|item| item.label)
        else {
            return;
        };
        let styles = self.styles();
        let style = styles.list;
        let mut digits = [0; 20];
        let count = (state.dragged.len() > 1).then(|| decimal(state.dragged.len(), &mut digits));
        let label_width = self
            .text_width(label, &styles.text.label)
            .min(metrics.row_height * 12.0);
        let count_width = count.map_or(0.0, |count| {
            self.text_width(count, &styles.text.detail) + style.gap
        });
        let ghost = Rect::from_origin_size(
            Vec2::new(point.x + style.indent, point.y + metrics.row_height * 0.5),
            Vec2::new(
                label_width + count_width + 2.0 * style.padding,
                metrics.row_height,
            ),
        );
        self.scene.push_rect(
            &RectPrimitive::new(ghost, fade(style.drag_background, style.drag_opacity))
                .with_corner_radius(style.drop_frame_radius),
        );
        let text = Rect::from_min_max(
            Vec2::new(ghost.min.x + style.padding, ghost.min.y),
            Vec2::new(ghost.min.x + style.padding + label_width, ghost.max.y),
        );
        let ink = fade(style.label, style.drag_opacity);
        self.label(label, text, &styles.text.label, ink, Align::Start);
        if let Some(count) = count {
            let rect = Rect::from_min_max(
                Vec2::new(text.max.x + style.gap, ghost.min.y),
                Vec2::new(ghost.max.x - style.padding, ghost.max.y),
            );
            let ink = fade(style.count, style.drag_opacity);
            self.label(count, rect, &styles.text.detail, ink, Align::Start);
        }
    }
}
