//! The timeline.

use super::*;
use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group TIMELINE: Component, start = NODE_CABLE::END;

    /// Timeline: height of the frame ruler.
    TIMELINE_RULER_HEIGHT = "timeline.ruler.height": Dimension -> semantic::SIZE_CONTROL_HEIGHT;
    /// Timeline: background of the frame ruler.
    TIMELINE_RULER_BACKGROUND = "timeline.ruler.background": Color ->
        semantic::BACKGROUND_RAISED_DEFAULT;
    /// Timeline: ticks of the frame ruler and lines between frames.
    TIMELINE_RULER_TICK = "timeline.ruler.tick": Color -> semantic::BORDER_COLOR_DEFAULT;
    /// Timeline: frame numbers.
    TIMELINE_RULER_LABEL = "timeline.ruler.label": Color -> semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Timeline: the current frame's line and its number's background.
    TIMELINE_PLAYHEAD_COLOR = "timeline.playhead.color": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Timeline: the current frame's number.
    TIMELINE_PLAYHEAD_LABEL = "timeline.playhead.label": Color ->
        semantic::STATIC_TEXT_COLOR_WHITE;
    /// Timeline: keyframes.
    TIMELINE_KEYFRAME_COLOR = "timeline.keyframe.color": Color ->
        semantic::ICON_COLOR_SECONDARY_DEFAULT;
    /// Timeline: side of a keyframe.
    TIMELINE_KEYFRAME_SIZE = "timeline.keyframe.size": Dimension = px(8.0), range(2.0, 32.0);
    /// Timeline: width of a frame at the default zoom.
    TIMELINE_FRAME_WIDTH = "timeline.frame.width": Dimension = px(12.0), range(1.0, 200.0);
    /// Timeline: the frames outside the scene's range.
    TIMELINE_RANGE_DIM = "timeline.range.dim": Color -> semantic::BACKGROUND_DIM_DEFAULT;
}
