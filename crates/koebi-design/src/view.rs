//! [`View`]: token values in any theme with any set of layers.

use crate::active::Active;
use crate::graph::Graph;
use crate::kind::Kind;
use crate::layers::{Layers, Origin};
use crate::resolve::Resolve;
use crate::theme::ThemeId;
use crate::tier::Tier;
use crate::token::{TextStyle, Token, TokenId, Typography};
use crate::value::{Definition, Value};

/// The most tokens a resolution chain follows: far more than token tables nest.
const MAX_CHAIN: usize = 32;

/// What a token of a resolution chain does for the value ([`View::chain`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainRole {
    /// It takes its value from the next token.
    Passes,
    /// It holds the value.
    Source,
    /// It lies past the token holding the value: what that token would refer to without its
    /// override. It has no effect now.
    Unused,
}

/// One token of a resolution chain.
#[derive(Clone, Debug, PartialEq)]
pub struct ChainStep {
    /// The token.
    pub token: TokenId,
    /// What it does for the value.
    pub role: ChainRole,
    /// Which layer defines it.
    pub origin: Origin,
    /// Its value.
    pub value: Option<Value>,
}

/// Token values in one theme with a chosen set of layers.
///
/// Created by [`DesignSystem::view`](crate::DesignSystem::view). The active theme with every
/// layer reads the materialized values; any other view resolves tokens on demand by walking
/// their aliases (a few steps), which is cheap enough for preview cards and editors.
#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    graph: &'a Graph,
    active: &'a Active,
    theme: ThemeId,
    layers: Layers,
}

impl<'a> View<'a> {
    pub(crate) fn new(
        graph: &'a Graph,
        active: &'a Active,
        theme: ThemeId,
        layers: Layers,
    ) -> Self {
        Self {
            graph,
            active,
            theme,
            layers,
        }
    }

    /// Number of tokens of the schema.
    pub(crate) fn schema_len(&self) -> usize {
        self.graph.len()
    }

    /// The theme of the view.
    pub fn theme(&self) -> ThemeId {
        self.theme
    }

    /// The layers of the view.
    pub fn layers(&self) -> Layers {
        self.layers
    }

    fn is_active(&self) -> bool {
        self.theme == self.graph.active && self.layers == Layers::ALL
    }

    /// The value of `token` in this view.
    pub fn get<K: Kind, T: Tier>(&self, token: Token<K, T>) -> K::Value {
        let value = self.value(token.id());
        let typed = value.as_ref().and_then(K::from_value);
        debug_assert!(
            typed.is_some(),
            "{} is not a token of this schema",
            token.id()
        );
        typed.unwrap_or_else(K::fallback)
    }

    /// The value of a token in this view, untyped; `None` for a token of another schema.
    pub fn value(&self, token: impl Into<TokenId>) -> Option<Value> {
        let token = token.into();
        self.graph.info(token)?;
        if self.is_active() {
            return self.active.value(token).cloned();
        }
        Some(self.graph.value(token, self.theme, self.layers))
    }

    /// The resolved text style of a typography composite in this view.
    pub fn typography<T: Tier>(&self, style: Typography<T>) -> TextStyle {
        Resolve::typography(self, style)
    }

    /// The definition of `token` itself in this view (the topmost layer allowed by the view
    /// and the token's mask), before following aliases.
    pub fn definition(&self, token: impl Into<TokenId>) -> Option<&'a Definition> {
        let token = token.into();
        self.graph.info(token)?;
        let slot = self.graph.slot(self.theme)?;
        let layers = self.layers & self.graph.masks[token.index()];
        Some(self.graph.winning(token, self.theme, slot, layers, None).0)
    }

    /// Fills `steps` with how `token` gets its value, from `token` itself: the tokens it refers
    /// to, down to the one holding the value ([`ChainRole::Source`]). When that one holds an
    /// override where it had a reference, what it referred to follows, as far as the theme's own
    /// definitions go ([`ChainRole::Unused`]).
    pub fn chain(&self, token: impl Into<TokenId>, steps: &mut Vec<ChainStep>) {
        steps.clear();
        let mut view = *self;
        let mut role = ChainRole::Passes;
        let mut next = Some(token.into());
        while let Some(token) = next {
            if steps.len() >= MAX_CHAIN || steps.iter().any(|step| step.token == token) {
                return;
            }
            let Some(definition) = view.definition(token) else {
                return;
            };
            let target = definition.alias_target();
            let holds = target.is_none() && role == ChainRole::Passes;
            steps.push(ChainStep {
                token,
                role: if holds { ChainRole::Source } else { role },
                origin: view.origin(token),
                value: view.value(token),
            });
            next = target;
            if holds {
                // What it was before the user's overrides, which have no effect now.
                role = ChainRole::Unused;
                view.layers = view.layers.intersection(Layers::THEME);
                next = view.definition(token).and_then(Definition::alias_target);
            }
        }
    }

    /// Which layer gives `token` its definition in this view.
    pub fn origin(&self, token: impl Into<TokenId>) -> Origin {
        let token = token.into();
        let (Some(slot), Some(_)) = (self.graph.slot(self.theme), self.graph.info(token)) else {
            return Origin::Base;
        };
        let layers = self.layers & self.graph.masks[token.index()];
        self.graph.winning(token, self.theme, slot, layers, None).1
    }
}
