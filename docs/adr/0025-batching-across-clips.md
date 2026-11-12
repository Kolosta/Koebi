# 0025. Batching across clips: each instance carries its clip, overlaps found in a grid

- Status: Accepted
- Date: 2026-11-11
- Deciders: @Kolosta

## Context

A `Scene` holds the primitives of a frame (rectangles, segments, fills, glyphs, icons), one
instanced draw per batch. Batches came from consecutive primitives of one type and one clip,
and the scissor and the rounded clip were draw state:

- **Every control is a clip.** Number fields, text fields, lists and scroll areas clip their
  content, so each one started new batches of rectangles and glyphs. The Gallery drew in 354
  draws, the Theme page in 400, the workspace in about 230.
- **Painter's order splits the rest.** A background, its label, the next background, its label:
  each type change was a new draw even where nothing overlapped.
- **Each draw costs.** With the validation layers of a debug build, a draw with its scissor,
  push constants and vertex buffer cost about 9 µs: 2 ms of a 3 ms frame went to recording. On
  mobile drivers and MoltenVK, draw calls are the dearest part of a frame without validation as
  well.

The canvas comes next ([#15](https://github.com/Kolosta/Koebi/issues/15),
[#16](https://github.com/Kolosta/Koebi/issues/16)), as a graph of nodes rendered in passes, not as
layers. The interface renderer stays its own path, and its draw count must not grow with the
number of controls a screen shows.

## Decision

Clips become per-instance data, and a primitive joins the latest batch of its type on its layer
unless something drawn after that batch paints where it paints.

- **Clips as data.** `Scene::push_clip` and `push_rounded_clip` number each clip of the frame.
  Every instance keeps the number of the clip in force. The renderer gathers the clips of each
  type's instances into a second vertex stream (`ClipInstance`: the kept pixels, the rounded
  rectangle and its radii, 48 bytes), read by every vertex shader at locations 12 to 14
  (`shaders/clip_vertex.glsl`).
- **The scissor stays open.** The fragment shaders keep a pixel when its center lies inside the
  clip's whole pixels, which is exactly what a scissor keeps (`shaders/clip.glsl`), then cut the
  rounded corners as before. Push constants hold only the viewport, set once per pipeline.
- **Overlaps in a grid** (`crates/koebi-render/src/batch.rs`). The frame has a grid of 32-pixel
  cells. Each cell keeps, for each batch that painted there, the box of what it painted there.
  A primitive finds the latest batch of its layer that painted under it in the cells it covers.
  It joins the latest batch of its type if that one is not older, else starts a batch. There is
  no look-back limit, and the cost is the cells a primitive covers.
- **Exact boxes.** A primitive's box is clipped to its clip: nothing is drawn outside it. Glyphs
  take the box of their outline, from the glyph header in the `glyf` table, carried by
  `PlacedGlyph::ink` from shaping. Glyphs without an outline (spaces) are not pushed at all.
- **Instances are gathered, not sorted.** `DrawOrder` counts each batch's start in its type's
  buffer and scatters the instances there, in push order. Draws start at instance zero at a
  buffer offset (MoltenVK, #61).

## Consequences

- Draws per frame: Gallery 354 → 26, Theme page 400 → 41, the workspace about 230 → 47. Debug
  frames with validation went from 3.1 ms to 1.7 ms; recording takes 0.04 ms without validation.
- Pixels are unchanged: every golden image and pixel test passed untouched by the change.
- The instance data grows by 48 bytes per instance in a second stream: about 100 KB a frame for
  a full interface, uploaded with the instances.
- Fragments outside a clip still run the fragment shader before being dropped; quads are small
  in an interface. A canvas with large clipped content would shrink its quads to the clip in
  the vertex shader, or render through its own passes.
- Callers push in painter's order and never think about batches. A new primitive type gives the
  scene its box, as the existing ones do.
- The grid treats a box shared by two primitives of one batch in a cell as covered, so it may see
  an overlap where there is none: an extra batch, never a wrong order.

## Alternatives considered

- **Scissors per batch, batches split by clip** (the former design): the draw count grows with
  the controls on screen.
- **A clip table in a storage buffer, indexed per instance**: one 4-byte index per instance
  instead of 48 bytes, but a descriptor set for every pipeline, storage buffer bindings in
  `koebi-gpu`, and a table rebuilt each frame. Worth it if instance bandwidth ever matters; the
  interface's does not.
- **One union box per batch with a look-back of N batches** (Skia's and WebRender's batching
  are of this kind): a batch's box soon covers a whole column, so a background under the next
  label cannot join its batch. The Gallery kept 90 draws with it, against 26.
- **Ordering by depth with a depth buffer**: it only works for opaque content. An interface is
  mostly translucent edges and anti-aliased curves, drawn in painter's order.
