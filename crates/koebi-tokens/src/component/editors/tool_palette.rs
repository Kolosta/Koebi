//! Tool palettes.

use super::*;
use koebi_design::define::*;

koebi_design::tokens! {
    group TOOL_PALETTE: Component, start = DROP_TARGET::END;

    /// Tool palette: button size.
    TOOL_PALETTE_BUTTON_SIZE = "tool-palette.button.size": Dimension = px(32.0), range(16.0, 64.0);
    /// Tool palette: icon scale.
    TOOL_PALETTE_ICON_SCALE = "tool-palette.icon.scale": Number = 0.68, range(0.3, 1.0);
}
