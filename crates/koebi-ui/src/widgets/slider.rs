//! Sliders: a grab that travels along a track.

use core::ops::RangeInclusive;

use koebi_core::{Rect, Vec2};
use koebi_design::Resolve;
use koebi_render::RectPrimitive;
use koebi_tokens::component;

use crate::id::WidgetId;
use crate::style::StateColors;
use crate::ui::{Response, Ui};

/// Colors and metrics of [`Ui::slider`], from the `component.slider.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderStyle {
    /// Track color per state (pressed while dragging).
    pub track: StateColors,
    /// Grab color per state.
    pub grab: StateColors,
    /// Grab width.
    pub grab_width: f32,
    /// Space between the grab and the track edges.
    pub grab_padding: f32,
    /// Corner radius of the track; the grab's follows, inset by the padding.
    pub corner_radius: f32,
}

impl SliderStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        let track_hovered = tokens.get(component::SLIDER_TRACK_COLOR_HOVER);
        Self {
            track: StateColors {
                normal: tokens.get(component::SLIDER_TRACK_COLOR_DEFAULT),
                hovered: track_hovered,
                pressed: track_hovered,
            },
            grab: StateColors {
                normal: tokens.get(component::SLIDER_GRAB_COLOR_DEFAULT),
                hovered: tokens.get(component::SLIDER_GRAB_COLOR_HOVER),
                pressed: tokens.get(component::SLIDER_GRAB_COLOR_PRESSED),
            },
            grab_width: tokens.get(component::SLIDER_GRAB_WIDTH_DEFAULT),
            grab_padding: tokens.get(component::SLIDER_GRAB_PADDING_DEFAULT),
            corner_radius: tokens.get(component::SLIDER_CORNER_RADIUS_DEFAULT),
        }
    }

    /// Where the grab's center can travel in a track `rect`: start x and length.
    fn travel(&self, rect: Rect) -> (f32, f32) {
        let grab = self.grab_width.min(rect.width());
        let start = rect.min.x + self.grab_padding + grab * 0.5;
        let length = (rect.width() - 2.0 * self.grab_padding - grab).max(0.0);
        (start, length)
    }

    /// The grab of a track `rect` for a value at `fraction` (0 to 1) of its range.
    pub fn grab_rect(&self, rect: Rect, fraction: f32) -> Rect {
        let (start, length) = self.travel(rect);
        let width = self.grab_width.min(rect.width());
        let height = (rect.height() - 2.0 * self.grab_padding).max(0.0);
        let center = Vec2::new(start + fraction.clamp(0.0, 1.0) * length, rect.center().y);
        Rect::from_center_size(center, Vec2::new(width, height))
    }

    /// The fraction of the range under pointer position `x`, for a track `rect`.
    fn fraction_at(&self, rect: Rect, x: f32) -> f32 {
        let (start, length) = self.travel(rect);
        if length > 0.0 {
            ((x - start) / length).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// Where `value` lies in `range`, from 0 to 1 (0 for an empty range or a NaN).
pub(crate) fn fraction(value: f32, range: &RangeInclusive<f32>) -> f32 {
    let (min, max) = (*range.start(), *range.end());
    let fraction = (value - min) / (max - min);
    if fraction.is_finite() {
        fraction.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// The value at `fraction` of `range`.
pub(crate) fn lerp(range: &RangeInclusive<f32>, fraction: f32) -> f32 {
    let (min, max) = (*range.start(), *range.end());
    min + (max - min) * fraction
}

impl Ui<'_> {
    /// A horizontal slider filling `rect`. Pressing the track moves the grab under the
    /// pointer, and dragging moves it along; `value` stays within `range` and the response
    /// reports `changed` when it moves.
    pub fn slider(
        &mut self,
        id: WidgetId,
        rect: Rect,
        value: &mut f32,
        range: RangeInclusive<f32>,
    ) -> Response {
        let mut response = self.interact(id, rect);
        let style = &self.styles().slider;
        if response.held
            && let Some(pointer) = self.input().pointer()
        {
            let new = lerp(&range, style.fraction_at(rect, pointer.x));
            if new.total_cmp(value).is_ne() {
                *value = new;
                response.changed = true;
            }
        }
        let state = response.drag_state();
        self.scene.push_rect(
            &RectPrimitive::new(rect, style.track.pick(state))
                .with_corner_radius(style.corner_radius),
        );
        let grab_radius = (style.corner_radius - style.grab_padding).max(0.0);
        let grab = style.grab_rect(rect, fraction(*value, &range));
        self.scene.push_rect(
            &RectPrimitive::new(grab, style.grab.pick(state)).with_corner_radius(grab_radius),
        );
        response
    }
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;

    use crate::{Input, UiState};

    use super::*;

    fn track() -> Rect {
        Rect::from_origin_size(Vec2::new(100.0, 0.0), Vec2::new(200.0, 20.0))
    }

    #[test]
    fn fractions_handle_empty_ranges() {
        assert_eq!(fraction(5.0, &(0.0..=10.0)), 0.5);
        assert_eq!(fraction(-5.0, &(0.0..=10.0)), 0.0);
        assert_eq!(fraction(5.0, &(3.0..=3.0)), 0.0);
        assert_eq!(fraction(f32::NAN, &(0.0..=1.0)), 0.0);
        assert_eq!(lerp(&(10.0..=20.0), 0.25), 12.5);
    }

    #[test]
    fn the_grab_travels_inside_the_padding() {
        let style = crate::test_styles().slider;
        let (padding, width) = (style.grab_padding, style.grab_width);
        let start = style.grab_rect(track(), 0.0);
        let end = style.grab_rect(track(), 1.0);
        assert_eq!(start.min.x, 100.0 + padding);
        assert_eq!(end.max.x, 300.0 - padding);
        assert_eq!(start.width(), width);
        assert_eq!(start.height(), 20.0 - 2.0 * padding);
    }

    #[test]
    fn pressing_and_dragging_set_the_value() {
        let styles = crate::test_styles();
        let (mut scene, mut state, mut input) = (Scene::new(), UiState::new(), Input::new());
        let mut value = 0.0;
        let mut frame = |input: &mut Input, value: &mut f32| {
            let mut ui = Ui::new(&mut scene, &styles, input, &mut state);
            let response = ui.slider(WidgetId::new("radius"), track(), value, 0.0..=10.0);
            drop(ui);
            input.end_frame();
            response
        };
        // Pressing the middle of the track jumps there.
        input.primary_button(true, Vec2::new(200.0, 10.0));
        assert!(frame(&mut input, &mut value).changed);
        assert!((value - 5.0).abs() < 1e-5, "{value}");
        // Dragging far right clamps to the maximum.
        input.pointer_moved(Vec2::new(900.0, 10.0));
        frame(&mut input, &mut value);
        assert_eq!(value, 10.0);
        // Without a press, moving does nothing.
        input.primary_button(false, Vec2::new(900.0, 10.0));
        frame(&mut input, &mut value);
        input.pointer_moved(Vec2::new(150.0, 10.0));
        assert!(!frame(&mut input, &mut value).changed);
        assert_eq!(value, 10.0);
    }
}
