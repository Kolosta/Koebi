# 0008. Design token system: typed tiers, compiled tables, layered resolution

- Status: Accepted
- Date: 2026-10-24
- Deciders: @Kolosta

## Context

Every color, size, radius, duration and font property of Koebi's UI must come from design
tokens, with these requirements:

- three tiers with strict rules: primitive tokens hold values, semantic tokens alias primitive
  or semantic tokens, component tokens alias semantic or component tokens or hold a value of
  their own;
- themes that only state what they change, user overrides per theme and global, access to
  the original values and to other themes (preview cards);
- live editing with a slider, applied to the whole UI in the same frame, with hundreds of
  components reading tokens every frame;
- a color-only accessibility layer for color vision deficiencies, with exemptions;
- type safety: a component expecting colors and a dimension must not compile with anything
  else, and tokens must be Rust items with autocompletion, not strings;
- nothing parsed at run time; no new dependency (ADR 0003): no `serde`, no `syn`.

The previous C++ application had a mature token set. Its names, values and references are
ported; its implementation is not.

## Decision

A new crate, `koebi-design`, implements the engine. Koebi's own token set lives in its own crate
on top of it.

### Tiers

`Token<K, T>` is a two-byte `Copy` handle; `T` is `tier::Primitive`, `tier::Semantic` or
`tier::Component`.

| Tier | Defined in token tables by |
|---|---|
| Primitive | a value |
| Semantic | an alias to a primitive or semantic token |
| Component | an alias to a semantic or component token, or a value |

The alias rules are a trait, `MayAlias`: an alias that breaks them does not compile, with a
message stating the rule. Token tables reject values in semantic tokens while compiling.
Component tokens may hold values because many component properties have no shared intent (a
scrollbar's thickness); semantic tokens never do, since their purpose is to name an intent.
Overrides and themes may give any token a value (a custom color picked in an editor, an
imported theme); their aliases follow the same rules.

### Kinds

`K` is a functional kind aligned with the types of the W3C Design Tokens format: color,
dimension (logical pixels), number, integer, duration, cubic Bézier, font weight and font
family, plus the typography composite.

- Not raw numbers: an `f32` token could be pixels, milliseconds or a ratio, and mixing them is
  a real class of bugs. Kinds carry the unit, the Rust value type and the validation rules.
- Not one type per role (radius, spacing, border width): the role is in the token's name, and
  aliases across roles are legitimate (a radius equal to a spacing step). Role types would
  multiply conversions without catching real mistakes.
- One color kind with alpha, not separate RGB and RGBA kinds: opacity is a channel, an opaque
  color has alpha 1, and a single kind keeps aliases and overrides uniform.
- Opacity is a number with a `0..1` range; the Design Tokens format has no opacity type either.

Numeric tokens may declare a range, inherited along aliases; overrides outside it are rejected.

### Names

A token has a lowercase dotted path (`semantic.background.accent.kbd-focus`; `.` separates
segments, `-` joins words) and a Rust constant in its tier's module, the path in upper case with
`.` and `-` turned into `_`: `semantic::BACKGROUND_ACCENT_KBD_FOCUS`. The correspondence is
checked at compile time, and two paths that would give the same constant cannot coexist because
the compiler rejects the duplicate. Paths are used only at the boundaries: persistence, JSON and
editors.

### Token tables

Tokens are defined in Rust with declarative macros: `tokens!` (one group of one tier, each line
`CONSTANT = "path": Kind = value;` or `CONSTANT = "path": Kind -> TARGET;`), `typography!`
(composites whose five fields are tokens of their own) and `theme!`. The `///` comment of a token
is its description. A `Schema` lists the groups and themes; calling
`Schema::assert_valid` in a `const` item checks, while compiling: contiguous groups, path syntax,
constant names, values against kinds and ranges, alias targets (existence, kind, tier) and the
absence of alias cycles in the base theme and in every theme. `Schema::validate` adds the checks
that are too costly for compile time (unique paths, no token below another token's path).

### Themes, layers, views

The base theme is the token tables: it defines every token. A theme is a sparse set of
definitions. Resolution looks, from top to bottom, at the global overrides, the overrides of the
theme, the theme, then the base theme; the color vision layer transforms the resulting colors.

- A `View` reads any theme with any set of `Layers`: the base theme, a theme as designed, one
  layer of overrides, other themes for preview cards.
- Each token has a layer mask, narrowed at every step of its alias chain: a token can ignore the
  overrides, stay on the base theme, or skip color vision, and so does everything it aliases
  from there.
- Every change is validated before it is applied: values against kinds and ranges, aliases
  against the tier rules, and cycles in every combination of layers of every affected theme.
  Removing an override is checked too, since it reveals the definition below.

### Resolution

The active theme with every layer is materialized: reading a token is an array access. A reverse
dependency graph (the alias edges of every layer of the active theme, in compressed rows) lists
the tokens that may depend on each token; a change resolves only the changed token and its
dependents. Buffers are reused, so a value change does not allocate once warm. Other views
resolve tokens on demand by walking their aliases.

### Colors

- Primitive colors keep exact sRGB values: the port from hexadecimal values is lossless and the
  renderer consumes sRGB (ADR 0004). OKLCH serves editing (lightness, chroma and hue sliders),
  palette derivation and import. This supersedes the "tokens are defined in OKLCH" item of
  ADR 0004; the rest of ADR 0004 stands.
- Token tests check WCAG 2.x contrast ratios between text and surfaces in every theme.
- Color vision uses 3×3 matrices in linear sRGB from `koebi-core` (simulation from Machado,
  Oliveira and Fernandes 2009; correction by daltonization), applied once per color token during
  resolution. Tokens whose mask lacks the color vision layer are never transformed.
- UI tokens are display-referred. Canvas content gets its own color management (issue #48) and
  never goes through the color vision layer.

## Consequences

- Kind and tier mistakes, misspelled tokens, broken aliases and cycles are compile errors.
- Reading a token costs an array access; a slider edit costs the resolution of its dependents.
- Token tables are verbose (path and constant on every line) and adding a token needs a rebuild:
  tokens are code. Positions are `u16`: a schema holds at most 65 535 tokens.
- Macro errors are less direct than plain code; `#[diagnostic::on_unimplemented]` and messages
  naming the token keep them readable.
- Follow-up work: persistence (#43), JSON interchange (#44), canvas color management (#48).

## Alternatives considered

- **Token files (JSON, TOML) loaded at run time**: parsing at startup, no autocompletion, errors
  found at run time. JSON stays the interchange format.
- **Code generation from a JSON source** (`build.rs` or `xtask`): two representations of every
  token, generated code in Git or invisible to the IDE until built, and a JSON parser duplicated
  in the build script.
- **Procedural macros**: practical only with `syn` and `quote`, new dependencies; declarative
  macros are enough.
- **String keys and hash maps**: slower reads, typos found at run time, no autocompletion.
- **Nested modules mirroring paths** (`semantic::background::accent::DEFAULT`): numeric segments
  need awkward identifiers (`_700`), and the macros become far more complex.
- **Recomputing every token on each change**: fast enough today (tens of microseconds for about
  1 400 tokens) but proportional to the whole set; incremental propagation keeps an edit
  proportional to what it affects.
- **Component values through semantic tokens**: one primitive and one semantic token per
  component-specific value, named after the component. That is noise without a shared intent;
  component tokens hold such values directly, like the component tokens of Material Design.
