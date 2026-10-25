//! Definition layers, layer sets and per-token layer masks.
//!
//! A token's value comes from the topmost layer that defines it:
//!
//! ```text
//! color vision        transforms resolved colors (not a definition layer)
//! global overrides    the user's changes, whatever the theme
//! theme overrides     the user's changes to one theme
//! theme               the theme's own definitions (written in code or imported)
//! base theme          the token tables: every token is defined here
//! ```
//!
//! A [`Layers`] set selects which layers take part: [`Layers::ALL`] is what the application
//! shows, [`Layers::NONE`] the base theme only, [`Layers::THEME`] a theme as designed. The
//! same set doubles as a per-token mask: a token whose mask lacks
//! [`Layers::COLOR_VISION`] is never color-transformed, one whose mask lacks the override
//! layers ignores the user's changes.

use core::fmt;
use core::ops::{BitAnd, BitOr, Not, Sub};

use crate::token::TokenId;
use crate::value::Definition;

/// A set of layers.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Layers(u8);

impl Layers {
    /// No layer: the base theme only, without color transformation.
    pub const NONE: Self = Self(0);
    /// The theme's own definitions.
    pub const THEME: Self = Self(1);
    /// The user's overrides of the theme.
    pub const THEME_OVERRIDES: Self = Self(1 << 1);
    /// The user's overrides that apply to every theme.
    pub const GLOBAL_OVERRIDES: Self = Self(1 << 2);
    /// The color vision transformation.
    pub const COLOR_VISION: Self = Self(1 << 3);
    /// Both override layers.
    pub const OVERRIDES: Self = Self(Self::THEME_OVERRIDES.0 | Self::GLOBAL_OVERRIDES.0);
    /// The definition layers above the base theme.
    pub const DEFINITIONS: Self = Self(Self::THEME.0 | Self::OVERRIDES.0);
    /// Every layer: what the application shows.
    pub const ALL: Self = Self(Self::DEFINITIONS.0 | Self::COLOR_VISION.0);

    /// The raw bits.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// The set with these bits; `None` if a bit names no layer.
    pub const fn from_bits(bits: u8) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// `true` when every layer of `other` is in `self`.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// `true` when no layer is selected.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The layers of both sets.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// The layers common to both sets.
    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// The layers of `self` that are not in `other`.
    #[must_use]
    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }
}

impl Default for Layers {
    /// [`Layers::ALL`].
    fn default() -> Self {
        Self::ALL
    }
}

impl BitOr for Layers {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

impl BitAnd for Layers {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        self.intersection(rhs)
    }
}

impl Sub for Layers {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self.difference(rhs)
    }
}

impl Not for Layers {
    type Output = Self;
    /// The other layers.
    fn not(self) -> Self {
        Self::ALL.difference(self)
    }
}

impl fmt::Debug for Layers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const NAMES: [(Layers, &str); 4] = [
            (Layers::THEME, "THEME"),
            (Layers::THEME_OVERRIDES, "THEME_OVERRIDES"),
            (Layers::GLOBAL_OVERRIDES, "GLOBAL_OVERRIDES"),
            (Layers::COLOR_VISION, "COLOR_VISION"),
        ];
        if self.is_empty() {
            return f.write_str("NONE");
        }
        let mut first = true;
        for (layer, name) in NAMES {
            if self.contains(layer) {
                if !first {
                    f.write_str(" | ")?;
                }
                f.write_str(name)?;
                first = false;
            }
        }
        Ok(())
    }
}

/// The layer that provided a token's definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Origin {
    /// The token tables.
    Base,
    /// The theme's own definitions.
    Theme,
    /// An override of the theme.
    ThemeOverride,
    /// A global override.
    GlobalOverride,
}

impl Origin {
    /// `true` for the user's overrides.
    pub const fn is_override(self) -> bool {
        matches!(self, Self::ThemeOverride | Self::GlobalOverride)
    }
}

