# Color

How Koebi's colors are defined, why each value is what it is, and the contrast every theme
guarantees. The decision is [ADR 0021](../adr/0021-contrast-ordered-palettes.md); the data is
`crates/koebi-tokens/src/palettes.rs`, the generator `koebi_core::palette`, the guarantees
`crates/koebi-tokens/tests/contrast.rs` and `tests/palettes.rs`.

## Palettes ordered by contrast

A palette step is a **contrast with the window's background**, not a color. Step 900 of a hue is
the color of that hue with a given contrast against the theme's background (`gray.50`): dark on
a light background, light on a dark one. A semantic token names a step for a use ("an accent fill
holding white text is `accent.color.900`"), and every theme gives each step the color that keeps
that contrast against its own background. The semantic and component tiers are the same in every
theme; a theme changes the values of its palettes, and only the few aliases where its scheme
needs another step ([below](#what-a-theme-changes)).

This is how Adobe Spectrum builds its palettes (with its Leonardo tool). Koebi's hue palettes
start from Spectrum's light palettes, which the previous application ported: measured against
white, every hue's step has the same contrast within 1 %, and those ratios are the light table
below.

### Hue palettes (18 hues, steps 100 to 1600)

| Step | 100 | 200 | 300 | 400 | 500 | 600 | 700 | 800 | 900 | 1000 | 1100 | 1200 | 1300 | 1400 | 1500 | 1600 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Light themes | 1.06 | 1.15 | 1.33 | 1.61 | 2.0 | 2.61 | 3.23 | 3.96 | 4.81 | 6.31 | 7.94 | 10.04 | 12.36 | 14.85 | 17.04 | 19.53 |
| Dark themes | 1.03 | 1.06 | 1.15 | 1.33 | 1.61 | 2.0 | 2.61 | 3.23 | 3.96 | 4.81 | 6.31 | 7.94 | 10.04 | 12.36 | 14.85 | 17.04 |

The dark table is the light one shifted by a step. The reason is the most common use of a
colored step: a fill holding white content (an accent button, a selected toggle). White text at
4.5:1 needs a fill of relative luminance below 0.183. Against white, step 900 (4.81:1) has it;
against Koebi's dark gray (#131313), the step at 3.96:1 has it. With the shifted table, step 900
is that fill on both sides, and a semantic token names one step for one use in every theme. A
dark theme whose background is lighter than #131313 would lift step 900 above that luminance:
the generator then darkens steps 100 to 900 until white content reaches 4.55:1
(`palettes::WHITE_CONTENT`).

A step out of reach (17:1 against a background that cannot give it) takes the end of the scale.

### The neutral palette (steps 25 to 1000)

| Step | 25 | 50 | 75 | 100 | 150 | 200 | 250 | 300 | 350 | 400 | 500 | 600 | 700 | 800 | 900 | 1000 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Contrast | deepest | background | 1.115 | 1.18 | 1.278 | 1.45 | 1.69 | 2.03 | 2.30 | 3.95 | 5.75 | 8.09 | 10.9 | 13.3 | 15.3 | end |
| Dark | #000000 | #131313 | #1E1E1E | #232323 | #292929 | #323232 | #3C3C3C | #484848 | #505050 | #747474 | #8F8F8F | #ABABAB | #C6C6C6 | #DADADA | #E9E9E9 | #FFFFFF |

The ratios are the dark theme's own grays, measured: the dark theme keeps its look, and every
other theme gets the same structure. 25 is the deepest surface (black in a dark theme, white in
a light one), 50 the window's background, 75 to 350 the surfaces and controls, 400 the first
step above 3:1 (borders), 500 and up text-like marks and text (900, the primary text, above
15:1).

### How a step's color is found

`koebi_core::palette::ColorScale` is a hue's path from black to white through **key colors**,
in OKLCH: lightness from 0 to 1, chroma and hue interpolated between the keys (hues the short
way round), out-of-gamut colors brought into sRGB by reducing chroma at constant lightness and
hue. Below the darkest key the chroma falls to black, above the lightest to white.
`with_contrast(background, ratio)` finds the color of the scale with that WCAG 2 contrast ratio
against the background by bisection on lightness, on the side away from it (lighter on a
background darker than luminance 0.18, darker otherwise). A palette is one such search per step.

- A built-in hue's keys are its 16 Spectrum light values: against white, the scale gives them
  back within 6/255.
- A theme may make a hue from one chosen color instead (`ThemePalettes::seeds`): the scale from
  black through that color to white. Muted green's sage, Snow's salmon and Arctic's slate are
  seeds.
- A muted theme lowers the chroma of its keys (`ThemePalettes::chroma`, 0.8 for Muted green).
- A neutral palette's keys are its tint along lightness: a single gray for Dark, greens going to
  sepia for Muted green, warm browns for Snow, blue grays for Arctic.

`tests/palettes.rs` writes the values into the token tables and theme files
(`KOEBI_BLESS=1 cargo nextest run -p koebi-tokens palettes`) and checks them otherwise: each
step within 4 % of its contrast. A value may be tuned by hand within that tolerance; the
literals in the sources are the tokens, the generator only proposes them.

## Which step for which use

| Use | Step | Why |
|---|---|---|
| Surfaces: deepest, background, controls' area, canvas, panels, nested, fields, hovered field | gray 25, 50, 75, 100, 150, 200, 300, 350 | the dark theme's elevation |
| Borders, tree lines | gray 400 | 3:1 against the panels |
| Strong borders, scrollbar grab, disabled text | gray 500 | |
| Text: primary, secondary, tertiary and placeholder | gray 900, 800, 700, 700 | 4.5:1 on every surface, fields included |
| A colored fill holding white content (accent, positive, negative, informative, notice buttons; selected toggles; the active title) | 900 | white at 4.5:1 or more |
| Its hover and press, dark themes | 800, 700 | darker: white keeps its contrast |
| Its hover and press, light themes | 1000, 1100 | darker too, which is up the steps there |
| A neutral fill holding white content | gray 400 (dark), 500 (light) | |
| A fill holding the primary text: the bar of a number field, selected text (`background.accent.muted.*`) | 500, hover 600 | the text reads on it on both sides |
| Row highlights under the primary text: hover, selected, selected and hovered, active | 400, 500, 600, 700 | |
| Subtle tints (status backgrounds) | 400 | |
| Accent text and links | 1400, states 1500 | 4.5:1 on hovered fields too |
| Icons in a color, focus rings, accent borders | 1300 | 3:1 on every surface |

Content on a colored fill is `semantic.text.color.on-accent.default`: white, black in the
high-contrast theme whose fills are bright. Static black and white (`semantic.static.*`) alias
`color.transparent-black.1000` and `color.transparent-white.1000`, which no theme changes: they
are the same in every theme, unlike gray 25 and 1000, whose ends swap between dark and light.

## Contrast guaranteed

`crates/koebi-tokens/tests/contrast.rs` checks every built-in theme. Content is paired with
what it is drawn on from the token paths (`<component>.label.color.<state>` on
`<component>.background.color.<state>`, icons and marks the same), so a new component, state or
theme is checked without a new test.

| What | Required | High contrast | WCAG 2.2 |
|---|---|---|---|
| Text: labels, values, placeholders, links, on every surface and on its fill | 4.5:1 | 7:1 | 1.4.3 (AA), 1.4.6 (AAA) |
| Content on colored fills, every state | 4.5:1 | 7:1 | 1.4.3, 1.4.6 |
| Text on row highlights, every state | 4.5:1 | 7:1 | 1.4.3 |
| Icons, marks, focus rings | 3:1 | 4.5:1 (icons 7:1) | 1.4.11, 2.4.13 |
| Control outlines, where a theme draws them | 3:1 against the panel | 7:1 | 1.4.11 |
| Disabled content | exempt | 4.5:1 | 1.4.3 exempts it |

### Accepted exceptions

Koebi is a dense tool, like Blender: the base theme does not reach everything WCAG asks for
non-text content, and says where.

- **Controls without outlines.** The dark themes without outlines (Dark, Dark pink) tell
  controls apart by their fill: a field (gray 300) on a panel (gray 150) is 1.6:1. WCAG 1.4.11
  asks 3:1 for what identifies a control; a field's label, its position in a property row and
  its value identify it here, but the boundary itself does not reach 3:1. Themes with outlined
  controls (Muted green, Snow, Arctic, High contrast) reach it, and the user can turn outlines
  on in any theme (`semantic.border.width.control`).
- **The bar of a number field** shows where the value lies in its range; the value itself is
  the text, which reaches 4.5:1. The bar is redundant with it and is not held to 3:1.
- **Hovered and pressed fills** keep 4.5:1 for their content; what tells the state is a change
  of the fill, which WCAG does not measure.
- **Disabled content** is exempt (WCAG 1.4.3), and stays visible: gray 500, 5.75:1 on the dark
  background.

## Themes

| Theme | Scheme | Neutral palette | Accent | Controls |
|---|---|---|---|---|
| Dark (base) | dark | neutral gray, #131313 background | blue | told apart by their fill |
| Dark pink | dark | Dark's | pink | as Dark |
| Muted green | dark | dark greens to a pale sepia, #161A14 background; hues at 80 % chroma | sage (celery made from #8EA066) | outlined |
| Snow | light | warm whites to a reddish brown text, #F8F5F3 background | salmon (cinnamon made from #D77B6B) | outlined |
| Arctic | light | cool whites to a blue-gray text, #F3F6F9 background | slate (indigo made from #64788C) | outlined |
| High contrast | dark | black surfaces, white content | bright cyan, black content on it | outlined at 2 px |

Muted green, Snow and Arctic follow RapidRAW's themes of those names (1.5): their backgrounds,
text tones and accents, made into palettes rather than copied as a handful of colors.

### What a theme changes

- **Its palettes**: the values of `color.gray.*` and the 18 hues, made for its background (the
  `values` block of its file, written by the generator).
- **Its accent**: `semantic.accent.color.*` aliases another hue (Dark pink, Muted green, Snow,
  Arctic, High contrast).
- **Its outlines**: `semantic.border.width.control` and `.surface` alias the thin width; floating surfaces (`.floating`: menus, tooltips, popups) follow `.surface`.
- **Light themes**: their surfaces (panels and fields white on a light background, nested
  surfaces on the background's tone), their secondary text (gray 600, tertiary 500), and the
  direction of states (hover and press go up the steps).
- **High contrast**: everything below.

### High contrast

The high-contrast theme follows WCAG 2.2 at level AAA where it applies to color, and the
principles of the systems' contrast modes (Windows contrast themes, macOS Increase contrast):
few colors, each with one meaning, no faint content, every boundary drawn.

- Black surfaces (luminance below 0.02) and white content: text 7:1 everywhere (1.4.6), the
  secondary and tertiary text white or nearly, never gray.
- Every control outlined at 2 px in white (7:1), separators and subtle borders 14:1.
- A focus ring of 3 px (2.4.13: at least 2 px, 3:1).
- Colored fills bright (step 1400) with black content (7:1 and more); accent text, icons and
  focus in step 1500.
- Disabled content at 4.5:1 or more: visible, told apart by its tone.
- No meaning by color alone (1.4.1): selection keeps its fill and its text, focus its ring, an
  error its border and its message.

## Color settings

*Settings › Theme › Colors* sit above the tokens: they change a whole family of colors from one
color, and the tokens stay there for finer changes (*Customisation*). Nothing in the built-in
themes uses them.

- **What can be made from a color**: the five roles (accent, informative, positive, notice,
  negative: `semantic.<role>.color.*`), the neutral grays (`primitive.color.gray.*`, tinted:
  the seed's hue and up to 0.06 of chroma, the theme's own lightness for the deepest surface and
  the background), and the 18 hue palettes (`primitive.color.<hue>.*`).
- **How**: the chosen color is the palette's source (`semantic.<role>.color.source`,
  `semantic.neutral.color.source`, `primitive.color.<hue>.source`, tokens like the others, so
  that the choice is saved and exported). Its scale, from black through it to white, gives each
  step the color at that step's contrast with the theme's background
  (`koebi_tokens::palettes::seed`), the dark themes' fills darkened for white content. A step
  is a contrast, so every state, part and component aliasing the steps keeps its contrast,
  whatever the color: `tests/contrast.rs` seeds every role with a pale yellow, a gray, nearly
  black, a saturated cyan and a magenta in every theme and checks every component again.
- **Where**: the steps become overrides of the active theme, or of every theme ("Every
  theme"), each made for that theme's own background: one palette, its light and dark versions.
  The reset dot removes them.
- **Never for every theme at once**: a palette is made for a theme's background, so an override
  of one of its tokens in the global layer (one color for every theme) contradicts it, and,
  global overrides coming first, would hide the palette made for the theme (#243). Making or
  resetting a palette removes the global overrides of its source and steps; a row shows its
  reset dot as soon as its palette differs from the theme's own (`palettes::is_changed`).
- **What a row shows**: the palette on the theme's background, the contrast of each step with
  it, and what its main steps hold: a fill with its content (900), text on a panel (1400), a fill
  under the primary text (500); for the grays, text on a panel and on a field, and a border.

The gallery's accent editor makes the accent palette the same way, with the same function
(`palettes::seed`), from three OKLCH fields: for the active theme, or for every theme in its
global scope.

## Rules for components

**Surfaces.** A surface's step says where it lies; content on it uses the text steps, which
reach 4.5:1 on every surface up to a hovered field (gray 350). A component placed on a surface
does not choose its colors from what lies below: it reads its own tokens, which alias the
semantic step for its use.

| Surface | Token | Holds |
|---|---|---|
| Window background | `background.base` (gray 50) | areas, the app bar |
| Panels | `background.raised` (gray 150) | controls, rows, text |
| Nested surfaces in a panel | `background.app.child` (gray 200) | the same, one level deeper |
| Fields | `background.app.frame` (gray 300), hovered 350 | their value |
| Floating surfaces: menus, dropdown lists, popups (the color picker) | `background.floating` (gray 250), its highlighted row `.floating.hover` (gray 350) | items, controls, text |
| Tooltips | `background.sunken` (gray 25) | their text |

**Floating surfaces stand apart from what they cover.** A menu opens over any surface, so its
background is a step apart from all of them, and its edge shows without an outline:

- **Dark themes**: gray 250, the first step lighter than every panel and nested surface. The
  nearest surface is the nested one (gray 200): 1.16:1 in Dark, 1.18:1 in Muted green; fields,
  lighter, are 1.2:1 away. Primary text reads on it at 9:1, secondary at 7.9:1; a highlighted row
  (gray 350) is 1.36:1 away from it.
- **Light themes** (Snow, Arctic): their surfaces lie on both sides of the background (white
  panels and fields, darker wells), so no step stands apart from all of them without being darker
  than the darkest. Floating surfaces stay white and keep the thin outline
  (`border.width.floating` aliases `border.width.thin` there, whatever the surfaces do); their
  highlighted row is gray 100.
- **High contrast**: floating surfaces are gray 250 (#191919) outlined in white like every
  surface; a highlighted row is black, white text at 21:1.
- **Tooltips** open over the control under the pointer, often a hovered field, and over menus:
  they take the deepest surface, apart from both (2.3:1 from a hovered field in Dark).

**Borders.** Every border width derives from `border.width.thin` (the primitive stroke width
100, 200 in high contrast) through the role of what it outlines: `border.width.control` (fields,
dropdowns, button groups, toggles), `.surface` (panels, nested surfaces) and `.floating` (menus,
tooltips, popups, which follows `.surface`). A theme outlines a role by aliasing it to the thin
width, and removing outlines is a matter of setting the role, or `thin` itself, to zero. Content
outlines (color swatches, key caps, theme cards, the window's edge) use `thin` directly: their
fill is arbitrary, so they always need an edge. Emphasis (a selected swatch or card) and focus
rings use `medium` and `focus`.

**Fills.** A fill holding white content uses step 900 of its role and
`text.color.on-accent`; a fill holding the primary text uses `background.accent.muted` (500) or a
row highlight (400 to 700). Never put the primary text on a 900 fill or white text on a muted
one: the contrast test fails for it.

**Variants for other surfaces.** When the same component must sit on a colored fill (a button
on an accent banner, a field on an image), it gets a variant aliasing static colors
(`semantic.static.*`, the same in every theme) rather than the surface steps, as Spectrum's
"static white" and "static black" variants do.

**New colors.** A component never holds a color literal for something a role or a surface
step expresses; a new use takes the step of the table above, or a new semantic token that names
one. A new theme gives its `ThemePalettes` in `palettes.rs`, runs the generator, and passes the
contrast tests.
