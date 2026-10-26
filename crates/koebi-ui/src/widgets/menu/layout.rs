//! The layout of a menu panel: its columns (marks, icons, labels, shortcuts, submenu arrows),
//! its rows and its size; and the drawing of each row.

use koebi_core::{Color, Rect, Vec2};
use koebi_render::{LinePrimitive, RectPrimitive};

use super::{Mark, MenuItem, MenuStyle};
use crate::icon::Icon;
use crate::text::Align;
use crate::ui::Ui;

/// The columns of a panel and its natural size.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Columns {
    /// Room for check and radio marks.
    pub(super) mark: bool,
    /// Room for icons.
    pub(super) icon: bool,
    /// Room for submenu arrows.
    pub(super) arrow: bool,
    /// The widest shortcut.
    pub(super) shortcut: f32,
    /// The panel's natural size, borders included.
    pub(super) size: Vec2,
}

/// The height of `item`'s row.
pub(super) fn row_height(style: &MenuStyle, item: &MenuItem<'_>) -> f32 {
    match item {
        MenuItem::Separator => style.padding_vertical + style.separator_width,
        _ => style.item_height,
    }
}

/// Measures the panel of `items`: the columns its items need and its size.
pub(super) fn measure(ui: &mut Ui<'_>, items: &[MenuItem<'_>]) -> Columns {
    let styles = ui.styles();
    let style = &styles.menu;
    let (mut labels, mut columns) = (0.0_f32, Columns::default());
    for item in items {
        match item {
            MenuItem::Action(action) => {
                labels = labels.max(ui.text_width(action.label, &styles.text.label));
                if let Some(shortcut) = action.shortcut {
                    columns.shortcut = columns
                        .shortcut
                        .max(ui.text_width(shortcut, &styles.text.detail));
                }
                columns.mark |= action.mark != Mark::None;
                columns.icon |= action.icon.is_some();
            }
            MenuItem::Submenu(submenu) => {
                labels = labels.max(ui.text_width(submenu.label, &styles.text.label));
                columns.icon |= submenu.icon.is_some();
                columns.arrow = true;
            }
            MenuItem::Title(title) => {
                labels = labels.max(ui.text_width(title, &styles.text.detail));
            }
            MenuItem::Separator => {}
        }
    }
    let side = |present: bool| {
        if present {
            style.icon_size + style.icon_gap
        } else {
            0.0
        }
    };
    let shortcut = if columns.shortcut > 0.0 {
        style.column_gap + columns.shortcut
    } else {
        0.0
    };
    let arrow = if columns.arrow {
        style.icon_gap + style.icon_size
    } else {
        0.0
    };
    let inset = style.border_width + style.item_gap + style.item_padding;
    let width = 2.0 * inset + side(columns.mark) + side(columns.icon) + labels + shortcut + arrow;
    let rows: f32 = items.iter().map(|item| row_height(style, item)).sum();
    let height = rows + 2.0 * (style.border_width + style.padding_vertical);
    columns.size = Vec2::new(width.ceil(), height.ceil());
    columns
}

/// How a row shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RowLook {
    /// At rest.
    Normal,
    /// Under the pointer or the keyboard, or holding the open submenu.
    Highlighted,
    /// The current choice of a dropdown.
    Selected,
}

