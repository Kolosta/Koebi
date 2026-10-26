//! The Timeline template: tracks of keyframes along the frames of the scene, scrolling both
//! ways, with a frame ruler on top where a press or a drag sets the current frame.
//!
//! The current frame and the scene's range are shared by every timeline; each keeps its own
//! scroll position and zoom.

use koebi_core::input::{Key, ModifierKey};
use koebi_core::{Color, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_input::{ActionDef, ActionGroup, Context, KeyTrigger, Shortcut, Trigger};
use koebi_render::RectPrimitive;
use koebi_tokens::component;
use koebi_ui::{Align, ScrollAxes, ScrollState, Ui, WidgetId};

/// The timeline: its keys apply where the pointer is over one.
pub(crate) const TIMELINE: Context = Context::new("timeline");

const fn repeating(key: Key) -> Shortcut {
    Shortcut::new(Trigger::Key {
        key,
        when: KeyTrigger::Press,
        repeat: true,
    })
}

/// Goes to the next frame.
pub(crate) static NEXT_FRAME: ActionDef = ActionDef {
    id: "timeline.next-frame",
    label: "Next frame",
    description: "Makes the next frame current.",
    context: TIMELINE,
    defaults: &[repeating(Key::Right)],
};

/// Goes to the previous frame.
pub(crate) static PREVIOUS_FRAME: ActionDef = ActionDef {
    id: "timeline.previous-frame",
    label: "Previous frame",
    description: "Makes the previous frame current.",
    context: TIMELINE,
    defaults: &[repeating(Key::Left)],
};

/// Goes to the first frame of the scene.
pub(crate) static FIRST_FRAME: ActionDef = ActionDef {
    id: "timeline.first-frame",
    label: "First frame",
    description: "Makes the first frame of the scene current.",
    context: TIMELINE,
    defaults: &[Shortcut::key(Key::Left).with(ModifierKey::Shift)],
};

/// Goes to the last frame of the scene.
pub(crate) static LAST_FRAME: ActionDef = ActionDef {
    id: "timeline.last-frame",
    label: "Last frame",
    description: "Makes the last frame of the scene current.",
    context: TIMELINE,
    defaults: &[Shortcut::key(Key::Right).with(ModifierKey::Shift)],
};

/// Widens the frames.
pub(crate) static ZOOM_IN: ActionDef = ActionDef {
    id: "timeline.zoom-in",
    label: "Zoom in",
    description: "Widens the frames, around the current one.",
    context: TIMELINE,
    defaults: &[Shortcut::key(Key::KeypadPlus)],
};

/// Narrows the frames.
pub(crate) static ZOOM_OUT: ActionDef = ActionDef {
    id: "timeline.zoom-out",
    label: "Zoom out",
    description: "Narrows the frames, around the current one.",
    context: TIMELINE,
    defaults: &[Shortcut::key(Key::KeypadMinus)],
};

/// Shows the whole scene.
pub(crate) static VIEW_ALL: ActionDef = ActionDef {
    id: "timeline.view-all",
    label: "Frame all",
    description: "Zooms so that every frame of the scene shows.",
    context: TIMELINE,
    defaults: &[Shortcut::key(Key::Home)],
};

/// The timeline's actions, for the keymap.
pub(crate) static ACTIONS: ActionGroup = ActionGroup {
    name: "Timeline",
    actions: &[
        &NEXT_FRAME,
        &PREVIOUS_FRAME,
        &FIRST_FRAME,
        &LAST_FRAME,
        &ZOOM_IN,
        &ZOOM_OUT,
        &VIEW_ALL,
    ],
};

/// Frames shown beyond the scene's range, dimmed, on each side.
const MARGIN_FRAMES: i32 = 10;

/// The template tracks.
const TRACKS: i32 = 8;

/// The scene's frames and the current one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Playhead {
    pub(crate) current: i32,
    pub(crate) start: i32,
    pub(crate) end: i32,
}

impl Playhead {
    pub(crate) fn new() -> Self {
        Self {
            current: 1,
            start: 1,
            end: 120,
        }
    }

    /// Makes `frame` current, kept within the scene.
    fn go(&mut self, frame: i32) {
        self.current = frame.clamp(self.start, self.end);
    }

    /// The first frame shown.
    fn first(self) -> i32 {
        self.start - MARGIN_FRAMES
    }

