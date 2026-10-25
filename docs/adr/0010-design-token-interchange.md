# 0010. Design token interchange: the W3C Design Tokens format with a Koebi extension

- Status: Accepted
- Date: 2026-10-25
- Deciders: @Kolosta

## Context

Themes must leave and enter Koebi as files that people can read and edit, and that design
tools can understand: a designer tweaks a theme, shares it, imports it on another machine.
The token tables are compiled Rust (ADR 0008) and the saved state is binary (ADR 0009), so
neither is the answer. The W3C Design Tokens Community Group format (DTCG, stable version
2025.10) is the common ground of design tools, but it knows nothing about tiers, layers or
Koebi's theme identity. ADR 0003 rules out `serde`.

## Decision

`koebi_design::dtcg` reads and writes DTCG JSON, with Koebi's metadata in
`$extensions["io.github.kolosta.koebi"]`.

- **Paths**: full token paths nested as groups (`semantic` → `background` → `base` →
  `default`); aliases are `{primitive.color.gray.50}`.
- **Values**: the DTCG forms. Colors are objects (`colorSpace`, `components`, `alpha`, plus a
  `hex` for readers); dimensions and durations are `{ "value", "unit" }`; cubic Béziers are
  arrays; text styles are `typography` composites whose five fields map to Koebi's field
  tokens. Integers are `number`s, marked `"kind": "integer"` in the extension.
- **Extension**: at the root, the format version, the schema, the base theme and the theme's
  key, name and description; on each token, its tier, the layer it comes from and its range.
- **Export**: `export_theme` writes what a theme changes compared with the base theme (its
  definitions, its overrides and the global overrides, the topmost one per token);
  `export_schema` writes every token with its base definition and documentation.
- **Import**: `import_theme` creates a theme from a file. Validation is strict and complete:
  RFC 8259 JSON (no comments, no trailing commas, no duplicate keys), known `$` properties
  only, known tiers and tokens, types matching kinds, values in range, tier rules, no cycles.
  Every problem is reported with its line, column and token path; nothing is created when there
  is one. Colors are also accepted in `srgb-linear`, `hsl`, `oklab`, `oklch` and as hexadecimal
  strings, dimensions in `rem` (16 px).
- **JSON**: a small strict parser and pretty printer in `koebi-design`, keeping positions for
  error messages.

## Consequences

- Themes are portable, diffable and editable by hand; an export imports back to the same
  values, which tests check for every built-in theme and for the full schema.
- Other tools can read Koebi's exports and ignore the extension; Koebi reads their files when
  they use Koebi's token paths.
- Masks and the color vision setting are user preferences, not part of a theme: they are not
  exported.
- DTCG types Koebi has no token for (shadow, border, gradient, transition, stroke style) are
  rejected until Koebi needs them.

## Alternatives considered

- **A Koebi-specific JSON layout**: simpler to write, unknown to every other tool.
- **The binary persistence format**: compact but not readable or editable.
- **TOML or YAML**: no design tool speaks them for tokens, and their parsers are larger.
- **`serde_json`**: a dependency for a few hundred lines of code we can test fully.
- **Lenient import** (skip what does not fit): silent losses; a strict import with a complete
  report lets the author fix the file once.
