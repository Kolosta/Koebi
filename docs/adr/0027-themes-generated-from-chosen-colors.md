# 0027. Themes generated from chosen colors, by contrast rules

- Status: Accepted
- Date: 2026-11-12
- Deciders: @Kolosta

## Context

Settings › Theme › Colors makes a palette again from one color ([ADR 0021](0021-contrast-ordered-palettes.md)):
the color's scale, from black through it to white, gives each step the color at that step's
contrast with the theme's background. Using it showed four limits:

- **The chosen color is not used.** It is a seed: the steps take the colors of its scale at their
  contrasts, so a chosen accent shows as the step of the same hue at 3.96:1, lighter or darker
  than chosen. The neutral seed only tints the grays: the background keeps the theme's own
  lightness, whatever color is chosen.
- **One palette at a time.** A new background asks for every palette again: each step is a
  contrast with the background, so hues and roles made for the old one lose their contrasts.
- **The structure is fixed.** A theme also decides which step each use takes:
  - raised surfaces darker or lighter than the background;
  - states one step darker or lighter;
  - white or dark content on fills.

  The built-in themes write these aliases by hand (Snow and Arctic share 45 of them). A dark
  theme given a light background keeps its dark structure, and a very dark accent has no room
  for darker states.
- **Nothing says what cannot be done.** A mid-gray background cannot hold readable text, and a
  background near black leaves no room for a darker surface. These cases happened silently.

The settings should offer levels of automation: choose the background only, the background and
the accent, or every palette. Choosing a color must change the whole theme coherently, by
written rules, as Spectrum's are.

## Decision

A theme's colors are **generated** from a few chosen colors by fixed rules. The token layers stay
as they are:
- primitive steps hold values;
- semantic tokens alias steps for uses;
- components alias semantic tokens.

The inputs are tokens. The generator writes its outputs as overrides of the theme, never as new
primitives: a transformation of the theme's own palettes, not a redefinition of the primitive
palette.

- **Inputs**, saved and exported with the theme:
  - `primitive.color.mode` (Integer): `0` every palette from its own color, `1` from the
    background, `2` from the background and the accent;
  - the palettes' sources. `semantic.neutral.color.source` is now the background
    itself (gray 50), and a source overridden in the theme is a *chosen* color.
- **Outputs** (`koebi_tokens::generate::regenerate`), written as theme overrides:
  - the 16 neutral steps;
  - the 18 hue palettes;
  - the roles' steps when derived or chosen;
  - the structure's aliases;
  - the content on each role's fills.

  An output equal to the theme's own definition is no override, so a theme with nothing chosen
  holds no override.
- **Rules**, each with its constants, in `docs/design-system/color.md` (*Generated themes*):
  1. scheme from the background's luminance;
  2. a readable background, moved out of the mid-tones: text needs 12:1 of room, its weakest
     pair (hint text on a field) keeping 4.5:1;
  3. room for the deepest surface, the chosen color becoming that surface near black or white;
  4. neutral steps at their contrasts, text steps compressed when the background caps them;
  5. hues made again for the new background;
  6. a chosen color is its palette's fill exactly, the other steps placed around it in log
     contrast. It is at least as far from the background as the theme's own fill (3.96:1 dark,
     4.81:1 light), or moves to that contrast;
  7. the accent from the background's hue, and the other roles harmonized toward it, in the two
     background modes;
  8. states at offsets from their default fill (dark themes toward the background, light ones
     away from it), turned the other way for a background of the other scheme;
  9. states and content chosen together: the first of the theme's offsets and their opposite,
     with white then black content, whose content reads on every state;
  10. the structure Snow and Arctic share when the background turns light, the base theme's
      when it turns dark.
- **Content per role.** `text.color.on-{info,positive,notice,negative,neutral}.default` join
  `on-accent`: a pale chosen fill takes black content while the others keep white.
- **What could not be honored is reported** (`generate::report`): a background moved for
  legibility or used as the deepest surface, a fill brought to its theme's contrast. The
  settings show it under the color.
- **The high-contrast theme's states** of its bright fills follow its accent's (hover 1500,
  pressed 1300): its black content kept 2.6:1 on them, which the generated themes' check found.

## Consequences

- The chosen colors are used as they are, apart from the reported cases. The rest of the theme
  follows by rules that a test checks over a grid of backgrounds, accents and modes:
  - text at 7:1 and 4.5:1;
  - borders at 3:1;
  - fills at 3:1 from the background;
  - content at 4.5:1 on its fill;
  - steps in contrast order;
  - surfaces apart.
- A background change rewrites about 400 overrides: the neutral steps, 18 hues, the roles and
  the structure. Dragging in a color field does it every frame, measured by `cargo xtask bench`.
- The built-in themes are unchanged: their own data, regenerated for their own background, gives
  their own values, which leave no override.
- Snow's and Arctic's shared aliases become data (`generate::LIGHT`), and their states offsets,
  both checked against the two theme files.
- Dark backgrounds lighter than about #363636 and light ones darker than #C4C4C4 move: Blender's
  #303030 stays.
- Overrides saved by the former seeding stay valid, read as chosen sources and generated again
  on the next change.

## Alternatives considered

- **Redefine the primitive palette per choice** (new primitives for each generated theme): the
  token tables are static and checked at compile time, and every alias would need moving. The
  overrides do the same on the existing steps.
- **Seeds only, as before, with more seeds**: still no exact color, no structure, nothing
  reported.
- **Infer the mode from what is overridden**: no token to save, but a palette edited by hand in
  the token editor would read as derived, and switching modes would rewrite choices. An explicit
  token is simpler and exported with the theme.
- **Material's dynamic color (HCT tones)**: tones are absolute lightness, not contrast with the
  background, so a tone's legibility changes with the background. Koebi's steps are contrasts
  already.
