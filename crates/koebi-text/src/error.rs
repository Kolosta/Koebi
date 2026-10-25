//! Errors of font reading and glyph rasterization.

use std::fmt;

use crate::read::Tag;

/// Why a font cannot be read, or a glyph drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The data is not a TrueType or OpenType font.
    NotAFont,
    /// A table the font needs is missing.
    MissingTable(Tag),
    /// A table is truncated or holds inconsistent values.
    Malformed {
        /// The table.
        table: Tag,
        /// What is wrong.
        reason: &'static str,
    },
    /// The font uses something Koebi does not read yet.
    Unsupported(&'static str),
    /// A glyph is out of the font's range.
    UnknownGlyph(u16),
    /// A glyph's outline or bitmap exceeds the limits that protect memory.
    TooComplex,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAFont => f.write_str("not a TrueType or OpenType font"),
            Self::MissingTable(tag) => write!(f, "the font has no {tag} table"),
            Self::Malformed { table, reason } => write!(f, "malformed {table} table: {reason}"),
            Self::Unsupported(what) => write!(f, "unsupported font: {what}"),
            Self::UnknownGlyph(glyph) => write!(f, "glyph {glyph} is not in the font"),
            Self::TooComplex => f.write_str("the glyph is too complex or too large to draw"),
        }
    }
}

impl std::error::Error for Error {}

/// Result of text operations.
pub type Result<T> = std::result::Result<T, Error>;
