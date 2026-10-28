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
    // Right Shift: ten times finer from here.
    harness.input.set_modifiers(Modifiers::RIGHT_SHIFT);
    let five = harness.steps(5.0);
    harness.input.pointer_moved_by(five);
    harness.show(&mut value);
    assert!(close(value, 1.25), "{value}");
    // Ctrl: round steps.
    harness.input.set_modifiers(Modifiers::LEFT_CTRL);
    let little = harness.steps(0.4);
    harness.input.pointer_moved_by(little);
    harness.show(&mut value);
    assert!(close(value, 1.3), "{value}");
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
    assert!(close(value, 1.4), "{value}");
    click(&mut harness, &mut value, left, Modifiers::LEFT_ALT);
    assert!(close(value, 1.3), "the last decimal of 1.4 is 0.1: {value}");
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
    let draw = |harness: &mut Harness, value: &mut f64| {
        harness
            .frame(|ui| {
                NumberField::new(ID, value)
                    .label("Size")
                    .unit(&MILLIMETER)
                    .range(0.0, 10.0)
                    .bar(BarMapping::Linear)
                    .show(ui, field());
            })
            .1
    };
    let idle = draw(&mut harness, &mut value);
    // The box and the bar; "Size", "2.50" and " mm".
    assert_eq!(idle.rect_count(), 2);
    assert_eq!(idle.glyph_count(), 4 + 4 + 3);
    assert_eq!(idle.line_count(), 0);
    harness.input.pointer_moved(field().center());
    draw(&mut harness, &mut value);
    let hovered = draw(&mut harness, &mut value);
    // The step buttons' chevrons.
    assert!(hovered.line_count() >= 4);
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
