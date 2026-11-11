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
   (`koebi_icons::navigation::CHEVRON_DOWN`). Components take it (`Button::icon`,
   `MenuItem::icon`...); `Ui::icon(bounds, icon, color)` paints one in its component's icon
   color for its state, `koebi_ui::paint_icon` with `IconPaint::Original` in its own colors.

The gallery's *Vector icons* panel shows every icon at any size, recolored on hover and press,
or in its own colors.

**Settings › Icons** lists every icon by category. A click opens a preview of the icon in each
state and tone the interface gives icons (normal, hovered, pressed, active, disabled; accent,
negative, notice, neutral, yellow), in its own colors, two-toned or not, with its zones
recolored one by one for the preview alone, and the constant that names it
(`koebi_icons::settings::GENERAL`). The icon tokens (`component.icon.*`) are edited below.

Editors have icons of their own (`editors/*`), shown in their tabs and the type dropdown
(`EditorHost::icon`, `EditorHost::kind_icons`), as do the settings pages (`settings/*`) in the
settings' column, and the outliner's rows: the document, groups (`objects/folder`) and layers
(`objects/layer`, in the layer's color).

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

Zones are told apart by color: a secondary part needs a color of its own in the file
(`#9c9c9c` beside the `#e3e3e3` of the primary ink, so that the original colors show two tones
too), a class alone is not enough.

Opacity stays with what it colors: the translucent circle above stays translucent in any
color. A gradient's stops belong to the zones of their colors, so a recolored gradient keeps its
shape. A standard interface icon has a single `ds-primary` ink, or two for a two-tone icon.

## Style guide

Two families share the interface. Whatever the family, an icon reads at 16 px, its weight
matches its neighbours', and it stays inside its square.

**Content icons** (`actions`, `editors`, `objects`, `settings`, `shapes`, `text`, `tools`,
`transform`, `view`, `navigation`): [Material Symbols](https://fonts.google.com/icons), outlined, weight 400, grade 0,
optical size 24, unfilled, in their `0 -960 960 960` frame. New ones come from the same set
when it has them; drawn ones follow its rules:

- a 24-unit grid, the drawing in the 20-unit live area (2 units of margin), strokes and
  outlines 2 units wide (80 in the 960 frame), corners rounded by 2 units, ends square;
- outlines rather than solid shapes, a solid shape for what must stand out (a dot, a selected
  state);
- the icon's own space kept: an arrow pointing right sits right, the rest of the square empty.

**Interface glyphs** (`interface`, `input`): marks of controls and of keys, drawn for small
sizes on a 16-unit grid: strokes 1.5 units wide (1.25 for the chevrons that fold and open, which
are drawn larger), round caps and joins, about 2 units of margin, open shapes (a check, a
chevron) centered on the grid so that their axis falls on a pixel.

**Colors and zones.** One ink (`ds-primary`) for standard icons: the interface colors them by
the state of their component. A second ink (`ds-secondary`, drawn fainter) for the supporting
part of a two-tone icon (below). The accent (`ds-accent`) for a single part that tells a state
(the pressed button of a mouse). No more than these three in an interface icon; a gradient only
where the original colors show (`IconPaint::Original`: the logo, illustrations, colors that mean
something). Translucency belongs to the drawing (`fill-opacity`), never to emphasis: emphasis is
the role.

### Two-tone icons

An icon whose parts do not weigh the same draws the lesser part in the secondary ink, at
`component.icon.zone.secondary.opacity` of the main color (60 %), so that what matters reads
first. Which part is which follows three rules, in order:

1. **An action mark is primary**: a plus, a minus, an arrow, a slash, a check, the line or the
   color an action applies (strikethrough, underline, the color bar), the guide an alignment
   aligns to.
2. **What an action applies to is secondary**: the document of *New*, the window of *New window*,
   the disk of *Save as*, the eye struck by a slash, the letter that gets smaller, the boxes being
   aligned.
3. **Without an action mark, the outline of the main object is primary and what lies inside or
   behind it is secondary**: the picture inside a frame, the lines of a list beside its bullets
   or checks, a shadow, an object behind another, the flame of a rocket, a key hole, the curve
   between the points that shape it.

An icon drawn as one shape stays in one ink: its holes belong to the shape that holds them, so
its parts cannot be told apart without drawing it again (the grid of *Background grid* is the
holes of its frame). Splitting a Material icon moves whole subpaths, a hole always with the
outline around it. The gallery's *Two tones* button shows the icons in one tone or two
(`IconStyle::one_tone` and `IconStyle::ink`).

60 icons have two tones; `koebi-icons` checks a sample of them.

**Size.** Icons take the interface unit (`semantic.size.control-height`, 20 px) less
`component.icon.size.margin` (1 px) on each side: 18 px. Key caps take
`component.key-cap.icon.size` (12 px). The renderer puts an icon's center on the nearest half
pixel and gives it the side that keeps its edges on pixel edges: a symmetric glyph centered on a
pixel stays crisp along its axis.

**Cursors** (`cursors`): one ink, the hot spot at the center of the square; the application
draws them white over a dark outline (`apps/koebi/src/cursors.rs`), so that they read on any
background.

**The application's icon** (`app/icon.svg`, the logo of `app/logo.svg` with its inside opaque so
that the arrow reads on dark task bars): the windows get it at start-up, and the platforms' files
are drawn from it: `platform/windows/koebi.ico` (embedded in the executable by
`apps/koebi/build.rs`), `platform/macos/Koebi.icns`, `platform/ios/AppIcon*.png` (opaque, on the
window's background), `platform/android/.../mipmap-*/ic_launcher.png`. After changing it, run
`KOEBI_BLESS=1 cargo nextest run -p koebi platforms_icon_files` to draw them again; the test fails
while they differ from it.

## Icons still to draw or choose

The interface uses an icon wherever one helps recognition. These places have none yet:

- the pages of the settings (General, Language, Theme, Customisation, Accessibility, Inputs,
  Keymaps, Navigation, Icons, Dev), and the Icons page itself, still empty;
- the editors (Gallery, Outliner, Properties, Viewport), for their tabs and their menu;
- the kinds of the outliner's items (document, sketch, layer, colors, notes);
- the viewport's tools and the transform gizmo, with the viewport;
- the window buttons (minimize, maximize, restore, close) stay line glyphs drawn on whole
  physical pixels (`koebi_ui::AppBar`), as each system draws its own.
