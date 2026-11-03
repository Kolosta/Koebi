//! Dragging with the pointer locked and every modifier, cancelling, typing expressions,
//! stepping, bars, vertical fields, and where the pointer comes back.

use koebi_core::expr::MILLIMETER;
use koebi_core::input::Modifiers;
use koebi_core::{Rect, Vec2};

use super::*;
use crate::PointerButton;
use crate::widgets::text_field::{TEXT_CANCEL, TEXT_CONFIRM};

#[test]
fn a_press_locks_the_pointer_and_travel_changes_the_value() {
    let mut harness = Harness::new();
    let mut value = 1.0;
    harness.show(&mut value);
    let press = field().center();
    harness.input.primary_button(true, press);
    harness.show(&mut value);
    // Locked at once, coming back where the press was.
    assert_eq!(harness.state.pointer_lock(), Some(press));
    let mut chain = Vec::new();
    harness.state.chain_at(Some(press), &mut chain);
    assert_eq!(chain, [NUMBER]);
    // The locked pointer reports motion: a step per step distance.
    let three = harness.steps(3.0);
    harness.input.pointer_moved_by(three);
    let dragged = harness.show(&mut value);
    assert!(dragged.dragging && dragged.changed);
    assert!(close(value, 1.3), "{value}");
    harness.input.primary_button(false, press);
    let released = harness.show(&mut value);
    assert_eq!(released.ended, Some(EditEnd::Confirmed));
    assert!(close(value, 1.3));
    // The lock lasts the frame of the release, then the pointer comes back.
    assert_eq!(harness.state.pointer_lock(), Some(press));
    harness.show(&mut value);
    assert_eq!(harness.state.pointer_lock(), None);
}

#[test]
fn shift_makes_drags_finer_and_ctrl_rounds_them() {
    let mut harness = Harness::new();
    let mut value = 1.0;
    harness.show(&mut value);
    harness.input.primary_button(true, field().center());
    harness.show(&mut value);
    let two = harness.steps(2.0);
    harness.input.pointer_moved_by(two);
    harness.show(&mut value);
    assert!(close(value, 1.2), "{value}");
    // Left Shift, without a bar: ten times finer from here.
    harness.input.set_modifiers(Modifiers::LEFT_SHIFT);
    let five = harness.steps(5.0);
    harness.input.pointer_moved_by(five);
    harness.show(&mut value);
    assert!(close(value, 1.25), "{value}");
    // Ctrl: round units of ten steps, each as far as a step.
    harness.input.set_modifiers(Modifiers::LEFT_CTRL);
    let little = harness.steps(0.4);
    harness.input.pointer_moved_by(little);
    harness.show(&mut value);
    assert!(close(value, 2.0), "{value}");
    // Ctrl and Shift: units of one step.
    harness
        .input
        .set_modifiers(Modifiers::LEFT_CTRL.union(Modifiers::LEFT_SHIFT));
    let one = harness.steps(1.0);
    harness.input.pointer_moved_by(one);
    harness.show(&mut value);
    assert!(close(value, 2.1), "{value}");
}

#[test]
fn alt_moves_by_units_and_keeps_the_last_digits() {
    let mut harness = Harness::new();
    let mut value = 1.23;
    harness.show(&mut value);
    harness.input.primary_button(true, field().center());
    harness.show(&mut value);
    harness.input.set_modifiers(Modifiers::LEFT_ALT);
    let two = harness.steps(2.2);
    harness.input.pointer_moved_by(two);
    harness.show(&mut value);
    assert!(close(value, 3.23), "{value}");
    // With Shift: units ten times finer, from the value reached.
    harness
        .input
        .set_modifiers(Modifiers::LEFT_ALT.union(Modifiers::LEFT_SHIFT));
    let one = harness.steps(1.0);
    harness.input.pointer_moved_by(one);
    harness.show(&mut value);
    assert!(close(value, 3.33), "{value}");
}

