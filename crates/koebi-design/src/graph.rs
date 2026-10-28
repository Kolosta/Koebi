//! The token graph: every definition layer, the per-token masks, and resolution.
//!
//! Resolving a token walks its aliases: at each step the topmost layer allowed by the layer
//! set (narrowed by each token's mask) gives the definition, until a value is reached. Every
//! change is validated before it is applied, so the graph is acyclic in every combination of
//! layers and every walk ends.

use std::collections::HashMap;

use koebi_core::ColorMatrix;

use crate::error::{Error, Result};
use crate::layers::{Layers, Origin, Overlay};
use crate::schema::{Schema, TokenInfo};
use crate::theme::{Theme, ThemeId, ThemeOrigin};
use crate::tier::TierId;
use crate::token::TokenId;
use crate::value::{Definition, Range, Value};
use crate::vision::ColorVision;

/// Every combination of the definition layers above the base theme.
const COMBINATIONS: [Layers; 8] = [
    Layers::NONE,
    Layers::THEME,
    Layers::THEME_OVERRIDES,
    Layers::THEME.union(Layers::THEME_OVERRIDES),
    Layers::GLOBAL_OVERRIDES,
    Layers::THEME.union(Layers::GLOBAL_OVERRIDES),
    Layers::OVERRIDES,
    Layers::DEFINITIONS,
];

/// A theme and its two definition layers.
#[derive(Clone, Debug)]
pub(crate) struct ThemeSlot {
    pub(crate) theme: Theme,
    pub(crate) definition: Overlay,
    pub(crate) overrides: Overlay,
}

/// A pending change of one definition, used to validate it before applying it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Change<'a> {
    pub(crate) token: TokenId,
    /// [`Layers::THEME`], [`Layers::THEME_OVERRIDES`] or [`Layers::GLOBAL_OVERRIDES`].
    pub(crate) layer: Layers,
    /// The theme changed; `None` for global overrides.
    pub(crate) theme: Option<ThemeId>,
    /// The new definition; `None` removes the current one.
    pub(crate) definition: Option<&'a Definition>,
}

/// The result of walking a token's aliases.
pub(crate) struct Resolution<'a> {
    pub(crate) value: &'a Value,
    /// Whether the color vision layer applies to this value.
    pub(crate) color_vision: bool,
}

