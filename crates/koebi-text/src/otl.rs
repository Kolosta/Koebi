//! OpenType layout, the part a horizontal shaper of Latin and CJK text needs: pair kerning
//! (`GPOS` lookup type 2, both formats, and the older `kern` table) and ligatures (`GSUB` lookup
//! type 4), through extension lookups too.
//!
//! The features come from the default language system of the `DFLT` script, else of `latn`,
//! else of the first script. Every read is bounds-checked: a malformed table gives no kerning
//! and no ligature rather than an error.

use crate::read::{i16_at, u16_at, u32_at};

/// Most lookups a feature may name, and subtables a lookup may hold: far above real fonts.
const MAX_LOOKUPS: usize = 256;
const MAX_SUBTABLES: usize = 256;

/// The lookups of the features `tags` in a `GPOS` or `GSUB` table, in lookup order.
pub(crate) fn feature_lookups(table: &[u8], tags: &[[u8; 4]]) -> Vec<u16> {
    let (Some(scripts), Some(features)) = (u16_at(table, 4), u16_at(table, 6)) else {
        return Vec::new();
    };
    let (scripts, features) = (usize::from(scripts), usize::from(features));
    let Some(lang_sys) = default_lang_sys(table, scripts) else {
        return Vec::new();
    };
    let feature_count = u16_at(table, features).unwrap_or(0);
    let indices = u16_at(table, lang_sys + 4).unwrap_or(0);
    let mut lookups = Vec::new();
    for i in 0..usize::from(indices) {
        let Some(index) = u16_at(table, lang_sys + 6 + 2 * i) else {
            break;
        };
        if index >= feature_count {
            continue;
        }
        let record = features + 2 + 6 * usize::from(index);
        let Some(tag) = table.get(record..record + 4) else {
            continue;
        };
        if !tags.iter().any(|wanted| wanted == tag) {
            continue;
        }
        let Some(offset) = u16_at(table, record + 4) else {
            continue;
        };
        let feature = features + usize::from(offset);
        let count = usize::from(u16_at(table, feature + 2).unwrap_or(0)).min(MAX_LOOKUPS);
        for k in 0..count {
            if let Some(lookup) = u16_at(table, feature + 4 + 2 * k) {
                lookups.push(lookup);
            }
        }
    }
    lookups.sort_unstable();
    lookups.dedup();
    lookups
}

/// The subtables of type `kind` (`extension` the extension lookup type) of the features
/// `tags`, in lookup order: resolved once, when the font loads, so that a kerning pair or a
/// ligature reads them without walking the lookup list, and without allocating.
pub(crate) fn feature_subtables(
    table: &[u8],
    tags: &[[u8; 4]],
    kind: u16,
    extension: u16,
) -> Vec<usize> {
    feature_lookups(table, tags)
        .into_iter()
        .flat_map(|lookup| subtables(table, lookup, kind, extension))
        .collect()
}

/// The offset of the default language system to use, from the script list at `scripts`.
fn default_lang_sys(table: &[u8], scripts: usize) -> Option<usize> {
    let count = usize::from(u16_at(table, scripts)?);
    let record = |i: usize| -> Option<([u8; 4], usize)> {
        let at = scripts + 2 + 6 * i;
        let tag: [u8; 4] = table.get(at..at + 4)?.try_into().ok()?;
        Some((tag, scripts + usize::from(u16_at(table, at + 4)?)))
    };
    let records: Vec<_> = (0..count.min(64)).filter_map(record).collect();
    let script = [*b"DFLT", *b"latn"]
        .iter()
        .find_map(|wanted| records.iter().find(|(tag, _)| tag == wanted))
        .or_else(|| records.first())
        .map(|(_, offset)| *offset)?;
    let default = u16_at(table, script)?;
    (default != 0).then(|| script + usize::from(default))
}

/// The subtables of lookup `index` whose type is `kind`, extension subtables resolved
/// (`extension` the extension lookup type).
fn subtables(table: &[u8], index: u16, kind: u16, extension: u16) -> Vec<usize> {
    let Some(list) = u16_at(table, 8).map(usize::from) else {
        return Vec::new();
    };
    let Some(offset) = u16_at(table, list + 2 + 2 * usize::from(index)) else {
        return Vec::new();
    };
    if index >= u16_at(table, list).unwrap_or(0) {
        return Vec::new();
    }
    let lookup = list + usize::from(offset);
    let (Some(lookup_type), Some(count)) = (u16_at(table, lookup), u16_at(table, lookup + 4))
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for i in 0..usize::from(count).min(MAX_SUBTABLES) {
        let Some(offset) = u16_at(table, lookup + 6 + 2 * i) else {
            break;
        };
        let subtable = lookup + usize::from(offset);
        if lookup_type == kind {
            out.push(subtable);
        } else if lookup_type == extension
            && u16_at(table, subtable) == Some(1)
            && u16_at(table, subtable + 2) == Some(kind)
            && let Some(target) = u32_at(table, subtable + 4)
            && let Ok(target) = usize::try_from(target)
        {
            out.push(subtable + target);
        }
    }
    out
}

