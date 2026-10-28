//! Scroll areas: content larger than its rectangle, scrolled by the wheel or a trackpad and by
//! overlay scrollbars.
//!
//! The scrollbars live in the area's margin and take no space from the content: a thin grab
//! at rest that grows and brightens as the pointer comes near, shown only when the content
//! overflows. Dragging the grab scrolls; a press on the track brings the grab under the
//! pointer.

use core::time::Duration;

use koebi_core::{Rect, Vec2};
use koebi_design::Resolve;
use koebi_design::value::Curve;
use koebi_input::Context;
use koebi_tokens::component;

use crate::id::WidgetId;
use crate::style::StateColors;
use crate::ui::Ui;
use crate::widgets::scrollbar::{ScrollAxis, Scrollbar, ScrollbarState};

/// A scroll area: its keys apply where the pointer is over it.
pub const SCROLL: Context = Context::new("scroll");

/// Which ways a scroll area scrolls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScrollAxes {
    /// Up and down: lists.
    #[default]
    Vertical,
    /// Left and right.
    Horizontal,
    /// Both: timelines, wide tables.
    Both,
}

impl ScrollAxes {
    fn has(self, axis: usize) -> bool {
        matches!(
            (self, axis),
            (Self::Both, _) | (Self::Vertical, 1) | (Self::Horizontal, 0)
        )
    }
}

/// Colors and metrics of [`Ui::scroll_area`], from the `component.scrollbar.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollStyle {
    /// Width of the margin the grab lives in, on the right and at the bottom.
    pub margin: f32,
    /// Width of the grab at rest.
    pub width_rest: f32,
    /// Width of the grab under the pointer or dragged.
    pub width_hover: f32,
    /// How far from the grab the pointer starts to grow it.
    pub proximity: f32,
    /// Space between the track's ends and the area's edges.
    pub padding: f32,
    /// Corner radius of the grab (at most half its width).
    pub corner_radius: f32,
    /// Grab color at rest, near the pointer, and dragged.
    pub grab: StateColors,
    /// How far one notch of the wheel scrolls.
    pub wheel_step: f32,
    /// Time the grab takes to grow from rest to its full width, or back.
    pub grow_duration: Duration,
    /// Curve of the grab growing and shrinking.
    pub grow_curve: Curve,
}

impl ScrollStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            margin: tokens.get(component::SCROLLBAR_OVERLAY_MARGIN_DEFAULT),
            width_rest: tokens.get(component::SCROLLBAR_OVERLAY_WIDTH_REST),
            width_hover: tokens.get(component::SCROLLBAR_OVERLAY_WIDTH_HOVER),
            proximity: tokens.get(component::SCROLLBAR_OVERLAY_PROXIMITY_DEFAULT),
            padding: tokens.get(component::SCROLLBAR_OVERLAY_PADDING_DEFAULT),
            corner_radius: tokens.get(component::SCROLLBAR_CORNER_RADIUS_DEFAULT),
            grab: StateColors {
                normal: tokens.get(component::SCROLLBAR_GRAB_COLOR_DEFAULT),
                hovered: tokens.get(component::SCROLLBAR_GRAB_COLOR_HOVER),
                pressed: tokens.get(component::SCROLLBAR_GRAB_COLOR_PRESSED),
            },
            wheel_step: tokens.get(component::SCROLLBAR_WHEEL_STEP_DEFAULT),
            grow_duration: tokens.get(component::SCROLLBAR_OVERLAY_GROW_DURATION),
            grow_curve: tokens.get(component::SCROLLBAR_OVERLAY_GROW_CURVE),
        }
    }
}

/// Where a scroll area is scrolled to, and its scrollbars' animation. Keep one per area.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollState {
    /// How far the content is scrolled, in logical pixels: x then y.
    offset: Vec2,
    /// The scrollbars, horizontal first.
    bars: [ScrollbarState; 2],
}

impl ScrollState {
    /// Scrolled to the start.
    pub const fn new() -> Self {
        Self {
            offset: Vec2::ZERO,
            bars: [ScrollbarState::new(); 2],
        }
    }

    /// How far the content is scrolled, in logical pixels.
    pub fn offset(&self) -> Vec2 {
        self.offset
    }

    /// Scrolls to `offset`; the next frame keeps it within the content.
    pub fn set_offset(&mut self, offset: Vec2) {
        self.offset = offset;
    }
}

/// What a scroll area gives its content.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollContent {
    /// Where the content's top-left corner lies, scrolled.
    pub origin: Vec2,
    /// The visible part of the area the content may use: the area without the scrollbars'
    /// margins.
    pub viewport: Rect,
}

