# 0029. Canvas rendering: paths from their curves, compositing as a program, exact at any zoom

- Status: Accepted
- Date: 2026-11-17
- Deciders: @Kolosta

## Context

The document graph (ADR 0028) evaluates, per page, a `Program`: draw, merge, blend, mask and
isolate steps over prepared geometry, in f64 document coordinates. The canvas renderer turns a
program and a view (camera, size) into pixels. It serves the Viewport, the Node Graph's
previews, the Outliner's vignettes and the file preview.

Carto's experience, and its documented failures:

- **Tessellation tiers.** It flattened and triangulated paths with a tolerance per zoom tier.
  - A tier change cost a 120 ms re-tessellation.
  - Meshes were stored in node-local f32, so large shapes lost precision at deep zoom: the
    "badly rendered when very zoomed" objects.
  - The tolerance ignored the object's scale, and the tiers stopped at 2^±24.
- **MSAA ×4 only.** Blended group edges were aliased, since isolation targets were not
  multisampled.
- **The compositor** leaked blends onto siblings and let Erase wipe the parent (ADR 0028).

Koebi already draws icons (ADR 0020) and glyphs (ADR 0022) from **quadratic curves**:
- coverage is computed in the fragment shader by two rays per pixel, over curves filed in
  bands;
- the result equals a CPU reference within 2/255 on every driver (lavapipe, NVIDIA, MoltenVK);
- it is exact at every size and angle, with no tessellation and no atlas.

## Decision

### Geometry: `koebi-geom`

A new crate, `koebi-geom` (std only, depends on `koebi-core`), holds every geometric algorithm
in f64, with no GPU:

- **Paths:**
  - subpaths of Bézier anchors, with in and out handles of kinds Corner, Smooth and Symmetric;
  - NURBS subpaths, by homogeneous de Boor;
  - polylines;
  - builders for rectangles, ellipses, polygons and the exact NURBS circle.
- **Measures:** arc length and parameter by length, bounds, nearest point, winding and
  hit tests (fill rules, distance to a stroke).
- **Strokes to outlines:** a stroke becomes a fill outline.
  - Align: center, inside, outside, left, right, with an offset.
  - Caps: butt with a tilt, round, square, taper.
  - Joins: miter with a limit, round, bevel.
  - Dashes with offset and fit, real caps on every dash, marks and repeats placed by arc
    length.
  - Overlaps of one stroke are united, so a translucent stroke never darkens itself: Carto's
    known v1 defect.
- **Booleans** on curves: union, subtract, intersect, xor, robust to collinear and coincident
  edges. Results are curves again, not polygons.
- **Curve preparation for the GPU:** cubic and conic segments become quadratics within a
  tolerance, split monotonic in x and y.

### Coverage on the GPU: the icons' and glyphs' method, for documents

`koebi-canvas`, a new crate (depends on `koebi-gpu`, `koebi-geom` and the program types of
`koebi-doc`), draws a filled outline as **one instance over its bounds**. The fragment shader
computes coverage analytically from the outline's quadratic curves, filed in bands, as for
glyphs, with the non-zero or even-odd rule.

- **Exact anti-aliasing at every zoom**, the same on every driver. There is no MSAA, so
  isolated targets keep exact edges too.
- **Geometry does not depend on the zoom.** Outlines in document units are prepared once per
  edit, so a zoom costs nothing.
  - Two things follow the zoom, by octave with hysteresis: the cubic-to-quadratic tolerance
    (an error under 1/16 px at the current octave) and the outlines of viewport-space strokes
    (constant-pixel hairlines).
  - Both are prepared in the background and kept by octave, with no upper or lower bound on
    octaves.
  - *Refined in #16:* viewport-space strokes follow quarter octaves, with a sixteenth of an
    octave of hysteresis: a whole octave would let a hairline's width on screen vary by 41%, a
    quarter keeps it within 14%. They are outlined on the render thread for now, and only those
    in sight.
- **Precision without limits:**
  - Positions and transforms stay f64 on the CPU.
  - Curves are stored relative to the origin of their *cell*: the band cells of a large
    outline have origins in f64. Their magnitudes stay small whatever the shape's extent.
  - Each draw receives the camera-relative affine of its cells, computed in f64 and sent in
    f32. Zooming 10^9 into a kilometre-wide path keeps sub-pixel exactness.
- **Data:**
  - curves and bands in persistent GPU buffers, updated by range when an object changes;
  - instances (outline, cell, paint, transform) in a per-view buffer;
  - one draw per paint kind and target, as ADR 0025 batches the interface.

### Paints

- **Solid paint:** a constant color, linear and premultiplied in the shader.
- **Pattern fill:** a motif drawn as instances on a lattice (spacing, phase, angles, scale),
  clipped to the bounds, the contour, or the stroke's inner or outer edge (Carto's clip
  options), anchored to the object or to the document.
