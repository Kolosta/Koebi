//! Icons: the size and stroke of line icons, and how vector icons recolor their zones.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group ICON: Component, start = ICON_BUTTON::END;

    /// Icon: space between an icon and the edges of the interface unit it sits in, on each side:
    /// icons are the unit (`semantic.size.control-height`) less twice this.
    ICON_SIZE_MARGIN = "icon.size.margin": Dimension = px(1.0), range(0.0, 8.0);
    /// Icon: opacity of the secondary zones of a vector icon drawn in one color, which its
    /// primary zones take whole.
    ICON_ZONE_SECONDARY_OPACITY = "icon.zone.secondary.opacity": Number ->
        semantic::OPACITY_MODERATE;
    /// Icon: opacity of the tertiary zones of a vector icon drawn in one color.
    ICON_ZONE_TERTIARY_OPACITY = "icon.zone.tertiary.opacity": Number -> semantic::OPACITY_REDUCED;
    /// Icon: color of the accent zones of a vector icon drawn in one color.
    ICON_ZONE_ACCENT_COLOR = "icon.zone.accent.color": Color -> semantic::ICON_COLOR_ACCENT_DEFAULT;
}
