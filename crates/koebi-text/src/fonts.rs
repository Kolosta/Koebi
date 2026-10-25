//! The fonts an application uses, by family and weight, and the fonts Koebi embeds.

use std::fmt;
use std::sync::Arc;

use crate::font::Font;

/// Fonts embedded in the binary: Noto Sans, under the SIL Open Font License 1.1
/// (`crates/koebi-text/fonts/OFL.txt`).
pub mod embedded {
    /// The family name the design tokens use for Noto Sans (`font.family.sans`).
    pub const NOTO_SANS: &str = "NotoSans";
    /// Noto Sans Regular (weight 400): body text.
    pub const NOTO_SANS_REGULAR: &[u8] = include_bytes!("../fonts/NotoSans-Regular.ttf");
    /// Noto Sans Medium (weight 500): labels.
    pub const NOTO_SANS_MEDIUM: &[u8] = include_bytes!("../fonts/NotoSans-Medium.ttf");
}

/// A font of a [`Fonts`] collection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontId(u16);

impl FontId {
    /// The position of the font in its collection.
    pub fn index(self) -> usize {
        usize::from(self.0)
    }
}

#[derive(Clone)]
struct Entry {
    /// Family name, folded for matching (`notosans`).
    family: String,
    font: Arc<Font>,
}

/// The fonts available to text, by family and weight. Cheap to clone and append-only, so a
/// [`FontId`] stays valid in every clone made after the font was added.
#[derive(Clone, Default)]
pub struct Fonts {
    entries: Arc<Vec<Entry>>,
}

impl Fonts {
    /// An empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// The fonts Koebi embeds ([`embedded`]): Noto Sans Regular and Medium.
    pub fn with_embedded() -> Self {
        let mut fonts = Self::new();
        for data in [embedded::NOTO_SANS_REGULAR, embedded::NOTO_SANS_MEDIUM] {
            match Font::from_static(data) {
                Ok(font) => {
                    fonts.add(embedded::NOTO_SANS, font);
                }
                // Tests read every embedded font: this cannot happen in a tested build.
                Err(error) => koebi_core::error!("embedded font unreadable: {error}"),
            }
        }
        fonts
    }

    /// Adds `font` to `family` and returns its identifier.
    pub fn add(&mut self, family: &str, font: Font) -> FontId {
        let entries = Arc::make_mut(&mut self.entries);
        let id = FontId(u16::try_from(entries.len()).unwrap_or(u16::MAX));
        entries.push(Entry {
            family: fold(family),
            font: Arc::new(font),
        });
        id
    }

    /// The font `id`, if it belongs to this collection.
    pub fn get(&self, id: FontId) -> Option<&Font> {
        self.entries.get(id.index()).map(|entry| &*entry.font)
    }

    /// The font of `family` closest to `weight`, as CSS matches weights: below 400, lighter
    /// fonts first; from 400 to 500, up to 500 first, then lighter; above 500, heavier fonts
    /// first. Family names match ignoring case, spaces, hyphens and underscores.
    pub fn select(&self, family: &str, weight: u16) -> Option<FontId> {
        // Compared without allocating: components select their font every frame.
        (0_u16..)
            .zip(self.entries.iter())
            .filter(|(_, entry)| folded(family).eq(entry.family.chars()))
            .min_by_key(|(_, entry)| preference(weight, entry.font.weight()))
            .map(|(index, _)| FontId(index))
    }

    /// Number of fonts.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` without any font.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl fmt::Debug for Fonts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(
                self.entries
                    .iter()
                    .map(|entry| (&entry.family, entry.font.weight())),
            )
            .finish()
    }
}

/// The characters of a family name folded for matching: lowercase, without separators.
fn folded(family: &str) -> impl Iterator<Item = char> + '_ {
    family
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | '_'))
        .flat_map(char::to_lowercase)
}

/// A family name folded for matching.
fn fold(family: &str) -> String {
    folded(family).collect()
}

/// How well `weight` serves a request for `desired`: lower is better (CSS Fonts 4, 5.2).
fn preference(desired: u16, weight: u16) -> (u8, u16) {
    let distance = desired.abs_diff(weight);
    if weight == desired {
        (0, 0)
    } else if (400..=500).contains(&desired) {
        if weight > desired && weight <= 500 {
            (1, distance)
        } else if weight < desired {
            (2, distance)
        } else {
            (3, distance)
        }
    } else if (desired < 400) == (weight < desired) {
        (1, distance)
    } else {
        (2, distance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn best(desired: u16, available: &[u16]) -> u16 {
        available
            .iter()
            .copied()
            .min_by_key(|weight| preference(desired, *weight))
            .unwrap()
    }

    #[test]
    fn weights_match_like_css() {
        let available = [300, 400, 500, 700];
        assert_eq!(best(400, &available), 400);
        // From 400 to 500: up to 500 first, then lighter, then heavier.
        assert_eq!(best(450, &available), 500);
        assert_eq!(best(450, &[300, 700]), 300);
        assert_eq!(best(500, &[400, 700]), 400);
        // Below 400: lighter first; above 500: heavier first.
        assert_eq!(best(350, &available), 300);
        assert_eq!(best(200, &[400, 500]), 400);
        assert_eq!(best(600, &available), 700);
        assert_eq!(best(800, &available), 700);
    }

    #[test]
    fn family_names_fold_case_and_separators() {
        assert_eq!(fold("Noto Sans"), "notosans");
        assert_eq!(fold("noto-sans"), fold("NotoSans"));
        assert_eq!(fold("Noto_Sans_Mono"), "notosansmono");
    }
}
