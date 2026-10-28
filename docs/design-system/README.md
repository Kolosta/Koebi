# Design system

Every color, size, radius, duration and font property of Koebi's UI comes from a design token.
This guide explains the concepts and how to use them. [ADR 0008](../adr/0008-design-token-system.md)
explains why the system works this way.

- `crates/koebi-design`: the engine (tokens, tables, themes, overrides, views, color vision).
- `crates/koebi-tokens`: Koebi's tokens (`primitive`, `semantic`, `component`) and built-in
  themes (Dark, Light, Muted green, High contrast). They come from the previous C++
  application; the [porting report](porting.md) lists every difference.

## Tokens and tiers

A design token is a named design decision. Tokens live in three tiers:

| Tier | Defined by | Example |
|---|---|---|
| Primitive | a value | `primitive.color.blue.700` = `#5d89ff` |
| Semantic | an alias to a primitive or semantic token: an intent | `semantic.background.accent.default` → `semantic.accent.color.700` |
| Component | an alias to a semantic or component token, or a value | `component.button.background.color.default` → `semantic.background.accent.default`, `component.scrollbar.size.default` = 14 px |

Components read component tokens. Changing a primitive changes every semantic and component
token that aliases it, through any number of steps. Semantic tokens never hold values: a value
that only one component uses belongs to that component's token.

## Kinds

Every token has a kind, part of its Rust type:

| Kind | Rust value | Written in tables as |
|---|---|---|
| `Color` | `koebi_core::Color` (sRGB, straight alpha) | `hex("#5D89FF")`, `hex("#00000070")`, `rgb(93, 137, 255)`, `hsl(225.0, 1.0, 0.68)`, `oklch(0.68, 0.17, 266.0)`... |
| `Dimension` | `f32`, logical pixels | `px(4.0)` |
| `Number` | `f32`, unitless (opacity, line height, em) | `0.5` |
| `Integer` | `i32` | `100` |
| `Duration` | `core::time::Duration` | `ms(150)` |
| `CubicBezier` | `value::CubicBezier` | `bezier(0.4, 0.0, 1.0, 1.0)` |
| `FontWeight` | `value::FontWeight` (1 to 1000) | `weight(600)` |
| `FontFamily` | `Arc<str>` | `family("Noto Sans")` |
| `Curve` | `value::Curve` (2 to 8 keys from (0, 0) to (1, 1)) | `curve(&[key(0.0, 0.0, bezier(...)), key(0.7, 1.04, bezier(...)), key(1.0, 1.0, bezier(...))])` |

Kinds describe the value domain, not the role: a radius, a spacing and a border width are all
dimensions.

Animations read a duration and a curve. A `Curve` passes through its keys, each segment eased
by its own cubic Bézier, so that it can bounce or overshoot and settle; `Curve::ease(t)`
evaluates it ([ADR 0018](../adr/0018-easing-curves.md)). Primitive curves (`curve.*`) feed
semantic ones by intent (`animation.curve.expand`, `collapse`, `grow`, `move`...), which feed
the components' (`list-row.expand.curve`, `scrollbar.overlay.grow.curve`). Curves export as
Koebi's own `curve` type in Design Tokens files.

Colors accept several notations, so that a value can be copied from its source as is:
hexadecimal (`hex("#F00")`, `hex("#5D89FF80")`), 8-bit or 10-bit channels (`rgb`, `rgba`,
`rgb10`, `rgba10`), channels from 0 to 1 (`rgb_f32`, `rgba_f32`), HSL (`hsl`, `hsla`) and OKLCH
(`oklch`, `oklcha`). All are checked while compiling. Every notation but OKLCH is converted to
sRGB by the compiler; OKLCH needs trigonometry, so the design system converts it once when it
loads. The table keeps the original notation. The typography composite bundles five field tokens (family, size, weight, line
height, letter spacing).

## Paths and constants

A token's path is lowercase: `.` separates segments, `-` joins words inside a segment. Its Rust
constant is the path in upper case with `.` and `-` replaced by `_`, in the tier's module:

```text
semantic.background.accent.kbd-focus   ->   semantic::BACKGROUND_ACCENT_KBD_FOCUS
primitive.color.blue.700               ->   primitive::COLOR_BLUE_700
```

The constant's documentation is the token's description, shown by the IDE on hover.

## Reading tokens

```rust
use koebi_design::DesignSystem;

let design = DesignSystem::new(&SCHEMA)?;
let background = design.get(component::BUTTON_BACKGROUND_COLOR_DEFAULT); // koebi_core::Color
let radius = design.get(component::BUTTON_CORNER_RADIUS_DEFAULT);         // f32
let body = design.typography(semantic::TEXT_BODY_M);                      // TextStyle
```

`get` reads the active theme with every layer. It is an array access: read tokens every frame
without caching them. Components read theirs through typed styles, which also serve previews
of other themes: see [components](components.md).

