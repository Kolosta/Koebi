//! Primitive tokens: the raw values of the design system, palettes and scales.
//!
//! Components never read primitive tokens: semantic and component tokens alias them, so that a
//! theme or an override can change a value in one place.

mod layout;
mod motion;
mod neutral;
mod palette;
mod scale;
mod typography;

pub use layout::*;
pub use motion::*;
pub use neutral::*;
pub use palette::*;
pub use scale::*;
pub use typography::*;
