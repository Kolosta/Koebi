//! Single-line layout: glyphs shaped ([`crate::shape`]), cut with an ellipsis to fit a width.

use crate::font::GlyphId;
use crate::fonts::{FontId, Fonts};

/// The typographic features a style uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Features {
    /// Pair kerning (`kern`).
    pub kerning: bool,
    /// Standard and contextual ligatures (`liga`, `clig`), never with tracking: letters spaced
    /// apart are not joined.
    pub ligatures: bool,
}

impl Default for Features {
    /// Both on, as browsers and type setters do.
    fn default() -> Self {
        Self {
            kerning: true,
            ligatures: true,
        }
    }
}

/// How a run of text looks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    /// The font; characters it lacks come from the fallbacks ([`Fonts::add_fallback`]).
    pub font: FontId,
    /// Size in pixels per em (logical pixels for the UI).
    pub size: f32,
    /// Extra space between glyphs, in ems (the `font.tracking.*` tokens).
    pub tracking: f32,
    /// Kerning and ligatures.
    pub features: Features,
}

impl TextStyle {
    /// `font` at `size`, without tracking, with the default features.
    pub fn new(font: FontId, size: f32) -> Self {
        Self {
            font,
            size,
            tracking: 0.0,
            features: Features::default(),
        }
    }
}

/// A glyph placed on a line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedGlyph {
    /// Its font.
    pub font: FontId,
    /// The glyph.
    pub glyph: GlyphId,
    /// Pen position from the start of the line, in the style's pixels.
    pub x: f32,
    /// Byte offset of its character in the text.
    pub cluster: u32,
}

/// One line of laid-out text. Reuse one across frames: layout keeps its storage.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextLine {
    glyphs: Vec<PlacedGlyph>,
    size: f32,
    width: f32,
    ascent: f32,
    descent: f32,
    truncated: bool,
}

impl TextLine {
    /// An empty line.
    pub fn new() -> Self {
        Self::default()
    }

    /// Lays `text` out on one line. Control characters are skipped; characters the font lacks
    /// show its missing-glyph box. An unknown font gives an empty line.
    pub fn layout(&mut self, fonts: &Fonts, text: &str, style: &TextStyle) {
        self.glyphs.clear();
        self.size = style.size;
        self.width = 0.0;
        self.truncated = false;
        let Some(font) = fonts.get(style.font) else {
            self.ascent = 0.0;
            self.descent = 0.0;
            return;
        };
        let scale = style.size / f32::from(font.units_per_em());
        let metrics = font.metrics();
        self.ascent = f32::from(metrics.ascender) * scale;
        self.descent = -f32::from(metrics.descender) * scale;
        self.width = crate::shape::shape(fonts, text, style, &mut self.glyphs);
    }

    /// Lays `text` out, then, if it is wider than `max_width`, keeps the longest start that
    /// fits followed by an ellipsis ("…"), without the spaces before it. When not even the
    /// ellipsis fits, the line is empty.
    pub fn layout_truncated(
        &mut self,
        fonts: &Fonts,
        text: &str,
        style: &TextStyle,
        max_width: f32,
    ) {
        self.layout(fonts, text, style);
        // Rounding must not cut text that measures exactly the available width.
        if self.width <= max_width + 1e-3 {
            return;
        }
        self.truncated = true;
        let Some(font) = fonts.get(style.font) else {
            return;
        };
        let scale = style.size / f32::from(font.units_per_em());
        let spacing = style.tracking * style.size;
        let advance = |glyph| f32::from(font.advance(glyph)) * scale;
        // Where each glyph's advance ends, in its own font.
        let end =
            |placed: &PlacedGlyph| placed.x + crate::shape::advance(fonts, placed, style.size);
        let ellipsis = font.glyph_index('\u{2026}');
        let (ellipsis, dots, ellipsis_width) = if let Some(glyph) = ellipsis {
            (glyph, 1, advance(glyph))
        } else {
            let dot = font.glyph_index('.').unwrap_or(GlyphId::NOTDEF);
            (dot, 3, advance(dot) * 3.0 + spacing * 2.0)
        };
        let fits = |end: f32| end + spacing + ellipsis_width <= max_width;
        let mut kept = self
            .glyphs
            .iter()
            .take_while(|placed| fits(end(placed)))
            .count();
        let is_space = |placed: &PlacedGlyph| {
            usize::try_from(placed.cluster)
                .ok()
                .and_then(|at| text.get(at..))
                .and_then(|rest| rest.chars().next())
                .is_some_and(char::is_whitespace)
        };
        while kept > 0 && is_space(&self.glyphs[kept - 1]) {
            kept -= 1;
        }
        let cluster = self.glyphs.get(kept).map_or(0, |placed| placed.cluster);
        let mut x = match kept.checked_sub(1).map(|last| self.glyphs[last]) {
            Some(last) => end(&last) + spacing,
            None => 0.0,
        };
        self.glyphs.truncate(kept);
        if ellipsis_width > max_width {
            self.glyphs.clear();
            self.width = 0.0;
            return;
        }
        for dot in 0..dots {
            if dot > 0 {
                x += spacing;
            }
            self.glyphs.push(PlacedGlyph {
                font: style.font,
                glyph: ellipsis,
                x,
                cluster,
            });
            x += advance(ellipsis);
        }
        self.width = x;
    }

    /// The glyphs, left to right.
    pub fn glyphs(&self) -> &[PlacedGlyph] {
        &self.glyphs
    }

    /// The size the line was laid out at, in pixels per em: glyphs are drawn at this size.
    pub fn size(&self) -> f32 {
        self.size
    }

    /// Width from the start of the first glyph to the end of the last one's advance.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Distance from the baseline to the top of the line box (positive).
    pub fn ascent(&self) -> f32 {
        self.ascent
    }

    /// Distance from the baseline to the bottom of the line box (positive).
    pub fn descent(&self) -> f32 {
        self.descent
    }

    /// The baseline that centres the line box vertically in a box from `top` of `height`.
    pub fn centered_baseline(&self, top: f32, height: f32) -> f32 {
        top + (height - self.ascent - self.descent) * 0.5 + self.ascent
    }

    /// `true` when [`TextLine::layout_truncated`] had to cut the text.
    pub fn is_truncated(&self) -> bool {
        self.truncated
    }

    /// `true` without any glyph.
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }
}
