//! Labels and paragraphs: text drawn with the typography tokens, on one line or wrapped.

use std::sync::Arc;

use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_text::{TextLine, TextStyle};
use koebi_tokens::semantic;

use crate::id::WidgetId;
use crate::ui::Ui;

/// A font role of the typography tokens: family, weight, size and tracking.
#[derive(Clone, Debug, PartialEq)]
pub struct FontStyle {
    /// Family name, as the tokens write it (`NotoSans`).
    pub family: Arc<str>,
    /// Weight, from 1 to 1000.
    pub weight: u16,
    /// Size in logical pixels per em.
    pub size: f32,
    /// Extra space between glyphs, in ems.
    pub tracking: f32,
    /// Kerning and ligatures (`semantic.font.kerning`, `semantic.font.ligatures`).
    pub features: koebi_text::Features,
}

/// The text styles of the interface, from the `semantic.font.*` and `semantic.text.*` tokens.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyles {
    /// Labels of controls, menus and tabs.
    pub label: FontStyle,
    /// Body text.
    pub body: FontStyle,
    /// Small print: shortcuts, counts, units.
    pub detail: FontStyle,
    /// The main text color.
    pub primary: Color,
    /// Text of less importance (shortcuts, hints).
    pub secondary: Color,
    /// Text of disabled items.
    pub disabled: Color,
}

impl TextStyles {
    /// Reads the styles from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        let family = tokens.get(semantic::FONT_FAMILY_BODY);
        let features = koebi_text::Features {
            kerning: tokens.get(semantic::FONT_KERNING) >= 0.5,
            ligatures: tokens.get(semantic::FONT_LIGATURES) >= 0.5,
        };
        Self {
            label: FontStyle {
                family: Arc::clone(&family),
                weight: tokens.get(semantic::FONT_WEIGHT_LABEL_M).get(),
                size: tokens.get(semantic::FONT_SIZE_LABEL_M),
                tracking: tokens.get(semantic::FONT_TRACKING_LABEL_M),
                features,
            },
            body: FontStyle {
                family: Arc::clone(&family),
                weight: tokens.get(semantic::FONT_WEIGHT_BODY_M).get(),
                size: tokens.get(semantic::FONT_SIZE_BODY_M),
                tracking: tokens.get(semantic::FONT_TRACKING_BODY_M),
                features,
            },
            detail: FontStyle {
                family,
                weight: tokens.get(semantic::FONT_WEIGHT_BODY_M).get(),
                size: tokens.get(semantic::FONT_SIZE_DETAIL_S),
                tracking: 0.0,
                features,
            },
            primary: tokens.get(semantic::TEXT_COLOR_PRIMARY_DEFAULT),
            secondary: tokens.get(semantic::TEXT_COLOR_SECONDARY_DEFAULT),
            disabled: tokens.get(semantic::TEXT_COLOR_DISABLED_DEFAULT),
        }
    }
}

/// The distance between the lines of a paragraph, in line heights.
const LEADING: f32 = 1.3;

/// Where a label sits along its rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    /// Against the left edge.
    #[default]
    Start,
    /// Centred.
    Center,
    /// Against the right edge.
    End,
}

/// What a label does with a text wider than its rectangle ([`Ui::label_with`]). Whatever it
/// does, a label cut short shows its whole text in a tooltip.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Overflow {
    /// Cut at the last characters that fit, followed by an ellipsis ("…"): the label says it
    /// goes on.
    #[default]
    Ellipsis,
    /// Cut at the rectangle's edge, without an ellipsis: for text that scrolls or that a
    /// neighbour continues.
    Clip,
    /// Set smaller, down to `min_size` pixels per em, then cut with an ellipsis if it still
    /// does not fit: for short values in tight cells.
    Shrink {
        /// The least size the text takes.
        min_size: f32,
    },
}

