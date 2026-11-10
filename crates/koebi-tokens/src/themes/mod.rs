//! Built-in themes, one module each. Dark is the base theme: the token tables define it.
//! Every other theme lists what it changes: the values of its palettes, made for its own
//! background (`crate::palettes`), and the aliases where it uses other steps or other widths.

use koebi_design::schema::ThemeInfo;
use koebi_i18n::n_;

mod arctic;
mod dark;
mod dark_pink;
mod high_contrast;
mod muted_green;
mod snow;

pub use arctic::ARCTIC;
pub use dark::DARK;
pub use dark_pink::DARK_PINK;
pub use high_contrast::HIGH_CONTRAST;
pub use muted_green::MUTED_GREEN;
pub use snow::SNOW;

/// The themes besides the base theme, in the order the application lists them.
pub const BUILT_IN: &[ThemeInfo] = &[DARK_PINK, MUTED_GREEN, SNOW, ARCTIC, HIGH_CONTRAST];

/// The names of the built-in themes, by key, marked for translation (the `theme!` invocations
/// hold them as literals): the interface shows them with `koebi_i18n::tr`. A theme imported
/// from a file keeps the name it was given.
const NAMES: [(&str, &str); 6] = [
    ("dark", n_!("Dark")),
    ("dark-pink", n_!("Dark pink")),
    ("muted-green", n_!("Muted green")),
    ("snow", n_!("Snow")),
    ("arctic", n_!("Arctic")),
    ("high-contrast", n_!("High contrast")),
];

/// The English name of the built-in theme of `key`, to translate; `None` for another theme.
pub fn built_in_name(key: &str) -> Option<&'static str> {
    NAMES
        .iter()
        .find(|(candidate, _)| *candidate == key)
        .map(|(_, name)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_built_in_theme_has_its_name_marked() {
        for info in core::iter::once(&DARK).chain(BUILT_IN) {
            assert_eq!(
                built_in_name(info.key()),
                Some(info.name()),
                "{}",
                info.key()
            );
        }
        assert_eq!(built_in_name("imported"), None);
    }
}
