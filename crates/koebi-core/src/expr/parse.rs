//! The lexer and the recursive-descent parser of expressions, which evaluates as it parses.
//!
//! ```text
//! sum      := product (("+" | "-") product)*
//! product  := unary (("*" | "/") unary | implicit unary)*     implicit: "(" or a name
//! unary    := ("-" | "+") unary | power
//! power    := quantity ("^" unary)?
//! quantity := primary (unit ("^" unary)? (number unit ("^" unary)?)*)?
//! primary  := number | "(" sum ")" | constant | function "(" (sum ("," sum)*)? ")"
//! ```

use super::units::{self, Unit};
use super::{Context, Dimensions, ErrorKind, ExprError, Quantity, dimensions_of};

/// What a token is.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind<'t> {
    Number(f64),
    Name(&'t str),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Open,
    Close,
    Comma,
    End,
}

/// A token and where it is in the text.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Token<'t> {
    kind: Kind<'t>,
    start: usize,
    end: usize,
    /// Whitespace right before it.
    spaced: bool,
}

impl Token<'_> {
    fn error(&self, kind: ErrorKind) -> ExprError {
        ExprError {
            kind,
            start: self.start,
            end: self.end,
        }
    }
}

/// The most arguments a function takes.
const MAX_ARGUMENTS: usize = 8;

/// Parses and evaluates one expression.
pub(super) struct Parser<'t, 'u> {
    text: &'t str,
    /// Where the lexer reads next.
    position: usize,
    current: Token<'t>,
    context: Context<'u>,
    /// How deep in function arguments the parser is: commas separate arguments there, and are
    /// decimal commas elsewhere.
    arguments: u32,
}

impl<'t, 'u> Parser<'t, 'u> {
    pub(super) fn new(text: &'t str, context: &Context<'u>) -> Self {
        Self {
            text,
            position: 0,
            current: Token {
                kind: Kind::End,
                start: 0,
                end: 0,
                spaced: false,
            },
            context: *context,
            arguments: 0,
        }
    }

    /// The value of the whole text.
    pub(super) fn parse(mut self) -> Result<Quantity, ExprError> {
        self.advance()?;
        if self.current.kind == Kind::End {
            return Err(self.current.error(ErrorKind::Empty));
        }
        let value = self.sum()?;
        match self.current.kind {
            Kind::End => Ok(value),
            _ => Err(self.current.error(ErrorKind::Unexpected)),
        }
    }

    // ---- Lexer -------------------------------------------------------------------------

    fn advance(&mut self) -> Result<(), ExprError> {
        self.current = self.lex()?;
        Ok(())
    }

