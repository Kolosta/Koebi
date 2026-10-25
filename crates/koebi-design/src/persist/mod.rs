//! Persistence: the user's changes survive restarts, on every platform.
//!
//! Only what is not part of the schema is saved: themes created at run time, the overrides of
//! every theme, global overrides, layer masks, the color vision setting and the active theme.
//! The token tables and built-in themes come from the code, so a new version of the schema
//! applies to existing users without migration.
//!
//! [`Store`] keeps this state in a directory (the platform's preferences directory in the
//! application); [`encode`] and [`restore`] work on bytes. The format is a small versioned
//! binary file with a checksum (see the `format` module source); ADR 0009 explains the choices.

mod codec;
mod format;
mod store;

pub use format::{RestoreReport, encode, restore};
pub use store::Store;
