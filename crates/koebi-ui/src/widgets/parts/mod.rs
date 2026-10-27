//! The parts components are built from: visual states ([`ControlState`], [`States`]), colors
//! by role and family ([`Tone`]), content ([`Content`]), where floating panels open
//! ([`Placement`]), and animations along token curves (`Tween`).

mod content;
mod placement;
mod states;
mod tone;
mod tween;

pub use content::Content;
pub(crate) use content::{ContentColors, ContentMetrics};
pub use placement::{PanelAlign, Placed, Placement, Side};
pub use states::{ControlState, States};
pub use tone::{Hue, HueColors, Shades, Tone, ToneColors, Variant};
pub(crate) use tone::{hued, shade};
pub(crate) use tween::Tween;
