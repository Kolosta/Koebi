//! Line icons.

use super::*;
use koebi_design::define::*;

koebi_design::tokens! {
    group ICON: Component, start = ICON_BUTTON::END;

    /// Icon: size of line icons.
    ICON_SIZE_DEFAULT = "icon.size.default": Dimension = px(16.0), range(8.0, 64.0);
    /// Icon: stroke width of line icons (check marks excepted).
    ICON_STROKE_WIDTH_DEFAULT = "icon.stroke-width.default": Dimension = px(1.5), range(0.5, 4.0);
}
