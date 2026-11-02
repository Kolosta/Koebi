//! The arithmetic of number fields, apart from drawing and input so that every rule can be
//! tested alone: where a value lies on a bounded field's bar ([`BarMapping`]), the value a
//! drag gives with its modifiers ([`DragState`]), and what a step button does ([`stepped`]).

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
    /// Alt: units from where the pace last changed, the last digits kept.
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
    /// The step: what one `step_distance` of travel adds to an unbounded value.
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
    /// Pixels of travel counted from `base`, at the plain pace and at the fine one. What goes
    /// past a bound at a pace is not counted at that pace, so that coming back moves the value
    /// at once; each counts on its own, so that a Shift from the start comes back to its value.
    counted: [f64; 2],
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
            counted: [0.0; 2],
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
            self.counted = [0.0; 2];
            self.pace = pace;
        }
        let plain = scale.per_pixel(coarse);
        let rates = [plain, plain * scale.fine];
        let (low, high) = scale.limits();
        for (counted, rate) in self.counted.iter_mut().zip(rates) {
            *counted += delta;
            let position = self.base + *counted * rate;
            if (position < low || position > high) && rate > 0.0 {
                *counted = (position.max(low).min(high) - self.base) / rate;
            }
        }
        let pace = usize::from(precision.is_fine());
        let clamped = (self.base + self.counted[pace] * rates[pace])
            .max(low)
            .min(high);
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
