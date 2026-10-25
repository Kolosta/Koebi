//! A strict JSON parser and a pretty printer, for the interchange format.
//!
//! The parser follows RFC 8259 strictly: no comments, no trailing commas, no `NaN`, no leading
//! zeros, escaped control characters, and duplicate object keys are errors. Every value keeps
//! the line and column where it starts, so that import errors point at the right place.

use core::fmt::{self, Write as _};

/// Where a value starts in the source text, both 1-based.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Position {
    pub(crate) line: u32,
    pub(crate) column: u32,
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// A JSON value with its position.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Json {
    pub(crate) value: JsonValue,
    pub(crate) position: Position,
}

/// The value itself. Objects keep their keys in source order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    /// A value without a meaningful position, for documents built in code.
    pub(crate) fn new(value: JsonValue) -> Self {
        Self {
            value,
            position: Position::default(),
        }
    }

    pub(crate) fn as_str(&self) -> Option<&str> {
        match &self.value {
            JsonValue::String(text) => Some(text),
            _ => None,
        }
    }

    pub(crate) fn as_f64(&self) -> Option<f64> {
        match self.value {
            JsonValue::Number(number) => Some(number),
            _ => None,
        }
    }

    pub(crate) fn as_object(&self) -> Option<&[(String, Json)]> {
        match &self.value {
            JsonValue::Object(members) => Some(members),
            _ => None,
        }
    }

    pub(crate) fn as_array(&self) -> Option<&[Json]> {
        match &self.value {
            JsonValue::Array(items) => Some(items),
            _ => None,
        }
    }

    /// The member `key` of an object.
    pub(crate) fn get(&self, key: &str) -> Option<&Json> {
        self.as_object()?
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    /// A short name of the value's type, for messages.
    pub(crate) fn type_name(&self) -> &'static str {
        match self.value {
            JsonValue::Null => "null",
            JsonValue::Bool(_) => "a boolean",
            JsonValue::Number(_) => "a number",
            JsonValue::String(_) => "a string",
            JsonValue::Array(_) => "an array",
            JsonValue::Object(_) => "an object",
        }
    }
}

/// A syntax error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParseError {
    pub(crate) position: Position,
    pub(crate) message: String,
}

const MAX_DEPTH: usize = 128;

/// Parses a complete JSON document.
pub(crate) fn parse(text: &str) -> Result<Json, ParseError> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        index: 0,
        line: 1,
        column: 1,
    };
    parser.skip_whitespace();
    let value = parser.value(0)?;
    parser.skip_whitespace();
    if parser.index < parser.bytes.len() {
        return Err(parser.error("unexpected text after the end of the document"));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    index: usize,
    line: u32,
    column: u32,
}

