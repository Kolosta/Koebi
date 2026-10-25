//! Component tokens: the properties of each component, aliasing a semantic or another
//! component token, or holding a value of their own (a component-specific size).
//!
//! One file per component, in a folder per category:
//! `component/controls/button.rs` holds the `component.button.*` tokens.

mod controls;
mod editors;
mod surfaces;

pub use controls::*;
pub use editors::*;
pub use surfaces::*;
