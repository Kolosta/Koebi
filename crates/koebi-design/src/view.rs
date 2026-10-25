//! [`View`]: token values in any theme with any set of layers.

use crate::active::Active;
use crate::graph::Graph;
use crate::kind::Kind;
use crate::layers::{Layers, Origin};
use crate::theme::ThemeId;
use crate::tier::Tier;
use crate::token::{TextStyle, Token, TokenId, Typography};
use crate::value::{Definition, Value};

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
        TextStyle {
            font_family: self.get(style.font_family),
            font_size: self.get(style.font_size),
            font_weight: self.get(style.font_weight),
            line_height: self.get(style.line_height),
            letter_spacing: self.get(style.letter_spacing),
        }
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
