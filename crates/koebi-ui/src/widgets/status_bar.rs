//! The status bar: a bar at the bottom of a window, holding items one interface unit high, the
//! bar's inset above and below. On the left, the shortcuts that apply where the pointer is, each
//! as key caps and what it does, thin lines between them; on the right, text (the version).
//!
//! When the shortcuts do not all fit, the bar keeps the highest priorities; those it shows keep
//! the order they are given in (by kind), so that similar shortcuts stay in the same places.

use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_input::Shortcut;
use koebi_render::RectPrimitive;
use koebi_tokens::component;

use crate::text::Align;
use crate::ui::Ui;

/// Colors and metrics of the status bar, from the `component.status-bar.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatusBarStyle {
    /// Background.
    pub background: Color,
    /// Color of the labels and of the text at the end.
    pub label: Color,
    /// Height: what it holds, one interface unit, and its inset above and below.
    pub height: f32,
    /// Space above and below what it holds.
    pub inset: f32,
    /// Space between its sides and what it holds.
    pub padding: f32,
    /// Space on each side of the lines between shortcuts.
    pub gap: f32,
}

impl StatusBarStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        let inset = tokens.get(c::STATUS_BAR_PADDING_VERTICAL_DEFAULT);
        Self {
            background: tokens.get(c::STATUS_BAR_BACKGROUND_COLOR_DEFAULT),
            label: tokens.get(c::STATUS_BAR_LABEL_COLOR_DEFAULT),
            height: tokens.get(c::STATUS_BAR_SIZE_HEIGHT) + 2.0 * inset,
            inset,
            padding: tokens.get(c::STATUS_BAR_PADDING_HORIZONTAL_DEFAULT),
            gap: tokens.get(c::STATUS_BAR_GAP_DEFAULT),
        }
    }
}

/// A shortcut the status bar may show.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatusEntry<'a> {
    /// The shortcut, shown as key caps.
    pub shortcut: Shortcut,
    /// What it does, short.
    pub label: &'a str,
    /// How much it matters: the highest stay when room is short.
    pub priority: u8,
}

/// The most shortcuts a bar weighs.
const MAX_ENTRIES: usize = 32;

/// Which of entries `widths` wide fit in `room`, `between` apart: the highest `priorities`
/// first, the earlier first among equals.
fn fitting(widths: &[f32], priorities: &[u8], room: f32, between: f32) -> [bool; MAX_ENTRIES] {
    let count = widths.len().min(priorities.len()).min(MAX_ENTRIES);
    let mut order = [0_usize; MAX_ENTRIES];
    for (index, slot) in order.iter_mut().enumerate().take(count) {
        *slot = index;
    }
    order[..count].sort_by_key(|&index| core::cmp::Reverse(priorities[index]));
    let mut shown = [false; MAX_ENTRIES];
    let mut used = 0.0;
    let mut any = false;
    for &index in &order[..count] {
        let needed = widths[index] + if any { between } else { 0.0 };
        if used + needed <= room {
            used += needed;
            shown[index] = true;
            any = true;
        }
    }
    shown
}

