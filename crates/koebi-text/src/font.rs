//! A TrueType font: its tables, character map, metrics, advances, family name, and the layout
//! data a horizontal shaper reads (kerning pairs and ligatures, [`crate::otl`]). Fonts of a
//! collection (`.ttc`) are read one by one.

use std::borrow::Cow;
use std::fmt;
use std::ops::Range;

use koebi_core::{Rect, Vec2};

use crate::read::{Tag, slice, u16_at, u32_at};
use crate::{Error, Result};

/// A glyph of a font, by index.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlyphId(pub u16);

impl GlyphId {
    /// The glyph drawn for characters the font lacks.
    pub const NOTDEF: Self = Self(0);
}

/// Vertical metrics of a font, in font units, y up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontMetrics {
    /// Font units per em: the size the other values are relative to.
    pub units_per_em: u16,
    /// Distance from the baseline to the top of the line (positive).
    pub ascender: i16,
    /// Distance from the baseline to the bottom of the line (negative).
    pub descender: i16,
    /// Extra space between lines.
    pub line_gap: i16,
}

/// The character map subtable chosen at load.
#[derive(Clone, Copy, Debug)]
enum Cmap {
    /// Segment mapping to delta values: the Basic Multilingual Plane.
    Format4 { offset: usize, segments: usize },
    /// Segmented coverage: every plane.
    Format12 { offset: usize, groups: usize },
}

/// A TrueType font, parsed enough to map characters and draw glyphs.
///
/// Every table the font needs is located and checked at load; glyph outlines are read when
/// needed ([`crate::Outline::load`]), with every read bounds-checked.
pub struct Font {
    data: Cow<'static, [u8]>,
    metrics: FontMetrics,
    glyph_count: u16,
    weight: u16,
    long_loca: bool,
    h_metrics: u16,
    hmtx: Range<usize>,
    loca: Range<usize>,
    glyf: Range<usize>,
    cmap_table: Range<usize>,
    cmap: Cmap,
    /// Glyphs of the ASCII characters, looked up once.
    ascii: [u16; 128],
    /// The family name (`name` table, typographic family first), empty when the font has none.
    family: String,
    gpos: Option<Range<usize>>,
    gsub: Option<Range<usize>>,
    kern: Option<Range<usize>>,
    /// The `GPOS` lookups of the `kern` feature.
    kerning_lookups: Vec<u16>,
    /// The `GSUB` lookups of the `liga` and `clig` features.
    ligature_lookups: Vec<u16>,
}

impl Font {
    /// Reads a font embedded in the binary.
    ///
    /// # Errors
    ///
    /// See [`Font::from_vec`].
    pub fn from_static(data: &'static [u8]) -> Result<Self> {
        Self::parse(Cow::Borrowed(data), 0)
    }

    /// Reads font `index` of a collection (`.ttc`) or, at index 0, a single font.
    ///
    /// # Errors
    ///
    /// See [`Font::from_vec`]; [`Error::Malformed`] for an index past the collection.
    pub fn from_collection(data: Vec<u8>, index: u32) -> Result<Self> {
        let at = collection_offset(&data, index)?;
        Self::parse(Cow::Owned(data), at)
    }

    /// How many fonts `data` holds: a collection's count, 1 for a single font, 0 for other data.
    pub fn count_in(data: &[u8]) -> u32 {
        match u32_at(data, 0) {
            Some(0x7474_6366) => u32_at(data, 8).unwrap_or(0).min(256),
            Some(0x0001_0000 | 0x7472_7565 | 0x4F54_544F) => 1,
            _ => 0,
        }
    }

    /// Reads a font from its file's bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotAFont`] for other data, [`Error::Unsupported`] for CFF outlines and
    /// collections, [`Error::MissingTable`] or [`Error::Malformed`] for broken fonts.
    pub fn from_vec(data: Vec<u8>) -> Result<Self> {
        Self::parse(Cow::Owned(data), 0)
    }

