//! The frame-time overlay, at the bottom left of the window: how many frames per second the
//! application could draw, and the CPU and GPU time of a frame ([`FrameTimes`]).
//!
//! Koebi has no text yet: a 5 x 7 pixel font made of rectangles draws the few characters
//! the overlay needs. Its text is formatted only when the averages change, into fixed
//! buffers, so the overlay allocates nothing and costs a few dozen rectangles per frame.

use core::fmt::{self, Write as _};

use koebi_core::{Color, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_render::{RectPrimitive, Scene};
use koebi_tokens::{component, semantic};

use crate::frame_times::{Averages, FrameTimes};

/// Logical pixels per font pixel.
const PIXEL: f32 = 2.0;
/// Glyph width and height, in font pixels.
const GLYPH_WIDTH: usize = 5;
const GLYPH_HEIGHT: usize = 7;
/// Font pixels between two characters, and between two lines.
const SPACING: usize = 1;
const LINE_GAP: usize = 3;
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

    fn text(&self) -> &[u8] {
        &self.bytes[..self.len]
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

/// The rows of a character, top to bottom, five bits each, the highest bit being the leftmost
/// pixel. Characters the overlay never shows are blank.
fn glyph(character: u8) -> [u8; GLYPH_HEIGHT] {
    match character {
        b'0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        b'1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        b'2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        b'3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        b'4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        b'5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        b'6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        b'7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        b'8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        b'9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        b'.' => [0, 0, 0, 0, 0, 0b01100, 0b01100],
        b'-' => [0, 0, 0, 0b11111, 0, 0, 0],
        b'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        b'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        b'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
        b'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        b'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        b'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        b'm' => [0, 0, 0b11010, 0b10101, 0b10101, 0b10001, 0b10001],
        b's' => [0, 0, 0b01111, 0b10000, 0b01110, 0b00001, 0b11110],
        _ => [0; GLYPH_HEIGHT],
    }
}

/// Width of `length` characters, in logical pixels.
fn text_width(length: usize) -> f32 {
    (length * (GLYPH_WIDTH + SPACING)).saturating_sub(SPACING) as f32 * PIXEL
}

/// Draws `text` with its top-left corner at `origin`: one rectangle per run of lit pixels.
fn draw_text(scene: &mut Scene, origin: Vec2, text: &[u8], color: Color) {
    for (index, &character) in text.iter().enumerate() {
        let left = origin.x + (index * (GLYPH_WIDTH + SPACING)) as f32 * PIXEL;
        for (row, bits) in glyph(character).into_iter().enumerate() {
            let lit = |column: usize| bits & (1 << (GLYPH_WIDTH - 1 - column)) != 0;
            let top = origin.y + row as f32 * PIXEL;
            let mut column = 0;
            while column < GLYPH_WIDTH {
                if !lit(column) {
                    column += 1;
                    continue;
                }
                let start = column;
                while column < GLYPH_WIDTH && lit(column) {
                    column += 1;
                }
                let run = Rect::from_origin_size(
                    Vec2::new(left + start as f32 * PIXEL, top),
                    Vec2::new((column - start) as f32 * PIXEL, PIXEL),
                );
                scene.push_rect(&RectPrimitive::new(run, color));
            }
        }
    }
}

/// The overlay's text, formatted again only when the shown averages change.
#[derive(Clone, Debug)]
pub(crate) struct Overlay {
    lines: [Line; 3],
    revision: Option<u64>,
}

impl Overlay {
    pub(crate) fn new() -> Self {
        Self {
            lines: lines(None),
            revision: None,
        }
    }

    /// Draws the overlay at the bottom left of `area`, over everything already in `scene`,
    /// with the tooltip's tokens.
    pub(crate) fn draw(
        &mut self,
        scene: &mut Scene,
        design: &DesignSystem,
        times: &FrameTimes,
        area: Rect,
    ) {
        if self.revision != Some(times.shown_revision()) {
            self.lines = lines(times.shown());
            self.revision = Some(times.shown_revision());
        }
        let margin = design.get(semantic::SPACING_EDITOR_INSET);
        let padding = Vec2::new(
            design.get(component::TOOLTIP_PADDING_HORIZONTAL_DEFAULT),
            design.get(component::TOOLTIP_PADDING_VERTICAL_DEFAULT),
        );
        let border = design.get(component::TOOLTIP_BORDER_WIDTH_DEFAULT);
        let longest = self.lines.iter().map(|line| line.len).max().unwrap_or(0);
        let rows = self.lines.len();
        let text = Vec2::new(
            text_width(longest),
            (rows * GLYPH_HEIGHT + (rows - 1) * LINE_GAP) as f32 * PIXEL,
        );
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
        for (index, line) in self.lines.iter().enumerate() {
            let top = (index * (GLYPH_HEIGHT + LINE_GAP)) as f32 * PIXEL;
            let at = Vec2::new(origin.x.round(), (origin.y + top).round());
            draw_text(scene, at, line.text(), ink);
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
        core::str::from_utf8(line.text()).expect("ASCII")
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
    fn every_character_shown_has_a_glyph() {
        for character in b"0123456789.-CFGPSUms".iter().copied() {
            assert_ne!(glyph(character), [0; GLYPH_HEIGHT], "{}", character as char);
        }
        assert_eq!(glyph(b' '), [0; GLYPH_HEIGHT]);
    }

    #[test]
    fn text_is_drawn_with_one_rectangle_per_run() {
        let mut scene = Scene::new();
        // `-` is one run; `1` has runs on seven rows (its foot is one run).
        draw_text(&mut scene, Vec2::ZERO, b"-", Color::WHITE);
        assert_eq!(scene.rect_count(), 1);
        draw_text(&mut scene, Vec2::ZERO, b"1", Color::WHITE);
        assert_eq!(scene.rect_count(), 8);
        assert!((text_width(3) - 34.0).abs() < f32::EPSILON);
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
        overlay.draw(&mut scene, &design, &times, area);
        assert_eq!(text(&overlay.lines[0]), "1000 FPS");
        assert!(scene.rect_count() > 30, "{}", scene.rect_count());
    }
}
