//! Socket types and the values sockets hold, with the Rust types that stand for them.
//!
//! [`SocketType`] is a closed set. Each socket type has a Rust type implementing [`Socket`]:
//! values ([`SocketValue`]: `bool`, `i64`, `f64`, [`Vector`], [`LinearRgba`], [`Affine`],
//! [`Geometry`], [`Text`] and the enums) that an unlinked input holds, and the evaluated kinds
//! of data ([`Paint`], [`Layer`], [`Coverage`]) that only links carry. Typed socket handles
//! (`In<K, T>`, `Out<K, T>`, see [`crate::kind`]) are built on them, so that a node kind reading
//! a socket with the wrong type does not compile.

use std::sync::Arc;

use koebi_core::LinearRgba;
use koebi_geom::{Affine, FillRule, Path, Vector};

/// The type of a socket.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SocketType {
    /// True or false.
    Bool,
    /// A whole number.
    Int,
    /// A number.
    Float,
    /// A 2D vector.
    Vector,
    /// A linear color with straight alpha.
    Color,
    /// A 2D affine transform, in f64.
    Transform,
    /// Paths, in document units.
    Geometry,
    /// A fill or a stroke.
    Paint,
    /// Renderable content with its compositing.
    Layer,
    /// A mask.
    Coverage,
    /// Text.
    Text,
    /// One value of an enumeration.
    Enum(EnumKind),
    /// Anything: the input of an inspection node (Preview), which never feeds another.
    Any,
}

impl SocketType {
    /// Whether two types are the same, in a const context.
    pub const fn same(self, other: Self) -> bool {
        match (self, other) {
            (Self::Enum(a), Self::Enum(b)) => a as u8 == b as u8,
            (Self::Enum(_), _) | (_, Self::Enum(_)) => false,
            _ => self.rank() == other.rank(),
        }
    }

    /// A number per variant, for comparisons in const contexts.
    const fn rank(self) -> u8 {
        match self {
            Self::Bool => 0,
            Self::Int => 1,
            Self::Float => 2,
            Self::Vector => 3,
            Self::Color => 4,
            Self::Transform => 5,
            Self::Geometry => 6,
            Self::Paint => 7,
            Self::Layer => 8,
            Self::Coverage => 9,
            Self::Text => 10,
            Self::Enum(_) => 11,
            Self::Any => 12,
        }
    }

    /// What a link from an output of this type to an input of type `to` does: carry the value,
    /// convert it, or nothing (the types do not connect).
    pub fn conversion(self, to: Self) -> Option<Conversion> {
        use SocketType::{Any, Bool, Color, Float, Int};
        if self == to || (to == Any && self != Any) {
            return Some(Conversion::Same);
        }
        match (self, to) {
            (Float, Color) => Some(Conversion::Gray),
            (Color, Float) => Some(Conversion::Luminance),
            (Int, Float) => Some(Conversion::IntToFloat),
            (Float, Int) => Some(Conversion::FloatToInt),
            (Bool, Float) => Some(Conversion::BoolToFloat),
            _ => None,
        }
    }
}

/// How a link turns its source's value into its target's type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Conversion {
    /// The same type: nothing to do.
    Same,
    /// A number as an opaque gray of that linear level.
    Gray,
    /// A color as its relative luminance.
    Luminance,
    /// A whole number as a number.
    IntToFloat,
    /// A number rounded to the nearest whole number.
    FloatToInt,
    /// True as 1, false as 0.
    BoolToFloat,
}

impl Conversion {
    /// `value` converted, or `None` when it is not of the conversion's source type.
    pub fn apply(self, value: &Value) -> Option<Value> {
        Some(match (self, value) {
            (Self::Same, value) => value.clone(),
            (Self::Gray, Value::Float(level)) => {
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "a color channel needs no more than f32"
                )]
                let level = *level as f32;
                Value::Color(LinearRgba::new(level, level, level, 1.0))
            }
            (Self::Luminance, Value::Color(color)) => Value::Float(f64::from(
                0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b,
            )),
            #[allow(
                clippy::cast_precision_loss,
                reason = "a socket's whole numbers are small"
            )]
            (Self::IntToFloat, Value::Int(n)) => Value::Float(*n as f64),
            #[allow(
                clippy::cast_possible_truncation,
                reason = "rounded and saturated, as a conversion should"
            )]
            (Self::FloatToInt, Value::Float(x)) => Value::Int(x.round() as i64),
            (Self::BoolToFloat, Value::Bool(b)) => Value::Float(if *b { 1.0 } else { 0.0 }),
            _ => return None,
        })
    }
}