#[test]
fn escape_delete_and_right_clicks_cancel_and_bring_the_pointer_back() {
    for right_click in [false, true] {
        let mut harness = Harness::new();
        let mut value = 4.0;
        harness.show(&mut value);
        let press = field().center() + Vec2::new(-20.0, 0.0);
        harness.input.primary_button(true, press);
        harness.show(&mut value);
        harness.input.pointer_moved_by(Vec2::new(80.0, 0.0));
        harness.show(&mut value);
        assert!(value > 4.0);
        if right_click {
            harness.input.button(PointerButton::Secondary, true, press);
        } else {
            harness.input.trigger(&NUMBER_CANCEL);
        }
        let cancelled = harness.show(&mut value);
        assert_eq!(cancelled.ended, Some(EditEnd::Cancelled));
        assert!(close(value, 4.0) && cancelled.changed);
        assert_eq!(harness.state.pointer_lock(), Some(press));
        // Moving afterwards changes nothing.
        harness.input.pointer_moved_by(Vec2::new(80.0, 0.0));
        harness.show(&mut value);
        assert!(close(value, 4.0));
    }
}

#[test]
fn enter_keeps_the_value_while_the_button_is_still_down() {
    let mut harness = Harness::new();
    let mut value = 0.0;
    harness.show(&mut value);
    harness.input.primary_button(true, field().center());
    harness.show(&mut value);
    harness.input.pointer_moved_by(Vec2::new(40.0, 0.0));
    harness.show(&mut value);
    let dragged = value;
    harness.input.trigger(&NUMBER_CONFIRM);
    assert_eq!(harness.show(&mut value).ended, Some(EditEnd::Confirmed));
    harness.input.pointer_moved_by(Vec2::new(40.0, 0.0));
    harness.show(&mut value);
    assert!(close(value, dragged));
}

#[test]
fn a_press_without_a_drag_types_an_expression_with_units() {
    let mut harness = Harness::new();
    let mut value = 2.0;
    let show = |harness: &mut Harness, value: &mut f64| {
        harness
            .frame(|ui| {
                NumberField::new(ID, value)
                    .unit(&MILLIMETER)
                    .show(ui, field())
            })
            .0
    };
    show(&mut harness, &mut value);
    harness.input.primary_button(true, field().center());
    show(&mut harness, &mut value);
    harness.input.primary_button(false, field().center());
    show(&mut harness, &mut value);
    // The edit starts with the value selected.
    let editing = show(&mut harness, &mut value);
    assert!(editing.editing);
    assert!(harness.state.is_editing_text());
    harness.input.typed("1cm + 2*3");
    show(&mut harness, &mut value);
    harness.input.trigger(&TEXT_CONFIRM);
    let typed = show(&mut harness, &mut value);
    assert_eq!(typed.ended, Some(EditEnd::Confirmed));
    assert!(typed.changed && close(value, 16.0), "{value}");
    assert!(!show(&mut harness, &mut value).editing);
}

#[test]
fn invalid_expressions_keep_the_edit_open_and_escape_leaves_the_value() {
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
    let refused = harness.show(&mut value);
    assert!(refused.editing && close(value, 3.0));
    assert!(
        harness
            .state
            .number
            .edit
            .as_ref()
            .is_some_and(|edit| edit.error)
    );
    harness.show(&mut value);
    harness.input.trigger(&TEXT_CANCEL);
    let cancelled = harness.show(&mut value);
    assert_eq!(cancelled.ended, Some(EditEnd::Cancelled));
    assert!(close(value, 3.0) && !cancelled.editing);
}

#[test]
fn step_buttons_step_and_their_modifiers_shape_the_step() {
    let mut harness = Harness::new();
    let mut value = 1.23;
    let side = harness.styles.number_field.step_width;
    let left = Vec2::new(field().min.x + side * 0.5, field().center().y);
    let right = Vec2::new(field().max.x - side * 0.5, field().center().y);
    let click = |harness: &mut Harness, value: &mut f64, at: Vec2, modifiers: Modifiers| {
        harness.input.set_modifiers(modifiers);
        harness.input.primary_button(true, at);
        harness.input.primary_button(false, at);
        harness.show(value)
    };
    harness.show(&mut value);
    assert!(click(&mut harness, &mut value, right, Modifiers::NONE).changed);
    assert!(close(value, 1.33), "{value}");
    click(&mut harness, &mut value, left, Modifiers::LEFT_SHIFT);
    assert!(close(value, 1.32), "{value}");
    click(&mut harness, &mut value, right, Modifiers::LEFT_CTRL);
    assert!(close(value, 2.0), "the next round unit: {value}");
    value = 2.25;
    click(&mut harness, &mut value, left, Modifiers::LEFT_ALT);
    assert!(
        close(value, 1.25),
        "a unit down, the last digits kept: {value}"
    );
}

