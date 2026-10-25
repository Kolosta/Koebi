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

Koebi has added tokens since the port; they are listed in [Added after the port](#added-after-the-port).

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

- **`down` was ImGui's word for the pressed state; the semantic tier says `pressed`** (13):

  <details><summary>show</summary>

  - `component.frame.background.color.down` → `component.frame.background.color.pressed`
  - `component.button.background.color.down` → `component.button.background.color.pressed`
  - `component.combo.background.color.down` → `component.combo.background.color.pressed`
  - `component.header.background.color.down` → `component.header.background.color.pressed`
  - `component.scrollbar.grab.color.down` → `component.scrollbar.grab.color.pressed`
  - `component.slider.grab.color.down` → `component.slider.grab.color.pressed`
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

- Light (`light`): 20 aliases.
- Muted green (`muted-green`): 16 aliases.
- High contrast (`high-contrast`): 24 aliases.

`ThemeDefinition.cpp` holds no theme: it redefines tokens per interface zone (`settings`, `editors/viewport`, `devTest/...`) for every theme. Zones are not part of the token system yet, so these redefinitions are not ported.

## Kept as in the source

- `primitive.color.gray.875` (`#232323`) is lighter than `primitive.color.gray.850` (`#1E1E1E`), unlike the rest of the gray scale.
- Several semantic scale steps share a value (`semantic.size.xxs` and `semantic.size.xs` are both 12 px).

## Added after the port

New component tokens for the first components of `koebi-ui` (#45). They alias semantic tokens,
or hold a value where no semantic token fits (30):

<details><summary>show</summary>

- `component.button.border.color.default` → `semantic.border.color.default`
- `component.button.border.width.default` → `semantic.border.width.none`
- `component.button.corner-radius.default` → `semantic.radius.control`
- `component.icon.size.default` = 16 px
- `component.icon.stroke-width.default` = 1.5 px
- `component.checkbox.mark.width.default` = 2 px
- `component.slider.grab.color.hover` → `semantic.background.accent.hover`
- `component.slider.grab.width.default` = 10 px
- `component.slider.grab.padding.default` = 2 px
- `component.slider.track.color.default` → `semantic.background.app.frame`
- `component.slider.track.color.hover` → `semantic.background.app.frame-hover`
- `component.drag-value.fill.color.default` → `semantic.background.accent.default`
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
