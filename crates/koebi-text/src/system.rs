//! The system's fonts: where each platform keeps them, the families they hold (read from their
//! headers only), and the Korean and Chinese fonts that complete the embedded ones.
//!
//! Scanning reads a few kilobytes of each file (its table directory, `OS/2` and `name`), so that
//! a list of hundreds of fonts costs little; loading reads the whole file. Fonts with CFF
//! outlines (`OTTO` files, most `.otf`) are listed as unsupported and skipped: Koebi reads
//! TrueType outlines (`glyf`) for now.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::font::Font;
use crate::read::{u16_at, u32_at};
use crate::{Error, Result};

/// A font of the system: its family, weight, file and index in the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemFont {
    /// Its family name (`Segoe UI`).
    pub family: String,
    /// Its weight, from 1 to 1000.
    pub weight: u16,
    /// Its file.
    pub path: PathBuf,
    /// Its index in a collection, 0 for a single font.
    pub index: u32,
}

impl SystemFont {
    /// Reads the font.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] when the file cannot be read; see [`Font::from_collection`].
    pub fn load(&self) -> Result<Font> {
        let data = std::fs::read(&self.path).map_err(|error| Error::Io(error.to_string()))?;
        Font::from_collection(data, self.index)
    }
}

/// Where the platform keeps fonts, the user's own first.
pub fn directories() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if cfg!(target_os = "windows") {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            out.push(
                PathBuf::from(local)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Fonts"),
            );
        }
        let windows =
            std::env::var_os("WINDIR").map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
        out.push(windows.join("Fonts"));
    } else if cfg!(target_os = "macos") {
        if let Some(home) = &home {
            out.push(home.join("Library").join("Fonts"));
        }
        out.extend(
            [
                "/Library/Fonts",
                "/System/Library/Fonts",
                "/System/Library/Fonts/Supplemental",
            ]
            .map(PathBuf::from),
        );
    } else if cfg!(target_os = "android") {
        out.push(PathBuf::from("/system/fonts"));
    } else if cfg!(target_os = "ios") {
        out.push(PathBuf::from("/System/Library/Fonts"));
    } else {
        if let Some(home) = &home {
            out.push(home.join(".local").join("share").join("fonts"));
            out.push(home.join(".fonts"));
        }
        out.extend(["/usr/local/share/fonts", "/usr/share/fonts"].map(PathBuf::from));
    }
    out
}

/// Most files a scan reads, and most directory levels it goes down: a bounded cost.
const MAX_FILES: usize = 4096;
const MAX_DEPTH: usize = 4;

/// Every TrueType font in [`directories`], by family then weight. Files that are not fonts, or
/// whose outlines Koebi cannot read, are skipped.
pub fn scan() -> Vec<SystemFont> {
    let mut files = Vec::new();
    for directory in directories() {
        collect(&directory, 0, &mut files);
    }
    let mut fonts: Vec<SystemFont> = files.iter().flat_map(|path| fonts_in(path)).collect();
    fonts.sort_by(|a, b| a.family.cmp(&b.family).then(a.weight.cmp(&b.weight)));
    fonts.dedup_by(|a, b| a.family == b.family && a.weight == b.weight);
    fonts
}

fn collect(directory: &Path, depth: usize, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        if files.len() >= MAX_FILES {
            return;
        }
        let path = entry.path();
        if path.is_dir() {
            if depth < MAX_DEPTH {
                collect(&path, depth + 1, files);
            }
            continue;
        }
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        if matches!(extension.as_deref(), Some("ttf" | "ttc" | "otf")) {
            files.push(path);
        }
    }
}

/// The fonts of one file, from its headers.
fn fonts_in(path: &Path) -> Vec<SystemFont> {
    let Ok(mut file) = File::open(path) else {
        return Vec::new();
    };
    let Some(head) = read_at(&mut file, 0, 16) else {
        return Vec::new();
    };
    let offsets: Vec<u32> = match u32_at(&head, 0) {
        Some(0x7474_6366) => {
            let count = u32_at(&head, 8).unwrap_or(0).min(64);
            read_at(&mut file, 12, 4 * count as usize)
                .map(|table| {
                    (0..count as usize)
                        .filter_map(|i| u32_at(&table, 4 * i))
                        .collect()
                })
                .unwrap_or_default()
        }
        Some(0x0001_0000 | 0x7472_7565) => vec![0],
        _ => Vec::new(),
    };
    let mut out = Vec::new();
    for (index, offset) in (0_u32..).zip(offsets) {
        if let Some((family, weight)) = header(&mut file, u64::from(offset)) {
            out.push(SystemFont {
                family,
                weight,
                path: path.to_path_buf(),
                index,
            });
        }
    }
    out
}

