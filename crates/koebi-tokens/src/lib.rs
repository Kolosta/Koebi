//! Koebi's design tokens and built-in themes.
//!
//! - [`primitive`]: palettes and scales, the raw values (`primitive::COLOR_BLUE_700`).
//! - [`semantic`]: intents aliasing primitives or other semantic tokens
//!   (`semantic::BACKGROUND_ACCENT_DEFAULT`), and typography composites
//!   (`semantic::TEXT_BODY_M`).
//! - [`component`]: the properties of each component, aliasing semantic tokens or holding
//!   their own value (`component::BUTTON_BACKGROUND_COLOR_DEFAULT`).
//! - [`themes`]: the themes besides the base theme (Dark): Light, Muted green, High contrast.
//!
//! [`KOEBI`] puts everything together; it is checked while compiling.
//!
//! ```
//! use koebi_design::DesignSystem;
//! use koebi_tokens::{KOEBI, component};
//!
//! let design = DesignSystem::new(&KOEBI)?;
//! let background = design.get(component::BUTTON_BACKGROUND_COLOR_DEFAULT);
//! assert_eq!(background.to_string(), "#5d89ff");
//! # Ok::<(), koebi_design::Error>(())
//! ```
//!
//! The names and values come from the token table of the previous application; the porting
//! report (`docs/design-system/porting.md`) lists every difference.

#![allow(
    clippy::wildcard_imports,
    reason = "token tables are data: each imports the value constructors and the tokens of its tier"
)]

pub mod component;
pub mod primitive;
pub mod semantic;
pub mod themes;

use koebi_design::schema::{Schema, ThemeInfo};

/// Koebi's token schema: every token, the base theme and the built-in themes.
pub static KOEBI: Schema = SCHEMA;

const SCHEMA: Schema = Schema::new(
    "koebi",
    ThemeInfo::new(
        "dark",
        "Dark",
        "Default theme: light content on dark gray surfaces.",
        &[],
    ),
    &[
        primitive::PALETTE::GROUP,
        primitive::NEUTRAL::GROUP,
        primitive::SCALE::GROUP,
        primitive::LAYOUT::GROUP,
        primitive::TYPOGRAPHY::GROUP,
        primitive::MOTION::GROUP,
        semantic::ROLE::GROUP,
        semantic::BACKGROUND::GROUP,
        semantic::CONTENT::GROUP,
        semantic::DATA_VIZ::GROUP,
        semantic::SCALE::GROUP,
        semantic::TYPOGRAPHY::GROUP,
        semantic::MOTION::GROUP,
        semantic::TEXT::GROUP,
        component::CONTROLS::GROUP,
        component::SURFACES::GROUP,
        component::EDITOR::GROUP,
    ],
    &[themes::LIGHT, themes::MUTED_GREEN, themes::HIGH_CONTRAST],
);

const _: () = SCHEMA.assert_valid();