    fn lex(&mut self) -> Result<Token<'t>, ExprError> {
        let rest = &self.text[self.position..];
        let trimmed = rest.trim_start();
        let spaced = trimmed.len() < rest.len();
        let start = self.position + (rest.len() - trimmed.len());
        let token = |kind, end| Token {
            kind,
            start,
            end,
            spaced,
        };
        let Some(first) = trimmed.chars().next() else {
            self.position = start;
            return Ok(token(Kind::End, start));
        };
        let single = match first {
            '+' => Some(Kind::Plus),
            '-' | '\u{2212}' => Some(Kind::Minus),
            '*' | '\u{d7}' => Some(Kind::Star),
            '/' | '\u{f7}' => Some(Kind::Slash),
            '^' => Some(Kind::Caret),
            '(' => Some(Kind::Open),
            ')' => Some(Kind::Close),
            ',' => Some(Kind::Comma),
            '\u{b0}' => Some(Kind::Name("\u{b0}")),
            '"' => Some(Kind::Name("\"")),
            _ => None,
        };
        if let Some(kind) = single {
            self.position = start + first.len_utf8();
            return Ok(token(kind, self.position));
        }
        let starts_number = first.is_ascii_digit()
            || (first == '.' && trimmed[1..].starts_with(|c: char| c.is_ascii_digit()));
        if starts_number {
            let length = number_length(trimmed);
            self.position = start + length;
            let value: f64 = trimmed[..length]
                .parse()
                .map_err(|_| token(Kind::End, self.position).error(ErrorKind::InvalidNumber))?;
            return Ok(token(Kind::Number(value), self.position));
        }
        if first.is_alphabetic() || first == '_' {
            let length = trimmed
                .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                .unwrap_or(trimmed.len());
            self.position = start + length;
            return Ok(token(Kind::Name(&trimmed[..length]), self.position));
        }
        Err(ExprError {
            kind: ErrorKind::UnexpectedCharacter,
            start,
            end: start + first.len_utf8(),
        })
    }

    /// `true` when the next character after the current token, without space, is a digit.
    fn digit_follows(&self) -> bool {
        self.text[self.position..].starts_with(|c: char| c.is_ascii_digit())
    }

    /// `true` when an opening parenthesis follows the current token: a function call.
    fn call_follows(&self) -> bool {
        self.text[self.position..].trim_start().starts_with('(')
    }

    fn field(&self) -> Option<(Dimensions, f64)> {
        self.context
            .unit
            .map(|unit| (dimensions_of(unit), unit.base(self.context.pixels_per_inch)))
    }

    // ---- Grammar -----------------------------------------------------------------------

    fn sum(&mut self) -> Result<Quantity, ExprError> {
        let mut left = self.product()?;
        loop {
            let sign = match self.current.kind {
                Kind::Plus => 1.0,
                Kind::Minus => -1.0,
                _ => return Ok(left),
            };
            let operator = self.current;
            self.advance()?;
            let right = self.product()?;
            let (left_value, right_value) = self.align(left, right, operator)?;
            left = Quantity {
                value: left_value.value + sign * right_value.value,
                dimensions: left_value.dimensions,
            };
        }
    }

    /// `a` and `b` in the same dimensions: a plain number beside a quantity with units counts
    /// in the field's unit.
    fn align(
        &self,
        a: Quantity,
        b: Quantity,
        at: Token<'_>,
    ) -> Result<(Quantity, Quantity), ExprError> {
        if a.dimensions == b.dimensions {
            return Ok((a, b));
        }
        let in_field = |plain: Quantity, dimensions: Dimensions, base: f64| Quantity {
            value: plain.value * base,
            dimensions,
        };
        match self.field() {
            Some((dimensions, base)) if a.is_plain() && b.dimensions == dimensions => {
                Ok((in_field(a, dimensions, base), b))
            }
            Some((dimensions, base)) if b.is_plain() && a.dimensions == dimensions => {
                Ok((a, in_field(b, dimensions, base)))
            }
            _ => Err(at.error(ErrorKind::IncompatibleUnits)),
        }
    }

    fn product(&mut self) -> Result<Quantity, ExprError> {
        let mut left = self.unary()?;
        loop {
            let operator = self.current;
            let divide = match operator.kind {
                Kind::Star => false,
                Kind::Slash => true,
                // Implicit multiplication: `2(3+4)`, `2pi`, `2 sqrt(2)`.
                Kind::Open | Kind::Name(_) => {
                    let right = self.unary()?;
                    left = combine(left, right, 1, operator)?;
                    continue;
                }
                _ => return Ok(left),
            };
            self.advance()?;
            let right = self.unary()?;
            if divide && right.value == 0.0 {
                return Err(operator.error(ErrorKind::NotFinite));
            }
            left = if divide {
                combine(
                    left,
                    Quantity {
                        value: 1.0 / right.value,
                        dimensions: right.dimensions,
                    },
                    -1,
                    operator,
                )?
            } else {
                combine(left, right, 1, operator)?
            };
        }
    }

    fn unary(&mut self) -> Result<Quantity, ExprError> {
        match self.current.kind {
            Kind::Minus => {
                self.advance()?;
                let operand = self.unary()?;
                Ok(Quantity {
                    value: -operand.value,
                    ..operand
                })
            }
            Kind::Plus => {
                self.advance()?;
                self.unary()
            }
            _ => self.power(),
        }
    }

    fn power(&mut self) -> Result<Quantity, ExprError> {
        let base = self.quantity()?;
        if self.current.kind != Kind::Caret {
            return Ok(base);
        }
        let operator = self.current;
        self.advance()?;
        let exponent = self.unary()?;
        if !exponent.is_plain() {
            return Err(operator.error(ErrorKind::IncompatibleUnits));
        }
        let mut dimensions = base.dimensions;
        if !base.is_plain() {
            // A quantity with units takes whole powers only.
            let whole = exponent.value;
            if whole.fract() != 0.0 || whole.abs() > 8.0 {
                return Err(operator.error(ErrorKind::IncompatibleUnits));
            }
            #[allow(
                clippy::cast_possible_truncation,
                reason = "a whole number between -8 and 8"
            )]
            let whole = whole as i8;
            for power in &mut dimensions {
                *power = power
                    .checked_mul(whole)
                    .ok_or_else(|| operator.error(ErrorKind::IncompatibleUnits))?;
            }
        }
        let value = base.value.powf(exponent.value);
        if !value.is_finite() {
            return Err(operator.error(ErrorKind::NotFinite));
        }
        Ok(Quantity { value, dimensions })
    }

    /// The unit named by the current token, unless it calls a function (`min(...)`).
    fn unit_here(&self) -> Option<&'static Unit> {
        match self.current.kind {
            Kind::Name(name) if !self.call_follows() => units::unit(name),
            _ => None,
        }
    }

    fn quantity(&mut self) -> Result<Quantity, ExprError> {
        let first = self.current;
        let value = self.primary()?;
        let Some(unit) = self.unit_here() else {
            return Ok(value);
        };
        if !value.is_plain() {
            return Err(self.current.error(ErrorKind::IncompatibleUnits));
        }
        self.advance()?;
        let mut total = self.in_unit(value.value, unit)?;
        // Quantities side by side add up: `1m 20cm`.
        while let Kind::Number(_) = self.current.kind {
            let number = self.current;
            let value = self.primary()?;
            let Some(unit) = self.unit_here() else {
                return Err(number.error(ErrorKind::Unexpected));
            };
            let unit_token = self.current;
            self.advance()?;
            let next = self.in_unit(value.value, unit)?;
            if next.dimensions != total.dimensions {
                return Err(ExprError {
                    kind: ErrorKind::IncompatibleUnits,
                    start: first.start,
                    end: unit_token.end,
                });
            }
            total.value += next.value;
        }
        Ok(total)
    }

    /// `value` of `unit`, the unit raised to the power that follows it if any: `4cm^2` is
    /// four square centimeters.
    fn in_unit(&mut self, value: f64, unit: &Unit) -> Result<Quantity, ExprError> {
        let size = unit.base(self.context.pixels_per_inch);
        let mut dimensions = dimensions_of(unit);
        if self.current.kind != Kind::Caret {
            return Ok(Quantity {
                value: value * size,
                dimensions,
            });
        }
        let operator = self.current;
        self.advance()?;
        let exponent = self.unary()?;
        if !exponent.is_plain() || exponent.value.fract() != 0.0 || exponent.value.abs() > 8.0 {
            return Err(operator.error(ErrorKind::IncompatibleUnits));
        }
        #[allow(
            clippy::cast_possible_truncation,
            reason = "a whole number between -8 and 8"
        )]
        let power = exponent.value as i8;
        for dimension in &mut dimensions {
            *dimension *= power;
        }
        Ok(Quantity {
            value: value * size.powi(i32::from(power)),
            dimensions,
        })
    }

    fn primary(&mut self) -> Result<Quantity, ExprError> {
        let token = self.current;
        match token.kind {
            Kind::Number(value) => {
                self.advance()?;
                let integer = self.text[token.start..token.end]
                    .bytes()
                    .all(|byte| byte.is_ascii_digit());
                let decimal_comma = self.arguments == 0
                    && integer
                    && self.current.kind == Kind::Comma
                    && !self.current.spaced
                    && self.digit_follows();
                if !decimal_comma {
                    return Ok(Quantity::plain(value));
                }
                // `1,5`: the comma is the decimal separator.
                self.advance()?;
                let fraction = self.current;
                let digits = &self.text[fraction.start..fraction.end];
                if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err(fraction.error(ErrorKind::InvalidNumber));
                }
                let Kind::Number(fraction_value) = fraction.kind else {
                    return Err(fraction.error(ErrorKind::InvalidNumber));
                };
                self.advance()?;
                let scale = 10_f64.powi(i32::try_from(digits.len()).unwrap_or(i32::MAX));
                Ok(Quantity::plain(value + fraction_value / scale))
            }
            Kind::Open => {
                self.advance()?;
                let outer = core::mem::replace(&mut self.arguments, 0);
                let inner = self.sum()?;
                self.arguments = outer;
                match self.current.kind {
                    Kind::Close => {
                        self.advance()?;
                        Ok(inner)
                    }
                    Kind::End => Err(token.error(ErrorKind::UnclosedParenthesis)),
                    _ => Err(self.current.error(ErrorKind::Unexpected)),
                }
            }
            Kind::Name(name) => {
                if let Some(value) = constant(name) {
                    self.advance()?;
                    return Ok(Quantity::plain(value));
                }
                if FUNCTIONS.contains(&name) {
                    return self.call(token, name);
                }
                let kind = if units::unit(name).is_some() {
                    // A unit with no value before it.
                    ErrorKind::Unexpected
                } else {
                    ErrorKind::UnknownName
                };
                Err(token.error(kind))
            }
            Kind::End => Err(token.error(ErrorKind::UnexpectedEnd)),
            _ => Err(token.error(ErrorKind::Unexpected)),
        }
    }

    fn call(&mut self, name_token: Token<'t>, name: &str) -> Result<Quantity, ExprError> {
        self.advance()?;
        let open = self.current;
        if open.kind != Kind::Open {
            let kind = if open.kind == Kind::End {
                ErrorKind::UnexpectedEnd
            } else {
                ErrorKind::Unexpected
            };
            return Err(open.error(kind));
        }
        self.advance()?;
        let mut arguments = [Quantity::plain(0.0); MAX_ARGUMENTS];
        let mut count = 0;
        self.arguments += 1;
        if self.current.kind != Kind::Close {
            loop {
                let value = self.sum()?;
                if let Some(slot) = arguments.get_mut(count) {
                    *slot = value;
                }
                count += 1;
                match self.current.kind {
                    Kind::Comma => self.advance()?,
                    Kind::Close => break,
                    Kind::End => return Err(open.error(ErrorKind::UnclosedParenthesis)),
                    _ => return Err(self.current.error(ErrorKind::Unexpected)),
                }
            }
        }
        self.arguments -= 1;
        let close = self.current;
        self.advance()?;
        let span = ExprError {
            kind: ErrorKind::WrongArgumentCount,
            start: name_token.start,
            end: close.end,
        };
        if count > MAX_ARGUMENTS {
            return Err(span);
        }
        self.apply(name, &arguments[..count], span)
    }

    /// The value of the function `name` of `arguments`; `span` covers the call.
    fn apply(
        &self,
        name: &str,
        arguments: &[Quantity],
        span: ExprError,
    ) -> Result<Quantity, ExprError> {
        let error = |kind| ExprError { kind, ..span };
        let count = |expected: usize| {
            if arguments.len() == expected {
                Ok(())
            } else {
                Err(span)
            }
        };
        let plain = |quantity: Quantity| {
            if quantity.is_plain() {
                Ok(quantity.value)
            } else {
                Err(error(ErrorKind::IncompatibleUnits))
            }
        };
        let angle = |quantity: Quantity| {
            if quantity.is_plain() || quantity.dimensions == [0, 1, 0] {
                Ok(quantity.value)
            } else {
                Err(error(ErrorKind::IncompatibleUnits))
            }
        };
        let radians = |value: f64| Quantity {
            value,
            dimensions: [0, 1, 0],
        };
        let result = match name {
            "sqrt" => {
                count(1)?;
                let quantity = arguments[0];
                if quantity.dimensions.iter().any(|power| power % 2 != 0) {
                    return Err(error(ErrorKind::IncompatibleUnits));
                }
                Quantity {
                    value: quantity.value.sqrt(),
                    dimensions: quantity.dimensions.map(|power| power / 2),
                }
            }
            "abs" => {
                count(1)?;
                Quantity {
                    value: arguments[0].value.abs(),
                    ..arguments[0]
                }
            }
            "floor" | "ceil" | "round" => {
                count(1)?;
                let rounding = |value: f64| match name {
                    "floor" => value.floor(),
                    "ceil" => value.ceil(),
                    _ => value.round(),
                };
                let quantity = arguments[0];
                match self.field() {
                    _ if quantity.is_plain() => Quantity::plain(rounding(quantity.value)),
                    // Rounded in the field's unit.
                    Some((dimensions, base)) if quantity.dimensions == dimensions => Quantity {
                        value: rounding(quantity.value / base) * base,
                        dimensions,
                    },
                    _ => return Err(error(ErrorKind::IncompatibleUnits)),
                }
            }
            "min" | "max" => {
                let Some((first, rest)) = arguments.split_first() else {
                    return Err(span);
                };
                let mut best = *first;
                for next in rest {
                    let (a, b) = self.align(best, *next, function_token(span))?;
                    let pick = if name == "min" {
                        a.value.min(b.value)
                    } else {
                        a.value.max(b.value)
                    };
                    best = Quantity {
                        value: pick,
                        dimensions: a.dimensions,
                    };
                }
                best
            }
            "mod" => {
                count(2)?;
                let (a, b) = self.align(arguments[0], arguments[1], function_token(span))?;
                if b.value == 0.0 {
                    return Err(error(ErrorKind::NotFinite));
                }
                Quantity {
                    value: a.value.rem_euclid(b.value),
                    dimensions: a.dimensions,
                }
            }
            "exp" | "ln" | "log" | "log2" => {
                count(1)?;
                let value = plain(arguments[0])?;
                Quantity::plain(match name {
                    "exp" => value.exp(),
                    "ln" => value.ln(),
                    "log" => value.log10(),
                    _ => value.log2(),
                })
            }
            "sin" | "cos" | "tan" => {
                count(1)?;
                let value = angle(arguments[0])?;
                Quantity::plain(match name {
                    "sin" => value.sin(),
                    "cos" => value.cos(),
                    _ => value.tan(),
                })
            }
            "asin" | "acos" | "atan" => {
                count(1)?;
                let value = plain(arguments[0])?;
                radians(match name {
                    "asin" => value.asin(),
                    "acos" => value.acos(),
                    _ => value.atan(),
                })
            }
            "atan2" => {
                count(2)?;
                let (y, x) = self.align(arguments[0], arguments[1], function_token(span))?;
                radians(y.value.atan2(x.value))
            }
            _ => return Err(error(ErrorKind::UnknownName)),
        };
        if result.value.is_finite() {
            Ok(result)
        } else {
            Err(error(ErrorKind::NotFinite))
        }
    }
}

