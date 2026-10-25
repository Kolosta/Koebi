//! Koebi text basics: TrueType fonts, glyph rasterization and single-line layout.
//!
//! The minimum the interface needs to draw labels ([ADR 0014]), built to grow into full
//! typography: kerning, ligatures, line breaking and font fallback come later.
//!
//! - [`Font`] reads a TrueType font (bounds-checked: malformed fonts are errors, never panics).
//! - [`Fonts`] collects fonts by family and weight; [`Fonts::with_embedded`] holds Noto Sans
//!   Regular and Medium, embedded under the SIL Open Font License.
//! - [`TextLine`] places the glyphs of one line and cuts it with an ellipsis to fit a width.
//! - [`Rasterizer`] turns a glyph into 8-bit coverage at a pixel size and subpixel offset;
//!   the renderer caches the bitmaps in a GPU atlas.
//!
//! ```
//! use koebi_text::{Fonts, GlyphBitmap, Rasterizer, TextLine, TextStyle, embedded};
//!
//! let fonts = Fonts::with_embedded();
//! let font = fonts.select(embedded::NOTO_SANS, 500).expect("Noto Sans Medium is embedded");
//! let style = TextStyle { font, size: 14.0, tracking: 0.0 };
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
//! [ADR 0014]: https://github.com/Kolosta/Koebi/blob/develop/docs/adr/0014-text-basics.md

mod error;
mod font;
mod fonts;
mod line;
mod outline;
mod raster;
mod read;

pub use error::{Error, Result};
pub use font::{Font, FontMetrics, GlyphId};
pub use fonts::{FontId, Fonts, embedded};
pub use line::{PlacedGlyph, TextLine, TextStyle};
pub use outline::{Outline, PathCommand};
pub use raster::{GlyphBitmap, MAX_GLYPH_SIZE, Rasterizer};
pub use read::Tag;