impl Ui<'_> {
    /// The text style of `font` in the fonts of the [`crate::UiState`], if it has the family.
    fn text_style(&self, font: &FontStyle) -> Option<TextStyle> {
        let id = self.state.fonts.select(&font.family, font.weight)?;
        Some(TextStyle {
            font: id,
            size: font.size,
            tracking: font.tracking,
            features: font.features,
        })
    }

    /// The height of a line of text set in `font`: its ascent plus its descent, the box
    /// [`Ui::label`] centers in its rectangle; 0 without fonts. A rectangle at least this
    /// high holds any label in `font`.
    pub fn line_height(&mut self, font: &FontStyle) -> f32 {
        let Some(style) = self.text_style(font) else {
            return 0.0;
        };
        let state = &mut *self.state;
        let line = state.lines.line(&state.fonts, "", &style, None);
        line.ascent() + line.descent()
    }

    /// Lays `text` out in `font` on the shared line (`UiState::line`), for components that
    /// place a caret or hit-test characters; `false` without fonts (the line is then empty).
    pub(crate) fn lay_out(&mut self, text: &str, font: &FontStyle) -> bool {
        let style = self.text_style(font);
        let state = &mut *self.state;
        if let Some(style) = style {
            state.line.layout(&state.fonts, text, &style);
            true
        } else {
            state.line = TextLine::new();
            false
        }
    }

    /// The width of `text` set in `font`, in logical pixels; 0 without fonts.
    pub fn text_width(&mut self, text: &str, font: &FontStyle) -> f32 {
        let Some(style) = self.text_style(font) else {
            return 0.0;
        };
        let state = &mut *self.state;
        koebi_core::profile_scope!("text.width");
        state.lines.line(&state.fonts, text, &style, None).width()
    }

    /// Draws `text` on one line in `rect`: centred vertically, placed by `align`, cut with an
    /// ellipsis when wider than the rectangle. Returns the width drawn.
    ///
    /// A label cut short shows its whole text in a tooltip when the pointer rests on it, over
    /// whatever widget it lies on: nothing a caller must do. Cutting stays a last resort: lay
    /// the interface out from measured widths ([`Ui::text_width`]) so that labels fit, and see
    /// [`Ui::label_with`] for other ways to fit.
    pub fn label(
        &mut self,
        text: &str,
        rect: Rect,
        font: &FontStyle,
        color: Color,
        align: Align,
    ) -> f32 {
        self.label_with(text, rect, font, color, align, Overflow::Ellipsis)
    }

    /// [`Ui::label`], fitting a text wider than `rect` the way `overflow` says.
    pub fn label_with(
        &mut self,
        text: &str,
        rect: Rect,
        font: &FontStyle,
        color: Color,
        align: Align,
        overflow: Overflow,
    ) -> f32 {
        let Some(mut style) = self.text_style(font) else {
            return 0.0;
        };
        if let Overflow::Shrink { min_size } = overflow {
            // Widths grow with the size: the size at which the text fits, not below the least.
            let natural = self.text_width(text, font);
            if natural > rect.width() && natural > 0.0 {
                style.size = (style.size * rect.width() / natural).max(min_size.min(style.size));
            }
        }
        let limit = (overflow != Overflow::Clip).then_some(rect.width());
        let state = &mut *self.state;
        let line = {
            koebi_core::profile_scope!("text.label");
            state.lines.line(&state.fonts, text, &style, limit)
        };
        let width = line.width();
        let cut = line.is_truncated() || width > rect.width() + 0.5;
        let x = match align {
            _ if overflow == Overflow::Clip && width > rect.width() => rect.min.x,
            Align::Start => rect.min.x,
            Align::Center => rect.min.x + (rect.width() - width) * 0.5,
            Align::End => rect.max.x - width,
        };
        let baseline = line.centered_baseline(rect.min.y, rect.height());
        if overflow == Overflow::Clip && cut {
            self.scene.push_clip(rect);
            self.scene.push_text(line, Vec2::new(x, baseline), color);
            self.scene.pop_clip();
        } else {
            self.scene.push_text(line, Vec2::new(x, baseline), color);
        }
        if cut {
            self.cut_label(text, rect);
        }
        width.min(rect.width())
    }

    /// Records that the label `text` in `rect` was cut short, and shows its whole text in a
    /// tooltip while the pointer rests on it.
    fn cut_label(&mut self, text: &str, rect: Rect) {
        const CUT: WidgetId = WidgetId::new("cut-label");
        let visible = match self.clip_rect() {
            Some(clip) => clip.intersection(&rect),
            None => Some(rect),
        };
        let Some(visible) = visible else { return };
        // Its text and its place, to the pixel: the same label from frame to frame.
        let id = CUT
            .with_text(text)
            .with(u64::from(rect.min.x.round().to_bits()))
            .with(u64::from(rect.min.y.round().to_bits()));
        let layer = self.layer();
        self.state.truncated.push(id, visible, layer, text);
        self.tooltip(id, text, rect);
    }

    /// Draws `text` wrapped to the width of `rect`, line under line from its top, breaking
    /// where lines may break (between words, and between the characters of Japanese, Chinese or
    /// Korean, [`koebi_text::breaks`]); a line feed starts a new line, an empty line leaves one
    /// blank. A word wider than the rectangle breaks between two characters. Returns the height
    /// used, as [`Ui::paragraph_height`] measures it; without fonts, nothing.
    pub fn paragraph(&mut self, text: &str, rect: Rect, font: &FontStyle, color: Color) -> f32 {
        let height = self.wrap(text, rect.width(), font);
        let state = &*self.state;
        self.scene.push_block(&state.block, rect.min, color, 0.0);
        height
    }

    /// The height `text` takes wrapped to `width` in `font` ([`Ui::paragraph`]).
    pub fn paragraph_height(&mut self, text: &str, width: f32, font: &FontStyle) -> f32 {
        self.wrap(text, width, font)
    }

    /// Lays `text` out in the shared block (`UiState::block`), in lines of `width` at most, a
    /// line every `LEADING` line heights rounded to a whole pixel. Returns the height of the
    /// lines.
    fn wrap(&mut self, text: &str, width: f32, font: &FontStyle) -> f32 {
        let line = self.line_height(font);
        let Some(style) = self.text_style(font).filter(|_| line > 0.0) else {
            self.state.block = koebi_text::TextBlock::new();
            return 0.0;
        };
        let pitch = (line * LEADING).round();
        let paragraph = koebi_text::Paragraph {
            width: Some(width),
            line_height: pitch / font.size.max(1e-3),
            align: koebi_text::Align::Start,
        };
        let state = &mut *self.state;
        koebi_core::profile_scope!("text.paragraph.layout");
        state.block.layout(&state.fonts, text, &style, &paragraph);
        state.block.height()
    }
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;
    use koebi_text::Fonts;

    use super::*;
    use crate::{Input, UiState};

    fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect::from_origin_size(Vec2::new(x, y), Vec2::new(width, height))
    }

    #[test]
    fn text_styles_follow_the_typography_tokens() {
        let design =
            koebi_design::DesignSystem::new(&koebi_tokens::KOEBI).expect("Koebi's schema is valid");
        let text = TextStyles::resolve(&design);
        assert_eq!(&*text.label.family, "NotoSans");
        assert_eq!((text.label.weight, text.label.size), (500, 14.0));
        assert_eq!((text.body.weight, text.detail.size), (400, 12.0));
    }

    #[test]
    fn labels_align_and_truncate_within_their_rectangle() {
        let styles = crate::test_styles();
        let (mut scene, mut state, input) = (Scene::new(), UiState::new(), Input::new());
        state.set_fonts(Fonts::with_embedded());
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        let font = styles.text.label.clone();
        let full = ui.text_width("Viewport", &font);
        assert!(full > 40.0, "{full}");
        let wide = rect(10.0, 0.0, 200.0, 20.0);
        assert!(
            (ui.label("Viewport", wide, &font, Color::WHITE, Align::Start) - full).abs() < 1e-3
        );
        // The line box holds the ascent and the descent: more than the font size.
        let line = ui.line_height(&font);
        assert!(line > font.size && line < 2.0 * font.size, "{line}");
        let narrow = rect(0.0, 0.0, full / 2.0, 20.0);
        let cut = ui.label("Viewport", narrow, &font, Color::WHITE, Align::End);
        assert!(cut <= full / 2.0 && cut > 0.0);
        drop(ui);
        // "Viewport" (8 glyphs), then a shorter cut ending with the ellipsis.
        assert!(scene.glyph_count() > 8 && scene.glyph_count() < 16);
    }

    #[test]
    fn a_label_cut_short_shows_its_whole_text_where_the_pointer_rests() {
        let styles = crate::test_styles();
        let mut state = UiState::new();
        state.set_fonts(Fonts::with_embedded());
        let mut input = Input::new();
        let font = styles.text.label.clone();
        let text = "A label far too long for its place";
        let place = rect(0.0, 0.0, 60.0, 20.0);
        let button = WidgetId::new("button");
        let frame = |state: &mut UiState, input: &Input| {
            let mut scene = Scene::new();
            let mut ui = Ui::new(&mut scene, &styles, input, state);
            // On a button, whose label it is.
            let response = ui.interact(button, place);
            ui.label(text, place, &font, Color::WHITE, Align::Start);
            drop(ui);
            (scene, response)
        };
        frame(&mut state, &input);
        assert_eq!(state.truncated_labels().collect::<Vec<_>>(), [text]);
        input.pointer_moved(Vec2::new(30.0, 10.0));
        let (_, response) = frame(&mut state, &input);
        // The button stays hovered: the label takes nothing from it.
        assert!(response.hovered);
        assert!(state.tooltip.shown.is_none());
        // Once the pointer has rested, the tooltip holds the whole text.
        input.set_time(core::time::Duration::from_secs(5));
        let (cut, _) = frame(&mut state, &input);
        assert!(state.tooltip.shown.is_some());
        let whole = text.chars().filter(|c| !c.is_whitespace()).count();
        assert!(cut.glyph_count() > whole, "{} glyphs", cut.glyph_count());
    }

    #[test]
    fn overflow_ways_fit_a_long_text() {
        let styles = crate::test_styles();
        let (mut scene, mut state, input) = (Scene::new(), UiState::new(), Input::new());
        state.set_fonts(Fonts::with_embedded());
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        let font = styles.text.label.clone();
        let full = ui.text_width("Viewport", &font);
        let short = rect(0.0, 0.0, full * 0.6, 20.0);
        let (white, start) = (Color::WHITE, Align::Start);
        // Smaller and whole.
        let shrink = Overflow::Shrink {
            min_size: font.size * 0.5,
        };
        let drawn = ui.label_with("Viewport", short, &font, white, start, shrink);
        assert!(drawn <= short.width() + 0.5, "{drawn}");
        // Not as small as it would need: cut after all.
        let at_most = Overflow::Shrink {
            min_size: font.size,
        };
        ui.label_with("Overview", short, &font, white, start, at_most);
        // Clipped at the edge, without an ellipsis: whole glyphs, cut too.
        ui.label_with("Outliner", short, &font, white, start, Overflow::Clip);
        drop(ui);
        let cut: Vec<_> = state.truncated_labels().collect();
        assert_eq!(cut, ["Overview", "Outliner"]);
    }

    #[test]
    fn without_fonts_labels_draw_nothing() {
        let styles = crate::test_styles();
        let (mut scene, mut state, input) = (Scene::new(), UiState::new(), Input::new());
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        let font = styles.text.label.clone();
        assert_eq!(ui.text_width("Outliner", &font), 0.0);
        assert_eq!(
            ui.label(
                "Outliner",
                rect(0.0, 0.0, 100.0, 20.0),
                &font,
                Color::WHITE,
                Align::Center
            ),
            0.0
        );
        drop(ui);
        assert!(scene.is_empty());
    }

    #[test]
    fn paragraphs_wrap_between_words_and_measure_what_they_draw() {
        let styles = crate::test_styles();
        let (mut scene, mut state, input) = (Scene::new(), UiState::new(), Input::new());
        state.set_fonts(Fonts::with_embedded());
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        let font = styles.text.body.clone();
        let text = "Buttons run an action when they are clicked.";
        let one = ui.paragraph_height(text, 1000.0, &font);
        let word = ui.text_width("Buttons", &font);
        // As wide as its longest word or two: several lines, each a whole line high.
        let narrow = ui.paragraph_height(text, word * 2.2, &font);
        assert!(narrow >= 3.0 * one, "{narrow} for lines of {one}");
        assert!((narrow / one - (narrow / one).round()).abs() < 1e-4);
        // A line break starts a paragraph; an empty line leaves one blank.
        let two = ui.paragraph_height("First.\n\nSecond.", 1000.0, &font);
        assert!((two - 3.0 * one).abs() < 1e-4, "{two}");
        let drawn = ui.paragraph(text, rect(0.0, 0.0, word * 2.2, 400.0), &font, Color::WHITE);
        assert!((drawn - narrow).abs() < 1e-4);
        drop(ui);
        // Every letter drawn once: wrapping cuts nothing. Spaces draw nothing and take no
        // instance.
        let letters = text.chars().filter(|c| !c.is_whitespace()).count();
        assert_eq!(scene.glyph_count(), letters);
    }

    /// Paragraphs are laid out once and kept: drawn again, at any width, they allocate nothing
    /// (the hot-path rule of CLAUDE.md, counted by `koebi_testkit::CountingAllocator`).
    #[test]
    fn a_paragraph_drawn_again_allocates_nothing() {
        let styles = crate::test_styles();
        let (mut scene, mut state, input) = (Scene::new(), UiState::new(), Input::new());
        state.set_fonts(Fonts::with_embedded());
        let font = styles.text.body.clone();
        for text in [
            "Frames per second, CPU and GPU time of a frame, above the status bar.",
            "A message not translated yet shows in English: Koebi is written in English, the              other languages come from translators.",
        ] {
            for width in [2000.0, 300.0] {
                let mut draw = || {
                    scene.clear();
                    let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
                    ui.paragraph(text, rect(0.0, 0.0, width, 200.0), &font, Color::WHITE);
                };
                for _ in 0..3 {
                    draw();
                }
                let (count, ()) = koebi_testkit::allocations(draw);
                assert_eq!(count, 0, "{width} px: {text}");
            }
        }
    }

    /// Shaping reads the font's kerning and ligature subtables, resolved when it loads: laying
    /// a line out again, kerned pairs and ligatures included, allocates nothing.
    #[test]
    fn a_line_laid_out_again_allocates_nothing() {
        let fonts = Fonts::with_embedded();
        let font = fonts
            .select(koebi_text::embedded::NOTO_SANS, 400)
            .expect("embedded font");
        let style = TextStyle::new(font, 14.0);
        let mut line = TextLine::new();
        for text in ["abcdefghij", "a b c d e f", "office affair", "AVAWAY Te"] {
            line.layout(&fonts, text, &style);
            let (count, ()) = koebi_testkit::allocations(|| line.layout(&fonts, text, &style));
            assert_eq!(count, 0, "{text}");
        }
    }
}
