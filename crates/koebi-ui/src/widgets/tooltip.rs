//! Tooltips: a line of help shown near the pointer after it rests on a control, above
//! everything.
//!
//! A tooltip shows once the pointer has rested on its widget for the input settings' tooltip
//! delay, or for the short delay right after another tooltip showed (moving along a toolbar).
//! A press hides it until the pointer moves to another widget; it never takes the pointer.

use core::time::Duration;

use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_render::RectPrimitive;
use koebi_tokens::component;

use crate::id::WidgetId;
use crate::routing::Layer;
use crate::text::Align;
use crate::ui::Ui;
use crate::widgets::parts::Placement;

/// Colors and metrics of tooltips, from the `component.tooltip.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TooltipStyle {
    /// Background.
    pub background: Color,
    /// Text color.
    pub label: Color,
    /// Border color.
    pub border_color: Color,
    /// Border width.
    pub border_width: f32,
    /// Corner radius.
    pub corner_radius: f32,
    /// Space between the sides and the text.
    pub padding_horizontal: f32,
    /// Space above and below the text.
    pub padding_vertical: f32,
    /// Distance between the pointer and the tooltip.
    pub offset: f32,
}

impl TooltipStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        Self {
            background: tokens.get(c::TOOLTIP_BACKGROUND_COLOR_DEFAULT),
            label: tokens.get(c::TOOLTIP_LABEL_COLOR_DEFAULT),
            border_color: tokens.get(c::TOOLTIP_BORDER_COLOR_DEFAULT),
            border_width: tokens.get(c::TOOLTIP_BORDER_WIDTH_DEFAULT),
            corner_radius: tokens.get(c::TOOLTIP_CORNER_RADIUS_DEFAULT),
            padding_horizontal: tokens.get(c::TOOLTIP_PADDING_HORIZONTAL_DEFAULT),
            padding_vertical: tokens.get(c::TOOLTIP_PADDING_VERTICAL_DEFAULT),
            offset: tokens.get(c::TOOLTIP_OFFSET_DEFAULT),
        }
    }
}

/// What tooltips remember between frames: the widget under the pointer since when, and the
/// last time a tooltip showed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct TooltipState {
    /// The widget on top under the pointer, and since when.
    pub(crate) target: Option<WidgetId>,
    pub(crate) since: Duration,
    /// A press hid the tooltip of the widget under the pointer.
    pub(crate) dismissed: bool,
    /// When a tooltip last showed.
    pub(crate) shown: Option<Duration>,
}

impl TooltipState {
    /// Follows the widget under the pointer at the start of a frame at `now`.
    pub(crate) fn update(&mut self, hovered: Option<WidgetId>, pressed: bool, now: Duration) {
        if hovered != self.target {
            self.target = hovered;
            self.since = now;
            self.dismissed = false;
        }
        if pressed {
            self.dismissed = true;
        }
    }
}

