//! Background colors: structural levels, application surfaces, window parts, roles with their
//! states and tagged hues.

use super::*;
use crate::primitive;

koebi_design::tokens! {
    group BACKGROUND: Semantic, start = ROLE::END;

    /// Base background of the application.
    BACKGROUND_BASE_DEFAULT = "background.base.default": Color -> primitive::COLOR_GRAY_900;

    /// Background sunken below the base level: wells and tracks.
    BACKGROUND_SUNKEN_DEFAULT = "background.sunken.default": Color -> primitive::COLOR_GRAY_1000;

    /// Background raised above the base level: cards and bars.
    BACKGROUND_RAISED_DEFAULT = "background.raised.default": Color -> primitive::COLOR_GRAY_800;

    /// Background of floating surfaces: popovers, menus and tooltips.
    BACKGROUND_FLOATING_DEFAULT = "background.floating.default": Color -> primitive::COLOR_GRAY_800;

    /// Scrim that dims the content behind a modal surface.
    BACKGROUND_DIM_DEFAULT = "background.dim.default": Color ->
        primitive::COLOR_TRANSPARENT_BLACK_500;
    /// Scrim behind the window switcher.
    BACKGROUND_DIM_WINDOWING = "background.dim.windowing": Color ->
        primitive::COLOR_TRANSPARENT_BLACK_500;
    /// Scrim behind modal windows.
    BACKGROUND_DIM_MODAL = "background.dim.modal": Color -> primitive::COLOR_TRANSPARENT_BLACK_500;

    /// Background of large content surfaces such as editor canvases.
    BACKGROUND_SURFACE_CANVAS = "background.surface.canvas": Color -> primitive::COLOR_GRAY_875;
    /// Background of bars and toolbars above a canvas.
    BACKGROUND_SURFACE_RAISED = "background.surface.raised": Color -> primitive::COLOR_GRAY_800;

    /// Background of clickable controls such as buttons and dropdowns.
    BACKGROUND_APP_CONTROL = "background.app.control": Color -> primitive::COLOR_GRAY_850;
    /// Background of surfaces nested inside a panel.
    BACKGROUND_APP_CHILD = "background.app.child": Color -> primitive::COLOR_GRAY_780;
    /// Background of input fields and frames.
    BACKGROUND_APP_FRAME = "background.app.frame": Color -> primitive::COLOR_GRAY_740;
    /// Background of input fields and frames, hovered.
    BACKGROUND_APP_FRAME_HOVER = "background.app.frame-hover": Color -> primitive::COLOR_GRAY_700;

    /// Background of child surfaces nested in a window.
    BACKGROUND_CHILD_DEFAULT = "background.child.default": Color -> primitive::COLOR_GRAY_900;

    /// Background of menu bars.
    BACKGROUND_MENU_BAR_DEFAULT = "background.menu-bar.default": Color ->
        primitive::COLOR_GRAY_1000;

    /// Background of window title bars.
    BACKGROUND_TITLE_DEFAULT = "background.title.default": Color -> primitive::COLOR_GRAY_1000;
    /// Background of the title bar of the focused window.
    BACKGROUND_TITLE_ACTIVE = "background.title.active": Color -> ACCENT_COLOR_700;
    /// Background of collapsed window title bars.
    BACKGROUND_TITLE_COLLAPSED = "background.title.collapsed": Color -> primitive::COLOR_GRAY_1000;

    /// Background of scrollbar tracks.
    BACKGROUND_SCROLLBAR_TRACK_DEFAULT = "background.scrollbar-track.default": Color ->
        primitive::COLOR_GRAY_1000;

    /// Background of empty docking areas.
    BACKGROUND_DOCKING_EMPTY_DEFAULT = "background.docking-empty.default": Color ->
        primitive::COLOR_GRAY_1000;

    /// Highlight of a drop target.
    BACKGROUND_DROP_TARGET_DEFAULT = "background.drop-target.default": Color -> ACCENT_COLOR_600;

    /// Background behind selected text.
    BACKGROUND_TEXT_SELECTION_DEFAULT = "background.text-selection.default": Color ->
        ACCENT_COLOR_900;

    /// Background of accent elements.
    BACKGROUND_ACCENT_DEFAULT = "background.accent.default": Color -> ACCENT_COLOR_700;
    /// Background of accent elements, hovered.
    BACKGROUND_ACCENT_HOVER = "background.accent.hover": Color -> ACCENT_COLOR_600;
    /// Background of accent elements, pressed.
    BACKGROUND_ACCENT_PRESSED = "background.accent.pressed": Color -> ACCENT_COLOR_600;
    /// Background of accent elements, focused with the keyboard.
    BACKGROUND_ACCENT_KBD_FOCUS = "background.accent.kbd-focus": Color -> ACCENT_COLOR_600;
    /// Accent highlight of a drop target.
    BACKGROUND_ACCENT_DROP_TARGET = "background.accent.drop-target": Color ->
        primitive::COLOR_YELLOW_500;
    /// Preview of the place a docked window will take.
    BACKGROUND_ACCENT_DOCKING_PREVIEW = "background.accent.docking-preview": Color ->
        ACCENT_COLOR_500;

    /// Subtle background of accent content, such as badges.
    BACKGROUND_ACCENT_SUBTLE_DEFAULT = "background.accent.subtle.default": Color ->
        ACCENT_COLOR_1400;

    /// Background of neutral elements.
    BACKGROUND_NEUTRAL_DEFAULT = "background.neutral.default": Color -> NEUTRAL_COLOR_500;
    /// Background of neutral elements, hovered.
    BACKGROUND_NEUTRAL_HOVER = "background.neutral.hover": Color -> NEUTRAL_COLOR_400;
    /// Background of neutral elements, pressed.
    BACKGROUND_NEUTRAL_PRESSED = "background.neutral.pressed": Color -> NEUTRAL_COLOR_400;
    /// Background of neutral elements, focused with the keyboard.
    BACKGROUND_NEUTRAL_KBD_FOCUS = "background.neutral.kbd-focus": Color -> NEUTRAL_COLOR_400;

    /// Subtle background of neutral content, such as badges.
    BACKGROUND_NEUTRAL_SUBTLE_DEFAULT = "background.neutral.subtle.default": Color ->
        NEUTRAL_COLOR_800;

    /// Background of informative elements.
    BACKGROUND_INFO_DEFAULT = "background.info.default": Color -> INFO_COLOR_700;
    /// Background of informative elements, hovered.
    BACKGROUND_INFO_HOVER = "background.info.hover": Color -> INFO_COLOR_600;
    /// Background of informative elements, pressed.
    BACKGROUND_INFO_PRESSED = "background.info.pressed": Color -> INFO_COLOR_600;
    /// Background of informative elements, focused with the keyboard.
    BACKGROUND_INFO_KBD_FOCUS = "background.info.kbd-focus": Color -> INFO_COLOR_600;

    /// Subtle background of informative content, such as badges.
    BACKGROUND_INFO_SUBTLE_DEFAULT = "background.info.subtle.default": Color -> INFO_COLOR_1400;

    /// Background of positive elements.
    BACKGROUND_POSITIVE_DEFAULT = "background.positive.default": Color -> POSITIVE_COLOR_700;
    /// Background of positive elements, hovered.
    BACKGROUND_POSITIVE_HOVER = "background.positive.hover": Color -> POSITIVE_COLOR_600;
    /// Background of positive elements, pressed.
    BACKGROUND_POSITIVE_PRESSED = "background.positive.pressed": Color -> POSITIVE_COLOR_600;
    /// Background of positive elements, focused with the keyboard.
    BACKGROUND_POSITIVE_KBD_FOCUS = "background.positive.kbd-focus": Color -> POSITIVE_COLOR_600;

    /// Subtle background of positive content, such as badges.
    BACKGROUND_POSITIVE_SUBTLE_DEFAULT = "background.positive.subtle.default": Color ->
        POSITIVE_COLOR_1400;

    /// Background of notice elements.
    BACKGROUND_NOTICE_DEFAULT = "background.notice.default": Color -> NOTICE_COLOR_600;

    /// Subtle background of notice content, such as badges.
    BACKGROUND_NOTICE_SUBTLE_DEFAULT = "background.notice.subtle.default": Color ->
        NOTICE_COLOR_1400;

    /// Background of negative elements.
    BACKGROUND_NEGATIVE_DEFAULT = "background.negative.default": Color -> NEGATIVE_COLOR_700;
    /// Background of negative elements, hovered.
    BACKGROUND_NEGATIVE_HOVER = "background.negative.hover": Color -> NEGATIVE_COLOR_600;
    /// Background of negative elements, pressed.
    BACKGROUND_NEGATIVE_PRESSED = "background.negative.pressed": Color -> NEGATIVE_COLOR_600;
    /// Background of negative elements, focused with the keyboard.
    BACKGROUND_NEGATIVE_KBD_FOCUS = "background.negative.kbd-focus": Color -> NEGATIVE_COLOR_600;
    /// Background of negative elements while recording.
    BACKGROUND_NEGATIVE_RECORDING = "background.negative.recording": Color -> NEGATIVE_COLOR_600;

    /// Subtle background of negative content, such as badges.
    BACKGROUND_NEGATIVE_SUBTLE_DEFAULT = "background.negative.subtle.default": Color ->
        NEGATIVE_COLOR_1400;

    /// Blue background for tagged or categorized elements.
    BACKGROUND_BLUE_DEFAULT = "background.blue.default": Color -> primitive::COLOR_BLUE_600;

    /// Subtle blue background for tagged or categorized elements.
    BACKGROUND_BLUE_SUBTLE_DEFAULT = "background.blue.subtle.default": Color ->
        primitive::COLOR_BLUE_1400;

    /// Vivid blue background for data visualization.
    BACKGROUND_BLUE_VISUAL_DEFAULT = "background.blue.visual.default": Color ->
        primitive::COLOR_BLUE_500;

    /// Brown background for tagged or categorized elements.
    BACKGROUND_BROWN_DEFAULT = "background.brown.default": Color -> primitive::COLOR_BROWN_600;

    /// Subtle brown background for tagged or categorized elements.
    BACKGROUND_BROWN_SUBTLE_DEFAULT = "background.brown.subtle.default": Color ->
        primitive::COLOR_BROWN_1400;

    /// Vivid brown background for data visualization.
    BACKGROUND_BROWN_VISUAL_DEFAULT = "background.brown.visual.default": Color ->
        primitive::COLOR_BROWN_500;

    /// Celery background for tagged or categorized elements.
    BACKGROUND_CELERY_DEFAULT = "background.celery.default": Color -> primitive::COLOR_CELERY_600;

    /// Subtle celery background for tagged or categorized elements.
    BACKGROUND_CELERY_SUBTLE_DEFAULT = "background.celery.subtle.default": Color ->
        primitive::COLOR_CELERY_1400;

    /// Vivid celery background for data visualization.
    BACKGROUND_CELERY_VISUAL_DEFAULT = "background.celery.visual.default": Color ->
        primitive::COLOR_CELERY_500;

    /// Chartreuse background for tagged or categorized elements.
    BACKGROUND_CHARTREUSE_DEFAULT = "background.chartreuse.default": Color ->
        primitive::COLOR_CHARTREUSE_600;

    /// Subtle chartreuse background for tagged or categorized elements.
    BACKGROUND_CHARTREUSE_SUBTLE_DEFAULT = "background.chartreuse.subtle.default": Color ->
        primitive::COLOR_CHARTREUSE_1400;

    /// Vivid chartreuse background for data visualization.
    BACKGROUND_CHARTREUSE_VISUAL_DEFAULT = "background.chartreuse.visual.default": Color ->
        primitive::COLOR_CHARTREUSE_500;

    /// Cinnamon background for tagged or categorized elements.
    BACKGROUND_CINNAMON_DEFAULT = "background.cinnamon.default": Color ->
        primitive::COLOR_CINNAMON_600;

    /// Subtle cinnamon background for tagged or categorized elements.
    BACKGROUND_CINNAMON_SUBTLE_DEFAULT = "background.cinnamon.subtle.default": Color ->
        primitive::COLOR_CINNAMON_1400;

    /// Vivid cinnamon background for data visualization.
    BACKGROUND_CINNAMON_VISUAL_DEFAULT = "background.cinnamon.visual.default": Color ->
        primitive::COLOR_CINNAMON_500;

    /// Cyan background for tagged or categorized elements.
    BACKGROUND_CYAN_DEFAULT = "background.cyan.default": Color -> primitive::COLOR_CYAN_600;

    /// Subtle cyan background for tagged or categorized elements.
    BACKGROUND_CYAN_SUBTLE_DEFAULT = "background.cyan.subtle.default": Color ->
        primitive::COLOR_CYAN_1400;

    /// Vivid cyan background for data visualization.
    BACKGROUND_CYAN_VISUAL_DEFAULT = "background.cyan.visual.default": Color ->
        primitive::COLOR_CYAN_500;

    /// Fuchsia background for tagged or categorized elements.
    BACKGROUND_FUCHSIA_DEFAULT = "background.fuchsia.default": Color ->
        primitive::COLOR_FUCHSIA_600;

    /// Subtle fuchsia background for tagged or categorized elements.
    BACKGROUND_FUCHSIA_SUBTLE_DEFAULT = "background.fuchsia.subtle.default": Color ->
        primitive::COLOR_FUCHSIA_1400;

    /// Vivid fuchsia background for data visualization.
    BACKGROUND_FUCHSIA_VISUAL_DEFAULT = "background.fuchsia.visual.default": Color ->
        primitive::COLOR_FUCHSIA_500;

    /// Green background for tagged or categorized elements.
    BACKGROUND_GREEN_DEFAULT = "background.green.default": Color -> primitive::COLOR_GREEN_600;

    /// Subtle green background for tagged or categorized elements.
    BACKGROUND_GREEN_SUBTLE_DEFAULT = "background.green.subtle.default": Color ->
        primitive::COLOR_GREEN_1400;

    /// Vivid green background for data visualization.
    BACKGROUND_GREEN_VISUAL_DEFAULT = "background.green.visual.default": Color ->
        primitive::COLOR_GREEN_500;

    /// Indigo background for tagged or categorized elements.
    BACKGROUND_INDIGO_DEFAULT = "background.indigo.default": Color -> primitive::COLOR_INDIGO_600;

    /// Subtle indigo background for tagged or categorized elements.
    BACKGROUND_INDIGO_SUBTLE_DEFAULT = "background.indigo.subtle.default": Color ->
        primitive::COLOR_INDIGO_1400;

    /// Vivid indigo background for data visualization.
    BACKGROUND_INDIGO_VISUAL_DEFAULT = "background.indigo.visual.default": Color ->
        primitive::COLOR_INDIGO_500;

    /// Magenta background for tagged or categorized elements.
    BACKGROUND_MAGENTA_DEFAULT = "background.magenta.default": Color ->
        primitive::COLOR_MAGENTA_600;

    /// Subtle magenta background for tagged or categorized elements.
    BACKGROUND_MAGENTA_SUBTLE_DEFAULT = "background.magenta.subtle.default": Color ->
        primitive::COLOR_MAGENTA_1400;

    /// Vivid magenta background for data visualization.
    BACKGROUND_MAGENTA_VISUAL_DEFAULT = "background.magenta.visual.default": Color ->
        primitive::COLOR_MAGENTA_500;

    /// Orange background for tagged or categorized elements.
    BACKGROUND_ORANGE_DEFAULT = "background.orange.default": Color -> primitive::COLOR_ORANGE_600;

    /// Subtle orange background for tagged or categorized elements.
    BACKGROUND_ORANGE_SUBTLE_DEFAULT = "background.orange.subtle.default": Color ->
        primitive::COLOR_ORANGE_1400;

    /// Vivid orange background for data visualization.
    BACKGROUND_ORANGE_VISUAL_DEFAULT = "background.orange.visual.default": Color ->
        primitive::COLOR_ORANGE_500;

    /// Pink background for tagged or categorized elements.
    BACKGROUND_PINK_DEFAULT = "background.pink.default": Color -> primitive::COLOR_PINK_600;

    /// Subtle pink background for tagged or categorized elements.
    BACKGROUND_PINK_SUBTLE_DEFAULT = "background.pink.subtle.default": Color ->
        primitive::COLOR_PINK_1400;

    /// Vivid pink background for data visualization.
    BACKGROUND_PINK_VISUAL_DEFAULT = "background.pink.visual.default": Color ->
        primitive::COLOR_PINK_500;

    /// Purple background for tagged or categorized elements.
    BACKGROUND_PURPLE_DEFAULT = "background.purple.default": Color -> primitive::COLOR_PURPLE_600;

    /// Subtle purple background for tagged or categorized elements.
    BACKGROUND_PURPLE_SUBTLE_DEFAULT = "background.purple.subtle.default": Color ->
        primitive::COLOR_PURPLE_1400;

    /// Vivid purple background for data visualization.
    BACKGROUND_PURPLE_VISUAL_DEFAULT = "background.purple.visual.default": Color ->
        primitive::COLOR_PURPLE_500;

    /// Red background for tagged or categorized elements.
    BACKGROUND_RED_DEFAULT = "background.red.default": Color -> primitive::COLOR_RED_600;

    /// Subtle red background for tagged or categorized elements.
    BACKGROUND_RED_SUBTLE_DEFAULT = "background.red.subtle.default": Color ->
        primitive::COLOR_RED_1400;

    /// Vivid red background for data visualization.
    BACKGROUND_RED_VISUAL_DEFAULT = "background.red.visual.default": Color ->
        primitive::COLOR_RED_500;

    /// Seafoam background for tagged or categorized elements.
    BACKGROUND_SEAFOAM_DEFAULT = "background.seafoam.default": Color ->
        primitive::COLOR_SEAFOAM_600;

    /// Subtle seafoam background for tagged or categorized elements.
    BACKGROUND_SEAFOAM_SUBTLE_DEFAULT = "background.seafoam.subtle.default": Color ->
        primitive::COLOR_SEAFOAM_1400;

    /// Vivid seafoam background for data visualization.
    BACKGROUND_SEAFOAM_VISUAL_DEFAULT = "background.seafoam.visual.default": Color ->
        primitive::COLOR_SEAFOAM_500;

    /// Silver background for tagged or categorized elements.
    BACKGROUND_SILVER_DEFAULT = "background.silver.default": Color -> primitive::COLOR_SILVER_600;

    /// Subtle silver background for tagged or categorized elements.
    BACKGROUND_SILVER_SUBTLE_DEFAULT = "background.silver.subtle.default": Color ->
        primitive::COLOR_SILVER_1400;

    /// Vivid silver background for data visualization.
    BACKGROUND_SILVER_VISUAL_DEFAULT = "background.silver.visual.default": Color ->
        primitive::COLOR_SILVER_500;

    /// Turquoise background for tagged or categorized elements.
    BACKGROUND_TURQUOISE_DEFAULT = "background.turquoise.default": Color ->
        primitive::COLOR_TURQUOISE_600;

    /// Subtle turquoise background for tagged or categorized elements.
    BACKGROUND_TURQUOISE_SUBTLE_DEFAULT = "background.turquoise.subtle.default": Color ->
        primitive::COLOR_TURQUOISE_1400;

    /// Vivid turquoise background for data visualization.
    BACKGROUND_TURQUOISE_VISUAL_DEFAULT = "background.turquoise.visual.default": Color ->
        primitive::COLOR_TURQUOISE_500;

    /// Yellow background for tagged or categorized elements.
    BACKGROUND_YELLOW_DEFAULT = "background.yellow.default": Color -> primitive::COLOR_YELLOW_600;

    /// Subtle yellow background for tagged or categorized elements.
    BACKGROUND_YELLOW_SUBTLE_DEFAULT = "background.yellow.subtle.default": Color ->
        primitive::COLOR_YELLOW_1400;

    /// Vivid yellow background for data visualization.
    BACKGROUND_YELLOW_VISUAL_DEFAULT = "background.yellow.visual.default": Color ->
        primitive::COLOR_YELLOW_500;

    /// Gray background for tagged or categorized elements.
    BACKGROUND_GRAY_DEFAULT = "background.gray.default": Color -> primitive::COLOR_GRAY_500;

    /// Subtle gray background for tagged or categorized elements.
    BACKGROUND_GRAY_SUBTLE_DEFAULT = "background.gray.subtle.default": Color ->
        primitive::COLOR_GRAY_800;

    /// Vivid gray background for data visualization.
    BACKGROUND_GRAY_VISUAL_DEFAULT = "background.gray.visual.default": Color ->
        primitive::COLOR_GRAY_500;

    /// Color of scrollbar grabs.
    FOREGROUND_SCROLLBAR_GRAB_DEFAULT = "foreground.scrollbar-grab.default": Color ->
        primitive::COLOR_GRAY_500;
}