impl Parser<'_> {
    fn position(&self) -> Position {
        Position {
            line: self.line,
            column: self.column,
        }
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        ParseError {
            position: self.position(),
            message: message.into(),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.index).copied()
    }

    fn advance(&mut self) {
        if let Some(&byte) = self.bytes.get(self.index) {
            self.index += 1;
            if byte == b'\n' {
                self.line += 1;
                self.column = 1;
            } else if byte & 0xC0 != 0x80 {
                // Count characters, not UTF-8 continuation bytes.
                self.column += 1;
            }
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.advance();
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), ParseError> {
        if self.peek() == Some(byte) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(format!("expected '{}'", char::from(byte))))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, ParseError> {
        if depth > MAX_DEPTH {
            return Err(self.error("nesting deeper than 128 levels"));
        }
        let position = self.position();
        let value = match self.peek() {
            Some(b'{') => self.object(depth)?,
            Some(b'[') => self.array(depth)?,
            Some(b'"') => JsonValue::String(self.string()?),
            Some(b't') => self.keyword("true", JsonValue::Bool(true))?,
            Some(b'f') => self.keyword("false", JsonValue::Bool(false))?,
            Some(b'n') => self.keyword("null", JsonValue::Null)?,
            Some(b'-' | b'0'..=b'9') => JsonValue::Number(self.number()?),
            Some(_) => return Err(self.error("expected a value")),
            None => return Err(self.error("unexpected end of the document")),
        };
        Ok(Json { value, position })
    }

    fn keyword(&mut self, word: &str, value: JsonValue) -> Result<JsonValue, ParseError> {
        if self.bytes[self.index..].starts_with(word.as_bytes()) {
            for _ in 0..word.len() {
                self.advance();
            }
            Ok(value)
        } else {
            Err(self.error("expected a value"))
        }
    }

    fn object(&mut self, depth: usize) -> Result<JsonValue, ParseError> {
        self.advance();
        let mut members: Vec<(String, Json)> = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.advance();
            return Ok(JsonValue::Object(members));
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(self.error("expected a key in double quotes"));
            }
            let key_position = self.position();
            let key = self.string()?;
            if members.iter().any(|(name, _)| *name == key) {
                return Err(ParseError {
                    position: key_position,
                    message: format!("duplicate key \"{key}\""),
                });
            }
            self.skip_whitespace();
            self.expect(b':')?;
            self.skip_whitespace();
            let value = self.value(depth + 1)?;
            members.push((key, value));
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.advance(),
                Some(b'}') => {
                    self.advance();
                    return Ok(JsonValue::Object(members));
                }
                _ => return Err(self.error("expected ',' or '}'")),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<JsonValue, ParseError> {
        self.advance();
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.advance();
            return Ok(JsonValue::Array(items));
        }
        loop {
            self.skip_whitespace();
            items.push(self.value(depth + 1)?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.advance(),
                Some(b']') => {
                    self.advance();
                    return Ok(JsonValue::Array(items));
                }
                _ => return Err(self.error("expected ',' or ']'")),
            }
        }
    }

    fn string(&mut self) -> Result<String, ParseError> {
        self.advance();
        let mut text = String::new();
        loop {
            let start = self.index;
            // Copy runs of plain characters at once.
            while let Some(byte) = self.peek() {
                if byte == b'"' || byte == b'\\' || byte < 0x20 {
                    break;
                }
                self.advance();
            }
            text.push_str(
                core::str::from_utf8(&self.bytes[start..self.index])
                    .map_err(|_| self.error("invalid UTF-8"))?,
            );
            match self.peek() {
                Some(b'"') => {
                    self.advance();
                    return Ok(text);
                }
                Some(b'\\') => {
                    self.advance();
                    self.escape(&mut text)?;
                }
                Some(_) => return Err(self.error("control characters must be escaped")),
                None => return Err(self.error("unterminated string")),
            }
        }
    }

    fn escape(&mut self, text: &mut String) -> Result<(), ParseError> {
        let Some(byte) = self.peek() else {
            return Err(self.error("unterminated string"));
        };
        self.advance();
        let simple = match byte {
            b'"' => Some('"'),
            b'\\' => Some('\\'),
            b'/' => Some('/'),
            b'b' => Some('\u{8}'),
            b'f' => Some('\u{c}'),
            b'n' => Some('\n'),
            b'r' => Some('\r'),
            b't' => Some('\t'),
            b'u' => None,
            _ => return Err(self.error("invalid escape sequence")),
        };
        if let Some(ch) = simple {
            text.push(ch);
            return Ok(());
        }
        let high = self.hex4()?;
        let code = if (0xD800..0xDC00).contains(&high) {
            if self.peek() != Some(b'\\') {
                return Err(self.error("unpaired surrogate"));
            }
            self.advance();
            self.expect(b'u')?;
            let low = self.hex4()?;
            if !(0xDC00..0xE000).contains(&low) {
                return Err(self.error("unpaired surrogate"));
            }
            0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00)
        } else {
            high
        };
        let ch = char::from_u32(code).ok_or_else(|| self.error("invalid code point"))?;
        text.push(ch);
        Ok(())
    }

    fn hex4(&mut self) -> Result<u32, ParseError> {
        let mut value = 0;
        for _ in 0..4 {
            let digit = self
                .peek()
                .and_then(|byte| char::from(byte).to_digit(16))
                .ok_or_else(|| self.error("expected four hexadecimal digits"))?;
            value = value * 16 + digit;
            self.advance();
        }
        Ok(value)
    }

    fn number(&mut self) -> Result<f64, ParseError> {
        let start = self.index;
        let digits = |parser: &mut Self| {
            let begin = parser.index;
            while matches!(parser.peek(), Some(b'0'..=b'9')) {
                parser.advance();
            }
            parser.index > begin
        };
        if self.peek() == Some(b'-') {
            self.advance();
        }
        if self.peek() == Some(b'0') {
            self.advance();
            if matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error("numbers cannot have leading zeros"));
            }
        } else if !digits(self) {
            return Err(self.error("expected digits"));
        }
        if self.peek() == Some(b'.') {
            self.advance();
            if !digits(self) {
                return Err(self.error("expected digits after the decimal point"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.advance();
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.advance();
            }
            if !digits(self) {
                return Err(self.error("expected digits in the exponent"));
            }
        }
        let text = core::str::from_utf8(&self.bytes[start..self.index])
            .map_err(|_| self.error("invalid number"))?;
        let number: f64 = text.parse().map_err(|_| self.error("invalid number"))?;
        if number.is_finite() {
            Ok(number)
        } else {
            Err(self.error("number out of range"))
        }
    }
}

