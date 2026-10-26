//! [`Resolve`]: one way to read token values, whatever their source.

use crate::kind::Kind;
use crate::system::DesignSystem;
use crate::tier::Tier;
use crate::token::{TextStyle, Token, TokenId, Typography};
use crate::value::Value;
use crate::view::View;

/// Reads resolved token values: the active theme ([`DesignSystem`]), any [`View`], or a
/// [`Snapshot`] of either. The trait is sealed: these three are its only sources.
///
/// Code that styles components takes `&impl Resolve`, so the same function styles the live
/// interface and previews of other themes:
///
/// ```
/// # use koebi_design::Resolve;
/// # use koebi_core::Color;
/// # mod tokens {
/// #     use koebi_design::define::*;
/// #     koebi_design::tokens! {
/// #         group PALETTE: Primitive, start = 0;
/// #         /// Blue.
/// #         COLOR_ACCENT = "color.accent": Color = hex("#5D89FF");
/// #         /// Corner radius.
/// #         RADIUS_CONTROL = "radius.control": Dimension = px(3.0);
/// #     }
/// #     pub const SCHEMA: koebi_design::schema::Schema = koebi_design::schema::Schema::new(
/// #         "example",
/// #         koebi_design::schema::ThemeInfo::new("base", "Base", "Base theme.", &[]),
/// #         &[PALETTE::GROUP],
/// #         &[],
/// #     );
/// # }
/// # static SCHEMA: koebi_design::schema::Schema = tokens::SCHEMA;
/// struct ButtonStyle {
///     background: Color,
///     corner_radius: f32,
/// }
///
/// impl ButtonStyle {
///     fn resolve(source: &impl Resolve) -> Self {
///         Self {
///             background: source.get(tokens::COLOR_ACCENT),
///             corner_radius: source.get(tokens::RADIUS_CONTROL),
///         }
///     }
/// }
///
/// let design = koebi_design::DesignSystem::new(&SCHEMA)?;
/// let live = ButtonStyle::resolve(&design);
/// let original = ButtonStyle::resolve(
///     &design.view(design.active_theme(), koebi_design::Layers::NONE).expect("live theme"),
/// );
/// assert_eq!(live.corner_radius, original.corner_radius);
/// # Ok::<(), koebi_design::Error>(())
/// ```
pub trait Resolve: sealed::Sealed {
    /// The value of `token`.
    ///
    /// A token of another schema reads as the kind's fallback (magenta for colors).
    fn get<K: Kind, T: Tier>(&self, token: Token<K, T>) -> K::Value;

    /// Captures every token's value into `snapshot`, reusing its memory: what reads tokens
    /// later without borrowing the source.
    fn snapshot_into(&self, snapshot: &mut Snapshot);

    /// Every token's value, captured in a new [`Snapshot`].
    fn snapshot(&self) -> Snapshot {
        let mut snapshot = Snapshot::default();
        self.snapshot_into(&mut snapshot);
        snapshot
    }

    /// The resolved text style of a typography composite.
    fn typography<T: Tier>(&self, style: Typography<T>) -> TextStyle {
        TextStyle {
            font_family: self.get(style.font_family),
            font_size: self.get(style.font_size),
            font_weight: self.get(style.font_weight),
            line_height: self.get(style.line_height),
            letter_spacing: self.get(style.letter_spacing),
        }
    }
}

/// Keeps [`Resolve`] to the sources of this crate.
mod sealed {
    pub trait Sealed {}

    impl Sealed for crate::DesignSystem {}
    impl Sealed for crate::View<'_> {}
    impl Sealed for super::Snapshot {}
}

impl Resolve for DesignSystem {
    #[inline]
    fn get<K: Kind, T: Tier>(&self, token: Token<K, T>) -> K::Value {
        DesignSystem::get(self, token)
    }

    fn snapshot_into(&self, snapshot: &mut Snapshot) {
        snapshot.fill(self.schema_len(), |token| self.value(token).cloned());
    }
}

impl Resolve for View<'_> {
    #[inline]
    fn get<K: Kind, T: Tier>(&self, token: Token<K, T>) -> K::Value {
        View::get(self, token)
    }

    fn snapshot_into(&self, snapshot: &mut Snapshot) {
        snapshot.fill(self.schema_len(), |token| self.value(token));
    }
}

/// Every token's value at one moment, read in constant time without borrowing the design
/// system: what components keep to resolve tokens a call gives them while drawing.
///
/// Take one with [`Resolve::snapshot`] whenever the design system changes (its
/// [revision](DesignSystem::revision)).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    values: Vec<Value>,
}

impl Snapshot {
    /// Replaces the values with those of the `len` tokens of a schema, as `value` gives them;
    /// the memory is kept from one capture to the next.
    fn fill(&mut self, len: usize, value: impl Fn(TokenId) -> Option<Value>) {
        self.values.clear();
        self.values.reserve(len);
        let tokens = (0..len).map_while(|index| u16::try_from(index).ok());
        self.values.extend(
            tokens.map(|index| value(TokenId::from_index(index)).unwrap_or(Value::Integer(0))),
        );
    }

    /// The value of `token`; `None` for a token of another schema.
    pub fn value(&self, token: impl Into<TokenId>) -> Option<&Value> {
        self.values.get(token.into().index())
    }

    /// Number of tokens captured.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// `true` for a snapshot of no schema.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl Resolve for Snapshot {
    fn get<K: Kind, T: Tier>(&self, token: Token<K, T>) -> K::Value {
        self.value(token.id())
            .and_then(K::from_value)
            .unwrap_or_else(K::fallback)
    }

    fn snapshot_into(&self, snapshot: &mut Snapshot) {
        snapshot.values.clone_from(&self.values);
    }
}
