//! Input settings: how far a press moves before it drags, how fast a double click is, how long
//! the pointer rests before tooltips and hover previews appear, how keys repeat. They are
//! preferences: a theme or the user can change them like any token.

use super::*;
use crate::primitive;

koebi_design::tokens! {
    group INPUT: Semantic, start = MOTION::END;

    /// Distance the pointer moves with a button down before the press becomes a drag.
    INPUT_DRAG_THRESHOLD = "input.drag-threshold": Dimension -> primitive::SPACING_100;
    /// Longest time between the two presses of a double click.
    INPUT_DOUBLE_CLICK_TIME = "input.double-click.time": Duration -> primitive::DURATION_SLOWER;
    /// Time the pointer rests on an element before its tooltip appears.
    INPUT_TOOLTIP_DELAY = "input.tooltip.delay": Duration -> primitive::DURATION_SLOWER;
    /// Time before the tooltip of the next element appears while a tooltip is shown.
    INPUT_TOOLTIP_DELAY_SHORT = "input.tooltip.delay-short": Duration -> primitive::DURATION_FAST;
    /// Time the pointer must stay still before a tooltip appears.
    INPUT_TOOLTIP_DELAY_STATIONARY = "input.tooltip.delay-stationary": Duration ->
        primitive::DURATION_SLOW;
    /// Time the pointer rests on a hover-revealed element (the band of a gutter) before it
    /// appears.
    INPUT_HOVER_DELAY = "input.hover.delay": Duration -> primitive::DURATION_SLOWER;
    /// Time a button is held before an action repeats.
    INPUT_REPEAT_DELAY = "input.repeat.delay": Duration -> primitive::DURATION_SLOWER;
    /// Time between two repetitions of a held action.
    INPUT_REPEAT_INTERVAL = "input.repeat.interval": Duration -> primitive::DURATION_FAST;
}
