//! The frame-time overlay, at the bottom left of the window: how many frames per second the
//! application could draw, and the CPU and GPU time of a frame ([`FrameTimes`]).
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

    /// Draws the overlay at the bottom left of `area`, over everything already in `scene`,
    /// with the tooltip's tokens and the body text's font.
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
        let margin = design.get(semantic::SPACING_EDITOR_INSET);
        let padding = Vec2::new(
            design.get(component::TOOLTIP_PADDING_HORIZONTAL_DEFAULT),
            design.get(component::TOOLTIP_PADDING_VERTICAL_DEFAULT),
        );
        let border = design.get(component::TOOLTIP_BORDER_WIDTH_DEFAULT);
        let line_height = (size * design.get(semantic::FONT_LINE_HEIGHT_DETAIL_S)).round();
        let widest = self
            .laid_out
            .iter()
            .map(TextLine::width)
            .fold(0.0, f32::max);
        let text = Vec2::new(widest.ceil(), line_height * self.laid_out.len() as f32);
        let size = text + (padding + Vec2::splat(border)) * 2.0;
        let min = Vec2::new(area.min.x + margin, area.max.y - margin - size.y);
        let panel = Rect::from_origin_size(
            Vec2::new(min.x.round(), min.y.round()),
            Vec2::new(size.x.round(), size.y.round()),
        );
        scene.push_rect(
            &RectPrimitive::new(
                panel,
                design.get(component::TOOLTIP_BACKGROUND_COLOR_DEFAULT),
            )
            .with_corner_radius(design.get(component::TOOLTIP_CORNER_RADIUS_DEFAULT))
            .with_border(border, design.get(component::TOOLTIP_BORDER_COLOR_DEFAULT)),
        );
        let ink = design.get(component::TOOLTIP_LABEL_COLOR_DEFAULT);
        let origin = panel.min + padding + Vec2::splat(border);
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
}
