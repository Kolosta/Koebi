//! Token schemas: the static description of every token and every theme written in code.
//!
//! Token tables ([`tokens!`](crate::tokens), [`typography!`](crate::typography)) each produce a
//! [`Group`] of consecutive tokens; [`theme!`](crate::theme) produces [`ThemeInfo`]s. A
//! [`Schema`] puts the groups and themes together. Schemas are `const`: build them in a
//! `const` item and call [`Schema::assert_valid`] in a `const _: () = ...;` item so that a
//! broken schema does not compile.

use core::fmt;

use crate::check;
#[doc(hidden)]
pub use crate::check::assert_group;
use crate::kind::KindId;
use crate::tier::{MayAlias, Tier, TierId};
use crate::token::{Token, TokenId};
use crate::value::{Def, Range};

/// The static description of one token.
#[derive(Clone, Copy, Debug)]
pub struct TokenInfo {
    path: &'static str,
    ident: &'static str,
    tier: TierId,
    kind: KindId,
    description: &'static str,
    definition: Def,
    range: Option<Range>,
}

impl TokenInfo {
    /// Describes a token. Used by the token table macros.
    #[doc(hidden)]
    pub const fn new(
        path: &'static str,
        ident: &'static str,
        tier: TierId,
        kind: KindId,
        description: &'static str,
        definition: Def,
        range: Option<Range>,
    ) -> Self {
        Self {
            path,
            ident,
            tier,
            kind,
            description,
            definition,
            range,
        }
    }

    /// The path inside the tier, such as `background.accent.default`.
    pub const fn path(&self) -> &'static str {
        self.path
    }

    /// The full path, tier first: `semantic.background.accent.default`.
    pub const fn qualified(&self) -> QualifiedPath {
        QualifiedPath {
            tier: self.tier,
            path: self.path,
        }
    }

    /// The Rust constant naming the token (`BACKGROUND_ACCENT_DEFAULT`); empty for the fields
    /// of a composite, which are reached through the composite's constant.
    pub const fn ident(&self) -> &'static str {
        self.ident
    }

    /// The tier of the token.
    pub const fn tier(&self) -> TierId {
        self.tier
    }

    /// The kind of the token.
    pub const fn kind(&self) -> KindId {
        self.kind
    }

    /// What the token is for, from its documentation comment.
    pub fn description(&self) -> &'static str {
        self.description.trim()
    }

    /// The definition of the token in the base theme.
    pub const fn definition(&self) -> Def {
        self.definition
    }

    /// The valid range declared by the table, if any. Tokens without one inherit the range
    /// of the token they alias (see [`DesignSystem::range`](crate::DesignSystem::range)).
    pub const fn range(&self) -> Option<Range> {
        self.range
    }
}

/// A token path with its tier, displayed as `tier.path`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct QualifiedPath {
    tier: TierId,
    path: &'static str,
}

impl QualifiedPath {
    /// The tier.
    pub const fn tier(&self) -> TierId {
        self.tier
    }

    /// The path inside the tier.
    pub const fn path(&self) -> &'static str {
        self.path
    }
}

impl fmt::Display for QualifiedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.tier.name(), self.path)
    }
}

/// The tokens of one table, stored at consecutive positions of the schema.
#[derive(Clone, Copy, Debug)]
pub struct Group {
    name: &'static str,
    start: u16,
    tokens: &'static [TokenInfo],
    composites: &'static [CompositeInfo],
}

impl Group {
    /// Describes a group. Used by the token table macros.
    #[doc(hidden)]
    pub const fn new(
        name: &'static str,
        start: u16,
        tokens: &'static [TokenInfo],
        composites: &'static [CompositeInfo],
    ) -> Self {
        Self {
            name,
            start,
            tokens,
            composites,
        }
    }

    /// The name of the group, as written in its table.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Position of the first token.
    pub const fn start(&self) -> u16 {
        self.start
    }

    /// Position after the last token.
    pub const fn end(&self) -> usize {
        self.start as usize + self.tokens.len()
    }

    /// The tokens, in order.
    pub const fn tokens(&self) -> &'static [TokenInfo] {
        self.tokens
    }

    /// The composites whose fields this group holds.
    pub const fn composites(&self) -> &'static [CompositeInfo] {
        self.composites
    }
}

/// The kind of a composite token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompositeKind {
    /// A text style: font family, size, weight, line height and letter spacing.
    Typography,
}

/// One field of a composite kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositeField {
    /// Last segment of the field token's path (`font-size`).
    pub segment: &'static str,
    /// Key of the field in the Design Tokens format (`fontSize`).
    pub key: &'static str,
    /// Kind of the field token.
    pub kind: KindId,
}

impl CompositeKind {
    /// The name of the kind in the Design Tokens format.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Typography => "typography",
        }
    }

    /// The fields of the kind, in the order of their tokens.
    pub const fn fields(self) -> &'static [CompositeField] {
        match self {
            Self::Typography => &[
                CompositeField {
                    segment: "font-family",
                    key: "fontFamily",
                    kind: KindId::FontFamily,
                },
                CompositeField {
                    segment: "font-size",
                    key: "fontSize",
                    kind: KindId::Dimension,
                },
                CompositeField {
                    segment: "font-weight",
                    key: "fontWeight",
                    kind: KindId::FontWeight,
                },
                CompositeField {
                    segment: "line-height",
                    key: "lineHeight",
                    kind: KindId::Number,
                },
                CompositeField {
                    segment: "letter-spacing",
                    key: "letterSpacing",
                    kind: KindId::Number,
                },
            ],
        }
    }
}

/// The static description of a composite token: a named bundle of consecutive field tokens.
#[derive(Clone, Copy, Debug)]
pub struct CompositeInfo {
    path: &'static str,
    description: &'static str,
    tier: TierId,
    kind: CompositeKind,
    first: u16,
}

