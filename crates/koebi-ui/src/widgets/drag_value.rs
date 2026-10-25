//! Drag values: a number field changed by dragging, with step buttons at both ends.

use core::ops::RangeInclusive;

use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_render::RectPrimitive;
use koebi_tokens::component;

use crate::icon::Icon;
use crate::id::WidgetId;
use crate::style::StateColors;
use crate::ui::{Response, Ui, WidgetState};
use crate::widgets::slider::fraction;

/// Colors and metrics of [`Ui::drag_value`], from the `component.drag-value.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragValueStyle {
    /// Background per state; pressed is a press that has not moved yet.
    pub background: StateColors,
    /// Background while dragging.
    pub background_dragging: Color,
    /// Fill showing where the value lies in its range.
    pub fill: Color,
    /// Border color.
    pub border_color: Color,
    /// Border width.
    pub border_width: f32,
    /// Corner radius.
    pub corner_radius: f32,
    /// Step button icons.
    pub step_icon: Color,
    /// Step button background, hovered.
    pub step_background_hovered: Color,
}

impl DragValueStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            background: StateColors {
                normal: tokens.get(component::DRAG_VALUE_BACKGROUND_COLOR_DEFAULT),
                hovered: tokens.get(component::DRAG_VALUE_BACKGROUND_COLOR_HOVER),
                pressed: tokens.get(component::DRAG_VALUE_BACKGROUND_COLOR_PRESSED),
            },
            background_dragging: tokens.get(component::DRAG_VALUE_BACKGROUND_COLOR_DRAG),
            fill: tokens.get(component::DRAG_VALUE_FILL_COLOR_DEFAULT),
            border_color: tokens.get(component::DRAG_VALUE_BORDER_COLOR_DEFAULT),
            border_width: tokens.get(component::DRAG_VALUE_BORDER_WIDTH_DEFAULT),
            corner_radius: tokens.get(component::DRAG_VALUE_CORNER_RADIUS_DEFAULT),
            step_icon: tokens.get(component::DRAG_VALUE_STEP_BUTTON_COLOR_DEFAULT),
            step_background_hovered: tokens
                .get(component::DRAG_VALUE_STEP_BUTTON_BACKGROUND_COLOR_HOVER),
        }
    }
}

/// The areas of a drag value: square step buttons at both ends when the field is at least
/// three times as wide as it is tall, and the draggable body between them.
fn parts(rect: Rect) -> (Rect, Option<[Rect; 2]>) {
    let side = rect.height();
    if rect.width() < 3.0 * side {
        return (rect, None);
    }
    let left = Rect::from_origin_size(rect.min, Vec2::splat(side));
    let right = Rect::from_origin_size(Vec2::new(rect.max.x - side, rect.min.y), Vec2::splat(side));
    let body = Rect::from_min_max(
        Vec2::new(left.max.x, rect.min.y),
        Vec2::new(right.min.x, rect.max.y),
    );
    (body, Some([left, right]))
}

/// `value` clamped to `range`, on the grid of `step` from the range start when `step > 0`.
/// Unlike `f32::clamp`, it never panics: a NaN bound is ignored.
pub(crate) fn quantize(value: f32, range: &RangeInclusive<f32>, step: f32) -> f32 {
    let (min, max) = (*range.start(), *range.end());
    let snapped = if step > 0.0 {
        min + ((value - min) / step).round() * step
    } else {
        value
    };
    snapped.max(min).min(max)
}

