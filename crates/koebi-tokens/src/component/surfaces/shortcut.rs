//! Shortcut states: conflicts and recording.

use super::*;
use crate::semantic;

koebi_design::tokens! {
    group SHORTCUT: Component, start = SHORTCUT_ROW::END;

    /// Shortcut: conflict color soft.
    SHORTCUT_CONFLICT_COLOR_SOFT = "shortcut.conflict.color.soft": Color ->
        semantic::BACKGROUND_NOTICE_DEFAULT;
    /// Shortcut: conflict color hard.
    SHORTCUT_CONFLICT_COLOR_HARD = "shortcut.conflict.color.hard": Color ->
        semantic::BACKGROUND_NEGATIVE_DEFAULT;
    /// Shortcut: recording color.
    SHORTCUT_RECORDING_COLOR_DEFAULT = "shortcut.recording.color.default": Color ->
        semantic::BACKGROUND_NEGATIVE_DEFAULT;
    /// Shortcut: capture background color.
    SHORTCUT_CAPTURE_BACKGROUND_COLOR_DEFAULT =
        "shortcut.capture.background.color.default": Color -> semantic::BACKGROUND_RAISED_DEFAULT;
}
