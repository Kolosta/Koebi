//! Token values and definitions.
//!
//! A token is *defined* either by a value or by an alias to another token. Token tables are
//! compiled into [`Def`]s holding [`Literal`]s (plain data, usable in `const`); run-time layers
//! (themes created at run time, overrides) hold [`Definition`]s with owned [`Value`]s.

use core::fmt;
use core::time::Duration;
use std::sync::Arc;

use koebi_core::{Color, Oklch};

use crate::kind::{Kind, KindId};
use crate::tier::{MayAlias, Tier};
use crate::token::{Token, TokenId};

/// A cubic Bézier easing curve from `(0, 0)` to `(1, 1)` through two control points, the usual
/// definition of easing curves in design and animation tools.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier {
    /// First control point, x (`0.0..=1.0`).
    pub x1: f32,
    /// First control point, y (may leave `0..1` to overshoot).
    pub y1: f32,
    /// Second control point, x (`0.0..=1.0`).
    pub x2: f32,
    /// Second control point, y (may leave `0..1` to overshoot).
    pub y2: f32,
}

impl CubicBezier {
    /// Constant speed.
    pub const LINEAR: Self = Self::new(0.0, 0.0, 1.0, 1.0);

    /// Creates a curve from its two control points.
    pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// `true` when every coordinate is finite and both x coordinates lie in `0.0..=1.0`.
    pub const fn is_valid(&self) -> bool {
        self.x1 >= 0.0
            && self.x1 <= 1.0
            && self.x2 >= 0.0
            && self.x2 <= 1.0
            && self.y1.is_finite()
            && self.y2.is_finite()
    }

    /// Eases a linear progress `t` in `0.0..=1.0` (clamped): returns the curve's y at x = `t`.
    #[must_use]
    pub fn ease(&self, t: f32) -> f32 {
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
        let x = |u: f32| bezier(self.x1, self.x2, u);
        // Newton-Raphson converges in a few steps for well-behaved curves.
        let mut u = t;
        for _ in 0..8 {
            let error = x(u) - t;
            if error.abs() < 1e-6 {
                return bezier(self.y1, self.y2, u);
            }
            let slope = bezier_slope(self.x1, self.x2, u);
            if slope.abs() < 1e-6 {
                break;
            }
            u -= error / slope;
        }
        // Bisection always converges: x(u) is monotonic for valid curves.
        let (mut low, mut high) = (0.0_f32, 1.0_f32);
        u = t;
        for _ in 0..32 {
            if x(u) < t {
                low = u;
            } else {
                high = u;
            }
            u = f32::midpoint(low, high);
        }
        bezier(self.y1, self.y2, u)
    }
}

/// One coordinate of a cubic Bézier from 0 to 1 with control coordinates `p1` and `p2`.
fn bezier(p1: f32, p2: f32, u: f32) -> f32 {
    let c = 3.0 * p1;
    let b = 3.0f32.mul_add(p2 - p1, -c);
    let a = 1.0 - c - b;
    a.mul_add(u, b).mul_add(u, c) * u
}

fn bezier_slope(p1: f32, p2: f32, u: f32) -> f32 {
    let c = 3.0 * p1;
    let b = 3.0f32.mul_add(p2 - p1, -c);
    let a = 1.0 - c - b;
    (3.0 * a).mul_add(u, 2.0 * b).mul_add(u, c)
}

impl fmt::Display for CubicBezier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cubic-bezier({}, {}, {}, {})",
            self.x1, self.y1, self.x2, self.y2
        )
    }
}

/// A font weight from 1 (thinnest) to 1000 (heaviest): 400 is regular, 700 bold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontWeight(u16);

impl FontWeight {
    /// Regular weight (400).
    pub const REGULAR: Self = Self(400);
    /// Bold weight (700).
    pub const BOLD: Self = Self(700);

    /// Creates a weight; `None` outside `1..=1000`.
    pub const fn new(weight: u16) -> Option<Self> {
        if weight >= 1 && weight <= 1000 {
            Some(Self(weight))
        } else {
            None
        }
    }

