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

/// The modifiers that shape a drag or a step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "four modifier keys, each held or not on its own"
)]
pub(crate) struct Precision {
    /// Left Shift: finer, as if the whole drag had been finer from its start.
    pub(crate) fine_from_start: bool,
    /// Right Shift: finer from the current value on.
    pub(crate) fine_from_here: bool,
    /// Ctrl: round steps.
    pub(crate) round: bool,
    /// Alt: steps of the last decimal the value had when the drag started.
    pub(crate) keep_decimal: bool,
}

impl Precision {
    /// The precision the modifiers held ask for.
    pub(crate) const fn of(modifiers: Modifiers) -> Self {
        let shift = modifiers.sides(ModifierKey::Shift);
        Self {
            fine_from_start: shift.left,
            fine_from_here: shift.right,
            round: modifiers.sides(ModifierKey::Ctrl).any(),
            keep_decimal: modifiers.sides(ModifierKey::Alt).any(),
        }
    }

    /// `true` with either Shift.
    pub(crate) const fn is_fine(self) -> bool {
        self.fine_from_start || self.fine_from_here
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
    /// Decimals shown: the last decimal never goes below them.
    pub(crate) decimals: usize,
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

    /// How far one pixel of travel moves the position.
    fn per_pixel(&self) -> f64 {
        match self.bar {
            Some((_, length)) if length > 0.0 => 1.0 / f64::from(length),
            Some(_) => 0.0,
            None => self.step / f64::from(self.step_distance.max(1.0)),
        }
    }
}

/// A drag in progress: where it started, and where travel counts from since the last change
/// of precision.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DragState {
    /// The value at the press.
    pub(crate) start: f64,
    /// The position (see [`Scale::position`]) and travel the current precision counts from.
    anchor: (f64, f32),
    /// The precision travel counted with on the last frame.
    fine_here: bool,
    /// The left Shift was held on the last frame.
    fine_start: bool,
    /// Travel the left Shift skips: what went past a bound, which coming back does not undo.
    skipped: f32,
    /// The unsnapped position and the travel of the last frame.
    last: (f64, f32),
}

impl DragState {
    /// A drag of a field holding `start`.
    pub(crate) fn new(start: f64, scale: &Scale) -> Self {
        let start = scale.finite(start);
        let position = scale.position(start);
        Self {
            start,
            anchor: (position, 0.0),
            fine_here: false,
            fine_start: false,
            skipped: 0.0,
            last: (position, 0.0),
        }
    }

    /// The value after `travel` pixels from the press, with `precision`: finer with Shift
    /// (left: as if from the start; right: from here), on round steps with Ctrl (finer ones
    /// with Shift too), on steps of the start's last decimal with Alt, within the bounds.
    /// Travel past a bound is not counted: coming back moves the value at once.
    pub(crate) fn value(&mut self, travel: f32, precision: Precision, scale: &Scale) -> f64 {
        // A change of precision: travel counts from where the drag is, without a jump,
        // unless the left Shift asks to count from the start.
        if precision.fine_from_here != self.fine_here
            || precision.fine_from_start != self.fine_start
        {
            self.anchor = self.last;
            if precision.fine_from_start && !self.fine_start {
                self.skipped = 0.0;
            }
            self.fine_here = precision.fine_from_here;
            self.fine_start = precision.fine_from_start;
        }
        let per_pixel = scale.per_pixel();
        let origin = scale.position(self.start);
        let position = if precision.fine_from_start {
            origin + f64::from(travel - self.skipped) * per_pixel * scale.fine
        } else {
            let factor = if precision.fine_from_here {
                scale.fine
            } else {
                1.0
            };
            let (from, at) = self.anchor;
            from + f64::from(travel - at) * per_pixel * factor
        };
        let (low, high) = scale.limits();
        let clamped = position.max(low).min(high);
        if position < low || position > high {
            // Past a bound: travel counts from the bound, as if the press had moved there.
            if precision.fine_from_start {
                let rate = per_pixel * scale.fine;
                if rate > 0.0 {
                    #[allow(clippy::cast_possible_truncation, reason = "travel in pixels fits f32")]
                    let counted = ((clamped - origin) / rate) as f32;
                    self.skipped = travel - counted;
                }
            } else {
                self.anchor = (clamped, travel);
            }
        }
        self.last = (clamped, travel);
        let raw = scale.at(clamped);
        let snapped = if precision.round && scale.step > 0.0 {
            let unit = if precision.is_fine() {
                scale.fine_step()
            } else {
                scale.step
            };
            (raw / unit).round() * unit
        } else if precision.keep_decimal {
            let unit = last_decimal(self.start, scale.decimals).max(scale.min_step);
            self.start + ((raw - self.start) / unit).round() * unit
        } else {
            raw
        };
        snapped.max(scale.min).min(scale.max)
    }
}

