//! One test per bug found in review, each failing before its fix.

use koebi_core::input::Modifiers;
use koebi_core::{Rect, Vec2};

use super::*;
use crate::PointerButton;
use crate::widgets::text_field::TEXT_CONFIRM;

/// One frame of a 0..10 field with a bar, drawn twice like the application's frames after a
/// change of the design system.
fn bar_twice(harness: &mut Harness, value: &mut f64) -> NumberFieldResponse {
    harness.frame_twice(|ui| {
        NumberField::new(ID, value)
            .range(0.0, 10.0)
            .bar(BarMapping::Linear)
            .show(ui, field())
    })
}

#[test]
fn the_pointer_comes_back_right_when_a_frame_is_drawn_twice() {
    // Regression test: the second pass asked for no lock, and the pointer came back where the
    // previous frame wanted it, not where the press was.
    for cancel in [true, false] {
        let mut harness = Harness::new();
        let mut value = 2.0;
        bar_twice(&mut harness, &mut value);
        let press = Vec2::new(60.0, field().center().y);
        harness.input.primary_button(true, press);
        bar_twice(&mut harness, &mut value);
        harness.input.pointer_moved_by(Vec2::new(60.0, 0.0));
        bar_twice(&mut harness, &mut value);
        let dragged = harness.state.pointer_lock().expect("locked");
        if cancel {
            harness.input.trigger(&NUMBER_CANCEL);
            bar_twice(&mut harness, &mut value);
            assert_eq!(harness.state.pointer_lock(), Some(press));
        } else {
            harness.input.pointer_moved_by(Vec2::new(20.0, 0.0));
            harness.input.primary_button(false, press);
            bar_twice(&mut harness, &mut value);
            let back = harness
                .state
                .pointer_lock()
                .expect("locked for the release");
            assert!(back.x > dragged.x, "at the new end of the bar: {back:?}");
        }
        bar_twice(&mut harness, &mut value);
        assert_eq!(harness.state.pointer_lock(), None);
    }
}

#[test]
fn integer_steps_are_whole_and_shift_still_steps() {
    // Regression test: a finer step rounded back to the same integer, and a step of 0.5 from
    // 8 rounded back to 8.
    let mut harness = Harness::new();
    let mut value = 5;
    let right = Vec2::new(field().max.x - 4.0, field().center().y);
    let left = Vec2::new(field().min.x + 4.0, field().center().y);
    let click = |harness: &mut Harness, value: &mut i32, at: Vec2, step: f64, modifiers| {
        harness.input.set_modifiers(modifiers);
        harness.input.primary_button(true, at);
        harness.input.primary_button(false, at);
        harness
            .frame(|ui| NumberField::new(ID, value).step(step).show(ui, field()))
            .0
    };
    harness.frame(|ui| NumberField::new(ID, &mut value).show(ui, field()));
    assert!(click(&mut harness, &mut value, right, 1.0, Modifiers::LEFT_SHIFT).changed);
    assert_eq!(value, 6);
    value = 8;
    for _ in 0..3 {
        click(&mut harness, &mut value, left, 0.5, Modifiers::NONE);
    }
    assert_eq!(value, 5);
}

#[test]
fn travel_back_from_past_a_bound_moves_the_value_at_once() {
    // Regression test: past the end, the value stayed at the bound until the travel beyond
    // it had been undone.
    let mut harness = Harness::new();
    let mut value = 5.0;
    let show = |harness: &mut Harness, value: &mut f64| {
        harness
            .frame(|ui| {
                NumberField::new(ID, value)
                    .range(0.0, 10.0)
                    .bar(BarMapping::Linear)
                    .show(ui, field())
            })
            .0
    };
    show(&mut harness, &mut value);
    harness.input.primary_button(true, field().center());
    show(&mut harness, &mut value);
    harness.input.pointer_moved_by(Vec2::new(400.0, 0.0));
    show(&mut harness, &mut value);
    assert!(close(value, 10.0));
    harness.input.pointer_moved_by(Vec2::new(-30.0, 0.0));
    show(&mut harness, &mut value);
    assert!(value < 10.0, "{value}");
}

#[test]
fn a_field_disabled_while_typed_drops_what_was_typed() {
    // Regression test: the edit went on, and Enter set the typed value.
    let mut harness = Harness::new();
    let mut value = 3.0;
    let show = |harness: &mut Harness, value: &mut f64, enabled: bool| {
        harness
            .frame(|ui| {
                NumberField::new(ID, value)
                    .enabled(enabled)
                    .show(ui, field())
            })
            .0
    };
    show(&mut harness, &mut value, true);
    harness.input.primary_button(true, field().center());
    show(&mut harness, &mut value, true);
    harness.input.primary_button(false, field().center());
    show(&mut harness, &mut value, true);
    show(&mut harness, &mut value, true);
    harness.input.typed("7");
    show(&mut harness, &mut value, true);
    let disabled = show(&mut harness, &mut value, false);
    assert_eq!(disabled.ended, Some(EditEnd::Cancelled));
    harness.input.trigger(&TEXT_CONFIRM);
    assert!(!show(&mut harness, &mut value, false).editing);
    assert!(close(value, 3.0));
}

