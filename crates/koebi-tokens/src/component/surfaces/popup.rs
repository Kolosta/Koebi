//! Popups.

use super::*;
use crate::semantic;

koebi_design::tokens! {
    group POPUP: Component, start = SPLASH::END;

    /// Popup: background color.
    POPUP_BACKGROUND_COLOR_DEFAULT = "popup.background.color.default": Color ->
        semantic::BACKGROUND_FLOATING_DEFAULT;
    /// Popup: corner radius.
    POPUP_CORNER_RADIUS_DEFAULT = "popup.corner-radius.default": Dimension -> semantic::RADIUS_M;
    /// Popup: border width.
    POPUP_BORDER_WIDTH_DEFAULT = "popup.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Popup: menu bar background color.
    POPUP_MENU_BAR_BACKGROUND_COLOR_DEFAULT = "popup.menu-bar.background.color.default": Color ->
        semantic::BACKGROUND_SUNKEN_DEFAULT;
}