impl Ui<'_> {
    /// Shows `text` near the pointer once it has rested on the widget `id`, kept inside
    /// `bounds` (the window). Call it after the widget, every frame.
    pub fn tooltip(&mut self, id: WidgetId, text: &str, bounds: Rect) {
        let now = self.input().time();
        let tips = self.state.tooltip;
        if tips.target != Some(id) || tips.dismissed || self.state.active().is_some() {
            return;
        }
        let Some(pointer) = self.input().pointer() else {
            return;
        };
        let input = &self.styles().input;
        let recent = tips
            .shown
            .is_some_and(|shown| now.saturating_sub(shown) <= input.tooltip_delay);
        let delay = if recent {
            input.tooltip_delay_short
        } else {
            input.tooltip_delay
        };
        let due = tips.since.saturating_add(delay);
        if now < due {
            self.request_redraw_at(due);
            return;
        }
        self.state.tooltip.shown = Some(now);
        let styles = self.styles();
        let style = &styles.tooltip;
        let font = &styles.text.body;
        let size = Vec2::new(
            self.text_width(text, font) + 2.0 * (style.padding_horizontal + style.border_width),
            self.line_height(font) + 2.0 * (style.padding_vertical + style.border_width),
        );
        // Below the pointer, or above it near the bottom of the window.
        let reach = Rect::from_min_max(pointer, pointer + Vec2::new(0.0, style.offset));
        let rect = Placement::BELOW
            .place(reach, Vec2::new(size.x.ceil(), size.y.ceil()), bounds)
            .rect;
        self.on_layer(Layer::OVERLAY, |ui| {
            ui.scene().push_rect(
                &RectPrimitive::new(rect, style.background)
                    .with_corner_radius(style.corner_radius)
                    .with_border(style.border_width, style.border_color),
            );
            let inner = rect.inset(style.border_width);
            let text_rect = Rect::from_min_max(
                Vec2::new(inner.min.x + style.padding_horizontal, inner.min.y),
                Vec2::new(inner.max.x - style.padding_horizontal, inner.max.y),
            );
            ui.label(text, text_rect, font, style.label, Align::Start);
        });
    }
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;
    use koebi_text::Fonts;

    use super::*;
    use crate::{Input, Styles, UiState};

    const BUTTON: WidgetId = WidgetId::new("button");
    const OTHER: WidgetId = WidgetId::new("other");

    fn bounds() -> Rect {
        Rect::from_origin_size(Vec2::ZERO, Vec2::new(300.0, 200.0))
    }

    /// Two widgets side by side, each with a tooltip; returns the frame's scene.
    fn frame(state: &mut UiState, input: &mut Input, styles: &Styles) -> Scene {
        let mut scene = Scene::new();
        {
            let mut ui = Ui::new(&mut scene, styles, input, state);
            ui.interact(
                BUTTON,
                Rect::from_origin_size(Vec2::ZERO, Vec2::new(40.0, 20.0)),
            );
            ui.tooltip(BUTTON, "Adds a layer", bounds());
            ui.interact(
                OTHER,
                Rect::from_origin_size(Vec2::new(40.0, 0.0), Vec2::new(40.0, 20.0)),
            );
            ui.tooltip(OTHER, "Removes the layer", bounds());
        }
        input.end_frame();
        scene
    }

    #[test]
    fn tooltips_show_after_the_delay_and_hide_on_press() {
        let styles = crate::test_styles();
        let delay = styles.input.tooltip_delay;
        let (mut state, mut input) = (UiState::new(), Input::new());
        state.set_fonts(Fonts::with_embedded());
        frame(&mut state, &mut input, &styles);
        input.pointer_moved(Vec2::new(10.0, 10.0));
        frame(&mut state, &mut input, &styles);
        let scene = frame(&mut state, &mut input, &styles);
        assert_eq!(scene.glyph_count(), 0);
        assert_eq!(state.redraw_at(), Some(delay));
        input.set_time(delay);
        let scene = frame(&mut state, &mut input, &styles);
        assert!(scene.glyph_count() > 5, "{}", scene.glyph_count());
        // A press hides it until the pointer moves to another widget.
        input.primary_button(true, Vec2::new(10.0, 10.0));
        input.primary_button(false, Vec2::new(10.0, 10.0));
        frame(&mut state, &mut input, &styles);
        input.set_time(delay * 3);
        assert_eq!(frame(&mut state, &mut input, &styles).glyph_count(), 0);
    }

    #[test]
    fn the_next_tooltip_comes_sooner() {
        let styles = crate::test_styles();
        let input_style = styles.input;
        let (mut state, mut input) = (UiState::new(), Input::new());
        state.set_fonts(Fonts::with_embedded());
        frame(&mut state, &mut input, &styles);
        input.pointer_moved(Vec2::new(10.0, 10.0));
        frame(&mut state, &mut input, &styles);
        input.set_time(input_style.tooltip_delay);
        frame(&mut state, &mut input, &styles);
        // Onto the neighbour: its tooltip after the short delay.
        input.pointer_moved(Vec2::new(50.0, 10.0));
        frame(&mut state, &mut input, &styles);
        input.set_time(input_style.tooltip_delay + input_style.tooltip_delay_short);
        let scene = frame(&mut state, &mut input, &styles);
        assert!(scene.glyph_count() > 5);
    }
}
