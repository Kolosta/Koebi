//! Role palettes (accent, neutral, negative, positive, informative, notice), their vivid colors and
//! the row highlight colors. Themes re-map these scales: the muted green theme maps the accent to
//! green.

use crate::primitive;

koebi_design::tokens! {
    group ROLE: Semantic, start = primitive::MOTION::END;

    /// Accent role, palette step 100 (primary actions and selection).
    ACCENT_COLOR_100 = "accent.color.100": Color -> primitive::COLOR_BLUE_100;
    /// Accent role, palette step 200 (primary actions and selection).
    ACCENT_COLOR_200 = "accent.color.200": Color -> primitive::COLOR_BLUE_200;
    /// Accent role, palette step 300 (primary actions and selection).
    ACCENT_COLOR_300 = "accent.color.300": Color -> primitive::COLOR_BLUE_300;
    /// Accent role, palette step 400 (primary actions and selection).
    ACCENT_COLOR_400 = "accent.color.400": Color -> primitive::COLOR_BLUE_400;
    /// Accent role, palette step 500 (primary actions and selection).
    ACCENT_COLOR_500 = "accent.color.500": Color -> primitive::COLOR_BLUE_500;
    /// Accent role, palette step 600 (primary actions and selection).
    ACCENT_COLOR_600 = "accent.color.600": Color -> primitive::COLOR_BLUE_600;
    /// Accent role, palette step 700 (primary actions and selection).
    ACCENT_COLOR_700 = "accent.color.700": Color -> primitive::COLOR_BLUE_700;
    /// Accent role, palette step 800 (primary actions and selection).
    ACCENT_COLOR_800 = "accent.color.800": Color -> primitive::COLOR_BLUE_800;
    /// Accent role, palette step 900 (primary actions and selection).
    ACCENT_COLOR_900 = "accent.color.900": Color -> primitive::COLOR_BLUE_900;
    /// Accent role, palette step 1000 (primary actions and selection).
    ACCENT_COLOR_1000 = "accent.color.1000": Color -> primitive::COLOR_BLUE_1000;
    /// Accent role, palette step 1100 (primary actions and selection).
    ACCENT_COLOR_1100 = "accent.color.1100": Color -> primitive::COLOR_BLUE_1100;
    /// Accent role, palette step 1200 (primary actions and selection).
    ACCENT_COLOR_1200 = "accent.color.1200": Color -> primitive::COLOR_BLUE_1200;
    /// Accent role, palette step 1300 (primary actions and selection).
    ACCENT_COLOR_1300 = "accent.color.1300": Color -> primitive::COLOR_BLUE_1300;
    /// Accent role, palette step 1400 (primary actions and selection).
    ACCENT_COLOR_1400 = "accent.color.1400": Color -> primitive::COLOR_BLUE_1400;
    /// Accent role, palette step 1500 (primary actions and selection).
    ACCENT_COLOR_1500 = "accent.color.1500": Color -> primitive::COLOR_BLUE_1500;
    /// Accent role, palette step 1600 (primary actions and selection).
    ACCENT_COLOR_1600 = "accent.color.1600": Color -> primitive::COLOR_BLUE_1600;
    /// Vivid accent color, for indicators and data visualization (the accent counterpart of
    /// `info.color.visual.default`).
    ACCENT_COLOR_VISUAL_BASE = "accent.color.visual-base": Color -> ACCENT_COLOR_500;

    /// Row highlight, strong tint: default variant.
    ACCENT_COLOR_VISUAL_DEFAULT = "accent.color.visual.default": Color -> NEUTRAL_COLOR_900;
    /// Row highlight, strong tint: positive variant.
    ACCENT_COLOR_VISUAL_POSITIVE = "accent.color.visual.positive": Color -> POSITIVE_COLOR_1300;
    /// Row highlight, strong tint: negative variant.
    ACCENT_COLOR_VISUAL_NEGATIVE = "accent.color.visual.negative": Color -> NEGATIVE_COLOR_1300;
    /// Row highlight, strong tint: informative variant.
    ACCENT_COLOR_VISUAL_INFO = "accent.color.visual.info": Color -> INFO_COLOR_1300;
    /// Row highlight, strong tint: neutral variant.
    ACCENT_COLOR_VISUAL_NEUTRAL = "accent.color.visual.neutral": Color -> NEUTRAL_COLOR_900;
    /// Row highlight, strong tint: notice variant.
    ACCENT_COLOR_VISUAL_NOTICE = "accent.color.visual.notice": Color -> NOTICE_COLOR_1300;
    /// Row highlight, strong tint: brand variant.
    ACCENT_COLOR_VISUAL_BRAND = "accent.color.visual.brand": Color -> primitive::COLOR_PURPLE_1300;

    /// Row highlight, hovered: default variant.
    ACCENT_COLOR_HOVER_DEFAULT = "accent.color.hover.default": Color -> NEUTRAL_COLOR_500;
    /// Row highlight, hovered: positive variant.
    ACCENT_COLOR_HOVER_POSITIVE = "accent.color.hover.positive": Color -> POSITIVE_COLOR_700;
    /// Row highlight, hovered: negative variant.
    ACCENT_COLOR_HOVER_NEGATIVE = "accent.color.hover.negative": Color -> NEGATIVE_COLOR_700;
    /// Row highlight, hovered: informative variant.
    ACCENT_COLOR_HOVER_INFO = "accent.color.hover.info": Color -> INFO_COLOR_700;
    /// Row highlight, hovered: neutral variant.
    ACCENT_COLOR_HOVER_NEUTRAL = "accent.color.hover.neutral": Color -> NEUTRAL_COLOR_500;
    /// Row highlight, hovered: notice variant.
    ACCENT_COLOR_HOVER_NOTICE = "accent.color.hover.notice": Color -> NOTICE_COLOR_700;
    /// Row highlight, hovered: brand variant.
    ACCENT_COLOR_HOVER_BRAND = "accent.color.hover.brand": Color -> primitive::COLOR_PURPLE_700;

    /// Row highlight, text on a highlighted row: default variant.
    ACCENT_COLOR_TEXT_DEFAULT = "accent.color.text.default": Color -> NEUTRAL_COLOR_300;
    /// Row highlight, text on a highlighted row: positive variant.
    ACCENT_COLOR_TEXT_POSITIVE = "accent.color.text.positive": Color -> POSITIVE_COLOR_300;
    /// Row highlight, text on a highlighted row: negative variant.
    ACCENT_COLOR_TEXT_NEGATIVE = "accent.color.text.negative": Color -> NEGATIVE_COLOR_300;
    /// Row highlight, text on a highlighted row: informative variant.
    ACCENT_COLOR_TEXT_INFO = "accent.color.text.info": Color -> INFO_COLOR_300;
    /// Row highlight, text on a highlighted row: neutral variant.
    ACCENT_COLOR_TEXT_NEUTRAL = "accent.color.text.neutral": Color -> NEUTRAL_COLOR_300;
    /// Row highlight, text on a highlighted row: notice variant.
    ACCENT_COLOR_TEXT_NOTICE = "accent.color.text.notice": Color -> NOTICE_COLOR_300;
    /// Row highlight, text on a highlighted row: brand variant.
    ACCENT_COLOR_TEXT_BRAND = "accent.color.text.brand": Color -> primitive::COLOR_PURPLE_300;

    /// Row highlight, selected: default variant.
    ACCENT_COLOR_SELECTED_DEFAULT = "accent.color.selected.default": Color -> ACCENT_COLOR_800;
    /// Row highlight, selected: positive variant.
    ACCENT_COLOR_SELECTED_POSITIVE = "accent.color.selected.positive": Color -> POSITIVE_COLOR_800;
    /// Row highlight, selected: negative variant.
    ACCENT_COLOR_SELECTED_NEGATIVE = "accent.color.selected.negative": Color -> NEGATIVE_COLOR_800;
    /// Row highlight, selected: informative variant.
    ACCENT_COLOR_SELECTED_INFO = "accent.color.selected.info": Color -> INFO_COLOR_800;
    /// Row highlight, selected: neutral variant.
    ACCENT_COLOR_SELECTED_NEUTRAL = "accent.color.selected.neutral": Color -> NEUTRAL_COLOR_800;
    /// Row highlight, selected: notice variant.
    ACCENT_COLOR_SELECTED_NOTICE = "accent.color.selected.notice": Color -> NOTICE_COLOR_800;
    /// Row highlight, selected: brand variant.
    ACCENT_COLOR_SELECTED_BRAND = "accent.color.selected.brand": Color ->
        primitive::COLOR_PURPLE_800;

    /// Row highlight, hovered while selected: default variant.
    ACCENT_COLOR_HOVER_SELECTED_DEFAULT = "accent.color.hover-selected.default": Color ->
        ACCENT_COLOR_700;
    /// Row highlight, hovered while selected: positive variant.
    ACCENT_COLOR_HOVER_SELECTED_POSITIVE = "accent.color.hover-selected.positive": Color ->
        POSITIVE_COLOR_700;
    /// Row highlight, hovered while selected: negative variant.
    ACCENT_COLOR_HOVER_SELECTED_NEGATIVE = "accent.color.hover-selected.negative": Color ->
        NEGATIVE_COLOR_700;
    /// Row highlight, hovered while selected: informative variant.
    ACCENT_COLOR_HOVER_SELECTED_INFO = "accent.color.hover-selected.info": Color -> INFO_COLOR_700;
    /// Row highlight, hovered while selected: neutral variant.
    ACCENT_COLOR_HOVER_SELECTED_NEUTRAL = "accent.color.hover-selected.neutral": Color ->
        NEUTRAL_COLOR_700;
    /// Row highlight, hovered while selected: notice variant.
    ACCENT_COLOR_HOVER_SELECTED_NOTICE = "accent.color.hover-selected.notice": Color ->
        NOTICE_COLOR_700;
    /// Row highlight, hovered while selected: brand variant.
    ACCENT_COLOR_HOVER_SELECTED_BRAND = "accent.color.hover-selected.brand": Color ->
        primitive::COLOR_PURPLE_700;

    /// Row highlight, active: default variant.
    ACCENT_COLOR_ACTIVE_DEFAULT = "accent.color.active.default": Color -> ACCENT_COLOR_600;
    /// Row highlight, active: positive variant.
    ACCENT_COLOR_ACTIVE_POSITIVE = "accent.color.active.positive": Color -> POSITIVE_COLOR_600;
    /// Row highlight, active: negative variant.
    ACCENT_COLOR_ACTIVE_NEGATIVE = "accent.color.active.negative": Color -> NEGATIVE_COLOR_600;
    /// Row highlight, active: informative variant.
    ACCENT_COLOR_ACTIVE_INFO = "accent.color.active.info": Color -> INFO_COLOR_600;
    /// Row highlight, active: neutral variant.
    ACCENT_COLOR_ACTIVE_NEUTRAL = "accent.color.active.neutral": Color -> NEUTRAL_COLOR_600;
    /// Row highlight, active: notice variant.
    ACCENT_COLOR_ACTIVE_NOTICE = "accent.color.active.notice": Color -> NOTICE_COLOR_600;
    /// Row highlight, active: brand variant.
    ACCENT_COLOR_ACTIVE_BRAND = "accent.color.active.brand": Color -> primitive::COLOR_PURPLE_600;

    /// Neutral role, palette step 25 (default UI chrome).
    NEUTRAL_COLOR_25 = "neutral.color.25": Color -> primitive::COLOR_GRAY_25;
    /// Neutral role, palette step 50 (default UI chrome).
    NEUTRAL_COLOR_50 = "neutral.color.50": Color -> primitive::COLOR_GRAY_50;
    /// Neutral role, palette step 75 (default UI chrome).
    NEUTRAL_COLOR_75 = "neutral.color.75": Color -> primitive::COLOR_GRAY_75;
    /// Neutral role, palette step 100 (default UI chrome).
    NEUTRAL_COLOR_100 = "neutral.color.100": Color -> primitive::COLOR_GRAY_100;
    /// Neutral role, palette step 200 (default UI chrome).
    NEUTRAL_COLOR_200 = "neutral.color.200": Color -> primitive::COLOR_GRAY_200;
    /// Neutral role, palette step 300 (default UI chrome).
    NEUTRAL_COLOR_300 = "neutral.color.300": Color -> primitive::COLOR_GRAY_300;
    /// Neutral role, palette step 400 (default UI chrome).
    NEUTRAL_COLOR_400 = "neutral.color.400": Color -> primitive::COLOR_GRAY_400;
    /// Neutral role, palette step 500 (default UI chrome).
    NEUTRAL_COLOR_500 = "neutral.color.500": Color -> primitive::COLOR_GRAY_500;
    /// Neutral role, palette step 600 (default UI chrome).
    NEUTRAL_COLOR_600 = "neutral.color.600": Color -> primitive::COLOR_GRAY_600;
    /// Neutral role, palette step 700 (default UI chrome).
    NEUTRAL_COLOR_700 = "neutral.color.700": Color -> primitive::COLOR_GRAY_700;
    /// Neutral role, palette step 800 (default UI chrome).
    NEUTRAL_COLOR_800 = "neutral.color.800": Color -> primitive::COLOR_GRAY_800;
    /// Neutral role, palette step 900 (default UI chrome).
    NEUTRAL_COLOR_900 = "neutral.color.900": Color -> primitive::COLOR_GRAY_900;
    /// Neutral role, palette step 1000 (default UI chrome).
    NEUTRAL_COLOR_1000 = "neutral.color.1000": Color -> primitive::COLOR_GRAY_1000;

    /// Vivid neutral color, for indicators and data visualization.
    NEUTRAL_COLOR_VISUAL_DEFAULT = "neutral.color.visual.default": Color ->
        primitive::COLOR_GRAY_500;

    /// Negative role, palette step 100 (errors and destructive actions).
    NEGATIVE_COLOR_100 = "negative.color.100": Color -> primitive::COLOR_RED_100;
    /// Negative role, palette step 200 (errors and destructive actions).
    NEGATIVE_COLOR_200 = "negative.color.200": Color -> primitive::COLOR_RED_200;
    /// Negative role, palette step 300 (errors and destructive actions).
    NEGATIVE_COLOR_300 = "negative.color.300": Color -> primitive::COLOR_RED_300;
    /// Negative role, palette step 400 (errors and destructive actions).
    NEGATIVE_COLOR_400 = "negative.color.400": Color -> primitive::COLOR_RED_400;
    /// Negative role, palette step 500 (errors and destructive actions).
    NEGATIVE_COLOR_500 = "negative.color.500": Color -> primitive::COLOR_RED_500;
    /// Negative role, palette step 600 (errors and destructive actions).
    NEGATIVE_COLOR_600 = "negative.color.600": Color -> primitive::COLOR_RED_600;
    /// Negative role, palette step 700 (errors and destructive actions).
    NEGATIVE_COLOR_700 = "negative.color.700": Color -> primitive::COLOR_RED_700;
    /// Negative role, palette step 800 (errors and destructive actions).
    NEGATIVE_COLOR_800 = "negative.color.800": Color -> primitive::COLOR_RED_800;
    /// Negative role, palette step 900 (errors and destructive actions).
    NEGATIVE_COLOR_900 = "negative.color.900": Color -> primitive::COLOR_RED_900;
    /// Negative role, palette step 1000 (errors and destructive actions).
    NEGATIVE_COLOR_1000 = "negative.color.1000": Color -> primitive::COLOR_RED_1000;
    /// Negative role, palette step 1100 (errors and destructive actions).
    NEGATIVE_COLOR_1100 = "negative.color.1100": Color -> primitive::COLOR_RED_1100;
    /// Negative role, palette step 1200 (errors and destructive actions).
    NEGATIVE_COLOR_1200 = "negative.color.1200": Color -> primitive::COLOR_RED_1200;
    /// Negative role, palette step 1300 (errors and destructive actions).
    NEGATIVE_COLOR_1300 = "negative.color.1300": Color -> primitive::COLOR_RED_1300;
    /// Negative role, palette step 1400 (errors and destructive actions).
    NEGATIVE_COLOR_1400 = "negative.color.1400": Color -> primitive::COLOR_RED_1400;
    /// Negative role, palette step 1500 (errors and destructive actions).
    NEGATIVE_COLOR_1500 = "negative.color.1500": Color -> primitive::COLOR_RED_1500;
    /// Negative role, palette step 1600 (errors and destructive actions).
    NEGATIVE_COLOR_1600 = "negative.color.1600": Color -> primitive::COLOR_RED_1600;

    /// Vivid negative color, for indicators and data visualization.
    NEGATIVE_COLOR_VISUAL_DEFAULT = "negative.color.visual.default": Color -> NEGATIVE_COLOR_500;

    /// Positive role, palette step 100 (success and validation).
    POSITIVE_COLOR_100 = "positive.color.100": Color -> primitive::COLOR_GREEN_100;
    /// Positive role, palette step 200 (success and validation).
    POSITIVE_COLOR_200 = "positive.color.200": Color -> primitive::COLOR_GREEN_200;
    /// Positive role, palette step 300 (success and validation).
    POSITIVE_COLOR_300 = "positive.color.300": Color -> primitive::COLOR_GREEN_300;
    /// Positive role, palette step 400 (success and validation).
    POSITIVE_COLOR_400 = "positive.color.400": Color -> primitive::COLOR_GREEN_400;
    /// Positive role, palette step 500 (success and validation).
    POSITIVE_COLOR_500 = "positive.color.500": Color -> primitive::COLOR_GREEN_500;
    /// Positive role, palette step 600 (success and validation).
    POSITIVE_COLOR_600 = "positive.color.600": Color -> primitive::COLOR_GREEN_600;
    /// Positive role, palette step 700 (success and validation).
    POSITIVE_COLOR_700 = "positive.color.700": Color -> primitive::COLOR_GREEN_700;
    /// Positive role, palette step 800 (success and validation).
    POSITIVE_COLOR_800 = "positive.color.800": Color -> primitive::COLOR_GREEN_800;
    /// Positive role, palette step 900 (success and validation).
    POSITIVE_COLOR_900 = "positive.color.900": Color -> primitive::COLOR_GREEN_900;
    /// Positive role, palette step 1000 (success and validation).
    POSITIVE_COLOR_1000 = "positive.color.1000": Color -> primitive::COLOR_GREEN_1000;
    /// Positive role, palette step 1100 (success and validation).
    POSITIVE_COLOR_1100 = "positive.color.1100": Color -> primitive::COLOR_GREEN_1100;
    /// Positive role, palette step 1200 (success and validation).
    POSITIVE_COLOR_1200 = "positive.color.1200": Color -> primitive::COLOR_GREEN_1200;
    /// Positive role, palette step 1300 (success and validation).
    POSITIVE_COLOR_1300 = "positive.color.1300": Color -> primitive::COLOR_GREEN_1300;
    /// Positive role, palette step 1400 (success and validation).
    POSITIVE_COLOR_1400 = "positive.color.1400": Color -> primitive::COLOR_GREEN_1400;
    /// Positive role, palette step 1500 (success and validation).
    POSITIVE_COLOR_1500 = "positive.color.1500": Color -> primitive::COLOR_GREEN_1500;
    /// Positive role, palette step 1600 (success and validation).
    POSITIVE_COLOR_1600 = "positive.color.1600": Color -> primitive::COLOR_GREEN_1600;

    /// Vivid positive color, for indicators and data visualization.
    POSITIVE_COLOR_VISUAL_DEFAULT = "positive.color.visual.default": Color -> POSITIVE_COLOR_500;

    /// Informative role, palette step 100 (information).
    INFO_COLOR_100 = "info.color.100": Color -> primitive::COLOR_BLUE_100;
    /// Informative role, palette step 200 (information).
    INFO_COLOR_200 = "info.color.200": Color -> primitive::COLOR_BLUE_200;
    /// Informative role, palette step 300 (information).
    INFO_COLOR_300 = "info.color.300": Color -> primitive::COLOR_BLUE_300;
    /// Informative role, palette step 400 (information).
    INFO_COLOR_400 = "info.color.400": Color -> primitive::COLOR_BLUE_400;
    /// Informative role, palette step 500 (information).
    INFO_COLOR_500 = "info.color.500": Color -> primitive::COLOR_BLUE_500;
    /// Informative role, palette step 600 (information).
    INFO_COLOR_600 = "info.color.600": Color -> primitive::COLOR_BLUE_600;
    /// Informative role, palette step 700 (information).
    INFO_COLOR_700 = "info.color.700": Color -> primitive::COLOR_BLUE_700;
    /// Informative role, palette step 800 (information).
    INFO_COLOR_800 = "info.color.800": Color -> primitive::COLOR_BLUE_800;
    /// Informative role, palette step 900 (information).
    INFO_COLOR_900 = "info.color.900": Color -> primitive::COLOR_BLUE_900;
    /// Informative role, palette step 1000 (information).
    INFO_COLOR_1000 = "info.color.1000": Color -> primitive::COLOR_BLUE_1000;
    /// Informative role, palette step 1100 (information).
    INFO_COLOR_1100 = "info.color.1100": Color -> primitive::COLOR_BLUE_1100;
    /// Informative role, palette step 1200 (information).
    INFO_COLOR_1200 = "info.color.1200": Color -> primitive::COLOR_BLUE_1200;
    /// Informative role, palette step 1300 (information).
    INFO_COLOR_1300 = "info.color.1300": Color -> primitive::COLOR_BLUE_1300;
    /// Informative role, palette step 1400 (information).
    INFO_COLOR_1400 = "info.color.1400": Color -> primitive::COLOR_BLUE_1400;
    /// Informative role, palette step 1500 (information).
    INFO_COLOR_1500 = "info.color.1500": Color -> primitive::COLOR_BLUE_1500;
    /// Informative role, palette step 1600 (information).
    INFO_COLOR_1600 = "info.color.1600": Color -> primitive::COLOR_BLUE_1600;

    /// Vivid informative color, for indicators and data visualization.
    INFO_COLOR_VISUAL_DEFAULT = "info.color.visual.default": Color -> INFO_COLOR_500;

    /// Notice role, palette step 100 (warnings).
    NOTICE_COLOR_100 = "notice.color.100": Color -> primitive::COLOR_ORANGE_100;
    /// Notice role, palette step 200 (warnings).
    NOTICE_COLOR_200 = "notice.color.200": Color -> primitive::COLOR_ORANGE_200;
    /// Notice role, palette step 300 (warnings).
    NOTICE_COLOR_300 = "notice.color.300": Color -> primitive::COLOR_ORANGE_300;
    /// Notice role, palette step 400 (warnings).
    NOTICE_COLOR_400 = "notice.color.400": Color -> primitive::COLOR_ORANGE_400;
    /// Notice role, palette step 500 (warnings).
    NOTICE_COLOR_500 = "notice.color.500": Color -> primitive::COLOR_ORANGE_500;
    /// Notice role, palette step 600 (warnings).
    NOTICE_COLOR_600 = "notice.color.600": Color -> primitive::COLOR_ORANGE_600;
    /// Notice role, palette step 700 (warnings).
    NOTICE_COLOR_700 = "notice.color.700": Color -> primitive::COLOR_ORANGE_700;
    /// Notice role, palette step 800 (warnings).
    NOTICE_COLOR_800 = "notice.color.800": Color -> primitive::COLOR_ORANGE_800;
    /// Notice role, palette step 900 (warnings).
    NOTICE_COLOR_900 = "notice.color.900": Color -> primitive::COLOR_ORANGE_900;
    /// Notice role, palette step 1000 (warnings).
    NOTICE_COLOR_1000 = "notice.color.1000": Color -> primitive::COLOR_ORANGE_1000;
    /// Notice role, palette step 1100 (warnings).
    NOTICE_COLOR_1100 = "notice.color.1100": Color -> primitive::COLOR_ORANGE_1100;
    /// Notice role, palette step 1200 (warnings).
    NOTICE_COLOR_1200 = "notice.color.1200": Color -> primitive::COLOR_ORANGE_1200;
    /// Notice role, palette step 1300 (warnings).
    NOTICE_COLOR_1300 = "notice.color.1300": Color -> primitive::COLOR_ORANGE_1300;
    /// Notice role, palette step 1400 (warnings).
    NOTICE_COLOR_1400 = "notice.color.1400": Color -> primitive::COLOR_ORANGE_1400;
    /// Notice role, palette step 1500 (warnings).
    NOTICE_COLOR_1500 = "notice.color.1500": Color -> primitive::COLOR_ORANGE_1500;
    /// Notice role, palette step 1600 (warnings).
    NOTICE_COLOR_1600 = "notice.color.1600": Color -> primitive::COLOR_ORANGE_1600;

    /// Vivid notice color, for indicators and data visualization.
    NOTICE_COLOR_VISUAL_DEFAULT = "notice.color.visual.default": Color -> NOTICE_COLOR_500;
}
