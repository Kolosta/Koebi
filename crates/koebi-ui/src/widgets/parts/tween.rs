//! [`Tween`]: a value from 0 to 1 animating towards a target with a duration and an easing
//! curve from tokens: the openness of a fold, the growth of a scrollbar under the pointer.
//!
//! A new target restarts the animation from where the value is, so that a target moving every
//! frame (the pointer's distance) is followed smoothly. The duration is the time of a whole
//! change, from 0 to 1: a shorter change takes proportionally less.

use core::time::Duration;

use koebi_design::value::Curve;

/// A value from 0 to 1 animating towards a target.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Tween {
    /// The value when the animation started.
    from: f32,
    /// The target.
    to: f32,
    /// The value now.
    current: f32,
    /// When the animation started; `None` at rest on the target.
    start: Option<Duration>,
}

impl Tween {
    /// At rest on `value`.
    pub(crate) const fn new(value: f32) -> Self {
        Self {
            from: value,
            to: value,
            current: value,
            start: None,
        }
    }

    /// The value now.
    pub(crate) const fn value(&self) -> f32 {
        self.current
    }

    /// Jumps to `value`, without animation.
    pub(crate) fn set(&mut self, value: f32) {
        *self = Self::new(value);
    }

    /// Moves towards `target` at `now`, a whole change taking `duration` along `curve`.
    /// Returns `true` while the value still animates: ask for the next frame.
    pub(crate) fn advance(
        &mut self,
        target: f32,
        now: Duration,
        duration: Duration,
        curve: &Curve,
    ) -> bool {
        // Changes too small to see do not restart the animation.
        if (target - self.to).abs() > 1e-4 {
            self.from = self.current;
            self.to = target;
            self.start = Some(now);
        }
        let Some(start) = self.start else {
            return false;
        };
        let length = duration.as_secs_f32() * (self.to - self.from).abs();
        let t = if length > 0.0 {
            now.saturating_sub(start).as_secs_f32() / length
        } else {
            1.0
        };
        if t >= 1.0 {
            self.set(self.to);
            return false;
        }
        self.current = curve.ease(t).mul_add(self.to - self.from, self.from);
        true
    }
}

#[cfg(test)]
mod tests {
    use koebi_design::value::{CubicBezier, CurveKey};

    use super::*;

    const MS: fn(u64) -> Duration = Duration::from_millis;

    #[test]
    fn a_tween_follows_its_curve_to_the_target() {
        let mut tween = Tween::new(0.0);
        assert!(tween.advance(1.0, MS(1000), MS(200), &Curve::LINEAR));
        assert_eq!(tween.value(), 0.0);
        assert!(tween.advance(1.0, MS(1050), MS(200), &Curve::LINEAR));
        assert!((tween.value() - 0.25).abs() < 1e-5);
        // Past the duration, it rests on the target.
        assert!(!tween.advance(1.0, MS(1200), MS(200), &Curve::LINEAR));
        assert_eq!(tween.value(), 1.0);
        assert!(!tween.advance(1.0, MS(1300), MS(200), &Curve::LINEAR));
    }

    #[test]
    fn a_new_target_restarts_from_the_value_and_short_changes_take_less_time() {
        let mut tween = Tween::new(0.0);
        tween.advance(1.0, MS(0), MS(200), &Curve::LINEAR);
        tween.advance(1.0, MS(100), MS(200), &Curve::LINEAR);
        assert!((tween.value() - 0.5).abs() < 1e-5);
        // Back to 0 from 0.5: half a change, half the duration.
        assert!(tween.advance(0.0, MS(100), MS(200), &Curve::LINEAR));
        tween.advance(0.0, MS(150), MS(200), &Curve::LINEAR);
        assert!((tween.value() - 0.25).abs() < 1e-5);
        assert!(!tween.advance(0.0, MS(200), MS(200), &Curve::LINEAR));
        assert_eq!(tween.value(), 0.0);
    }

    #[test]
    fn curves_shape_the_way_and_zero_durations_jump() {
        let settle = Curve::new(&[
            CurveKey::new(0.0, 0.0, CubicBezier::LINEAR),
            CurveKey::new(0.5, 1.2, CubicBezier::LINEAR),
            CurveKey::new(1.0, 1.0, CubicBezier::LINEAR),
        ]);
        let mut tween = Tween::new(0.0);
        tween.advance(1.0, MS(0), MS(100), &settle);
        tween.advance(1.0, MS(50), MS(100), &settle);
        assert!(
            (tween.value() - 1.2).abs() < 1e-5,
            "past the target: {}",
            tween.value()
        );
        let mut instant = Tween::new(0.0);
        assert!(!instant.advance(1.0, MS(0), Duration::ZERO, &settle));
        assert_eq!(instant.value(), 1.0);
    }
}
