//! Interchange with the W3C Design Tokens format (DTCG, 2025.10), extended with Koebi's metadata.
//!
//! - [`export_theme`] writes a theme as a readable JSON file: every token the theme, its
//!   overrides and the global overrides change compared with the base theme, with its type,
//!   description and tier. Exporting the active theme gives "everything that differs from
//!   the default".
//! - [`export_schema`] writes every token of the schema with its base definition: a complete,
//!   documented reference for designers and tools.
//! - [`import_theme`] reads such a file into a new theme. Validation is strict and complete:
//!   every problem is reported with its line, column and token path, and nothing is created
//!   when there is one.
//!
//! Token paths are the full paths of the schema (`semantic.background.base.default`), nested
//! as groups; aliases are written `{primitive.color.gray.50}`. Values follow the Design Tokens
//! format: colors as `{ "colorSpace": "srgb", "components": [...], "alpha": 1, "hex": ... }`
//! (import also accepts `srgb-linear`, `hsl`, `oklab`, `oklch` and hexadecimal strings),
//! dimensions and durations as `{ "value": 4, "unit": "px" }`, cubic Béziers as arrays, text
//! styles as `typography` composites. Koebi's own metadata lives under
//! `$extensions["io.github.kolosta.koebi"]`: the theme's identity, each token's tier and the
//! layer it comes from, and the `integer` kind (a `number` for other tools).

mod export;
mod import;

pub use export::{export_schema, export_theme};
pub use import::{ImportIssue, import_theme};

/// The extension key of Koebi's metadata.
pub(crate) const EXTENSION: &str = "io.github.kolosta.koebi";
/// The version of Koebi's extension, bumped when its meaning changes.
pub(crate) const FORMAT: f64 = 1.0;