impl Ui<'_> {
    /// The status bar in `rect`: as many of `entries` as fit, the highest priorities first,
    /// shown in the order given from the left, thin lines between them; `end` on the right.
    pub fn status_bar(&mut self, rect: Rect, entries: &[StatusEntry<'_>], end: &str) {
        let style = self.styles().status_bar;
        self.scene()
            .push_rect(&RectPrimitive::new(rect, style.background));
        let font = self.styles().text.detail.clone();
        let inner = Rect::from_min_max(
            Vec2::new(rect.min.x + style.padding, rect.min.y + style.inset),
            Vec2::new(
                (rect.max.x - style.padding).max(rect.min.x + style.padding),
                (rect.max.y - style.inset).max(rect.min.y + style.inset),
            ),
        );
        let end_width = self.text_width(end, &font).ceil();
        self.label(end, inner, &font, style.label, Align::End);
        // Each entry: its caps, a gap, its label.
        let label_gap = self.styles().key_caps.gap * 2.0;
        let mut widths = [0.0_f32; MAX_ENTRIES];
        let mut priorities = [0_u8; MAX_ENTRIES];
        let count = entries.len().min(MAX_ENTRIES);
        for (index, entry) in entries.iter().take(count).enumerate() {
            let caps = self.key_caps_width(&entry.shortcut);
            let label = self.text_width(entry.label, &font).ceil();
            widths[index] = caps + label_gap + label;
            priorities[index] = entry.priority;
        }
        let thickness = self.styles().separator.thickness;
        let between = 2.0 * style.gap + thickness;
        let room = inner.width() - end_width - style.gap;
        let shown = fitting(&widths[..count], &priorities[..count], room, between);
        let mut x = inner.min.x;
        let mut first = true;
        for (index, entry) in entries.iter().take(count).enumerate() {
            if !shown[index] {
                continue;
            }
            if !first {
                let line = Rect::from_origin_size(
                    Vec2::new(x + style.gap, inner.min.y),
                    Vec2::new(thickness, inner.height()),
                );
                self.separator(line);
                x += between;
            }
            first = false;
            let at = Rect::from_min_max(Vec2::new(x, inner.min.y), inner.max);
            let caps = self.key_caps(at, &entry.shortcut, Align::Start);
            let label = Rect::from_min_max(Vec2::new(x + caps + label_gap, inner.min.y), inner.max);
            self.label(entry.label, label, &font, style.label, Align::Start);
            x += widths[index];
        }
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::input::Key;
    use koebi_render::Scene;
    use koebi_text::Fonts;

    use super::*;
    use crate::{Input, UiState};

    #[test]
    fn the_highest_priorities_fit_and_keep_their_order() {
        let widths = [30.0, 30.0, 30.0, 30.0];
        let priorities = [1, 9, 5, 9];
        // Room for two, 10 apart: the two nines.
        let shown = fitting(&widths, &priorities, 70.0, 10.0);
        assert_eq!(&shown[..4], [false, true, false, true]);
        // Room for three: the five joins them.
        let shown = fitting(&widths, &priorities, 110.0, 10.0);
        assert_eq!(&shown[..4], [false, true, true, true]);
        // Everything fits.
        assert!(
            fitting(&widths, &priorities, 1000.0, 10.0)[..4]
                .iter()
                .all(|&shown| shown)
        );
        // Nothing does.
        assert!(!fitting(&widths, &priorities, 10.0, 10.0).contains(&true));
        // A narrow one still fits after a wide one is left out.
        let shown = fitting(&[80.0, 20.0], &[9, 1], 50.0, 5.0);
        assert_eq!(&shown[..2], [false, true]);
    }

    #[test]
    fn the_bar_shows_what_fits_and_its_end() {
        let styles = crate::test_styles();
        let (mut scene, mut state, input) = (Scene::new(), UiState::new(), Input::new());
        state.set_fonts(Fonts::with_embedded());
        let entries = [
            StatusEntry {
                shortcut: Shortcut::key(Key::Enter),
                label: "Confirm",
                priority: 9,
            },
            StatusEntry {
                shortcut: Shortcut::key(Key::Escape),
                label: "Cancel",
                priority: 8,
            },
        ];
        let bar = |width: f32, scene: &mut Scene, state: &mut UiState| {
            scene.clear();
            let mut ui = Ui::new(scene, &styles, &input, state);
            let rect = Rect::from_origin_size(Vec2::ZERO, Vec2::new(width, 28.0));
            ui.status_bar(rect, &entries, "Koebi 0.4");
            drop(ui);
            scene.rect_count()
        };
        let wide = bar(600.0, &mut scene, &mut state);
        let narrow = bar(120.0, &mut scene, &mut state);
        // Wide: the background, two caps and a line between them; narrow: fewer.
        assert!(wide >= 4, "{wide}");
        assert!(narrow < wide, "{narrow} against {wide}");
        // One interface unit, the bar's inset above and below: a panel header's height.
        assert_eq!(
            styles.status_bar.height,
            styles.collapsible_panel.header_height
        );
    }
}