    fn parse(data: Cow<'static, [u8]>, at: usize) -> Result<Self> {
        let directory = Directory::read(&data, at)?;
        let table = |tag| directory.find(&data, tag)?.ok_or(Error::MissingTable(tag));

        let head = table(Tag::HEAD)?;
        let malformed = |table, reason| Error::Malformed { table, reason };
        let head_data = &data[head];
        if u32_at(head_data, 12) != Some(0x5F0F_3CF5) {
            return Err(malformed(Tag::HEAD, "bad magic number"));
        }
        let units_per_em = u16_at(head_data, 18)
            .filter(|units| (16..=16384).contains(units))
            .ok_or(malformed(Tag::HEAD, "units per em out of range"))?;
        let long_loca = match u16_at(head_data, 50) {
            Some(0) => false,
            Some(1) => true,
            _ => return Err(malformed(Tag::HEAD, "unknown loca format")),
        };

        let maxp = table(Tag::MAXP)?;
        let glyph_count = u16_at(&data[maxp], 4)
            .filter(|count| *count > 0)
            .ok_or(malformed(Tag::MAXP, "no glyph"))?;

        let hhea = table(Tag::HHEA)?;
        let hhea_data = &data[hhea];
        let vertical = |offset| {
            crate::read::i16_at(hhea_data, offset).ok_or(malformed(Tag::HHEA, "truncated"))
        };
        let metrics = FontMetrics {
            units_per_em,
            ascender: vertical(4)?,
            descender: vertical(6)?,
            line_gap: vertical(8)?,
        };
        let h_metrics = u16_at(hhea_data, 34)
            .filter(|count| (1..=glyph_count).contains(count))
            .ok_or(malformed(Tag::HHEA, "bad number of horizontal metrics"))?;

        let hmtx = table(Tag::HMTX)?;
        let needed = 4 * usize::from(h_metrics) + 2 * usize::from(glyph_count - h_metrics);
        if hmtx.len() < needed {
            return Err(malformed(Tag::HMTX, "truncated"));
        }
        let loca = table(Tag::LOCA)?;
        let entry = if long_loca { 4 } else { 2 };
        if loca.len() < (usize::from(glyph_count) + 1) * entry {
            return Err(malformed(Tag::LOCA, "truncated"));
        }
        let glyf = table(Tag::GLYF)?;
        let weight = match directory.find(&data, Tag::OS2)? {
            Some(os2) => u16_at(&data[os2], 4)
                .filter(|weight| (1..=1000).contains(weight))
                .unwrap_or(400),
            None => 400,
        };
        let cmap_table = table(Tag::CMAP)?;
        let cmap = Cmap::choose(&data[cmap_table.clone()])?;
        let gpos = directory.find(&data, Tag::GPOS)?;
        let gsub = directory.find(&data, Tag::GSUB)?;
        let kern = directory.find(&data, Tag::KERN)?;
        let kerning_lookups = gpos
            .clone()
            .map(|range| crate::otl::feature_lookups(&data[range], &[*b"kern"]))
            .unwrap_or_default();
        let ligature_lookups = gsub
            .clone()
            .map(|range| crate::otl::feature_lookups(&data[range], &[*b"liga", *b"clig"]))
            .unwrap_or_default();
        let family = directory
            .find(&data, Tag::NAME)?
            .and_then(|range| family_name(&data[range]))
            .unwrap_or_default();

        let mut font = Self {
            data,
            metrics,
            glyph_count,
            weight,
            long_loca,
            h_metrics,
            hmtx,
            loca,
            glyf,
            cmap_table,
            cmap,
            ascii: [0; 128],
            family,
            gpos,
            gsub,
            kern,
            kerning_lookups,
            ligature_lookups,
        };
        let mut ascii = [0; 128];
        for (code, glyph) in (0_u32..).zip(&mut ascii) {
            *glyph = font.lookup(code).map_or(0, |found| found.0);
        }
        font.ascii = ascii;
        Ok(font)
    }

    /// Vertical metrics.
    pub fn metrics(&self) -> FontMetrics {
        self.metrics
    }

    /// Font units per em.
    pub fn units_per_em(&self) -> u16 {
        self.metrics.units_per_em
    }

    /// Number of glyphs.
    pub fn glyph_count(&self) -> u16 {
        self.glyph_count
    }

    /// Weight, from 1 to 1000 (400 regular, 700 bold); 400 when the font does not say.
    pub fn weight(&self) -> u16 {
        self.weight
    }

    /// The glyph of `c`, if the font has one.
    pub fn glyph_index(&self, c: char) -> Option<GlyphId> {
        let code = u32::from(c);
        if let Some(glyph) = usize::try_from(code).ok().and_then(|i| self.ascii.get(i)) {
            return (*glyph != 0).then_some(GlyphId(*glyph));
        }
        self.lookup(code)
    }

    /// The family name the font gives (`Noto Sans`), empty when it gives none.
    pub fn family(&self) -> &str {
        &self.family
    }

