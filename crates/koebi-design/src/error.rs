//! Errors of the design system.

use core::fmt;

use crate::kind::KindId;
use crate::value::Range;

/// An error of the design system. Nothing is changed when an operation fails.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// The schema breaks a rule. Schemas checked at compile time never do.
    InvalidSchema(Vec<String>),
    /// No token has this handle or path.
    UnknownToken(String),
    /// No theme has this handle or key.
    UnknownTheme(String),
    /// Another theme already has this key.
    DuplicateTheme(String),
    /// A theme key breaks the naming rules.
    InvalidThemeKey {
        /// The key.
        key: String,
        /// Which rule.
        reason: &'static str,
    },
    /// The active theme cannot be removed.
    ActiveTheme(String),
    /// Themes of the schema cannot be removed or redefined.
    BuiltInTheme(String),
    /// A value or alias target of another kind.
    KindMismatch {
        /// The token being defined.
        token: String,
        /// Its kind.
        expected: KindId,
        /// The kind received.
        found: KindId,
    },
    /// An alias breaks the tier rules.
    TierViolation {
        /// The token being defined.
        token: String,
        /// The alias target.
        target: String,
    },
    /// A value is not valid for its kind (not finite, empty font family...).
    InvalidValue {
        /// The token being defined.
        token: String,
        /// Why.
        reason: String,
    },
    /// A value outside the token's range.
    OutOfRange {
        /// The token being defined.
        token: String,
        /// The value received (durations in milliseconds, the alpha of colors).
        value: f64,
        /// The values the token accepts.
        range: Range,
    },
    /// The change would make a chain of aliases loop back to itself.
    Cycle {
        /// The token being changed.
        token: String,
    },
    /// An invalid color vision setting.
    InvalidColorVision(&'static str),
    /// A file operation failed.
    Io {
        /// What was being done (`read`, `write`, `rename`...).
        operation: &'static str,
        /// The file.
        path: String,
        /// The system's message.
        message: String,
    },
    /// Saved data is damaged: wrong signature, checksum or structure.
    Corrupt(String),
    /// Saved data comes from a newer version of the format.
    UnsupportedVersion(u16),
    /// Saved data belongs to another token schema.
    ForeignSchema(String),
    /// An imported file has problems; every one is listed, located in the file.
    Import(Vec<crate::dtcg::ImportIssue>),
}

/// Result type of the design system.
pub type Result<T, E = Error> = core::result::Result<T, E>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSchema(problems) => {
                write!(f, "invalid token schema: {}", problems.join("; "))
            }
            Self::UnknownToken(token) => write!(f, "unknown token {token}"),
            Self::UnknownTheme(theme) => write!(f, "unknown theme {theme}"),
            Self::DuplicateTheme(key) => write!(f, "a theme already has the key {key:?}"),
            Self::InvalidThemeKey { key, reason } => {
                write!(f, "invalid theme key {key:?}: {reason}")
            }
            Self::ActiveTheme(key) => write!(f, "theme {key:?} is active and cannot be removed"),
            Self::BuiltInTheme(key) => {
                write!(
                    f,
                    "theme {key:?} is part of the schema and cannot be changed"
                )
            }
            Self::KindMismatch {
                token,
                expected,
                found,
            } => write!(f, "{token} holds a {expected}, not a {found}"),
            Self::TierViolation { token, target } => {
                write!(f, "{token} cannot alias {target}: the tier rules forbid it")
            }
            Self::InvalidValue { token, reason } => {
                write!(f, "invalid value for {token}: {reason}")
            }
            Self::OutOfRange {
                token,
                value,
                range,
            } => write!(f, "{value} is outside the range of {token} ({range})"),
            Self::Cycle { token } => write!(f, "changing {token} would create an alias cycle"),
            Self::InvalidColorVision(reason) => write!(f, "invalid color vision setting: {reason}"),
            Self::Io {
                operation,
                path,
                message,
            } => write!(f, "cannot {operation} {path}: {message}"),
            Self::Corrupt(reason) => write!(f, "damaged design data: {reason}"),
            Self::UnsupportedVersion(version) => {
                write!(
                    f,
                    "design data of format version {version}, newer than this build"
                )
            }
            Self::Import(issues) => {
                f.write_str("invalid design tokens file:")?;
                for issue in issues {
                    write!(
                        f,
                        "
  {issue}"
                    )?;
                }
                Ok(())
            }
            Self::ForeignSchema(name) => {
                write!(f, "design data saved for the token schema {name:?}")
            }
        }
    }
}

impl std::error::Error for Error {}