#[derive(Debug)]
pub(crate) struct Graph {
    pub(crate) schema: &'static Schema,
    pub(crate) infos: Box<[&'static TokenInfo]>,
    pub(crate) base: Box<[Definition]>,
    pub(crate) ranges: Box<[Option<Range>]>,
    pub(crate) paths: HashMap<(TierId, &'static str), TokenId>,
    pub(crate) themes: Vec<Option<ThemeSlot>>,
    pub(crate) global: Overlay,
    pub(crate) masks: Box<[Layers]>,
    pub(crate) active: ThemeId,
    pub(crate) vision: ColorVision,
    pub(crate) matrix: ColorMatrix,
}

impl Graph {
    pub(crate) fn new(schema: &'static Schema) -> Result<Self> {
        schema.validate().map_err(Error::InvalidSchema)?;
        let tokens: Vec<_> = schema.tokens().collect();
        let len = tokens.len();
        let infos: Box<[_]> = tokens.iter().map(|(_, info)| *info).collect();
        let base: Box<[_]> = infos
            .iter()
            .map(|info| Definition::from(info.definition()))
            .collect();
        let paths = tokens
            .iter()
            .map(|(id, info)| ((info.tier(), info.path()), *id))
            .collect();

        let base_info = schema.base_theme();
        let mut themes = vec![Some(ThemeSlot {
            theme: Theme::new(
                base_info.key(),
                base_info.name(),
                base_info.description(),
                ThemeOrigin::Base,
            ),
            definition: Overlay::new(len),
            overrides: Overlay::new(len),
        })];
        for info in schema.themes() {
            let mut definition = Overlay::new(len);
            for entry in info.entries() {
                definition.set(entry.token(), entry.definition().into());
            }
            themes.push(Some(ThemeSlot {
                theme: Theme::new(
                    info.key(),
                    info.name(),
                    info.description(),
                    ThemeOrigin::BuiltIn,
                ),
                definition,
                overrides: Overlay::new(len),
            }));
        }

        let mut graph = Self {
            schema,
            ranges: vec![None; len].into_boxed_slice(),
            infos,
            base,
            paths,
            themes,
            global: Overlay::new(len),
            masks: vec![Layers::ALL; len].into_boxed_slice(),
            active: ThemeId::BASE,
            vision: ColorVision::Normal,
            matrix: ColorMatrix::IDENTITY,
        };
        graph.ranges = (0..len).map(|index| graph.inherited_range(index)).collect();
        Ok(graph)
    }

    pub(crate) fn len(&self) -> usize {
        self.infos.len()
    }

    pub(crate) fn info(&self, token: TokenId) -> Option<&'static TokenInfo> {
        self.infos.get(token.index()).copied()
    }

    pub(crate) fn name(&self, token: TokenId) -> String {
        self.info(token)
            .map_or_else(|| token.to_string(), |info| info.qualified().to_string())
    }

    /// A token's own range, or the range of the token its base definition aliases.
    fn inherited_range(&self, index: usize) -> Option<Range> {
        let mut current = index;
        for _ in 0..=self.len() {
            let info = self.infos[current];
            if info.range().is_some() {
                return info.range();
            }
            match self.base[current] {
                Definition::Alias(target) => current = target.index(),
                Definition::Value(_) => return None,
            }
        }
        None
    }

    pub(crate) fn slot(&self, theme: ThemeId) -> Option<&ThemeSlot> {
        self.themes.get(theme.index()).and_then(Option::as_ref)
    }

    pub(crate) fn slot_mut(&mut self, theme: ThemeId) -> Option<&mut ThemeSlot> {
        self.themes.get_mut(theme.index()).and_then(Option::as_mut)
    }

    pub(crate) fn theme_ids(&self) -> impl Iterator<Item = ThemeId> + '_ {
        self.themes
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.is_some())
            .filter_map(|(index, _)| u16::try_from(index).ok().map(ThemeId::from_index))
    }

    pub(crate) fn active_slot(&self) -> &ThemeSlot {
        self.slot(self.active)
            .expect("the active theme always exists: it cannot be removed")
    }

    /// The definition of `token` in the topmost layer of `layers`, with an optional pending
    /// change applied.
    pub(crate) fn winning<'a>(
        &'a self,
        token: TokenId,
        theme: ThemeId,
        slot: &'a ThemeSlot,
        layers: Layers,
        change: Option<&Change<'a>>,
    ) -> (&'a Definition, Origin) {
        let pick = |layer: Layers, overlay: &'a Overlay| match change {
            Some(change)
                if change.token == token
                    && change.layer == layer
                    && (change.theme.is_none() || change.theme == Some(theme)) =>
            {
                change.definition
            }
            _ => overlay.get(token),
        };
        if layers.contains(Layers::GLOBAL_OVERRIDES)
            && let Some(definition) = pick(Layers::GLOBAL_OVERRIDES, &self.global)
        {
            return (definition, Origin::GlobalOverride);
        }
        if layers.contains(Layers::THEME_OVERRIDES)
            && let Some(definition) = pick(Layers::THEME_OVERRIDES, &slot.overrides)
        {
            return (definition, Origin::ThemeOverride);
        }
        if layers.contains(Layers::THEME)
            && let Some(definition) = pick(Layers::THEME, &slot.definition)
        {
            return (definition, Origin::Theme);
        }
        (&self.base[token.index()], Origin::Base)
    }

    /// Walks the aliases of `token` in `theme` with `layers`, narrowing the layers by each
    /// token's mask on the way.
    pub(crate) fn resolve(&self, token: TokenId, theme: ThemeId, layers: Layers) -> Resolution<'_> {
        self.try_resolve(token, theme, layers)
            .expect("validated token graphs are acyclic in every combination of layers")
    }

    fn try_resolve(
        &self,
        token: TokenId,
        theme: ThemeId,
        layers: Layers,
    ) -> Option<Resolution<'_>> {
        let slot = self.slot(theme).unwrap_or_else(|| self.active_slot());
        let mut layers = layers;
        let mut current = token;
        for _ in 0..=self.len() {
            layers = layers & self.masks[current.index()];
            match self.winning(current, theme, slot, layers, None).0 {
                Definition::Value(value) => {
                    return Some(Resolution {
                        value,
                        color_vision: layers.contains(Layers::COLOR_VISION),
                    });
                }
                Definition::Alias(next) => current = *next,
            }
        }
        None
    }

    /// The value of `token` in `theme` with `layers`, color vision applied where allowed, and
    /// kept inside the token's range: an alias to a token that accepts more cannot bring in a
    /// value this one refuses.
    pub(crate) fn value(&self, token: TokenId, theme: ThemeId, layers: Layers) -> Value {
        let resolution = self.resolve(token, theme, layers);
        let value = match resolution.value {
            Value::Color(color) if resolution.color_vision => {
                Value::Color(self.matrix.apply(*color))
            }
            value => value.clone(),
        };
        match self.ranges.get(token.index()).copied().flatten() {
            Some(range) => limited(value, range),
            None => value,
        }
    }

    /// Validates a definition for `token` and normalizes it (colors clamped to their gamut
    /// within rounding error).
    pub(crate) fn check_definition(
        &self,
        token: TokenId,
        definition: Definition,
    ) -> Result<Definition> {
        let info = self
            .info(token)
            .ok_or_else(|| Error::UnknownToken(token.to_string()))?;
        match definition {
            Definition::Alias(target) => {
                let target_info = self
                    .info(target)
                    .ok_or_else(|| Error::UnknownToken(target.to_string()))?;
                if target_info.kind() != info.kind() {
                    return Err(Error::KindMismatch {
                        token: self.name(token),
                        expected: info.kind(),
                        found: target_info.kind(),
                    });
                }
                if !info.tier().may_alias(target_info.tier()) {
                    return Err(Error::TierViolation {
                        token: self.name(token),
                        target: self.name(target),
                    });
                }
                if target == token {
                    return Err(Error::Cycle {
                        token: self.name(token),
                    });
                }
                Ok(Definition::Alias(target))
            }
            Definition::Value(value) => {
                if value.kind() != info.kind() {
                    return Err(Error::KindMismatch {
                        token: self.name(token),
                        expected: info.kind(),
                        found: value.kind(),
                    });
                }
                self.check_value(token, value).map(Definition::Value)
            }
        }
    }

    fn check_value(&self, token: TokenId, value: Value) -> Result<Value> {
        let invalid = |reason: &str| Error::InvalidValue {
            token: self.name(token),
            reason: reason.to_owned(),
        };
        let value = match value {
            Value::Color(color) => {
                const SLACK: f32 = 1e-4;
                let channels = [color.r, color.g, color.b, color.a];
                if !channels.iter().all(|c| (-SLACK..=1.0 + SLACK).contains(c)) {
                    return Err(invalid("color channels must lie in 0..=1"));
                }
                Value::Color(koebi_core::Color::new(
                    color.r.clamp(0.0, 1.0),
                    color.g.clamp(0.0, 1.0),
                    color.b.clamp(0.0, 1.0),
                    color.a.clamp(0.0, 1.0),
                ))
            }
            Value::Dimension(v) | Value::Number(v) if !v.is_finite() => {
                return Err(invalid("the number is not finite"));
            }
            Value::CubicBezier(curve) if !curve.is_valid() => {
                return Err(invalid("cubic Bézier x coordinates must lie in 0..=1"));
            }
            Value::Curve(curve) if !curve.is_valid() => {
                return Err(invalid(crate::value::CURVE_RULE));
            }
            Value::FontFamily(ref name) if name.trim().is_empty() || name.len() > 256 => {
                return Err(invalid("font family names have 1 to 256 bytes"));
            }
            other => other,
        };
        if let Some(range) = self.ranges[token.index()] {
            let checked = match &value {
                Value::Color(color) => Some(f64::from(color.a)),
                other => other.as_number(),
            };
            if let Some(number) = checked
                && !range.contains(number)
            {
                return Err(Error::OutOfRange {
                    token: self.name(token),
                    value: number,
                    range,
                });
            }
        }
        Ok(value)
    }

    /// Rejects a change that would close an alias cycle in any theme it affects and any
    /// combination of layers. Removing a definition is a change too: it reveals the one below.
    pub(crate) fn check_change(&self, change: &Change<'_>) -> Result<()> {
        let check = |theme: ThemeId| {
            let Some(slot) = self.slot(theme) else {
                return Ok(());
            };
            let loops = COMBINATIONS.iter().any(|&layers| {
                layers.contains(change.layer)
                    && self.loops_back(change.token, theme, slot, layers, Some(change))
            });
            if loops {
                Err(Error::Cycle {
                    token: self.name(change.token),
                })
            } else {
                Ok(())
            }
        };
        match change.theme {
            Some(theme) => check(theme),
            None => self.theme_ids().try_for_each(check),
        }
    }

    /// `true` when following aliases from `start` comes back to `start`.
    fn loops_back(
        &self,
        start: TokenId,
        theme: ThemeId,
        slot: &ThemeSlot,
        layers: Layers,
        change: Option<&Change<'_>>,
    ) -> bool {
        let mut current = start;
        for _ in 0..=self.len() {
            match self.winning(current, theme, slot, layers, change).0 {
                Definition::Value(_) => return false,
                Definition::Alias(next) if *next == start => return true,
                Definition::Alias(next) => current = *next,
            }
        }
        true
    }

    /// Rejects a theme whose definitions close an alias cycle, alone or with the global
    /// overrides.
    pub(crate) fn check_slot(&self, slot: &ThemeSlot) -> Result<()> {
        let theme = ThemeId::from_index(u16::MAX);
        for layers in COMBINATIONS {
            for index in 0..self.len() {
                let token = TokenId::from_index(u16::try_from(index).unwrap_or(u16::MAX));
                if self.loops_back(token, theme, slot, layers, None) {
                    return Err(Error::Cycle {
                        token: self.name(token),
                    });
                }
            }
        }
        Ok(())
    }
}

