//! Text, icon, border and focus colors, the border switch, disabled and static colors, tracks,
//! titles and selection markers.

use super::*;
use crate::primitive;

koebi_design::tokens! {
    group CONTENT: Semantic, start = BACKGROUND::END;

    /// Text color of accent content.
    TEXT_COLOR_ACCENT_DEFAULT = "text.color.accent.default": Color -> ACCENT_COLOR_500;
    /// Text color of accent content, hovered.
    TEXT_COLOR_ACCENT_HOVER = "text.color.accent.hover": Color -> ACCENT_COLOR_400;
    /// Text color of accent content, pressed.
    TEXT_COLOR_ACCENT_PRESSED = "text.color.accent.pressed": Color -> ACCENT_COLOR_400;
    /// Text color of accent content, focused with the keyboard.
    TEXT_COLOR_ACCENT_KBD_FOCUS = "text.color.accent.kbd-focus": Color -> ACCENT_COLOR_400;
    /// Text color of accent content, selected.
    TEXT_COLOR_ACCENT_SELECTED = "text.color.accent.selected": Color -> ACCENT_COLOR_400;

    /// Text color of negative content.
    TEXT_COLOR_NEGATIVE_DEFAULT = "text.color.negative.default": Color -> NEGATIVE_COLOR_500;
    /// Text color of negative content, hovered.
    TEXT_COLOR_NEGATIVE_HOVER = "text.color.negative.hover": Color -> NEGATIVE_COLOR_400;
    /// Text color of negative content, pressed.
    TEXT_COLOR_NEGATIVE_PRESSED = "text.color.negative.pressed": Color -> NEGATIVE_COLOR_400;
    /// Text color of negative content, focused with the keyboard.
    TEXT_COLOR_NEGATIVE_KBD_FOCUS = "text.color.negative.kbd-focus": Color -> NEGATIVE_COLOR_400;

    /// Text color of neutral content.
    TEXT_COLOR_NEUTRAL_DEFAULT = "text.color.neutral.default": Color -> NEUTRAL_COLOR_200;
    /// Text color of neutral content, hovered.
    TEXT_COLOR_NEUTRAL_HOVER = "text.color.neutral.hover": Color -> NEUTRAL_COLOR_100;
    /// Text color of neutral content, pressed.
    TEXT_COLOR_NEUTRAL_PRESSED = "text.color.neutral.pressed": Color -> NEUTRAL_COLOR_100;
    /// Text color of neutral content, focused with the keyboard.
    TEXT_COLOR_NEUTRAL_KBD_FOCUS = "text.color.neutral.kbd-focus": Color -> NEUTRAL_COLOR_100;

    /// Primary text color.
    TEXT_COLOR_PRIMARY_DEFAULT = "text.color.primary.default": Color -> primitive::COLOR_GRAY_100;

    /// Secondary text color, for supporting content.
    TEXT_COLOR_SECONDARY_DEFAULT = "text.color.secondary.default": Color ->
        primitive::COLOR_GRAY_300;

    /// Tertiary text color, for hints and de-emphasized content.
    TEXT_COLOR_TERTIARY_DEFAULT = "text.color.tertiary.default": Color -> primitive::COLOR_GRAY_400;

    /// Text color of disabled elements.
    TEXT_COLOR_DISABLED_DEFAULT = "text.color.disabled.default": Color -> primitive::COLOR_GRAY_500;

    /// Text color of links.
    TEXT_COLOR_LINK_DEFAULT = "text.color.link.default": Color -> ACCENT_COLOR_500;

    /// Placeholder text color of empty input fields.
    TEXT_COLOR_PLACEHOLDER_DEFAULT = "text.color.placeholder.default": Color ->
        primitive::COLOR_GRAY_500;

    /// Blue text color for tagged content.
    TEXT_COLOR_BLUE_DEFAULT = "text.color.blue.default": Color -> primitive::COLOR_BLUE_500;

    /// Magenta text color for tagged content.
    TEXT_COLOR_MAGENTA_DEFAULT = "text.color.magenta.default": Color ->
        primitive::COLOR_MAGENTA_500;

    /// Green text color for tagged content.
    TEXT_COLOR_GREEN_DEFAULT = "text.color.green.default": Color -> primitive::COLOR_GREEN_500;

    /// Red text color for tagged content.
    TEXT_COLOR_RED_DEFAULT = "text.color.red.default": Color -> primitive::COLOR_RED_500;

    /// Yellow text color for tagged content.
    TEXT_COLOR_YELLOW_DEFAULT = "text.color.yellow.default": Color -> primitive::COLOR_YELLOW_500;

    /// Purple text color for tagged content.
    TEXT_COLOR_PURPLE_DEFAULT = "text.color.purple.default": Color -> primitive::COLOR_PURPLE_500;

    /// Orange text color for tagged content.
    TEXT_COLOR_ORANGE_DEFAULT = "text.color.orange.default": Color -> primitive::COLOR_ORANGE_500;

    /// Cyan text color for tagged content.
    TEXT_COLOR_CYAN_DEFAULT = "text.color.cyan.default": Color -> primitive::COLOR_CYAN_500;

    /// Primary icon color.
    ICON_COLOR_PRIMARY_DEFAULT = "icon.color.primary.default": Color -> primitive::COLOR_GRAY_100;

    /// Secondary icon color, for supporting content.
    ICON_COLOR_SECONDARY_DEFAULT = "icon.color.secondary.default": Color ->
        primitive::COLOR_GRAY_400;

    /// Tertiary icon color, for hints and de-emphasized content.
    ICON_COLOR_TERTIARY_DEFAULT = "icon.color.tertiary.default": Color -> primitive::COLOR_GRAY_500;

    /// Icon color of disabled elements.
    ICON_COLOR_DISABLED_DEFAULT = "icon.color.disabled.default": Color -> primitive::COLOR_GRAY_500;

    /// Icon color of accent content.
    ICON_COLOR_ACCENT_DEFAULT = "icon.color.accent.default": Color -> ACCENT_COLOR_500;

    /// Icon color of negative content.
    ICON_COLOR_NEGATIVE_DEFAULT = "icon.color.negative.default": Color -> NEGATIVE_COLOR_500;

    /// Icon color of positive content.
    ICON_COLOR_POSITIVE_DEFAULT = "icon.color.positive.default": Color -> POSITIVE_COLOR_500;

    /// Icon color of notice content.
    ICON_COLOR_NOTICE_DEFAULT = "icon.color.notice.default": Color -> NOTICE_COLOR_500;

    /// Icon color of informative content.
    ICON_COLOR_INFO_DEFAULT = "icon.color.info.default": Color -> INFO_COLOR_500;

    /// Blue icon color for tagged content.
    ICON_COLOR_BLUE_DEFAULT = "icon.color.blue.default": Color -> primitive::COLOR_BLUE_500;

    /// Magenta icon color for tagged content.
    ICON_COLOR_MAGENTA_DEFAULT = "icon.color.magenta.default": Color ->
        primitive::COLOR_MAGENTA_500;

    /// Green icon color for tagged content.
    ICON_COLOR_GREEN_DEFAULT = "icon.color.green.default": Color -> primitive::COLOR_GREEN_500;

    /// Red icon color for tagged content.
    ICON_COLOR_RED_DEFAULT = "icon.color.red.default": Color -> primitive::COLOR_RED_500;

    /// Yellow icon color for tagged content.
    ICON_COLOR_YELLOW_DEFAULT = "icon.color.yellow.default": Color -> primitive::COLOR_YELLOW_500;

    /// Purple icon color for tagged content.
    ICON_COLOR_PURPLE_DEFAULT = "icon.color.purple.default": Color -> primitive::COLOR_PURPLE_500;

    /// Orange icon color for tagged content.
    ICON_COLOR_ORANGE_DEFAULT = "icon.color.orange.default": Color -> primitive::COLOR_ORANGE_500;

    /// Cyan icon color for tagged content.
    ICON_COLOR_CYAN_DEFAULT = "icon.color.cyan.default": Color -> primitive::COLOR_CYAN_500;

    /// Default border color.
    BORDER_COLOR_DEFAULT = "border.color.default": Color -> primitive::COLOR_GRAY_600;

    /// Subtle border color, for low-emphasis separations.
    BORDER_COLOR_SUBTLE_DEFAULT = "border.color.subtle.default": Color -> primitive::COLOR_GRAY_700;

    /// Strong border color, for high-emphasis outlines.
    BORDER_COLOR_STRONG_DEFAULT = "border.color.strong.default": Color -> primitive::COLOR_GRAY_500;

    /// Border color of disabled elements.
    BORDER_COLOR_DISABLED_DEFAULT = "border.color.disabled.default": Color ->
        primitive::COLOR_GRAY_700;

    /// Border color of negative elements.
    BORDER_COLOR_NEGATIVE_DEFAULT = "border.color.negative.default": Color -> NEGATIVE_COLOR_500;
    /// Border color of negative elements, hovered.
    BORDER_COLOR_NEGATIVE_HOVER = "border.color.negative.hover": Color -> NEGATIVE_COLOR_400;
    /// Border color of negative elements, pressed.
    BORDER_COLOR_NEGATIVE_PRESSED = "border.color.negative.pressed": Color -> NEGATIVE_COLOR_300;
    /// Border color of negative elements, focused.
    BORDER_COLOR_NEGATIVE_FOCUS = "border.color.negative.focus": Color -> NEGATIVE_COLOR_400;
    /// Border color of negative elements, focused with the keyboard.
    BORDER_COLOR_NEGATIVE_KBD_FOCUS = "border.color.negative.kbd-focus": Color ->
        NEGATIVE_COLOR_400;

    /// Border color of accent elements.
    BORDER_COLOR_ACCENT_DEFAULT = "border.color.accent.default": Color -> ACCENT_COLOR_500;

    /// Border color of the focused element.
    BORDER_COLOR_FOCUS_DEFAULT = "border.color.focus.default": Color -> ACCENT_COLOR_500;

    /// Blue border color for tagged elements.
    BORDER_COLOR_BLUE_DEFAULT = "border.color.blue.default": Color -> primitive::COLOR_BLUE_500;

    /// Magenta border color for tagged elements.
    BORDER_COLOR_MAGENTA_DEFAULT = "border.color.magenta.default": Color ->
        primitive::COLOR_MAGENTA_500;

    /// Green border color for tagged elements.
    BORDER_COLOR_GREEN_DEFAULT = "border.color.green.default": Color -> primitive::COLOR_GREEN_500;

    /// Red border color for tagged elements.
    BORDER_COLOR_RED_DEFAULT = "border.color.red.default": Color -> primitive::COLOR_RED_500;

    /// Yellow border color for tagged elements.
    BORDER_COLOR_YELLOW_DEFAULT = "border.color.yellow.default": Color ->
        primitive::COLOR_YELLOW_500;

    /// Purple border color for tagged elements.
    BORDER_COLOR_PURPLE_DEFAULT = "border.color.purple.default": Color ->
        primitive::COLOR_PURPLE_500;

    /// Orange border color for tagged elements.
    BORDER_COLOR_ORANGE_DEFAULT = "border.color.orange.default": Color ->
        primitive::COLOR_ORANGE_500;

    /// Cyan border color for tagged elements.
    BORDER_COLOR_CYAN_DEFAULT = "border.color.cyan.default": Color -> primitive::COLOR_CYAN_500;

    /// Shadow line drawn along borders.
    BORDER_COLOR_SHADOW_DEFAULT = "border.color.shadow.default": Color ->
        primitive::COLOR_TRANSPARENT_BLACK_500;

    /// Color of tree connector lines in outliners and graphs.
    BORDER_COLOR_TREE_LINE_DEFAULT = "border.color.tree-line.default": Color ->
        primitive::COLOR_GRAY_600;

    /// Whether controls draw borders: 1 draws them, 0 does not.
    BORDER_ENABLED_DEFAULT = "border.enabled.default": Integer -> primitive::BORDER_ENABLED;

    /// Color of the focus indicator ring.
    FOCUS_COLOR_INDICATOR_DEFAULT = "focus.color.indicator.default": Color ->
        primitive::COLOR_BLUE_500;

    /// Focus color, highlighting the focused element.
    FOCUS_COLOR_DEFAULT = "focus.color.default": Color -> ACCENT_COLOR_500;
    /// Highlight of the window selected in the window switcher.
    FOCUS_COLOR_WINDOWING = "focus.color.windowing": Color -> primitive::COLOR_GRAY_100;

    /// Background of disabled elements.
    DISABLED_BACKGROUND_DEFAULT = "disabled.background.default": Color -> primitive::COLOR_GRAY_100;

    /// Border of disabled elements.
    DISABLED_BORDER_DEFAULT = "disabled.border.default": Color -> primitive::COLOR_GRAY_300;

    /// Content (text and icons) of disabled elements.
    DISABLED_CONTENT_DEFAULT = "disabled.content.default": Color -> primitive::COLOR_GRAY_400;

    /// Background of disabled elements on light media, in every theme.
    DISABLED_STATIC_BLACK_BACKGROUND_DEFAULT = "disabled.static-black.background.default": Color ->
        primitive::COLOR_TRANSPARENT_BLACK_100;

    /// Border of disabled elements on light media, in every theme.
    DISABLED_STATIC_BLACK_BORDER_DEFAULT = "disabled.static-black.border.default": Color ->
        primitive::COLOR_TRANSPARENT_BLACK_300;

    /// Content (text and icons) of disabled elements on light media, in every theme.
    DISABLED_STATIC_BLACK_CONTENT_DEFAULT = "disabled.static-black.content.default": Color ->
        primitive::COLOR_TRANSPARENT_BLACK_400;

    /// Background of disabled elements on dark media, in every theme.
    DISABLED_STATIC_WHITE_BACKGROUND_DEFAULT = "disabled.static-white.background.default": Color ->
        primitive::COLOR_TRANSPARENT_WHITE_100;

    /// Border of disabled elements on dark media, in every theme.
    DISABLED_STATIC_WHITE_BORDER_DEFAULT = "disabled.static-white.border.default": Color ->
        primitive::COLOR_TRANSPARENT_WHITE_300;

    /// Content (text and icons) of disabled elements on dark media, in every theme.
    DISABLED_STATIC_WHITE_CONTENT_DEFAULT = "disabled.static-white.content.default": Color ->
        primitive::COLOR_TRANSPARENT_WHITE_400;

    /// Text color on light media (black in every theme).
    STATIC_TEXT_COLOR_BLACK = "static.text.color.black": Color -> primitive::COLOR_GRAY_1000;
    /// Text color on dark media (white in every theme).
    STATIC_TEXT_COLOR_WHITE = "static.text.color.white": Color -> primitive::COLOR_GRAY_25;

    /// Track color on light media (black in every theme).
    STATIC_TRACK_COLOR_BLACK = "static.track.color.black": Color ->
        primitive::COLOR_TRANSPARENT_BLACK_300;
    /// Track color on dark media (white in every theme).
    STATIC_TRACK_COLOR_WHITE = "static.track.color.white": Color ->
        primitive::COLOR_TRANSPARENT_WHITE_300;

    /// Track fill color on light media (black in every theme).
    STATIC_TRACK_INDICATOR_COLOR_BLACK = "static.track-indicator.color.black": Color ->
        primitive::COLOR_TRANSPARENT_BLACK_900;
    /// Track fill color on dark media (white in every theme).
    STATIC_TRACK_INDICATOR_COLOR_WHITE = "static.track-indicator.color.white": Color ->
        primitive::COLOR_TRANSPARENT_WHITE_900;

    /// Focus color on light media (black in every theme).
    STATIC_FOCUS_COLOR_BLACK = "static.focus.color.black": Color -> primitive::COLOR_GRAY_1000;
    /// Focus color on dark media (white in every theme).
    STATIC_FOCUS_COLOR_WHITE = "static.focus.color.white": Color -> primitive::COLOR_GRAY_25;

    /// Color of slider and progress tracks.
    TRACK_COLOR_DEFAULT = "track.color.default": Color -> primitive::COLOR_GRAY_300;

    /// Color of titles.
    TITLE_COLOR_DEFAULT = "title.color.default": Color -> primitive::COLOR_GRAY_900;

    /// Marker of the active object on a page.
    STATE_ACTIVE_ON_PAGE = "state.active.on-page": Color -> primitive::COLOR_ORANGE_500;
    /// Marker of the active object outside any page.
    STATE_ACTIVE_LOOSE = "state.active.loose": Color -> primitive::COLOR_PURPLE_700;

    /// Marker of selected objects on a page.
    STATE_SELECTED_ON_PAGE = "state.selected.on-page": Color -> primitive::COLOR_ORANGE_700;
    /// Marker of selected objects outside any page.
    STATE_SELECTED_LOOSE = "state.selected.loose": Color -> primitive::COLOR_PURPLE_600;
}
