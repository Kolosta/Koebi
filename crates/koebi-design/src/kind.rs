//! Value kinds: what a token holds.
//!
//! Every token has exactly one kind, carried by its Rust type: a `Token<kind::Color, _>` is
//! read as a [`koebi_core::Color`], can only alias another color token and can only be
//! overridden with a color.
//!
//! Kinds describe the *value domain* and its unit, like the types of the W3C Design Tokens
//! format; they do not describe the role of a token. A corner radius, a spacing and a border
//! width are all [`Dimension`]s: the role is in the token's name, and a radius may legitimately
//! alias a spacing step. ADR 0008 explains why Koebi uses functional kinds rather than raw
//! numbers or one type per role.
//!
//! | Kind | Rust value | Unit or domain |
//! |---|---|---|
//! | [`Color`] | [`koebi_core::Color`] | gamma-encoded sRGB, straight alpha |
//! | [`Dimension`] | `f32` | logical pixels |
//! | [`Number`] | `f32` | unitless: ratios, opacities, multiples of the font size, em |
//! | [`Integer`] | `i32` | counts, stacking order |
//! | [`Duration`] | [`core::time::Duration`] | time |
//! | [`CubicBezier`] | [`value::CubicBezier`] | easing curve |
//! | [`FontWeight`] | [`value::FontWeight`] | 1 to 1000 |
//! | [`FontFamily`] | `Arc<str>` | font family name |
//! | [`Curve`] | [`value::Curve`] | easing curve of several segments |
//!
//! ```compile_fail
//! use koebi_design::{Token, kind, tier, value::Def};
//! const RADIUS: Token<kind::Dimension, tier::Primitive> = Token::from_index(0);
//! // A color token cannot alias a dimension token.
//! const BROKEN: Def = Def::alias::<kind::Color, tier::Semantic, _>(RADIUS);
//! ```

use core::fmt;
use std::sync::Arc;

use crate::define::Px;
use crate::value::{self, Def, Literal, Value};

mod sealed {
    pub trait Sealed {}
}

/// A value kind. Implemented by the marker types of this module only.
pub trait Kind: sealed::Sealed + 'static {
    /// The Rust type of resolved values.
    type Value: Clone + PartialEq + fmt::Debug;

    /// Run-time identifier of the kind.
    const ID: KindId;

    /// Wraps a typed value.
    fn into_value(value: Self::Value) -> Value;

    /// Unwraps a value of this kind; `None` for another kind.
    fn from_value(value: &Value) -> Option<Self::Value>;

    /// The value returned for a token handle that does not belong to the schema in use.
    /// Colors fall back to magenta so that the mistake is visible.
    #[doc(hidden)]
    fn fallback() -> Self::Value;
}

/// Run-time identifier of a kind, for untyped code (persistence, import, tooling).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum KindId {
    /// See [`Color`].
    Color,
    /// See [`Dimension`].
    Dimension,
    /// See [`Number`].
    Number,
    /// See [`Integer`].
    Integer,
    /// See [`Duration`].
    Duration,
    /// See [`CubicBezier`].
    CubicBezier,
    /// See [`FontWeight`].
    FontWeight,
    /// See [`FontFamily`].
    FontFamily,
    /// See [`Curve`].
    Curve,
}

impl KindId {
    /// Every kind, in declaration order.
    pub const ALL: [Self; 9] = [
        Self::Color,
        Self::Dimension,
        Self::Number,
        Self::Integer,
        Self::Duration,
        Self::CubicBezier,
        Self::FontWeight,
        Self::FontFamily,
        Self::Curve,
    ];

    /// The name of the kind, in the camel case of the Design Tokens format (`color`,
    /// `dimension`, `cubicBezier`, ...). `integer` and `curve` are Koebi's own.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Color => "color",
            Self::Dimension => "dimension",
            Self::Number => "number",
            Self::Integer => "integer",
            Self::Duration => "duration",
            Self::CubicBezier => "cubicBezier",
            Self::FontWeight => "fontWeight",
            Self::FontFamily => "fontFamily",
            Self::Curve => "curve",
        }
    }

    /// Parses [`Self::name`].
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == name)
    }

    /// `true` for kinds whose values are numbers, compared against a
    /// [`Range`](crate::value::Range).
    pub const fn is_numeric(self) -> bool {
        matches!(
            self,
            Self::Dimension | Self::Number | Self::Integer | Self::Duration | Self::FontWeight
        )
    }

    /// `true` for kinds a [`Range`](crate::value::Range) can constrain: numeric kinds, and
    /// colors through their alpha.
    pub const fn takes_range(self) -> bool {
        self.is_numeric() || matches!(self, Self::Color)
    }
}

impl fmt::Display for KindId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Kind of color tokens: [`koebi_core::Color`], gamma-encoded sRGB with straight alpha.
#[derive(Debug)]
pub enum Color {}

