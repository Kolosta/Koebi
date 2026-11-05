//! What goes wrong compiling an icon: a message, and the line of the SVG file it concerns.

use std::fmt;

/// An icon that cannot be compiled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    /// The line of the SVG file, from 1; 0 when the whole file is concerned.
    pub line: usize,
    /// What is wrong, and how to put it right when the reason is known.
    pub message: String,
}

impl Error {
    pub(crate) fn at(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            f.write_str(&self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for Error {}

/// The results of the compiler.
pub type Result<T> = core::result::Result<T, Error>;
