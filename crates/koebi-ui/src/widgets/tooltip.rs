//! Tooltips: help shown below a control once the pointer rests on it, above everything,
//! inspired by Blender's.
//!
//! - **When**: once the pointer rested on the control for the input settings' tooltip delay
//!   (`input.tooltip.delay`); moving more than the drag threshold before then starts the wait
//!   again. Right after another tooltip closed (moving along a row of controls), the next one
//!   comes after the short delay (`input.tooltip.delay-short`).
//! - **Where**: below the control, `tooltip.offset` from it, never over it; its text starts at
//!   the pointer, as Blender's does. Kept inside the window (`tooltip.margin` from its edges):
//!   shifted left near the right edge, above the control near the bottom. Placed once when it
//!   appears, it never moves while it shows.
//! - **Until**: the pointer leaves the control, or a press or the wheel: it then stays hidden
//!   until the pointer reaches another control.
//! - **Look**: an inverted background (`semantic.background.inverse`), light where the theme is
//!   dark, so that it stands out on every surface and on menus; its text wraps past
//!   `tooltip.max-width`. It never takes the pointer.

use core::time::Duration;

use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_render::RectPrimitive;
use koebi_tokens::component;

use crate::id::WidgetId;
use crate::routing::Layer;
use crate::text::Align;
use crate::ui::Ui;

/// Colors and metrics of tooltips, from the `component.tooltip.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TooltipStyle {
    /// Background.
    pub background: Color,
    /// Text color.
    pub label: Color,
    /// Border color.
    pub border_color: Color,
    /// Border width.
    pub border_width: f32,
    /// Corner radius.
    pub corner_radius: f32,
    /// Space between the sides and the text.
    pub padding_horizontal: f32,
    /// Space above and below the text.
    pub padding_vertical: f32,
    /// Space between the control and the tooltip.
    pub offset: f32,
    /// Space kept from the window's edges.
    pub margin: f32,
    /// Widest the text goes before it wraps.
    pub max_width: f32,
}

impl TooltipStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        Self {
            background: tokens.get(c::TOOLTIP_BACKGROUND_COLOR_DEFAULT),
            label: tokens.get(c::TOOLTIP_LABEL_COLOR_DEFAULT),
            border_color: tokens.get(c::TOOLTIP_BORDER_COLOR_DEFAULT),
            border_width: tokens.get(c::TOOLTIP_BORDER_WIDTH_DEFAULT),
            corner_radius: tokens.get(c::TOOLTIP_CORNER_RADIUS_DEFAULT),
            padding_horizontal: tokens.get(c::TOOLTIP_PADDING_HORIZONTAL_DEFAULT),
            padding_vertical: tokens.get(c::TOOLTIP_PADDING_VERTICAL_DEFAULT),
            offset: tokens.get(c::TOOLTIP_OFFSET_DEFAULT),
            margin: tokens.get(c::TOOLTIP_MARGIN_DEFAULT),
            max_width: tokens.get(c::TOOLTIP_MAX_WIDTH_DEFAULT),
        }
    }
}

/// The tooltip showing: whose, and where, fixed while it shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Shown {
    pub(crate) widget: WidgetId,
    pub(crate) rect: Rect,
    /// Its text wraps.
    pub(crate) wraps: bool,
}

/// What tooltips remember between frames: the widget under the pointer, since when the pointer
/// rests on it and where, the tooltip showing, and when the last one closed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct TooltipState {
    /// The widget on top under the pointer.
    pub(crate) target: Option<WidgetId>,
    /// Since when, and from where, the pointer rests on it.
    pub(crate) since: Duration,
    pub(crate) rest: Option<Vec2>,
    /// A press or the wheel hid the tooltip of the widget under the pointer.
    pub(crate) dismissed: bool,
    /// The tooltip showing.
    pub(crate) shown: Option<Shown>,
    /// Whether the tooltip showing was drawn in the last frame: one that is not drawn closes.
    pub(crate) drawn: bool,
    /// When a tooltip last closed.
    pub(crate) closed: Option<Duration>,
}