## Defining tokens

Tokens are defined in Rust tables. Each line defines a token by a value
(`CONSTANT = "path": Kind = value;`) or by an alias (`CONSTANT = "path": Kind -> TARGET;`),
optionally followed by a valid range:

```rust
use koebi_design::define::*;

koebi_design::tokens! {
    group SCALE: Primitive, start = PALETTE::END;

    /// Corner radius scale, step 200.
    RADIUS_200 = "radius.200": Dimension = px(4.0), range(0.0, 32.0);
}

koebi_design::tokens! {
    group SHAPES: Semantic, start = SCALE::END;

    /// Small corner radius: inputs and form fields.
    RADIUS_S = "radius.s": Dimension -> primitive::RADIUS_200;
}

koebi_design::tokens! {
    group SCROLLBAR: Component, start = SEPARATOR::END;

    /// Scrollbar: size.
    SCROLLBAR_SIZE_DEFAULT = "scrollbar.size.default": Dimension = px(14.0), range(1.0, 32.0);
    /// Scrollbar: corner radius.
    SCROLLBAR_CORNER_RADIUS_DEFAULT = "scrollbar.corner-radius.default": Dimension -> semantic::RADIUS_M;
}
```

The compiler checks everything: kinds and tiers of aliases, paths and constant names, values
against their range, and the absence of alias cycles in the base theme and every theme.

### Constraints

A token may constrain its values, after its definition:

| Constraint | Accepts |
|---|---|
| `range(min, max)`, `range(min, max, step)` | numbers from `min` to `max`; `step` is a hint for editors |
| `at_least(min)`, `at_most(max)` | numbers from `min` up, or up to `max` |
| `positive()` | numbers above zero |
| `steps(min, max, step)` | multiples of `step` from `min` to `max` |
| `alpha(min, max)`, `opaque()` | colors whose alpha lies in the range, or is 1 |

Durations count in milliseconds. A token without its own constraint takes the one of the token
it aliases. Every layer is held to it: token tables while compiling, themes, overrides and
imports when they are set (out-of-range values are refused with the accepted range in the
error). Resolved values are also kept inside the token's own range, so a token aliasing one that
accepts more never receives a value it refuses (`component.window.background.color.default`
stays opaque whatever it aliases). Every numeric token of Koebi's set has bounds, and the
built-in themes never need that limiting: `koebi-tokens`' tests check both.

To add a token:

1. Pick the tier: a raw value is primitive, an intent semantic, a component property component.
2. Name it by its position in a scale (`radius.200`) or its intent (`background.accent.hover`),
   never by its value. States come last (`default`, `hover`, `pressed`, `selected`, `disabled`).
3. Write a one-sentence description in the `///` comment.
4. Alias the most specific existing token: components alias semantic tokens, never primitives.
   A component may hold a value of its own when no semantic intent fits.

Koebi's tables live in `crates/koebi-tokens/src/`, one token group per file:

| Files | Content |
|---|---|
| `primitive/<family>.rs` | palettes and scales: `palette.rs`, `neutral.rs`, `scale.rs`... |
| `semantic/<family>.rs` | intents: `background.rs`, `content.rs`, `typography.rs`... |
| `component/<category>/<component>.rs` | one file per component, in `controls/`, `surfaces/` or `editors/`: `component/controls/button.rs` holds every `component.button.*` token |
| `themes/<theme>.rs` | one file per built-in theme; `themes/mod.rs` lists them in order (`BUILT_IN`) |

Each group starts where the previous one ends (`start = PREVIOUS::END`), and `lib.rs` lists
the groups in that order: a new component gets its file, a line in its category's `mod.rs`
and its group in `lib.rs`. A new theme gets its file and its place in `BUILT_IN`.

## Themes

The base theme is the token tables themselves: it defines every token. A theme only lists the
tokens it changes, as aliases:

```rust
// crates/koebi-tokens/src/themes/light.rs
koebi_design::theme! {
    /// Light theme: dark content on light surfaces.
    LIGHT = "light", "Light" {
        semantic::BACKGROUND_BASE_DEFAULT => primitive::COLOR_GRAY_50;
    }
}
```

Each built-in theme has its own file: a theme is reviewed, compared and exported on its own,
and adding or removing one touches its file and the list in `themes/mod.rs` only.

Themes can also be created at run time (`DesignSystem::create_theme`), for instance from an
imported file. Switch with `set_active_theme`.

## Overrides

The user changes tokens on top of themes, with a value or an alias:

```rust
use koebi_design::Scope;

design.set_value(Scope::ActiveTheme, semantic::RADIUS_CONTROL, 6.0)?;
design.set_value(Scope::Global, primitive::COLOR_BLUE_700, Color::from_hex(0x2D_6A_FF))?;
design.set_alias(Scope::Theme(light), semantic::ACCENT_COLOR_700, primitive::COLOR_PURPLE_700)?;
design.reset(Scope::ActiveTheme, semantic::RADIUS_CONTROL)?;
```

