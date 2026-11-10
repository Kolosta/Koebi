//! Built-in themes, one module each. Dark is the base theme: the token tables define it.
//! Every other theme lists what it changes: the values of its palettes, made for its own
//! background (`crate::palettes`), and the aliases where it uses other steps or other widths.

use koebi_design::schema::ThemeInfo;

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
