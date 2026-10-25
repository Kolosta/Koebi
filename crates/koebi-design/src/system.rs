//! [`DesignSystem`]: the state of the design system and its public API.

use crate::active::Active;
use crate::error::{Error, Result};
use crate::graph::{Change, Graph, ThemeSlot};
use crate::kind::Kind;
use crate::layers::{Layers, Origin, Overlay};
use crate::resolve::Resolve;
use crate::schema::{Schema, TokenInfo};
use crate::theme::{self, NewTheme, Theme, ThemeId, ThemeOrigin};
use crate::tier::{MayAlias, Tier, TierId};
use crate::token::{TextStyle, Token, TokenId, Typography};
use crate::value::{Definition, Range, Value};
use crate::view::View;
use crate::vision::ColorVision;

/// Where an override applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    /// Every theme, on top of their own overrides.
    Global,
    /// The theme active when the override is set.
    ActiveTheme,
    /// One theme, active or not.
    Theme(ThemeId),
}

/// The design system: token definitions in every layer, the active theme, the color vision
/// setting, and the resolved value of every token in the active theme.
///
/// Reading a token ([`get`](Self::get)) is an array access. Changes (theme switch, override,
/// mask, color vision) are validated, applied, then propagated to every token that depends on
/// the changed one before the call returns, so the next frame sees a consistent state.
#[derive(Debug)]
pub struct DesignSystem {
    graph: Graph,
    active: Active,
    revision: u64,
}

impl DesignSystem {
    /// Creates the design system of `schema`, on its base theme.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidSchema`] when the schema breaks a rule (schemas checked with
    /// [`Schema::assert_valid`] never do).
    pub fn new(schema: &'static Schema) -> Result<Self> {
        let graph = Graph::new(schema)?;
        let active = Active::new(&graph);
        Ok(Self {
            graph,
            active,
            revision: 0,
        })
    }