/// The family and weight of the font whose table directory is at `offset`; `None` for CFF
/// outlines or broken headers.
fn header(file: &mut File, offset: u64) -> Option<(String, u16)> {
    let directory = read_at(file, offset, 12)?;
    if !matches!(u32_at(&directory, 0)?, 0x0001_0000 | 0x7472_7565) {
        return None;
    }
    let tables = usize::from(u16_at(&directory, 4)?).min(64);
    let records = read_at(file, offset + 12, 16 * tables)?;
    let mut name = None;
    let mut weight = 400;
    let mut has_glyf = false;
    for i in 0..tables {
        let record = &records[16 * i..16 * i + 16];
        let tag = &record[..4];
        let at = u64::from(u32_at(record, 8)?);
        let len = u32_at(record, 12)?.min(1 << 20) as usize;
        match tag {
            b"name" => name = read_at(file, at, len),
            b"OS/2" => {
                weight = read_at(file, at + 4, 2)
                    .and_then(|bytes| u16_at(&bytes, 0))
                    .filter(|w| (1..=1000).contains(w))
                    .unwrap_or(400);
            }
            b"glyf" => has_glyf = true,
            _ => {}
        }
    }
    if !has_glyf {
        return None;
    }
    let family = crate::font::family_name(&name?)?;
    Some((family, weight))
}

fn read_at(file: &mut File, offset: u64, len: usize) -> Option<Vec<u8>> {
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut bytes = vec![0; len];
    file.read_exact(&mut bytes).ok()?;
    Some(bytes)
}

/// Fonts systems ship for Korean and Chinese, by file name, in order of preference: what
/// [`crate::Fonts::add_system_fallbacks`] looks for.
pub const CJK_FILES: &[&str] = &[
    // Windows: Malgun Gothic, Microsoft YaHei, Microsoft JhengHei.
    "malgun.ttf",
    "msyh.ttc",
    "msjh.ttc",
    // macOS and iOS.
    "AppleSDGothicNeo.ttc",
    "PingFang.ttc",
    "STHeiti Light.ttc",
    // Linux: Nanum, WenQuanYi.
    "NanumGothic.ttf",
    "wqy-microhei.ttc",
    "wqy-zenhei.ttc",
];

/// The Korean and Chinese system fonts of [`CJK_FILES`] found in [`directories`].
pub fn cjk_fallbacks() -> Vec<PathBuf> {
    let mut found = Vec::new();
    for name in CJK_FILES {
        for directory in directories() {
            let mut files = Vec::new();
            collect(&directory, 0, &mut files);
            if let Some(path) = files.into_iter().find(|path| {
                path.file_name()
                    .and_then(|file| file.to_str())
                    .is_some_and(|file| file.eq_ignore_ascii_case(name))
            }) {
                found.push(path);
                break;
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_platform_has_font_directories() {
        assert_ne!(directories().len(), 0);
    }

    #[test]
    fn a_scan_reads_families_and_loads_them() {
        // Whatever the machine has: every listed font loads, and is the family it says.
        for font in scan().into_iter().take(3) {
            let loaded = font.load().expect("a listed font loads");
            assert_eq!(loaded.family(), font.family);
        }
    }

    #[test]
    fn files_that_are_not_fonts_list_nothing() {
        let dir = std::env::temp_dir().join("koebi-system-fonts");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("not-a-font.ttf");
        std::fs::write(&path, b"this is not a font").unwrap();
        assert_eq!(fonts_in(&path), []);
        let embedded = dir.join("noto.ttf");
        std::fs::write(&embedded, crate::fonts::embedded::NOTO_SANS_REGULAR).unwrap();
        let fonts = fonts_in(&embedded);
        assert_eq!(fonts.len(), 1);
        assert_eq!(
            (fonts[0].family.as_str(), fonts[0].weight),
            ("Noto Sans", 400)
        );
    }
}