#[test]
fn bounded_bars_run_the_range_along_their_length_and_take_the_pointer_back_there() {
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
    // The bar's track: the whole field inside its border.
    let border = harness.styles.number_field.border_width;
    let track = field().inset(border);
    show(&mut harness, &mut value);
    harness.input.primary_button(true, field().center());
    show(&mut harness, &mut value);
    harness
        .input
        .pointer_moved_by(Vec2::new(track.width() * 0.25, 0.0));
    show(&mut harness, &mut value);
    assert!(close(value, 7.5), "{value}");
    // The pointer comes back at the bar's drawn edge.
    let x = harness.state.pointer_lock().expect("locked").x;
    assert!(
        (x - (track.min.x + track.width() * 0.75)).abs() < 0.01,
        "{x}"
    );
    // Past the end: clamped, and the pointer stays inside the field.
    harness
        .input
        .pointer_moved_by(Vec2::new(track.width() * 2.0, 0.0));
    show(&mut harness, &mut value);
    assert!(close(value, 10.0));
    assert!(harness.state.pointer_lock().expect("locked").x < track.max.x);
}

#[test]
fn vertical_fields_grow_upwards() {
    let mut harness = Harness::new();
    let mut value = 0;
    let tall = Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::new(22.0, 160.0));
    let show = |harness: &mut Harness, value: &mut i32| {
        harness
            .frame(|ui| NumberField::new(ID, value).vertical().show(ui, tall))
            .0
    };
    show(&mut harness, &mut value);
    harness.input.primary_button(true, tall.center());
    show(&mut harness, &mut value);
    let distance = harness.styles.number_field.step_distance;
    harness
        .input
        .pointer_moved_by(Vec2::new(0.0, -3.4 * distance));
    show(&mut harness, &mut value);
    // Integers step by whole numbers and round.
    assert_eq!(value, 3);
}

#[test]
fn disabled_fields_neither_drag_nor_step() {
    let mut harness = Harness::new();
    let mut value = 1.0;
    let show = |harness: &mut Harness, value: &mut f64| {
        harness
            .frame(|ui| NumberField::new(ID, value).enabled(false).show(ui, field()))
            .0
    };
    show(&mut harness, &mut value);
    harness.input.primary_button(true, field().center());
    show(&mut harness, &mut value);
    harness.input.pointer_moved_by(Vec2::new(80.0, 0.0));
    show(&mut harness, &mut value);
    assert_eq!(harness.state.pointer_lock(), None);
    assert!(close(value, 1.0));
}

#[test]
fn fields_show_their_label_value_unit_bar_and_steps_on_hover() {
    let mut harness = Harness::new();
    let mut value = 2.5;
    let draw = |harness: &mut Harness, value: &mut f64, steps: Option<bool>| {
        harness
            .frame(|ui| {
                let field_on_bar = NumberField::new(ID, value)
                    .label("Size")
                    .unit(&MILLIMETER)
                    .range(0.0, 10.0)
                    .bar(BarMapping::Linear);
                match steps {
                    Some(shown) => field_on_bar.step_buttons(shown),
                    None => field_on_bar,
                }
                .show(ui, field());
            })
            .1
    };
    let idle = draw(&mut harness, &mut value, Some(true));
    // The box and the bar; "Size", "2.50" and " mm".
    assert_eq!(idle.rect_count(), 2);
    assert_eq!(idle.glyph_count(), 4 + 4 + 3);
    assert_eq!(idle.line_count(), 0);
    harness.input.pointer_moved(field().center());
    draw(&mut harness, &mut value, Some(true));
    let hovered = draw(&mut harness, &mut value, Some(true));
    // The step buttons' chevrons, asked for: a stroke each.
    assert!(hovered.line_count() >= 2);
    // A field on a bar has none by default.
    let hovered = draw(&mut harness, &mut value, None);
    assert_eq!(hovered.line_count(), 0);
}

#[test]
fn ctrl_stays_of_use_on_a_small_range() {
    // Regression test: a unit of 1 on a range of 1 left only its two ends.
    let mut harness = Harness::new();
    let mut value = 0.37;
    let show = |harness: &mut Harness, value: &mut f64| {
        harness
            .frame(|ui| {
                NumberField::new(ID, value)
                    .range(0.0, 1.0)
                    .bar(BarMapping::Linear)
                    .show(ui, field())
            })
            .0
    };
    show(&mut harness, &mut value);
    harness.input.primary_button(true, field().center());
    show(&mut harness, &mut value);
    harness.input.set_modifiers(Modifiers::LEFT_CTRL);
    let travel = harness.track_share(0.1);
    harness.input.pointer_moved_by(travel);
    show(&mut harness, &mut value);
    assert!(close(value, 0.5), "{value}");
}

