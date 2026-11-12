//! Shaping one line of text: the glyph of each character, ligatures, advances, kerning and
//! tracking, left to right.
//!
//! Each character takes the style's font when it has a glyph for it, else the first fallback
//! that has one ([`Fonts::glyph_for`]), else the font's missing-glyph box. Consecutive glyphs of
//! one font may join into a ligature (`liga`, `clig`), never when tracking spaces letters apart.
//! Pairs of one font are kerned (`kern`). Shaping writes into the caller's buffer and allocates
//! nothing else: labels are shaped every frame.

use crate::font::GlyphId;
use crate::fonts::Fonts;
use crate::line::{PlacedGlyph, TextStyle};

/// Glyphs a ligature is looked up over at most.
const LIGATURE_WINDOW: usize = 8;

/// The advance of `placed` in its own font at `size` pixels per em.
pub(crate) fn advance(fonts: &Fonts, placed: &PlacedGlyph, size: f32) -> f32 {
    fonts.get(placed.font).map_or(0.0, |font| {
        f32::from(font.advance(placed.glyph)) * size / f32::from(font.units_per_em())
    })
}

/// Shapes `text` on one line into `out` (cleared first), pen positions from 0; returns the
/// width, from the first glyph's pen to the end of the last one's advance. Control characters
/// are skipped.
pub(crate) fn shape(
    fonts: &Fonts,
    text: &str,
    style: &TextStyle,
    out: &mut Vec<PlacedGlyph>,
) -> f32 {
    out.clear();
    for (offset, c) in text.char_indices() {
        if c.is_control() {
            continue;
        }
        let (font, glyph) = fonts.glyph_for(style.font, c);
        out.push(PlacedGlyph {
            font,
            glyph,
            x: 0.0,
            cluster: u32::try_from(offset).unwrap_or(u32::MAX),
            ink: None,
        });
    }
    if style.features.ligatures && style.tracking == 0.0 {
        ligatures(fonts, out);
    }
    let spacing = style.tracking * style.size;
    let mut x = 0.0;
    let mut previous: Option<PlacedGlyph> = None;
    for placed in out.iter_mut() {
        let Some(font) = fonts.get(placed.font) else {
            continue;
        };
        let scale = style.size / f32::from(font.units_per_em());
        if let Some(before) = previous {
            x += spacing;
            if style.features.kerning && before.font == placed.font {
                x += f32::from(font.kerning(before.glyph, placed.glyph)) * scale;
            }
        }
        placed.x = x;
        placed.ink = font.ink(placed.glyph, style.size);
        x += f32::from(font.advance(placed.glyph)) * scale;
        previous = Some(*placed);
    }
    x
}

/// Replaces runs of glyphs that form ligatures, in place: a ligature keeps the cluster of its
/// first character.
fn ligatures(fonts: &Fonts, glyphs: &mut Vec<PlacedGlyph>) {
    let (mut read, mut write) = (0, 0);
    let mut run = [GlyphId::NOTDEF; LIGATURE_WINDOW];
    while read < glyphs.len() {
        let first = glyphs[read];
        let mut len = 0;
        while len < LIGATURE_WINDOW
            && read + len < glyphs.len()
            && glyphs[read + len].font == first.font
        {
            run[len] = glyphs[read + len].glyph;
            len += 1;
        }
        let joined = (len >= 2)
            .then(|| fonts.get(first.font)?.ligature(&run[..len]))
            .flatten();
        if let Some((glyph, count)) = joined {
            glyphs[write] = PlacedGlyph { glyph, ..first };
            read += count;
        } else {
            glyphs[write] = first;
            read += 1;
        }
        write += 1;
    }
    glyphs.truncate(write);
}