    /// The number of frames shown.
    fn shown(self) -> i32 {
        self.end - self.start + 1 + 2 * MARGIN_FRAMES
    }
}

/// Whether track `track` has a keyframe on `frame`: a template pattern.
fn keyframe(track: i32, frame: i32) -> bool {
    let period = 6 + 3 * track;
    (frame - 1 - 2 * track).rem_euclid(period) == 0
}

/// A timeline's own view: scrolling and zoom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct View {
    scroll: ScrollState,
    /// Width of a frame, in logical pixels; `None` until the first frame takes the token's.
    frame_width: Option<f32>,
}

impl Default for View {
    fn default() -> Self {
        Self {
            scroll: ScrollState::new(),
            frame_width: None,
        }
    }
}

/// Colors and metrics of the timeline, from the `component.timeline.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Style {
    ruler: f32,
    ruler_background: Color,
    tick: Color,
    label: Color,
    playhead: Color,
    playhead_label: Color,
    keyframe: Color,
    keyframe_size: f32,
    frame_width: f32,
    dim: Color,
    zebra: [Color; 2],
    row: f32,
}

impl Style {
    fn resolve(design: &DesignSystem) -> Self {
        Self {
            ruler: design.get(component::TIMELINE_RULER_HEIGHT),
            ruler_background: design.get(component::TIMELINE_RULER_BACKGROUND),
            tick: design.get(component::TIMELINE_RULER_TICK),
            label: design.get(component::TIMELINE_RULER_LABEL),
            playhead: design.get(component::TIMELINE_PLAYHEAD_COLOR),
            playhead_label: design.get(component::TIMELINE_PLAYHEAD_LABEL),
            keyframe: design.get(component::TIMELINE_KEYFRAME_COLOR),
            keyframe_size: design.get(component::TIMELINE_KEYFRAME_SIZE),
            frame_width: design.get(component::TIMELINE_FRAME_WIDTH).max(1.0),
            dim: design.get(component::TIMELINE_RANGE_DIM),
            zebra: [
                design.get(component::TABLE_ROW_BACKGROUND_COLOR_DEFAULT),
                design.get(component::TABLE_ROW_BACKGROUND_COLOR_ALT),
            ],
            row: design.get(component::LIST_ROW_HEIGHT_DEFAULT).max(1.0),
        }
    }
}

/// The frames between two numbered ticks: the smallest of 1, 2, 5, 10, 20, 50... whose
/// ticks are at least `spacing` apart.
fn label_step(frame_width: f32, spacing: f32) -> i32 {
    let mut step = 1;
    for factor in [2, 5, 10, 20, 50, 100, 200, 500, 1000] {
        #[allow(clippy::cast_precision_loss, reason = "a few hundred frames")]
        if step as f32 * frame_width >= spacing {
            break;
        }
        step = factor;
    }
    step
}

#[allow(
    clippy::cast_precision_loss,
    reason = "frame numbers stay far below 2^24"
)]
fn frames(count: i32) -> f32 {
    count as f32
}

/// The whole number of frames in `value` (already rounded), kept to a sane range.
fn whole(value: f32) -> i32 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "clamped to a million frames either way"
    )]
    let frames = value.clamp(-1.0e6, 1.0e6) as i32;
    frames
}

/// `value` in decimal, written into `digits`.
fn integer(value: i32, digits: &mut [u8; 12]) -> &str {
    let mut rest = value.unsigned_abs();
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = b"0123456789"[usize::try_from(rest % 10).unwrap_or(0)];
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    if value < 0 {
        start -= 1;
        digits[start] = b'-';
    }
    core::str::from_utf8(&digits[start..]).expect("decimal digits are ASCII")
}

