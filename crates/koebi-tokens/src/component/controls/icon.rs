//! Icons: the size and stroke of line icons, and how vector icons recolor their zones.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group ICON: Component, start = ICON_BUTTON::END;

    /// Icon: size of line icons.
    ICON_SIZE_DEFAULT = "icon.size.default": Dimension = px(16.0), range(8.0, 64.0);
    /// Icon: stroke width of line icons (check marks and the chevrons that fold excepted).
    ICON_STROKE_WIDTH_DEFAULT = "icon.stroke-width.default": Dimension = px(1.5), range(0.5, 4.0);
    /// Icon: stroke width of the chevrons that fold and open (fold arrows, panel headers,
    /// dropdowns), a little finer than other icons, which are larger.
    ICON_STROKE_WIDTH_CHEVRON = "icon.stroke-width.chevron": Dimension = px(1.25),
        range(0.5, 4.0);
    /// Icon: opacity of the secondary zones of a vector icon drawn in one color, which its
    /// primary zones take whole.
    ICON_ZONE_SECONDARY_OPACITY = "icon.zone.secondary.opacity": Number ->
        semantic::OPACITY_MODERATE;
    /// Icon: opacity of the tertiary zones of a vector icon drawn in one color.
    ICON_ZONE_TERTIARY_OPACITY = "icon.zone.tertiary.opacity": Number -> semantic::OPACITY_REDUCED;
    /// Icon: color of the accent zones of a vector icon drawn in one color.
    ICON_ZONE_ACCENT_COLOR = "icon.zone.accent.color": Color -> semantic::ICON_COLOR_ACCENT_DEFAULT;
}
