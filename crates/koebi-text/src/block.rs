//! Paragraphs: text shaped ([`crate::shape`]), broken into lines to fit a width where
//! [`crate::breaks`] allows it, placed line after line with a line height and an alignment.
//!
//! Lines follow the CSS line box: each is `line_height × size` high, the font's ascent and
//! descent centered in it (half the leading above, half below). A line feed ends a line; an
//! empty line between two line feeds keeps its height. Spaces at the end of a line hang past
//! its width, and are left out of its glyphs. A word wider than the width breaks between two
//! characters, the last that fit.

use core::ops::Range;

use crate::breaks::{Break, between};
use crate::fonts::Fonts;
use crate::line::{PlacedGlyph, TextStyle};

/// How lines are placed across the block.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    /// Against the start (the left, text being written left to right).
    #[default]
    Start,
    /// Centered.
    Center,
    /// Against the end.
    End,
}

/// How a block is laid out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paragraph {
    /// The widest a line may be, in the style's pixels; `None` breaks at line feeds only.
    pub width: Option<f32>,
    /// From one baseline to the next, as a multiple of the size (the `font.line-height.*`
    /// tokens).
    pub line_height: f32,
    /// How lines are placed across the block.
    pub align: Align,
}

impl Default for Paragraph {
    /// No width, a line height of 1.4, aligned to the start.
    fn default() -> Self {
        Self {
            width: None,
            line_height: 1.4,
            align: Align::Start,
        }
    }
}

/// A line of a block.
#[derive(Clone, Debug, PartialEq)]
pub struct LaidLine {
    /// Its glyphs in [`TextBlock::glyphs`].
    pub glyphs: Range<usize>,
    /// From its first glyph's pen to the end of its last visible glyph.
    pub width: f32,
    /// Its baseline, from the top of the block.
    pub baseline: f32,
}

/// Laid-out paragraphs. Reuse one: layout keeps its storage.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextBlock {
    /// Every line's glyphs, their pen positions from the block's left edge.
    glyphs: Vec<PlacedGlyph>,
    lines: Vec<LaidLine>,
    shaped: Vec<PlacedGlyph>,
    size: f32,
    width: f32,
    height: f32,
}

impl TextBlock {
    /// An empty block.
    pub fn new() -> Self {
        Self::default()
    }

    /// Lays `text` out. An unknown font gives an empty block.
    pub fn layout(&mut self, fonts: &Fonts, text: &str, style: &TextStyle, paragraph: &Paragraph) {
        self.glyphs.clear();
        self.lines.clear();
        self.size = style.size;
        self.width = 0.0;
        self.height = 0.0;
        let Some(font) = fonts.get(style.font) else {
            return;
        };
        let scale = style.size / f32::from(font.units_per_em());
        let metrics = font.metrics();
        let ascent = f32::from(metrics.ascender) * scale;
        let descent = -f32::from(metrics.descender) * scale;
        let line = (paragraph.line_height * style.size).max(0.0);
        let first_baseline = (line - ascent - descent) * 0.5 + ascent;

        crate::shape::shape(fonts, text, style, &mut self.shaped);
        let shaped = core::mem::take(&mut self.shaped);
        let char_at = |glyph: &PlacedGlyph| {
            usize::try_from(glyph.cluster)
                .ok()
                .and_then(|at| text.get(at..))
                .and_then(|rest| rest.chars().next())
                .unwrap_or(' ')
        };
        let advance = |glyph: &PlacedGlyph| crate::shape::advance(fonts, glyph, style.size);
        // Line feeds between the previous glyph's character and this one's.
        let feeds_before = |index: usize| {
            let from = index
                .checked_sub(1)
                .and_then(|previous| usize::try_from(shaped[previous].cluster).ok())
                .map_or(0, |cluster| cluster + 1);
            let to = usize::try_from(shaped[index].cluster).unwrap_or(text.len());
            text.get(from.min(to)..to)
                .map_or(0, |between| between.matches('\n').count())
        };
        let mut start = 0;
        let mut breakable: Option<usize> = None;
        let mut index = 0;
        while index < shaped.len() {
            let feeds = feeds_before(index);
            if feeds > 0 && index > 0 {
                self.finish(&shaped, start..index, &char_at, &advance);
                for _ in 1..feeds {
                    self.finish(&shaped, index..index, &char_at, &advance);
                }
                start = index;
                breakable = None;
            } else if index == 0 {
                for _ in 0..feeds {
                    self.finish(&shaped, 0..0, &char_at, &advance);
                }
            }
            if index > start
                && between(char_at(&shaped[index - 1]), char_at(&shaped[index])) == Break::Allowed
            {
                breakable = Some(index);
            }
            let end = shaped[index].x + advance(&shaped[index]) - shaped[start].x;
            let overflows = paragraph.width.is_some_and(|width| end > width + 1e-3);
            if overflows && index > start && !char_at(&shaped[index]).is_whitespace() {
                let at = breakable.filter(|&at| at > start).unwrap_or(index);
                self.finish(&shaped, start..at, &char_at, &advance);
                start = at;
                breakable = None;
                // The glyphs after the break are measured again from the new line's start.
                index = at;
                continue;
            }
            index += 1;
        }
        let trailing = text
            .get(
                shaped
                    .last()
                    .and_then(|last| usize::try_from(last.cluster).ok())
                    .map_or(0, |c| c + 1)..,
            )
            .map_or(0, |rest| rest.matches('\n').count());
        if !shaped.is_empty() || trailing > 0 {
            self.finish(&shaped, start..shaped.len(), &char_at, &advance);
        }
        for _ in 0..trailing.saturating_sub(usize::from(shaped.is_empty())) {
            self.finish(&shaped, shaped.len()..shaped.len(), &char_at, &advance);
        }
        self.shaped = shaped;

        // Baselines, then alignment across the widest line or the paragraph's width.
        let across = paragraph.width.unwrap_or(self.width).max(self.width);
        for (number, laid) in self.lines.iter_mut().enumerate() {
            laid.baseline = first_baseline + number as f32 * line;
            let shift = match paragraph.align {
                Align::Start => 0.0,
                Align::Center => (across - laid.width) * 0.5,
                Align::End => across - laid.width,
            };
            for glyph in &mut self.glyphs[laid.glyphs.clone()] {
                glyph.x += shift;
            }
        }
        self.height = self.lines.len() as f32 * line;
    }

