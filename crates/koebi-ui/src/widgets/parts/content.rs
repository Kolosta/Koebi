//! [`Content`]: what a control shows, in order: a leading icon, a label or an icon, a status
//! dot and a trailing icon; measured to size the control, laid out and drawn inside it.

use koebi_core::{Color, Rect, Vec2};
use koebi_render::RectPrimitive;

use crate::icon::Icon;
use crate::text::{Align, FontStyle};
use crate::ui::Ui;

/// What a control shows, left to right: each part is optional. By default, nothing, centered.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Content<'a> {
    /// An icon before everything.
    pub leading: Option<Icon>,
    /// The label.
    pub label: Option<&'a str>,
    /// An icon in place of, or after, the label.
    pub icon: Option<Icon>,
    /// A status dot of this color, after the label.
    pub dot: Option<Color>,
    /// An icon after everything: a chevron, a shortcut hint.
    pub trailing: Option<Icon>,
    /// Where the content sits when the control is wider than it.
    pub align: Align,
}

/// The sizes content is laid out with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ContentMetrics<'f> {
    pub(crate) gap: f32,
    pub(crate) icon_size: f32,
    pub(crate) stroke_width: f32,
    pub(crate) dot_size: f32,
    pub(crate) font: &'f FontStyle,
}

/// One part of laid-out content.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Part<'a> {
    Icon(Icon),
    Label(&'a str, f32),
    Dot(Color),
}

impl Default for Content<'_> {
    fn default() -> Self {
        Self {
            leading: None,
            label: None,
            icon: None,
            dot: None,
            trailing: None,
            align: Align::Center,
        }
    }
}

impl<'a> Content<'a> {
    /// A label.
    pub const fn label(label: &'a str) -> Self {
        Self {
            leading: None,
            label: Some(label),
            icon: None,
            dot: None,
            trailing: None,
            align: Align::Center,
        }
    }

    /// An icon alone.
    pub const fn icon(icon: Icon) -> Self {
        Self {
            leading: None,
            label: None,
            icon: Some(icon),
            dot: None,
            trailing: None,
            align: Align::Center,
        }
    }

    /// `true` when the content is one icon and nothing else.
    pub(crate) fn is_icon_only(&self) -> bool {
        let icons = [self.leading, self.icon, self.trailing]
            .iter()
            .filter(|icon| icon.is_some())
            .count();
        icons == 1 && self.label.is_none() && self.dot.is_none()
    }

    /// `true` when nothing shows.
    pub fn is_empty(&self) -> bool {
        self.leading.is_none()
            && self.label.is_none()
            && self.icon.is_none()
            && self.dot.is_none()
            && self.trailing.is_none()
    }

