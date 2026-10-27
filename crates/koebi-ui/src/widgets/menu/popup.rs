//! [`Ui::popup`]: a floating panel of custom content that opens, places itself and closes like
//! a menu.

use koebi_core::{Rect, Vec2};
use koebi_render::{Corners, RectPrimitive};

use super::state::Anchor;
use super::{MENU, MENU_CLOSE, MenuState, OUTSIDE, at_point};
use crate::id::WidgetId;
use crate::routing::Layer;
use crate::ui::Ui;
use crate::widgets::parts::{PanelAlign, Placed, Side};

impl Ui<'_> {
    /// Shows a floating panel whose content is `size` when `state` is open: opened and placed
    /// like a menu (at a point, or beside a control, squaring the corners where they meet),
    /// with the menus' background, border and padding around the content. `add` draws the
    /// content in the rectangle it receives, and the popup returns what `add` returns.
    ///
    /// A press outside closes it, and so does [`MENU_CLOSE`] when nothing inside used it (a
    /// menu open in the popup closes first); moving away never does, so that its content can
    /// be dragged.
    pub fn popup<R>(
        &mut self,
        id: WidgetId,
        state: &mut MenuState,
        size: Vec2,
        bounds: Rect,
        add: impl FnOnce(&mut Self, Rect) -> R,
    ) -> Option<R> {
        let anchor = state.anchor.filter(|_| state.is_open())?;
        self.grab(MENU);
        let style = &self.styles().menu;
        let inset = Vec2::new(
            style.border_width + style.padding_horizontal,
            style.border_width + style.padding_vertical,
        );
        let total = size + inset * 2.0;
        let placed = match anchor {
            Anchor::Point(point) => Placed {
                rect: at_point(point, total, bounds),
                side: Side::Right,
                align: PanelAlign::Start,
            },
            Anchor::Control(control, placement) => {
                let placed = placement.place(control, total, bounds);
                state.placed = Some(placed);
                placed
            }
        };
        let mut result = None;
        self.on_layer(Layer::FLOATING, |ui| {
            let outside = ui.interact(id.with(OUTSIDE), bounds);
            if outside.pressed || outside.secondary_pressed {
                state.close();
                return;
            }
            ui.zone(placed.rect, MENU, |ui| {
                let radii = match state.control() {
                    Some(control) => {
                        placed.panel_corners(control, Corners::all(style.corner_radius))
                    }
                    None => Corners::all(style.corner_radius),
                };
                ui.scene().push_rect(
                    &RectPrimitive::new(placed.rect, style.background)
                        .with_corner_radii(radii)
                        .with_border(style.border_width, style.border_color),
                );
                let content = Rect::from_min_max(placed.rect.min + inset, placed.rect.max - inset);
                result = Some(ui.clipped(content, |ui| add(ui, content)));
            });
        });
        // Escape, unless a menu inside the popup used it to close itself.
        if state.is_open() && self.triggered(&MENU_CLOSE) {
            self.consume(&MENU_CLOSE);
            state.close();
            return None;
        }
        state.levels[0].rect = placed.rect;
        result.filter(|_| state.is_open())
    }
}
