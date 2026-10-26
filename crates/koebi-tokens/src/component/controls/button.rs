//! Push buttons: the default variant, the colored variants, color families, states and
//! metrics.
//!
//! `button.*` without a variant is the default variant, a neutral control. The colored variants
//! (`button.accent.*`, `button.positive.*`...) fill the button with their intent's color, and a
//! color family (`button.hue.<hue>.*`) can replace that color, its hovered and pressed shades
//! derived from it by the lightness changes of `button.shade.*.lightness`.

use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group BUTTON: Component, start = semantic::TEXT::END;

    /// Button: background color.
    BUTTON_BACKGROUND_COLOR_DEFAULT = "button.background.color.default": Color ->
        semantic::BACKGROUND_APP_FRAME;
    /// Button: background color, hovered.
    BUTTON_BACKGROUND_COLOR_HOVER = "button.background.color.hover": Color ->
        semantic::BACKGROUND_APP_FRAME_HOVER;
    /// Button: background color, pressed.
    BUTTON_BACKGROUND_COLOR_PRESSED = "button.background.color.pressed": Color ->
        semantic::BACKGROUND_APP_CHILD;
    /// Button: label color.
    BUTTON_LABEL_COLOR_DEFAULT = "button.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Button: border color.
    BUTTON_BORDER_COLOR_DEFAULT = "button.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Button: border width.
    BUTTON_BORDER_WIDTH_DEFAULT = "button.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_NONE;
    /// Button: corner radius.
    BUTTON_CORNER_RADIUS_DEFAULT = "button.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;

    /// Button: border color, hovered.
    BUTTON_BORDER_COLOR_HOVER = "button.border.color.hover": Color ->
        semantic::BORDER_COLOR_STRONG_DEFAULT;
    /// Button: border color, pressed.
    BUTTON_BORDER_COLOR_PRESSED = "button.border.color.pressed": Color ->
        semantic::BORDER_COLOR_STRONG_DEFAULT;
    /// Button: background color, selected (a toggle button switched on).
    BUTTON_BACKGROUND_COLOR_SELECTED = "button.background.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Button: background color, selected and hovered.
    BUTTON_BACKGROUND_COLOR_SELECTED_HOVER = "button.background.color.selected-hover": Color ->
        semantic::BACKGROUND_ACCENT_HOVER;
    /// Button: label color, selected.
    BUTTON_LABEL_COLOR_SELECTED = "button.label.color.selected": Color ->
        semantic::STATIC_TEXT_COLOR_WHITE;
    /// Button: border color, selected.
    BUTTON_BORDER_COLOR_SELECTED = "button.border.color.selected": Color ->
        semantic::BORDER_COLOR_ACCENT_DEFAULT;
    /// Button: background color, disabled.
    BUTTON_BACKGROUND_COLOR_DISABLED = "button.background.color.disabled": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Button: label color, disabled.
    BUTTON_LABEL_COLOR_DISABLED = "button.label.color.disabled": Color ->
        semantic::TEXT_COLOR_DISABLED_DEFAULT;
    /// Button: border color, disabled.
    BUTTON_BORDER_COLOR_DISABLED = "button.border.color.disabled": Color ->
        semantic::BORDER_COLOR_DISABLED_DEFAULT;

    /// Button: height, one interface unit.
    BUTTON_HEIGHT_DEFAULT = "button.height.default": Dimension -> semantic::SIZE_CONTROL_HEIGHT;
    /// Button: smallest width of a button fitting its content.
    BUTTON_MIN_WIDTH_DEFAULT = "button.min-width.default": Dimension ->
        semantic::SIZE_CONTROL_HEIGHT;
    /// Button: space between the sides and the content.
    BUTTON_PADDING_HORIZONTAL_DEFAULT = "button.padding.horizontal.default": Dimension =
        px(8.0), range(0.0, 32.0);
    /// Button: space between the parts of the content (icons, label, status dot).
    BUTTON_GAP_DEFAULT = "button.gap.default": Dimension = px(4.0), range(0.0, 24.0);
    /// Button: side of the status dot.
    BUTTON_STATUS_DOT_SIZE_DEFAULT = "button.status-dot.size.default": Dimension =
        px(6.0), range(2.0, 16.0);

    /// Accent button: background color.
    BUTTON_ACCENT_BACKGROUND_COLOR_DEFAULT = "button.accent.background.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Accent button: background color, hovered.
    BUTTON_ACCENT_BACKGROUND_COLOR_HOVER = "button.accent.background.color.hover": Color ->
        semantic::BACKGROUND_ACCENT_HOVER;
    /// Accent button: background color, pressed.
    BUTTON_ACCENT_BACKGROUND_COLOR_PRESSED = "button.accent.background.color.pressed": Color ->
        semantic::ACCENT_COLOR_800;
    /// Accent button: label color.
    BUTTON_ACCENT_LABEL_COLOR_DEFAULT = "button.accent.label.color.default": Color ->
        semantic::STATIC_TEXT_COLOR_WHITE;
    /// Accent button: border color.
    BUTTON_ACCENT_BORDER_COLOR_DEFAULT = "button.accent.border.color.default": Color ->
        semantic::BORDER_COLOR_ACCENT_DEFAULT;

    /// Positive button: background color.
    BUTTON_POSITIVE_BACKGROUND_COLOR_DEFAULT = "button.positive.background.color.default": Color ->
        semantic::BACKGROUND_POSITIVE_DEFAULT;
    /// Positive button: background color, hovered.
    BUTTON_POSITIVE_BACKGROUND_COLOR_HOVER = "button.positive.background.color.hover": Color ->
        semantic::BACKGROUND_POSITIVE_HOVER;
    /// Positive button: background color, pressed.
    BUTTON_POSITIVE_BACKGROUND_COLOR_PRESSED = "button.positive.background.color.pressed": Color ->
        semantic::POSITIVE_COLOR_800;
    /// Positive button: label color.
    BUTTON_POSITIVE_LABEL_COLOR_DEFAULT = "button.positive.label.color.default": Color ->
        semantic::STATIC_TEXT_COLOR_WHITE;
    /// Positive button: border color.
    BUTTON_POSITIVE_BORDER_COLOR_DEFAULT = "button.positive.border.color.default": Color ->
        semantic::POSITIVE_COLOR_500;

    /// Negative button: background color.
    BUTTON_NEGATIVE_BACKGROUND_COLOR_DEFAULT = "button.negative.background.color.default": Color ->
        semantic::BACKGROUND_NEGATIVE_DEFAULT;
    /// Negative button: background color, hovered.
    BUTTON_NEGATIVE_BACKGROUND_COLOR_HOVER = "button.negative.background.color.hover": Color ->
        semantic::BACKGROUND_NEGATIVE_HOVER;
    /// Negative button: background color, pressed.
    BUTTON_NEGATIVE_BACKGROUND_COLOR_PRESSED = "button.negative.background.color.pressed": Color ->
        semantic::NEGATIVE_COLOR_800;
    /// Negative button: label color.
    BUTTON_NEGATIVE_LABEL_COLOR_DEFAULT = "button.negative.label.color.default": Color ->
        semantic::STATIC_TEXT_COLOR_WHITE;
    /// Negative button: border color.
    BUTTON_NEGATIVE_BORDER_COLOR_DEFAULT = "button.negative.border.color.default": Color ->
        semantic::BORDER_COLOR_NEGATIVE_DEFAULT;

    /// Info button: background color.
    BUTTON_INFO_BACKGROUND_COLOR_DEFAULT = "button.info.background.color.default": Color ->
        semantic::BACKGROUND_INFO_DEFAULT;
    /// Info button: background color, hovered.
    BUTTON_INFO_BACKGROUND_COLOR_HOVER = "button.info.background.color.hover": Color ->
        semantic::BACKGROUND_INFO_HOVER;
    /// Info button: background color, pressed.
    BUTTON_INFO_BACKGROUND_COLOR_PRESSED = "button.info.background.color.pressed": Color ->
        semantic::INFO_COLOR_800;
    /// Info button: label color.
    BUTTON_INFO_LABEL_COLOR_DEFAULT = "button.info.label.color.default": Color ->
        semantic::STATIC_TEXT_COLOR_WHITE;
    /// Info button: border color.
    BUTTON_INFO_BORDER_COLOR_DEFAULT = "button.info.border.color.default": Color ->
        semantic::INFO_COLOR_500;

    /// Neutral button: background color.
    BUTTON_NEUTRAL_BACKGROUND_COLOR_DEFAULT = "button.neutral.background.color.default": Color ->
        semantic::BACKGROUND_NEUTRAL_DEFAULT;
    /// Neutral button: background color, hovered.
    BUTTON_NEUTRAL_BACKGROUND_COLOR_HOVER = "button.neutral.background.color.hover": Color ->
        semantic::BACKGROUND_NEUTRAL_HOVER;
    /// Neutral button: background color, pressed.
    BUTTON_NEUTRAL_BACKGROUND_COLOR_PRESSED = "button.neutral.background.color.pressed": Color ->
        semantic::NEUTRAL_COLOR_600;
    /// Neutral button: label color.
    BUTTON_NEUTRAL_LABEL_COLOR_DEFAULT = "button.neutral.label.color.default": Color ->
        semantic::STATIC_TEXT_COLOR_WHITE;
    /// Neutral button: border color.
    BUTTON_NEUTRAL_BORDER_COLOR_DEFAULT = "button.neutral.border.color.default": Color ->
        semantic::BORDER_COLOR_STRONG_DEFAULT;

    /// Notice button: background color.
    BUTTON_NOTICE_BACKGROUND_COLOR_DEFAULT = "button.notice.background.color.default": Color ->
        semantic::BACKGROUND_NOTICE_DEFAULT;
    /// Notice button: background color, hovered.
    BUTTON_NOTICE_BACKGROUND_COLOR_HOVER = "button.notice.background.color.hover": Color ->
        semantic::NOTICE_COLOR_500;
    /// Notice button: background color, pressed.
    BUTTON_NOTICE_BACKGROUND_COLOR_PRESSED = "button.notice.background.color.pressed": Color ->
        semantic::NOTICE_COLOR_700;
    /// Notice button: label color.
    BUTTON_NOTICE_LABEL_COLOR_DEFAULT = "button.notice.label.color.default": Color ->
        semantic::STATIC_TEXT_COLOR_BLACK;
    /// Notice button: border color.
    BUTTON_NOTICE_BORDER_COLOR_DEFAULT = "button.notice.border.color.default": Color ->
        semantic::NOTICE_COLOR_500;

    /// Button shaded from one color (a color family, a selected filled variant): lightness
    /// added to the color when hovered (OKLCH).
    BUTTON_SHADE_HOVER_LIGHTNESS = "button.shade.hover.lightness": Number =
        0.06, range(-0.5, 0.5);
    /// Button shaded from one color (a color family): lightness added to the color when
    /// pressed (OKLCH).
    BUTTON_SHADE_PRESSED_LIGHTNESS = "button.shade.pressed.lightness": Number =
        -0.05, range(-0.5, 0.5);
    /// Disabled button colored by a family: how much of the family's color stays.
    BUTTON_HUE_DISABLED_TINT = "button.hue.disabled.tint": Number = 0.35, range(0.0, 1.0);

    /// Blue buttons: fill of colored variants.
    BUTTON_HUE_BLUE_FILL = "button.hue.blue.fill": Color -> semantic::BACKGROUND_BLUE_DEFAULT;
    /// Blue buttons: background of the default variant.
    BUTTON_HUE_BLUE_SUBTLE = "button.hue.blue.subtle": Color ->
        semantic::BACKGROUND_BLUE_SUBTLE_DEFAULT;
    /// Blue buttons: label and border of the default variant.
    BUTTON_HUE_BLUE_TEXT = "button.hue.blue.text": Color -> semantic::TEXT_COLOR_BLUE_DEFAULT;
    /// Cyan buttons: fill of colored variants.
    BUTTON_HUE_CYAN_FILL = "button.hue.cyan.fill": Color -> semantic::BACKGROUND_CYAN_DEFAULT;
    /// Cyan buttons: background of the default variant.
    BUTTON_HUE_CYAN_SUBTLE = "button.hue.cyan.subtle": Color ->
        semantic::BACKGROUND_CYAN_SUBTLE_DEFAULT;
    /// Cyan buttons: label and border of the default variant.
    BUTTON_HUE_CYAN_TEXT = "button.hue.cyan.text": Color -> semantic::TEXT_COLOR_CYAN_DEFAULT;
    /// Green buttons: fill of colored variants.
    BUTTON_HUE_GREEN_FILL = "button.hue.green.fill": Color -> semantic::BACKGROUND_GREEN_DEFAULT;
    /// Green buttons: background of the default variant.
    BUTTON_HUE_GREEN_SUBTLE = "button.hue.green.subtle": Color ->
        semantic::BACKGROUND_GREEN_SUBTLE_DEFAULT;
    /// Green buttons: label and border of the default variant.
    BUTTON_HUE_GREEN_TEXT = "button.hue.green.text": Color -> semantic::TEXT_COLOR_GREEN_DEFAULT;
    /// Yellow buttons: fill of colored variants.
    BUTTON_HUE_YELLOW_FILL = "button.hue.yellow.fill": Color ->
        semantic::BACKGROUND_YELLOW_DEFAULT;
    /// Yellow buttons: background of the default variant.
    BUTTON_HUE_YELLOW_SUBTLE = "button.hue.yellow.subtle": Color ->
        semantic::BACKGROUND_YELLOW_SUBTLE_DEFAULT;
    /// Yellow buttons: label and border of the default variant.
    BUTTON_HUE_YELLOW_TEXT = "button.hue.yellow.text": Color ->
        semantic::TEXT_COLOR_YELLOW_DEFAULT;
    /// Orange buttons: fill of colored variants.
    BUTTON_HUE_ORANGE_FILL = "button.hue.orange.fill": Color ->
        semantic::BACKGROUND_ORANGE_DEFAULT;
    /// Orange buttons: background of the default variant.
    BUTTON_HUE_ORANGE_SUBTLE = "button.hue.orange.subtle": Color ->
        semantic::BACKGROUND_ORANGE_SUBTLE_DEFAULT;
    /// Orange buttons: label and border of the default variant.
    BUTTON_HUE_ORANGE_TEXT = "button.hue.orange.text": Color ->
        semantic::TEXT_COLOR_ORANGE_DEFAULT;
    /// Red buttons: fill of colored variants.
    BUTTON_HUE_RED_FILL = "button.hue.red.fill": Color -> semantic::BACKGROUND_RED_DEFAULT;
    /// Red buttons: background of the default variant.
    BUTTON_HUE_RED_SUBTLE = "button.hue.red.subtle": Color ->
        semantic::BACKGROUND_RED_SUBTLE_DEFAULT;
    /// Red buttons: label and border of the default variant.
    BUTTON_HUE_RED_TEXT = "button.hue.red.text": Color -> semantic::TEXT_COLOR_RED_DEFAULT;
    /// Magenta buttons: fill of colored variants.
    BUTTON_HUE_MAGENTA_FILL = "button.hue.magenta.fill": Color ->
        semantic::BACKGROUND_MAGENTA_DEFAULT;
    /// Magenta buttons: background of the default variant.
    BUTTON_HUE_MAGENTA_SUBTLE = "button.hue.magenta.subtle": Color ->
        semantic::BACKGROUND_MAGENTA_SUBTLE_DEFAULT;
    /// Magenta buttons: label and border of the default variant.
    BUTTON_HUE_MAGENTA_TEXT = "button.hue.magenta.text": Color ->
        semantic::TEXT_COLOR_MAGENTA_DEFAULT;
    /// Purple buttons: fill of colored variants.
    BUTTON_HUE_PURPLE_FILL = "button.hue.purple.fill": Color ->
        semantic::BACKGROUND_PURPLE_DEFAULT;
    /// Purple buttons: background of the default variant.
    BUTTON_HUE_PURPLE_SUBTLE = "button.hue.purple.subtle": Color ->
        semantic::BACKGROUND_PURPLE_SUBTLE_DEFAULT;
    /// Purple buttons: label and border of the default variant.
    BUTTON_HUE_PURPLE_TEXT = "button.hue.purple.text": Color ->
        semantic::TEXT_COLOR_PURPLE_DEFAULT;

    /// Button group: width of the borders around and between grouped buttons.
    BUTTON_GROUP_BORDER_WIDTH_DEFAULT = "button.group.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
}