/// Draws `item` in `row`, the rectangle of its row inside the panel. `toward_left` points the
/// arrow of a submenu left, where it opens.
pub(super) fn draw_row(
    ui: &mut Ui<'_>,
    item: &MenuItem<'_>,
    row: Rect,
    columns: &Columns,
    look: RowLook,
    toward_left: bool,
) {
    let styles = ui.styles();
    let style = &styles.menu;
    let text = Rect::from_min_max(
        Vec2::new(row.min.x + style.item_padding, row.min.y),
        Vec2::new(
            (row.max.x - style.item_padding).max(row.min.x + style.item_padding),
            row.max.y,
        ),
    );
    match item {
        MenuItem::Separator => {
            let middle = row.center().y;
            ui.scene().push_line(&LinePrimitive::new(
                Vec2::new(text.min.x, middle),
                Vec2::new(text.max.x, middle),
                style.separator_width,
                style.separator_color,
            ));
        }
        MenuItem::Title(title) => {
            ui.label(
                title,
                text,
                &styles.text.detail,
                style.title_color,
                Align::Start,
            );
        }
        MenuItem::Action(_) | MenuItem::Submenu(_) => {
            let enabled = item.is_active();
            let background = match look {
                RowLook::Highlighted if enabled => Some(style.item_hover),
                RowLook::Selected => Some(style.item_selected),
                _ => None,
            };
            if let Some(background) = background {
                ui.scene().push_rect(
                    &RectPrimitive::new(row, background).with_corner_radius(style.corner_radius),
                );
            }
            let (label_color, icon_color, hint_color) = if enabled {
                let label = match look {
                    RowLook::Normal => style.label,
                    RowLook::Highlighted => style.label_hover,
                    RowLook::Selected => style.label_selected,
                };
                (label, style.icon_color, style.shortcut_color)
            } else {
                (
                    style.label_disabled,
                    style.label_disabled,
                    style.label_disabled,
                )
            };
            let mark_color = if enabled {
                style.mark_color
            } else {
                style.label_disabled
            };
            draw_parts(
                ui,
                item,
                text,
                columns,
                Colors {
                    label: label_color,
                    icon: icon_color,
                    hint: hint_color,
                    mark: mark_color,
                },
                toward_left,
            );
        }
    }
}

/// The colors of a row's parts.
#[derive(Clone, Copy, Debug)]
struct Colors {
    label: Color,
    icon: Color,
    hint: Color,
    mark: Color,
}

/// Draws the mark, icon, label, shortcut and arrow of an action or a submenu in `text`.
fn draw_parts(
    ui: &mut Ui<'_>,
    item: &MenuItem<'_>,
    text: Rect,
    columns: &Columns,
    colors: Colors,
    toward_left: bool,
) {
    let styles = ui.styles();
    let style = &styles.menu;
    let (label, icon, shortcut, mark) = match item {
        MenuItem::Action(action) => (action.label, action.icon, action.shortcut, action.mark),
        MenuItem::Submenu(submenu) => (submenu.label, submenu.icon, None, Mark::None),
        _ => return,
    };
    let slot = |x: f32| {
        Icon::centered(
            Rect::from_min_max(
                Vec2::new(x, text.min.y),
                Vec2::new(x + style.icon_size, text.max.y),
            ),
            style.icon_size,
        )
    };
    let mut x = text.min.x;
    if columns.mark {
        let bounds = slot(x);
        match mark {
            Mark::Check(true) => {
                Icon::Check.draw(ui.scene(), bounds, colors.mark, style.stroke_width);
            }
            Mark::Radio(true) => {
                let dot = Rect::from_center_size(bounds.center(), Vec2::splat(style.radio_size));
                ui.scene().push_rect(
                    &RectPrimitive::new(dot, colors.mark)
                        .with_corner_radius(style.radio_size * 0.5),
                );
            }
            _ => {}
        }
        x += style.icon_size + style.icon_gap;
    }
    if columns.icon {
        if let Some(icon) = icon {
            icon.draw(ui.scene(), slot(x), colors.icon, style.stroke_width);
        }
        x += style.icon_size + style.icon_gap;
    }
    let mut end = text.max.x;
    if columns.arrow {
        end -= style.icon_size;
        if matches!(item, MenuItem::Submenu(_)) {
            let arrow = if toward_left {
                Icon::ChevronLeft
            } else {
                Icon::ChevronRight
            };
            arrow.draw(ui.scene(), slot(end), colors.icon, style.stroke_width);
        }
        end -= style.icon_gap;
    }
    let body = Rect::from_min_max(Vec2::new(x, text.min.y), Vec2::new(end.max(x), text.max.y));
    if let Some(shortcut) = shortcut {
        ui.label(shortcut, body, &styles.text.detail, colors.hint, Align::End);
    }
    let label_end = if columns.shortcut > 0.0 {
        (end - columns.shortcut - style.column_gap).max(x)
    } else {
        end.max(x)
    };
    ui.label(
        label,
        Rect::from_min_max(Vec2::new(x, text.min.y), Vec2::new(label_end, text.max.y)),
        &styles.text.label,
        colors.label,
        Align::Start,
    );
}
