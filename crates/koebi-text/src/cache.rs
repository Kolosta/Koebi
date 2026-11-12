//! Lines laid out once and reused while their text and style stay the same.
//!
//! An interface draws the same labels frame after frame: shaping them again each time (fonts
//! for each character, `cmap`, kerning and ligature lookups) was most of a frame. A
//! [`LineCache`] keeps each line laid out under its text and style; a frame looks it up with a
//! hash and one comparison of the text, and only text never seen, or not seen for a while, is
//! shaped.
//!
//! Lines unused for [`KEEP_FRAMES`] frames are dropped at the end of a frame, and the cache never
//! holds more than [`MAX_LINES`]: memory stays bounded however much text goes by (a counter, a
//! field being typed in). A steady frame allocates nothing.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use crate::fonts::{FontId, Fonts};
use crate::line::{TextLine, TextStyle};

/// Frames a line stays cached without being used.
pub const KEEP_FRAMES: u64 = 120;
/// Most lines cached at once.
pub const MAX_LINES: usize = 8192;

/// What a line was laid out from, but its text: the text's hash stands for it, its bytes are
/// compared on a match.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Key {
    text: u64,
    font: FontId,
    size: u32,
    tracking: u32,
    kerning: bool,
    ligatures: bool,
    /// The width the line was cut to, `u32::MAX` for none.
    max_width: u32,
}

impl Key {
    fn new(text: &str, style: &TextStyle, max_width: Option<f32>) -> Self {
        Self {
            text: hash_text(text),
            font: style.font,
            size: style.size.to_bits(),
            tracking: style.tracking.to_bits(),
            kerning: style.features.kerning,
            ligatures: style.features.ligatures,
            max_width: max_width.map_or(u32::MAX, f32::to_bits),
        }
    }
}

/// FNV-1a over the text's bytes: fast on short strings, enough to tell labels apart (a match
/// still compares the text).
fn hash_text(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// A hasher for keys already made of a hash: it mixes the fields as they come, without the
/// cost of `SipHash`'s protection against chosen keys (the keys are the interface's own text).
#[derive(Clone, Copy, Debug, Default)]
struct KeyHasher(u64);

impl Hasher for KeyHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.write_u64(u64::from(byte));
        }
    }

    fn write_u8(&mut self, value: u8) {
        self.write_u64(u64::from(value));
    }

    fn write_u32(&mut self, value: u32) {
        self.write_u64(u64::from(value));
    }

    fn write_u64(&mut self, value: u64) {
        // The multiply-rotate step of FxHash.
        self.0 = (self.0.rotate_left(5) ^ value).wrapping_mul(0x517c_c1b7_2722_0a95);
    }

    fn write_usize(&mut self, value: usize) {
        self.write_u64(value as u64);
    }
}

#[derive(Clone, Debug)]
struct Entry {
    text: Box<str>,
    line: TextLine,
    used: u64,
}

/// Lines laid out from their text and style, kept while they are used ([module](self)).
#[derive(Clone, Debug, Default)]
pub struct LineCache {
    lines: HashMap<Key, Entry, BuildHasherDefault<KeyHasher>>,
    frame: u64,
    /// Kept for [`Self::end_frame`]: the keys to drop, without allocating each frame.
    stale: Vec<Key>,
}

impl LineCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// `text` laid out in `style` on one line, cut with an ellipsis to `max_width` when given
    /// ([`TextLine::layout_truncated`]): from the cache, or laid out and kept.
    pub fn line(
        &mut self,
        fonts: &Fonts,
        text: &str,
        style: &TextStyle,
        max_width: Option<f32>,
    ) -> &TextLine {
        let key = Key::new(text, style, max_width);
        let frame = self.frame;
        let hit = matches!(self.lines.get(&key), Some(entry) if &*entry.text == text);
        if !hit && !self.lines.contains_key(&key) && self.lines.len() >= MAX_LINES {
            self.evict_oldest();
        }
        let entry = self.lines.entry(key).or_insert_with(|| Entry {
            text: Box::from(""),
            line: TextLine::new(),
            used: frame,
        });
        if !hit {
            // A new line, or a hash collision replacing the other text: rare, it only costs a
            // layout.
            entry.text = text.into();
            match max_width {
                Some(width) => entry.line.layout_truncated(fonts, text, style, width),
                None => entry.line.layout(fonts, text, style),
            }
        }
        entry.used = frame;
        &entry.line
    }

    /// Ends a frame: lines unused for [`KEEP_FRAMES`] frames go.
    pub fn end_frame(&mut self) {
        self.frame += 1;
        let frame = self.frame;
        // Every frame, a look at each line would cost as much as the lookups; once a while is
        // enough.
        if !frame.is_multiple_of(32) {
            return;
        }
        self.stale.clear();
        self.stale.extend(
            self.lines
                .iter()
                .filter(|(_, entry)| frame - entry.used > KEEP_FRAMES)
                .map(|(key, _)| *key),
        );
        for key in &self.stale {
            self.lines.remove(key);
        }
    }

    /// Forgets every line: the fonts changed.
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// How many lines are cached.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether nothing is cached.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Makes room for a line: the least recently used goes.
    fn evict_oldest(&mut self) {
        if let Some(key) = self
            .lines
            .iter()
            .min_by_key(|(_, entry)| entry.used)
            .map(|(key, _)| *key)
        {
            self.lines.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style(fonts: &Fonts) -> TextStyle {
        let font = fonts
            .select(crate::embedded::NOTO_SANS, 400)
            .expect("Noto Sans is embedded");
        TextStyle::new(font, 14.0)
    }

    #[test]
    fn a_line_is_laid_out_once_and_matches_a_fresh_layout() {
        let fonts = Fonts::with_embedded();
        let style = style(&fonts);
        let mut cache = LineCache::new();
        let mut fresh = TextLine::new();
        fresh.layout(&fonts, "Settings", &style);
        let width = cache.line(&fonts, "Settings", &style, None).width();
        assert_eq!(width, fresh.width());
        assert_eq!(
            cache.line(&fonts, "Settings", &style, None).glyphs(),
            fresh.glyphs()
        );
        assert_eq!(cache.len(), 1);
        // Another width to cut to, another size: other lines.
        let cut = cache.line(&fonts, "Settings", &style, Some(20.0)).width();
        assert!(cut <= 20.0);
        let mut bigger = style;
        bigger.size = 20.0;
        cache.line(&fonts, "Settings", &bigger, None);
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn unused_lines_go_and_used_ones_stay() {
        let fonts = Fonts::with_embedded();
        let style = style(&fonts);
        let mut cache = LineCache::new();
        cache.line(&fonts, "Gone", &style, None);
        for _ in 0..KEEP_FRAMES + 64 {
            cache.line(&fonts, "Kept", &style, None);
            cache.end_frame();
        }
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.line(&fonts, "Kept", &style, None).glyphs().len(), 4);
    }

    #[test]
    fn the_cache_stays_bounded() {
        let fonts = Fonts::with_embedded();
        let style = style(&fonts);
        let mut cache = LineCache::new();
        for number in 0..MAX_LINES + 10 {
            cache.line(&fonts, &number.to_string(), &style, None);
        }
        assert_eq!(cache.len(), MAX_LINES);
    }
}
