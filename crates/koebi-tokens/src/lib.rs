//! Koebi's design tokens and built-in themes.
//!
//! - [`primitive`]: palettes and scales, the raw values (`primitive::COLOR_BLUE_700`).
//! - [`semantic`]: intents aliasing primitives or other semantic tokens
//!   (`semantic::BACKGROUND_ACCENT_DEFAULT`), and typography composites
//!   (`semantic::TEXT_BODY_M`).
//! - [`component`]: the properties of each component, aliasing semantic tokens or holding
//!   their own value (`component::BUTTON_BACKGROUND_COLOR_DEFAULT`).
//! - [`themes`]: the built-in themes, one module each: Dark (the base theme), Dark pink,
//!   Muted green, Snow, Arctic and High contrast.
//! - [`palettes`]: what the themes' palettes are made of, and how a palette is made from one
//!   color.
//!
//! [`KOEBI`] puts everything together; it is checked while compiling.
//!
//! ```
//! use koebi_design::DesignSystem;
//! use koebi_tokens::{KOEBI, component};
//!
//! let design = DesignSystem::new(&KOEBI)?;
//! let background = design.get(component::BUTTON_ACCENT_BACKGROUND_COLOR_DEFAULT);
//! assert_eq!(background.to_string(), "#3d65fc");
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
pub mod palettes;
pub mod primitive;
pub mod semantic;
pub mod themes;

use koebi_design::schema::{Group, Schema};
use koebi_i18n::n_;

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
        component::NUMBER_FIELD::GROUP,
        component::TEXT_FIELD::GROUP,
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
        component::COLOR_FIELD::GROUP,
        component::COLOR_PICKER::GROUP,
        component::CURVE_EDITOR::GROUP,
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

/// A part of the interface and the token groups that style it: how settings show the tokens
/// (Settings > Customisation, by feature).
#[derive(Clone, Copy, Debug)]
pub struct Feature {
    /// Its name, in English: shown with `koebi_i18n::tr`.
    pub title: &'static str,
    /// What its tokens decide, in a sentence, in English.
    pub description: &'static str,
    /// Its groups, in schema order.
    pub groups: &'static [Group],
}

/// Every group of [`KOEBI`] by feature, each group once: the components' (controls, surfaces,
/// editors), then the intents they alias, then the palettes and scales beneath.
pub static FEATURES: [Feature; 5] = [
    Feature {
        title: n_!("Controls"),
        description: n_!("Buttons, toggles, fields, dropdowns, pickers and the other controls."),
        groups: &[
            component::BUTTON::GROUP,
            component::ICON_BUTTON::GROUP,
            component::ICON::GROUP,
            component::TOGGLE::GROUP,
            component::CHECKBOX::GROUP,
            component::NUMBER_FIELD::GROUP,
            component::TEXT_FIELD::GROUP,
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
            component::COLOR_FIELD::GROUP,
            component::COLOR_PICKER::GROUP,
            component::CURVE_EDITOR::GROUP,
        ],
    },
    Feature {
        title: n_!("Surfaces"),
        description: n_!("Windows, panels, menus, tooltips, the status bar and what they hold."),
        groups: &[
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
        ],
    },
    Feature {
        title: n_!("Editors"),
        description: n_!("The workspace's areas and tabs, and the editors inside them."),
        groups: &[
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
    },
    Feature {
        title: n_!("Intents"),
        description: n_!(
            "Semantic tokens: the roles, backgrounds, text, sizes and motion components alias."
        ),
        groups: &[
            semantic::ROLE::GROUP,
            semantic::BACKGROUND::GROUP,
            semantic::CONTENT::GROUP,
            semantic::DATA_VIZ::GROUP,
            semantic::SCALE::GROUP,
            semantic::TYPOGRAPHY::GROUP,
            semantic::MOTION::GROUP,
            semantic::INPUT::GROUP,
            semantic::TEXT::GROUP,
        ],
    },
    Feature {
        title: n_!("Palettes and scales"),
        description: n_!(
            "Primitive tokens: the raw colors, sizes, fonts and durations beneath everything."
        ),
        groups: &[
            primitive::PALETTE::GROUP,
            primitive::NEUTRAL::GROUP,
            primitive::SCALE::GROUP,
            primitive::LAYOUT::GROUP,
            primitive::TYPOGRAPHY::GROUP,
            primitive::MOTION::GROUP,
        ],
    },
];
