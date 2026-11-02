//! The arithmetic of number fields, apart from drawing and input so that every rule can be
//! tested alone: where a value lies on a bounded field's bar ([`BarMapping`]), the value a
//! drag gives with its modifiers ([`DragState`]), what a step button does ([`stepped`]), and
//! the last decimal of a value ([`last_decimal`]).

use koebi_core::input::{ModifierKey, Modifiers};

/// How the bar of a bounded field maps values to its length, and travel along it to values.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub enum BarMapping {
    /// Equal travel adds equal amounts (the default).
    #[default]
    Linear,
    /// Equal travel multiplies by equal factors: frequencies, scales. A range that does not
    /// lie above zero maps linearly.
    Logarithmic,
    /// The caller's mapping: `to_fraction(value, min, max)` in `0.0..=1.0`, and its inverse
    /// `from_fraction(fraction, min, max)`.
    Custom {
        /// From a value to where it lies on the bar.
        to_fraction: fn(f64, f64, f64) -> f64,
        /// From where on the bar to a value.
        from_fraction: fn(f64, f64, f64) -> f64,
    },
}

impl BarMapping {
    /// Where `value` lies on the bar of a field from `min` to `max`, from 0 to 1.
    pub fn fraction(&self, value: f64, min: f64, max: f64) -> f64 {
        let fraction = match *self {
            Self::Logarithmic if min > 0.0 && max > min => {
                (value.max(min) / min).ln() / (max / min).ln()
            }
            Self::Custom { to_fraction, .. } => to_fraction(value, min, max),
            _ if max > min => (value - min) / (max - min),
            _ => 0.0,
        };
        if fraction.is_nan() {
            0.0
        } else {
            fraction.clamp(0.0, 1.0)
        }
    }

    /// The value at `fraction` (0 to 1) of the bar of a field from `min` to `max`.
    pub fn value(&self, fraction: f64, min: f64, max: f64) -> f64 {
        let fraction = if fraction.is_nan() {
            0.0
        } else {
            fraction.clamp(0.0, 1.0)
        };
        match *self {
            Self::Logarithmic if min > 0.0 && max > min => min * (max / min).powf(fraction),
            Self::Custom { from_fraction, .. } => from_fraction(fraction, min, max),
            _ => fraction.mul_add(max - min, min),
        }
    }
}

/// What the left Shift does to a drag; the right Shift does the other one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FineMode {
    /// As if the whole drag had been finer since its start, and when released, as if it never
    /// had been: the value follows the share of the drag done so far. The default of a field
    /// showing a bar.
    FromStart,
    /// Finer from the value reached, and from there again when released, without a jump. The
    /// default of other fields.
    #[default]
    FromHere,
}

/// The modifiers that shape a drag or a step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "four modifier keys, each held or not on its own"
)]
pub(crate) struct Precision {
    /// Finer, as if the whole drag had been finer from its start.
    pub(crate) fine_from_start: bool,
    /// Finer from the current value on.
    pub(crate) fine_from_here: bool,
    /// Ctrl: round units.
    pub(crate) round: bool,
    /// Alt: units from where it started, its last digits kept.
    pub(crate) keep_digits: bool,
}

impl Precision {
    /// The precision the modifiers held ask for, in a field whose left Shift does `mode`.
    pub(crate) const fn of(modifiers: Modifiers, mode: FineMode) -> Self {
        let shift = modifiers.sides(ModifierKey::Shift);
        let (from_start, from_here) = match mode {
            FineMode::FromStart => (shift.left, shift.right),
            FineMode::FromHere => (shift.right, shift.left),
        };
        Self {
            fine_from_start: from_start,
            fine_from_here: from_here,
            round: modifiers.sides(ModifierKey::Ctrl).any(),
            keep_digits: modifiers.sides(ModifierKey::Alt).any(),
        }
    }

    /// `true` with either Shift.
    pub(crate) const fn is_fine(self) -> bool {
        self.fine_from_start || self.fine_from_here
    }

    /// `true` with Ctrl or Alt: the value moves by units, larger than steps.
    pub(crate) const fn is_coarse(self) -> bool {
        self.round || self.keep_digits
    }
}

/// What a drag needs to turn travel into values.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Scale {
    /// The step: what one `step_distance` of travel adds to an unbounded value, and what
    /// round steps are multiples of.
    pub(crate) step: f64,
    /// Pointer travel that adds one step to an unbounded value.
    pub(crate) step_distance: f32,
    /// How much finer Shift makes the drag.
    pub(crate) fine: f64,
    /// How many steps make a unit, what Ctrl and Alt move by.
    pub(crate) coarse: f64,
    /// The bounds; a bounded field with a bar maps travel along the bar's `length`.
    pub(crate) min: f64,
    /// See `min`.
    pub(crate) max: f64,
    /// The bar's mapping and length, for a bounded field that shows one.
    pub(crate) bar: Option<(BarMapping, f32)>,
    /// The smallest step a button or a round makes: 1 for integers, which a finer step could
    /// not change.
    pub(crate) min_step: f64,
}

impl Scale {
    /// Where `value` is in the space travel moves through: the bar's fraction, or the value.
    fn position(&self, value: f64) -> f64 {
        match self.bar {
            Some((mapping, _)) => mapping.fraction(value, self.min, self.max),
            None => value,
        }
    }