    /// The schema.
    pub fn schema(&self) -> &'static Schema {
        self.graph.schema
    }

    /// A counter incremented by every successful change: compare it with a saved value to
    /// know whether to redraw or to save.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    // --- Reading ----------------------------------------------------------------------------

    /// The value of `token` in the active theme, with every layer.
    ///
    /// A token of another schema reads as the kind's fallback (magenta for colors).
    #[inline]
    pub fn get<K: Kind, T: Tier>(&self, token: Token<K, T>) -> K::Value {
        let value = self.active.value(token.id()).and_then(K::from_value);
        debug_assert!(
            value.is_some(),
            "{} is not a token of this schema",
            token.id()
        );
        value.unwrap_or_else(K::fallback)
    }

    /// The value of a token in the active theme, untyped.
    pub fn value(&self, token: impl Into<TokenId>) -> Option<&Value> {
        self.active.value(token.into())
    }

    /// The resolved text style of a typography composite in the active theme.
    pub fn typography<T: Tier>(&self, style: Typography<T>) -> TextStyle {
        Resolve::typography(self, style)
    }

    /// A view on any theme with any set of layers: original values ([`Layers::NONE`] or
    /// [`Layers::THEME`]), one layer of overrides, other themes for preview cards...
    /// `None` for a removed theme.
    pub fn view(&self, theme: ThemeId, layers: Layers) -> Option<View<'_>> {
        self.graph.slot(theme)?;
        Some(View::new(&self.graph, &self.active, theme, layers))
    }

    /// The static description of a token.
    pub fn info(&self, token: impl Into<TokenId>) -> Option<&'static TokenInfo> {
        self.graph.info(token.into())
    }

    /// Finds a token by its full path, such as `semantic.background.accent.default`.
    pub fn find(&self, path: &str) -> Option<TokenId> {
        let (tier, path) = path.split_once('.')?;
        self.graph
            .paths
            .get(&(TierId::from_name(tier)?, path))
            .copied()
    }

    /// The valid range of a numeric token: its own, or the one of the token its base
    /// definition aliases. Durations are in milliseconds.
    pub fn range(&self, token: impl Into<TokenId>) -> Option<Range> {
        self.graph
            .ranges
            .get(token.into().index())
            .copied()
            .flatten()
    }

    /// Which layer defines `token` in the active theme.
    pub fn origin(&self, token: impl Into<TokenId>) -> Origin {
        View::new(&self.graph, &self.active, self.graph.active, Layers::ALL).origin(token.into())
    }

    // --- Themes -----------------------------------------------------------------------------

    /// Every theme, base theme first.
    pub fn themes(&self) -> impl Iterator<Item = (ThemeId, &Theme)> + '_ {
        self.graph
            .theme_ids()
            .filter_map(|id| Some((id, &self.graph.slot(id)?.theme)))
    }

    /// The identity of a theme.
    pub fn theme(&self, theme: ThemeId) -> Option<&Theme> {
        self.graph.slot(theme).map(|slot| &slot.theme)
    }

    /// Finds a theme by key.
    pub fn find_theme(&self, key: &str) -> Option<ThemeId> {
        self.themes()
            .find(|(_, theme)| theme.key() == key)
            .map(|(id, _)| id)
    }

    /// The active theme.
    pub fn active_theme(&self) -> ThemeId {
        self.graph.active
    }

    /// Switches the active theme. Every token is resolved again.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownTheme`] for a removed or foreign theme.
    pub fn set_active_theme(&mut self, theme: ThemeId) -> Result<()> {
        self.slot(theme)?;
        if theme != self.graph.active {
            self.graph.active = theme;
            self.active.rebuild(&self.graph);
            self.revision += 1;
        }
        Ok(())
    }

    /// The definitions of a theme, on top of the base theme (empty for the base theme).
    pub fn theme_definitions(
        &self,
        theme: ThemeId,
    ) -> Option<impl Iterator<Item = (TokenId, &Definition)>> {
        self.graph.slot(theme).map(|slot| slot.definition.iter())
    }

    /// Creates a theme at run time, for instance from an imported file.
    ///
    /// # Errors
    ///
    /// When the key is invalid or taken, a definition is invalid (unknown token, wrong kind,
    /// tier violation, value out of range), a token is defined twice, or the definitions close
    /// an alias cycle. Nothing is created on error.
    pub fn create_theme(&mut self, new: NewTheme) -> Result<ThemeId> {
        theme::check_key(&new.key)?;
        if self.find_theme(&new.key).is_some() {
            return Err(Error::DuplicateTheme(new.key));
        }
        let index = u16::try_from(self.graph.themes.len())
            .map_err(|_| Error::DuplicateTheme(new.key.clone()))?;
        let len = self.graph.len();
        let mut definition = Overlay::new(len);
        for (token, entry) in new.definitions {
            let entry = self.graph.check_definition(token, entry)?;
            if definition.set(token, entry).is_some() {
                return Err(Error::InvalidValue {
                    token: self.graph.name(token),
                    reason: "defined twice by the theme".to_owned(),
                });
            }
        }
        let slot = ThemeSlot {
            theme: Theme::new(&new.key, &new.name, &new.description, ThemeOrigin::User),
            definition,
            overrides: Overlay::new(len),
        };
        self.graph.check_slot(&slot)?;
        self.graph.themes.push(Some(slot));
        self.revision += 1;
        Ok(ThemeId::from_index(index))
    }

    /// Removes a theme created at run time, with its overrides.
    ///
    /// # Errors
    ///
    /// [`Error::BuiltInTheme`] for themes of the schema, [`Error::ActiveTheme`] for the
    /// active theme, [`Error::UnknownTheme`] for a theme already removed.
    pub fn remove_theme(&mut self, theme: ThemeId) -> Result<()> {
        let slot = self.slot(theme)?;
        if slot.theme.origin() != ThemeOrigin::User {
            return Err(Error::BuiltInTheme(slot.theme.key().to_owned()));
        }
        if theme == self.graph.active {
            return Err(Error::ActiveTheme(slot.theme.key().to_owned()));
        }
        self.graph.themes[theme.index()] = None;
        self.revision += 1;
        Ok(())
    }

    // --- Overrides --------------------------------------------------------------------------

    /// Overrides `token` with a value in `scope`.
    ///
    /// # Errors
    ///
    /// When the value is invalid or out of the token's [range](Self::range), or the scope
    /// names an unknown theme.
    pub fn set_value<K: Kind, T: Tier>(
        &mut self,
        scope: Scope,
        token: Token<K, T>,
        value: K::Value,
    ) -> Result<()> {
        self.set_override(scope, token.id(), Definition::Value(K::into_value(value)))
    }

    /// Overrides `token` with an alias to `target` in `scope`. Kinds and tiers are checked at
    /// compile time, like in token tables.
    ///
    /// # Errors
    ///
    /// [`Error::Cycle`] when the alias would loop back to `token` in an affected theme, or the
    /// scope names an unknown theme.
    pub fn set_alias<K: Kind, T, U>(
        &mut self,
        scope: Scope,
        token: Token<K, T>,
        target: Token<K, U>,
    ) -> Result<()>
    where
        T: MayAlias<U>,
        U: Tier,
    {
        self.set_override(scope, token.id(), Definition::Alias(target.id()))
    }

    /// Overrides a token in `scope`, untyped: kinds and tiers are checked at run time.
    ///
    /// # Errors
    ///
    /// When the definition is invalid (unknown token, wrong kind, tier violation, value out
    /// of range), would close an alias cycle, or the scope names an unknown theme.
    pub fn set_override(
        &mut self,
        scope: Scope,
        token: TokenId,
        definition: Definition,
    ) -> Result<()> {
        let definition = self.graph.check_definition(token, definition)?;
        let (layer, theme) = self.target(scope)?;
        if definition.alias_target().is_some() {
            self.graph.check_change(&Change {
                token,
                layer,
                theme,
                definition: Some(&definition),
            })?;
        }
        let structural = definition.alias_target().is_some();
        let previous = self.overlay_mut(layer, theme)?.set(token, definition);
        let structural = structural || previous.is_some_and(|p| p.alias_target().is_some());
        self.applied(theme, token, structural);
        Ok(())
    }

    /// Validates a definition for `token` without applying it: kind, value range and tier
    /// rules. Cycles depend on where the definition goes; [`Self::set_override`] and
    /// [`Self::create_theme`] check them.
    ///
    /// # Errors
    ///
    /// The error [`Self::set_override`] would return for those reasons.
    pub fn check_definition(&self, token: TokenId, definition: Definition) -> Result<Definition> {
        self.graph.check_definition(token, definition)
    }

    /// Removes the override of `token` in `scope`. Returns `false` when there was none.
    ///
    /// # Errors
    ///
    /// [`Error::Cycle`] when the definition revealed below would close an alias cycle (remove
    /// the other override first), or the scope names an unknown theme.
    pub fn reset(&mut self, scope: Scope, token: impl Into<TokenId>) -> Result<bool> {
        let token = token.into();
        if self.graph.info(token).is_none() {
            return Err(Error::UnknownToken(token.to_string()));
        }
        let (layer, theme) = self.target(scope)?;
        if self.overlay(layer, theme)?.get(token).is_none() {
            return Ok(false);
        }
        self.graph.check_change(&Change {
            token,
            layer,
            theme,
            definition: None,
        })?;
        let previous = self.overlay_mut(layer, theme)?.remove(token);
        let structural = previous.is_some_and(|p| p.alias_target().is_some());
        self.applied(theme, token, structural);
        Ok(true)
    }

    /// Removes every override of `scope`.
    ///
    /// # Errors
    ///
    /// When the scope names an unknown theme.
    pub fn reset_scope(&mut self, scope: Scope) -> Result<()> {
        let (layer, theme) = self.target(scope)?;
        let overlay = self.overlay_mut(layer, theme)?;
        if overlay.is_empty() {
            return Ok(());
        }
        overlay.clear();
        self.revision += 1;
        if theme.is_none_or(|theme| theme == self.graph.active) {
            self.active.rebuild(&self.graph);
        }
        Ok(())
    }

    /// The override of `token` in `scope`, if any.
    pub fn override_of(&self, scope: Scope, token: impl Into<TokenId>) -> Option<&Definition> {
        let (layer, theme) = self.target(scope).ok()?;
        self.overlay(layer, theme).ok()?.get(token.into())
    }

    /// Every override of `scope`, in no particular order.
    ///
    /// # Errors
    ///
    /// When the scope names an unknown theme.
    pub fn overrides(&self, scope: Scope) -> Result<impl Iterator<Item = (TokenId, &Definition)>> {
        let (layer, theme) = self.target(scope)?;
        Ok(self.overlay(layer, theme)?.iter())
    }

    // --- Layer masks ------------------------------------------------------------------------

    /// Restricts the layers that apply to `token` and to everything it aliases. The default is
    /// [`Layers::ALL`]. For instance, a logo color without [`Layers::COLOR_VISION`] is never
    /// color-transformed; a token without [`Layers::OVERRIDES`] ignores the user's changes;
    /// with [`Layers::NONE`] it keeps its base value in every theme.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownToken`] for a token of another schema.
    pub fn set_token_layers(&mut self, token: impl Into<TokenId>, layers: Layers) -> Result<()> {
        let token = token.into();
        let mask = self
            .graph
            .masks
            .get_mut(token.index())
            .ok_or_else(|| Error::UnknownToken(token.to_string()))?;
        if *mask != layers {
            *mask = layers;
            self.revision += 1;
            self.active.refresh_from(&self.graph, token);
        }
        Ok(())
    }

    /// The layers that apply to `token` (see [`Self::set_token_layers`]).
    pub fn token_layers(&self, token: impl Into<TokenId>) -> Layers {
        self.graph
            .masks
            .get(token.into().index())
            .copied()
            .unwrap_or(Layers::ALL)
    }

    // --- Color vision -----------------------------------------------------------------------

    /// The color vision setting.
    pub fn color_vision(&self) -> ColorVision {
        self.graph.vision
    }

    /// Changes the color vision setting. Every color token is resolved again.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidColorVision`] for an amount outside `0..=1` or a non-finite matrix.
    pub fn set_color_vision(&mut self, vision: ColorVision) -> Result<()> {
        vision.validate()?;
        if vision != self.graph.vision {
            self.graph.vision = vision;
            self.graph.matrix = vision.matrix();
            self.active.refresh_all(&self.graph);
            self.revision += 1;
        }
        Ok(())
    }

    /// The token graph, for the modules of this crate (export reads raw layers).
    pub(crate) fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Number of tokens resolved again by the last change. Exposed for tests and profiling.
    #[doc(hidden)]
    pub fn last_refresh_len(&self) -> usize {
        self.active.refreshed()
    }

    // --- Internals --------------------------------------------------------------------------

    fn slot(&self, theme: ThemeId) -> Result<&ThemeSlot> {
        self.graph
            .slot(theme)
            .ok_or_else(|| Error::UnknownTheme(theme.to_string()))
    }

    /// The layer and theme of a scope.
    fn target(&self, scope: Scope) -> Result<(Layers, Option<ThemeId>)> {
        match scope {
            Scope::Global => Ok((Layers::GLOBAL_OVERRIDES, None)),
            Scope::ActiveTheme => Ok((Layers::THEME_OVERRIDES, Some(self.graph.active))),
            Scope::Theme(theme) => {
                self.slot(theme)?;
                Ok((Layers::THEME_OVERRIDES, Some(theme)))
            }
        }
    }

    fn overlay(&self, layer: Layers, theme: Option<ThemeId>) -> Result<&Overlay> {
        match theme {
            None => Ok(&self.graph.global),
            Some(theme) => {
                let slot = self.slot(theme)?;
                Ok(if layer == Layers::THEME {
                    &slot.definition
                } else {
                    &slot.overrides
                })
            }
        }
    }

    fn overlay_mut(&mut self, layer: Layers, theme: Option<ThemeId>) -> Result<&mut Overlay> {
        match theme {
            None => Ok(&mut self.graph.global),
            Some(theme) => {
                let slot = self
                    .graph
                    .slot_mut(theme)
                    .ok_or_else(|| Error::UnknownTheme(theme.to_string()))?;
                Ok(if layer == Layers::THEME {
                    &mut slot.definition
                } else {
                    &mut slot.overrides
                })
            }
        }
    }

    /// Propagates an applied change to the active view.
    fn applied(&mut self, theme: Option<ThemeId>, token: TokenId, structural: bool) {
        self.revision += 1;
        if theme.is_none_or(|theme| theme == self.graph.active) {
            if structural {
                self.active.rebuild_dependents(&self.graph);
            }
            self.active.refresh_from(&self.graph, token);
        }
    }
}