impl TooltipState {
    /// Follows the pointer at the start of a frame at `now`: over `hovered`, at `pointer`,
    /// pressing or turning the wheel (`dismiss`), a move past `threshold` before the tooltip
    /// shows starting the wait again.
    pub(crate) fn update(
        &mut self,
        hovered: Option<WidgetId>,
        pointer: Option<Vec2>,
        dismiss: bool,
        threshold: f32,
        now: Duration,
    ) {
        // A tooltip not drawn in the last frame (its control went) closes.
        if !core::mem::take(&mut self.drawn) {
            self.close(now);
        }
        if hovered != self.target {
            self.close(now);
            self.target = hovered;
            self.since = now;
            self.rest = pointer;
            self.dismissed = false;
        }
        if dismiss {
            self.dismissed = true;
            self.close(now);
        }
        if self.shown.is_none()
            && let (Some(rest), Some(pointer)) = (self.rest, pointer)
            && (pointer - rest).length() > threshold
        {
            self.since = now;
            self.rest = Some(pointer);
        }
    }

    fn close(&mut self, now: Duration) {
        if self.shown.take().is_some() {
            self.closed = Some(now);
        }
    }
}

impl Ui<'_> {
    /// Shows `text` below the control `anchor` (its rectangle) once the pointer rests on the
    /// widget `id`, kept inside the window. Call it after the widget, every frame.
    pub fn tooltip(&mut self, id: WidgetId, text: &str, anchor: Rect) {
        let now = self.input().time();
        let tips = self.state.tooltip;
        if tips.target != Some(id) || tips.dismissed || self.state.active().is_some() {
            return;
        }
        let shown = if let Some(shown) = tips.shown.filter(|shown| shown.widget == id) {
            shown
        } else {
            let input = &self.styles().input;
            // Right after another one closed: the next comes sooner.
            let recent = tips
                .closed
                .is_some_and(|closed| now.saturating_sub(closed) <= input.tooltip_delay);
            let delay = if recent {
                input.tooltip_delay_short
            } else {
                input.tooltip_delay
            };
            let due = tips.since.saturating_add(delay);
            if now < due {
                self.request_redraw_at(due);
                return;
            }
            let Some(pointer) = self.input().pointer() else {
                return;
            };
            let shown = self.place_tooltip(id, text, anchor, pointer);
            self.state.tooltip.shown = Some(shown);
            shown
        };
        self.state.tooltip.drawn = true;
        self.draw_tooltip(text, shown);
    }

    /// Where a tooltip of `text` for the control `anchor` goes, the pointer at `pointer`: its
    /// text starting at the pointer, below the control or above it near the window's bottom,
    /// inside the window.
    fn place_tooltip(&mut self, id: WidgetId, text: &str, anchor: Rect, pointer: Vec2) -> Shown {
        let styles = self.styles();
        let style = styles.tooltip;
        let font = &styles.text.body;
        let frame = Vec2::new(
            style.padding_horizontal + style.border_width,
            style.padding_vertical + style.border_width,
        );
        let window = self
            .state
            .window()
            .unwrap_or(Rect::from_min_max(Vec2::splat(-1.0e6), Vec2::splat(1.0e6)));
        let inner = window.inset(style.margin);
        // As wide as the tokens let it, and never wider than the window.
        let room = (style.max_width.min(inner.width()) - 2.0 * frame.x).max(1.0);
        let line = self.text_width(text, font);
        let wraps = line > room;
        let text_size = if wraps {
            Vec2::new(room, self.paragraph_height(text, room, font))
        } else {
            Vec2::new(line, self.line_height(font))
        };
        let size = Vec2::new(
            (text_size.x + 2.0 * frame.x).ceil(),
            (text_size.y + 2.0 * frame.y).ceil(),
        );
        // The text starts at the pointer; past the right edge, it shifts left.
        let x = (pointer.x - frame.x)
            .min(inner.max.x - size.x)
            .max(inner.min.x);
        // Below the control; where the window ends first, above it.
        let below = anchor.max.y + style.offset;
        let above = anchor.min.y - style.offset - size.y;
        let y = if below + size.y <= inner.max.y || above < inner.min.y {
            below.min(inner.max.y - size.y).max(inner.min.y)
        } else {
            above
        };
        let rect = Rect::from_origin_size(Vec2::new(x.round(), y.round()), size);
        Shown {
            widget: id,
            rect,
            wraps,
        }
    }

    fn draw_tooltip(&mut self, text: &str, shown: Shown) {
        let styles = self.styles();
        let style = styles.tooltip;
        let font = &styles.text.body;
        self.on_layer(Layer::OVERLAY, |ui| {
            ui.scene().push_rect(
                &RectPrimitive::new(shown.rect, style.background)
                    .with_corner_radius(style.corner_radius)
                    .with_border(style.border_width, style.border_color),
            );
            let inner = shown.rect.inset(style.border_width);
            let text_rect = Rect::from_min_max(
                Vec2::new(
                    inner.min.x + style.padding_horizontal,
                    inner.min.y + style.padding_vertical,
                ),
                Vec2::new(
                    inner.max.x - style.padding_horizontal,
                    inner.max.y - style.padding_vertical,
                ),
            );
            if shown.wraps {
                ui.paragraph(text, text_rect, font, style.label);
            } else {
                ui.label(text, text_rect, font, style.label, Align::Start);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;
    use koebi_text::Fonts;

    use super::*;
    use crate::{Input, Styles, UiState};

    const BUTTON: WidgetId = WidgetId::new("button");
    const OTHER: WidgetId = WidgetId::new("other");

    fn window() -> Rect {
        Rect::from_origin_size(Vec2::ZERO, Vec2::new(300.0, 200.0))
    }

    /// Frames of two controls side by side at `origin`, each with a tooltip.
    struct Harness {
        styles: Styles,
        state: UiState,
        input: Input,
        origin: Vec2,
        text: &'static str,
    }

    impl Harness {
        fn new(origin: Vec2) -> Self {
            let mut state = UiState::new();
            state.set_fonts(Fonts::with_embedded());
            state.set_window(window());
            Self {
                styles: crate::test_styles(),
                state,
                input: Input::new(),
                origin,
                text: "Adds a layer",
            }
        }

        fn button(&self) -> Rect {
            Rect::from_origin_size(self.origin, Vec2::new(40.0, 20.0))
        }

        fn other(&self) -> Rect {
            Rect::from_origin_size(self.origin + Vec2::new(40.0, 0.0), Vec2::new(40.0, 20.0))
        }

        fn frame(&mut self) -> Scene {
            let mut scene = Scene::new();
            let (button, other) = (self.button(), self.other());
            {
                let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
                ui.interact(BUTTON, button);
                ui.tooltip(BUTTON, self.text, button);
                ui.interact(OTHER, other);
                ui.tooltip(OTHER, "Removes the layer", other);
            }
            self.input.end_frame();
            scene
        }

        /// Rests the pointer at `at` until the tooltip is due, then draws a frame.
        fn rest(&mut self, at: Vec2) -> Scene {
            // Two frames: hovering reads the regions of the frame before.
            self.input.pointer_moved(at);
            self.frame();
            self.frame();
            let later = self.input.time() + self.styles.input.tooltip_delay;
            self.input.set_time(later);
            self.frame()
        }

        fn shown(&self) -> Option<Shown> {
            self.state.tooltip.shown
        }
    }

    #[test]
    fn a_tooltip_shows_below_its_control_after_the_delay_and_stays_put() {
        let mut harness = Harness::new(Vec2::new(20.0, 20.0));
        harness.frame();
        harness.input.pointer_moved(Vec2::new(30.0, 30.0));
        harness.frame();
        assert_eq!(harness.frame().glyph_count(), 0);
        assert_eq!(
            harness.state.redraw_at(),
            Some(harness.styles.input.tooltip_delay)
        );
        let scene = harness.rest(Vec2::new(30.0, 30.0));
        assert!(scene.glyph_count() > 5, "{}", scene.glyph_count());
        let shown = harness.shown().expect("shown");
        let style = harness.styles.tooltip;
        // Below the control, the offset apart, never over it; its text starts at the pointer.
        assert!((shown.rect.min.y - (harness.button().max.y + style.offset)).abs() < 1.0);
        let text_x = shown.rect.min.x + style.padding_horizontal + style.border_width;
        assert!((text_x - 30.0).abs() < 1.0, "{shown:?}");
        // The pointer moves within the control: the tooltip does not.
        harness
            .input
            .pointer_moved(Vec2::new(55.0 - 20.0 + 20.0, 35.0));
        harness.frame();
        assert_eq!(harness.shown().map(|shown| shown.rect), Some(shown.rect));
    }

    #[test]
    fn tooltips_stay_inside_the_window_and_go_above_near_its_bottom() {
        // At the bottom right: shifted left, above the control.
        let mut harness = Harness::new(Vec2::new(200.0, 175.0));
        harness.text = "A rather long help that would leave the window";
        harness.rest(Vec2::new(235.0, 185.0));
        let shown = harness.shown().expect("shown");
        let style = harness.styles.tooltip;
        let inner = window().inset(style.margin);
        assert!(shown.rect.max.x <= inner.max.x + 1e-3 && shown.rect.min.x >= inner.min.x);
        assert!(
            shown.rect.max.y <= harness.button().min.y - style.offset + 1.0,
            "{shown:?}"
        );
    }

    #[test]
    fn a_move_before_it_shows_starts_the_wait_again() {
        let mut harness = Harness::new(Vec2::new(20.0, 20.0));
        harness.frame();
        harness.input.pointer_moved(Vec2::new(22.0, 30.0));
        harness.frame();
        let delay = harness.styles.input.tooltip_delay;
        // Just before it is due, a move past the drag threshold within the control.
        harness
            .input
            .set_time(delay.saturating_sub(Duration::from_millis(10)));
        harness.input.pointer_moved(Vec2::new(50.0, 30.0));
        harness.frame();
        harness.input.set_time(delay);
        assert_eq!(harness.frame().glyph_count(), 0);
        harness.input.set_time(delay * 2);
        assert!(harness.frame().glyph_count() > 5);
    }

    #[test]
    fn a_press_or_the_wheel_hides_it_until_another_control() {
        let mut harness = Harness::new(Vec2::new(20.0, 20.0));
        harness.frame();
        harness.rest(Vec2::new(30.0, 30.0));
        assert!(harness.shown().is_some());
        harness.input.primary_button(true, Vec2::new(30.0, 30.0));
        harness.input.primary_button(false, Vec2::new(30.0, 30.0));
        harness.frame();
        let later = harness.input.time() + harness.styles.input.tooltip_delay * 3;
        harness.input.set_time(later);
        assert_eq!(harness.frame().glyph_count(), 0);
        // The neighbour's comes, then the wheel hides it.
        harness.rest(Vec2::new(70.0, 30.0));
        assert_eq!(harness.shown().map(|shown| shown.widget), Some(OTHER));
        harness.input.scrolled(Vec2::new(0.0, 1.0));
        harness.frame();
        assert_eq!(harness.shown(), None);
    }

    #[test]
    fn the_next_tooltip_comes_sooner() {
        let mut harness = Harness::new(Vec2::new(20.0, 20.0));
        harness.frame();
        harness.rest(Vec2::new(30.0, 30.0));
        // Onto the neighbour: its tooltip after the short delay.
        harness.input.pointer_moved(Vec2::new(70.0, 30.0));
        harness.frame();
        let soon = harness.input.time() + harness.styles.input.tooltip_delay_short;
        harness.input.set_time(soon);
        let scene = harness.frame();
        assert!(scene.glyph_count() > 5);
        assert_eq!(harness.shown().map(|shown| shown.widget), Some(OTHER));
    }

    #[test]
    fn long_help_wraps_within_the_widest_a_tooltip_goes() {
        let mut harness = Harness::new(Vec2::new(20.0, 20.0));
        harness.styles.tooltip.max_width = 120.0;
        harness.text = "A long help that takes several lines once it wraps";
        harness.rest(Vec2::new(30.0, 30.0));
        let shown = harness.shown().expect("shown");
        assert!(shown.wraps);
        assert!(shown.rect.width() <= 120.0 + 1.0);
        assert!(shown.rect.height() > 2.0 * harness.styles.tooltip.padding_vertical + 20.0);
    }
}
