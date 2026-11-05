# Icons

Koebi's icons are SVG files in [`crates/koebi-icons/icons/`](../../crates/koebi-icons/icons/),
compiled at build time into vector icons the renderer draws from their curves
([ADR 0020](../adr/0020-vector-icons.md)). This page is the contract a file must meet; the build
checks every rule and fails with the file and the line of what breaks one.

## Adding an icon

1. Draw it on a square artboard (24 × 24 for new icons; Material's `0 -960 960 960` is accepted
   as it is), export it as plain SVG.
2. Save it as `icons/<category>/<name>.svg`: one level of folders, names in lowercase
   kebab-case starting with a letter (`chevron-down.svg`, `crop-21-9.svg`).
3. Build: the icon is the constant `koebi_icons::<category>::<NAME>`
   (`koebi_icons::navigation::CHEVRON_DOWN`). Paint it with `koebi_ui::paint_icon`, its colors
   from `IconStyle::ink` (the component's icon color for its state) or `IconPaint::Original`.

The gallery's *Vector icons* panel shows every icon at any size, recolored on hover and press,
or in its own colors.

## The file

- **A square view box.** `viewBox="0 0 24 24"`, or `width` and `height` without one. The whole
  view box is the icon: a drawing that stands against one side keeps the empty space on the
  other (`transform/align-justify-flex-end.svg`), so icons of a family line up. Do not crop the
  view box to the drawing.
- **Shapes**: `path` (every command, arcs included), `rect` (rounded too), `circle`, `ellipse`,
  `line`, `polyline`, `polygon`, inside `g` groups; `transform` on any of them.
- **Paint**: `fill` and `stroke` with a color (`#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()`,
  names, `currentColor`) or a gradient (`url(#id)`); `fill-rule`, `fill-opacity`,
  `stroke-opacity`, `opacity`; `stroke-width`, `stroke-linecap`, `stroke-linejoin`,
  `stroke-miterlimit`. These can be attributes or declarations of a `style` attribute.
- **Gradients**: `linearGradient` and `radialGradient` in `defs`, with `stop`s,
  `gradientUnits`, `gradientTransform` and `href` to another gradient; `spreadMethod="pad"`
  only, and a radial gradient's focal point on its center.
- **Refused**: text (`text`, `tspan`, `textPath`: convert text to paths), `image`, `use` and
  `symbol` (repeat the shape), `mask`, `clipPath` and `clip-path` (cut the shape instead),
  `filter`, `pattern`, `marker`, `foreignObject`, `style` elements, nested `svg`, links,
  `stroke-dasharray` (draw the dashes), more than eight colors.

## Colors and zones

Every distinct color of an icon is a **zone**; the interface recolors an icon zone by zone.
A zone's role comes from the `class` of what it colors:

| Class contains | Role | Recolored with |
|---|---|---|
| `primary` (not `secondary`) | primary | the component's icon color for its state |
| `secondary` | secondary | the same color, fainter (`icon.zone.secondary.opacity`) |
| `tertiary` | tertiary | the same color, fainter still (`icon.zone.tertiary.opacity`) |
| `accent` | accent | the accent (`icon.zone.accent.color`) |
| none | primary | |

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <rect class="ds-secondary" x="3" y="3" width="12" height="12" fill="#ea4335"/>
  <circle class="ds-primary" cx="15" cy="15" r="6" fill="#1a73e8" fill-opacity="0.6"/>
</svg>
```

Opacity stays with what it colors: the translucent circle above stays translucent in any
color. A gradient's stops belong to the zones of their colors, so a recolored gradient keeps its
shape. A standard interface icon has a single `ds-primary` ink.