/// The enumerations a socket can hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EnumKind {
    /// [`BlendMode`].
    BlendMode,
    /// [`FillRule`].
    FillRule,
    /// [`StrokeAlign`].
    StrokeAlign,
    /// [`StrokeCap`].
    StrokeCap,
    /// [`StrokeJoin`].
    StrokeJoin,
    /// [`StrokeUnits`].
    StrokeUnits,
}

/// What a socket's number means, for its units in the interface. It never changes the type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Quantity {
    /// A plain number.
    #[default]
    Scalar,
    /// A length, in document units.
    Length,
    /// An angle, in radians.
    Angle,
    /// A share, shown as a percentage (1 is 100%).
    Percent,
}

/// A value an unlinked input holds, or a link carries.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// [`SocketType::Bool`].
    Bool(bool),
    /// [`SocketType::Int`].
    Int(i64),
    /// [`SocketType::Float`].
    Float(f64),
    /// [`SocketType::Vector`].
    Vector(Vector),
    /// [`SocketType::Color`].
    Color(LinearRgba),
    /// [`SocketType::Transform`].
    Transform(Affine),
    /// [`SocketType::Geometry`].
    Geometry(Geometry),
    /// [`SocketType::Text`].
    Text(Text),
    /// [`SocketType::Enum`]: the kind and the variant's index.
    Enum(EnumKind, u32),
    /// Nothing: an unlinked input of data that only links carry (paint, layer, coverage).
    None,
}

impl Value {
    /// The type of this value; `None` for [`Value::None`].
    pub const fn socket_type(&self) -> Option<SocketType> {
        Some(match self {
            Self::Bool(_) => SocketType::Bool,
            Self::Int(_) => SocketType::Int,
            Self::Float(_) => SocketType::Float,
            Self::Vector(_) => SocketType::Vector,
            Self::Color(_) => SocketType::Color,
            Self::Transform(_) => SocketType::Transform,
            Self::Geometry(_) => SocketType::Geometry,
            Self::Text(_) => SocketType::Text,
            Self::Enum(kind, _) => SocketType::Enum(*kind),
            Self::None => return None,
        })
    }
}

/// Geometry: paths in document units, shared rather than copied.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Geometry(pub Arc<Path>);

/// Text, shared rather than copied.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Text(pub Arc<str>);

/// The evaluated paint of a fill or a stroke: a type that links carry and evaluation (#261)
/// produces.
#[derive(Clone, Copy, Debug)]
pub struct Paint;

/// Evaluated renderable content: a type that links carry.
#[derive(Clone, Copy, Debug)]
pub struct Layer;

/// An evaluated mask: a type that links carry.
#[derive(Clone, Copy, Debug)]
pub struct Coverage;

/// A Rust type standing for a socket type.
pub trait Socket {
    /// Its socket type.
    const TYPE: SocketType;
}

/// A socket type with values an unlinked input can hold.
pub trait SocketValue: Socket + Clone {
    /// The value, when `value` is of this type.
    fn from_value(value: &Value) -> Option<Self>;
    /// This as a value.
    fn into_value(self) -> Value;
}

macro_rules! value_socket {
    ($type:ty, $variant:ident) => {
        impl Socket for $type {
            const TYPE: SocketType = SocketType::$variant;
        }

        impl SocketValue for $type {
            fn from_value(value: &Value) -> Option<Self> {
                match value {
                    Value::$variant(inner) => Some(inner.clone()),
                    _ => None,
                }
            }

            fn into_value(self) -> Value {
                Value::$variant(self)
            }
        }
    };
}

value_socket!(bool, Bool);
value_socket!(i64, Int);
value_socket!(f64, Float);
value_socket!(Vector, Vector);
value_socket!(LinearRgba, Color);
value_socket!(Affine, Transform);
value_socket!(Geometry, Geometry);
value_socket!(Text, Text);

impl Socket for Paint {
    const TYPE: SocketType = SocketType::Paint;
}

impl Socket for Layer {
    const TYPE: SocketType = SocketType::Layer;
}

impl Socket for Coverage {
    const TYPE: SocketType = SocketType::Coverage;
}

/// Any type: the input of an inspection node.
#[derive(Clone, Copy, Debug)]
pub struct Anything;

impl Socket for Anything {
    const TYPE: SocketType = SocketType::Any;
}

/// A socket type whose inputs always hold a value (never only links): read without an
/// `Option` by [`crate::NodeRef::get`].
pub trait Stored: SocketValue {}

