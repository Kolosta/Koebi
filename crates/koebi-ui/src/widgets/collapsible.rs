//! Collapsible panels: boxes holding trees of sections that fold, as in Blender's properties.
//!
//! - A [`CollapsiblePanel`] is a header (a chevron turning as the panel opens, an optional
//!   icon, a title, room for the caller's buttons) above content the caller draws in the
//!   rectangle it gets, measuring it: the panel keeps the height for the next frame
//!   ([`PanelState`]).
//! - **Nesting**: first-level panels lie on the window, framed and rounded. A second-level
//!   panel inside a first-level panel's content spans its full width: its header takes its
//!   parent's content color, its content the second color; a third level the third color.
//! - **Folding**: a press on the header opens or closes the panel; the content slides along
//!   `panel.expand.*` or `panel.collapse.*` (duration and curve), clipped to the share shown.

use core::f32::consts::FRAC_PI_2;
use core::time::Duration;

use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_design::value::Curve;
use koebi_render::{Corners, RectPrimitive};
use koebi_tokens::component;

use crate::icon::Icon;
use crate::id::WidgetId;
use crate::text::Align;
use crate::ui::{Response, Ui};
use crate::widgets::panel::PanelLevel;
use crate::widgets::parts::{Tween, shade};

/// Metrics, colors and motion of collapsible panels, from the `component.panel.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollapsiblePanelStyle {
    /// Height of a header.
    pub header_height: f32,
    /// Background of a first-level header; deeper headers take their parent's content color.
    pub header_background: Color,
    /// Space between a header's sides and its content.
    pub header_padding: f32,
    /// Space between a header's chevron, icon and title.
    pub header_gap: f32,
    /// Lightness added to a header under the pointer.
    pub hover_lightness: f32,
    /// Side of the chevron.
    pub chevron_size: f32,
    /// Chevron color.
    pub chevron: Color,
    /// Title color.
    pub title: Color,
    /// Space above and below the content.
    pub content_padding: f32,
    /// Gap between panels stacked one after the other.
    pub gap: f32,
    /// Duration and curve of a panel opening.
    pub expand: (Duration, Curve),
    /// Duration and curve of a panel closing.
    pub collapse: (Duration, Curve),
}

impl CollapsiblePanelStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        Self {
            header_height: tokens.get(c::PANEL_HEADER_HEIGHT_DEFAULT),
            header_background: tokens.get(c::PANEL_HEADER_BACKGROUND_COLOR_DEFAULT),
            header_padding: tokens.get(c::PANEL_HEADER_PADDING_DEFAULT),
            header_gap: tokens.get(c::PANEL_HEADER_GAP_DEFAULT),
            hover_lightness: tokens.get(c::PANEL_HEADER_HOVER_LIGHTNESS),
            chevron_size: tokens.get(c::PANEL_CHEVRON_SIZE),
            chevron: tokens.get(c::PANEL_CHEVRON_COLOR_DEFAULT),
            title: tokens.get(c::PANEL_LABEL_COLOR_DEFAULT),
            content_padding: tokens.get(c::PANEL_CONTENT_PADDING_VERTICAL),
            gap: tokens.get(c::PANEL_GAP_SIZE_DEFAULT),
            expand: (
                tokens.get(c::PANEL_EXPAND_DURATION),
                tokens.get(c::PANEL_EXPAND_CURVE),
            ),
            collapse: (
                tokens.get(c::PANEL_COLLAPSE_DURATION),
                tokens.get(c::PANEL_COLLAPSE_CURVE),
            ),
        }
    }
}

/// Whether a panel is open, how far its folding has come, and the height of its content
/// measured on the last frame. Keep one per panel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelState {
    open: bool,
    openness: Tween,
    content_height: f32,
}

impl PanelState {
    /// A panel open or closed, at rest.
    pub const fn new(open: bool) -> Self {
        Self {
            open,
            openness: Tween::new(if open { 1.0 } else { 0.0 }),
            content_height: 0.0,
        }
    }

    /// `true` when the panel is open or opening.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Opens or closes the panel; it folds on the next frames.
    pub fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    /// How far the panel is open, from 0 (closed) to 1 (open), animating between.
    pub fn openness(&self) -> f32 {
        self.openness.value().clamp(0.0, 1.0)
    }
}

impl Default for PanelState {
    /// Open.
    fn default() -> Self {
        Self::new(true)
    }
}