/// Draws a timeline in `rect` and applies its input.
pub(crate) fn show(
    ui: &mut Ui<'_>,
    id: WidgetId,
    rect: Rect,
    design: &DesignSystem,
    playhead: &mut Playhead,
    view: &mut View,
) {
    let style = Style::resolve(design);
    let width = *view.frame_width.get_or_insert(style.frame_width);
    let ruler = Rect::from_min_max(rect.min, Vec2::new(rect.max.x, rect.min.y + style.ruler));
    let tracks = Rect::from_min_max(Vec2::new(rect.min.x, ruler.max.y), rect.max);
    ui.zone(rect, TIMELINE, |ui| {
        keys(ui, rect, tracks, playhead, view, &style);
        let width = view.frame_width.unwrap_or(width);
        let content = Vec2::new(frames(playhead.shown()) * width, frames(TRACKS) * style.row);
        let mut scroll = view.scroll;
        let origin = ui.scroll_area(
            id,
            tracks,
            &mut scroll,
            content,
            ScrollAxes::Both,
            |ui, scrolled| {
                draw_tracks(
                    ui,
                    scrolled.origin,
                    scrolled.viewport,
                    *playhead,
                    width,
                    &style,
                );
                scrolled.origin
            },
        );
        view.scroll = scroll;
        draw_ruler(ui, id, ruler, origin.x, playhead, width, &style);
    });
}

/// The x of the middle of `frame`, for content starting at `origin`.
fn frame_x(origin: f32, playhead: Playhead, frame: i32, width: f32) -> f32 {
    origin + (frames(frame - playhead.first()) + 0.5) * width
}

fn keys(
    ui: &mut Ui<'_>,
    rect: Rect,
    tracks: Rect,
    playhead: &mut Playhead,
    view: &mut View,
    style: &Style,
) {
    if !ui
        .input()
        .pointer()
        .is_some_and(|point| rect.contains(point))
    {
        return;
    }
    let before = playhead.current;
    if ui.triggered(&NEXT_FRAME) {
        playhead.go(playhead.current + 1);
    }
    if ui.triggered(&PREVIOUS_FRAME) {
        playhead.go(playhead.current - 1);
    }
    if ui.triggered(&FIRST_FRAME) {
        playhead.go(playhead.start);
    }
    if ui.triggered(&LAST_FRAME) {
        playhead.go(playhead.end);
    }
    let width = view.frame_width.unwrap_or(style.frame_width);
    let zoom = if ui.triggered(&ZOOM_IN) {
        1.25
    } else if ui.triggered(&ZOOM_OUT) {
        0.8
    } else {
        1.0
    };
    let visible = (tracks.width() - ui.styles().scroll.margin).max(1.0);
    let mut new_width = (width * zoom).clamp(1.0, 200.0);
    if ui.triggered(&VIEW_ALL) {
        new_width = (visible / frames(playhead.shown())).clamp(1.0, 200.0);
    }
    view.frame_width = Some(new_width);
    // Keep the current frame in sight when it or the zoom changed.
    if playhead.current != before || (new_width - width).abs() > f32::EPSILON {
        let x = frame_x(0.0, *playhead, playhead.current, new_width);
        let offset = view.scroll.offset();
        let scrolled = if x < offset.x || x > offset.x + visible {
            x - visible * 0.5
        } else {
            offset.x
        };
        view.scroll
            .set_offset(Vec2::new(scrolled.max(0.0), offset.y));
    }
}

fn draw_tracks(
    ui: &mut Ui<'_>,
    origin: Vec2,
    viewport: Rect,
    playhead: Playhead,
    width: f32,
    style: &Style,
) {
    let scene = ui.scene();
    let content_width = frames(playhead.shown()) * width;
    for track in 0..TRACKS {
        let top = origin.y + frames(track) * style.row;
        let row = Rect::from_min_max(
            Vec2::new(viewport.min.x, top),
            Vec2::new(viewport.max.x, top + style.row),
        );
        let color = style.zebra[usize::try_from(track % 2).unwrap_or(0)];
        if color.a > 0.0 {
            scene.push_rect(&RectPrimitive::new(row, color));
        }
    }
    // The frames outside the scene, dimmed.
    let start = origin.x + frames(MARGIN_FRAMES) * width;
    let end = origin.x + content_width - frames(MARGIN_FRAMES) * width;
    let bottom = origin.y + frames(TRACKS) * style.row;
    for (from, to) in [(origin.x, start), (end, origin.x + content_width)] {
        scene.push_rect(&RectPrimitive::new(
            Rect::from_min_max(Vec2::new(from, origin.y), Vec2::new(to, bottom)),
            style.dim,
        ));
    }
    // Keyframes in the visible frames only.
    let first = playhead.first() + whole(((viewport.min.x - origin.x) / width).floor().max(0.0));
    let last = playhead.first() + whole(((viewport.max.x - origin.x) / width).ceil());
    for track in 0..TRACKS {
        let y = origin.y + (frames(track) + 0.5) * style.row;
        for frame in first.max(playhead.start)..=last.min(playhead.end) {
            if keyframe(track, frame) {
                let x = frame_x(origin.x, playhead, frame, width);
                let size = style.keyframe_size;
                scene.push_rect(
                    &RectPrimitive::new(
                        Rect::from_center_size(Vec2::new(x, y), Vec2::splat(size)),
                        style.keyframe,
                    )
                    .with_corner_radius(size * 0.25),
                );
            }
        }
    }
    let x = frame_x(origin.x, playhead, playhead.current, width).round();
    scene.push_rect(&RectPrimitive::new(
        Rect::from_min_max(
            Vec2::new(x - 1.0, viewport.min.y),
            Vec2::new(x + 1.0, viewport.max.y),
        ),
        style.playhead,
    ));
}