/// Kind of length tokens, in logical pixels: spacing, sizes, radii, stroke widths.
#[derive(Debug)]
pub enum Dimension {}

/// Kind of unitless numbers: opacities, line heights, letter spacing in em, angles.
#[derive(Debug)]
pub enum Number {}

/// Kind of integers: counts and stacking order.
#[derive(Debug)]
pub enum Integer {}

/// Kind of durations: animation timings.
#[derive(Debug)]
pub enum Duration {}

/// Kind of easing curves.
#[derive(Debug)]
pub enum CubicBezier {}

/// Kind of font weights.
#[derive(Debug)]
pub enum FontWeight {}

/// Kind of font family names.
#[derive(Debug)]
pub enum FontFamily {}

/// Kind of easing curves of several segments.
#[derive(Debug)]
pub enum Curve {}

macro_rules! impl_kind {
    ($marker:ident, $value:ty, $fallback:expr) => {
        impl sealed::Sealed for $marker {}

        impl Kind for $marker {
            type Value = $value;
            const ID: KindId = KindId::$marker;

            fn into_value(value: Self::Value) -> Value {
                Value::$marker(value)
            }

            fn from_value(value: &Value) -> Option<Self::Value> {
                match value {
                    Value::$marker(inner) => Some(<$value as Clone>::clone(inner)),
                    _ => None,
                }
            }

            fn fallback() -> Self::Value {
                $fallback
            }
        }
    };
}

impl_kind!(
    Color,
    koebi_core::Color,
    koebi_core::Color::rgb(1.0, 0.0, 1.0)
);
impl_kind!(Dimension, f32, 0.0);
impl_kind!(Number, f32, 0.0);
impl_kind!(Integer, i32, 0);
impl_kind!(Duration, core::time::Duration, core::time::Duration::ZERO);
impl_kind!(CubicBezier, value::CubicBezier, value::CubicBezier::LINEAR);
impl_kind!(FontWeight, value::FontWeight, value::FontWeight::REGULAR);
impl_kind!(FontFamily, Arc<str>, Arc::from(""));
impl_kind!(Curve, value::Curve, value::Curve::LINEAR);

// Literal constructors used by `koebi_design::tokens!` for primitive tokens. Each takes the
// value type of its kind, so a value of the wrong kind does not compile.

impl Color {
    #[doc(hidden)]
    pub const fn literal(value: value::ColorLiteral) -> Def {
        Def::Literal(Literal::Color(value))
    }
}

impl Dimension {
    #[doc(hidden)]
    pub const fn literal(value: Px) -> Def {
        Def::Literal(Literal::Dimension(value.0))
    }
}

impl Number {
    #[doc(hidden)]
    pub const fn literal(value: f32) -> Def {
        Def::Literal(Literal::Number(value))
    }
}

impl Integer {
    #[doc(hidden)]
    pub const fn literal(value: i32) -> Def {
        Def::Literal(Literal::Integer(value))
    }
}

impl Duration {
    #[doc(hidden)]
    pub const fn literal(value: core::time::Duration) -> Def {
        Def::Literal(Literal::Duration(value))
    }
}

impl CubicBezier {
    #[doc(hidden)]
    pub const fn literal(value: value::CubicBezier) -> Def {
        Def::Literal(Literal::CubicBezier(value))
    }
}

impl FontWeight {
    #[doc(hidden)]
    pub const fn literal(value: value::FontWeight) -> Def {
        Def::Literal(Literal::FontWeight(value))
    }
}

impl FontFamily {
    #[doc(hidden)]
    pub const fn literal(value: &'static str) -> Def {
        Def::Literal(Literal::FontFamily(value))
    }
}

impl Curve {
    #[doc(hidden)]
    pub const fn literal(value: value::Curve) -> Def {
        Def::Literal(Literal::Curve(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_names_round_trip() {
        for kind in KindId::ALL {
            assert_eq!(KindId::from_name(kind.name()), Some(kind));
        }
    }

    #[test]
    fn values_round_trip_through_their_kind() {
        let value = Value::new::<Dimension>(4.0);
        assert_eq!(Dimension::from_value(&value), Some(4.0));
        assert_eq!(Number::from_value(&value), None);
        assert_eq!(value.kind(), Dimension::ID);
        let family = Value::new::<FontFamily>(Arc::from("Inter"));
        assert_eq!(FontFamily::from_value(&family).as_deref(), Some("Inter"));
    }

    #[test]
    fn literals_carry_their_kind() {
        let def = Color::literal(value::ColorLiteral::Srgb(koebi_core::Color::WHITE));
        assert!(matches!(def, Def::Literal(l) if l.kind() == KindId::Color));
        assert!(matches!(
            Dimension::literal(Px(2.0)),
            Def::Literal(Literal::Dimension(v)) if (v - 2.0).abs() < f32::EPSILON
        ));
    }
}