impl Stored for bool {}
impl Stored for i64 {}
impl Stored for f64 {}
impl Stored for Vector {}
impl Stored for LinearRgba {}
impl Stored for Affine {}
impl Stored for BlendMode {}
impl Stored for FillRule {}
impl Stored for StrokeAlign {}
impl Stored for StrokeCap {}
impl Stored for StrokeJoin {}
impl Stored for StrokeUnits {}

/// An enumeration a socket can hold.
pub trait SocketEnum: Copy + Sized + 'static {
    /// Which one.
    const KIND: EnumKind;
    /// Every variant, in index order.
    const ALL: &'static [Self];
    /// The variant's index.
    fn index(self) -> u32;
}

macro_rules! enum_socket {
    ($type:ty, $kind:ident) => {
        impl Socket for $type {
            const TYPE: SocketType = SocketType::Enum(EnumKind::$kind);
        }

        impl SocketValue for $type {
            fn from_value(value: &Value) -> Option<Self> {
                match value {
                    Value::Enum(EnumKind::$kind, index) => {
                        <$type as SocketEnum>::ALL.get(*index as usize).copied()
                    }
                    _ => None,
                }
            }

            fn into_value(self) -> Value {
                Value::Enum(EnumKind::$kind, SocketEnum::index(self))
            }
        }
    };
}

/// How a layer composites onto what lies below it: W3C Compositing Level 1, and Erase.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BlendMode {
    /// Source over.
    #[default]
    Normal,
    /// Separable modes.
    Multiply,
    /// Separable.
    Screen,
    /// Separable.
    Overlay,
    /// Separable.
    Darken,
    /// Separable.
    Lighten,
    /// Separable.
    ColorDodge,
    /// Separable.
    ColorBurn,
    /// Separable.
    HardLight,
    /// Separable.
    SoftLight,
    /// Separable.
    Difference,
    /// Separable.
    Exclusion,
    /// Non-separable: the source's hue.
    Hue,
    /// Non-separable: the source's saturation.
    Saturation,
    /// Non-separable: the source's hue and saturation.
    Color,
    /// Non-separable: the source's luminosity.
    Luminosity,
    /// Removes the backdrop where the source lies (destination-out).
    Erase,
}

impl SocketEnum for BlendMode {
    const KIND: EnumKind = EnumKind::BlendMode;
    const ALL: &'static [Self] = &[
        Self::Normal,
        Self::Multiply,
        Self::Screen,
        Self::Overlay,
        Self::Darken,
        Self::Lighten,
        Self::ColorDodge,
        Self::ColorBurn,
        Self::HardLight,
        Self::SoftLight,
        Self::Difference,
        Self::Exclusion,
        Self::Hue,
        Self::Saturation,
        Self::Color,
        Self::Luminosity,
        Self::Erase,
    ];
    fn index(self) -> u32 {
        self as u32
    }
}
enum_socket!(BlendMode, BlendMode);

impl SocketEnum for FillRule {
    const KIND: EnumKind = EnumKind::FillRule;
    const ALL: &'static [Self] = &[Self::NonZero, Self::EvenOdd];
    fn index(self) -> u32 {
        match self {
            Self::NonZero => 0,
            Self::EvenOdd => 1,
        }
    }
}
enum_socket!(FillRule, FillRule);

/// The side of the path a stroke lies on (see `koebi_geom::Align`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StrokeAlign {
    /// Across the path.
    #[default]
    Center,
    /// Toward the inside.
    Inside,
    /// Toward the outside.
    Outside,
    /// Left of the walk.
    Left,
    /// Right of the walk.
    Right,
}

impl SocketEnum for StrokeAlign {
    const KIND: EnumKind = EnumKind::StrokeAlign;
    const ALL: &'static [Self] = &[
        Self::Center,
        Self::Inside,
        Self::Outside,
        Self::Left,
        Self::Right,
    ];
    fn index(self) -> u32 {
        self as u32
    }
}
enum_socket!(StrokeAlign, StrokeAlign);

/// The ends of a stroke (see `koebi_geom::Cap`; the tilt and the taper's length are sockets of
/// their own).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StrokeCap {
    /// Cut across.
    #[default]
    Butt,
    /// A half disc.
    Round,
    /// Extended by half the width.
    Square,
    /// A point.
    Taper,
}

impl SocketEnum for StrokeCap {
    const KIND: EnumKind = EnumKind::StrokeCap;
    const ALL: &'static [Self] = &[Self::Butt, Self::Round, Self::Square, Self::Taper];
    fn index(self) -> u32 {
        self as u32
    }
}
enum_socket!(StrokeCap, StrokeCap);