/// The functions expressions call.
const FUNCTIONS: [&str; 19] = [
    "sqrt", "abs", "floor", "ceil", "round", "min", "max", "mod", "exp", "ln", "log", "log2",
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2",
];

/// The value of the constant `name`, if it is one.
fn constant(name: &str) -> Option<f64> {
    match name {
        "pi" | "\u{3c0}" => Some(core::f64::consts::PI),
        "tau" | "\u{3c4}" => Some(core::f64::consts::TAU),
        "e" => Some(core::f64::consts::E),
        _ => None,
    }
}

/// A token covering a function call, for the errors of its arguments.
fn function_token(span: ExprError) -> Token<'static> {
    Token {
        kind: Kind::End,
        start: span.start,
        end: span.end,
        spaced: false,
    }
}

/// `a` times `b` raised to `sign` for the dimensions (1 for a product, -1 for a quotient,
/// `b`'s value already inverted).
fn combine(a: Quantity, b: Quantity, sign: i8, at: Token<'_>) -> Result<Quantity, ExprError> {
    let mut dimensions = a.dimensions;
    for (power, other) in dimensions.iter_mut().zip(b.dimensions) {
        *power = power
            .checked_add(other * sign)
            .ok_or_else(|| at.error(ErrorKind::IncompatibleUnits))?;
    }
    Ok(Quantity {
        value: a.value * b.value,
        dimensions,
    })
}

/// The length of the number at the start of `text`: digits, a fraction, an exponent.
fn number_length(text: &str) -> usize {
    let bytes = text.as_bytes();
    let digits = |from: usize| {
        bytes[from..]
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count()
    };
    let mut end = digits(0);
    if bytes.get(end) == Some(&b'.') {
        end += 1 + digits(end + 1);
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(bytes.get(end + 1), Some(b'+' | b'-')));
        let exponent = digits(end + 1 + sign);
        // `2e` is two times e: an exponent needs digits.
        if exponent > 0 {
            end += 1 + sign + exponent;
        }
    }
    end
}