    /// The kerning between `left` and `right`, in font units: the `GPOS` `kern` feature's pair
    /// adjustment, else the `kern` table's; 0 without one.
    pub fn kerning(&self, left: GlyphId, right: GlyphId) -> i16 {
        if let Some(gpos) = &self.gpos
            && !self.kerning_lookups.is_empty()
        {
            return crate::otl::gpos_kerning(
                &self.data[gpos.clone()],
                &self.kerning_lookups,
                left.0,
                right.0,
            )
            .unwrap_or(0);
        }
        self.kern
            .as_ref()
            .and_then(|kern| crate::otl::kern_table(&self.data[kern.clone()], left.0, right.0))
            .unwrap_or(0)
    }

    /// `true` when the font kerns pairs (a `kern` feature or table).
    pub fn has_kerning(&self) -> bool {
        !self.kerning_lookups.is_empty() || self.kern.is_some()
    }

    /// The ligature (`liga` and `clig` features) the glyphs of `run` start with: the ligature
    /// glyph and how many glyphs of the run it replaces.
    pub fn ligature(&self, run: &[GlyphId]) -> Option<(GlyphId, usize)> {
        let gsub = self.gsub.as_ref()?;
        if self.ligature_lookups.is_empty() || run.len() < 2 {
            return None;
        }
        let mut glyphs = [0_u16; 8];
        let len = run.len().min(glyphs.len());
        for (slot, glyph) in glyphs.iter_mut().zip(run) {
            *slot = glyph.0;
        }
        crate::otl::ligature(
            &self.data[gsub.clone()],
            &self.ligature_lookups,
            &glyphs[..len],
        )
        .filter(|&(glyph, _)| glyph < self.glyph_count)
        .map(|(glyph, count)| (GlyphId(glyph), count))
    }

    /// Horizontal advance of `glyph`, in font units.
    pub fn advance(&self, glyph: GlyphId) -> u16 {
        let index = usize::from(glyph.0.min(self.h_metrics - 1));
        // Within the table: its length was checked at load.
        u16_at(&self.data[self.hmtx.clone()], 4 * index).unwrap_or(0)
    }

    /// The box the outline of `glyph` lies in, in font units, y up: `[x_min, y_min, x_max,
    /// y_max]` from its header in the `glyf` table. `None` for a glyph without outline (a space)
    /// or one that cannot be read.
    pub fn glyph_box(&self, glyph: GlyphId) -> Option<[i16; 4]> {
        let data = self.glyph_data(glyph).ok()?;
        let at = |offset| crate::read::i16_at(data, offset);
        let [x_min, y_min, x_max, y_max] = [at(2)?, at(4)?, at(6)?, at(8)?];
        (x_min <= x_max && y_min <= y_max).then_some([x_min, y_min, x_max, y_max])
    }

    /// The box the outline of `glyph` covers at `size` pixels per em, in pixels from its pen
    /// on the baseline, y down ([`Font::glyph_box`]).
    pub fn ink(&self, glyph: GlyphId, size: f32) -> Option<Rect> {
        let [x_min, y_min, x_max, y_max] = self.glyph_box(glyph)?;
        let scale = size / f32::from(self.units_per_em());
        Some(Rect::from_min_max(
            Vec2::new(f32::from(x_min) * scale, -f32::from(y_max) * scale),
            Vec2::new(f32::from(x_max) * scale, -f32::from(y_min) * scale),
        ))
    }

    /// The outline data of `glyph` in the `glyf` table; empty for glyphs without outline.
    pub(crate) fn glyph_data(&self, glyph: GlyphId) -> Result<&[u8]> {
        if glyph.0 >= self.glyph_count {
            return Err(Error::UnknownGlyph(glyph.0));
        }
        let loca = &self.data[self.loca.clone()];
        let index = usize::from(glyph.0);
        let range = if self.long_loca {
            u32_at(loca, 4 * index).zip(u32_at(loca, 4 * index + 4))
        } else {
            u16_at(loca, 2 * index)
                .zip(u16_at(loca, 2 * index + 2))
                .map(|(start, end)| (u32::from(start) * 2, u32::from(end) * 2))
        };
        let malformed = |reason| Error::Malformed {
            table: Tag::LOCA,
            reason,
        };
        let (start, end) = range.ok_or(malformed("truncated"))?;
        let len = end
            .checked_sub(start)
            .ok_or(malformed("glyph offsets go backwards"))?;
        let start = usize::try_from(start).map_err(|_| malformed("offset too large"))?;
        let len = usize::try_from(len).map_err(|_| malformed("offset too large"))?;
        slice(&self.data[self.glyf.clone()], start, len).ok_or(Error::Malformed {
            table: Tag::GLYF,
            reason: "glyph beyond the table",
        })
    }

