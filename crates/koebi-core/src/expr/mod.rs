//! [`evaluate`]: arithmetic with units, for number fields.
//!
//! - **Numbers**: `12`, `1.5`, `.5`, `1e-3`, and `1,5` with a decimal comma outside the
//!   arguments of a function.
//! - **Operators**: `+`, `-`, `*`, `/`, `^` (right-associative), signs and parentheses; `2pi`
//!   and `2(3+4)` multiply.
//! - **Functions**: `sqrt`, `abs`, `floor`, `ceil`, `round`, `min`, `max`, `mod`, `exp`, `ln`,
//!   `log`, `log2`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`; **constants**: `pi`,
//!   `tau`, `e`.
//! - **Units** ([`UNITS`]) follow numbers and parentheses: `10mm`, `(1+2)cm`, `90deg`; units
//!   side by side add up: `1m 20cm`, `1min 30s`. The value converts to the unit of the field
//!   ([`Context::unit`]); numbers without a unit count in it, so `10mm + 5` adds five of the
//!   field's unit. Trigonometric functions read numbers without a unit as radians; `floor`,
//!   `ceil` and `round` round in the field's unit.
//!
//! Errors say what is wrong and where ([`ExprError`]); evaluating allocates nothing.

mod parse;
pub mod units;

#[cfg(test)]
mod tests;

use core::fmt;

pub use units::{DEGREE, Dimension, MILLIMETER, PIXEL, RADIAN, SECOND, UNITS, Unit, unit};

/// How to evaluate an expression: the unit of the field it is typed in, and how many pixels
/// make an inch (to convert pixels to physical lengths).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Context<'u> {
    /// The unit of the field: values come back in it, and numbers without a unit count in it.
    /// `None` for plain numbers.
    pub unit: Option<&'u Unit>,
    /// Pixels in an inch.
    pub pixels_per_inch: f64,
}

impl Default for Context<'_> {
    /// Plain numbers, at 96 pixels per inch.
    fn default() -> Self {
        Self {
            unit: None,
            pixels_per_inch: 96.0,
        }
    }
}

impl<'u> Context<'u> {
    /// A field of values in `unit`, at 96 pixels per inch.
    pub fn in_unit(unit: &'u Unit) -> Self {
        Self {
            unit: Some(unit),
            ..Self::default()
        }
    }
}

/// What is wrong with an expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// There is nothing to evaluate.
    Empty,
    /// A character no expression holds.
    UnexpectedCharacter,
    /// Something in the wrong place: an operator missing its operand, a closing parenthesis
    /// without its opening one, two numbers side by side.
    Unexpected,
    /// The expression stops in the middle.
    UnexpectedEnd,
    /// A parenthesis is never closed.
    UnclosedParenthesis,
    /// A name that is neither a function, a constant nor a unit.
    UnknownName,
    /// A function given the wrong number of arguments.
    WrongArgumentCount,
    /// Units that do not go together: a length plus an angle, an angle in a field of lengths,
    /// a unit in a field of plain numbers.
    IncompatibleUnits,
    /// A number that cannot be read.
    InvalidNumber,
    /// No finite result: a division by zero, the square root of a negative number.
    NotFinite,
}

/// Why an expression has no value, and where: the bytes `start..end` of the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExprError {
    /// What is wrong.
    pub kind: ErrorKind,
    /// Where it starts, in bytes.
    pub start: usize,
    /// Where it ends, in bytes.
    pub end: usize,
}

impl fmt::Display for ExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self.kind {
            ErrorKind::Empty => "nothing to evaluate",
            ErrorKind::UnexpectedCharacter => "unexpected character",
            ErrorKind::Unexpected => "unexpected here",
            ErrorKind::UnexpectedEnd => "the expression is incomplete",
            ErrorKind::UnclosedParenthesis => "a parenthesis is not closed",
            ErrorKind::UnknownName => "unknown name",
            ErrorKind::WrongArgumentCount => "wrong number of arguments",
            ErrorKind::IncompatibleUnits => "incompatible units",
            ErrorKind::InvalidNumber => "invalid number",
            ErrorKind::NotFinite => "no finite result",
        };
        write!(f, "{message} at {}..{}", self.start, self.end)
    }
}

impl std::error::Error for ExprError {}

/// The powers of each [`Dimension`] of a quantity: `[1, 0, 0]` is a length, all zeros a plain
/// number.
pub(crate) type Dimensions = [i8; 3];

/// A value in the base units of its dimensions (meters, radians, seconds).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Quantity {
    pub(crate) value: f64,
    pub(crate) dimensions: Dimensions,
}

impl Quantity {
    pub(crate) const fn plain(value: f64) -> Self {
        Self {
            value,
            dimensions: [0; 3],
        }
    }

    pub(crate) fn is_plain(&self) -> bool {
        self.dimensions == [0; 3]
    }
}

/// The dimensions of `unit`.
pub(crate) fn dimensions_of(unit: &Unit) -> Dimensions {
    let mut dimensions = [0; 3];
    dimensions[unit.dimension.index()] = 1;
    dimensions
}

/// Evaluates `text` in `context`: the value in the field's unit.
///
/// # Errors
///
/// When `text` is not a valid expression, or its units do not go with the field's
/// ([`ErrorKind`]).
pub fn evaluate(text: &str, context: &Context<'_>) -> Result<f64, ExprError> {
    let whole = ExprError {
        kind: ErrorKind::IncompatibleUnits,
        start: 0,
        end: text.len(),
    };
    let quantity = parse::Parser::new(text, context).parse()?;
    let value = match context.unit {
        _ if quantity.is_plain() => quantity.value,
        Some(unit) if quantity.dimensions == dimensions_of(unit) => {
            quantity.value / unit.base(context.pixels_per_inch)
        }
        _ => return Err(whole),
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ExprError {
            kind: ErrorKind::NotFinite,
            ..whole
        })
    }
}