    /// The value at `position` (see [`Scale::position`]).
    fn at(&self, position: f64) -> f64 {
        match self.bar {
            Some((mapping, _)) => mapping.value(position, self.min, self.max),
            None => position,
        }
    }

    /// The positions of the bounds: where travel stops.
    fn limits(&self) -> (f64, f64) {
        match self.bar {
            Some(_) => (0.0, 1.0),
            None => (self.min, self.max),
        }
    }

    /// `value` if it is a number, else the number nearest to 0 within the bounds: where a
    /// field holding NaN or an infinity drags and steps from.
    pub(crate) fn finite(&self, value: f64) -> f64 {
        if value.is_finite() {
            value
        } else {
            0.0_f64.max(self.min).min(self.max)
        }
    }

    /// The step Shift makes: finer, but not below the smallest step.
    fn fine_step(&self) -> f64 {
        (self.step * self.fine).max(self.min_step)
    }

    /// The unit Ctrl and Alt move by: `coarse` steps, finer with Shift, but not below the
    /// smallest step.
    fn unit(&self, fine: bool) -> f64 {
        let unit = self.step * self.coarse;
        let unit = if fine { unit * self.fine } else { unit };
        unit.max(self.min_step)
    }

    /// How far one pixel of travel moves the position: along a bar, its length runs the range;
    /// elsewhere, a `step_distance` adds a step, or a unit when `coarse`. A unit is then as far
    /// as a step is, instead of `coarse` times farther.
    fn per_pixel(&self, coarse: bool) -> f64 {
        match self.bar {
            Some((_, length)) if length > 0.0 => 1.0 / f64::from(length),
            Some(_) => 0.0,
            None => {
                let step = if coarse { self.unit(false) } else { self.step };
                step / f64::from(self.step_distance.max(1.0))
            }
        }
    }
}

/// A drag in progress: where it started, and how travel adds up since the last change of pace.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DragState {
    /// The value at the press.
    pub(crate) start: f64,
    /// The position (see [`Scale::position`]) travel counts from.
    base: f64,
    /// Pixels of travel counted from `base`. What goes past a bound is not counted, so that
    /// coming back moves the value at once.
    counted: f64,
    /// The pace travel counted at on the last frame: finer from here, and by units.
    pace: (bool, bool),
    /// The travel of the last frame.
    travel: f32,
    /// The value of the last frame.
    shown: f64,
}

impl DragState {
    /// A drag of a field holding `start`.
    pub(crate) fn new(start: f64, scale: &Scale) -> Self {
        let start = scale.finite(start);
        Self {
            start,
            base: scale.position(start),
            counted: 0.0,
            pace: (false, false),
            travel: 0.0,
            shown: start,
        }
    }

    /// The value after `travel` pixels from the press, with `precision`, within the bounds:
    /// - finer with Shift, as if from the start or from here ([`FineMode`]);
    /// - on round units with Ctrl, or on units from where the pace last changed with Alt, which
    ///   keeps the last digits; finer ones with Shift too. A unit takes as much travel as a step
    ///   on a field without a bar. Along a bar, travel keeps its meaning: it runs the range.
    pub(crate) fn value(&mut self, travel: f32, precision: Precision, scale: &Scale) -> f64 {
        let delta = f64::from(travel - self.travel);
        self.travel = travel;
        let coarse = precision.is_coarse() && scale.bar.is_none();
        let pace = (precision.fine_from_here, coarse);
        if pace != self.pace {
            // Another pace counts from where the drag is. A Shift counting from the start does
            // not change the pace: it scales everything counted so far.
            self.base = scale.position(self.shown);
            self.counted = 0.0;
            self.pace = pace;
        }
        let fine = if precision.is_fine() { scale.fine } else { 1.0 };
        let rate = scale.per_pixel(coarse) * fine;
        self.counted += delta;
        let position = self.base + self.counted * rate;
        let (low, high) = scale.limits();
        let clamped = position.max(low).min(high);
        if (position < low || position > high) && rate > 0.0 {
            self.counted = (clamped - self.base) / rate;
        }
        let raw = scale.at(clamped);
        let unit = scale.unit(precision.is_fine());
        let snapped = if precision.round && unit > 0.0 {
            (raw / unit).round() * unit
        } else if precision.keep_digits && unit > 0.0 {
            let from = scale.at(self.base);
            from + ((raw - from) / unit).round() * unit
        } else {
            raw
        };
        self.shown = snapped.max(scale.min).min(scale.max);
        self.shown
    }
}

/// The value a step button gives: `direction` (+1 or -1) times the step, a finer step with
/// Shift; with Ctrl, to the next round unit; with Alt, by a unit, which keeps the last digits;
/// units are finer with Shift too. Within the bounds.
pub(crate) fn stepped(value: f64, direction: f64, precision: Precision, scale: &Scale) -> f64 {
    let value = scale.finite(value);
    let unit = scale.unit(precision.is_fine());
    let next = if precision.round && unit > 0.0 {
        // The next multiple of the unit that way, even from a multiple.
        let units = value / unit;
        let target = if direction > 0.0 {
            (units + 1e-9).floor() + 1.0
        } else {
            (units - 1e-9).ceil() - 1.0
        };
        target * unit
    } else if precision.keep_digits {
        direction.mul_add(unit, value)
    } else {
        let step = if precision.is_fine() {
            scale.fine_step()
        } else {
            scale.step
        };
        direction.mul_add(step, value)
    };
    next.max(scale.min).min(scale.max)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