/// What happened to a collapsible panel this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CollapsiblePanelResponse {
    /// The pointer's interaction with the header.
    pub header: Response,
    /// The header was pressed: the panel opens or closes.
    pub toggled: bool,
    /// The height the panel took: its header and the share of its content shown.
    pub height: f32,
    /// Room kept at the end of the header for the caller's buttons, when it asked for some.
    pub trailing: Option<Rect>,
}

/// A collapsible panel, configured then shown with [`CollapsiblePanel::show`].
#[derive(Debug)]
pub struct CollapsiblePanel<'a> {
    id: WidgetId,
    state: &'a mut PanelState,
    title: &'a str,
    icon: Option<Icon>,
    level: PanelLevel,
    trailing: f32,
}

impl<'a> CollapsiblePanel<'a> {
    /// A first-level panel identified by `id`, titled `title`, folding as `state` says.
    pub fn new(id: WidgetId, state: &'a mut PanelState, title: &'a str) -> Self {
        Self {
            id,
            state,
            title,
            icon: None,
            level: PanelLevel::First,
            trailing: 0.0,
        }
    }

    /// Shows `icon` between the chevron and the title.
    #[must_use]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// How deep the panel is nested: second and third levels lie inside a panel's content.
    #[must_use]
    pub fn level(mut self, level: PanelLevel) -> Self {
        self.level = level;
        self
    }

    /// Keeps `width` at the end of the header for the caller's buttons
    /// ([`CollapsiblePanelResponse::trailing`]), drawn after the panel so that they take the
    /// pointer over the header.
    #[must_use]
    pub fn trailing(mut self, width: f32) -> Self {
        self.trailing = width.max(0.0);
        self
    }

    /// The backgrounds of the header and of the content at the panel's level.
    fn colors(&self, ui: &Ui<'_>) -> (Color, Color) {
        let panel = &ui.styles().panel;
        let style = &ui.styles().collapsible_panel;
        match self.level {
            PanelLevel::First => (style.header_background, panel.backgrounds[0]),
            PanelLevel::Second => (panel.backgrounds[0], panel.backgrounds[1]),
            PanelLevel::Third => (panel.backgrounds[1], panel.backgrounds[2]),
        }
    }

    /// The height the panel takes: its header and the share of its content shown, as measured
    /// on the last frame.
    pub fn height(&self, ui: &Ui<'_>) -> f32 {
        let style = &ui.styles().collapsible_panel;
        style.header_height + self.shown_content(ui)
    }

    /// The height of the content shown, with its padding, at the current openness.
    fn shown_content(&self, ui: &Ui<'_>) -> f32 {
        let style = &ui.styles().collapsible_panel;
        let full = if self.state.content_height > 0.0 {
            self.state.content_height + 2.0 * style.content_padding
        } else {
            0.0
        };
        (full * self.state.openness()).round()
    }