#[test]
fn fields_without_a_bar_have_step_buttons_unless_told_otherwise() {
    let mut harness = Harness::new();
    let mut value = 2.5;
    let draw = |harness: &mut Harness, value: &mut f64, steps: Option<bool>| {
        harness
            .frame(|ui| {
                let field_alone = NumberField::new(ID, value);
                match steps {
                    Some(shown) => field_alone.step_buttons(shown),
                    None => field_alone,
                }
                .show(ui, field());
            })
            .1
    };
    draw(&mut harness, &mut value, None);
    harness.input.pointer_moved(field().center());
    draw(&mut harness, &mut value, None);
    assert!(draw(&mut harness, &mut value, None).line_count() >= 2);
    assert_eq!(draw(&mut harness, &mut value, Some(false)).line_count(), 0);
}

#[test]
fn a_field_on_a_bar_counts_shift_from_the_start_and_another_from_here() {
    let mut harness = Harness::new();
    let mut value = 5.0;
    let show = |harness: &mut Harness, value: &mut f64, mode: Option<FineMode>| {
        harness
            .frame(|ui| {
                let field_on_bar = NumberField::new(ID, value)
                    .range(0.0, 10.0)
                    .bar(BarMapping::Linear);
                match mode {
                    Some(mode) => field_on_bar.fine_mode(mode),
                    None => field_on_bar,
                }
                .show(ui, field())
            })
            .0
    };
    show(&mut harness, &mut value, None);
    harness.input.primary_button(true, field().center());
    show(&mut harness, &mut value, None);
    // A fifth of the track: 2.0 more.
    let travel = harness.track_share(0.2);
    harness.input.pointer_moved_by(travel);
    show(&mut harness, &mut value, None);
    assert!(close(value, 7.0), "{value}");
    // Left Shift: as if the whole drag had been twenty times finer.
    harness.input.set_modifiers(Modifiers::LEFT_SHIFT);
    show(&mut harness, &mut value, None);
    assert!(close(value, 5.1), "{value}");
    // Released: back, as if it had never been held.
    harness.input.set_modifiers(Modifiers::NONE);
    show(&mut harness, &mut value, None);
    assert!(close(value, 7.0), "{value}");
    // The right Shift does the other one: from the value reached, without a jump.
    harness.input.set_modifiers(Modifiers::RIGHT_SHIFT);
    show(&mut harness, &mut value, None);
    assert!(close(value, 7.0), "{value}");
    harness.input.set_modifiers(Modifiers::NONE);
    harness.input.primary_button(false, field().center());
    show(&mut harness, &mut value, None);
    // Told otherwise, the left Shift counts from here on a bar too.
    let mut value = 5.0;
    let mut harness = Harness::new();
    let mode = Some(FineMode::FromHere);
    show(&mut harness, &mut value, mode);
    harness.input.primary_button(true, field().center());
    show(&mut harness, &mut value, mode);
    let travel = harness.track_share(0.2);
    harness.input.pointer_moved_by(travel);
    show(&mut harness, &mut value, mode);
    harness.input.set_modifiers(Modifiers::LEFT_SHIFT);
    show(&mut harness, &mut value, mode);
    assert!(close(value, 7.0), "{value}");
}

#[test]
fn a_field_no_longer_shown_drops_its_drag() {
    let mut harness = Harness::new();
    let mut value = 1.0;
    harness.show(&mut value);
    harness.input.primary_button(true, field().center());
    harness.show(&mut value);
    harness.frame(|_| ());
    assert_eq!(harness.state.number.drag, None);
    assert_eq!(harness.state.pointer_lock(), None);
}

/// One frame of a plain field with Ctrl and the wheel: `notches` turned, `modifiers` held.
fn wheel_frame(harness: &mut Harness, value: &mut f64, notches: f32, modifiers: Modifiers) {
    harness.input.set_modifiers(modifiers);
    if notches != 0.0 {
        harness.input.scrolled(Vec2::new(0.0, notches));
    }
    harness.show(value);
}

