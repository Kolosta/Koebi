# The gallery

Until Koebi has editors, its window shows the design system gallery
([`apps/koebi/src/gallery/`](../../apps/koebi/src/gallery/)): every component, drawn from the
tokens, and the means to change those tokens while the application runs. Components have no
labels until Koebi renders text, so this page says what each part does.

## Layout

From top to bottom (the two middle panels stack on narrow windows):

1. **Theme cards**: one card per theme, each painted with that theme's own tokens, its
   overrides and the color vision setting included. The active theme's card has an accent
   border. Click a card to activate its theme.
2. **Components**, on the left: a button, icon buttons (plus, minus, cross), two toggle
   buttons, two checkboxes, a slider, a drag value, a separator, the twelve chart colors as
   swatches (click one to select it) and the three panel levels.
3. **Token editor**, on the right:
   - First row: the **scope** of the edits, *every theme* (four squares) or *the active theme*
     (one square), and on the right a cross that removes every override of that scope.
   - Three sliders edit the **accent color in OKLCH**: hue, chroma and lightness, beside a
     strip of the values each one reaches. They write `semantic.accent.color.700`, and
     `semantic.accent.color.600` a little lighter for hovered and pressed states.
   - Three drag values edit dimensions, beside a preview: the control corner radius
     (`semantic.radius.control`), the thin border width (`semantic.border.width.thin`) and
     the checkbox size (`component.checkbox.box-size.default`). Drag them, or click the
     arrows at their ends to step.
   - A dot at the start of a row means its token is overridden in the current scope; click
     the dot to remove that override.
4. **Color vision**: normal vision, the simulations of protanopia, deuteranopia, tritanopia
   and achromatopsia, then the corrections of the three dichromacies. Each button previews its
   mode on four chart colors; click one to apply it to every color token of the interface.
5. **Grid**: as many checkboxes, toggles, buttons and swatches as the window holds, about
   nine hundred in the default 1280 x 800 window. They are all interactive, and none of them
   reads a token while drawing: styles are resolved once per change of the design system.

Every edit shows in the same frame: when a frame changes the design system, the application
draws it again with the new styles before presenting it.

## Saved state

Theme, imported themes, overrides and color vision setting are saved
([ADR 0009](../adr/0009-design-state-persistence.md)) half a second after the last change,
when the application goes to the background, and when it quits. The file is `design.kds`
in the platform's preferences directory, given by SDL for the organization `Kolosta` and the
application `Koebi`:

| Platform | Directory |
|---|---|
| Windows | `%APPDATA%\Kolosta\Koebi\` |
| Linux | `$XDG_DATA_HOME/Kolosta/Koebi/` (usually `~/.local/share/Kolosta/Koebi/`) |
| macOS | `~/Library/Application Support/Kolosta/Koebi/` |
| Android, iOS | the application's private storage |

Delete the file to return to the built-in themes.

## Command line

The same changes can be made at startup, for scripts and tests; they are saved as soon as the
application starts:

```sh
cargo run -p koebi -- --import sunset.tokens.json     # a theme from a Design Tokens file
cargo run -p koebi -- --theme light
cargo run -p koebi -- --override semantic.radius.control=6 --override component.button.background.color.default=#ff8800
cargo run -p koebi -- --override "semantic.accent.color.700={primitive.color.green.700}"
cargo run -p koebi -- --color-vision simulate-deutan
cargo run -p koebi -- --data-dir target/design-state   # keep the state somewhere else
cargo run -p koebi -- --export light.tokens.json --export-schema koebi.tokens.json
```

`--import` reads a theme in the [Design Tokens format](README.md#import-and-export), refuses
the file with every problem and its line if anything is wrong, activates the theme and saves
the state at once. `--override` sets a global override: a value of the token's kind (`#rrggbb`
colors, numbers with an optional `px` or `ms` unit) or an alias written `{path}`. `--export`
writes the active theme (what it changes from the base theme, overrides included) once the
other options are applied, and `--export-schema` every token with its base definition.
`koebi --help` lists every option. The end-to-end tests (`apps/koebi/tests/e2e.rs`) use these options to check that the
state survives a restart.