    /// Shows the panel at the top of `area`, its full width. `add` draws the content in the
    /// rectangle it gets (the full width, from the top of the content; nested panels fill it,
    /// rows inset it by `panel.padding`) and returns the height it used.
    pub fn show(
        self,
        ui: &mut Ui<'_>,
        area: Rect,
        add: impl FnOnce(&mut Ui<'_>, Rect) -> f32,
    ) -> CollapsiblePanelResponse {
        let style = ui.styles().collapsible_panel;
        let header = Rect::from_min_max(
            area.min,
            Vec2::new(area.max.x, area.min.y + style.header_height),
        );
        let response = ui.interact(self.id, header);
        if response.pressed {
            self.state.open = !self.state.open;
        }
        let (target, (duration, curve)) = if self.state.open {
            (1.0, style.expand)
        } else {
            (0.0, style.collapse)
        };
        let now = ui.input().time();
        if self.state.openness.advance(target, now, duration, &curve) {
            ui.request_redraw_after(Duration::from_millis(16));
        }
        let shown = self.shown_content(ui);
        let height = style.header_height + shown;
        let body = Rect::from_min_max(
            Vec2::new(area.min.x, header.max.y),
            Vec2::new(area.max.x, header.max.y + shown),
        );
        self.draw_box(ui, header, body, response.hovered || response.held);
        let trailing = self.draw_header(ui, header);
        // Open even a little: the content draws, clipped to the share shown, and measures
        // itself (the first frame of a panel never measured shows none of it yet).
        if self.state.openness() > 0.0 {
            let content = Rect::from_min_max(
                Vec2::new(area.min.x, body.min.y + style.content_padding),
                Vec2::new(
                    area.max.x,
                    body.min.y + style.content_padding + self.state.content_height,
                ),
            );
            let used = ui.clipped(body, |ui| add(ui, content)).max(0.0);
            if (used - self.state.content_height).abs() > 0.01 {
                // The panel takes another height: the next frame shows it at once.
                self.state.content_height = used;
                ui.request_redraw_at(now);
            }
        }
        CollapsiblePanelResponse {
            header: response,
            toggled: response.pressed,
            height,
            trailing,
        }
    }

    /// The header's and the content's backgrounds, and for a first-level panel its frame.
    fn draw_box(&self, ui: &mut Ui<'_>, header: Rect, body: Rect, hovered: bool) {
        let panel = ui.styles().panel;
        let style = &ui.styles().collapsible_panel;
        let (header_color, content_color) = self.colors(ui);
        let header_color = if hovered {
            shade(header_color, style.hover_lightness)
        } else {
            header_color
        };
        let first = self.level == PanelLevel::First;
        let radius = if first { panel.corner_radius } else { 0.0 };
        let open = body.height() > 0.0;
        let header_radii = Corners {
            top_left: radius,
            top_right: radius,
            bottom_left: if open { 0.0 } else { radius },
            bottom_right: if open { 0.0 } else { radius },
        };
        ui.scene()
            .push_rect(&RectPrimitive::new(header, header_color).with_corner_radii(header_radii));
        if open {
            let body_radii = Corners {
                top_left: 0.0,
                top_right: 0.0,
                bottom_left: radius,
                bottom_right: radius,
            };
            ui.scene()
                .push_rect(&RectPrimitive::new(body, content_color).with_corner_radii(body_radii));
        }
        if first && panel.border_width > 0.0 {
            let frame = Rect::from_min_max(
                header.min,
                Vec2::new(header.max.x, body.max.y.max(header.max.y)),
            );
            ui.scene().push_rect(
                &RectPrimitive::new(frame, Color::TRANSPARENT)
                    .with_corner_radius(radius)
                    .with_border(panel.border_width, panel.border_color),
            );
        }
    }

    /// The chevron, the icon and the title; returns the room kept for the caller's buttons.
    fn draw_header(&self, ui: &mut Ui<'_>, header: Rect) -> Option<Rect> {
        let style = ui.styles().collapsible_panel;
        let stroke = ui.styles().icon.stroke_width;
        let mut x = header.min.x + style.header_padding;
        let slot = |x: f32, side: f32| {
            Rect::from_origin_size(Vec2::new(x, header.min.y), Vec2::new(side, header.height()))
        };
        // The chevron points right while closed and down while open, turning as it folds.
        let chevron = Icon::centered(slot(x, style.chevron_size), style.chevron_size);
        Icon::ChevronRight.draw_rotated(
            ui.scene(),
            chevron,
            style.chevron,
            stroke,
            self.state.openness() * FRAC_PI_2,
        );
        x += style.chevron_size + style.header_gap;
        if let Some(icon) = self.icon {
            let side = ui.styles().icon.size;
            icon.draw(
                ui.scene(),
                Icon::centered(slot(x, side), side),
                style.title,
                stroke,
            );
            x += side + style.header_gap;
        }
        let end = header.max.x - style.header_padding;
        let trailing = (self.trailing > 0.0).then(|| {
            Rect::from_min_max(
                Vec2::new((end - self.trailing).max(x), header.min.y),
                Vec2::new(end, header.max.y),
            )
        });
        let title_end = trailing.map_or(end, |room| room.min.x - style.header_gap);
        let title = Rect::from_min_max(
            Vec2::new(x, header.min.y),
            Vec2::new(title_end.max(x), header.max.y),
        );
        let font = &ui.styles().text.label;
        ui.label(self.title, title, font, style.title, Align::Start);
        trailing
    }
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;
    use koebi_text::Fonts;

    use super::*;
    use crate::{Input, Styles, UiState};

    const ID: WidgetId = WidgetId::new("transform");

    fn area() -> Rect {
        Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::new(200.0, 400.0))
    }

    struct Harness {
        state: UiState,
        input: Input,
        styles: Styles,
    }

    impl Harness {
        fn new() -> Self {
            let mut state = UiState::new();
            state.set_fonts(Fonts::with_embedded());
            Self {
                state,
                input: Input::new(),
                styles: crate::test_styles(),
            }
        }