    /// The parts in order, the label with its natural width; calls `each` with each part.
    fn parts(&self, ui: &mut Ui<'_>, metrics: &ContentMetrics<'_>, mut each: impl FnMut(Part<'a>)) {
        if let Some(icon) = self.leading {
            each(Part::Icon(icon));
        }
        if let Some(label) = self.label {
            let width = ui.text_width(label, metrics.font);
            each(Part::Label(label, width));
        }
        if let Some(icon) = self.icon {
            each(Part::Icon(icon));
        }
        if let Some(color) = self.dot {
            each(Part::Dot(color));
        }
        if let Some(icon) = self.trailing {
            each(Part::Icon(icon));
        }
    }

    /// The width of the content, gaps between parts included.
    pub(crate) fn width(&self, ui: &mut Ui<'_>, metrics: &ContentMetrics<'_>) -> f32 {
        let (mut width, mut count) = (0.0, 0_u16);
        self.parts(ui, metrics, |part| {
            width += part_width(part, metrics);
            count += 1;
        });
        width + metrics.gap * f32::from(count.saturating_sub(1))
    }

    /// The height of the content: its tallest part, a label counting the line box of its font
    /// ([`Ui::line_height`]).
    pub(crate) fn height(&self, ui: &mut Ui<'_>, metrics: &ContentMetrics<'_>) -> f32 {
        let mut height = 0.0_f32;
        if self.leading.is_some() || self.icon.is_some() || self.trailing.is_some() {
            height = height.max(metrics.icon_size);
        }
        if self.label.is_some() {
            height = height.max(ui.line_height(metrics.font));
        }
        if self.dot.is_some() {
            height = height.max(metrics.dot_size);
        }
        height
    }

    /// Draws the content inside `rect` in `color`, placed by its alignment; a label too long
    /// for the room left is cut with an ellipsis, and content that still does not fit (icons
    /// wider or taller than `rect`) is clipped to `bounds`.
    pub(crate) fn draw(
        &self,
        ui: &mut Ui<'_>,
        rect: Rect,
        bounds: Rect,
        color: Color,
        metrics: &ContentMetrics<'_>,
    ) {
        let mut parts = [None; 5];
        let (mut count, mut natural, mut label) = (0_u16, 0.0, 0.0);
        self.parts(ui, metrics, |part| {
            parts[usize::from(count)] = Some(part);
            count += 1;
            natural += part_width(part, metrics);
            if let Part::Label(_, width) = part {
                label = width;
            }
        });
        natural += metrics.gap * f32::from(count.saturating_sub(1));
        // The label gives up what does not fit.
        let fixed = natural - label;
        let room = (rect.width() - fixed).max(0.0);
        let width = natural.min(rect.width());
        let overflows =
            fixed > rect.width() + 0.5 || self.height(ui, metrics) > rect.height() + 0.5;
        if overflows {
            ui.scene().push_clip(bounds);
        }
        let mut x = match self.align {
            Align::Start => rect.min.x,
            Align::Center => rect.min.x + (rect.width() - width) * 0.5,
            Align::End => rect.max.x - width,
        }
        .round();
        let mut first = true;
        for part in parts.into_iter().flatten() {
            if !first {
                x += metrics.gap;
            }
            first = false;
            match part {
                Part::Icon(icon) => {
                    let slot = Rect::from_min_max(
                        Vec2::new(x, rect.min.y),
                        Vec2::new(x + metrics.icon_size, rect.max.y),
                    );
                    let bounds = Icon::centered(slot, metrics.icon_size);
                    icon.draw(ui.scene(), bounds, color, metrics.stroke_width);
                    x += metrics.icon_size;
                }
                Part::Label(label, natural_width) => {
                    let width = natural_width.min(room);
                    let slot = Rect::from_min_max(
                        Vec2::new(x, rect.min.y),
                        Vec2::new(x + width, rect.max.y),
                    );
                    ui.label(label, slot, metrics.font, color, Align::Start);
                    x += width;
                }
                Part::Dot(dot) => {
                    let center = Vec2::new(x + metrics.dot_size * 0.5, rect.center().y);
                    let bounds = Rect::from_center_size(center, Vec2::splat(metrics.dot_size));
                    ui.scene().push_rect(
                        &RectPrimitive::new(bounds, dot).with_corner_radius(metrics.dot_size * 0.5),
                    );
                    x += metrics.dot_size;
                }
            }
        }
        if overflows {
            ui.scene().pop_clip();
        }
    }
}

/// The width of one part.
fn part_width(part: Part<'_>, metrics: &ContentMetrics<'_>) -> f32 {
    match part {
        Part::Icon(_) => metrics.icon_size,
        Part::Label(_, width) => width,
        Part::Dot(_) => metrics.dot_size,
    }
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;
    use koebi_text::Fonts;

    use super::*;
    use crate::{Input, UiState};

    #[test]
    fn content_measures_its_parts_and_gaps() {
        let styles = crate::test_styles();
        let (mut scene, mut state, input) = (Scene::new(), UiState::new(), Input::new());
        state.set_fonts(Fonts::with_embedded());
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        let metrics = ContentMetrics {
            gap: 4.0,
            icon_size: 16.0,
            stroke_width: 1.5,
            dot_size: 6.0,
            font: &styles.text.label,
        };
        assert_eq!(Content::default().width(&mut ui, &metrics), 0.0);
        assert_eq!(Content::icon(Icon::Plus).width(&mut ui, &metrics), 16.0);
        let label = ui.text_width("Save", &styles.text.label);
        let full = Content {
            leading: Some(Icon::Check),
            dot: Some(Color::WHITE),
            trailing: Some(Icon::ChevronDown),
            ..Content::label("Save")
        };
        assert!(
            (full.width(&mut ui, &metrics) - (16.0 + label + 6.0 + 16.0 + 3.0 * 4.0)).abs() < 1e-3
        );
        // A label counts the line box of its font.
        let line = ui.line_height(&styles.text.label);
        assert_eq!(full.height(&mut ui, &metrics), line.max(16.0));
        assert_eq!(Content::label("Save").height(&mut ui, &metrics), line);
        assert_eq!(Content::icon(Icon::Plus).height(&mut ui, &metrics), 16.0);
        // Drawn in a rectangle too narrow, the label is cut and the icons stay.
        let narrow = Rect::from_origin_size(Vec2::ZERO, Vec2::new(50.0, 20.0));
        full.draw(&mut ui, narrow, narrow, Color::WHITE, &metrics);
        assert_eq!(ui.scene().clip(), None);
        drop(ui);
        assert_eq!(scene.line_count(), 4);
        assert_eq!(scene.rect_count(), 1);
    }

    #[test]
    fn content_that_cannot_fit_is_clipped_to_its_bounds() {
        let styles = crate::test_styles();
        let (mut scene, mut state, input) = (Scene::new(), UiState::new(), Input::new());
        state.set_fonts(Fonts::with_embedded());
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        let metrics = ContentMetrics {
            gap: 4.0,
            icon_size: 16.0,
            stroke_width: 1.5,
            dot_size: 6.0,
            font: &styles.text.label,
        };
        let icons = Content {
            leading: Some(Icon::Check),
            trailing: Some(Icon::ChevronDown),
            ..Content::icon(Icon::Plus)
        };
        // Three icons and two gaps need 56 pixels: drawn from the left of a 10 pixel wide
        // button, the second and third icons fall outside it and are clipped away.
        let rect = Rect::from_origin_size(Vec2::ZERO, Vec2::new(10.0, 20.0));
        icons.draw(&mut ui, rect, rect, Color::WHITE, &metrics);
        // The clip ends with the content.
        assert_eq!(ui.scene().clip(), None);
        drop(ui);
        // The check mark's two segments only.
        assert_eq!(scene.line_count(), 2);
    }
}
