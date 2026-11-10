# 0022. Text on the GPU from glyph curves, shaping and paragraphs

- Status: Accepted
- Date: 2026-11-10
- Deciders: @Kolosta
- Supersedes the glyph atlas and the single-line layout of [0014](0014-text-basics.md)

## Context

ADR 0014 drew each glyph from a CPU bitmap at its physical size, cached in an atlas texture,
on one line, by advances alone. That served labels, not what text has to do: every size (a
canvas zooms), any angle (rotated labels, text on a canvas), fonts switched while running,
letter spacing, line height, kerning, ligatures, line breaking, Japanese, Korean and Chinese
written left to right, all of it displayed in real time. A size or an angle the atlas has not
seen costs a rasterization; a large glyph costs a large bitmap. Later, a font editor will edit
outlines in the same engine.

Icons already solved the drawing ([ADR 0020](0020-vector-icons.md)): coverage computed in the
fragment shader from quadratic curves, two rays per pixel, equal to its CPU reference.

## Decision

Glyphs are drawn from their curves like icons, and text is shaped and laid out in paragraphs
by `koebi-text`, std only.

- **Drawing** (`koebi-render/src/glyphs.rs`). The first time a frame draws a glyph, its outline
  (TrueType quadratic contours, `Outline::monotonic_curves`) becomes curves monotonic in x and
  y in the unit square of its bounds, filed by bands: each horizontal band holds the curves its
  rightward rays may cross, each vertical band those of the downward rays (up to 16 bands each
  way, about 4 curves per band). The record goes into a data texture of `vec4` texels that keeps
  every glyph drawn. Only the records stored since the last upload stay on the CPU; they go up
  through a staging buffer per frame in flight, as at most three rectangles and the header
  texel. The texture grows by doubling to 1024 rows of 4096 texels, its content copied on the
  GPU, and is cleared when full. Every fallible step runs before the store changes, so a failure
  leaves the texture as it was and ready to be sampled. An instance is the pen position and the
  em's two axes on screen: any size, any angle. The fragment shader covers a pixel from the
  curves of its two bands with the icons' coverage (`shaders/curves.glsl`), so a pixel reads a
  few curves, not all of a kanji's. Upright glyphs keep their baseline on a whole pixel; their
  pen keeps its exact place across.
- **The same pixels on every driver**. The fragment shader finds the pixel's point in the glyph
  from `gl_FragCoord` and values flat across the quad (the inverse of the em's axes), never from
  an interpolated coordinate: rasterizers interpolate with their own precision, and a point a
  rounding away from a curve's end may be covered differently. With interpolation, lavapipe and
  an NVIDIA GPU disagreed on a few pixels of every page, by up to 78/255; with the exact point,
  every golden image blessed on one matches on the other.
- **Shaping** (`shape.rs`, `otl.rs`). Each character takes the style's font, else the first
  fallback that has it in the closest weight (Noto Sans JP is embedded; the system's Korean and
  Chinese fonts are loaded on a thread at start-up). Ligatures (`GSUB` `liga`, `clig`) join
  glyphs of one font, never with letter spacing; pairs are kerned (`GPOS` `kern`, or the `kern`
  table); tracking is added between glyphs. `TextStyle::features` turns kerning and ligatures
  on or off; the interface reads them from `semantic.font.kerning` and `semantic.font.ligatures`.
- **Lines** (`breaks.rs`, `block.rs`). Lines break where UAX #14 allows it for Latin and CJK
  text written left to right: after spaces and hyphens, between ideographs, kana and Hangul
  syllables, except where the Japanese kinsoku rules forbid it. `TextBlock` lays paragraphs out
  to a width, with a line height (the CSS line box) and an alignment.
- **Fonts**. `Font` reads collections (`.ttc`) and the family name; `koebi_text::system` lists
  the system's TrueType fonts from their headers and loads one on demand.

## Consequences

- A glyph costs one record whatever its size and angle; text zooms and turns in real time, and
  a font change only stores the new glyphs. The GPU and the CPU reference agree to 2/255.
- The same coverage draws icons and text: one engine for the future font editor, whose edited
  outlines will be records like any other.
- Small text has no hinting, as before; positions are exact instead of quarter pixels.
- Japanese adds 11 MiB of fonts to the binary (Noto Sans JP Regular and Medium, OFL).
- Not covered: CFF outlines (most `.otf`, the Noto CJK files Linux and Android ship), variable
  fonts' variations (`gvar`), complex scripts and right-to-left text, vertical writing, mark
  positioning. Each fits the same model: a reader for its outlines or a stage of the shaper.

## Alternatives considered

- **Keep the atlas for small upright text, curves for the rest**: two paths to keep equal; the
  atlas's per-size cost remains for zooming text.
- **Signed distance fields (MSDF)**: one texture per glyph, but corners round off when zoomed,
  and a distance field is a bitmap again, which the font editor cannot edit.
- **A shaping crate (rustybuzz, HarfBuzz)**: complete, but a dependency (ADR 0003) for what Latin
  and CJK left to right need, which `koebi-text` covers in a few hundred lines.
