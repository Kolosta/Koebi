//! Scrollbars: the overlay scrollbar of scroll areas as a component of its own, usable without
//! a scroll area (a canvas, a timeline that scroll their own way).
//!
//! A [`Scrollbar`] lives in the margin at the end of the area it scrolls (`scrollbar.overlay.*`):
//! at the right for a vertical one, at the bottom for a horizontal one. Its track runs along
//! the area inside `scrollbar.overlay.padding`; its grab is as long as the visible share of the
//! content (never shorter than twice its full width) and sits where the offset is. As the
//! pointer comes near (`scrollbar.overlay.proximity`), the grab grows from its rest width to
//! its full width and brightens, along `scrollbar.overlay.grow.*`. Dragging the grab scrolls;
//! a press on the track brings the grab's middle under the pointer, then drags. The whole
//! margin along the track takes the pointer.

use core::time::Duration;

use koebi_core::{Color, Rect, Vec2};
use koebi_render::RectPrimitive;

use crate::id::WidgetId;
use crate::ui::{Response, Ui};
use crate::widgets::parts::Tween;
use crate::widgets::scroll::ScrollStyle;

/// Which way a scrollbar runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScrollAxis {
    /// Along the bottom of the area: scrolls sideways.
    Horizontal,
    /// Along the right of the area: scrolls up and down.
    Vertical,
}

/// A scrollbar's growth towards the pointer and its drag. Keep one per scrollbar.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollbarState {
    /// Growth towards the pointer, from 0 (rest) to 1 (full width).
    grow: Tween,
    /// While the grab is dragged: where the pointer holds it, from the grab's start.
    hold: Option<f32>,
}

impl ScrollbarState {
    /// At rest.
    pub const fn new() -> Self {
        Self {
            grow: Tween::new(0.0),
            hold: None,
        }
    }

    /// How far the grab has grown towards the pointer, from 0 (rest) to 1 (full width).
    pub fn growth(&self) -> f32 {
        self.grow.value()
    }

    /// `true` while the grab is dragged.
    pub fn is_dragging(&self) -> bool {
        self.hold.is_some()
    }
}

/// What happened to a scrollbar this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollbarResponse {
    /// The pointer's interaction with the margin the scrollbar lives in.
    pub response: Response,
    /// The scrollbar changed the offset this frame.
    pub changed: bool,
    /// The content overflows: the scrollbar shows.
    pub shown: bool,
}

/// A scrollbar's track along its axis, and its grab.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Track {
    /// The track's start and length along the axis.
    pub(crate) start: f32,
    pub(crate) length: f32,
    /// The grab's start and length along the axis.
    pub(crate) grab_start: f32,
    pub(crate) grab_length: f32,
}

/// The track of a viewport `visible` long showing content `content` long scrolled by
/// `offset`, in a span from `start` to `start + span`; `None` when nothing overflows.
pub(crate) fn track(
    start: f32,
    span: f32,
    visible: f32,
    content: f32,
    offset: f32,
    style: &ScrollStyle,
) -> Option<Track> {
    let overflow = content - visible;
    if overflow.is_nan() || overflow <= 0.5 || visible.is_nan() || visible <= 1.0 {
        return None;
    }
    let length = (span - 2.0 * style.padding).max(1.0);
    let grab_length = (length * visible / content)
        .max(2.0 * style.width_hover)
        .min(length);
    let fraction = unit(offset / overflow);
    Some(Track {
        start: start + style.padding,
        length,
        grab_start: start + style.padding + (length - grab_length) * fraction,
        grab_length,
    })
}

/// `value` within 0 and 1; 0 for NaN.
fn unit(value: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

/// `a` moved towards `b` by `t`.
fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// A scrollbar, configured then shown with [`Scrollbar::show`].
#[derive(Debug)]
pub struct Scrollbar<'a> {
    id: WidgetId,
    state: &'a mut ScrollbarState,
    axis: ScrollAxis,
    offset: &'a mut f32,
    visible: f32,
    content: f32,
    near: bool,
}

impl<'a> Scrollbar<'a> {
    /// A scrollbar identified by `id`, running along `axis`, changing `offset` (how far the
    /// content is scrolled, from 0 to `content - visible`) over a viewport `visible` long
    /// showing content `content` long.
    pub fn new(
        id: WidgetId,
        state: &'a mut ScrollbarState,
        axis: ScrollAxis,
        offset: &'a mut f32,
        visible: f32,
        content: f32,
    ) -> Self {
        Self {
            id,
            state,
            axis,
            offset,
            visible,
            content,
            near: true,
        }
    }