#[test]
fn the_wheel_changes_a_value_with_ctrl_only() {
    let mut harness = Harness::new();
    let mut value = 1.0;
    harness.input.pointer_moved(field().center());
    harness.show(&mut value);
    harness.show(&mut value);
    // Alone, it leaves the value to what scrolls around the field.
    wheel_frame(&mut harness, &mut value, 1.0, Modifiers::NONE);
    assert!(close(value, 1.0));
    wheel_frame(&mut harness, &mut value, 0.0, Modifiers::LEFT_CTRL);
    wheel_frame(&mut harness, &mut value, 1.0, Modifiers::LEFT_CTRL);
    assert!(close(value, 1.1), "{value}");
    wheel_frame(&mut harness, &mut value, 2.0, Modifiers::LEFT_CTRL);
    assert!(close(value, 1.3), "{value}");
    wheel_frame(&mut harness, &mut value, -1.0, Modifiers::LEFT_CTRL);
    assert!(close(value, 1.2), "{value}");
}

#[test]
fn shift_goes_on_and_off_during_the_wheel_as_in_a_drag() {
    let mut harness = Harness::new();
    let mut value = 1.0;
    let ctrl = Modifiers::LEFT_CTRL;
    let fine = ctrl.union(Modifiers::LEFT_SHIFT);
    harness.input.pointer_moved(field().center());
    harness.show(&mut value);
    harness.show(&mut value);
    wheel_frame(&mut harness, &mut value, 1.0, ctrl);
    assert!(close(value, 1.1), "{value}");
    // Without a bar, the left Shift is finer from here: a tenth of a step, on what shows.
    wheel_frame(&mut harness, &mut value, 1.0, fine);
    assert!(close(value, 1.11), "{value}");
    // Released, it goes on from there.
    wheel_frame(&mut harness, &mut value, 1.0, ctrl);
    assert!(close(value, 1.21), "{value}");
}

#[test]
fn the_wheel_over_a_field_with_a_bar_counts_shift_from_the_start() {
    let mut harness = Harness::new();
    let mut value = 0.5;
    let show = |harness: &mut Harness, value: &mut f64, notches: f32, modifiers: Modifiers| {
        harness.input.set_modifiers(modifiers);
        harness.input.scrolled(Vec2::new(0.0, notches));
        harness
            .frame(|ui| {
                NumberField::new(ID, value)
                    .range(0.0, 100.0)
                    .bar(BarMapping::Linear)
                    .show(ui, field())
            })
            .0
    };
    let ctrl = Modifiers::LEFT_CTRL;
    harness.input.pointer_moved(field().center());
    show(&mut harness, &mut value, 0.0, Modifiers::NONE);
    show(&mut harness, &mut value, 0.0, ctrl);
    // A step of 1 on a range of 100, four notches.
    show(&mut harness, &mut value, 4.0, ctrl);
    assert!(close(value, 4.5), "{value}");
    // The left Shift scales all of it, a twentieth on a bar: 0.2 from 0.5.
    show(
        &mut harness,
        &mut value,
        0.0,
        ctrl.union(Modifiers::LEFT_SHIFT),
    );
    assert!(close(value, 0.7), "{value}");
    // Released: as if it never had been.
    show(&mut harness, &mut value, 0.0, ctrl);
    assert!(close(value, 4.5), "{value}");
}

#[test]
fn shift_on_a_bar_is_blenders_twentieth_or_what_the_call_asks() {
    let mut harness = Harness::new();
    let style = harness.styles.number_field;
    assert!((style.bar_fine_factor - 0.05).abs() < 1e-6);
    assert!((style.fine_factor - 0.1).abs() < 1e-6);
    let mut drag = |factor: Option<f32>| {
        let mut value = 0.0;
        let show = |harness: &mut Harness, value: &mut f64| {
            harness
                .frame(|ui| {
                    let field_on_bar = NumberField::new(ID, value)
                        .range(0.0, 100.0)
                        .bar(BarMapping::Linear);
                    match factor {
                        Some(factor) => field_on_bar.fine_factor(factor),
                        None => field_on_bar,
                    }
                    .show(ui, field())
                })
                .0
        };
        harness.input.set_modifiers(Modifiers::NONE);
        show(&mut harness, &mut value);
        harness.input.primary_button(true, field().center());
        show(&mut harness, &mut value);
        harness.input.set_modifiers(Modifiers::LEFT_SHIFT);
        let travel = harness.track_share(0.4);
        harness.input.pointer_moved_by(travel);
        show(&mut harness, &mut value);
        harness.input.primary_button(false, field().center());
        show(&mut harness, &mut value);
        value
    };
    assert!(close(drag(None), 2.0));
    assert!(close(drag(Some(0.5)), 20.0));
}
