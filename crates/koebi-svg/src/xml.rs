//! A small XML reader, enough for SVG files: elements, attributes, comments, the prolog and
//! the doctype, character references. Text content is kept only to refuse it where SVG would
//! draw it. Every element remembers its line, for errors.

use crate::error::{Error, Result};

/// An element and what it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Element {
    /// Its name, without a namespace prefix for `svg:` ones (`svg:path` is `path`).
    pub(crate) name: String,
    /// Its attributes in the order written; `xlink:href` keeps its prefix.
    pub(crate) attributes: Vec<(String, String)>,
    pub(crate) children: Vec<Element>,
    /// Whether text other than white space lies directly inside it.
    pub(crate) has_text: bool,
    /// The line it starts on, from 1.
    pub(crate) line: usize,
}

impl Element {
    /// The value of the attribute `name`.
    pub(crate) fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// Reads the root element of `text`.
pub(crate) fn parse(text: &str) -> Result<Element> {
    let mut reader = Reader {
        bytes: text.as_bytes(),
        text,
        at: 0,
    };
    reader.skip_prolog()?;
    let root = reader.element()?;
    reader.skip_misc()?;
    if reader.at < reader.bytes.len() {
        return Err(reader.error("content after the root element"));
    }
    Ok(root)
}

struct Reader<'a> {
    bytes: &'a [u8],
    text: &'a str,
    at: usize,
}

impl Reader<'_> {
    fn line(&self) -> usize {
        self.text[..self.at.min(self.text.len())]
            .matches('\n')
            .count()
            + 1
    }

    fn error(&self, message: impl Into<String>) -> Error {
        Error::at(self.line(), message)
    }

    fn starts_with(&self, prefix: &str) -> bool {
        self.bytes[self.at..].starts_with(prefix.as_bytes())
    }

    fn skip_spaces(&mut self) {
        while self.at < self.bytes.len() && self.bytes[self.at].is_ascii_whitespace() {
            self.at += 1;
        }
    }

    /// Skips past `end`, which must come.
    fn skip_past(&mut self, end: &str, what: &str) -> Result<()> {
        match self.text[self.at..].find(end) {
            Some(offset) => {
                self.at += offset + end.len();
                Ok(())
            }
            None => Err(self.error(format!("unterminated {what}"))),
        }
    }

    /// Comments, processing instructions and white space.
    fn skip_misc(&mut self) -> Result<()> {
        loop {
            self.skip_spaces();
            if self.starts_with("<!--") {
                self.skip_past("-->", "comment")?;
            } else if self.starts_with("<?") {
                self.skip_past("?>", "processing instruction")?;
            } else {
                return Ok(());
            }
        }
    }

    /// The XML declaration, comments and a doctype before the root element.
    fn skip_prolog(&mut self) -> Result<()> {
        // A byte order mark.
        if self.starts_with("\u{feff}") {
            self.at += "\u{feff}".len();
        }
        loop {
            self.skip_misc()?;
            if self.starts_with("<!DOCTYPE") {
                // An internal subset would declare entities: refused, there are none to read.
                let end = self.text[self.at..]
                    .find('>')
                    .ok_or_else(|| self.error("unterminated doctype"))?;
                if self.text[self.at..self.at + end].contains('[') {
                    return Err(self.error("doctypes with an internal subset are not supported"));
                }
                self.at += end + 1;
            } else {
                return Ok(());
            }
        }
    }

    fn name(&mut self) -> Result<String> {
        let start = self.at;
        while self.at < self.bytes.len() {
            let byte = self.bytes[self.at];
            if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':') {
                self.at += 1;
            } else {
                break;
            }
        }
        if start == self.at {
            return Err(self.error("a name was expected"));
        }
        Ok(self.text[start..self.at].to_owned())
    }

    fn element(&mut self) -> Result<Element> {
        let line = self.line();
        if !self.starts_with("<") {
            return Err(self.error("an element was expected"));
        }
        self.at += 1;
        let raw = self.name()?;
        let name = raw.strip_prefix("svg:").unwrap_or(&raw).to_owned();
        let mut element = Element {
            name,
            attributes: Vec::new(),
            children: Vec::new(),
            has_text: false,
            line,
        };
        loop {
            self.skip_spaces();
            if self.starts_with("/>") {
                self.at += 2;
                return Ok(element);
            }
            if self.starts_with(">") {
                self.at += 1;
                break;
            }
            let key = self.name()?;
            self.skip_spaces();
            if !self.starts_with("=") {
                return Err(self.error(format!("the attribute {key} has no value")));
            }
            self.at += 1;
            self.skip_spaces();
            let quote = *self
                .bytes
                .get(self.at)
                .ok_or_else(|| self.error("unterminated attribute"))?;
            if quote != b'"' && quote != b'\'' {
                return Err(self.error(format!("the value of {key} is not quoted")));
            }
            self.at += 1;
            let end = self.text[self.at..]
                .find(char::from(quote))
                .ok_or_else(|| self.error(format!("unterminated value of {key}")))?;
            let value = unescape(&self.text[self.at..self.at + end], self.line())?;
            self.at += end + 1;
            if element
                .attributes
                .iter()
                .any(|(existing, _)| *existing == key)
            {
                return Err(self.error(format!("the attribute {key} is given twice")));
            }
            element.attributes.push((key, value));
        }
        // Content, until the closing tag.
        loop {
            let text_start = self.at;
            while self.at < self.bytes.len() && self.bytes[self.at] != b'<' {
                self.at += 1;
            }
            if self.text[text_start..self.at]
                .chars()
                .any(|character| !character.is_whitespace())
            {
                element.has_text = true;
            }
            if self.at >= self.bytes.len() {
                return Err(self.error(format!("the element {raw} is not closed")));
            }
            if self.starts_with("</") {
                self.at += 2;
                let closing = self.name()?;
                if closing != raw {
                    return Err(self.error(format!("{raw} is closed by {closing}")));
                }
                self.skip_spaces();
                if !self.starts_with(">") {
                    return Err(self.error("a closing tag ends with >"));
                }
                self.at += 1;
                return Ok(element);
            }
            if self.starts_with("<!--") {
                self.skip_past("-->", "comment")?;
            } else if self.starts_with("<![CDATA[") {
                self.skip_past("]]>", "CDATA section")?;
                element.has_text = true;
            } else if self.starts_with("<?") {
                self.skip_past("?>", "processing instruction")?;
            } else {
                let child = self.element()?;
                element.children.push(child);
            }
        }
    }
}

