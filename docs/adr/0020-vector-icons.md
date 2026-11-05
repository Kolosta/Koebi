# ADR 0020: Vector icons compiled at build time, covered from their curves in the shader

## Status

Accepted

## Context

Koebi's icons were line drawings written in code (`koebi_ui::Icon`): a few strokes per icon,
one color, no fills, no gradients, nothing a designer can draw. The C++ application this one
follows used SVG files rasterized at run time by a library, cached as textures per size and
color: an icon at a new size or in a new color cost a rasterization and a texture, and a tinted
icon lost its other colors.

The interface needs icons that are:

- drawn by designers as SVG files, in any tool, and checked, so that what ships is what was drawn;
- sharp at every size and at every display scale, resized in real time at no cost per size;
- multicolored, with gradients and transparency, yet recolored per instance from the design
  tokens and the state of their component (hovered, pressed, selected, disabled), or shown in
  their own colors;
- cheap: hundreds per frame, one draw call, nothing allocated per frame;
- free of new dependencies (ADR 0003).

## Decision

**Two internal crates and a shader.**

- **`koebi-svg`** (std only) compiles an SVG file into an icon: layers of quadratic Béziers in
  the unit square of the icon, each filled with a color or a gradient whose colors belong to
  color zones. It reads a subset of SVG (paths, basic shapes, groups, transforms, fills and
  strokes with their caps and joins, linear and radial gradients, opacity, `class` and `style`)
  and refuses everything else with the line it found it on: text, images, `use`, masks,
  clipping, filters, patterns, dashes, a focal point. Strokes become fills at compile time
  (offset curves, pie-wedge round joins, half-disc caps), shapes and arcs become curves, cubic
  curves become quadratics within 10⁻⁴ of the icon's side. Every distinct color is a zone (eight
  at most); its role (primary, secondary, tertiary, accent) comes from the classes of what it
  colors (`ds-primary`, ...), as in the files of the previous application. The crate also packs
  icons for the GPU and rasterizes them on the CPU with exactly the shader's coverage.
- **`koebi-icons`** holds the SVG files (`icons/<category>/<name>.svg`). Its build script
  compiles them all: a file outside the subset, a name that is not kebab-case, a view box that is
  not square fails the build with the file and the line. It generates one constant per icon
  (`koebi_icons::navigation::CHEVRON_DOWN`), the zones' roles, and the set packed for the GPU.
  Adding an icon is adding a file.
- **The renderer** (`koebi-render`) draws an icon as one instance (160 bytes: its square, its
  number, the color of each zone or none, an opacity, a rotation). The fragment shader reads the
  icon's curves from the set and computes the pixel's coverage itself: nothing is rasterized
  ahead of time, so every size costs the same and nothing is cached per size or color.

**Coverage from two rays.** For a pixel, a ray leaves its center to the right and another
downward. Every curve is monotonic in x and in y (the compiler splits them at their turning
points), so it crosses each ray at most once; a crossing counts as the share of the pixel on the
near side of it (a one-pixel box filter), signed by the curve's direction. The sum along a ray
is the winding number blurred across the pixel; the fill rule turns it into a coverage (nonzero
clamps it at 1, even-odd folds it), and the two rays' coverages are mixed by how close their
crossings are to the center, after the Slug algorithm (Lengyel, 2017). Layers are composited in
order. Because the clamped winding is what counts, shapes that overlap (the pieces of a stroke,
two subpaths of a fill) show no seam where they meet, and features thinner than a pixel keep
their weight. The CPU rasterizer runs the same arithmetic; tests require both to agree within
two levels in 255 on every pixel.

**The set lives in a data texture.** The packed icons (a header, then per icon its layers, zones,
curves and gradient stops, as `vec4`) are uploaded once into an RGBA32F texture 4096 texels wide,
read with `texelFetch` through the bind group layout the glyph atlas already uses. A storage
buffer would read the same data, but a bind group would have to keep a buffer alive while frames
in flight use it, which `koebi-gpu` only does for textures today; the texture costs nothing more.
`Renderer2D::set_icons` validates a set before taking it (`Packed::validate`: every offset and
count inside the data, at most 2²⁴ texels and 65 536 curves per icon), the shader guards every read
against the length in the header and draws nothing for an icon number past the set: neither a set
built by hand nor a scene can make it read outside the data. The format is a data format only:
`koebi-gpu` refuses it as a render target, which Vulkan does not require it to be.

**Recoloring.** An instance gives a color per zone, or none to keep the file's. The interface
maps roles to colors (`IconStyle::ink`): primary zones take the component's icon color for its
state, secondary and tertiary zones the same color fainter (`icon.zone.*.opacity` tokens),
accent zones the accent (`icon.zone.accent.color`); a disabled icon takes one color for all.
`IconPaint::Original` shows the file's colors: logos, illustrations, symbols whose colors mean
something. A gradient's stops follow the zones of their colors, so a recolored gradient keeps its
shape.

## Consequences

- An icon is sharp at any size and scale, recolored per frame for free; hovering a toolbar
  recolors its icons without any texture work. Icons snap to whole physical pixels.
- The SVG subset is a contract checked at every build; `docs/development/icons.md` documents it
  and the conventions. A designer exports plain shapes (no text, no masks, no filters).
- Cost grows with the curves under a pixel: per layer, a pixel outside the layer's bounds by
  more than half a pixel skips it. The 131 icons of the first set hold 4 700 curves, 36 on
  average; a stroke heavy icon reaches a thousand, still far below what a frame can afford.
- The CPU rasterizer gives cursors and the application's icon (window, taskbar, platform icon
  files) from the same files.
- Eight zones per icon at most: a file with more colors is refused. Masks, clipping, filters,
  text and images are refused rather than drawn wrong; supporting one later is a compiler change
  with tests, never a silent difference.
