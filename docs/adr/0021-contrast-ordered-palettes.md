# 0021. Palettes ordered by contrast, made for each theme

- Status: Accepted
- Date: 2026-11-10
- Deciders: @Kolosta

## Context

The palettes came from the previous application: Spectrum's light palettes, with gray steps
added for the dark interface (740, 760, 780, 850, 875). Their values were the same in every
theme; a theme changed which step a semantic token aliased (the light theme moved 27 aliases,
high contrast 27). Three problems followed:

- **Contrast was not designed.** Text met WCAG, but white content on the accent did not (2.67:1
  in the dark theme), nor several boundaries (#60). Each theme's choices were separate guesses.
- **A theme could not have its own palettes.** A warm or a muted theme needs every gray tinted
  and its hues adapted to its background; aliases can only point at the same values elsewhere.
- **Nothing could make a palette from a chosen color**, which the color settings need: change
  one color and have every state, part and component follow.

## Decision

Palette steps are contrasts with the window's background, as in Adobe Spectrum: step 900 of a
hue is the color of that hue at a given contrast against the theme's background. Semantic tokens
name steps for uses, identically in every theme; each theme gives its palettes the values that
keep each step's contrast against its own background. Built-in themes hold those values as
literals (`values` blocks, `ThemeEntry::literal`), written from key colors by a generator and
checked by tests within a tolerance.

- `koebi_core::palette::ColorScale`: a hue's path through key colors in OKLCH; a step is found
  by bisection on lightness for its contrast ratio.
- Contrast tables (`docs/design-system/color.md`): Spectrum's light ratios, measured; the dark
  table shifted by a step so that step 900 holds white content in both schemes.
- `koebi_tokens::palettes`: the keys (Spectrum's 18 light palettes), the tables, each theme's
  background, neutral keys, chroma and seeds, the color roles; `from_seed` makes a palette from
  one color for any background.
- `tests/palettes.rs` writes and checks the palettes; `tests/contrast.rs` checks every
  component's content against its fill in every theme.
- The gray steps are renamed to 16 contrast steps (25 to 1000, adding 150, 250 and 350, removing
  740 to 875); every semantic reference moved to the step of the same contrast, so that the dark
  theme keeps its look.

## Consequences

- A theme is a background, a few key colors and the aliases its scheme needs; Snow, Arctic,
  Muted green, Dark pink and High contrast are built this way, and the light theme is gone.
- Contrast is a property tested for every theme and component, and `#60` is settled by it.
- The color settings can regenerate a role's palette from one color with the same algorithm the
  built-in themes use.
- Overrides saved against the old gray steps 740 to 875 are dropped and reported on restore;
  overrides of other steps keep their values, on steps whose meaning moved. The light theme's
  overrides are dropped, and an active light theme falls back to Dark, with a report.
- Each built-in theme file holds about 300 generated values, regenerated rather than edited.

## Alternatives considered

- **Keep shared values and per-theme aliases**: no tinted themes, contrast by hand per theme.
- **Compute palettes at run time from keys**: no literal tokens to inspect, compare or tune,
  and every start would run the generator; the literals keep the tokens the source of truth.
- **APCA instead of WCAG 2 ratios**: closer to perception, but not the standard the
  accessibility requirements cite; the tables can move to it later without changing the model.