    /// The numeric weight.
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl fmt::Display for FontWeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A color as written in a token table (see [`define`](crate::define) for the notations).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorLiteral {
    /// Gamma-encoded sRGB with straight alpha.
    Srgb(Color),
    /// OKLCH, converted to sRGB when the design system loads (trigonometry is not available
    /// while compiling), with gamut mapping by chroma reduction.
    Oklch(Oklch),
}

impl ColorLiteral {
    /// `true` when every channel is in range: sRGB channels and opacity in `0.0..=1.0`; OKLCH
    /// lightness and opacity in `0.0..=1.0`, a finite positive chroma and a finite hue.
    pub const fn is_valid(&self) -> bool {
        match *self {
            Self::Srgb(c) => unit(c.r) && unit(c.g) && unit(c.b) && unit(c.a),
            Self::Oklch(c) => {
                unit(c.l) && unit(c.alpha) && c.c >= 0.0 && c.c.is_finite() && c.h.is_finite()
            }
        }
    }

    /// The opacity, `0.0` (transparent) to `1.0` (opaque).
    pub const fn alpha(&self) -> f32 {
        match *self {
            Self::Srgb(color) => color.a,
            Self::Oklch(color) => color.alpha,
        }
    }

    /// The color in gamma-encoded sRGB.
    #[must_use]
    pub fn to_srgb(self) -> Color {
        match self {
            Self::Srgb(color) => color,
            Self::Oklch(color) => color.to_color(),
        }
    }
}

const fn unit(value: f32) -> bool {
    value >= 0.0 && value <= 1.0
}

/// A value written in a token table. Plain data, so token tables are `const`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Literal {
    /// A color.
    Color(ColorLiteral),
    /// Logical pixels.
    Dimension(f32),
    /// A unitless number.
    Number(f32),
    /// An integer.
    Integer(i32),
    /// A duration.
    Duration(Duration),
    /// An easing curve.
    CubicBezier(CubicBezier),
    /// A font weight.
    FontWeight(FontWeight),
    /// A font family name.
    FontFamily(&'static str),
}

impl Literal {
    /// The kind of the literal.
    pub const fn kind(&self) -> KindId {
        match self {
            Self::Color(_) => KindId::Color,
            Self::Dimension(_) => KindId::Dimension,
            Self::Number(_) => KindId::Number,
            Self::Integer(_) => KindId::Integer,
            Self::Duration(_) => KindId::Duration,
            Self::CubicBezier(_) => KindId::CubicBezier,
            Self::FontWeight(_) => KindId::FontWeight,
            Self::FontFamily(_) => KindId::FontFamily,
        }
    }

    /// The literal as a number for range checks (durations in milliseconds), if numeric.
    #[allow(
        clippy::cast_lossless,
        reason = "`From` conversions are not callable in a const fn; every cast is lossless"
    )]
    pub const fn as_number(&self) -> Option<f64> {
        match *self {
            Self::Dimension(v) | Self::Number(v) => Some(v as f64),
            Self::Integer(v) => Some(v as f64),
            Self::Duration(d) => Some(d.as_secs_f64() * 1000.0),
            Self::FontWeight(w) => Some(w.get() as f64),
            Self::Color(_) | Self::CubicBezier(_) | Self::FontFamily(_) => None,
        }
    }
}

/// A token value at run time.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// A color (gamma-encoded sRGB, straight alpha).
    Color(Color),
    /// Logical pixels.
    Dimension(f32),
    /// A unitless number.
    Number(f32),
    /// An integer.
    Integer(i32),
    /// A duration.
    Duration(Duration),
    /// An easing curve.
    CubicBezier(CubicBezier),
    /// A font weight.
    FontWeight(FontWeight),
    /// A font family name.
    FontFamily(Arc<str>),
}

impl Value {
    /// The kind of the value.
    pub fn kind(&self) -> KindId {
        match self {
            Self::Color(_) => KindId::Color,
            Self::Dimension(_) => KindId::Dimension,
            Self::Number(_) => KindId::Number,
            Self::Integer(_) => KindId::Integer,
            Self::Duration(_) => KindId::Duration,
            Self::CubicBezier(_) => KindId::CubicBezier,
            Self::FontWeight(_) => KindId::FontWeight,
            Self::FontFamily(_) => KindId::FontFamily,
        }
    }

