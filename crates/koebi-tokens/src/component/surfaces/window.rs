//! Windows.

use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group WINDOW: Component, start = crate::component::THEME_CARD::END;

    /// Window: background color.
    WINDOW_BACKGROUND_COLOR_DEFAULT = "window.background.color.default": Color ->
        semantic::BACKGROUND_BASE_DEFAULT, opaque();
    /// Window: corner radius.
    WINDOW_CORNER_RADIUS_DEFAULT = "window.corner-radius.default": Dimension -> semantic::RADIUS_M;
    /// Window: border width.
    WINDOW_BORDER_WIDTH_DEFAULT = "window.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Window: border color.
    WINDOW_BORDER_COLOR_DEFAULT = "window.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Window: horizontal padding.
    WINDOW_PADDING_HORIZONTAL_DEFAULT = "window.padding.horizontal.default": Dimension = px(8.0), range(0.0, 32.0);
    /// Window: vertical padding.
    WINDOW_PADDING_VERTICAL_DEFAULT = "window.padding.vertical.default": Dimension = px(4.0), range(0.0, 32.0);
    /// Window: shadow color.
    WINDOW_SHADOW_COLOR_DEFAULT = "window.shadow.color.default": Color = hex("#00000040");
    /// Window: shadow size.
    WINDOW_SHADOW_SIZE_DEFAULT = "window.shadow.size.default": Dimension =
        px(18.0), range(0.0, 64.0);
}
