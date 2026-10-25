//! Koebi's design system engine: typed design tokens, themes, overrides and color vision.
//!
//! # Tokens
//!
//! A design token is a named design decision. Tokens live in three tiers:
//!
//! - **primitive** tokens hold values: palettes and scales (`color.blue.700`, `radius.200`);
//! - **semantic** tokens name an intent and alias a primitive or another semantic token
//!   (`background.accent.default` → `accent.color.700` → `color.blue.700`);
//! - **component** tokens belong to one component: they alias a semantic or another component
//!   token (`button.background.color.default` → `background.accent.default`), or hold a value
//!   of their own (`scrollbar.size.default` = 14 px).
//!
//! Every token is a Rust constant of type [`Token<Kind, Tier>`](Token), defined in a token
//! table ([`tokens!`]) with its path, description and definition. The kind (color, dimension,
//! duration...) and the tier are part of the type, so a component style that expects a color
//! cannot receive a dimension, and a component token cannot alias a primitive: both fail to
//! compile. Paths, constant names and the absence of alias cycles are checked at compile time
//! too ([`schema::Schema::assert_valid`]).
//!
//! # Themes, overrides, views
//!
//! The base theme defines every token; a theme ([`theme!`], or created at run time) only
//! lists what it changes. On top, the user overrides tokens for one theme or globally
//! ([`Scope`]). [`DesignSystem::get`] reads the active theme with every layer; a [`View`]
//! reads any theme with any [`Layers`]: original values, theme overrides only, other themes
//! for preview cards. A per-token layer mask opts a token out of overrides or of color vision.
//!
//! # Color vision
//!
//! A color-only matrix ([`ColorVision`]) corrects or simulates color vision deficiencies,
//! applied to color tokens when they are resolved.
//!
//! # Performance
//!
//! The active theme is materialized: reading a token is an array access. Changes propagate
//! incrementally to the tokens that depend on the changed one, so a slider can edit a token
//! every frame. Token tables are compiled Rust; nothing is parsed at run time.
//!
//! # Example
//!
//! ```
//! mod tokens {
//!     use koebi_design::define::*;
//!     koebi_design::tokens! {
//!         group PALETTE: Primitive, start = 0;
//!         /// Blue, palette step 700.
//!         COLOR_BLUE_700 = "color.blue.700": Color = hex("#5D89FF");
//!         /// Green, palette step 700.
//!         COLOR_GREEN_700 = "color.green.700": Color = hex("#139E5C");
//!     }
//!     koebi_design::tokens! {
//!         group INTENTS: Semantic, start = PALETTE::END;
//!         /// Background of accent elements.
//!         BACKGROUND_ACCENT_DEFAULT = "background.accent.default": Color -> COLOR_BLUE_700;
//!     }
//!     koebi_design::theme! {
//!         /// The same, with a green accent.
//!         GREEN = "green", "Green" {
//!             BACKGROUND_ACCENT_DEFAULT => COLOR_GREEN_700;
//!         }
//!     }
//!     pub const SCHEMA: koebi_design::schema::Schema = koebi_design::schema::Schema::new(
//!         "example",
//!         koebi_design::schema::ThemeInfo::new("blue", "Blue", "Blue accent.", &[]),
//!         &[PALETTE::GROUP, INTENTS::GROUP],
//!         &[GREEN],
//!     );
//!     const _: () = SCHEMA.assert_valid();
//! }
//!
//! use koebi_design::{DesignSystem, Scope};
//! use koebi_core::Color;
//!
//! static SCHEMA: koebi_design::schema::Schema = tokens::SCHEMA;
//!
//! fn main() -> Result<(), koebi_design::Error> {
//!     let mut design = DesignSystem::new(&SCHEMA)?;
//!     assert_eq!(design.get(tokens::BACKGROUND_ACCENT_DEFAULT), Color::from_hex(0x5D_89_FF));
//!
//!     let green = design.find_theme("green").expect("theme of the schema");
//!     design.set_active_theme(green)?;
//!     assert_eq!(design.get(tokens::BACKGROUND_ACCENT_DEFAULT), Color::from_hex(0x13_9E_5C));
//!
//!     // A global override changes every token aliasing the primitive, in every theme.
//!     design.set_value(Scope::Global, tokens::COLOR_GREEN_700, Color::WHITE)?;
//!     assert_eq!(design.get(tokens::BACKGROUND_ACCENT_DEFAULT), Color::WHITE);
//!     Ok(())
//! }
//! ```

pub mod define;
pub mod dtcg;
pub mod kind;
pub mod persist;
pub mod schema;
pub mod tier;
pub mod value;

mod active;
mod check;
mod error;
mod graph;
mod json;
mod layers;
mod macros;
mod system;
mod theme;
mod token;
mod view;
mod vision;

pub use error::{Error, Result};
pub use koebi_core::color_vision::{ColorMatrix, Deficiency};
pub use layers::{Layers, Origin};
pub use system::{DesignSystem, Scope};
pub use theme::{NewTheme, Theme, ThemeId, ThemeOrigin};
pub use token::{TextStyle, Token, TokenId, Typography};
pub use view::View;
pub use vision::ColorVision;