/// The corners of a stroke (see `koebi_geom::Join`; the miter limit is a socket of its own).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StrokeJoin {
    /// Edges extended until they meet, within the limit.
    #[default]
    Miter,
    /// An arc.
    Round,
    /// A straight cut.
    Bevel,
}

impl SocketEnum for StrokeJoin {
    const KIND: EnumKind = EnumKind::StrokeJoin;
    const ALL: &'static [Self] = &[Self::Miter, Self::Round, Self::Bevel];
    fn index(self) -> u32 {
        self as u32
    }
}
enum_socket!(StrokeJoin, StrokeJoin);

/// What a stroke's lengths count (its width, its offset in units, its taper, its dashes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StrokeUnits {
    /// Document units: the stroke scales with the zoom, as the shape does.
    #[default]
    Document,
    /// The pixels of the view showing it: the stroke keeps its width on screen at any zoom, as
    /// a hairline does. The canvas outlines it for each view.
    Viewport,
}

impl SocketEnum for StrokeUnits {
    const KIND: EnumKind = EnumKind::StrokeUnits;
    const ALL: &'static [Self] = &[Self::Document, Self::Viewport];
    fn index(self) -> u32 {
        self as u32
    }
}
enum_socket!(StrokeUnits, StrokeUnits);

#[cfg(test)]
mod tests {
    use super::*;

    const TYPES: [SocketType; 14] = [
        SocketType::Bool,
        SocketType::Int,
        SocketType::Float,
        SocketType::Vector,
        SocketType::Color,
        SocketType::Transform,
        SocketType::Geometry,
        SocketType::Paint,
        SocketType::Layer,
        SocketType::Coverage,
        SocketType::Text,
        SocketType::Enum(EnumKind::BlendMode),
        SocketType::Enum(EnumKind::StrokeCap),
        SocketType::Enum(EnumKind::StrokeUnits),
    ];

    #[test]
    fn every_type_pair_connects_by_identity_or_an_explicit_conversion_only() {
        use SocketType::{Bool, Color, Float, Int};
        let converts = [
            (Float, Color),
            (Color, Float),
            (Int, Float),
            (Float, Int),
            (Bool, Float),
        ];
        for from in TYPES {
            for to in TYPES {
                let found = from.conversion(to);
                if from == to {
                    assert_eq!(found, Some(Conversion::Same), "{from:?} -> {to:?}");
                } else if converts.contains(&(from, to)) {
                    assert!(matches!(found, Some(c) if c != Conversion::Same));
                } else {
                    assert_eq!(found, None, "{from:?} -> {to:?}");
                }
                assert_eq!(from.same(to), from == to);
            }
            // Anything feeds an inspection input, which feeds nothing.
            assert_eq!(from.conversion(SocketType::Any), Some(Conversion::Same));
            assert_eq!(SocketType::Any.conversion(from), None);
        }
    }

    #[test]
    fn conversions_turn_values() {
        let gray = Conversion::Gray.apply(&Value::Float(0.25));
        assert_eq!(
            gray,
            Some(Value::Color(LinearRgba::new(0.25, 0.25, 0.25, 1.0)))
        );
        let white = Value::Color(LinearRgba::new(1.0, 1.0, 1.0, 1.0));
        let luminance = Conversion::Luminance.apply(&white);
        assert!(matches!(luminance, Some(Value::Float(l)) if (l - 1.0).abs() < 1e-6));
        assert_eq!(
            Conversion::IntToFloat.apply(&Value::Int(3)),
            Some(Value::Float(3.0))
        );
        assert_eq!(
            Conversion::FloatToInt.apply(&Value::Float(2.6)),
            Some(Value::Int(3))
        );
        assert_eq!(
            Conversion::BoolToFloat.apply(&Value::Bool(true)),
            Some(Value::Float(1.0))
        );
        assert_eq!(Conversion::Gray.apply(&Value::Bool(true)), None);
    }

    #[test]
    fn enums_round_trip_through_values() {
        for &mode in BlendMode::ALL {
            assert_eq!(BlendMode::from_value(&mode.into_value()), Some(mode));
        }
        assert_eq!(BlendMode::ALL.len(), 17);
        for &rule in FillRule::ALL {
            assert_eq!(FillRule::from_value(&rule.into_value()), Some(rule));
        }
        // A value of another enumeration is not this one's.
        assert_eq!(BlendMode::from_value(&StrokeCap::Round.into_value()), None);
    }
}