        fn frame<R>(&mut self, show: impl FnOnce(&mut Ui<'_>) -> R) -> (R, Scene) {
            let mut scene = Scene::new();
            let result = {
                let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
                show(&mut ui)
            };
            self.input.end_frame();
            (result, scene)
        }

        /// One frame of a panel holding 60 pixels of content.
        fn panel(&mut self, panel: &mut PanelState) -> (CollapsiblePanelResponse, bool) {
            let mut drawn = false;
            let response = self
                .frame(|ui| {
                    CollapsiblePanel::new(ID, panel, "Transform").show(ui, area(), |_, _| {
                        drawn = true;
                        60.0
                    })
                })
                .0;
            (response, drawn)
        }
    }

    #[test]
    fn panels_measure_their_content_and_fold_on_a_press() {
        let mut harness = Harness::new();
        let style = harness.styles.collapsible_panel;
        let mut panel = PanelState::new(true);
        harness.panel(&mut panel);
        let (open, drawn) = harness.panel(&mut panel);
        assert!(drawn);
        assert_eq!(
            open.height,
            style.header_height + 60.0 + 2.0 * style.content_padding
        );
        // A press on the header closes it, along the collapse curve.
        let header = area().min + Vec2::new(50.0, style.header_height * 0.5);
        harness.input.primary_button(true, header);
        let (pressed, _) = harness.panel(&mut panel);
        assert!(pressed.toggled && !panel.is_open());
        let (duration, curve) = style.collapse;
        harness.input.set_time(duration / 2);
        let (half, _) = harness.panel(&mut panel);
        let expected = 1.0 - curve.ease(0.5);
        assert!(
            (panel.openness() - expected).abs() < 1e-3,
            "{}",
            panel.openness()
        );
        assert!(half.height < open.height && half.height > style.header_height);
        assert!(harness.state.redraw_at().is_some());
        harness.input.set_time(duration * 2);
        let (closed, drawn) = harness.panel(&mut panel);
        assert_eq!(closed.height, style.header_height);
        // Closed: the content is not drawn, and its height is kept for the next opening.
        assert!(!drawn);
        assert!(!harness.panel(&mut panel).1);
    }

    #[test]
    fn nested_panels_take_their_parent_s_color_for_their_header() {
        let styles = crate::test_styles();
        let mut state = PanelState::new(true);
        let mut colors = |level| {
            let mut harness = Harness::new();
            harness
                .frame(|ui| {
                    CollapsiblePanel::new(ID, &mut state, "Delta")
                        .level(level)
                        .colors(ui)
                })
                .0
        };
        let panel = styles.panel;
        assert_eq!(
            colors(PanelLevel::First),
            (
                styles.collapsible_panel.header_background,
                panel.backgrounds[0]
            )
        );
        assert_eq!(
            colors(PanelLevel::Second),
            (panel.backgrounds[0], panel.backgrounds[1])
        );
        assert_eq!(
            colors(PanelLevel::Third),
            (panel.backgrounds[1], panel.backgrounds[2])
        );
    }

    #[test]
    fn headers_keep_room_for_buttons_and_content_is_clipped_while_folding() {
        let mut harness = Harness::new();
        let style = harness.styles.collapsible_panel;
        let mut panel = PanelState::new(false);
        let response = harness
            .frame(|ui| {
                CollapsiblePanel::new(ID, &mut panel, "Transform")
                    .icon(Icon::Plus)
                    .trailing(40.0)
                    .show(ui, area(), |_, _| 60.0)
            })
            .0;
        let trailing = response.trailing.expect("room for buttons");
        assert_eq!(trailing.max.x, area().max.x - style.header_padding);
        assert_eq!(trailing.width(), 40.0);
        // Opening: the content draws clipped to the share shown.
        panel.set_open(true);
        let (duration, _) = style.expand;
        harness.panel(&mut panel);
        harness.input.set_time(duration / 4);
        // Never measured: this frame measures the content, and asks for the next at once.
        let (_, drawn) = harness.panel(&mut panel);
        assert!(drawn);
        assert_eq!(harness.state.redraw_at(), Some(duration / 4));
        harness.input.set_time(duration / 3);
        let (opening, _) = harness.panel(&mut panel);
        assert!(opening.height > style.header_height);
        assert!(opening.height < style.header_height + 60.0);
    }
}