- `Scope::ActiveTheme` and `Scope::Theme(id)` change one theme; `Scope::Global` applies on top
  of every theme.
- Values are checked against the token's kind and constraint (inherited along aliases);
  aliases against the tier rules. A change that would make a chain of aliases loop is rejected, in every
  theme it affects. On error, nothing changes.
- Every token that depends on the changed one is resolved again before the call returns.

## Layers and views

From top to bottom: global overrides, overrides of the theme, the theme, the base theme. Color
vision transforms the result. A `View` reads any theme with any set of layers:

```rust
use koebi_design::Layers;

let original = design.view(theme, Layers::THEME)?.get(token);       // the theme as designed
let base = design.view(theme, Layers::NONE)?.get(token);            // the base theme
let mine = design.view(theme, Layers::THEME | Layers::THEME_OVERRIDES)?.get(token);
let preview = design.view(other_theme, Layers::ALL)?.get(token);    // a preview card
let origin = design.origin(token);                                  // which layer wins
```

## Layer masks

Each token has a layer mask, `Layers::ALL` by default. It applies to the token and to everything
it aliases from there:

```rust
// A logo keeps its true colors under color vision correction.
design.set_token_layers(component::SPLASH_LOGO_COLOR, Layers::ALL - Layers::COLOR_VISION)?;
// A preview swatch ignores the user's overrides.
design.set_token_layers(token, Layers::ALL - Layers::OVERRIDES)?;
// A token that keeps its base value in every theme.
design.set_token_layers(token, Layers::NONE)?;
```

## Color vision

`ColorVision` corrects (daltonization) or simulates protan, deutan and tritan deficiencies,
simulates achromatopsia, or applies a custom matrix. The 3×3 matrix works in linear sRGB and is
applied once per color token when tokens are resolved, never per pixel. Grays are unchanged.
Tokens whose mask lacks `Layers::COLOR_VISION` keep their colors, and so does everything that
aliases them.

## Persistence

`koebi_design::persist::Store` saves what the user changed (run-time themes, overrides, layer
masks, color vision, active theme) in a small binary file, atomically and with a backup;
[ADR 0009](../adr/0009-design-state-persistence.md) explains the format.

```rust
use koebi_design::persist::Store;

let mut store = Store::new(preferences_dir);
store.load(&mut design)?;                    // at startup
store.save_if_due(&design, Instant::now())?; // every frame: writes once a change has settled
store.save(&design)?;                        // when the application goes to the background
```

## Import and export

`koebi_design::dtcg` exchanges themes as JSON in the W3C Design Tokens format, with Koebi's
tiers, layers and theme identity in an extension ([ADR 0010](../adr/0010-design-token-interchange.md)):

```rust
use koebi_design::dtcg::{export_schema, export_theme, import_theme};

let file = export_theme(&design, design.active_theme())?; // what differs from the base theme
let reference = export_schema(&design);                    // every token, documented
let theme = import_theme(&mut design, &file)?;            // strict: every problem is reported
store.save(&design)?;                                      // an imported theme is kept at once
```

The application does the same from the command line: `--import <FILE>` imports a theme,
activates it and saves the state at once, `--export <FILE>` writes the active theme and
`--export-schema <FILE>` every token (see the [gallery](gallery.md#command-line)).

An exported token looks like this:

```json
"semantic": {
  "background": {
    "base": {
      "default": {
        "$type": "color",
        "$value": "{primitive.color.gray.50}",
        "$description": "Base background of the application.",
        "$extensions": { "io.github.kolosta.koebi": { "tier": "semantic", "layer": "theme" } }
      }
    }
  }
}
```

## Color science

### OKLCH

OKLCH (Björn Ottosson, 2020) describes a color by perceived lightness (`l`), chroma (`c`) and
hue (`h`). Unlike HSL, equal steps of `l` look equal, and changing `h` keeps the perceived
lightness, so it is the right space to edit a color with sliders, derive hover states or build a
palette. Koebi implements it in `koebi_core::color` (`Oklch`, with gamut mapping by chroma
reduction): no dependency is involved. Token values stay in sRGB, the space the renderer
consumes; OKLCH is how they are edited and generated.

### WCAG contrast

WCAG 2.x defines the contrast ratio of two colors from their relative luminance, from 1:1 to
21:1. Body text needs at least 4.5:1 (level AA), large text and UI components 3:1. Koebi
implements it in `Color::contrast_ratio`; token tests check the main text and surface pairs in
every theme.

### UI colors and canvas colors

UI tokens are display-referred sRGB, blended like design tools do (ADR 0004). Canvas documents
will have their own working color space and color management (issue #48), sharing the renderer
but never going through tokens or the color vision layer.
