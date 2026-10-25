//! Token handles.
//!
//! A [`Token<K, T>`] is a `Copy` handle of two bytes: the token's position in its schema, plus
//! its kind `K` and tier `T` in the type. Token tables define one constant per token, so tokens
//! are real Rust items with autocompletion and documentation, never strings:
//!
//! ```text
//! let background = design.get(semantic::BACKGROUND_ACCENT_DEFAULT); // koebi_core::Color
//! ```

use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use std::sync::Arc;

use crate::kind;
use crate::value::FontWeight;

/// An untyped token handle: the token's position in its schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TokenId(u16);

impl TokenId {
    /// The token at `index` in its schema.
    pub const fn from_index(index: u16) -> Self {
        Self(index)
    }

    /// The position of the token in its schema.
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// The position as stored.
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl fmt::Display for TokenId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// A typed handle to a design token of kind `K` (a [`kind`] marker) and tier `T` (a
/// [`tier`](crate::tier) marker).
///
/// Token tables create these handles; read them with
/// [`DesignSystem::get`](crate::DesignSystem::get).
#[repr(transparent)]
pub struct Token<K, T> {
    id: TokenId,
    _marker: PhantomData<fn() -> (K, T)>,
}

impl<K, T> Token<K, T> {
    /// The handle of the token at `index` in its schema. Used by the token table macros.
    #[doc(hidden)]
    pub const fn from_index(index: u16) -> Self {
        Self {
            id: TokenId::from_index(index),
            _marker: PhantomData,
        }
    }

    /// The untyped handle.
    pub const fn id(self) -> TokenId {
        self.id
    }
}

impl<K, T> Clone for Token<K, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K, T> Copy for Token<K, T> {}

impl<K, T> PartialEq for Token<K, T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<K, T> Eq for Token<K, T> {}

impl<K, T> Hash for Token<K, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<K, T> fmt::Debug for Token<K, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Token({})", self.id)
    }
}

impl<K, T> From<Token<K, T>> for TokenId {
    fn from(token: Token<K, T>) -> Self {
        token.id
    }
}

/// A typography composite: the five field tokens of a text style.
///
/// Each field is a token of its own (`text.body.m.font-size`...), so a theme or an override can
/// change one property of one style. Read the whole style with
/// [`DesignSystem::typography`](crate::DesignSystem::typography).
pub struct Typography<T> {
    /// Font family.
    pub font_family: Token<kind::FontFamily, T>,
    /// Font size, in logical pixels.
    pub font_size: Token<kind::Dimension, T>,
    /// Font weight.
    pub font_weight: Token<kind::FontWeight, T>,
    /// Line height, as a multiple of the font size.
    pub line_height: Token<kind::Number, T>,
    /// Letter spacing, in em.
    pub letter_spacing: Token<kind::Number, T>,
}

impl<T> Typography<T> {
    /// Number of field tokens of a typography composite.
    pub const FIELDS: u16 = 5;

    /// The composite whose fields start at `first`. Used by the token table macros.
    #[doc(hidden)]
    pub const fn from_first(first: u16) -> Self {
        Self {
            font_family: Token::from_index(first),
            font_size: Token::from_index(first + 1),
            font_weight: Token::from_index(first + 2),
            line_height: Token::from_index(first + 3),
            letter_spacing: Token::from_index(first + 4),
        }
    }
}

impl<T> Clone for Typography<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Typography<T> {}

impl<T> fmt::Debug for Typography<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Typography({})", self.font_family.id())
    }
}

/// A resolved text style.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    /// Font family name.
    pub font_family: Arc<str>,
    /// Font size, in logical pixels.
    pub font_size: f32,
    /// Font weight.
    pub font_weight: FontWeight,
    /// Line height, as a multiple of the font size.
    pub line_height: f32,
    /// Letter spacing, in em.
    pub letter_spacing: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tier;

    #[test]
    fn handles_are_two_bytes_and_compare_by_position() {
        assert_eq!(size_of::<Token<kind::Color, tier::Semantic>>(), 2);
        let a: Token<kind::Color, tier::Semantic> = Token::from_index(3);
        assert_eq!(a, Token::from_index(3));
        assert_eq!(TokenId::from(a).index(), 3);
    }

    #[test]
    fn typography_fields_are_consecutive() {
        let style: Typography<tier::Semantic> = Typography::from_first(10);
        assert_eq!(style.font_family.id().index(), 10);
        assert_eq!(style.letter_spacing.id().index(), 14);
    }
}