/// Writes `value` with two-space indentation, keys in their order.
pub(crate) fn to_string_pretty(value: &Json) -> String {
    let mut out = String::new();
    write_value(&mut out, value, 0);
    out.push('\n');
    out
}

fn write_value(out: &mut String, value: &Json, indent: usize) {
    match &value.value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        JsonValue::Number(n) => write_number(out, *n),
        JsonValue::String(text) => write_string(out, text),
        JsonValue::Array(items) => {
            // Short arrays of numbers stay on one line: `[0.25, 0.1, 0.25, 1]`.
            if items
                .iter()
                .all(|item| matches!(item.value, JsonValue::Number(_)))
            {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    write_value(out, item, indent);
                }
                out.push(']');
                return;
            }
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                out.push_str(if i == 0 { "\n" } else { ",\n" });
                push_indent(out, indent + 1);
                write_value(out, item, indent + 1);
            }
            if !items.is_empty() {
                out.push('\n');
                push_indent(out, indent);
            }
            out.push(']');
        }
        JsonValue::Object(members) => {
            out.push('{');
            for (i, (key, item)) in members.iter().enumerate() {
                out.push_str(if i == 0 { "\n" } else { ",\n" });
                push_indent(out, indent + 1);
                write_string(out, key);
                out.push_str(": ");
                write_value(out, item, indent + 1);
            }
            if !members.is_empty() {
                out.push('\n');
                push_indent(out, indent);
            }
            out.push('}');
        }
    }
}

fn push_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn write_number(out: &mut String, number: f64) {
    if number.is_finite() {
        // The shortest form that reads back to the same value; `1` rather than `1.0`.
        if number.fract() == 0.0 && number.abs() < 1e15 {
            let _ = write!(out, "{number:.0}");
        } else {
            let _ = write!(out, "{number}");
        }
    } else {
        out.push_str("null");
    }
}

fn write_string(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error(text: &str) -> ParseError {
        parse(text).expect_err(text)
    }

    #[test]
    fn documents_parse_with_positions() {
        let doc =
            parse("{\n  \"a\": [1, -2.5e3, true, null],\n  \"b\": {\"c\": \"x\"}\n}").unwrap();
        let a = doc.get("a").unwrap();
        assert_eq!(a.position, Position { line: 2, column: 8 });
        let items = a.as_array().unwrap();
        assert_eq!(items[1].as_f64(), Some(-2500.0));
        assert_eq!(items[2].value, JsonValue::Bool(true));
        assert_eq!(doc.get("b").unwrap().get("c").unwrap().as_str(), Some("x"));
    }

    #[test]
    fn strings_decode_escapes_and_surrogates() {
        let doc = parse(r#""a\"b\\c\/d\né😀""#).unwrap();
        assert_eq!(doc.as_str(), Some("a\"b\\c/d\né😀"));
    }

    #[test]
    fn strict_errors_point_at_the_problem() {
        assert_eq!(
            error("{\"a\": 1,}").message,
            "expected a key in double quotes"
        );
        assert_eq!(error("[1, 2,]").message, "expected a value");
        assert_eq!(
            error("{\"a\": 01}").message,
            "numbers cannot have leading zeros"
        );
        assert_eq!(error("NaN").message, "expected a value");
        assert_eq!(error("// x\n{}").message, "expected a value");
        assert_eq!(
            error("{} {}").message,
            "unexpected text after the end of the document"
        );
        assert_eq!(
            error("\"\u{1}\"").message,
            "control characters must be escaped"
        );
        assert_eq!(error(r#""\ud800""#).message, "unpaired surrogate");
        let duplicate = error("{\n\"a\": 1,\n\"a\": 2}");
        assert_eq!(duplicate.message, "duplicate key \"a\"");
        assert_eq!(duplicate.position, Position { line: 3, column: 1 });
        assert!(error(&"[".repeat(200)).message.contains("nesting"));
    }

    #[test]
    fn pretty_printing_round_trips() {
        let text = "{\n  \"name\": \"Koebi \\\"tokens\\\"\",\n  \"curve\": [0.25, 0.1, 0.25, 1],\n  \"nested\": {\n    \"empty\": {},\n    \"list\": [\n      \"a\",\n      {}\n    ]\n  },\n  \"flag\": false\n}\n";
        let doc = parse(text).unwrap();
        assert_eq!(to_string_pretty(&doc), text);
        assert_eq!(parse(&to_string_pretty(&doc)).unwrap().value, doc.value);
    }

    #[test]
    fn numbers_print_in_their_shortest_form() {
        let mut out = String::new();
        for n in [1.0, -0.5, 0.1, 1e20, 123_456.0] {
            write_number(&mut out, n);
            out.push(' ');
        }
        assert_eq!(out, "1 -0.5 0.1 100000000000000000000 123456 ");
    }
}