/// The part of a scroll area over `rect` its content may use: without the margins the
/// scrollbars of `axes` live in, kept even when nothing overflows so that the content never
/// shifts when a scrollbar appears.
pub(crate) fn viewport(rect: Rect, axes: ScrollAxes, style: &ScrollStyle) -> Rect {
    Rect::from_min_max(
        rect.min,
        Vec2::new(
            rect.max.x - if axes.has(1) { style.margin } else { 0.0 },
            rect.max.y - if axes.has(0) { style.margin } else { 0.0 },
        ),
    )
}

impl Ui<'_> {
    /// A scroll area over `rect`, holding content `content` big (logical pixels): `add` draws
    /// the content from the [`ScrollContent`] origin, clipped to `rect`. The wheel (and
    /// trackpad) scroll it when the pointer is over it; overlay scrollbars in its margin show
    /// when the content overflows. Returns what `add` returns.
    pub fn scroll_area<R>(
        &mut self,
        id: WidgetId,
        rect: Rect,
        state: &mut ScrollState,
        content: Vec2,
        axes: ScrollAxes,
        add: impl FnOnce(&mut Self, ScrollContent) -> R,
    ) -> R {
        let style = self.styles().scroll;
        let viewport = viewport(rect, axes, &style);
        let visible = viewport.size().max(Vec2::ZERO);
        let overflow = (content - visible).max(Vec2::ZERO);
        let clamp = |offset: Vec2| {
            Vec2::new(
                if axes.has(0) {
                    offset.x.clamp(0.0, overflow.x)
                } else {
                    0.0
                },
                if axes.has(1) {
                    offset.y.clamp(0.0, overflow.y)
                } else {
                    0.0
                },
            )
        };
        if self.pointer_within(id) {
            let wheel = self.input().wheel();
            // Shift turns the wheel sideways, as everywhere.
            let wheel = if self.input().modifiers().shift() && wheel.x == 0.0 {
                Vec2::new(wheel.y, 0.0)
            } else {
                wheel
            };
            let step = style.wheel_step;
            state.offset -= Vec2::new(-wheel.x, wheel.y) * step;
        }
        state.offset = clamp(state.offset);

        let result = self.identified_zone(rect, SCROLL, id, |ui| {
            let origin = viewport.min - state.offset;
            let result = ui.clipped(viewport, |ui| add(ui, ScrollContent { origin, viewport }));
            ui.scrollbars(id, rect, viewport, state, content, axes);
            result
        });
        state.offset = clamp(state.offset);
        result
    }

    /// The overlay scrollbars of a scroll area, in its margins: vertical, then horizontal.
    fn scrollbars(
        &mut self,
        id: WidgetId,
        rect: Rect,
        viewport: Rect,
        state: &mut ScrollState,
        content: Vec2,
        axes: ScrollAxes,
    ) {
        let near = self.pointer_within(id);
        if axes.has(1) {
            Scrollbar::new(
                id.with(2),
                &mut state.bars[1],
                ScrollAxis::Vertical,
                &mut state.offset.y,
                viewport.height(),
                content.y,
            )
            .near(near)
            .show(self, rect);
        }
        if axes.has(0) {
            Scrollbar::new(
                id.with(1),
                &mut state.bars[0],
                ScrollAxis::Horizontal,
                &mut state.offset.x,
                viewport.width(),
                content.x,
            )
            .near(near)
            .show(self, rect);
        }
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::Color;
    use koebi_render::{RectPrimitive, Scene};

    use super::*;
    use crate::{Input, Styles, UiState};

    const AREA: WidgetId = WidgetId::new("list");

    fn rect() -> Rect {
        Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::new(200.0, 100.0))
    }

    struct Harness {
        state: UiState,
        scroll: ScrollState,
        input: Input,
        styles: Styles,
        content: Vec2,
    }

    impl Harness {
        fn new(content: Vec2) -> Self {
            let mut harness = Self {
                state: UiState::new(),
                scroll: ScrollState::new(),
                input: Input::new(),
                styles: crate::test_styles(),
                content,
            };
            harness.frame();
            harness
        }

        /// One frame; returns the scene and the content origin.
        fn frame(&mut self) -> (Scene, Vec2) {
            let mut scene = Scene::new();
            let origin = {
                let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
                let content = self.content;
                ui.scroll_area(
                    AREA,
                    rect(),
                    &mut self.scroll,
                    content,
                    ScrollAxes::Vertical,
                    |ui, view| {
                        // One rectangle standing for the content.
                        ui.scene().push_rect(&RectPrimitive::new(
                            Rect::from_origin_size(view.origin, content),
                            Color::WHITE,
                        ));
                        view.origin
                    },
                )
            };
            self.input.end_frame();
            (scene, origin)
        }
    }

    #[test]
    fn content_that_fits_shows_no_scrollbar_and_does_not_scroll() {
        let mut harness = Harness::new(Vec2::new(150.0, 80.0));
        harness.input.pointer_moved(Vec2::new(50.0, 50.0));
        harness.input.scrolled(Vec2::new(0.0, -3.0));
        let (scene, origin) = harness.frame();
        assert_eq!(origin, rect().min);
        // Only the content: no grab.
        assert_eq!(scene.rect_count(), 1);
    }

    #[test]
    fn the_wheel_scrolls_within_the_content() {
        let mut harness = Harness::new(Vec2::new(150.0, 500.0));
        harness.input.pointer_moved(Vec2::new(50.0, 50.0));
        harness.frame();
        // Turning towards the user scrolls down: two notches.
        harness.input.scrolled(Vec2::new(0.0, -2.0));
        let (scene, _) = harness.frame();
        let step = harness.styles.scroll.wheel_step;
        assert_eq!(harness.scroll.offset().y, 2.0 * step);
        // The content and the grab.
        assert_eq!(scene.rect_count(), 2);
        // Never past the end: 500 - 100 visible.
        harness.input.scrolled(Vec2::new(0.0, -100.0));
        harness.frame();
        assert_eq!(harness.scroll.offset().y, 400.0);
        harness.input.scrolled(Vec2::new(0.0, 100.0));
        harness.frame();
        assert_eq!(harness.scroll.offset().y, 0.0);
        // Outside the area, the wheel does nothing.
        harness.input.pointer_moved(Vec2::new(300.0, 50.0));
        harness.frame();
        harness.input.scrolled(Vec2::new(0.0, -2.0));
        harness.frame();
        assert_eq!(harness.scroll.offset().y, 0.0);
    }

    #[test]
    fn dragging_the_grab_scrolls_and_the_track_brings_it_under_the_pointer() {
        let mut harness = Harness::new(Vec2::new(150.0, 400.0));
        let style = harness.styles.scroll;
        // The track: 100 high minus the padding at both ends; the grab a quarter of it.
        let x = rect().max.x - style.margin * 0.5;
        let length = 100.0 - 2.0 * style.padding;
        let grab = length / 4.0;
        let top = rect().min.y + style.padding;
        harness.input.pointer_moved(Vec2::new(x, top + grab * 0.5));
        harness.frame();
        harness
            .input
            .primary_button(true, Vec2::new(x, top + grab * 0.5));
        harness.frame();
        // Moving the grab by half the free room scrolls half the overflow.
        harness
            .input
            .pointer_moved(Vec2::new(x, top + grab * 0.5 + (length - grab) * 0.5));
        harness.frame();
        assert!(
            (harness.scroll.offset().y - 150.0).abs() < 0.5,
            "{:?}",
            harness.scroll.offset()
        );
        harness.input.primary_button(
            false,
            Vec2::new(x, top + grab * 0.5 + (length - grab) * 0.5),
        );
        harness.frame();
        // A press at the bottom of the track brings the grab there: the end of the content.
        let bottom = Vec2::new(x, top + length - 1.0);
        harness.input.pointer_moved(bottom);
        harness.frame();
        harness.input.primary_button(true, bottom);
        harness.frame();
        assert!((harness.scroll.offset().y - 300.0).abs() < 0.5);
    }

    #[test]
    fn the_grab_grows_as_the_pointer_comes_near() {
        let mut harness = Harness::new(Vec2::new(150.0, 400.0));
        let style = harness.styles.scroll;
        let x = rect().max.x - style.margin * 0.5;
        harness.input.pointer_moved(Vec2::new(x, 50.0));
        harness.input.set_time(Duration::from_millis(16));
        harness.frame();
        // Easing: still growing, and asking for the next frame.
        harness.input.set_time(Duration::from_millis(32));
        harness.frame();
        let grown = harness.scroll.bars[1].growth();
        assert!(grown > 0.0 && grown < 1.0, "{grown}");
        assert!(harness.state.redraw_at().is_some());
        harness.input.set_time(Duration::from_secs(2));
        harness.frame();
        assert_eq!(harness.scroll.bars[1].growth(), 1.0);
        assert_eq!(harness.state.redraw_at(), None);
    }
}
