//! Token tiers and the rules about which tier may alias which.
//!
//! | Tier | Defined by | Example |
//! |---|---|---|
//! | [`Primitive`] | a value | `color.blue.700` |
//! | [`Semantic`] | an alias to a primitive or semantic token | `background.accent.default` |
//! | [`Component`] | an alias to a semantic or component token, or a value | `scrollbar.size.default` |
//!
//! The alias rules are traits ([`MayAlias`]): an alias that breaks them does not compile. Token
//! tables also check that semantic tokens never hold values. Overrides and themes may give any
//! token a value (a custom color picked by the user); their aliases follow the same rules.
//!
//! ```compile_fail
//! use koebi_design::{Token, kind, tier, value::Def};
//! const PRIMITIVE: Token<kind::Color, tier::Primitive> = Token::from_index(0);
//! // A component token cannot alias a primitive token.
//! const BROKEN: Def = Def::alias::<kind::Color, tier::Component, _>(PRIMITIVE);
//! ```

use core::fmt;

mod sealed {
    pub trait Sealed {}
}

/// A token tier. Implemented by [`Primitive`], [`Semantic`] and [`Component`] only.
pub trait Tier: sealed::Sealed + 'static {
    /// Run-time identifier of the tier.
    const ID: TierId;
}

/// Tier of tokens that hold raw values: palettes and scales. Never read by components.
#[derive(Debug)]
pub enum Primitive {}

/// Tier of tokens that name an intent and alias a primitive or another semantic token.
#[derive(Debug)]
pub enum Semantic {}

/// Tier of tokens owned by one component: an alias to a semantic or another component token,
/// or a value of their own.
#[derive(Debug)]
pub enum Component {}

impl sealed::Sealed for Primitive {}
impl sealed::Sealed for Semantic {}
impl sealed::Sealed for Component {}

impl Tier for Primitive {
    const ID: TierId = TierId::Primitive;
}

impl Tier for Semantic {
    const ID: TierId = TierId::Semantic;
}

impl Tier for Component {
    const ID: TierId = TierId::Component;
}

/// Implemented when a token of tier `Self` may alias a token of tier `T`.
#[diagnostic::on_unimplemented(
    message = "a `{Self}` token cannot alias a `{T}` token",
    label = "this alias breaks the tier rules",
    note = "semantic tokens alias primitive or semantic tokens; component tokens alias semantic \
            or component tokens; primitive tokens hold values"
)]
pub trait MayAlias<T: Tier>: Tier {}

impl MayAlias<Primitive> for Semantic {}
impl MayAlias<Semantic> for Semantic {}
impl MayAlias<Semantic> for Component {}
impl MayAlias<Component> for Component {}

/// Run-time identifier of a tier, for untyped code (persistence, import, tooling).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum TierId {
    /// See [`Primitive`].
    Primitive,
    /// See [`Semantic`].
    Semantic,
    /// See [`Component`].
    Component,
}

impl TierId {
    /// Every tier, from the bottom of the graph to the top.
    pub const ALL: [Self; 3] = [Self::Primitive, Self::Semantic, Self::Component];

    /// The lowercase name used in token paths: `primitive`, `semantic` or `component`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Primitive => "primitive",
            Self::Semantic => "semantic",
            Self::Component => "component",
        }
    }

    /// Parses [`Self::name`].
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tier| tier.name() == name)
    }

    /// `true` when a token of this tier may alias a token of `target`'s tier.
    pub const fn may_alias(self, target: Self) -> bool {
        matches!(
            (self, target),
            (Self::Semantic, Self::Primitive | Self::Semantic)
                | (Self::Component, Self::Semantic | Self::Component)
        )
    }
}

impl fmt::Display for TierId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_rules_match_the_traits() {
        use TierId::{Component, Primitive, Semantic};
        let allowed = [
            (Semantic, Primitive),
            (Semantic, Semantic),
            (Component, Semantic),
            (Component, Component),
        ];
        for from in TierId::ALL {
            for to in TierId::ALL {
                assert_eq!(
                    from.may_alias(to),
                    allowed.contains(&(from, to)),
                    "{from} -> {to}"
                );
            }
        }
    }

    #[test]
    fn tier_names_round_trip() {
        for tier in TierId::ALL {
            assert_eq!(TierId::from_name(tier.name()), Some(tier));
        }
        assert_eq!(TierId::from_name("Semantic"), None);
    }
}
