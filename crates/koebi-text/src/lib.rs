//! Koebi text: TrueType fonts, shaping, line breaking and paragraphs, for Latin and CJK text
//! written left to right ([ADR 0022]; the renderer draws glyphs from their curves).
//!
//! - [`Font`] reads a TrueType font or a font of a collection (bounds-checked: malformed fonts
//!   are errors, never panics): its glyphs, metrics, family name, kerning pairs and ligatures.
//! - [`Fonts`] collects fonts by family and weight, with fallbacks for the characters a font
//!   lacks; [`Fonts::with_embedded`] holds Noto Sans and Noto Sans JP, embedded under the SIL
//!   Open Font License. [`system`] lists and loads the system's fonts.
//! - [`TextLine`] shapes one line (ligatures, kerning, tracking) and cuts it with an ellipsis
//!   to fit a width; [`TextBlock`] lays paragraphs out to a width where [`breaks`] allows it.
//! - [`Outline::monotonic_curves`] gives a glyph as the curves the renderer draws it from.
//! - [`Rasterizer`] turns a glyph into 8-bit coverage on the CPU, for tests and images.
//!
//! ```
//! use koebi_text::{Fonts, GlyphBitmap, Rasterizer, TextLine, TextStyle, embedded};
//!
//! let fonts = Fonts::with_embedded();
//! let font = fonts.select(embedded::NOTO_SANS, 500).expect("Noto Sans Medium is embedded");
//! let style = TextStyle::new(font, 14.0);
//!
//! let mut line = TextLine::new();
//! line.layout_truncated(&fonts, "Outliner", &style, 40.0);
//! assert!(line.is_truncated() && line.width() <= 40.0);
//!
//! let mut rasterizer = Rasterizer::new();
//! let mut bitmap = GlyphBitmap::default();
//! let first = line.glyphs()[0];
//! let face = fonts.get(first.font).expect("the line's font");
//! rasterizer.rasterize(face, first.glyph, 14.0, 0.0, &mut bitmap)?;
//! assert!(!bitmap.is_empty());
//! # Ok::<(), koebi_text::Error>(())
//! ```
//!
//! [ADR 0022]: https://github.com/Kolosta/Koebi/blob/develop/docs/adr/0022-text-on-the-gpu.md

mod block;
pub mod breaks;
pub mod cache;
mod error;
mod font;
mod fonts;
mod line;
mod otl;
mod outline;
mod raster;
mod read;
mod shape;
pub mod system;

pub use block::{Align, LaidLine, Paragraph, TextBlock};
pub use cache::LineCache;
pub use error::{Error, Result};
pub use font::{Font, FontMetrics, GlyphId};
pub use fonts::{FontId, Fonts, embedded};
pub use line::{Features, PlacedGlyph, TextLine, TextStyle};
pub use outline::{MonotonicCurve, Outline, PathCommand};
pub use raster::{GlyphBitmap, MAX_GLYPH_SIZE, Rasterizer};
pub use read::Tag;