fn draw_ruler(
    ui: &mut Ui<'_>,
    id: WidgetId,
    ruler: Rect,
    origin: f32,
    playhead: &mut Playhead,
    width: f32,
    style: &Style,
) {
    // A press or a drag on the ruler sets the current frame.
    let response = ui.interact(id.with(10), ruler);
    if (response.pressed || response.held)
        && let Some(point) = ui.input().pointer()
    {
        let frame = whole(((point.x - origin) / width).floor()) + playhead.first();
        playhead.go(frame);
    }
    let font = ui.styles().text.detail.clone();
    ui.clipped(ruler, |ui| {
        ui.scene()
            .push_rect(&RectPrimitive::new(ruler, style.ruler_background));
        let step = label_step(width, 40.0);
        let first = playhead.first() + whole(((ruler.min.x - origin) / width).floor().max(0.0));
        let last = playhead.first() + whole(((ruler.max.x - origin) / width).ceil());
        let mut digits = [0; 12];
        for frame in first..=last {
            if frame.rem_euclid(step) != 0 {
                continue;
            }
            let x = frame_x(origin, *playhead, frame, width).round();
            ui.scene().push_rect(&RectPrimitive::new(
                Rect::from_min_max(
                    Vec2::new(x, ruler.max.y - ruler.height() * 0.3),
                    Vec2::new(x + 1.0, ruler.max.y),
                ),
                style.tick,
            ));
            let text = integer(frame, &mut digits);
            let label = Rect::from_min_max(
                Vec2::new(x - 30.0, ruler.min.y),
                Vec2::new(x + 30.0, ruler.max.y - ruler.height() * 0.25),
            );
            ui.label(text, label, &font, style.label, Align::Center);
        }
        // The current frame: its number on the playhead's color.
        let x = frame_x(origin, *playhead, playhead.current, width).round();
        let text = integer(playhead.current, &mut digits);
        let text_width = ui.text_width(text, &font) + 8.0;
        let tag = Rect::from_center_size(
            Vec2::new(x, ruler.center().y),
            Vec2::new(text_width.max(ruler.height()), ruler.height() - 4.0),
        );
        ui.scene().push_rect(
            &RectPrimitive::new(tag, style.playhead).with_corner_radius(tag.height() * 0.3),
        );
        ui.label(text, tag, &font, style.playhead_label, Align::Center);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_current_frame_stays_in_the_scene() {
        let mut playhead = Playhead::new();
        playhead.go(-5);
        assert_eq!(playhead.current, playhead.start);
        playhead.go(10_000);
        assert_eq!(playhead.current, playhead.end);
        assert_eq!(playhead.shown(), 120 + 2 * MARGIN_FRAMES);
    }

    #[test]
    fn frame_numbers_keep_their_distance() {
        assert_eq!(label_step(12.0, 40.0), 5);
        assert_eq!(label_step(50.0, 40.0), 1);
        assert_eq!(label_step(1.0, 40.0), 50);
        let mut digits = [0; 12];
        assert_eq!(integer(-9, &mut digits), "-9");
        assert_eq!(integer(120, &mut digits), "120");
    }

    #[test]
    fn frames_sit_side_by_side_from_the_first_shown() {
        let playhead = Playhead::new();
        let width = 12.0;
        assert_eq!(frame_x(0.0, playhead, playhead.first(), width), 6.0);
        assert_eq!(frame_x(0.0, playhead, playhead.start, width), 126.0);
        assert!(keyframe(0, 1));
        assert!(!keyframe(0, 2));
    }
}