    /// Looks `code` up in the character map.
    fn lookup(&self, code: u32) -> Option<GlyphId> {
        let cmap = &self.data[self.cmap_table.clone()];
        let glyph = match self.cmap {
            Cmap::Format4 { offset, segments } => format4(cmap, offset, segments, code)?,
            Cmap::Format12 { offset, groups } => format12(cmap, offset, groups, code)?,
        };
        (glyph != 0 && glyph < self.glyph_count).then_some(GlyphId(glyph))
    }
}

impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Font")
            .field("glyphs", &self.glyph_count)
            .field("weight", &self.weight)
            .field("metrics", &self.metrics)
            .finish_non_exhaustive()
    }
}

/// The offset of font `index`'s table directory: in a collection, its entry; 0 otherwise.
fn collection_offset(data: &[u8], index: u32) -> Result<usize> {
    if u32_at(data, 0) != Some(0x7474_6366) {
        return if index == 0 {
            Ok(0)
        } else {
            Err(Error::NotAFont)
        };
    }
    let malformed = Error::Malformed {
        table: Tag(*b"ttcf"),
        reason: "no font at this index",
    };
    let count = u32_at(data, 8).ok_or(Error::NotAFont)?;
    if index >= count {
        return Err(malformed);
    }
    let entry = 12_usize
        .checked_add(4 * usize::try_from(index).map_err(|_| malformed.clone())?)
        .ok_or(malformed.clone())?;
    let offset = u32_at(data, entry).ok_or(malformed.clone())?;
    usize::try_from(offset).map_err(|_| malformed)
}

/// The family name of a `name` table: the typographic family (name 16) if given, else the
/// family (name 1); Windows Unicode records first, then Macintosh Roman ones.
pub(crate) fn family_name(name: &[u8]) -> Option<String> {
    let count = usize::from(u16_at(name, 2)?);
    let strings = usize::from(u16_at(name, 4)?);
    let mut best: Option<(u8, String)> = None;
    for i in 0..count.min(1024) {
        let record = 6 + 12 * i;
        let (platform, encoding) = (u16_at(name, record)?, u16_at(name, record + 2)?);
        let language = u16_at(name, record + 4)?;
        let id = u16_at(name, record + 6)?;
        let (len, offset) = (
            usize::from(u16_at(name, record + 8)?),
            usize::from(u16_at(name, record + 10)?),
        );
        let Some(bytes) = slice(name, strings + offset, len) else {
            continue;
        };
        let text = match (platform, encoding) {
            (3, 0 | 1 | 10) | (0, _) => {
                let (pairs, _) = bytes.as_chunks::<2>();
                let units: Vec<u16> = pairs.iter().map(|pair| u16::from_be_bytes(*pair)).collect();
                String::from_utf16(&units).ok()
            }
            (1, 0) => Some(bytes.iter().map(|&b| char::from(b)).collect()),
            _ => None,
        };
        let Some(text) = text.filter(|text| !text.is_empty()) else {
            continue;
        };
        // Lower is better: typographic family first, English and Windows first.
        let rank = match id {
            16 => 0,
            1 => 4,
            _ => continue,
        } + u8::from(platform != 3)
            + 2 * u8::from(platform == 3 && language != 0x0409);
        if best.as_ref().is_none_or(|(kept, _)| rank < *kept) {
            best = Some((rank, text));
        }
    }
    best.map(|(_, text)| text)
}

/// The table directory, at its offset in the file.
struct Directory {
    at: usize,
    tables: u16,
}

impl Directory {
    fn read(data: &[u8], at: usize) -> Result<Self> {
        match u32_at(data, at).ok_or(Error::NotAFont)? {
            0x0001_0000 | 0x7472_7565 => {}
            0x4F54_544F => return Err(Error::Unsupported("CFF outlines")),
            0x7474_6366 => return Err(Error::Unsupported("nested font collections")),
            _ => return Err(Error::NotAFont),
        }
        let tables = u16_at(data, at + 4).ok_or(Error::NotAFont)?;
        Ok(Self { at, tables })
    }

    /// The byte range of table `tag`, checked to lie inside the data.
    fn find(&self, data: &[u8], tag: Tag) -> Result<Option<Range<usize>>> {
        for index in 0..usize::from(self.tables) {
            let record = self.at + 12 + 16 * index;
            let found = slice(data, record, 4).ok_or(Error::NotAFont)?;
            if found != tag.0 {
                continue;
            }
            let malformed = Error::Malformed {
                table: tag,
                reason: "outside the file",
            };
            let offset = u32_at(data, record + 8).ok_or(Error::NotAFont)?;
            let length = u32_at(data, record + 12).ok_or(Error::NotAFont)?;
            let start = usize::try_from(offset).map_err(|_| malformed.clone())?;
            let len = usize::try_from(length).map_err(|_| malformed.clone())?;
            let end = start.checked_add(len).ok_or(malformed.clone())?;
            if end > data.len() {
                return Err(malformed);
            }
            return Ok(Some(start..end));
        }
        Ok(None)
    }
}