    /// Whether the pointer is over what the scrollbar scrolls (true by default): the grab
    /// grows towards the pointer only then.
    #[must_use]
    pub fn near(mut self, near: bool) -> Self {
        self.near = near;
        self
    }

    /// Shows the scrollbar in the margin at the end of `area`, the area it scrolls.
    pub fn show(self, ui: &mut Ui<'_>, area: Rect) -> ScrollbarResponse {
        let style = ui.styles().scroll;
        let vertical = self.axis == ScrollAxis::Vertical;
        let (span_start, span) = if vertical {
            (area.min.y, area.height())
        } else {
            (area.min.x, area.width())
        };
        let Some(track) = track(
            span_start,
            span,
            self.visible,
            self.content,
            *self.offset,
            &style,
        ) else {
            self.state.grow.set(0.0);
            self.state.hold = None;
            return ScrollbarResponse::default();
        };
        // Across the axis, the grab sits in the middle of the margin.
        let middle = if vertical {
            area.max.x - style.margin * 0.5
        } else {
            area.max.y - style.margin * 0.5
        };
        let along = |point: Vec2| if vertical { point.y } else { point.x };
        let across = |point: Vec2| if vertical { point.x } else { point.y };
        let pointer = ui.input().pointer();
        let target = if self.state.hold.is_some() {
            1.0
        } else {
            pointer.filter(|_| self.near).map_or(0.0, |point| {
                let at = along(point);
                if at >= track.start && at <= track.start + track.length {
                    unit(1.0 - (across(point) - middle).abs() / style.proximity.max(1.0))
                } else {
                    0.0
                }
            })
        };
        let now = ui.input().time();
        if self
            .state
            .grow
            .advance(target, now, style.grow_duration, &style.grow_curve)
        {
            // Still growing or shrinking: the next frame continues the animation.
            ui.request_redraw_after(Duration::from_millis(16));
        }
        let growth = self.state.grow.value();
        let width = style.width_rest + (style.width_hover - style.width_rest) * growth;

        // The whole margin along the track takes the pointer.
        let half = style.margin.max(width) * 0.5;
        let lay = |start: f32, length: f32, half: f32| {
            if vertical {
                Rect::from_min_max(
                    Vec2::new(middle - half, start),
                    Vec2::new(middle + half, start + length),
                )
            } else {
                Rect::from_min_max(
                    Vec2::new(start, middle - half),
                    Vec2::new(start + length, middle + half),
                )
            }
        };
        let response = ui.interact(self.id, lay(track.start, track.length, half));
        if response.pressed
            && let Some(press) = ui.input().pressed_at()
        {
            let at = along(press);
            let on_grab = at >= track.grab_start && at <= track.grab_start + track.grab_length;
            // On the track, the grab's middle comes under the pointer.
            self.state.hold = Some(if on_grab {
                at - track.grab_start
            } else {
                track.grab_length * 0.5
            });
        }
        let mut grab_start = track.grab_start;
        let mut changed = false;
        if let Some(hold) = self.state.hold {
            if response.held || response.pressed {
                if let Some(point) = pointer {
                    let room = track.length - track.grab_length;
                    let fraction = if room > 0.5 {
                        unit((along(point) - hold - track.start) / room)
                    } else {
                        0.0
                    };
                    let offset = fraction * (self.content - self.visible);
                    changed = (offset - *self.offset).abs() > 1e-3;
                    *self.offset = offset;
                    grab_start = track.start + room * fraction;
                }
            } else {
                self.state.hold = None;
            }
        }
        let color = if self.state.hold.is_some() {
            mix(style.grab.normal, style.grab.pressed, growth)
        } else {
            mix(style.grab.normal, style.grab.hovered, growth)
        };
        ui.scene().push_rect(
            &RectPrimitive::new(lay(grab_start, track.grab_length, width * 0.5), color)
                .with_corner_radius(style.corner_radius.min(width * 0.5)),
        );
        ScrollbarResponse {
            response,
            changed,
            shown: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;

    use super::*;
    use crate::{Input, Styles, UiState};

    const BAR: WidgetId = WidgetId::new("bar");

    fn area() -> Rect {
        Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::new(300.0, 100.0))
    }

    struct Harness {
        state: UiState,
        bar: ScrollbarState,
        offset: f32,
        input: Input,
        styles: Styles,
    }

    impl Harness {
        fn new() -> Self {
            Self {
                state: UiState::new(),
                bar: ScrollbarState::new(),
                offset: 0.0,
                input: Input::new(),
                styles: crate::test_styles(),
            }
        }

        /// One frame of a horizontal scrollbar over a timeline 1200 wide shown 300 wide.
        fn frame(&mut self) -> (ScrollbarResponse, Scene) {
            let mut scene = Scene::new();
            let response = {
                let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
                Scrollbar::new(
                    BAR,
                    &mut self.bar,
                    ScrollAxis::Horizontal,
                    &mut self.offset,
                    300.0,
                    1200.0,
                )
                .show(&mut ui, area())
            };
            self.input.end_frame();
            (response, scene)
        }
    }

    #[test]
    fn tracks_follow_the_visible_share_of_the_content() {
        let style = crate::test_styles().scroll;
        assert_eq!(track(0.0, 100.0, 100.0, 100.0, 0.0, &style), None);
        assert_eq!(track(0.0, 100.0, f32::NAN, 1000.0, 0.0, &style), None);
        let track = track(0.0, 100.0, 100.0, 1000.0, 450.0, &style).unwrap();
        let length = 100.0 - 2.0 * style.padding;
        // A tenth of the content shows, but the grab keeps a minimum length.
        assert_eq!(
            track.grab_length,
            (length / 10.0).max(2.0 * style.width_hover)
        );
        assert!(
            (track.grab_start - (style.padding + (length - track.grab_length) * 0.5)).abs() < 1e-3
        );
    }

    #[test]
    fn scrollbars_work_without_a_scroll_area() {
        let mut harness = Harness::new();
        let style = harness.styles.scroll;
        let (shown, scene) = harness.frame();
        assert!(shown.shown);
        assert_eq!(scene.rect_count(), 1);
        // A press at the end of the track brings the grab there: the end of the content.
        let y = area().max.y - style.margin * 0.5;
        let end = Vec2::new(area().max.x - style.padding - 1.0, y);
        harness.input.pointer_moved(end);
        harness.frame();
        harness.input.primary_button(true, end);
        let (pressed, _) = harness.frame();
        assert!(pressed.changed && harness.bar.is_dragging());
        assert!((harness.offset - 900.0).abs() < 0.5, "{}", harness.offset);
        // Dragging back to the start scrolls back.
        harness
            .input
            .pointer_moved(Vec2::new(area().min.x + style.padding, y));
        harness.frame();
        assert!(harness.offset.abs() < 0.5, "{}", harness.offset);
        harness
            .input
            .primary_button(false, Vec2::new(area().min.x, y));
        harness.frame();
        assert!(!harness.bar.is_dragging());
    }

    #[test]
    fn grabs_grow_only_near_the_pointer_over_what_they_scroll() {
        let mut harness = Harness::new();
        let style = harness.styles.scroll;
        let y = area().max.y - style.margin * 0.5;
        harness.input.pointer_moved(Vec2::new(150.0, y));
        harness.frame();
        harness.input.set_time(Duration::from_secs(2));
        harness.frame();
        assert_eq!(harness.bar.growth(), 1.0);
        // Away from the bar, it shrinks back.
        harness.input.pointer_moved(Vec2::new(150.0, area().min.y));
        harness.frame();
        harness.input.set_time(Duration::from_secs(4));
        harness.frame();
        assert_eq!(harness.bar.growth(), 0.0);
        // Not near what it scrolls: no growth even over the bar.
        let mut scene = Scene::new();
        harness.input.pointer_moved(Vec2::new(150.0, y));
        harness.input.set_time(Duration::from_secs(6));
        {
            let mut ui = Ui::new(
                &mut scene,
                &harness.styles,
                &harness.input,
                &mut harness.state,
            );
            Scrollbar::new(
                BAR,
                &mut harness.bar,
                ScrollAxis::Horizontal,
                &mut harness.offset,
                300.0,
                1200.0,
            )
            .near(false)
            .show(&mut ui, area());
        }
        assert_eq!(harness.bar.growth(), 0.0);
    }
}
