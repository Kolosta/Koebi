//! The frame-time overlay, at the bottom left of the window above the status bar: how many
//! frames per second the application could draw, and the CPU and GPU time of a frame
//! ([`FrameTimes`]), on a slightly translucent background (`component.status-bar.frame-times.*`).
//!
//! Its text is formatted into fixed buffers and laid out again only when the averages or the
//! typography tokens change, so the overlay allocates nothing in steady state and costs one
//! rectangle and a few dozen glyphs per frame.

use core::fmt::{self, Write as _};

use koebi_core::{Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_render::{RectPrimitive, Scene};
use koebi_text::{Fonts, TextLine, TextStyle};
use koebi_tokens::{component, semantic};

use crate::frame_times::{Averages, FrameTimes};

/// Characters per line at most.
const LINE_LENGTH: usize = 16;

/// A line of text in a fixed buffer: formatting into it never allocates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Line {
    bytes: [u8; LINE_LENGTH],
    len: usize,
}

impl Line {
    const EMPTY: Self = Self {
        bytes: [0; LINE_LENGTH],
        len: 0,
    };

    fn text(&self) -> &str {
        // Only ASCII is ever written, so the bytes are always valid UTF-8.
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or_default()
    }
}

impl fmt::Write for Line {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for byte in text.bytes() {
            let slot = self.bytes.get_mut(self.len).ok_or(fmt::Error)?;
            *slot = byte;
            self.len += 1;
        }
        Ok(())
    }
}

/// Writes `duration` in milliseconds with three significant digits or so.
fn milliseconds(line: &mut Line, duration: core::time::Duration) -> fmt::Result {
    let ms = duration.as_secs_f64() * 1000.0;
    if ms < 10.0 {
        write!(line, "{ms:.2} ms")
    } else if ms < 100.0 {
        write!(line, "{ms:.1} ms")
    } else {
        write!(line, "{ms:.0} ms")
    }
}

/// The three lines of the overlay for `averages`.
fn lines(averages: Option<Averages>) -> [Line; 3] {
    let mut lines = [Line::EMPTY; 3];
    let [fps, cpu, gpu] = &mut lines;
    // A line too long for its buffer keeps what fits: the overlay never fails a frame.
    let _truncated = match averages {
        Some(averages) => write!(fps, "{:.0} FPS", averages.frames_per_second())
            .and_then(|()| write!(cpu, "CPU "))
            .and_then(|()| milliseconds(cpu, averages.cpu))
            .and_then(|()| write!(gpu, "GPU "))
            .and_then(|()| match averages.gpu {
                Some(time) => milliseconds(gpu, time),
                None => write!(gpu, "-"),
            }),
        None => write!(fps, "- FPS")
            .and_then(|()| write!(cpu, "CPU -"))
            .and_then(|()| write!(gpu, "GPU -")),
    };
    lines
}

/// The overlay's background: its color at its opacity.
fn background(design: &DesignSystem) -> koebi_core::Color {
    let color = design.get(component::STATUS_BAR_FRAME_TIMES_BACKGROUND_COLOR_DEFAULT);
    let opacity = design.get(component::STATUS_BAR_FRAME_TIMES_OPACITY_DEFAULT);
    color.with_alpha(color.a * opacity)
}

/// Whether the overlay is shown, shared by the windows, and how many times that changed.
///
/// Every main window draws the overlay, and a window draws only when something it shows
/// changes: the count of changes is part of [`Interface::revision`](crate::interface::Interface::revision),
/// so that turning the overlay on or off from one window (the settings, F12) redraws all of them.
#[derive(Clone, Debug)]
pub(crate) struct OverlaySwitch {
    overlay: Option<Overlay>,
    changes: u64,
}

impl OverlaySwitch {
    pub(crate) fn new(shown: bool) -> Self {
        Self {
            overlay: shown.then(Overlay::new),
            changes: 0,
        }
    }

    /// `true` while the overlay is shown.
    pub(crate) fn is_shown(&self) -> bool {
        self.overlay.is_some()
    }

    /// Shows or hides the overlay; counts a change only when it changes.
    pub(crate) fn set(&mut self, shown: bool) {
        if shown != self.is_shown() {
            self.overlay = shown.then(Overlay::new);
            self.changes += 1;
        }
    }

