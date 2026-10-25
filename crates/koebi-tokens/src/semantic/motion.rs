//! Animation durations and easing curves by intent, and gradient directions.

use super::*;
use crate::primitive;

koebi_design::tokens! {
    group MOTION: Semantic, start = TYPOGRAPHY::END;

    /// Duration of entrance transitions.
    ANIMATION_DURATION_ENTER = "animation.duration.enter": Duration -> primitive::DURATION_FAST;
    /// Duration of exit transitions.
    ANIMATION_DURATION_EXIT = "animation.duration.exit": Duration -> primitive::DURATION_FAST;
    /// Duration of expand animations, such as accordions.
    ANIMATION_DURATION_EXPAND = "animation.duration.expand": Duration -> primitive::DURATION_NORMAL;
    /// Duration of collapse animations.
    ANIMATION_DURATION_COLLAPSE = "animation.duration.collapse": Duration ->
        primitive::DURATION_NORMAL;
    /// Duration of page-level transitions.
    ANIMATION_DURATION_PAGE = "animation.duration.page": Duration -> primitive::DURATION_SLOW;
    /// Period of pulsing and loading rhythms.
    ANIMATION_DURATION_PULSE = "animation.duration.pulse": Duration -> primitive::DURATION_SLOWER;

    /// Easing of entrance transitions.
    ANIMATION_EASING_ENTER = "animation.easing.enter": CubicBezier -> primitive::EASING_DECELERATE;
    /// Easing of exit transitions.
    ANIMATION_EASING_EXIT = "animation.easing.exit": CubicBezier -> primitive::EASING_ACCELERATE;
    /// Easing of direct-manipulation feedback.
    ANIMATION_EASING_INTERACT = "animation.easing.interact": CubicBezier ->
        primitive::EASING_SPRING;
    /// Easing of movements across the screen.
    ANIMATION_EASING_MOVE = "animation.easing.move": CubicBezier -> primitive::EASING_EASE_IN_OUT;

    /// Angle of bottom-to-top gradients.
    GRADIENT_UPWARD_ANGLE = "gradient.upward.angle": Number -> primitive::GRADIENT_ANGLE_0;

    /// Angle of top-to-bottom gradients.
    GRADIENT_DOWNWARD_ANGLE = "gradient.downward.angle": Number -> primitive::GRADIENT_ANGLE_180;
}