impl CompositeInfo {
    /// Describes a composite. Used by the token table macros.
    #[doc(hidden)]
    pub const fn new(
        path: &'static str,
        description: &'static str,
        tier: TierId,
        kind: CompositeKind,
        first: u16,
    ) -> Self {
        Self {
            path,
            description,
            tier,
            kind,
            first,
        }
    }

    /// The path inside the tier, such as `text.body.m`.
    pub const fn path(&self) -> &'static str {
        self.path
    }

    /// The full path, tier first.
    pub const fn qualified(&self) -> QualifiedPath {
        QualifiedPath {
            tier: self.tier,
            path: self.path,
        }
    }

    /// What the composite is for.
    pub fn description(&self) -> &'static str {
        self.description.trim()
    }

    /// The tier of the composite and of its fields.
    pub const fn tier(&self) -> TierId {
        self.tier
    }

    /// The kind of composite.
    pub const fn kind(&self) -> CompositeKind {
        self.kind
    }

    /// The handle of the first field token; the others follow in [`CompositeKind::fields`]
    /// order.
    pub const fn first(&self) -> TokenId {
        TokenId::from_index(self.first)
    }
}

/// One token definition of a theme written in code.
#[derive(Clone, Copy, Debug)]
pub struct ThemeEntry {
    token: TokenId,
    definition: Def,
}

impl ThemeEntry {
    /// In this theme, `token` aliases `target`. Checked at compile time like table aliases.
    pub const fn alias<K, T, U>(token: Token<K, T>, target: Token<K, U>) -> Self
    where
        T: MayAlias<U>,
        U: Tier,
    {
        Self {
            token: token.id(),
            definition: Def::Alias(target.id()),
        }
    }

    /// The token this entry defines.
    pub const fn token(&self) -> TokenId {
        self.token
    }

    /// Its definition in the theme.
    pub const fn definition(&self) -> Def {
        self.definition
    }
}

/// A theme written in code: a key, a display name and the definitions it changes.
#[derive(Clone, Copy, Debug)]
pub struct ThemeInfo {
    key: &'static str,
    name: &'static str,
    description: &'static str,
    entries: &'static [ThemeEntry],
}

impl ThemeInfo {
    /// Describes a theme. Used by [`theme!`](crate::theme) and for base themes.
    pub const fn new(
        key: &'static str,
        name: &'static str,
        description: &'static str,
        entries: &'static [ThemeEntry],
    ) -> Self {
        Self {
            key,
            name,
            description,
            entries,
        }
    }

    /// Stable identifier, used by persistence and import (`light`, `high-contrast`).
    pub const fn key(&self) -> &'static str {
        self.key
    }

    /// Display name.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// What the theme is for.
    pub fn description(&self) -> &'static str {
        self.description.trim()
    }

    /// The definitions the theme changes, on top of the base theme.
    pub const fn entries(&self) -> &'static [ThemeEntry] {
        self.entries
    }
}

/// A complete token schema: token groups, the base theme and the themes written in code.
#[derive(Debug)]
pub struct Schema {
    name: &'static str,
    base: ThemeInfo,
    groups: &'static [Group],
    themes: &'static [ThemeInfo],
}

impl Schema {
    /// Puts a schema together. `base` names the theme defined by the token tables themselves;
    /// it has no entries.
    pub const fn new(
        name: &'static str,
        base: ThemeInfo,
        groups: &'static [Group],
        themes: &'static [ThemeInfo],
    ) -> Self {
        Self {
            name,
            base,
            groups,
            themes,
        }
    }

    /// Checks the schema at compile time: contiguous groups, alias targets that exist with
    /// the right kind and tier, an acyclic graph in every theme, valid themes and composites.
    ///
    /// # Panics
    ///
    /// With a message naming the first broken rule and token. Call it in a `const` item, so
    /// that the panic is a compile error.
    pub const fn assert_valid(&self) {
        check::assert_schema(self);
    }

    /// Checks every rule of [`Self::assert_valid`] plus the ones too costly for compile time
    /// (unique paths, no token path inside another token's path).
    ///
    /// # Errors
    ///
    /// Returns every problem found, one message each.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let problems = check::validate_schema(self);
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems)
        }
    }

    /// Name of the schema.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Number of tokens, composite fields included.
    pub const fn len(&self) -> usize {
        match self.groups.last() {
            Some(group) => group.end(),
            None => 0,
        }
    }

    /// `true` when the schema has no token.
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The groups, in position order.
    pub const fn groups(&self) -> &'static [Group] {
        self.groups
    }

    /// The description of a token; `None` when `id` belongs to another schema.
    pub fn token(&self, id: TokenId) -> Option<&'static TokenInfo> {
        let index = id.index();
        self.groups
            .iter()
            .find(|group| index >= usize::from(group.start) && index < group.end())
            .map(|group| &group.tokens[index - usize::from(group.start)])
    }

    /// Every token with its handle, in position order.
    pub fn tokens(&self) -> impl Iterator<Item = (TokenId, &'static TokenInfo)> {
        self.groups.iter().flat_map(|group| {
            group
                .tokens
                .iter()
                .zip(group.start..)
                .map(|(info, index)| (TokenId::from_index(index), info))
        })
    }

    /// Every composite token.
    pub fn composites(&self) -> impl Iterator<Item = &'static CompositeInfo> {
        self.groups.iter().flat_map(|group| group.composites.iter())
    }

    /// The base theme: the definitions of the token tables.
    pub const fn base_theme(&self) -> &ThemeInfo {
        &self.base
    }

    /// The themes written in code, besides the base theme.
    pub const fn themes(&self) -> &'static [ThemeInfo] {
        self.themes
    }
}
