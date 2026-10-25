//! Themes at run time.
//!
//! A theme is a set of definitions on top of the base theme: the base theme defines every
//! token, a theme only what it changes. The base theme and the themes written in code come
//! from the schema; more themes can be created at run time (imported files, user themes).
//! Every theme has its own layer of user overrides.

use core::fmt;

use crate::check;
use crate::error::{Error, Result};
use crate::token::TokenId;
use crate::value::Definition;

/// A theme handle, stable for the lifetime of a [`DesignSystem`](crate::DesignSystem).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ThemeId(u16);

impl ThemeId {
    /// The base theme.
    pub const BASE: Self = Self(0);

    pub(crate) const fn from_index(index: u16) -> Self {
        Self(index)
    }

    /// Position of the theme in the design system.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for ThemeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "theme #{}", self.0)
    }
}

/// Where a theme comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThemeOrigin {
    /// The base theme of the schema.
    Base,
    /// A theme written in code.
    BuiltIn,
    /// A theme created at run time (imported or created by the user).
    User,
}

/// The identity of a theme.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    key: String,
    name: String,
    description: String,
    origin: ThemeOrigin,
}

impl Theme {
    pub(crate) fn new(key: &str, name: &str, description: &str, origin: ThemeOrigin) -> Self {
        Self {
            key: key.to_owned(),
            name: name.to_owned(),
            description: description.to_owned(),
            origin,
        }
    }

    /// Stable identifier used by persistence and import (`light`, `high-contrast`).
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// What the theme is for.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Where the theme comes from.
    pub const fn origin(&self) -> ThemeOrigin {
        self.origin
    }
}

/// A theme to create at run time with
/// [`DesignSystem::create_theme`](crate::DesignSystem::create_theme).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NewTheme {
    /// Stable identifier: 1 to 64 characters among `a-z`, `0-9`, `-` and `.`.
    pub key: String,
    /// Display name.
    pub name: String,
    /// What the theme is for.
    pub description: String,
    /// The definitions the theme changes, on top of the base theme.
    pub definitions: Vec<(TokenId, Definition)>,
}

impl NewTheme {
    /// A theme without definitions yet.
    pub fn new(key: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            ..Self::default()
        }
    }

    /// Adds a definition.
    #[must_use]
    pub fn with(mut self, token: impl Into<TokenId>, definition: Definition) -> Self {
        self.definitions.push((token.into(), definition));
        self
    }
}

pub(crate) fn check_key(key: &str) -> Result<()> {
    match check::theme_key_problem(key) {
        None => Ok(()),
        Some(reason) => Err(Error::InvalidThemeKey {
            key: key.to_owned(),
            reason,
        }),
    }
}
