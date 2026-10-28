//! Values as text, without allocating per frame: rounded to the decimals a field shows, with
//! its unit, and with every decimal that matters when the value is typed.

use core::fmt::{self, Write as _};

use koebi_core::expr::Unit;

/// A short text written in place: a value and its unit. Text past its capacity is dropped.
#[derive(Clone, Copy)]
pub(crate) struct ValueText {
    bytes: [u8; Self::CAPACITY],
    len: usize,
}

impl ValueText {
    const CAPACITY: usize = 64;

    /// An empty text.
    pub(crate) const fn new() -> Self {
        Self {
            bytes: [0; Self::CAPACITY],
            len: 0,
        }
    }

    /// The text.
    pub(crate) fn as_str(&self) -> &str {
        // Whole characters copied from `&str`s.
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or_default()
    }

    /// `value` with `decimals` decimals (none for integers), then the unit's symbol: `1.50 m`,
    /// `90°`. Huge values show in scientific notation (`1.50e20`).
    pub(crate) fn shown(value: f64, decimals: usize, unit: Option<&Unit>) -> Self {
        let mut text = Self::new();
        // A rounded zero shows without its sign.
        let rounded = if format_is_zero(value, decimals) {
            0.0
        } else {
            value
        };
        let _ = if is_huge(rounded) {
            write!(text, "{rounded:.decimals$e}")
        } else {
            write!(text, "{rounded:.decimals$}")
        };
        text.push_unit(unit);
        text
    }

    /// `value` with every decimal that matters, up to `decimals` more than shown, then the
    /// unit's symbol: the text an edit starts from.
    pub(crate) fn typed(value: f64, decimals: usize, unit: Option<&Unit>) -> Self {
        let mut text = Self::new();
        if is_huge(value) {
            // Every digit that matters, as an expression reads it back (`1e70`).
            let _ = write!(text, "{value:e}");
            text.push_unit(unit);
            return text;
        }
        let precise = decimals + 4;
        let _ = write!(text, "{value:.precise$}");
        if text.as_str().contains('.') {
            while text.as_str().ends_with('0') {
                text.len -= 1;
            }
            if text.as_str().ends_with('.') {
                text.len -= 1;
            }
        }
        if text.as_str() == "-0" {
            text.len = 0;
            let _ = text.write_str("0");
        }
        text.push_unit(unit);
        text
    }

    /// What follows a value in `unit`: a space and its symbol, or the degree sign alone.
    pub(crate) fn unit(unit: Option<&Unit>) -> Self {
        let mut text = Self::new();
        text.push_unit(unit);
        text
    }

    /// Appends the unit's symbol, after a space unless it is a degree sign.
    fn push_unit(&mut self, unit: Option<&Unit>) {
        if let Some(unit) = unit {
            if unit.symbol != "\u{b0}" {
                let _ = self.write_str(" ");
            }
            let _ = self.write_str(unit.symbol);
        }
    }
}

/// `true` for values too large to write digit by digit, and for infinities and NaN.
fn is_huge(value: f64) -> bool {
    value.is_nan() || value.abs() >= 1e15
}

/// `true` when `value` shows as zero with `decimals` decimals.
fn format_is_zero(value: f64, decimals: usize) -> bool {
    let scale = 10f64.powi(i32::try_from(decimals).unwrap_or(i32::MAX));
    (value * scale).round() == 0.0
}

impl fmt::Write for ValueText {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let room = Self::CAPACITY - self.len;
        let mut end = text.len().min(room);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        self.bytes[self.len..self.len + end].copy_from_slice(&text.as_bytes()[..end]);
        self.len += end;
        if end < text.len() {
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

impl fmt::Debug for ValueText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::expr::{DEGREE, MILLIMETER};

    use super::*;

    #[test]
    fn values_show_rounded_with_their_unit() {
        assert_eq!(ValueText::shown(1.5, 2, None).as_str(), "1.50");
        assert_eq!(
            ValueText::shown(12.0, 1, Some(&MILLIMETER)).as_str(),
            "12.0 mm"
        );
        assert_eq!(
            ValueText::shown(90.0, 0, Some(&DEGREE)).as_str(),
            "90\u{b0}"
        );
        assert_eq!(ValueText::shown(-0.001, 2, None).as_str(), "0.00");
    }

    #[test]
    fn typed_values_keep_the_decimals_that_matter() {
        assert_eq!(ValueText::typed(1.25, 2, None).as_str(), "1.25");
        assert_eq!(ValueText::typed(2.0, 2, Some(&MILLIMETER)).as_str(), "2 mm");
        assert_eq!(ValueText::typed(0.123_456, 2, None).as_str(), "0.123456");
        assert_eq!(ValueText::typed(-0.000_000_1, 2, None).as_str(), "0");
    }

    #[test]
    fn huge_values_keep_their_magnitude() {
        // Regression test: the digits of 1e64 overflowed the buffer, and the cut text read
        // back ten times smaller.
        assert_eq!(ValueText::typed(1e64, 2, None).as_str(), "1e64");
        assert_eq!(ValueText::shown(1.5e20, 2, None).as_str(), "1.50e20");
        let max = ValueText::typed(f64::MAX, 2, None);
        let context = koebi_core::expr::Context::default();
        assert_eq!(
            koebi_core::expr::evaluate(max.as_str(), &context),
            Ok(f64::MAX)
        );
        assert_eq!(ValueText::shown(f64::INFINITY, 2, None).as_str(), "inf");
    }
}
