# 0030. The interface in zoomable spaces: every component scales with the space it lies in

- Status: Accepted
- Date: 2026-11-17
- Deciders: @Kolosta

## Context

The Node Graph editor (ADR 0028) shows nodes on an infinite canvas that pans and zooms, as in
Blender. A node is an interface component holding other components:
- sockets, labels, number fields, color fields, dropdowns, checkboxes;
- previews of what the node outputs.

Zooming must scale all of them together. They must stay usable at every scale: hover, clicks,
drags, text editing, menus.

Other places want the same thing:
- vignettes and previews that zoom;
- the Typography module's glyph canvas;
- future presentations of components on the canvas itself.

The maintainer's requirement: every component works inside a zoomable space, or inside another
component that zooms, **without code of its own**, now and for every component written later.

How `koebi-ui` works today:
- Components lay themselves out in logical pixels, from styles snapped to whole physical pixels
  at the window's scale factor (`Styles::resolve_at`).
- They draw by pushing primitives into the `Scene`: rects, lines, fills, icons, glyphs.
- They hit-test against the last frame's regions (ADR 0013).
- 18 files read the pointer from `Input` directly, 27 times.

Text and icons are drawn from curves (ADR 0020, 0022), so they stay sharp at any size: the
renderer is ready for scaled content.

## Decision

A **space** is a uniform scale and a translation, `Space { scale: f32, offset: Vec2 }`, applied
by `Ui` to everything drawn and tested inside it.

- **Opening a space:** `ui.space(space, |ui| …)` composes the space with the current one, and
  spaces nest: a scaled preview inside a zoomed node works too.
  - Inside, every rect a component computes is in the space's *local* units: the units it
    would use unscaled.
  - Styles stay those of the window, unscaled. A button inside a space at 0.5 lays itself out
    exactly as outside, and is drawn at half size.
- **Drawing:** the `Scene` keeps a transform stack. Every push maps its geometry through the
  current space:
  - rects, corner radii, border widths;
  - line ends and widths;
  - fill shapes, icon squares;
  - glyph origins and sizes;
  - clip rects and rounded clips.

  Primitives are axis-aligned, and a uniform scale keeps them so: nothing in the renderer
  changes but the push functions.
- **Pixel snapping happens after the transform, in physical pixels.**
  - `Edges::Snapped` rects and hairlines land on whole pixels at any zoom: a 1 px border stays
    crisp.
  - Under 1 physical pixel, a hairline keeps one pixel and fades its alpha, so outlines never
    vanish or alias.
- **Hit testing:** `ui.interact(id, rect)` registers the rect mapped to the window. The routing
  of ADR 0013 is unchanged: regions, layers and the last frame.
- **The pointer:** components read it through `Ui`, in local units:
  - `ui.pointer()`, `ui.pointer_delta()`, `ui.press_position()`;
  - drag deltas are divided by the scale, so a slider dragged at zoom 4 moves at the speed of
    the hand.

  `Input` keeps window units for the application. A clippy `disallowed-methods` rule forbids
  `Input::pointer` inside `koebi-ui` widgets, so that no component can bypass its space. The 27
  existing reads move to the `Ui` accessors.
- **Popups:**
  - Menus, dropdown lists, tooltips and color pickers open *outside* the space, at the window's
    scale. They are anchored to their control's rect mapped to the window: a dropdown in a node
    at zoom 0.3 opens a readable list next to it, as Blender does.
  - A component asks for this with the existing floating layers. `Ui` lifts the space for
    `Layer::FLOATING` and above.
- **Text input:** caret, selection and IME position map through the space like everything else.
  Keyboard focus is unaffected.
- **Detail by scale:** `ui.scale()` gives the absolute scale. A component may drop detail below
  a threshold through a small set of shared helpers (`Ui::readable`), so that thresholds stay
  consistent:
  - socket labels hide in a far-zoomed graph;
  - a node shows only its header.
- **Allocation:** spaces are a stack of plain values in `UiState`. The per-frame
  zero-allocation tests (ADR 0026) cover a screen with nested spaces.

## Consequences

- **Every component, present and future, works scaled:** the transform sits below them, in
  `Ui` and the `Scene`, and nothing else changes. Node bodies in the Node Graph are ordinary
  components.
- **One migration:** pointer reads in widgets move to `Ui`, and the clippy rule keeps it so.
- **Crisp at any zoom:** text, icons and hairlines stay sharp, since text and icons are curves
  and edges snap after the transform. Fills between snapped edges may be one physical pixel
  wider or narrower at fractional scales, as at fractional window scales today.
- **Tests:**
  - a functional test draws the same component at scales 0.25, 1 and 4 and compares it to
    the unscaled reference resampled;
  - interaction tests click, drag and type inside spaces at several scales, nested ones
    included.

## Alternatives considered

- **Re-resolving styles at every zoom** (`Styles::resolve_at(scale × zoom)`): exact snapping of
  every length, but a style refresh per frame while zooming, and one style set per space.
  Continuous zoom makes caching useless.
- **Rendering spaces to an offscreen texture and scaling the image:** text blurs when zoomed
  in, input needs a separate mapping anyway, and the texture costs memory per space.
- **A transform per component:** the code of its own the requirement rules out.
