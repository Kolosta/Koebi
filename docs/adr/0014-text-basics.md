# 0014. Text basics: in-house TrueType reading, CPU coverage glyphs, single-line labels

- Status: Accepted
- Date: 2026-10-25
- Deciders: @Kolosta

## Context

The workspace components (tabs, menus, top bars, outliner rows) need labels. Typography
proper (kerning, ligatures, shaping of complex scripts, line breaking, font fallback, variable
fonts, the text tokens' tracking and line heights) is planned as its own piece of work; for now
components need one line of text per label, cut with an ellipsis when it does not fit, drawn
crisply at every scale factor. The design tokens name Noto Sans (`font.family.sans`), with
regular body text and medium labels. ADR 0003 forbids new dependencies without a strong case.

## Decision

A new `koebi-text` crate (depends on `koebi-core` only) provides the minimum, shaped so that the
typography work extends it rather than replaces it:

- **Fonts**: TrueType outlines read in-house from `head`, `maxp`, `hhea`, `hmtx`, `cmap`
  (formats 4 and 12), `loca` and `glyf` (simple and composite glyphs), plus the weight from
  `OS/2`. Every read is bounds-checked: malformed fonts return errors, never panic. CFF
  outlines, collections and variable fonts are refused for now.
- **Embedded fonts**: Noto Sans Regular and Medium (SIL Open Font License 1.1, license file
  beside the fonts), registered as family `NotoSans` with CSS-like weight matching. Other
  weights join when a component needs them.
- **Glyphs**: rasterized on the CPU into 8-bit coverage at the exact physical size, with exact
  area coverage (the accumulation technique of `font-rs`), no hinting. Pen positions are
  quantized to a quarter of a physical pixel horizontally and to whole pixels vertically, so a
  glyph bitmap maps one texel to one pixel. The GPU glyph atlas belongs to the renderer.
- **Layout**: one line, from each glyph's advance; ellipsis truncation to a width. Every glyph
  carries its font, so fallback fonts can be added without changing the layout's shape.

## Consequences

- Labels render crisply and identically on every platform, independent of system fonts.
- The binary grows by about 1.3 MB of fonts.
- No kerning or ligatures yet: pairs such as "AV" are spaced by their advances only. Text
  scales by re-rasterizing at each size, which suits UI sizes; a zooming canvas will need
  another technique (distance fields or curves) decided with the typography work.
- Blending coverage in gamma space (ADR 0004) makes light text on dark backgrounds look thinner
  than dark text on light ones; a contrast correction is left to the typography work.

## Alternatives considered

- **`ttf-parser`, `fontdue`, `swash`, `rustybuzz`**: solid crates, but they add dependencies
  for what a few hundred lines cover today, and the full shaping question deserves its own ADR.
- **System font APIs** (DirectWrite, Core Text, FreeType): five code paths and rendering that
  differs per platform, against Koebi's exact-pixel goals.
- **Signed distance field atlases**: one rasterization for every size, but soft corners at UI
  sizes and a heavier shader; worth reconsidering for canvas text.