    /// The value as a number for range checks (durations in milliseconds), if numeric.
    pub fn as_number(&self) -> Option<f64> {
        match *self {
            Self::Dimension(v) | Self::Number(v) => Some(f64::from(v)),
            Self::Integer(v) => Some(f64::from(v)),
            Self::Duration(d) => Some(d.as_secs_f64() * 1000.0),
            Self::FontWeight(w) => Some(f64::from(w.get())),
            Self::Color(_) | Self::CubicBezier(_) | Self::FontFamily(_) => None,
        }
    }

    /// Wraps a typed value.
    pub fn new<K: Kind>(value: K::Value) -> Self {
        K::into_value(value)
    }
}

impl From<Literal> for Value {
    fn from(literal: Literal) -> Self {
        match literal {
            Literal::Color(c) => Self::Color(c.to_srgb()),
            Literal::Dimension(v) => Self::Dimension(v),
            Literal::Number(v) => Self::Number(v),
            Literal::Integer(v) => Self::Integer(v),
            Literal::Duration(d) => Self::Duration(d),
            Literal::CubicBezier(b) => Self::CubicBezier(b),
            Literal::FontWeight(w) => Self::FontWeight(w),
            Literal::FontFamily(name) => Self::FontFamily(Arc::from(name)),
        }
    }
}

impl fmt::Display for Value {
    /// Formats as `#3366ff`, `4px`, `1.5`, `120ms`, `cubic-bezier(...)`, `600` or `Inter`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Color(c) => c.fmt(f),
            Self::Dimension(v) => write!(f, "{v}px"),
            Self::Number(v) => v.fmt(f),
            Self::Integer(v) => v.fmt(f),
            Self::Duration(d) => write!(f, "{}ms", d.as_secs_f64() * 1000.0),
            Self::CubicBezier(b) => b.fmt(f),
            Self::FontWeight(w) => w.fmt(f),
            Self::FontFamily(name) => f.write_str(name),
        }
    }
}

/// How a token is defined in a token table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Def {
    /// The token holds this value.
    Literal(Literal),
    /// The token takes the value of another token.
    Alias(TokenId),
}

impl Def {
    /// An alias from a token of kind `K` and tier `T` to `target`, checked at compile time.
    ///
    /// The kinds must match and `T` must be allowed to alias the target's tier.
    #[allow(
        clippy::extra_unused_type_parameters,
        reason = "`T` is the tier of the token being defined: the bound checks the tier rule"
    )]
    pub const fn alias<K: Kind, T, U>(target: Token<K, U>) -> Self
    where
        T: MayAlias<U>,
        U: Tier,
    {
        Self::Alias(target.id())
    }
}

/// How a token is defined at run time: in a theme created at run time or an override.
#[derive(Clone, Debug, PartialEq)]
pub enum Definition {
    /// The token holds this value.
    Value(Value),
    /// The token takes the value of another token.
    Alias(TokenId),
}

impl Definition {
    /// The alias target, if this is an alias.
    pub fn alias_target(&self) -> Option<TokenId> {
        match self {
            Self::Alias(target) => Some(*target),
            Self::Value(_) => None,
        }
    }
}

impl From<Def> for Definition {
    fn from(def: Def) -> Self {
        match def {
            Def::Literal(literal) => Self::Value(literal.into()),
            Def::Alias(target) => Self::Alias(target),
        }
    }
}

/// The values a token accepts. Every layer (built-in themes, themes created at run time,
/// overrides, imports) is checked against it, and resolved values are kept inside it, so an
/// alias cannot bring in a value the token refuses.
///
/// - **Numeric tokens** (dimensions, numbers, integers, durations in milliseconds, font
///   weights): bounds, closed or open-ended ([`Range::at_least`], [`Range::at_most`],
///   [`Range::positive`]), and optionally a step values must be a multiple of
///   ([`Range::steps`]).
/// - **Colors**: the range bounds the alpha channel ([`Range::alpha`], [`Range::opaque`]).
///
/// Ranges also tell editors how far sliders go.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Range {
    min: f64,
    max: f64,
    step: f64,
    /// Values must be multiples of `step` from `min`.
    snap: bool,
}

