# Porting report

Koebi's token set comes from the previous C++ application (`TokenSchema.cpp`): token names, values, aliases and per-theme aliases. The token system itself is new. This page lists every difference between the two token sets, so that nothing is lost silently.

## Summary

| | Tokens |
|---|---:|
| Source tokens | 1527 |
| Not ported (ImGui residues) | 75 |
| Renamed | 39 |
| Value changed in form, not in meaning | 39 |
| Koebi token definitions | 1463 |
| Koebi tokens, typography fields included | 1547 |

Koebi has added tokens since the port; they are listed in [Added after the port](#added-after-the-port). A few ported tokens point elsewhere since; they are listed in [Changed after the port](#changed-after-the-port).

Every source token not listed below is ported with the same path, the same value or the same alias, and the same per-theme aliases.

## Renamed

Paths keep the source notation (`.` between segments, `-` inside a segment). In Rust, `semantic.background.accent.kbd-focus` is `semantic::BACKGROUND_ACCENT_KBD_FOCUS`.

- **`font-size` joins the `font` family, like `font.weight` and the `font.size.*` aliases** (20):

  <details><summary>show</summary>

  - `primitive.font-size.10` → `primitive.font.size.10`
  - `primitive.font-size.20` → `primitive.font.size.20`
  - `primitive.font-size.30` → `primitive.font.size.30`
  - `primitive.font-size.40` → `primitive.font.size.40`
  - `primitive.font-size.50` → `primitive.font.size.50`
  - `primitive.font-size.60` → `primitive.font.size.60`
  - `primitive.font-size.70` → `primitive.font.size.70`
  - `primitive.font-size.80` → `primitive.font.size.80`
  - `primitive.font-size.90` → `primitive.font.size.90`
  - `primitive.font-size.100` → `primitive.font.size.100`
  - `primitive.font-size.110` → `primitive.font.size.110`
  - `primitive.font-size.120` → `primitive.font.size.120`
  - `primitive.font-size.130` → `primitive.font.size.130`
  - `primitive.font-size.140` → `primitive.font.size.140`
  - `primitive.font-size.150` → `primitive.font.size.150`
  - `primitive.font-size.160` → `primitive.font.size.160`
  - `primitive.font-size.170` → `primitive.font.size.170`
  - `primitive.font-size.180` → `primitive.font.size.180`
  - `primitive.font-size.190` → `primitive.font.size.190`
  - `semantic.font-size.default` → `semantic.font.size.default`

  </details>

- **`accent.color.visual` is also the group of the `accent.color.visual.*` tokens, and a token cannot be a group** (1):

  <details><summary>show</summary>

  - `semantic.accent.color.visual` → `semantic.accent.color.visual-base`

  </details>

- **`border-width` joins the `border` family, like `border.color`** (5):

  <details><summary>show</summary>

  - `semantic.border-width.thin` → `semantic.border.width.thin`
  - `semantic.border-width.medium` → `semantic.border.width.medium`
  - `semantic.border-width.thick` → `semantic.border.width.thick`
  - `semantic.border-width.focus` → `semantic.border.width.focus`
  - `semantic.border-width.none` → `semantic.border.width.none`

  </details>

- **`down` was ImGui's word for the pressed state; the semantic tier says `pressed`** (12):

  <details><summary>show</summary>

  - `component.frame.background.color.down` → `component.frame.background.color.pressed`
  - `component.button.background.color.down` → `component.button.background.color.pressed`
  - `component.combo.background.color.down` → `component.combo.background.color.pressed`
  - `component.header.background.color.down` → `component.header.background.color.pressed`
  - `component.scrollbar.grab.color.down` → `component.scrollbar.grab.color.pressed`
  - `component.checkbox.background.color.down` → `component.checkbox.background.color.pressed`
  - `component.checkbox.background.color.selected-down` → `component.checkbox.background.color.selected-pressed`
  - `component.separator.color.down` → `component.separator.color.pressed`
  - `component.resize-grip.color.down` → `component.resize-grip.color.pressed`
  - `component.capture-field.background.color.down` → `component.capture-field.background.color.pressed`
  - `component.icon-button.background.color.down` → `component.icon-button.background.color.pressed`
  - `component.dropdown.background.color.down` → `component.dropdown.background.color.pressed`

  </details>

## Values changed in form

Component tokens may hold values, but not alias primitive tokens, and the token model has no two-dimensional kind. These tokens keep their meaning in another form.

| Token | Source | Koebi | Why |
|---|---|---|---|
| `component.window.padding.horizontal.default` | → `semantic.config.item-spacing` | 8.0 px | two-dimensional padding: one token per axis |
| `component.window.padding.vertical.default` | → `semantic.config.item-spacing` | 4.0 px | two-dimensional padding: one token per axis |
| `component.frame.padding.horizontal.default` | → `semantic.config.item-inner-spacing` | 4.0 px | two-dimensional padding: one token per axis |
| `component.frame.padding.vertical.default` | → `semantic.config.item-inner-spacing` | 4.0 px | two-dimensional padding: one token per axis |
| `component.combo.padding.horizontal.default` | → `semantic.config.item-inner-spacing` | 4.0 px | two-dimensional padding: one token per axis |
| `component.combo.padding.vertical.default` | → `semantic.config.item-inner-spacing` | 4.0 px | two-dimensional padding: one token per axis |
| `component.key-cap.padding.horizontal.default` | → `semantic.config.item-inner-spacing` | 4.0 px | two-dimensional padding: one token per axis |
| `component.key-cap.padding.vertical.default` | → `semantic.config.item-inner-spacing` | 4.0 px | two-dimensional padding: one token per axis |
| `component.status-bar.padding.horizontal.default` | `{4.f, 2.f}` | 4.0 px | two-dimensional padding: one token per axis |
| `component.status-bar.padding.vertical.default` | `{4.f, 2.f}` | 2.0 px | two-dimensional padding: one token per axis |
| `component.capture-field.padding.horizontal.default` | → `semantic.config.item-spacing` | 8.0 px | two-dimensional padding: one token per axis |
| `component.capture-field.padding.vertical.default` | → `semantic.config.item-spacing` | 4.0 px | two-dimensional padding: one token per axis |
| `component.dropdown.padding.horizontal.default` | `{8.f, 2.f}` | 8.0 px | two-dimensional padding: one token per axis |
| `component.dropdown.padding.vertical.default` | `{8.f, 2.f}` | 2.0 px | two-dimensional padding: one token per axis |
| `component.menu.padding.horizontal.default` | `{8.f, 6.f}` | 8.0 px | two-dimensional padding: one token per axis |
| `component.menu.padding.vertical.default` | `{8.f, 6.f}` | 6.0 px | two-dimensional padding: one token per axis |
| `component.tooltip.padding.horizontal.default` | `{8.f, 6.f}` | 8.0 px | two-dimensional padding: one token per axis |
| `component.tooltip.padding.vertical.default` | `{8.f, 6.f}` | 6.0 px | two-dimensional padding: one token per axis |
| `component.zone-tab.padding.horizontal.default` | `{8.f, 2.f}` | 8.0 px | two-dimensional padding: one token per axis |
| `component.zone-tab.padding.vertical.default` | `{8.f, 2.f}` | 2.0 px | two-dimensional padding: one token per axis |
| `component.zone-tab.drop-preview.color.default` | → `primitive.color.white-transparent` | its value | component tokens do not alias primitives; they may hold the value |
| `component.zone-tab.drop-preview.anim-duration` | `0.06 s` | 60 ms | durations are expressed in milliseconds |
| `component.viewport.page.border` | → `primitive.color.transparent-black.500` | its value | component tokens do not alias primitives; they may hold the value |
| `component.viewport.page-name.background.hover` | → `primitive.color.transparent-white.300` | its value | component tokens do not alias primitives; they may hold the value |
| `component.viewport.origin.outline` | → `primitive.color.transparent-black.700` | its value | component tokens do not alias primitives; they may hold the value |
| `component.viewport.cursor.ring-accent` | → `primitive.color.static.red.900` | its value | component tokens do not alias primitives; they may hold the value |
| `component.viewport.cursor.axis-x` | → `primitive.color.static.red.800` | its value | component tokens do not alias primitives; they may hold the value |
| `component.viewport.cursor.axis-y` | → `primitive.color.static.green.600` | its value | component tokens do not alias primitives; they may hold the value |
| `component.viewport.thumbnail.border` | → `primitive.color.transparent-black.500` | its value | component tokens do not alias primitives; they may hold the value |
| `component.edit-handle.vertex` | → `primitive.color.gray.900` | its value | component tokens do not alias primitives; they may hold the value |
| `component.edit-handle.vertex-ring` | → `primitive.color.transparent-white.700` | its value | component tokens do not alias primitives; they may hold the value |
| `component.zone-overlay.split-line` | → `primitive.color.transparent-white.900` | its value | component tokens do not alias primitives; they may hold the value |
| `component.zone-overlay.join.keep` | → `primitive.color.transparent-white.200` | its value | component tokens do not alias primitives; they may hold the value |
| `component.zone-overlay.join.remove` | → `primitive.color.transparent-black.500` | its value | component tokens do not alias primitives; they may hold the value |
| `component.zone-overlay.join.residual` | → `primitive.color.transparent-black.700` | its value | component tokens do not alias primitives; they may hold the value |
| `component.zone-overlay.transform-dim` | → `primitive.color.transparent-black.500` | its value | component tokens do not alias primitives; they may hold the value |
| `component.node-box.padding.horizontal.default` | `{8.f, 6.f}` | 8.0 px | two-dimensional padding: one token per axis |
| `component.node-box.padding.vertical.default` | `{8.f, 6.f}` | 6.0 px | two-dimensional padding: one token per axis |
| `primitive.radius.full` | `0.5` (ratio) | 9999 px | the renderer clamps every corner radius to half the shortest side, so the result is the same and the value stays a plain dimension |

## Not ported

ImGui residues, with their source value or alias:

### ImGui style and input settings (`config`), not design values (63)

<details><summary>show</summary>

- `primitive.config.item-spacing` `{8.0f,4.0f}`
- `primitive.config.item-inner-spacing` `{4.0f,4.0f}`
- `primitive.config.cell-padding` `{4.0f,2.0f}`
- `primitive.config.touch-extra-padding` `{0.0f,0.0f}`
- `primitive.config.indent-spacing` `21.0`
- `primitive.config.columns-min-spacing` `6.0`
- `primitive.config.undo-steps` `256`
- `primitive.config.display-window-padding` `{19.0f,19.0f}`
- `primitive.config.display-safe-area-padding` `{3.0f,3.0f}`
- `primitive.config.separator-text-padding` `{20.0f,3.0f}`
- `primitive.config.button-text-align` `{0.5f,0.5f}`
- `primitive.config.selectable-text-align` `{0.0f,0.0f}`
- `primitive.config.separator-text-align` `{0.0f,0.5f}`
- `primitive.config.color-button-position` `1`
- `primitive.config.grab-min-size` `12.0`
- `primitive.config.log-slider-deadzone` `4.0`
- `primitive.config.color-marker-size` `3.0`
- `primitive.config.mouse-cursor-scale` `1.0`
- `primitive.config.hover-delay-stationary` `0.15`
- `primitive.config.hover-delay-short` `0.15`
- `primitive.config.hover-delay-normal` `0.4`
- `primitive.config.drag-threshold` `6.0`
- `primitive.config.anti-aliased-lines` `1`
- `primitive.config.anti-aliased-lines-use-tex` `1`
- `primitive.config.anti-aliased-fill` `1`
- `primitive.config.curve-tessellation-tol` `1.25`
- `primitive.config.circle-tessellation-max-error` `0.3`
- `primitive.config.tree-lines-flags` `0`
- `primitive.config.tree-lines-size` `1.0`
- `semantic.config.item-spacing` → `primitive.config.item-spacing`
- `semantic.config.item-inner-spacing` → `primitive.config.item-inner-spacing`
- `semantic.config.cell-padding` → `primitive.config.cell-padding`
- `semantic.config.touch-extra-padding` → `primitive.config.touch-extra-padding`
- `semantic.config.indent-spacing` → `primitive.config.indent-spacing`
- `semantic.config.columns-min-spacing` → `primitive.config.columns-min-spacing`
- `semantic.config.undo-steps` → `primitive.config.undo-steps`
- `semantic.config.display-window-padding` → `primitive.config.display-window-padding`
- `semantic.config.display-safe-area-padding` → `primitive.config.display-safe-area-padding`
- `semantic.config.separator-text-padding` → `primitive.config.separator-text-padding`
- `semantic.config.button-text-align` → `primitive.config.button-text-align`
- `semantic.config.selectable-text-align` → `primitive.config.selectable-text-align`
- `semantic.config.separator-text-align` → `primitive.config.separator-text-align`
- `semantic.config.color-button-position` → `primitive.config.color-button-position`
- `semantic.config.grab-min-size` → `primitive.config.grab-min-size`
- `semantic.config.log-slider-deadzone` → `primitive.config.log-slider-deadzone`
- `semantic.config.color-marker-size` → `primitive.config.color-marker-size`
- `semantic.config.mouse-cursor-scale` → `primitive.config.mouse-cursor-scale`
- `semantic.config.hover-delay-stationary` → `primitive.config.hover-delay-stationary`
- `semantic.config.hover-delay-short` → `primitive.config.hover-delay-short`
- `semantic.config.hover-delay-normal` → `primitive.config.hover-delay-normal`
- `semantic.config.drag-threshold` → `primitive.config.drag-threshold`
- `semantic.config.anti-aliased-lines` → `primitive.config.anti-aliased-lines`
- `semantic.config.anti-aliased-lines-use-tex` → `primitive.config.anti-aliased-lines-use-tex`
- `semantic.config.anti-aliased-fill` → `primitive.config.anti-aliased-fill`
- `semantic.config.curve-tessellation-tol` → `primitive.config.curve-tessellation-tol`
- `semantic.config.circle-tessellation-max-error` → `primitive.config.circle-tessellation-max-error`
- `semantic.config.tree-lines-flags` → `primitive.config.tree-lines-flags`
- `semantic.config.tree-lines-size` → `primitive.config.tree-lines-size`
- `primitive.config.preview-placement` `0`
- `semantic.config.preview-placement` → `primitive.config.preview-placement`
- `semantic.config.placement-preview-alpha` → `primitive.opacity.400`
- `primitive.config.show-corner-zones` `1`
- `semantic.config.show-corner-zones` → `primitive.config.show-corner-zones`

</details>

### ImGui behaviour setting with no visual role in Koebi (12)

<details><summary>show</summary>

- `component.window.border.hover-padding` `4.0`
- `component.window.size.min` `{32.0f,32.0f}`
- `component.window.menu-button.position` `1`
- `component.window.title.text-align` → `semantic.config.separator-text-align`
- `component.tab.size.min-width-base` `1.0`
- `component.tab.size.min-width-shrink` `80.0`
- `component.tab.close-button.min-width-selected` `-1.0`
- `component.tab.close-button.min-width-unselected` `0.0`
- `component.table.angled-headers.angle` `35.0`
- `component.table.angled-headers.text-align` → `semantic.config.separator-text-align`
- `component.docking.node.has-close-button` `1`
- `component.docking.separator.size.default` `2.0`

</details>

## Themes

The base theme is Dark: the token tables define it. The other themes come from the per-theme aliases of the source and list only what differs from Dark:

- Light (`light`): 20 aliases, and `semantic.background.app.disabled` added after the port.
- Muted green (`muted-green`): 16 aliases.
- High contrast (`high-contrast`): 24 aliases, and `semantic.background.app.disabled` added
  after the port.

`ThemeDefinition.cpp` holds no theme: it redefines tokens per interface zone (`settings`, `editors/viewport`, `devTest/...`) for every theme. Zones are not part of the token system yet, so these redefinitions are not ported.

## Kept as in the source

- `primitive.color.gray.875` (`#232323`) is lighter than `primitive.color.gray.850` (`#1E1E1E`), unlike the rest of the gray scale.
- Several semantic scale steps share a value (`semantic.size.xxs` and `semantic.size.xs` are both 12 px).

## Added after the port

Number fields: `component.number-field.step-button.background.opacity` and
`component.number-field.drag.coarse-factor`.

The `component.slider.*` tokens are gone with the component, at the maintainer's request: a number
field on a bar replaces it.

Trees: `component.list-row.highlight.margin.vertical`,
`component.list-row.highlight.margin.horizontal`, `component.list-row.highlight.corner-radius`,
`component.outliner.tree-line.color.dotted`, `component.outliner.tree-line.dash.default` and
`component.outliner.tree-line.gap.default`. `component.outliner.tree-line.width.default` holds
1 pixel instead of aliasing the width of the borders of controls: a theme without borders keeps
the lines. `component.list-row.expand.duration` and `.collapse.duration` alias the entrance and
exit durations (100 ms) instead of the expand and collapse ones (200 ms), at the maintainer's
request.

Menus and bars (the interface unit): `component.menu.item.background.color.selected-hover`,
`component.menu.item.label.color.selected-hover`, `component.menu.item.margin.horizontal`,
`component.menu.open.pointer-share`, `component.editor-top-bar.gap.default` and
`component.zone.corner.top-height.default`. `component.editor-top-bar.padding.default` goes from
6 to 2 pixels at the maintainer's request.

Number fields: `component.number-field.drag.bar-fine-factor` (a twentieth, Blender's sliders).

Application bar: `component.title-bar.button.pressed-lightness`,
`component.title-bar.padding.default`, `component.title-bar.gap.default`,
`component.title-bar.button.width.default`, `component.title-bar.glyph.size.default`,
`.stroke.default` and `.restore-offset`, `component.title-bar.resize-border.default`,
`component.title-bar.menu.padding.default` and `.corner-radius.default`. Settings:
`component.side-panel.width.default`.

Reset dots (#175): `component.panel.override-badge.size.default` (6 px) and `.size.hover`
(9 px), the dot of a changed value, which brings it back.

Shortcut editor (#177): `component.key-cap.border.width.default` (→ `semantic.border.width.thin`),
`component.key-cap.separator.color.default` (→ `semantic.text.color.secondary.default`),
`component.key-cap.gap.default` (3 px) and `component.key-cap.icon.size.default` (12 px).
`component.key-cap.padding.vertical.default` aliases `semantic.spacing.control-inset` instead
of holding 4 px: it is the space above and below a cap in its row.

Insets, chevrons and disabled states (#174): `semantic.spacing.control-inset` (2 pixels, the
inset around controls one interface unit high), which `component.editor-top-bar.padding.default`,
`component.zone-tab.padding.vertical.default` and `component.title-bar.padding.default` now
alias, and `component.panel.header.inset.default`, which adds it above and below a panel
header's content; `component.icon.stroke-width.chevron` (1.25 px); `semantic.background.app.disabled`
(gray 780, gray 75 in Light, gray 850 in High contrast), which every disabled control
background now aliases (`button`, `toggle`, `checkbox`, `dropdown`, `number-field`,
`text-field`); `semantic.disabled.selected-tint` (0.4), aliased by
`component.button.selected.disabled.tint` and `component.checkbox.selected.disabled.tint`.
Both chevron sizes grow by a quarter, at the maintainer's request:
`component.dropdown.chevron.size` from 8 to 10 pixels, `component.panel.chevron.size` and
`component.list-row.fold.size.default` from 10 to 12.5 pixels.

States and panels (#183): `semantic.background.app.active` (→ `semantic.background.app.control`)
and `semantic.border.color.active` (→ `semantic.border.color.focus.default`), the colors of a
control holding the input, which `component.text-field.*.editing`,
`component.number-field.*.drag` and `component.capture-field.*.recording` now alias;
`component.button.background.color.selected-pressed` and
`component.toggle.background.color.selected-pressed` (→ `semantic.accent.color.800`);
`component.button.background.color.active`, `.border.color.active` and `.label.color.active`
(a button's open menu, aliasing the selected colors' semantic tokens);
`component.capture-field.border.color.hover`, `.background.color.disabled` and
`.border.color.disabled`. At the maintainer's request, the capture field looks like a field:
`component.capture-field.background.color.default`, `.hover` and `.pressed` alias
`semantic.background.app.frame`, `.frame-hover` and `.child` instead of the raised and sunken
backgrounds (black under the pointer), and its recording border the active border instead of
the negative color; `component.toggle.background.color.hover` aliases
`semantic.background.app.frame-hover` instead of `semantic.background.sunken.default` (black);
`component.panel.body.background.color.level-3` aliases `semantic.background.app.control`
instead of `semantic.background.base.default`, the color of the window and menus.

Color fields and pickers (#187): `component.color-field.*` (an outline of its own, its color
→ `semantic.neutral.color.500` and `.300` under the pointer, its width a value of 1 px so that it
stays when the interface's borders go; the corner radius and height of controls; a
checkerboard of 4 px aliasing the swatches' colors) and `component.color-picker.*` (the area of
192 px, strips of 16 px, swatches of 40 px, a gap of 6 px; white marks outlined in black 1 px
wide, triangles 7 px deep passing the strips by 2 px; the share of a drag with Shift, 0.1).

Tooltips (#204): `component.tooltip.background.color.default` aliases the new
`semantic.background.inverse.default` (gray 100, gray 850 in Light, white in High contrast)
instead of the editor top bar's color, and its label the new `semantic.text.color.inverse.default`
(gray 900, white in Light, black in High contrast), so that tooltips stand out on every surface
and on menus; its border aliases `semantic.border.color.shadow.default`.
`component.tooltip.offset.default` is 6 px from the control (was 16 px from the pointer); new
`component.tooltip.margin.default` (6 px) and `component.tooltip.max-width.default` (360 px).

Ranges (#189): sizes that draw nothing at zero no longer accept it: `primitive.size.*` (and so
`semantic.size.*`) from 1 px, `component.checkbox.box-size.default` from 6 px, the chevrons and
icons of dropdowns, text fields, number fields and panels from 4 px, a number field's step
buttons from 4 px, `component.scrollbar.overlay.width.hover` from 1 px, the text caret and node
cables from 0.5 px, node ports from 2 px. Stroke and border widths keep accepting zero (no
border), as do margins, gaps and the overlay scrollbar at rest.

Curve editor (#188, temporary): `component.curve-editor.*`, new: the graph 160 px high with a
padding of 10 px, the background, border and border width of number fields, the corner radius
of controls, grid lines in `semantic.border.color.default`, the curve in
`semantic.border.color.accent.default` 2 px wide, keys of 8 px (10 px selected or under the
pointer), white, the accent when selected and `semantic.neutral.color.500` for the fixed ends,
outlined in black 1 px wide, taken by a press within 8 px.

Status bar (#186): `component.status-bar.size.height` aliases `semantic.size.control-height`
instead of holding 22 px, the height of what the bar holds, and
`component.status-bar.padding.vertical.default` aliases `semantic.spacing.control-inset`
instead of holding 2 px: the bar is one interface unit and its inset above and below.

Docking: `component.zone-tab.reveal.delay` (350 ms) and `component.zone-tab.reveal.grace`
(250 ms), which were constants in the source.

Input settings, as semantic tokens aliasing durations and spacings (8): `semantic.input.drag-threshold`,
`semantic.input.double-click.time`, `semantic.input.tooltip.delay`,
`semantic.input.tooltip.delay-short`, `semantic.input.tooltip.delay-stationary`,
`semantic.input.hover.delay`, `semantic.input.repeat.delay` and `semantic.input.repeat.interval`.

New component tokens for the first components of `koebi-ui` (#45). They alias semantic tokens,
or hold a value where no semantic token fits (25):

<details><summary>show</summary>

- `component.button.border.color.default` → `semantic.border.color.default`
- `component.button.border.width.default` → `semantic.border.width.none`
- `component.button.corner-radius.default` → `semantic.radius.control`
- `component.icon.size.default` = 16 px
- `component.icon.stroke-width.default` = 1.5 px
- `component.checkbox.mark.width.default` = 2 px
- `component.number-field.fill.color.default` (`drag-value` in the C++ port) → `semantic.background.accent.default`
- `component.panel.border.width.default` → `semantic.border.width.thin`
- `component.panel.padding.default` = 8 px
- `component.color-swatch.corner-radius.default` → `semantic.radius.xs`
- `component.color-swatch.border.color.default` → `semantic.border.color.default`
- `component.color-swatch.border.color.hover` → `semantic.border.color.strong.default`
- `component.color-swatch.border.color.selected` → `semantic.border.color.focus.default`
- `component.color-swatch.border.width.default` → `semantic.border.width.thin`
- `component.color-swatch.border.width.selected` → `semantic.border.width.medium`
- `component.color-swatch.checker.color.light` → `semantic.neutral.color.500`
- `component.color-swatch.checker.color.dark` → `semantic.neutral.color.700`
- `component.color-swatch.checker.size.default` = 6 px
- `component.theme-card.corner-radius.default` → `semantic.radius.m`
- `component.theme-card.border.color.default` → `semantic.border.color.default`
- `component.theme-card.border.color.hover` → `semantic.border.color.strong.default`
- `component.theme-card.border.color.selected` → `semantic.background.accent.default`
- `component.theme-card.border.width.default` → `semantic.border.width.thin`
- `component.theme-card.border.width.selected` → `semantic.border.width.medium`
- `component.theme-card.padding.default` = 4 px

</details>

New component tokens for the workspace (#72), scroll areas (#71) and lists (#73) (23):

<details><summary>show</summary>

- `component.editor.corner-radius.default` → `semantic.radius.m`
- `component.zone.separator.hit-size.default` = 8 px
- `component.zone.min-size.default` = 40 px
- `component.zone.corner.size.default` = 12 px
- `component.zone-overlay.split-preview` = `#FFFFFF24`
- `component.zone-overlay.join.frame-width` → `semantic.border.width.medium`
- `component.scrollbar.wheel-step.default` = 48 px
- `component.outliner.tree-line.color.default` → `semantic.border.color.tree-line.default`
- `component.outliner.tree-line.width.default` → `semantic.border.width.thin`
- `component.list-row.height.default` → `semantic.size.control-height`
- `component.list-row.indent.default` = 16 px
- `component.list-row.padding.horizontal.default` = 4 px
- `component.list-row.gap.default` = 4 px
- `component.list-row.fold.size.default` = 10 px
- `component.list-row.fold.color.default` → `semantic.text.color.secondary.default`
- `component.list-row.expand.duration` → `semantic.animation.duration.expand`
- `component.list-row.collapse.duration` → `semantic.animation.duration.collapse`
- `component.list-row.swatch.size.default` = 12 px
- `component.list-row.swatch.corner-radius.default` → `semantic.radius.xs`
- `component.list-row.count.color.default` → `semantic.text.color.secondary.default`
- `component.list-row.drop-line.color.default` → `semantic.background.accent.default`
- `component.list-row.drop-line.width.default` = 2 px
- `component.drop-target.border.color.default` → `semantic.background.drop-target.default`

</details>

New component tokens for property groups (#116) (6): `component.property-group.row.gap.default`
= 2 px, `component.property-group.row.height.default` → `semantic.size.control-height`,
`component.property-group.label.width-ratio` = 0.35,
`component.property-group.column.gap.default` = 8 px,
`component.property-group.label.color.default` → `semantic.text.color.secondary.default`,
`component.property-group.label.color.disabled` → `semantic.text.color.disabled.default`.

New component tokens for collapsible panels (#117) (11):
`component.panel.header.height.default` → `semantic.size.control-height`,
`component.panel.header.padding.default` = 6 px, `component.panel.header.gap.default` = 4 px,
`component.panel.header.hover.lightness` = 0.04, `component.panel.chevron.size` = 10 px,
`component.panel.chevron.color.default` → `semantic.text.color.secondary.default`,
`component.panel.content.padding.vertical` = 6 px, `component.panel.expand.duration` →
`semantic.animation.duration.expand`, `component.panel.collapse.duration` →
`semantic.animation.duration.collapse`, `component.panel.expand.curve` →
`semantic.animation.curve.expand`, `component.panel.collapse.curve` →
`semantic.animation.curve.collapse`.

New component tokens for the viewport and timeline templates (#76) (18):

<details><summary>show</summary>

- `component.viewport.grid.color` → `semantic.border.color.tree-line.default`
- `component.viewport.grid.spacing` = 24 px
- `component.viewport.page.background` → `semantic.static.text.color.white`
- `component.viewport.ruler.size` = 20 px
- `component.viewport.ruler.background` → `semantic.background.raised.default`
- `component.viewport.ruler.tick` → `semantic.border.color.default`
- `component.viewport.ruler.label` → `semantic.text.color.secondary.default`
- `component.viewport.zoom-step` = 1.25
- `component.timeline.ruler.height` → `semantic.size.control-height`
- `component.timeline.ruler.background` → `semantic.background.raised.default`
- `component.timeline.ruler.tick` → `semantic.border.color.default`
- `component.timeline.ruler.label` → `semantic.text.color.secondary.default`
- `component.timeline.playhead.color` → `semantic.background.accent.default`
- `component.timeline.playhead.label` → `semantic.static.text.color.white`
- `component.timeline.keyframe.color` → `semantic.icon.color.secondary.default`
- `component.timeline.keyframe.size` = 8 px
- `component.timeline.frame.width` = 12 px
- `component.timeline.range.dim` → `semantic.background.dim.default`

</details>

New component tokens for buttons, button groups, toggle buttons and checkboxes (#108, #111):
the states every button, toggle and checkbox has (hovered and pressed borders, selected,
disabled), their metrics, the colored variants, the color families and the group borders (89):

<details><summary>show</summary>

- `component.button.border.color.hover` → `semantic.border.color.strong.default`
- `component.button.border.color.pressed` → `semantic.border.color.strong.default`
- `component.button.background.color.selected` → `semantic.background.accent.default`
- `component.button.background.color.selected-hover` → `semantic.background.accent.hover`
- `component.button.label.color.selected` → `semantic.static.text.color.white`
- `component.button.border.color.selected` → `semantic.border.color.accent.default`
- `component.button.background.color.disabled` → `semantic.background.app.disabled`
- `component.button.label.color.disabled` → `semantic.text.color.disabled.default`
- `component.button.border.color.disabled` → `semantic.border.color.disabled.default`
- `component.button.height.default` → `semantic.size.control-height`
- `component.button.min-width.default` → `semantic.size.control-height`
- `component.button.padding.horizontal.default` = 8 px
- `component.button.gap.default` = 4 px
- `component.button.status-dot.size.default` = 6 px
- `component.button.accent.background.color.default` → `semantic.background.accent.default`
- `component.button.accent.background.color.hover` → `semantic.background.accent.hover`
- `component.button.accent.background.color.pressed` → `semantic.accent.color.800`
- `component.button.accent.label.color.default` → `semantic.static.text.color.white`
- `component.button.accent.border.color.default` → `semantic.border.color.accent.default`
- `component.button.positive.background.color.default` → `semantic.background.positive.default`
- `component.button.positive.background.color.hover` → `semantic.background.positive.hover`
- `component.button.positive.background.color.pressed` → `semantic.positive.color.800`
- `component.button.positive.label.color.default` → `semantic.static.text.color.white`
- `component.button.positive.border.color.default` → `semantic.positive.color.500`
- `component.button.negative.background.color.default` → `semantic.background.negative.default`
- `component.button.negative.background.color.hover` → `semantic.background.negative.hover`
- `component.button.negative.background.color.pressed` → `semantic.negative.color.800`
- `component.button.negative.label.color.default` → `semantic.static.text.color.white`
- `component.button.negative.border.color.default` → `semantic.border.color.negative.default`
- `component.button.info.background.color.default` → `semantic.background.info.default`
- `component.button.info.background.color.hover` → `semantic.background.info.hover`
- `component.button.info.background.color.pressed` → `semantic.info.color.800`
- `component.button.info.label.color.default` → `semantic.static.text.color.white`
- `component.button.info.border.color.default` → `semantic.info.color.500`
- `component.button.neutral.background.color.default` → `semantic.background.neutral.default`
- `component.button.neutral.background.color.hover` → `semantic.background.neutral.hover`
- `component.button.neutral.background.color.pressed` → `semantic.neutral.color.600`
- `component.button.neutral.label.color.default` → `semantic.static.text.color.white`
- `component.button.neutral.border.color.default` → `semantic.border.color.strong.default`
- `component.button.notice.background.color.default` → `semantic.background.notice.default`
- `component.button.notice.background.color.hover` → `semantic.notice.color.500`
- `component.button.notice.background.color.pressed` → `semantic.notice.color.700`
- `component.button.notice.label.color.default` → `semantic.static.text.color.black`
- `component.button.notice.border.color.default` → `semantic.notice.color.500`
- `component.button.shade.hover.lightness` = 0.06
- `component.button.shade.pressed.lightness` = -0.05
- `component.button.hue.disabled.tint` = 0.35
- `component.button.hue.blue.fill` → `semantic.background.blue.default`
- `component.button.hue.blue.subtle` → `semantic.background.blue.subtle.default`
- `component.button.hue.blue.text` → `semantic.text.color.blue.default`
- `component.button.hue.cyan.fill` → `semantic.background.cyan.default`
- `component.button.hue.cyan.subtle` → `semantic.background.cyan.subtle.default`
- `component.button.hue.cyan.text` → `semantic.text.color.cyan.default`
- `component.button.hue.green.fill` → `semantic.background.green.default`
- `component.button.hue.green.subtle` → `semantic.background.green.subtle.default`
- `component.button.hue.green.text` → `semantic.text.color.green.default`
- `component.button.hue.yellow.fill` → `semantic.background.yellow.default`
- `component.button.hue.yellow.subtle` → `semantic.background.yellow.subtle.default`
- `component.button.hue.yellow.text` → `semantic.text.color.yellow.default`
- `component.button.hue.orange.fill` → `semantic.background.orange.default`
- `component.button.hue.orange.subtle` → `semantic.background.orange.subtle.default`
- `component.button.hue.orange.text` → `semantic.text.color.orange.default`
- `component.button.hue.red.fill` → `semantic.background.red.default`
- `component.button.hue.red.subtle` → `semantic.background.red.subtle.default`
- `component.button.hue.red.text` → `semantic.text.color.red.default`
- `component.button.hue.magenta.fill` → `semantic.background.magenta.default`
- `component.button.hue.magenta.subtle` → `semantic.background.magenta.subtle.default`
- `component.button.hue.magenta.text` → `semantic.text.color.magenta.default`
- `component.button.hue.purple.fill` → `semantic.background.purple.default`
- `component.button.hue.purple.subtle` → `semantic.background.purple.subtle.default`
- `component.button.hue.purple.text` → `semantic.text.color.purple.default`
- `component.button.group.border.width.default` → `semantic.border.width.thin`
- `component.checkbox.border.color.hover` → `semantic.border.color.strong.default`
- `component.checkbox.background.color.disabled` → `semantic.background.app.disabled`
- `component.checkbox.border.color.disabled` → `semantic.border.color.disabled.default`
- `component.checkbox.mark.color.disabled` → `semantic.text.color.disabled.default`
- `component.checkbox.label.color.default` → `semantic.text.color.primary.default`
- `component.checkbox.label.color.disabled` → `semantic.text.color.disabled.default`
- `component.checkbox.description.color.default` → `semantic.text.color.secondary.default`
- `component.checkbox.description.color.disabled` → `semantic.text.color.disabled.default`
- `component.checkbox.gap.default` = 6 px
- `component.checkbox.line-height.default` → `semantic.size.control-height`
- `component.toggle.background.color.pressed` → `semantic.background.app.child`
- `component.toggle.background.color.selected-hover` → `semantic.background.accent.hover`
- `component.toggle.background.color.disabled` → `semantic.background.app.disabled`
- `component.toggle.border.color.hover` → `semantic.border.color.strong.default`
- `component.toggle.border.color.disabled` → `semantic.border.color.disabled.default`
- `component.toggle.label.color.hover` → `semantic.text.color.primary.default`
- `component.toggle.label.color.disabled` → `semantic.text.color.disabled.default`

</details>

Input settings for submenus and menus, and component tokens for menu items, tooltips and
dropdowns (#109, #110) (22):

<details><summary>show</summary>

- `semantic.input.submenu.delay` → `primitive.duration.normal`
- `semantic.input.submenu.grace` → `primitive.duration.slow`
- `semantic.input.menu.close-distance` → `primitive.size.400`
- `component.menu.item.height.default` → `semantic.size.control-height`
- `component.menu.item.label.color.default` → `semantic.text.color.primary.default`
- `component.menu.item.label.color.hover` → `semantic.text.color.primary.default`
- `component.menu.item.label.color.selected` → `semantic.static.text.color.white`
- `component.menu.item.label.color.disabled` → `semantic.text.color.disabled.default`
- `component.menu.item.shortcut.color.default` → `semantic.text.color.secondary.default`
- `component.menu.item.icon.color.default` → `semantic.icon.color.primary.default`
- `component.menu.item.mark.color.default` → `semantic.text.color.primary.default`
- `component.menu.item.icon-gap.default` = 6 px
- `component.menu.item.radio.size.default` = 6 px
- `component.menu.wheel-step.default` = 48 px
- `component.tooltip.offset.default` = 16 px
- `component.dropdown.background.color.disabled` → `semantic.background.app.disabled`
- `component.dropdown.border.color.open` → `semantic.neutral.color.600`
- `component.dropdown.border.color.disabled` → `semantic.border.color.disabled.default`
- `component.dropdown.label.color.disabled` → `semantic.text.color.disabled.default`
- `component.dropdown.icon.color.disabled` → `semantic.text.color.disabled.default`
- `component.dropdown.gap.default` = 4 px
- `component.dropdown.min-width.default` → `semantic.size.control-height`

</details>

Vector icons (#205, #206): `component.icon.zone.secondary.opacity` (→ `semantic.opacity.moderate`),
`component.icon.zone.tertiary.opacity` (→ `semantic.opacity.reduced`),
`component.icon.zone.accent.color` (→ `semantic.icon.color.accent.default`), how icons drawn in
one color paint their zones, and `component.icon.size.margin` (1 px). `component.icon.size.default`
(16 px), `component.icon.stroke-width.default` and `component.icon.stroke-width.chevron` are
removed: icons take the interface unit less the margin on each side (18 px), and their strokes
are drawn in their SVG files ([icons](../development/icons.md)).

## Changed after the port

- `component.button.background.color.default`, `.hover` and `.pressed` aliased
  `semantic.background.accent.*`: in the source, every button was an accent button. Buttons
  now have variants ([ADR 0017](../adr/0017-component-library.md)): the default variant is a
  neutral control (`semantic.background.app.frame`, `semantic.background.app.frame-hover` and
  `semantic.background.app.child`), and the accent button reads the new
  `component.button.accent.*` tokens, which alias `semantic.background.accent.*`.
- `component.toggle.*` now styles the toggle buttons of the default tone (`Button::toggle`),
  completed with the states it lacked (pressed, selected and hovered, disabled).
- `component.toggle.border.width.default` held 1 px of its own; it aliases
  `semantic.border.width.thin`, like the button groups that toggles stand beside (#217).
- Tooltips (#204, #217) use the theme's own colors: `component.tooltip.background.color.default`
  → `semantic.background.base.default`, `component.tooltip.label.color.default` →
  `semantic.text.color.primary.default`, `component.tooltip.border.color.default` →
  `semantic.border.color.subtle.default`.
- The frame-time overlay (#216) has its own tokens instead of the tooltip's:
  `component.status-bar.frame-times.background.color.default` →
  `semantic.background.sunken.default` at `component.status-bar.frame-times.opacity.default`
  (0.72), `.label.color.default` → `semantic.text.color.primary.default`,
  `.corner-radius.default` → `semantic.radius.control`, `.padding.horizontal.default` (8 px) and
  `.padding.vertical.default` (6 px).
- **Palettes ordered by contrast** (#60, [ADR 0021](../adr/0021-contrast-ordered-palettes.md),
  [color](color.md)). Palette steps are contrasts with the theme's background, and each theme
  has its own palette values; the dark theme keeps its look.
  - Gray steps: 16 contrast steps. Old → new: 25 → 1000, 50 and 75 → 1000, 100 → 900,
    200 and 300 → 800, 400 → 700, 500 → 500, 600 → 400, 700 → 350, 740 → 300, 760 → 250,
    780 → 200, 800 → 150, 850 → 75, 875 → 100, 900 → 50, 1000 → 25. `color.gray.740`, `.760`,
    `.780`, `.850` and `.875` are removed; `.150`, `.250` and `.350` are new. The neutral role
    (`semantic.neutral.color.*`) follows the same steps.
  - Hue steps in the dark base theme are now dark-theme values; references moved to the step of
    the same contrast against #131313: 100 and 200 → 1600, 300 → 1500, 400 → 1400, 500 → 1300,
    600 → 1200, 700 → 1100, 800 → 1000, 900 → 900, 1000 → 800, 1100 → 700, 1200 → 600,
    1300 → 500, 1400 → 400, 1500 and 1600 → 200.
  - Contrast decisions (#60): accent, positive, negative, informative and notice fills on step
    900 (hover 800, pressed 700) under the new `semantic.text.color.on-accent.default` (white);
    neutral fills on gray 400; `semantic.background.accent.muted.default` and `.hover` (500, 600)
    for the number field's bar and selected text; row highlights (`semantic.accent.color.hover.*`,
    `.selected.*`, `.hover-selected.*`, `.active.*`) on 400, 500, 600 and 700; placeholder text on
    gray 700; accent text and links on 1400; text field and dropdown outlines on
    `semantic.border.color.default`; the toggle's selected label on the accent content color.
  - Border widths by role: `semantic.border.width.control` and `.surface` (none in the base
    theme), `.floating` (thin) for menus, tooltips and popups.
  - Static black and white (`semantic.static.*`) alias the opaque transparent-black and white
    primitives, the same in every theme.
  - Themes: Light is replaced by Snow and Arctic; Dark pink is new; Muted green and High
    contrast are made again with their own palettes.