/// The coverage index of `glyph` in the coverage table at `at`.
fn coverage(table: &[u8], at: usize, glyph: u16) -> Option<usize> {
    match u16_at(table, at)? {
        1 => {
            let count = usize::from(u16_at(table, at + 2)?);
            let (mut low, mut high) = (0, count);
            while low < high {
                let middle = usize::midpoint(low, high);
                let found = u16_at(table, at + 4 + 2 * middle)?;
                match found.cmp(&glyph) {
                    core::cmp::Ordering::Less => low = middle + 1,
                    core::cmp::Ordering::Greater => high = middle,
                    core::cmp::Ordering::Equal => return Some(middle),
                }
            }
            None
        }
        2 => {
            let count = usize::from(u16_at(table, at + 2)?);
            let (mut low, mut high) = (0, count);
            while low < high {
                let middle = usize::midpoint(low, high);
                let record = at + 4 + 6 * middle;
                let (start, end) = (u16_at(table, record)?, u16_at(table, record + 2)?);
                if glyph < start {
                    high = middle;
                } else if glyph > end {
                    low = middle + 1;
                } else {
                    let first = usize::from(u16_at(table, record + 4)?);
                    return Some(first + usize::from(glyph - start));
                }
            }
            None
        }
        _ => None,
    }
}

/// The class of `glyph` in the class definition at `at` (0 when not listed).
fn class(table: &[u8], at: usize, glyph: u16) -> u16 {
    let found = match u16_at(table, at) {
        Some(1) => (|| {
            let start = u16_at(table, at + 2)?;
            let count = u16_at(table, at + 4)?;
            let index = glyph.checked_sub(start)?;
            if index >= count {
                return None;
            }
            u16_at(table, at + 6 + 2 * usize::from(index))
        })(),
        Some(2) => (|| {
            let count = usize::from(u16_at(table, at + 2)?);
            let (mut low, mut high) = (0, count);
            while low < high {
                let middle = usize::midpoint(low, high);
                let record = at + 4 + 6 * middle;
                let (start, end) = (u16_at(table, record)?, u16_at(table, record + 2)?);
                if glyph < start {
                    high = middle;
                } else if glyph > end {
                    low = middle + 1;
                } else {
                    return u16_at(table, record + 4);
                }
            }
            None
        })(),
        _ => None,
    };
    found.unwrap_or(0)
}

/// Bytes of a value record of `format`.
fn value_size(format: u16) -> usize {
    2 * (format & 0x00FF).count_ones() as usize
}

/// The x advance adjustment of the value record at `at` of `format`.
fn x_advance(table: &[u8], at: usize, format: u16) -> Option<i16> {
    if format & 0x0004 == 0 {
        return None;
    }
    i16_at(table, at + 2 * (format & 0x0003).count_ones() as usize)
}

/// The kerning of the pair (`left`, `right`) in a `GPOS` table, from its pair adjustment
/// `subtables` ([`feature_subtables`]), in font units: the first that has the pair.
pub(crate) fn gpos_kerning(
    table: &[u8],
    subtables: &[usize],
    left: u16,
    right: u16,
) -> Option<i16> {
    subtables
        .iter()
        .find_map(|&subtable| pair_pos(table, subtable, left, right))
}

fn pair_pos(table: &[u8], at: usize, left: u16, right: u16) -> Option<i16> {
    let format = u16_at(table, at)?;
    let cover = coverage(table, at + usize::from(u16_at(table, at + 2)?), left)?;
    let (first, second) = (u16_at(table, at + 4)?, u16_at(table, at + 6)?);
    match format {
        1 => {
            let sets = usize::from(u16_at(table, at + 8)?);
            if cover >= sets {
                return None;
            }
            let set = at + usize::from(u16_at(table, at + 10 + 2 * cover)?);
            let count = usize::from(u16_at(table, set)?);
            let record = 2 + value_size(first) + value_size(second);
            let (mut low, mut high) = (0, count);
            while low < high {
                let middle = usize::midpoint(low, high);
                let entry = set + 2 + record * middle;
                let glyph = u16_at(table, entry)?;
                match glyph.cmp(&right) {
                    core::cmp::Ordering::Less => low = middle + 1,
                    core::cmp::Ordering::Greater => high = middle,
                    core::cmp::Ordering::Equal => return x_advance(table, entry + 2, first),
                }
            }
            None
        }
        2 => {
            let class_1 = class(table, at + usize::from(u16_at(table, at + 8)?), left);
            let class_2 = class(table, at + usize::from(u16_at(table, at + 10)?), right);
            let (count_1, count_2) = (u16_at(table, at + 12)?, u16_at(table, at + 14)?);
            if class_1 >= count_1 || class_2 >= count_2 {
                return None;
            }
            let record = value_size(first) + value_size(second);
            let entry = at
                + 16
                + record * (usize::from(class_1) * usize::from(count_2) + usize::from(class_2));
            x_advance(table, entry, first)
        }
        _ => None,
    }
}