/// `value` brought into `range`: numbers clamped (and snapped), the alpha of colors clamped.
fn limited(value: Value, range: Range) -> Value {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the limited value lies between bounds of the token's own kind"
    )]
    let narrow = |number: f64| number as f32;
    match value {
        Value::Color(color) => {
            Value::Color(color.with_alpha(narrow(range.limit(f64::from(color.a)))))
        }
        Value::Dimension(number) => Value::Dimension(narrow(range.limit(f64::from(number)))),
        Value::Number(number) => Value::Number(narrow(range.limit(f64::from(number)))),
        Value::Integer(number) => {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "the limited value lies between two integers of the token's range"
            )]
            let limited = range.limit(f64::from(number)).round() as i32;
            Value::Integer(limited)
        }
        Value::Duration(duration) => {
            let millis = range.limit(duration.as_secs_f64() * 1000.0).max(0.0);
            Value::Duration(core::time::Duration::from_secs_f64(millis / 1000.0))
        }
        Value::FontWeight(weight) => {
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "clamped to the weights 1 to 1000 first"
            )]
            let limited = range
                .limit(f64::from(weight.get()))
                .round()
                .clamp(1.0, 1000.0) as u16;
            Value::FontWeight(crate::value::FontWeight::new(limited).unwrap_or(weight))
        }
        other => other,
    }
}
