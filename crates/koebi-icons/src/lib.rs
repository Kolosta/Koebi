//! Koebi's icons: every SVG file of `icons/<category>/<name>.svg`, checked and compiled by the
//! build script (`koebi-svg`) into vector icons the renderer draws at any size.
//!
//! - Each icon is an [`IconId`] constant in the module of its category:
//!   [`navigation::CHEVRON_DOWN`] is `icons/navigation/chevron-down.svg`. Adding a file adds
//!   its constant; a file outside the SVG subset Koebi draws, or breaking the conventions of
//!   `docs/development/icons.md` (kebab-case names, a square view box), fails the build with
//!   its line.
//! - [`packed`] is the whole set laid out for the GPU, given once to the renderer
//!   (`Renderer2D::set_icons`); an icon is then drawn by its number, [`IconId::index`].
//! - [`IconId::roles`] tells what each of an icon's color zones stands for, so that the
//!   interface recolors them from tokens; [`IconId::compile`] gives the icon itself, for the
//!   CPU rasterizer (cursors, the application's icon).

use std::sync::OnceLock;

pub use koebi_svg::Role;
use koebi_svg::pack::Packed;

include!(concat!(env!("OUT_DIR"), "/icons.rs"));

/// The set packed by the build script: little-endian `f32`, four per texel.
static PACKED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icons.bin"));

/// An icon of the set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IconId(u32);

impl IconId {
    /// The icon's number in [`packed`]: what the renderer draws it by.
    pub const fn index(self) -> u32 {
        self.0
    }

    /// The icon numbered `index`, if the set holds one.
    pub fn from_index(index: u32) -> Option<Self> {
        (usize::try_from(index).is_ok_and(|index| index < COUNT)).then_some(Self(index))
    }

    /// Its path under `icons/`, without the extension: `navigation/chevron-down`.
    pub fn name(self) -> &'static str {
        NAMES[self.slot()]
    }

    /// The icon whose [`IconId::name`] is `name`.
    pub fn by_name(name: &str) -> Option<Self> {
        Self::all().find(|icon| icon.name() == name)
    }

    /// The category it belongs to: the folder of its file.
    pub fn category(self) -> &'static str {
        self.name()
            .split_once('/')
            .map_or("", |(category, _)| category)
    }

    /// What each of its color zones stands for, in the order of the zones.
    pub fn roles(self) -> &'static [Role] {
        ROLES[self.slot()]
    }

    /// Its SVG file.
    pub fn source(self) -> &'static str {
        SOURCES[self.slot()]
    }

    /// The icon compiled, for the CPU rasterizer ([`koebi_svg::raster`]).
    ///
    /// # Panics
    ///
    /// Never: the build script compiled every icon of the set.
    pub fn compile(self) -> koebi_svg::Icon {
        koebi_svg::compile(self.source()).expect("the build script compiled every icon")
    }

    /// Every icon, in order of number: by category, then by name.
    pub fn all() -> impl Iterator<Item = Self> {
        ALL.iter().copied()
    }

    fn slot(self) -> usize {
        // Every `IconId` comes from the generated constants or a checked index.
        self.0 as usize
    }
}

/// The whole set laid out for the GPU, in order of number: give it to the renderer once.
pub fn packed() -> &'static Packed {
    static SET: OnceLock<Packed> = OnceLock::new();
    SET.get_or_init(|| Packed {
        data: PACKED
            .as_chunks::<16>()
            .0
            .iter()
            .map(|texel| {
                let (values, _) = texel.as_chunks::<4>();
                core::array::from_fn(|i| f32::from_le_bytes(values[i]))
            })
            .collect(),
        icons: COUNT,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_names_and_numbers_agree() {
        assert_eq!(navigation::CHEVRON_DOWN.name(), "navigation/chevron-down");
        assert_eq!(navigation::CHEVRON_DOWN.category(), "navigation");
        assert_eq!(
            IconId::by_name("navigation/chevron-down"),
            Some(navigation::CHEVRON_DOWN)
        );
        assert_eq!(IconId::by_name("navigation/nothing"), None);
        assert_eq!(IconId::all().count(), COUNT);
        for (number, icon) in IconId::all().enumerate() {
            assert_eq!(icon.index() as usize, number);
            assert_eq!(IconId::from_index(icon.index()), Some(icon));
        }
        assert_eq!(IconId::from_index(u32::try_from(COUNT).unwrap()), None);
    }

    #[test]
    fn the_packed_set_holds_every_icon_compiled_again() {
        let set = packed();
        assert_eq!(set.icons, COUNT);
        let icons: Vec<koebi_svg::Icon> = IconId::all().map(IconId::compile).collect();
        let again = koebi_svg::pack::pack(&icons.iter().collect::<Vec<_>>());
        assert_eq!(*set, again);
        for (icon, compiled) in IconId::all().zip(&icons) {
            let roles: Vec<Role> = compiled.zones.iter().map(|zone| zone.role).collect();
            assert_eq!(icon.roles(), roles.as_slice(), "{}", icon.name());
        }
    }
}