    /// Ends a line made of `range` of the shaped glyphs.
    fn finish(
        &mut self,
        shaped: &[PlacedGlyph],
        range: Range<usize>,
        char_at: &impl Fn(&PlacedGlyph) -> char,
        advance: &impl Fn(&PlacedGlyph) -> f32,
    ) {
        let origin = shaped.get(range.start).map_or(0.0, |first| first.x);
        let first = self.glyphs.len();
        let mut width = 0.0_f32;
        // Spaces at the end of a line hang past it: they draw nothing and are left out.
        let mut end = range.end;
        while end > range.start && char_at(&shaped[end - 1]).is_whitespace() {
            end -= 1;
        }
        for glyph in &shaped[range.start..end] {
            let x = glyph.x - origin;
            if !char_at(glyph).is_whitespace() {
                width = width.max(x + advance(glyph));
            }
            self.glyphs.push(PlacedGlyph { x, ..*glyph });
        }
        self.width = self.width.max(width);
        self.lines.push(LaidLine {
            glyphs: first..self.glyphs.len(),
            width,
            baseline: 0.0,
        });
    }

    /// Every line's glyphs, their pen positions from the block's left edge.
    pub fn glyphs(&self) -> &[PlacedGlyph] {
        &self.glyphs
    }

    /// The lines, top to bottom.
    pub fn lines(&self) -> &[LaidLine] {
        &self.lines
    }

    /// The size the block was laid out at, in pixels per em.
    pub fn size(&self) -> f32 {
        self.size
    }

    /// The widest line.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// The height of every line box together.
    pub fn height(&self) -> f32 {
        self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::{Fonts, embedded};

    fn style(fonts: &Fonts) -> TextStyle {
        TextStyle::new(fonts.select(embedded::NOTO_SANS, 400).unwrap(), 20.0)
    }

    fn lines(block: &TextBlock, text: &str) -> Vec<String> {
        block
            .lines()
            .iter()
            .map(|line| {
                block.glyphs()[line.glyphs.clone()]
                    .iter()
                    .map(|glyph| {
                        let at = glyph.cluster as usize;
                        text[at..].chars().next().unwrap()
                    })
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect()
    }

    #[test]
    fn words_wrap_at_spaces_and_line_feeds_end_lines() {
        let fonts = Fonts::with_embedded();
        let style = style(&fonts);
        let mut block = TextBlock::new();
        let text = "The quick brown fox jumps over the lazy dog";
        let paragraph = Paragraph {
            width: Some(120.0),
            ..Paragraph::default()
        };
        block.layout(&fonts, text, &style, &paragraph);
        let shown = lines(&block, text);
        assert!(shown.len() >= 3, "{shown:?}");
        assert_eq!(shown.join(" "), text, "nothing lost, words whole");
        assert!(block.lines().iter().all(|line| line.width <= 120.0 + 1e-3));
        // Line feeds end lines, an empty line keeps its place.
        block.layout(&fonts, "one\n\ntwo", &style, &Paragraph::default());
        assert_eq!(lines(&block, "one\n\ntwo"), ["one", "", "two"]);
        assert!((block.height() - 3.0 * 1.4 * 20.0).abs() < 1e-3);
        let baselines: Vec<f32> = block.lines().iter().map(|line| line.baseline).collect();
        assert!((baselines[1] - baselines[0] - 28.0).abs() < 1e-3);
    }

    #[test]
    fn an_overlong_word_breaks_where_it_must_and_lines_align() {
        let fonts = Fonts::with_embedded();
        let style = style(&fonts);
        let mut block = TextBlock::new();
        let paragraph = Paragraph {
            width: Some(50.0),
            line_height: 1.2,
            align: Align::End,
        };
        block.layout(&fonts, "unbreakable", &style, &paragraph);
        assert!(block.lines().len() > 1);
        assert!(block.lines().iter().all(|line| line.width <= 50.0 + 1e-3));
        // Aligned to the end: each line's last glyph ends at the width.
        for line in block.lines() {
            let last = block.glyphs()[line.glyphs.end - 1];
            let end = last.x + crate::shape::advance(&fonts, &last, 20.0);
            assert!((end - 50.0).abs() < 0.5, "{end}");
        }
        block.layout(&fonts, "", &style, &paragraph);
        assert!(block.lines().is_empty() && block.height() == 0.0);
    }
}
