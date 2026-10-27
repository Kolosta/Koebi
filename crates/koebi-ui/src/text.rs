//! Labels: one line of text drawn with the typography tokens.

use std::sync::Arc;

use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_text::{TextLine, TextStyle};
use koebi_tokens::semantic;

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
        Self {
            label: FontStyle {
                family: Arc::clone(&family),
                weight: tokens.get(semantic::FONT_WEIGHT_LABEL_M).get(),
                size: tokens.get(semantic::FONT_SIZE_LABEL_M),
                tracking: tokens.get(semantic::FONT_TRACKING_LABEL_M),
            },
            body: FontStyle {
                family: Arc::clone(&family),
                weight: tokens.get(semantic::FONT_WEIGHT_BODY_M).get(),
                size: tokens.get(semantic::FONT_SIZE_BODY_M),
                tracking: tokens.get(semantic::FONT_TRACKING_BODY_M),
            },
            detail: FontStyle {
                family,
                weight: tokens.get(semantic::FONT_WEIGHT_BODY_M).get(),
                size: tokens.get(semantic::FONT_SIZE_DETAIL_S),
                tracking: 0.0,
            },
            primary: tokens.get(semantic::TEXT_COLOR_PRIMARY_DEFAULT),
            secondary: tokens.get(semantic::TEXT_COLOR_SECONDARY_DEFAULT),
            disabled: tokens.get(semantic::TEXT_COLOR_DISABLED_DEFAULT),
        }
    }
}

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

impl Ui<'_> {
    /// The text style of `font` in the fonts of the [`crate::UiState`], if it has the family.
    fn text_style(&self, font: &FontStyle) -> Option<TextStyle> {
        let id = self.state.fonts.select(&font.family, font.weight)?;
        Some(TextStyle {
            font: id,
            size: font.size,
            tracking: font.tracking,
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
        state.line.layout(&state.fonts, "", &style);
        state.line.ascent() + state.line.descent()
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
        state.line.layout(&state.fonts, text, &style);
        state.line.width()
    }

    /// Draws `text` on one line in `rect`: centred vertically, placed by `align`, cut with an
    /// ellipsis when wider than the rectangle. Returns the width drawn.
    pub fn label(
        &mut self,
        text: &str,
        rect: Rect,
        font: &FontStyle,
        color: Color,
        align: Align,
    ) -> f32 {
        let Some(style) = self.text_style(font) else {
            return 0.0;
        };
        let state = &mut *self.state;
        let line: &mut TextLine = &mut state.line;
        line.layout_truncated(&state.fonts, text, &style, rect.width());
        let width = line.width();
        let x = match align {
            Align::Start => rect.min.x,
            Align::Center => rect.min.x + (rect.width() - width) * 0.5,
            Align::End => rect.max.x - width,
        };
        let baseline = line.centered_baseline(rect.min.y, rect.height());
        self.scene.push_text(line, Vec2::new(x, baseline), color);
        width
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
}
