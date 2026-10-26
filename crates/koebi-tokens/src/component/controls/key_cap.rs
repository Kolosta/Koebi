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
    /// Key cap: vertical padding.
    KEY_CAP_PADDING_VERTICAL_DEFAULT = "key-cap.padding.vertical.default": Dimension = px(4.0), range(0.0, 32.0);
    /// Key cap: font scale.
    KEY_CAP_FONT_SCALE_DEFAULT = "key-cap.font-scale.default": Number = 0.85, range(0.5, 2.0);
}