- **Instanced fill:** grid or scatter layouts of primitives and line sets, generated on the
  CPU and drawn as instances.
- **Repeats and marks** along strokes are geometry from `koebi-geom`. Each mark object is fused
  into the stroke, blended as a separate drawable, or cut out (erase), per its mode.
- **Gradients and images** come later as further paint kinds. Images depend on #20.

### Compositing: the program, step by step

- **Targets:** the program's steps run on `Rgba16Float` targets, linear and premultiplied,
  taken from a pool sized by the visible part of each step's bounds. They are released at the
  end of the frame and reused on the next.
- **Merge:** a run of Normal inputs at full opacity draws straight into the current target,
  with no extra buffer.
- **Blend:** an input with another mode, an opacity below 1 or *isolate* draws into its own
  target. It is then composited onto the backdrop *accumulated so far in its own Merge* by the
  blend shader:
  - the W3C formulas for the twelve separable and four non-separable modes;
  - Erase as destination-out, scaled by the source's alpha and opacity.

  A blend can therefore never reach what lies above it or outside its Merge.
- **Mask:** the coverage of a geometry or the alpha of a layer, drawn into a single-channel
  target (`R16Float`), multiplies the content, optionally inverted.
  - Clip and mask from Layers mode are masks with the canonical source (ADR 0028).
- **Page:** content is composited onto the page's background. The background is a step of the
  page, not of its stack, so an Erase at the page's root reveals the background, not the
  view's color.

### Views

- **A view** is a camera (center and zoom in f64, no limits), a pixel size and a scale factor.
- **Its target:** each view renders into its own `Rgba16Float` target. The interface shows it
  as an image primitive in its rect.
  - `koebi-render` gains image primitives for this, and for vignettes and file previews.
  - It encodes to sRGB when composited into the window. Wide gamut follows later (#22).
- **Culling** skips steps outside the view and never changes the pixels.
- **Steady state:** an unchanged program revision and camera re-show the last target without
  recording anything. A steady frame of a still viewport costs one textured quad.
- **Picking** is exact on the CPU, from `koebi-geom` hit tests on the evaluated geometry
  (topmost first). The view's last-frame bounds index keeps it O(log n). A GPU id pass is an
  option for later.

### The background grid

The Viewport's dot grid (Blender's node-editor dots) is a fill primitive of `koebi-render`:
- one quad;
- dots computed in the shader;
- two levels, the finer one fading in as zoom grows;
- spacing chosen in powers of a base step so that dots are never closer than a minimum
  distance or farther than a maximum.

It is interface, drawn under the canvas image, with colors and sizes from tokens.

### Verification

- `koebi-geom`: unit tests on every algorithm, and property tests against a dense
  flatten-and-sample reference: outline exactness, boolean areas, arc lengths.
- `koebi-canvas`:
  - pixel tests against a CPU reference rasterizer, as `koebi_svg::raster` does for icons:
    fills under both rules, every join, cap and alignment, dashes, patterns, masks, every
    blend mode against the W3C formulas within 1/255;
  - golden images for complete programs;
  - a deep-zoom test that renders the same edge at 10^-6 and 10^9 and compares the pixels.
- Benchmarks in `cargo xtask bench`: 10 000 paths, a zoom sweep, editing one path among
  10 000, 500 blended groups, 100 000 instances.

## Consequences

- **The canvas is exact at any zoom and on any driver.** It shares the shader model of icons
  and text, so the Node Graph's text and the canvas's shapes look alike when scaled.
- **An edit prepares one object's curves, a zoom prepares nothing.** Carto's tier spikes and
  precision losses have no equivalent.
- **Two new crates** with clear layers:
  - `koebi-geom`: pure maths, usable by the document, the tools and the tests;
  - `koebi-canvas`: GPU.
- **Fragment cost grows with curve density per pixel.** Very dense paths need finer bands.
  This is measured by the benchmarks; tiling large outlines into cells bounds it.
- **Isolated steps cost targets.** Pooling and scissoring to visible bounds bound that cost,
  and merges without blends cost none. Carto measured 49 ms for 500 groups with full-screen
  targets: the benchmark sets the target to beat.

## Alternatives considered

- **Tessellation with MSAA (Carto):** zoom tiers, re-tessellation spikes, aliased isolated
  edges, f32 precision limits. These are exactly the failures to avoid.
- **Stencil-then-cover:** exact but needs two passes per path and MSAA for edges. Analytic
  coverage gives better edges in one pass, already proven in this renderer.
- **Signed distance fields:** fast for rendering, but corners round off at deep zoom and
  generation costs per edit.
- **Rendering on the CPU and uploading tiles:** simple and exact, far from the performance the
  project requires.
