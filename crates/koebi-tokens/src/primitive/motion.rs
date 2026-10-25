//! Durations, easing curves, stacking layers and gradient geometry.

use super::*;
use koebi_design::define::*;

koebi_design::tokens! {
    group MOTION: Primitive, start = TYPOGRAPHY::END;

    /// Instant: no animation.
    DURATION_INSTANT = "duration.instant": Duration = ms(0), range(0.0, 2000.0);
    /// Fast animation duration.
    DURATION_FAST = "duration.fast": Duration = ms(100), range(0.0, 2000.0);
    /// Normal animation duration.
    DURATION_NORMAL = "duration.normal": Duration = ms(200), range(0.0, 2000.0);
    /// Slow animation duration.
    DURATION_SLOW = "duration.slow": Duration = ms(300), range(0.0, 2000.0);
    /// Slower animation duration.
    DURATION_SLOWER = "duration.slower": Duration = ms(500), range(0.0, 2000.0);
    /// Slowest animation duration.
    DURATION_SLOWEST = "duration.slowest": Duration = ms(800), range(0.0, 2000.0);

    /// Linear easing: constant speed.
    EASING_LINEAR = "easing.linear": CubicBezier = bezier(0.0, 0.0, 1.0, 1.0);
    /// Standard easing: fast acceleration, long deceleration.
    EASING_EASE = "easing.ease": CubicBezier = bezier(0.25, 0.1, 0.25, 1.0);
    /// Ease-in: starts slowly.
    EASING_EASE_IN = "easing.ease-in": CubicBezier = bezier(0.42, 0.0, 1.0, 1.0);
    /// Ease-out: ends slowly.
    EASING_EASE_OUT = "easing.ease-out": CubicBezier = bezier(0.0, 0.0, 0.58, 1.0);
    /// Ease-in-out: starts and ends slowly.
    EASING_EASE_IN_OUT = "easing.ease-in-out": CubicBezier = bezier(0.42, 0.0, 0.58, 1.0);
    /// Spring easing with an overshoot, for physical interactions.
    EASING_SPRING = "easing.spring": CubicBezier = bezier(0.5, 1.5, 0.5, 1.0);
    /// Decelerating easing, for entrances.
    EASING_DECELERATE = "easing.decelerate": CubicBezier = bezier(0.0, 0.0, 0.2, 1.0);
    /// Accelerating easing, for exits.
    EASING_ACCELERATE = "easing.accelerate": CubicBezier = bezier(0.4, 0.0, 1.0, 1.0);
    /// Easing with a subtle bounce past the target.
    EASING_OVERSHOOT = "easing.overshoot": CubicBezier = bezier(0.34, 1.56, 0.64, 1.0);

    /// Stacking layer `ground`: higher layers draw on top.
    LAYER_GROUND = "layer.ground": Integer = 0, range(0.0, 10_000.0);
    /// Stacking layer `low`: higher layers draw on top.
    LAYER_LOW = "layer.low": Integer = 10, range(0.0, 10_000.0);
    /// Stacking layer `mid`: higher layers draw on top.
    LAYER_MID = "layer.mid": Integer = 100, range(0.0, 10_000.0);
    /// Stacking layer `high`: higher layers draw on top.
    LAYER_HIGH = "layer.high": Integer = 200, range(0.0, 10_000.0);
    /// Stacking layer `veil`: higher layers draw on top.
    LAYER_VEIL = "layer.veil": Integer = 300, range(0.0, 10_000.0);
    /// Stacking layer `raised`: higher layers draw on top.
    LAYER_RAISED = "layer.raised": Integer = 400, range(0.0, 10_000.0);
    /// Stacking layer `peak`: higher layers draw on top.
    LAYER_PEAK = "layer.peak": Integer = 500, range(0.0, 10_000.0);
    /// Stacking layer `critical`: higher layers draw on top.
    LAYER_CRITICAL = "layer.critical": Integer = 600, range(0.0, 10_000.0);
    /// Stacking layer `absolute`: higher layers draw on top.
    LAYER_ABSOLUTE = "layer.absolute": Integer = 9999, range(0.0, 10_000.0);

    /// Gradient angle of 0 degrees.
    GRADIENT_ANGLE_0 = "gradient.angle.0": Number = 0.0, range(0.0, 360.0);
    /// Gradient angle of 45 degrees.
    GRADIENT_ANGLE_45 = "gradient.angle.45": Number = 45.0, range(0.0, 360.0);
    /// Gradient angle of 90 degrees.
    GRADIENT_ANGLE_90 = "gradient.angle.90": Number = 90.0, range(0.0, 360.0);
    /// Gradient angle of 135 degrees.
    GRADIENT_ANGLE_135 = "gradient.angle.135": Number = 135.0, range(0.0, 360.0);
    /// Gradient angle of 180 degrees.
    GRADIENT_ANGLE_180 = "gradient.angle.180": Number = 180.0, range(0.0, 360.0);
    /// Gradient angle of 225 degrees.
    GRADIENT_ANGLE_225 = "gradient.angle.225": Number = 225.0, range(0.0, 360.0);
    /// Gradient angle of 270 degrees.
    GRADIENT_ANGLE_270 = "gradient.angle.270": Number = 270.0, range(0.0, 360.0);
    /// Gradient angle of 315 degrees.
    GRADIENT_ANGLE_315 = "gradient.angle.315": Number = 315.0, range(0.0, 360.0);

    /// Gradient stop at 0 % of the gradient.
    GRADIENT_STOP_0 = "gradient.stop.0": Number = 0.0, range(0.0, 1.0);
    /// Gradient stop at 25 % of the gradient.
    GRADIENT_STOP_25 = "gradient.stop.25": Number = 0.25, range(0.0, 1.0);
    /// Gradient stop at 50 % of the gradient.
    GRADIENT_STOP_50 = "gradient.stop.50": Number = 0.5, range(0.0, 1.0);
    /// Gradient stop at 75 % of the gradient.
    GRADIENT_STOP_75 = "gradient.stop.75": Number = 0.75, range(0.0, 1.0);
    /// Gradient stop at 100 % of the gradient.
    GRADIENT_STOP_100 = "gradient.stop.100": Number = 1.0, range(0.0, 1.0);
}