/// The kerning of the pair (`left`, `right`) in a `kern` table (format 0), in font units.
pub(crate) fn kern_table(table: &[u8], left: u16, right: u16) -> Option<i16> {
    let count = u16_at(table, 2)?;
    let mut at = 4;
    for _ in 0..count.min(16) {
        let length = usize::from(u16_at(table, at + 2)?);
        let coverage = u16_at(table, at + 4)?;
        // Format 0, horizontal, not cross-stream.
        if coverage >> 8 == 0 && coverage & 0x0001 != 0 && coverage & 0x0004 == 0 {
            let pairs = usize::from(u16_at(table, at + 6)?);
            let key = (u32::from(left) << 16) | u32::from(right);
            let (mut low, mut high) = (0, pairs);
            while low < high {
                let middle = usize::midpoint(low, high);
                let entry = at + 14 + 6 * middle;
                let found = u32_at(table, entry)?;
                match found.cmp(&key) {
                    core::cmp::Ordering::Less => low = middle + 1,
                    core::cmp::Ordering::Greater => high = middle,
                    core::cmp::Ordering::Equal => return i16_at(table, entry + 4),
                }
            }
        }
        at += length.max(6);
    }
    None
}

/// The ligature starting the glyphs of `run` in a `GSUB` table, from its ligature
/// `subtables` ([`feature_subtables`]): the ligature glyph and how many glyphs it replaces (2
/// or more), from the first subtable that has one.
pub(crate) fn ligature(table: &[u8], subtables: &[usize], run: &[u16]) -> Option<(u16, usize)> {
    let (&first, rest) = run.split_first()?;
    subtables
        .iter()
        .find_map(|&subtable| ligature_subst(table, subtable, first, rest))
}

fn ligature_subst(table: &[u8], at: usize, first: u16, rest: &[u16]) -> Option<(u16, usize)> {
    if u16_at(table, at)? != 1 {
        return None;
    }
    let cover = coverage(table, at + usize::from(u16_at(table, at + 2)?), first)?;
    if cover >= usize::from(u16_at(table, at + 4)?) {
        return None;
    }
    let set = at + usize::from(u16_at(table, at + 6 + 2 * cover)?);
    let count = usize::from(u16_at(table, set)?);
    for i in 0..count.min(MAX_SUBTABLES) {
        let ligature = set + usize::from(u16_at(table, set + 2 + 2 * i)?);
        let glyph = u16_at(table, ligature)?;
        let components = usize::from(u16_at(table, ligature + 2)?);
        if components < 2 || components - 1 > rest.len() {
            continue;
        }
        let matches =
            (0..components - 1).all(|k| u16_at(table, ligature + 4 + 2 * k) == Some(rest[k]));
        if matches {
            return Some((glyph, components));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_tables_of_both_formats_give_indices() {
        // Format 1: glyphs 3, 7, 9.
        let list = [0, 1, 0, 3, 0, 3, 0, 7, 0, 9];
        assert_eq!(coverage(&list, 0, 7), Some(1));
        assert_eq!(coverage(&list, 0, 8), None);
        // Format 2: glyphs 10 to 20 from index 5.
        let ranges = [0, 2, 0, 1, 0, 10, 0, 20, 0, 5];
        assert_eq!(coverage(&ranges, 0, 12), Some(7));
        assert_eq!(coverage(&ranges, 0, 21), None);
        // Truncated or unknown: nothing, no panic.
        assert_eq!(coverage(&[0, 1, 0, 9], 0, 3), None);
        assert_eq!(coverage(&[0, 3], 0, 3), None);
    }

    #[test]
    fn kern_tables_find_their_pairs() {
        // One format 0 subtable: (1, 2) = -50, (3, 4) = 20.
        let table = [
            0, 0, 0, 1, // version, one subtable
            0, 0, 0, 26, 0, 1, // version, length, coverage: horizontal, format 0
            0, 2, 0, 0, 0, 0, 0, 0, // two pairs, search fields
            0, 1, 0, 2, 0xFF, 0xCE, // (1, 2) -50
            0, 3, 0, 4, 0, 20, // (3, 4) 20
        ];
        assert_eq!(kern_table(&table, 1, 2), Some(-50));
        assert_eq!(kern_table(&table, 3, 4), Some(20));
        assert_eq!(kern_table(&table, 2, 1), None);
        assert_eq!(kern_table(&table[..20], 3, 4), None);
    }

    #[test]
    fn value_records_place_the_x_advance_after_the_placements() {
        assert_eq!(value_size(0x0004), 2);
        assert_eq!(value_size(0x000F), 8);
        let record = [0, 1, 0, 2, 0xFF, 0x9C];
        assert_eq!(x_advance(&record, 0, 0x0007), Some(-100));
        assert_eq!(x_advance(&record, 0, 0x0003), None);
    }

    #[test]
    fn malformed_layout_tables_give_nothing() {
        assert_eq!(feature_lookups(&[], &[*b"kern"]), [] as [u16; 0]);
        assert_eq!(feature_lookups(&[0; 10], &[*b"kern"]), [] as [u16; 0]);
        assert_eq!(gpos_kerning(&[0; 10], &[0], 1, 2), None);
        assert_eq!(ligature(&[0; 10], &[0], &[1, 2]), None);
    }
}