/// The value a step button gives: `direction` (+1 or -1) times the step, a finer step with
/// Shift, to the next round step with Ctrl (a finer one with Shift too), by the value's last
/// decimal with Alt; within the bounds.
pub(crate) fn stepped(value: f64, direction: f64, precision: Precision, scale: &Scale) -> f64 {
    let value = scale.finite(value);
    let step = if precision.is_fine() {
        scale.fine_step()
    } else {
        scale.step
    };
    let next = if precision.round && step > 0.0 {
        // The next multiple of the step that way, even from a multiple.
        let steps = value / step;
        let target = if direction > 0.0 {
            (steps + 1e-9).floor() + 1.0
        } else {
            (steps - 1e-9).ceil() - 1.0
        };
        target * step
    } else if precision.keep_decimal {
        direction.mul_add(
            last_decimal(value, scale.decimals).max(scale.min_step),
            value,
        )
    } else {
        direction.mul_add(step, value)
    };
    next.max(scale.min).min(scale.max)
}

/// The unit of the last decimal of `value` written with at most `decimals` decimals: 0.01
/// for 1.25, 1 for 3, 10⁻ᵈᵉᶜⁱᵐᵃˡˢ when it needs more.
pub(crate) fn last_decimal(value: f64, decimals: usize) -> f64 {
    let mut unit = 1.0;
    for _ in 0..decimals {
        let scaled = value / unit;
        if (scaled - scaled.round()).abs() < 1e-6 * scaled.abs().max(1.0) {
            return unit;
        }
        unit /= 10.0;
    }
    unit
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unbounded() -> Scale {
        Scale {
            step: 0.1,
            step_distance: 10.0,
            fine: 0.1,
            decimals: 3,
            min: f64::NEG_INFINITY,
            max: f64::INFINITY,
            bar: None,
            min_step: 0.0,
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    const LEFT_SHIFT: Precision = Precision {
        fine_from_start: true,
        fine_from_here: false,
        round: false,
        keep_decimal: false,
    };
    const RIGHT_SHIFT: Precision = Precision {
        fine_from_start: false,
        fine_from_here: true,
        round: false,
        keep_decimal: false,
    };
    const CTRL: Precision = Precision {
        fine_from_start: false,
        fine_from_here: false,
        round: true,
        keep_decimal: false,
    };
    const ALT: Precision = Precision {
        fine_from_start: false,
        fine_from_here: false,
        round: false,
        keep_decimal: true,
    };

    #[test]
    fn unbounded_drags_add_a_step_per_step_distance() {
        let scale = unbounded();
        let mut drag = DragState::new(1.0, &scale);
        assert!(close(drag.value(30.0, Precision::default(), &scale), 1.3));
        assert!(close(drag.value(-10.0, Precision::default(), &scale), 0.9));
    }

    #[test]
    fn the_right_shift_is_finer_from_here_and_the_left_from_the_start() {
        let scale = unbounded();
        let mut drag = DragState::new(1.0, &scale);
        drag.value(20.0, Precision::default(), &scale);
        // Right Shift: from 1.2, ten times finer.
        assert!(close(drag.value(30.0, RIGHT_SHIFT, &scale), 1.21));
        // Released: full speed again from there.
        assert!(close(drag.value(40.0, Precision::default(), &scale), 1.31));
        // Left Shift: as if the whole drag had been fine: 40 px from 1.0.
        assert!(close(drag.value(40.0, LEFT_SHIFT, &scale), 1.04));
        assert!(close(drag.value(50.0, LEFT_SHIFT, &scale), 1.05));
        // Released: from there, without a jump.
        assert!(close(drag.value(50.0, Precision::default(), &scale), 1.05));
    }

    #[test]
    fn ctrl_rounds_to_steps_and_alt_keeps_the_start_decimal() {
        let scale = Scale {
            step: 0.5,
            ..unbounded()
        };
        let mut drag = DragState::new(1.23, &scale);
        assert!(close(drag.value(7.0, CTRL, &scale), 1.5));
        let scale = unbounded();
        let mut drag = DragState::new(1.23, &scale);
        // 13 px: +0.13, in steps of 0.01 from 1.23.
        assert!(close(drag.value(13.0, ALT, &scale), 1.36));
        let mut whole = DragState::new(3.0, &scale);
        // 13 px from 3: +0.13, in steps of 1: 3.
        assert!(close(whole.value(13.0, ALT, &scale), 3.0));
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
    fn steps_follow_the_modifiers() {
        let scale = Scale {
            min: 0.0,
            max: 2.0,
            ..unbounded()
        };
        assert!(close(
            stepped(1.23, 1.0, Precision::default(), &scale),
            1.33
        ));
        assert!(close(stepped(1.23, -1.0, RIGHT_SHIFT, &scale), 1.22));
        assert!(close(stepped(1.23, 1.0, CTRL, &scale), 1.3));
        assert!(close(stepped(1.3, 1.0, CTRL, &scale), 1.4));
        assert!(close(stepped(1.3, -1.0, CTRL, &scale), 1.2));
        assert!(close(stepped(1.23, 1.0, ALT, &scale), 1.24));
        assert!(close(stepped(1.95, 1.0, Precision::default(), &scale), 2.0));
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
        // The same with the left Shift, which counts from the start.
        let mut fine = DragState::new(5.0, &scale);
        assert!(close(fine.value(4000.0, LEFT_SHIFT, &scale), 10.0));
        assert!(close(fine.value(3900.0, LEFT_SHIFT, &scale), 9.0));
    }

    #[test]
    fn integers_never_step_below_one_and_shift_rounds_finer() {
        // Regression test: a finer step rounded back to the same integer.
        let integers = Scale {
            step: 1.0,
            min_step: 1.0,
            decimals: 0,
            ..unbounded()
        };
        assert!(close(stepped(5.0, 1.0, RIGHT_SHIFT, &integers), 6.0));
        assert!(close(stepped(5.0, -1.0, ALT, &integers), 4.0));
        // Ctrl with Shift rounds to the finer step.
        let shift_ctrl = Precision {
            round: true,
            ..RIGHT_SHIFT
        };
        assert!(close(stepped(1.23, 1.0, shift_ctrl, &unbounded()), 1.24));
        let mut drag = DragState::new(1.0, &unbounded());
        assert!(close(drag.value(23.0, shift_ctrl, &unbounded()), 1.02));
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

    #[test]
    fn last_decimals_stop_at_the_decimals_shown() {
        assert!(close(last_decimal(1.25, 3), 0.01));
        assert!(close(last_decimal(3.0, 3), 1.0));
        assert!(close(last_decimal(0.1, 3), 0.1));
        assert!(close(last_decimal(1.23456, 3), 0.001));
        assert!(close(last_decimal(7.0, 0), 1.0));
    }
}
