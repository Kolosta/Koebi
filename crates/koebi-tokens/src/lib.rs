//! Koebi's design tokens and built-in themes.
//!
//! - [`primitive`]: palettes and scales, the raw values (`primitive::COLOR_BLUE_700`).
//! - [`semantic`]: intents aliasing primitives or other semantic tokens
//!   (`semantic::BACKGROUND_ACCENT_DEFAULT`), and typography composites
//!   (`semantic::TEXT_BODY_M`).
//! - [`component`]: the properties of each component, aliasing semantic tokens or holding
//!   their own value (`component::BUTTON_BACKGROUND_COLOR_DEFAULT`).
//! - [`themes`]: the built-in themes, one module each: Dark (the base theme), Light,
//!   Muted green and High contrast.
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

use koebi_design::schema::Schema;

/// Koebi's token schema: every token, the base theme and the built-in themes.
pub static KOEBI: Schema = SCHEMA;

const SCHEMA: Schema = Schema::new(
    "koebi",
    themes::DARK,
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
        semantic::INPUT::GROUP,
        semantic::TEXT::GROUP,
        // component::controls
        component::BUTTON::GROUP,
        component::ICON_BUTTON::GROUP,
        component::ICON::GROUP,
        component::TOGGLE::GROUP,
        component::CHECKBOX::GROUP,
        component::SLIDER::GROUP,
        component::DRAG_VALUE::GROUP,
        component::DROPDOWN::GROUP,
        component::COMBO::GROUP,
        component::FRAME::GROUP,
        component::CAPTURE_FIELD::GROUP,
        component::KEY_CAP::GROUP,
        component::SEPARATOR::GROUP,
        component::SCROLLBAR::GROUP,
        component::TAB::GROUP,
        component::HEADER::GROUP,
        component::RESIZE_GRIP::GROUP,
        component::COLOR_SWATCH::GROUP,
        component::THEME_CARD::GROUP,
        // component::surfaces
        component::WINDOW::GROUP,
        component::TITLE_BAR::GROUP,
        component::PREFERENCES_TITLE_BAR::GROUP,
        component::SPLASH::GROUP,
        component::POPUP::GROUP,
        component::CHILD::GROUP,
        component::PANEL::GROUP,
        component::SIDE_PANEL::GROUP,
        component::PROPERTY_GROUP::GROUP,
        component::MENU::GROUP,
        component::TOOLTIP::GROUP,
        component::STATUS_BAR::GROUP,
        component::SECTION_HEADER::GROUP,
        component::SHORTCUT_ROW::GROUP,
        component::SHORTCUT::GROUP,
        component::TABLE::GROUP,
        component::IMAGE::GROUP,
        component::DATA_VIZ::GROUP,
        component::CURSOR::GROUP,
        // component::editors
        component::EDITOR::GROUP,
        component::EDITOR_TOP_BAR::GROUP,
        component::ZONE::GROUP,
        component::ZONE_TAB::GROUP,
        component::ZONE_OVERLAY::GROUP,
        component::DROP_TARGET::GROUP,
        component::TOOL_PALETTE::GROUP,
        component::VIEWPORT::GROUP,
        component::EDIT_HANDLE::GROUP,
        component::OUTLINER::GROUP,
        component::LIST_ROW::GROUP,
        component::NODE_GRAPH::GROUP,
        component::NODE_BOX::GROUP,
        component::NODE_PORT::GROUP,
        component::NODE_CABLE::GROUP,
        component::TIMELINE::GROUP,
    ],
    themes::BUILT_IN,
);

const _: () = SCHEMA.assert_valid();
