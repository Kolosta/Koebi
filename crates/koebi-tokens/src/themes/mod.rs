//! Built-in themes, one module each. Dark is the base theme: the token tables define it.
//! Every other theme lists the aliases it changes; all other tokens keep their base
//! definition.

use koebi_design::schema::ThemeInfo;

mod dark;
mod high_contrast;
mod light;
mod muted_green;

pub use dark::DARK;
pub use high_contrast::HIGH_CONTRAST;
pub use light::LIGHT;
pub use muted_green::MUTED_GREEN;

/// The themes besides the base theme, in the order the application lists them.
pub const BUILT_IN: &[ThemeInfo] = &[LIGHT, MUTED_GREEN, HIGH_CONTRAST];