impl Range {
    /// Values from `min` to `max` (inclusive); `step` is a suggestion for editors, `0.0` for
    /// a continuous range.
    pub const fn new(min: f64, max: f64, step: f64) -> Self {
        Self {
            min,
            max,
            step,
            snap: false,
        }
    }

    /// Values from `min` up.
    pub const fn at_least(min: f64) -> Self {
        Self::new(min, f64::INFINITY, 0.0)
    }

    /// Values up to `max`.
    pub const fn at_most(max: f64) -> Self {
        Self::new(f64::NEG_INFINITY, max, 0.0)
    }

    /// Values above zero: sizes and factors that must not vanish.
    pub const fn positive() -> Self {
        Self::new(f64::MIN_POSITIVE, f64::INFINITY, 0.0)
    }

    /// Multiples of `step` from `min` to `max`: values snap to them (whole pixels, weights by
    /// hundreds).
    pub const fn steps(min: f64, max: f64, step: f64) -> Self {
        Self {
            min,
            max,
            step,
            snap: true,
        }
    }

    /// For a color: an alpha from `min` to `max`.
    pub const fn alpha(min: f64, max: f64) -> Self {
        Self::new(min, max, 0.0)
    }

    /// For a color: fully opaque only.
    pub const fn opaque() -> Self {
        Self::alpha(1.0, 1.0)
    }

    /// Lowest valid value (negative infinity when open-ended).
    pub const fn min(&self) -> f64 {
        self.min
    }

    /// Highest valid value (infinity when open-ended).
    pub const fn max(&self) -> f64 {
        self.max
    }

    /// Editing step (`0.0` when continuous); values snap to it when [`Range::is_snapped`].
    pub const fn step(&self) -> f64 {
        self.step
    }

    /// `true` for [`Range::positive`]: every value above zero, zero excluded.
    pub const fn excludes_zero(&self) -> bool {
        self.min > 0.0 && self.min <= f64::MIN_POSITIVE
    }

    /// `true` when values must be multiples of the step from the minimum.
    pub const fn is_snapped(&self) -> bool {
        self.snap
    }

    /// `true` when `value` lies in the range (both ends included) and, for snapped ranges, on
    /// a step.
    pub const fn contains(&self, value: f64) -> bool {
        if !(value >= self.min && value <= self.max) {
            return false;
        }
        if !self.snap {
            return true;
        }
        let steps = (value - self.min) / self.step;
        (steps - steps.round()).abs() < 1e-6
    }

    /// `value` brought into the range: clamped to the bounds, then to the nearest step of a
    /// snapped range.
    pub fn limit(&self, value: f64) -> f64 {
        let clamped = value.clamp(self.min, self.max);
        if !self.snap {
            return clamped;
        }
        let snapped = self.min + ((clamped - self.min) / self.step).round() * self.step;
        if snapped > self.max {
            snapped - self.step
        } else {
            snapped
        }
    }

    /// `true` when the bounds are ordered and not both infinite on the same side, and the step
    /// is finite and not negative (positive, with a finite minimum, when values snap to it).
    pub const fn is_valid(&self) -> bool {
        !self.min.is_nan()
            && !self.max.is_nan()
            && self.min <= self.max
            && self.min < f64::INFINITY
            && self.max > f64::NEG_INFINITY
            && self.step.is_finite()
            && self.step >= 0.0
            && (!self.snap || (self.step > 0.0 && self.min.is_finite()))
    }
}

