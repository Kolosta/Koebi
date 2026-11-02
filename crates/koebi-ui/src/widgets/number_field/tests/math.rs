//! Tests of the arithmetic of number fields.

use koebi_core::input::Modifiers;

use super::super::math::{BarMapping, DragState, FineMode, Precision, Scale, stepped};

fn unbounded() -> Scale {
    Scale {
        step: 0.1,
        step_distance: 10.0,
        fine: 0.1,
        coarse: 10.0,
        min: f64::NEG_INFINITY,
        max: f64::INFINITY,
        bar: None,
        min_step: 0.0,
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

const FINE_FROM_START: Precision = Precision {
    fine_from_start: true,
    fine_from_here: false,
    round: false,
    keep_digits: false,
};
const FINE_FROM_HERE: Precision = Precision {
    fine_from_start: false,
    fine_from_here: true,
    round: false,
    keep_digits: false,
};
const CTRL: Precision = Precision {
    fine_from_start: false,
    fine_from_here: false,
    round: true,
    keep_digits: false,
};
const ALT: Precision = Precision {
    fine_from_start: false,
    fine_from_here: false,
    round: false,
    keep_digits: true,
};

#[test]
fn unbounded_drags_add_a_step_per_step_distance() {
    let scale = unbounded();
    let mut drag = DragState::new(1.0, &scale);
    assert!(close(drag.value(30.0, Precision::default(), &scale), 1.3));
    assert!(close(drag.value(-10.0, Precision::default(), &scale), 0.9));
}

#[test]
fn each_shift_does_what_the_mode_says() {
    let (left, right) = (Modifiers::LEFT_SHIFT, Modifiers::RIGHT_SHIFT);
    let of = Precision::of;
    assert!(of(left, FineMode::FromStart).fine_from_start);
    assert!(of(right, FineMode::FromStart).fine_from_here);
    assert!(of(left, FineMode::FromHere).fine_from_here);
    assert!(of(right, FineMode::FromHere).fine_from_start);
}

#[test]
fn finer_from_here_goes_on_from_the_value_reached() {
    let scale = unbounded();
    let mut drag = DragState::new(1.0, &scale);
    drag.value(20.0, Precision::default(), &scale);
    // From 1.2, ten times finer.
    assert!(close(drag.value(30.0, FINE_FROM_HERE, &scale), 1.21));
    // Released: full speed again from there, without a jump.
    assert!(close(drag.value(30.0, Precision::default(), &scale), 1.21));
    assert!(close(drag.value(40.0, Precision::default(), &scale), 1.31));
}

#[test]
fn finer_from_the_start_follows_the_share_of_the_drag_both_ways() {
    let scale = unbounded();
    let mut drag = DragState::new(1.0, &scale);
    assert!(close(drag.value(40.0, Precision::default(), &scale), 1.4));
    // As if the whole drag had been fine: 40 px from 1.0.
    assert!(close(drag.value(40.0, FINE_FROM_START, &scale), 1.04));
    assert!(close(drag.value(50.0, FINE_FROM_START, &scale), 1.05));
    // Released: as if it never had been, as many times as wanted.
    assert!(close(drag.value(50.0, Precision::default(), &scale), 1.5));
    assert!(close(drag.value(50.0, FINE_FROM_START, &scale), 1.05));
    assert!(close(drag.value(60.0, Precision::default(), &scale), 1.6));
}

#[test]
fn a_shift_from_the_start_survives_what_the_plain_pace_lost_past_a_bound() {
    // Regression test: past a bound at the plain pace, the travel counted at the fine pace
    // was lost, and Shift came back to the wrong value.
    let scale = Scale {
        min: 0.0,
        max: 10.0,
        bar: Some((BarMapping::Linear, 100.0)),
        ..unbounded()
    };
    let mut drag = DragState::new(5.0, &scale);
    assert!(close(drag.value(300.0, FINE_FROM_START, &scale), 8.0));
    assert!(close(drag.value(300.0, Precision::default(), &scale), 10.0));
    assert!(close(drag.value(300.0, FINE_FROM_START, &scale), 8.0));
    // Below too.
    let mut drag = DragState::new(5.0, &scale);
    assert!(close(drag.value(-300.0, FINE_FROM_START, &scale), 2.0));
    assert!(close(drag.value(-300.0, Precision::default(), &scale), 0.0));
    assert!(close(drag.value(-300.0, FINE_FROM_START, &scale), 2.0));
    // Past the bound first, without Shift: as if the whole drag had been finer.
    let mut drag = DragState::new(5.0, &scale);
    assert!(close(drag.value(90.0, Precision::default(), &scale), 10.0));
    assert!(close(drag.value(90.0, FINE_FROM_START, &scale), 5.9));
}

#[test]
fn both_shifts_are_finer_once() {
    let scale = unbounded();
    let both = Precision {
        fine_from_start: true,
        fine_from_here: true,
        ..Precision::default()
    };
    let mut drag = DragState::new(1.0, &scale);
    assert!(close(drag.value(10.0, both, &scale), 1.01));
}

#[test]
fn a_bar_without_length_stays_put() {
    let scale = Scale {
        min: 0.0,
        max: 10.0,
        bar: Some((BarMapping::Linear, 0.0)),
        ..unbounded()
    };
    let mut drag = DragState::new(5.0, &scale);
    assert!(close(drag.value(50.0, Precision::default(), &scale), 5.0));
    assert!(close(drag.value(50.0, FINE_FROM_START, &scale), 5.0));
}

#[test]
fn a_bar_alternates_between_fine_and_coarse_from_the_start_within_its_bounds() {
    let scale = Scale {
        min: 0.0,
        max: 10.0,
        bar: Some((BarMapping::Linear, 100.0)),
        ..unbounded()
    };
    let mut drag = DragState::new(5.0, &scale);
    assert!(close(drag.value(20.0, Precision::default(), &scale), 7.0));
    assert!(close(drag.value(20.0, FINE_FROM_START, &scale), 5.2));
    assert!(close(drag.value(20.0, Precision::default(), &scale), 7.0));
    // Past the bound, travel is not counted: coming back moves the value at once.
    assert!(close(drag.value(90.0, Precision::default(), &scale), 10.0));
    assert!(close(drag.value(80.0, Precision::default(), &scale), 9.0));
}

#[test]
fn ctrl_goes_to_round_units_at_the_pace_of_steps_and_shift_makes_them_finer() {
    let scale = unbounded();
    let mut drag = DragState::new(1.23, &scale);
    // A unit is 10 steps, 1.0, and takes the travel of one step: 10 px.
    assert!(close(drag.value(7.0, CTRL, &scale), 2.0));
    assert!(close(drag.value(23.0, CTRL, &scale), 4.0));
    // With Shift: units of 0.1, for the same travel.
    let fine = Precision {
        fine_from_here: true,
        ..CTRL
    };
    let mut drag = DragState::new(1.23, &scale);
    assert!(close(drag.value(7.0, fine, &scale), 1.3));
    assert!(close(drag.value(23.0, fine, &scale), 1.5));
}

#[test]
fn alt_goes_by_units_and_keeps_the_last_digits() {
    let scale = unbounded();
    let mut drag = DragState::new(1.23, &scale);
    assert!(close(drag.value(13.0, ALT, &scale), 2.23));
    assert!(close(drag.value(27.0, ALT, &scale), 4.23));
    // With Shift: units of 0.1.
    let fine = Precision {
        fine_from_here: true,
        ..ALT
    };
    let mut drag = DragState::new(1.23, &scale);
    assert!(close(drag.value(13.0, fine, &scale), 1.33));
    // Pressed during a drag: from the value reached.
    let mut drag = DragState::new(1.0, &scale);
    assert!(close(drag.value(4.0, Precision::default(), &scale), 1.04));
    assert!(close(drag.value(14.0, ALT, &scale), 2.04));
}

#[test]
fn integers_have_coarse_units_and_shift_brings_them_back_to_one() {
    let scale = Scale {
        step: 1.0,
        min_step: 1.0,
        ..unbounded()
    };
    let mut drag = DragState::new(7.0, &scale);
    assert!(close(drag.value(12.0, CTRL, &scale), 20.0));
    let fine = Precision {
        fine_from_here: true,
        ..CTRL
    };
    let mut drag = DragState::new(7.0, &scale);
    assert!(close(drag.value(12.0, fine, &scale), 8.0));
}

#[test]
fn ctrl_and_alt_on_a_bar_snap_without_changing_how_far_the_range_runs() {
    let scale = Scale {
        min: 0.0,
        max: 10.0,
        bar: Some((BarMapping::Linear, 100.0)),
        ..unbounded()
    };
    let mut drag = DragState::new(5.0, &scale);
    assert!(close(drag.value(23.0, CTRL, &scale), 7.0));
    let mut drag = DragState::new(5.25, &scale);
    assert!(close(drag.value(23.0, ALT, &scale), 7.25));
}

#[test]
fn step_buttons_follow_the_same_units() {
    let scale = unbounded();
    assert!(close(
        stepped(1.23, 1.0, Precision::default(), &scale),
        1.33
    ));
    assert!(close(stepped(1.23, -1.0, FINE_FROM_HERE, &scale), 1.22));
    assert!(close(stepped(1.23, 1.0, CTRL, &scale), 2.0));
    assert!(close(stepped(2.0, -1.0, CTRL, &scale), 1.0));
    assert!(close(stepped(1.23, 1.0, ALT, &scale), 2.23));
}

#[test]
fn bounded_bars_map_travel_along_their_length_and_clamp() {
    let scale = Scale {
        min: 0.0,
        max: 10.0,
        bar: Some((BarMapping::Linear, 100.0)),
        ..unbounded()
    };
    let mut drag = DragState::new(5.0, &scale);
    assert!(close(drag.value(20.0, Precision::default(), &scale), 7.0));
    assert!(close(drag.value(500.0, Precision::default(), &scale), 10.0));
    assert!(close(drag.value(-500.0, Precision::default(), &scale), 0.0));
    let log = Scale {
        min: 1.0,
        max: 100.0,
        bar: Some((BarMapping::Logarithmic, 100.0)),
        ..unbounded()
    };
    let mut drag = DragState::new(1.0, &log);
    assert!(close(drag.value(50.0, Precision::default(), &log), 10.0));
}

#[test]
fn mappings_go_both_ways() {
    let squared = BarMapping::Custom {
        to_fraction: |value, min, max| ((value - min) / (max - min)).sqrt(),
        from_fraction: |fraction, min, max| (fraction * fraction).mul_add(max - min, min),
    };
    for mapping in [BarMapping::Linear, BarMapping::Logarithmic, squared] {
        for value in [1.0, 4.0, 25.0, 100.0] {
            let fraction = mapping.fraction(value, 1.0, 100.0);
            assert!((0.0..=1.0).contains(&fraction));
            assert!(
                (mapping.value(fraction, 1.0, 100.0) - value).abs() < 1e-9,
                "{mapping:?}"
            );
        }
    }
    // A logarithmic range that does not lie above zero maps linearly.
    assert!(close(BarMapping::Logarithmic.fraction(5.0, 0.0, 10.0), 0.5));
    assert_eq!(BarMapping::Linear.fraction(f64::NAN, 0.0, 1.0), 0.0);
}

#[test]
fn steps_follow_the_modifiers_within_the_bounds() {
    let scale = Scale {
        min: 0.0,
        max: 2.0,
        ..unbounded()
    };
    assert!(close(
        stepped(1.23, 1.0, Precision::default(), &scale),
        1.33
    ));
    assert!(close(stepped(1.23, -1.0, FINE_FROM_HERE, &scale), 1.22));
    assert!(close(stepped(1.95, 1.0, Precision::default(), &scale), 2.0));
    // Past the bound of the range, a unit stops there.
    assert!(close(stepped(1.23, 1.0, ALT, &scale), 2.0));
}

#[test]
fn travel_past_a_bound_is_not_counted() {
    // Regression test: past the end, travel back did nothing until it had undone the
    // travel beyond the end.
    let scale = Scale {
        min: 0.0,
        max: 10.0,
        bar: Some((BarMapping::Linear, 100.0)),
        ..unbounded()
    };
    let mut drag = DragState::new(5.0, &scale);
    assert!(close(drag.value(400.0, Precision::default(), &scale), 10.0));
    assert!(close(drag.value(390.0, Precision::default(), &scale), 9.0));
    // The same with a Shift counting from the start.
    let mut fine = DragState::new(5.0, &scale);
    assert!(close(fine.value(4000.0, FINE_FROM_START, &scale), 10.0));
    assert!(close(fine.value(3900.0, FINE_FROM_START, &scale), 9.0));
}

#[test]
fn integers_never_step_below_one() {
    // Regression test: a finer step rounded back to the same integer.
    let integers = Scale {
        step: 1.0,
        min_step: 1.0,
        ..unbounded()
    };
    assert!(close(stepped(5.0, 1.0, FINE_FROM_HERE, &integers), 6.0));
    // Ctrl with Shift rounds to the finer unit, down to one.
    let fine_round = Precision {
        round: true,
        ..FINE_FROM_HERE
    };
    assert!(close(stepped(5.0, 1.0, fine_round, &integers), 6.0));
    assert!(close(stepped(5.0, 1.0, CTRL, &integers), 10.0));
}

#[test]
fn values_that_are_not_numbers_drag_and_step_from_zero() {
    let scale = unbounded();
    let mut drag = DragState::new(f64::NAN, &scale);
    assert!(close(drag.value(10.0, Precision::default(), &scale), 0.1));
    assert!(close(
        stepped(f64::INFINITY, 1.0, Precision::default(), &scale),
        0.1
    ));
    let nan = BarMapping::Custom {
        to_fraction: |_, _, _| f64::NAN,
        from_fraction: |_, _, _| f64::NAN,
    };
    assert_eq!(nan.fraction(5.0, 0.0, 10.0), 0.0);
}