#[test]
fn a_quick_click_on_another_field_keeps_what_was_typed() {
    // Regression test: the other field, drawn first, replaced the edit, and the typed text
    // was lost.
    const FIRST: WidgetId = WidgetId::new("first");
    let other = Rect::from_origin_size(Vec2::new(10.0, 40.0), Vec2::new(160.0, 22.0));
    let mut harness = Harness::new();
    let (mut a, mut b) = (3.0, 4.0);
    let mut both = |harness: &mut Harness| {
        harness
            .frame(|ui| {
                let b = NumberField::new(ID, &mut b).show(ui, other);
                (NumberField::new(FIRST, &mut a).show(ui, field()), b)
            })
            .0
    };
    both(&mut harness);
    harness.input.primary_button(true, field().center());
    both(&mut harness);
    harness.input.primary_button(false, field().center());
    both(&mut harness);
    both(&mut harness);
    harness.input.typed("9");
    both(&mut harness);
    // Press and release on the other field before the next frame.
    harness.input.primary_button(true, other.center());
    harness.input.primary_button(false, other.center());
    let (first, _) = both(&mut harness);
    assert_eq!(first.ended, Some(EditEnd::Confirmed));
    let (_, second) = both(&mut harness);
    assert!(second.editing);
    assert!(close(a, 9.0) && close(b, 4.0), "{a} {b}");
}

#[test]
fn enter_on_an_invalid_expression_keeps_the_text_and_its_caret() {
    // Regression test: the edit restarted with everything selected, so the next key replaced
    // the expression instead of fixing it.
    let mut harness = Harness::new();
    let mut value = 3.0;
    harness.show(&mut value);
    harness.input.primary_button(true, field().center());
    harness.show(&mut value);
    harness.input.primary_button(false, field().center());
    harness.show(&mut value);
    harness.show(&mut value);
    harness.input.typed("2 +");
    harness.show(&mut value);
    harness.input.trigger(&TEXT_CONFIRM);
    assert!(harness.show(&mut value).editing);
    harness.input.typed(" 5");
    harness.show(&mut value);
    harness.input.trigger(&TEXT_CONFIRM);
    let fixed = harness.show(&mut value);
    assert_eq!(fixed.ended, Some(EditEnd::Confirmed));
    assert!(close(value, 7.0), "{value}");
}

#[test]
fn a_press_outside_on_an_invalid_expression_leaves_the_value() {
    let mut harness = Harness::new();
    let mut value = 3.0;
    harness.show(&mut value);
    harness.input.primary_button(true, field().center());
    harness.show(&mut value);
    harness.input.primary_button(false, field().center());
    harness.show(&mut value);
    harness.show(&mut value);
    harness.input.typed("2 +");
    harness.show(&mut value);
    harness.input.primary_button(true, Vec2::new(300.0, 200.0));
    let away = harness.show(&mut value);
    assert_eq!(away.ended, Some(EditEnd::Cancelled));
    assert!(close(value, 3.0));
}

#[test]
fn escape_before_the_threshold_types_nothing_and_right_clicks_open_no_menu() {
    let mut harness = Harness::new();
    let mut value = 3.0;
    harness.show(&mut value);
    harness.input.primary_button(true, field().center());
    harness.show(&mut value);
    harness.input.trigger(&NUMBER_CANCEL);
    harness.show(&mut value);
    harness.input.primary_button(false, field().center());
    assert!(!harness.show(&mut value).editing);
    assert!(!harness.show(&mut value).editing);
    // A right click that cancels a drag is not also a request for a context menu.
    harness.input.primary_button(true, field().center());
    harness.show(&mut value);
    harness
        .input
        .button(PointerButton::Secondary, true, field().center());
    let cancelled = harness.show(&mut value);
    assert!(cancelled.cancelled() && !cancelled.response.secondary_pressed);
}

#[test]
fn vertical_bars_take_the_pointer_back_to_their_top_and_type_across() {
    let mut harness = Harness::new();
    let mut value = 0.25;
    let tall = Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::new(22.0, 160.0));
    let show = |harness: &mut Harness, value: &mut f64| {
        harness
            .frame(|ui| {
                NumberField::new(ID, value)
                    .range(0.0, 1.0)
                    .bar(BarMapping::Linear)
                    .vertical()
                    .show(ui, tall)
            })
            .0
    };
    show(&mut harness, &mut value);
    harness.input.primary_button(true, tall.center());
    show(&mut harness, &mut value);
    harness.input.pointer_moved_by(Vec2::new(0.0, -10.0));
    show(&mut harness, &mut value);
    let track = tall.inset(harness.styles.number_field.border_width);
    #[allow(clippy::cast_possible_truncation, reason = "a fraction")]
    let fraction = value as f32;
    let y = harness.state.pointer_lock().expect("locked").y;
    assert!(
        (y - (track.max.y - fraction * track.height())).abs() < 0.01,
        "{y}"
    );
}

#[test]
fn a_field_holding_nan_drags_from_zero() {
    let mut harness = Harness::new();
    let mut value = f64::NAN;
    harness.show(&mut value);
    harness.input.primary_button(true, field().center());
    harness.show(&mut value);
    let distance = harness.styles.number_field.step_distance;
    harness
        .input
        .pointer_moved_by(Vec2::new(distance * 2.0, 0.0));
    harness.show(&mut value);
    assert!(close(value, 0.2), "{value}");
}