    /// Shows the overlay when hidden, hides it when shown.
    pub(crate) fn toggle(&mut self) {
        self.set(!self.is_shown());
    }

    /// How many times the overlay was shown or hidden.
    pub(crate) fn changes(&self) -> u64 {
        self.changes
    }

    /// The overlay, when shown.
    pub(crate) fn overlay_mut(&mut self) -> Option<&mut Overlay> {
        self.overlay.as_mut()
    }
}

/// The overlay's text, formatted and laid out again only when it changes.
#[derive(Clone, Debug)]
pub(crate) struct Overlay {
    lines: [Line; 3],
    laid_out: [TextLine; 3],
    revision: Option<u64>,
    style: Option<TextStyle>,
}

impl Overlay {
    pub(crate) fn new() -> Self {
        Self {
            lines: lines(None),
            laid_out: Default::default(),
            revision: None,
            style: None,
        }
    }

    /// Draws the overlay at the bottom left of `area` (the room above the status bar), over
    /// everything already in `scene`, on its translucent background, in the body text's font.
    pub(crate) fn draw(
        &mut self,
        scene: &mut Scene,
        design: &DesignSystem,
        fonts: &Fonts,
        times: &FrameTimes,
        area: Rect,
    ) {
        let family = design.get(semantic::FONT_FAMILY_BODY);
        let weight = design.get(semantic::FONT_WEIGHT_BODY_M).get();
        let Some(font) = fonts.select(&family, weight) else {
            return;
        };
        let size = design.get(semantic::FONT_SIZE_DETAIL_S);
        let style = TextStyle {
            font,
            size,
            tracking: 0.0,
        };
        if self.revision != Some(times.shown_revision()) || self.style != Some(style) {
            self.lines = lines(times.shown());
            self.revision = Some(times.shown_revision());
            self.style = Some(style);
            for (line, laid_out) in self.lines.iter().zip(&mut self.laid_out) {
                laid_out.layout(fonts, line.text(), &style);
            }
        }
        let margin = design.get(component::STATUS_BAR_FRAME_TIMES_MARGIN_DEFAULT);
        let padding = Vec2::new(
            design.get(component::STATUS_BAR_FRAME_TIMES_PADDING_HORIZONTAL_DEFAULT),
            design.get(component::STATUS_BAR_FRAME_TIMES_PADDING_VERTICAL_DEFAULT),
        );
        let line_height = (size * design.get(semantic::FONT_LINE_HEIGHT_DETAIL_S)).round();
        let widest = self
            .laid_out
            .iter()
            .map(TextLine::width)
            .fold(0.0, f32::max);
        let text = Vec2::new(widest.ceil(), line_height * self.laid_out.len() as f32);
        let size = text + padding * 2.0;
        let min = Vec2::new(area.min.x + margin, area.max.y - margin - size.y);
        let panel = Rect::from_origin_size(
            Vec2::new(min.x.round(), min.y.round()),
            Vec2::new(size.x.round(), size.y.round()),
        );
        scene.push_rect(
            &RectPrimitive::new(panel, background(design)).with_corner_radius(
                design.get(component::STATUS_BAR_FRAME_TIMES_CORNER_RADIUS_DEFAULT),
            ),
        );
        let ink = design.get(component::STATUS_BAR_FRAME_TIMES_LABEL_COLOR_DEFAULT);
        let origin = panel.min + padding;
        for (index, line) in self.laid_out.iter().enumerate() {
            let top = origin.y + line_height * index as f32;
            let baseline = line.centered_baseline(top, line_height);
            scene.push_text(line, Vec2::new(origin.x, baseline), ink);
        }
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;
    use std::time::Instant;

    use koebi_tokens::KOEBI;

    use super::*;

    fn text(line: &Line) -> &str {
        line.text()
    }

    #[test]
    fn lines_show_frames_per_second_and_milliseconds() {
        let averages = Averages {
            cpu: Duration::from_micros(1_250),
            gpu: Some(Duration::from_micros(400)),
        };
        let [fps, cpu, gpu] = lines(Some(averages));
        assert_eq!(text(&fps), "800 FPS");
        assert_eq!(text(&cpu), "CPU 1.25 ms");
        assert_eq!(text(&gpu), "GPU 0.40 ms");
        let [fps, _, gpu] = lines(Some(Averages {
            cpu: Duration::from_millis(125),
            gpu: None,
        }));
        assert_eq!((text(&fps), text(&gpu)), ("8 FPS", "GPU -"));
        assert_eq!(text(&lines(None)[0]), "- FPS");
    }

    #[test]
    fn the_overlay_sits_at_the_bottom_left() {
        let design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
        let mut times = FrameTimes::new();
        let start = Instant::now();
        times.record(Duration::from_millis(1), None, start);
        times.record(
            Duration::from_millis(1),
            None,
            start + Duration::from_secs(1),
        );
        let mut overlay = Overlay::new();
        let mut scene = Scene::new();
        let area = Rect::from_origin_size(Vec2::ZERO, Vec2::new(800.0, 600.0));
        let fonts = Fonts::with_embedded();
        overlay.draw(&mut scene, &design, &fonts, &times, area);
        assert_eq!(text(&overlay.lines[0]), "1000 FPS");
        // One panel and the glyphs of "1000 FPS", "CPU 1.00 ms" and "GPU -".
        assert_eq!(scene.rect_count(), 1);
        assert_eq!(scene.glyph_count(), 8 + 11 + 5);
        // Without fonts, the overlay shows nothing rather than failing the frame.
        let mut empty = Scene::new();
        Overlay::new().draw(&mut empty, &design, &Fonts::new(), &times, area);
        assert!(empty.is_empty());
    }

    #[test]
    fn the_switch_counts_only_changes() {
        let mut switch = OverlaySwitch::new(false);
        assert!(!switch.is_shown() && switch.overlay_mut().is_none());
        switch.set(false);
        assert_eq!(switch.changes(), 0, "hidden already");
        switch.set(true);
        assert!(switch.is_shown() && switch.overlay_mut().is_some());
        assert_eq!(switch.changes(), 1);
        switch.toggle();
        assert!(!switch.is_shown());
        assert_eq!(switch.changes(), 2);
    }

    #[test]
    fn a_window_drawn_before_the_switch_changed_draws_again() {
        // Regression: the Dev page turned the overlay on, but the main windows only drew it
        // once the pointer moved over them. The count of changes is part of what each window
        // compares with what it drew.
        let mut switch = OverlaySwitch::new(false);
        let drawn = switch.changes();
        assert!(!crate::window::is_due(false, drawn, switch.changes(), None));
        switch.set(true);
        assert!(crate::window::is_due(false, drawn, switch.changes(), None));
        assert!(crate::window::is_due(true, 0, 0, None), "asked to draw");
        assert!(
            crate::window::is_due(false, 0, 0, Some(Duration::ZERO)),
            "a delay ran out"
        );
        assert!(!crate::window::is_due(
            false,
            0,
            0,
            Some(Duration::from_millis(5))
        ));
    }

    #[test]
    fn the_background_is_translucent_and_the_text_reads_on_it_in_every_theme() {
        let mut design = DesignSystem::new(&KOEBI).expect("Koebi's schema is valid");
        let themes: Vec<_> = design.themes().map(|(id, _)| id).collect();
        for theme in themes {
            design.set_active_theme(theme).unwrap();
            let background = background(&design);
            assert!(
                background.a < 1.0,
                "{theme}: the overlay lets what lies below show"
            );
            let ink = design.get(component::STATUS_BAR_FRAME_TIMES_LABEL_COLOR_DEFAULT);
            // Over the surfaces the overlay may cover, the lightest included.
            for surface in [
                semantic::BACKGROUND_BASE_DEFAULT,
                semantic::BACKGROUND_RAISED_DEFAULT,
                semantic::BACKGROUND_APP_FRAME,
                semantic::BACKGROUND_APP_FRAME_HOVER,
                semantic::BACKGROUND_ACCENT_DEFAULT,
            ] {
                let below = background.over(design.get(surface));
                let ratio = ink.contrast_ratio(below);
                assert!(
                    ratio >= 4.5,
                    "{theme}: frame times on {surface:?}: {ratio:.2}"
                );
            }
        }
    }
}