impl Ui<'_> {
    /// A number field filling `rect`. Dragging horizontally changes `value` relative to its
    /// value at the press, the body's width spanning the whole `range`; the step buttons at
    /// both ends subtract or add `step` (a hundredth of the range when `step` is `0`).
    /// Values stay within `range`, on multiples of `step` from its start when `step > 0`.
    /// The fill shows where the value lies in the range, until fields can show numbers.
    pub fn drag_value(
        &mut self,
        id: WidgetId,
        rect: Rect,
        value: &mut f32,
        range: RangeInclusive<f32>,
        step: f32,
    ) -> Response {
        let (body, steps) = parts(rect);
        let mut response = self.interact(id, body);
        if response.pressed {
            self.set_press_value(*value);
        }
        let mut new = *value;
        if response.held && body.width() > 0.0 {
            let span = range.end() - range.start();
            new = self.press_value() + response.drag_delta.x * span / body.width();
        }
        let mut step_responses = [Response::default(); 2];
        if let Some(areas) = steps {
            let amount = if step > 0.0 {
                step
            } else {
                (range.end() - range.start()) / 100.0
            };
            for (index, (area, sign)) in areas.into_iter().zip([-1.0, 1.0]).enumerate() {
                let part = self.interact(id.with(index as u64), area);
                if part.clicked {
                    new = *value + sign * amount;
                }
                step_responses[index] = part;
                response = response.merge(part);
            }
        }
        let new = quantize(new, &range, step);
        if new.total_cmp(value).is_ne() {
            *value = new;
            response.changed = true;
        }

        let styles = self.styles();
        let style = &styles.drag_value;
        let background = match response.drag_state() {
            WidgetState::Pressed if response.drag_delta != Vec2::ZERO => style.background_dragging,
            state => style.background.pick(state),
        };
        let radius = style.corner_radius;
        self.scene
            .push_rect(&RectPrimitive::new(rect, background).with_corner_radius(radius));
        let filled = rect.width() * fraction(*value, &range);
        self.scene.push_rect(
            &RectPrimitive::new(
                Rect::from_origin_size(rect.min, Vec2::new(filled, rect.height())),
                style.fill,
            )
            .with_corner_radius(radius),
        );
        self.scene.push_rect(
            &RectPrimitive::new(rect, Color::TRANSPARENT)
                .with_corner_radius(radius)
                .with_border(style.border_width, style.border_color),
        );
        if let Some(areas) = steps {
            for ((area, icon), part) in areas
                .into_iter()
                .zip([Icon::ChevronLeft, Icon::ChevronRight])
                .zip(step_responses)
            {
                if part.hovered {
                    let inner = area.inset(style.border_width);
                    self.scene.push_rect(
                        &RectPrimitive::new(inner, style.step_background_hovered)
                            .with_corner_radius((radius - style.border_width).max(0.0)),
                    );
                }
                let bounds = Icon::centered(area, styles.icon.size);
                icon.draw(
                    self.scene,
                    bounds,
                    style.step_icon,
                    styles.icon.stroke_width,
                );
            }
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;

    use crate::{Input, UiState};

    use super::*;

    const ID: WidgetId = WidgetId::new("size");

    fn field() -> Rect {
        // 24 px tall: 24 px step buttons, a 152 px body from x = 24 to x = 176.
        Rect::from_origin_size(Vec2::ZERO, Vec2::new(200.0, 24.0))
    }

    struct Harness {
        styles: crate::Styles,
        scene: Scene,
        state: UiState,
        input: Input,
    }

    impl Harness {
        fn new() -> Self {
            Self {
                styles: crate::test_styles(),
                scene: Scene::new(),
                state: UiState::new(),
                input: Input::new(),
            }
        }

        fn frame(&mut self, value: &mut f32, range: RangeInclusive<f32>, step: f32) -> Response {
            self.scene.clear();
            let mut ui = Ui::new(&mut self.scene, &self.styles, &self.input, &mut self.state);
            let response = ui.drag_value(ID, field(), value, range, step);
            drop(ui);
            self.input.end_frame();
            response
        }
    }

    #[test]
    fn parts_leave_room_for_square_step_buttons() {
        let (body, steps) = parts(field());
        assert_eq!((body.min.x, body.max.x), (24.0, 176.0));
        let [left, right] = steps.unwrap();
        assert_eq!((left.width(), right.min.x), (24.0, 176.0));
        let narrow = Rect::from_origin_size(Vec2::ZERO, Vec2::new(60.0, 24.0));
        assert_eq!(parts(narrow), (narrow, None));
    }

    #[test]
    fn quantize_snaps_to_the_step_grid_and_clamps() {
        assert_eq!(quantize(3.26, &(0.0..=10.0), 0.5), 3.5);
        assert_eq!(quantize(3.26, &(1.0..=10.0), 2.0), 3.0);
        assert_eq!(quantize(42.0, &(0.0..=10.0), 0.0), 10.0);
        // Broken ranges do not panic.
        assert_eq!(quantize(4.0, &(f32::NAN..=10.0), 0.0), 4.0);
        assert_eq!(quantize(4.0, &(10.0..=0.0), 0.0), 0.0);
    }

    #[test]
    fn dragging_is_relative_to_the_value_at_the_press() {
        let mut harness = Harness::new();
        let mut value = 2.0;
        // Pressing anywhere in the body does not jump.
        harness.input.primary_button(true, Vec2::new(150.0, 12.0));
        assert!(!harness.frame(&mut value, 0.0..=8.0, 0.0).changed);
        assert_eq!(value, 2.0);
        // 38 px of a 152 px body is a quarter of the range: +2.
        harness.input.pointer_moved(Vec2::new(188.0, 12.0));
        assert!(harness.frame(&mut value, 0.0..=8.0, 0.0).changed);
        assert!((value - 4.0).abs() < 1e-5, "{value}");
        // Coming back restores the value: no drift.
        harness.input.pointer_moved(Vec2::new(150.0, 12.0));
        harness.frame(&mut value, 0.0..=8.0, 0.0);
        assert!((value - 2.0).abs() < 1e-5, "{value}");
    }

    #[test]
    fn step_buttons_add_and_subtract_the_step() {
        let mut harness = Harness::new();
        let mut value = 2.0;
        for (x, expected) in [(190.0, 2.5), (190.0, 3.0), (10.0, 2.5)] {
            harness.input.primary_button(true, Vec2::new(x, 12.0));
            harness.input.primary_button(false, Vec2::new(x, 12.0));
            assert!(harness.frame(&mut value, 0.0..=3.0, 0.5).changed);
            assert_eq!(value, expected);
        }
        // At the maximum, the plus button changes nothing.
        harness.input.primary_button(true, Vec2::new(190.0, 12.0));
        harness.input.primary_button(false, Vec2::new(190.0, 12.0));
        value = 3.0;
        assert!(!harness.frame(&mut value, 0.0..=3.0, 0.5).changed);
    }

    #[test]
    fn the_fill_follows_the_value() {
        let mut harness = Harness::new();
        let mut value = 0.0;
        harness.frame(&mut value, 0.0..=1.0, 0.0);
        // Background and border; the empty fill is culled; two chevrons.
        assert_eq!(
            (harness.scene.rect_count(), harness.scene.line_count()),
            (2, 4)
        );
        value = 0.5;
        harness.frame(&mut value, 0.0..=1.0, 0.0);
        assert_eq!(harness.scene.rect_count(), 3);
    }
}