impl Cmap {
    /// The best Unicode subtable: full coverage (format 12) first, then the BMP (format 4).
    fn choose(cmap: &[u8]) -> Result<Self> {
        let malformed = |reason| Error::Malformed {
            table: Tag::CMAP,
            reason,
        };
        let count = u16_at(cmap, 2).ok_or(malformed("truncated"))?;
        let mut best: Option<(u8, Self)> = None;
        for index in 0..usize::from(count) {
            let record = 4 + 8 * index;
            let platform = u16_at(cmap, record).ok_or(malformed("truncated"))?;
            let encoding = u16_at(cmap, record + 2).ok_or(malformed("truncated"))?;
            let offset = u32_at(cmap, record + 4).ok_or(malformed("truncated"))?;
            let offset = usize::try_from(offset).map_err(|_| malformed("offset too large"))?;
            let unicode = platform == 0 || (platform == 3 && matches!(encoding, 1 | 10));
            if !unicode {
                continue;
            }
            let (rank, subtable) = match u16_at(cmap, offset) {
                Some(4) => {
                    let segments = u16_at(cmap, offset + 6).ok_or(malformed("truncated"))? / 2;
                    // End codes, a pad, start codes, deltas and range offsets.
                    let arrays = 16 + 8 * usize::from(segments);
                    if segments == 0 || slice(cmap, offset, arrays).is_none() {
                        return Err(malformed("truncated format 4 subtable"));
                    }
                    let segments = usize::from(segments);
                    (1, Self::Format4 { offset, segments })
                }
                Some(12) => {
                    let groups = u32_at(cmap, offset + 12).ok_or(malformed("truncated"))?;
                    let groups =
                        usize::try_from(groups).map_err(|_| malformed("too many groups"))?;
                    let size = groups.checked_mul(12).and_then(|size| size.checked_add(16));
                    if size.is_none_or(|size| slice(cmap, offset, size).is_none()) {
                        return Err(malformed("truncated format 12 subtable"));
                    }
                    (2, Self::Format12 { offset, groups })
                }
                _ => continue,
            };
            if best.as_ref().is_none_or(|(best, _)| rank > *best) {
                best = Some((rank, subtable));
            }
        }
        best.map(|(_, subtable)| subtable).ok_or(Error::Unsupported(
            "no Unicode character map in format 4 or 12",
        ))
    }
}

/// Format 4: segments of consecutive codes, mapped by a delta or through an array.
fn format4(cmap: &[u8], offset: usize, segments: usize, code: u32) -> Option<u16> {
    let code = u16::try_from(code).ok()?;
    let ends = offset + 14;
    let starts = ends + 2 * segments + 2;
    let deltas = starts + 2 * segments;
    let ranges = deltas + 2 * segments;
    // The first segment whose end is at or after `code`.
    let (mut low, mut high) = (0, segments);
    while low < high {
        let middle = usize::midpoint(low, high);
        if u16_at(cmap, ends + 2 * middle)? < code {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    if low == segments {
        return None;
    }
    let start = u16_at(cmap, starts + 2 * low)?;
    if code < start {
        return None;
    }
    let delta = u16_at(cmap, deltas + 2 * low)?;
    let range_offset = u16_at(cmap, ranges + 2 * low)?;
    if range_offset == 0 {
        return Some(code.wrapping_add(delta));
    }
    // The offset is relative to its own position in the range offset array.
    let at = ranges + 2 * low + usize::from(range_offset) + 2 * usize::from(code - start);
    let glyph = u16_at(cmap, at)?;
    (glyph != 0).then(|| glyph.wrapping_add(delta))
}

/// Format 12: sorted groups of consecutive codes mapped to consecutive glyphs.
fn format12(cmap: &[u8], offset: usize, groups: usize, code: u32) -> Option<u16> {
    let (mut low, mut high) = (0, groups);
    while low < high {
        let middle = usize::midpoint(low, high);
        let group = offset + 16 + 12 * middle;
        let (start, end) = (u32_at(cmap, group)?, u32_at(cmap, group + 4)?);
        if code < start {
            high = middle;
        } else if code > end {
            low = middle + 1;
        } else {
            let first = u32_at(cmap, group + 8)?;
            return u16::try_from(first.checked_add(code - start)?).ok();
        }
    }
    None
}