/// `text` with its character and entity references replaced.
fn unescape(text: &str, line: usize) -> Result<String> {
    if !text.contains('&') {
        return Ok(text.to_owned());
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let end = after
            .find(';')
            .ok_or_else(|| Error::at(line, "unterminated reference"))?;
        let reference = &after[..end];
        let character = match reference {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let code = if let Some(hex) = reference.strip_prefix("#x") {
                    u32::from_str_radix(hex, 16).ok()
                } else if let Some(decimal) = reference.strip_prefix('#') {
                    decimal.parse().ok()
                } else {
                    None
                };
                code.and_then(char::from_u32)
                    .ok_or_else(|| Error::at(line, format!("unknown reference &{reference};")))?
            }
        };
        out.push(character);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elements_attributes_and_lines_are_read() {
        let text = "<?xml version=\"1.0\"?>\n<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"x\">\n<!-- a comment -->\n<svg viewBox='0 0 24 24' a=\"&lt;&#x41;&amp;\">\n  <g>\n    <path d=\"M0 0\"/>\n  </g>\n</svg>\n";
        let root = parse(text).unwrap();
        assert_eq!(root.name, "svg");
        assert_eq!(root.attribute("viewBox"), Some("0 0 24 24"));
        assert_eq!(root.attribute("a"), Some("<A&"));
        assert_eq!(root.line, 4);
        let path = &root.children[0].children[0];
        assert_eq!((path.name.as_str(), path.line), ("path", 6));
        assert!(!root.has_text);
    }

    #[test]
    fn malformed_files_are_refused_with_their_line() {
        let error = parse("<svg>\n<path d=\"M0 0\">\n</svg>").unwrap_err();
        assert_eq!(error.line, 3);
        assert!(error.message.contains("closed by"), "{error}");
        assert!(parse("<svg a=1/>").is_err());
        assert!(parse("<svg a=\"1\" a=\"2\"/>").is_err());
        assert!(parse("<svg/><svg/>").is_err());
        assert!(parse("<!DOCTYPE svg [<!ENTITY x \"y\">]><svg/>").is_err());
        assert!(parse("<svg>text</svg>").unwrap().has_text);
    }
}