/// A sparse layer of definitions with constant-time lookup: a slot per token, pointing into a
/// compact list of entries.
#[derive(Clone, Debug)]
pub(crate) struct Overlay {
    slots: Box<[u32]>,
    entries: Vec<(TokenId, Definition)>,
}

const EMPTY: u32 = u32::MAX;

impl Overlay {
    pub(crate) fn new(tokens: usize) -> Self {
        Self {
            slots: vec![EMPTY; tokens].into_boxed_slice(),
            entries: Vec::new(),
        }
    }

    pub(crate) fn get(&self, token: TokenId) -> Option<&Definition> {
        let slot = *self.slots.get(token.index())?;
        self.entries
            .get(usize::try_from(slot).ok()?)
            .map(|(_, definition)| definition)
    }

    /// Defines `token`, returning its previous definition.
    pub(crate) fn set(&mut self, token: TokenId, definition: Definition) -> Option<Definition> {
        let index = token.index();
        let slot = self.slots[index];
        if slot == EMPTY {
            self.slots[index] = u32::try_from(self.entries.len()).unwrap_or(EMPTY);
            self.entries.push((token, definition));
            None
        } else {
            let entry = &mut self.entries[slot as usize];
            Some(core::mem::replace(&mut entry.1, definition))
        }
    }

    /// Removes the definition of `token`, returning it.
    pub(crate) fn remove(&mut self, token: TokenId) -> Option<Definition> {
        let index = token.index();
        let slot = *self.slots.get(index)?;
        if slot == EMPTY {
            return None;
        }
        self.slots[index] = EMPTY;
        let (_, definition) = self.entries.swap_remove(slot as usize);
        if let Some((moved, _)) = self.entries.get(slot as usize) {
            self.slots[moved.index()] = slot;
        }
        Some(definition)
    }

    pub(crate) fn clear(&mut self) {
        for (token, _) in self.entries.drain(..) {
            self.slots[token.index()] = EMPTY;
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (TokenId, &Definition)> {
        self.entries
            .iter()
            .map(|(token, definition)| (*token, definition))
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    fn id(index: u16) -> TokenId {
        TokenId::from_index(index)
    }

    #[test]
    fn layers_combine_like_sets() {
        let user = Layers::THEME_OVERRIDES | Layers::GLOBAL_OVERRIDES;
        assert_eq!(user, Layers::OVERRIDES);
        assert!(Layers::ALL.contains(user));
        assert_eq!(Layers::ALL - Layers::COLOR_VISION, Layers::DEFINITIONS);
        assert_eq!(!Layers::COLOR_VISION, Layers::DEFINITIONS);
        assert_eq!(Layers::from_bits(0x10), None);
        assert_eq!(
            format!("{:?}", Layers::THEME | Layers::COLOR_VISION),
            "THEME | COLOR_VISION"
        );
        assert_eq!(format!("{:?}", Layers::NONE), "NONE");
    }

    #[test]
    fn overlay_sets_replaces_and_removes() {
        let mut overlay = Overlay::new(8);
        assert_eq!(overlay.set(id(3), Definition::Alias(id(1))), None);
        assert_eq!(
            overlay.set(id(5), Definition::Value(Value::Number(1.0))),
            None
        );
        let previous = overlay.set(id(3), Definition::Alias(id(2)));
        assert_eq!(previous, Some(Definition::Alias(id(1))));
        assert_eq!(overlay.get(id(3)), Some(&Definition::Alias(id(2))));
        assert_eq!(overlay.len(), 2);

        // Removing the first entry moves the last one into its place.
        assert_eq!(overlay.remove(id(3)), Some(Definition::Alias(id(2))));
        assert_eq!(overlay.get(id(3)), None);
        assert_eq!(
            overlay.get(id(5)),
            Some(&Definition::Value(Value::Number(1.0)))
        );
        assert_eq!(overlay.remove(id(3)), None);

        overlay.clear();
        assert!(overlay.is_empty());
        assert_eq!(overlay.get(id(5)), None);
        assert_eq!(
            overlay.get(id(42)),
            None,
            "out-of-range tokens are simply absent"
        );
    }
}