impl fmt::Display for Range {
    /// `0 to 4`, `at least 0`, `above 0`, `at most 1`, `0 to 100 in steps of 5`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.min.is_finite(), self.max.is_finite()) {
            (true, false) if self.excludes_zero() => f.write_str("above 0")?,
            (true, true) => write!(f, "{} to {}", self.min, self.max)?,
            (true, false) => write!(f, "at least {}", self.min)?,
            (false, true) => write!(f, "at most {}", self.max)?,
            (false, false) => f.write_str("any value")?,
        }
        if self.snap {
            write!(f, " in steps of {}", self.step)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cubic_bezier_linear_is_identity() {
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            assert!((CubicBezier::LINEAR.ease(t) - t).abs() < 1e-4, "t = {t}");
        }
    }

    #[test]
    fn cubic_bezier_hits_endpoints_and_is_symmetric() {
        let ease_in_out = CubicBezier::new(0.42, 0.0, 0.58, 1.0);
        assert!(ease_in_out.ease(0.0).abs() < 1e-5);
        assert!((ease_in_out.ease(1.0) - 1.0).abs() < 1e-5);
        assert!((ease_in_out.ease(0.5) - 0.5).abs() < 1e-4);
        let early = ease_in_out.ease(0.25);
        let late = ease_in_out.ease(0.75);
        assert!((early + late - 1.0).abs() < 1e-4);
        assert!(early < 0.25, "ease-in-out starts slowly");
    }

    #[test]
    fn cubic_bezier_overshoots_when_asked() {
        let spring = CubicBezier::new(0.34, 1.56, 0.64, 1.0);
        assert!((0..100).any(|i| spring.ease(i as f32 / 100.0) > 1.0));
        assert!(spring.is_valid());
        assert!(!CubicBezier::new(1.5, 0.0, 0.5, 1.0).is_valid());
    }

    #[test]
    fn font_weight_is_bounded() {
        assert_eq!(FontWeight::new(0), None);
        assert_eq!(FontWeight::new(1001), None);
        assert_eq!(FontWeight::new(600).map(FontWeight::get), Some(600));
    }

    #[test]
    fn value_displays_like_css() {
        assert_eq!(Value::Dimension(4.0).to_string(), "4px");
        assert_eq!(
            Value::Duration(Duration::from_millis(150)).to_string(),
            "150ms"
        );
        assert_eq!(
            Value::Color(Color::from_hex(0x33_66_FF)).to_string(),
            "#3366ff"
        );
        assert_eq!(
            Value::from(Literal::FontFamily("Inter")).to_string(),
            "Inter"
        );
    }

    #[test]
    fn numbers_are_extracted_for_range_checks() {
        assert_eq!(
            Literal::Duration(Duration::from_millis(250)).as_number(),
            Some(250.0)
        );
        assert_eq!(Value::Integer(-3).as_number(), Some(-3.0));
        assert_eq!(Value::Color(Color::WHITE).as_number(), None);
        assert!(Range::new(0.0, 1.0, 0.0).contains(1.0));
        assert!(!Range::new(1.0, 0.0, 0.0).is_valid());
    }

    #[test]
    fn ranges_can_be_open_ended_positive_or_stepped() {
        let at_least = Range::at_least(0.0);
        assert!(at_least.contains(0.0) && at_least.contains(1e9) && !at_least.contains(-0.1));
        assert_eq!(at_least.to_string(), "at least 0");
        let positive = Range::positive();
        assert!(!positive.contains(0.0) && positive.contains(0.5));
        assert_eq!(positive.to_string(), "above 0");
        assert_eq!(Range::at_most(1.0).to_string(), "at most 1");
        let steps = Range::steps(100.0, 900.0, 100.0);
        assert!(steps.contains(400.0) && !steps.contains(450.0) && !steps.contains(1000.0));
        assert_eq!(steps.to_string(), "100 to 900 in steps of 100");
        assert!(
            [at_least, positive, steps, Range::opaque()]
                .iter()
                .all(Range::is_valid)
        );
        assert!(!Range::new(2.0, 1.0, 0.0).is_valid());
        assert!(!Range::at_least(f64::INFINITY).is_valid());
        assert!(!Range::steps(f64::NEG_INFINITY, 1.0, 1.0).is_valid());
        assert!(!Range::steps(0.0, 1.0, 0.0).is_valid());
    }

    #[test]
    fn values_are_limited_into_their_range() {
        let range = Range::new(0.0, 4.0, 0.5);
        assert_eq!(range.limit(-1.0), 0.0);
        assert_eq!(range.limit(9.0), 4.0);
        assert_eq!(range.limit(1.3), 1.3);
        let steps = Range::steps(0.0, 10.0, 4.0);
        assert_eq!(steps.limit(5.0), 4.0);
        assert_eq!(steps.limit(7.0), 8.0);
        // The nearest step above the maximum gives way to the one below.
        assert_eq!(steps.limit(9.9), 8.0);
        assert_eq!(Range::positive().limit(-3.0), f64::MIN_POSITIVE);
    }
}
