//! Key caps, which show keyboard keys.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group KEY_CAP: Component, start = CAPTURE_FIELD::END;

    /// Key cap: background color.
    KEY_CAP_BACKGROUND_COLOR_DEFAULT = "key-cap.background.color.default": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Key cap: border color.
    KEY_CAP_BORDER_COLOR_DEFAULT = "key-cap.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Key cap: label color.
    KEY_CAP_LABEL_COLOR_DEFAULT = "key-cap.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Key cap: corner radius.
    KEY_CAP_CORNER_RADIUS_DEFAULT = "key-cap.corner-radius.default": Dimension ->
        semantic::RADIUS_XS;
    /// Key cap: horizontal padding.
    KEY_CAP_PADDING_HORIZONTAL_DEFAULT = "key-cap.padding.horizontal.default": Dimension = px(4.0), range(0.0, 32.0);
    /// Key cap: space above and below a cap in the row it lies in.
    KEY_CAP_PADDING_VERTICAL_DEFAULT = "key-cap.padding.vertical.default": Dimension ->
        semantic::SPACING_CONTROL_INSET;
    /// Key cap: font scale.
    KEY_CAP_FONT_SCALE_DEFAULT = "key-cap.font-scale.default": Number = 0.85, range(0.5, 2.0);
    /// Key cap: border width.
    KEY_CAP_BORDER_WIDTH_DEFAULT = "key-cap.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Key cap: color of the `+` between caps and of what follows them.
    KEY_CAP_SEPARATOR_COLOR_DEFAULT = "key-cap.separator.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Key cap: space around the `+` between caps, and between an icon and its label.
    KEY_CAP_GAP_DEFAULT = "key-cap.gap.default": Dimension = px(3.0), range(0.0, 16.0);
    /// Key cap: side of the icons in caps.
    KEY_CAP_ICON_SIZE_DEFAULT = "key-cap.icon.size.default": Dimension = px(12.0),
        range(6.0, 32.0);
}
