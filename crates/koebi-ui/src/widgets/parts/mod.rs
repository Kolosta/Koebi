//! The parts components are built from: visual states ([`ControlState`], [`States`]), colors
//! by role and family ([`Tone`]), content ([`Content`]), and where floating panels open
//! ([`Placement`]).

mod content;
mod placement;
mod states;
mod tone;

pub use content::Content;
pub(crate) use content::{ContentColors, ContentMetrics};
pub use placement::{Alignment, Placed, Placement, Side};
pub use states::{ControlState, States};
pub use tone::{Hue, HueColors, Shades, Tone, ToneColors, Variant};
pub(crate) use tone::{hued, shade};
