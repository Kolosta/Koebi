//! Collapsible panels: boxes holding trees of sections that fold, as in Blender's properties.
//!
//! - A [`CollapsiblePanel`] is a header (a chevron turning as the panel opens, an optional
//!   icon, a title, room for the caller's buttons) above content the caller draws in the
//!   rectangle it gets, measuring it: the panel keeps the height for the next frame
//!   ([`CollapsiblePanelState`]). While a nested panel folds, its parent shows it as it was on
//!   the frame before: nested levels follow one frame apart.
//! - **Nesting**: first-level panels lie on the window, framed and rounded. A second-level
//!   panel inside a first-level panel's content spans its full width: its header takes its
//!   parent's content color, its content the second color; a third level the third color.
//! - **Folding**: a press on the header opens or closes the panel; the content is revealed in
//!   place along `panel.expand.*` or `panel.collapse.*` (duration and curve), clipped to the
//!   share shown.

use core::f32::consts::FRAC_PI_2;
use core::time::Duration;

use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_design::value::Curve;
use koebi_render::{Corners, RectPrimitive};
use koebi_tokens::component;

use koebi_design::kind;

use crate::icon::Icon;
use crate::id::WidgetId;
use crate::style::Prop;
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
pub struct CollapsiblePanelState {
    open: bool,
    openness: Tween,
    content_height: f32,
}

impl CollapsiblePanelState {
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

impl Default for CollapsiblePanelState {
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
    state: &'a mut CollapsiblePanelState,
    title: &'a str,
    icon: Option<Icon>,
    level: PanelLevel,
    trailing: Option<Prop<kind::Dimension>>,
}

impl<'a> CollapsiblePanel<'a> {
    /// A first-level panel identified by `id`, titled `title`, folding as `state` says.
    pub fn new(id: WidgetId, state: &'a mut CollapsiblePanelState, title: &'a str) -> Self {
        Self {
            id,
            state,
            title,
            icon: None,
            level: PanelLevel::First,
            trailing: None,
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
    pub fn trailing(mut self, width: impl Into<Prop<kind::Dimension>>) -> Self {
        self.trailing = Some(width.into());
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
    /// rectangle it gets (inside the frame, from the top of the content down to the bottom of
    /// `area`: the room there is; nested panels fill its width, rows inset it by
    /// `panel.padding`) and returns the height it used.
    pub fn show(
        self,
        ui: &mut Ui<'_>,
        area: Rect,
        add: impl FnOnce(&mut Ui<'_>, Rect) -> f32,
    ) -> CollapsiblePanelResponse {
        let style = &ui.styles().collapsible_panel;
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
        // Open or opening: the content draws, clipped to the share shown, and measures itself
        // (a panel never measured shows its content from the next frame, asked for at once).
        if self.state.open || self.state.openness() > 0.0 {
            // A first-level panel's frame stays clear of its content.
            let inset = if self.level == PanelLevel::First {
                ui.styles().panel.border_width
            } else {
                0.0
            };
            let inner = |rect: Rect| {
                Rect::from_min_max(
                    Vec2::new(rect.min.x + inset, rect.min.y),
                    Vec2::new((rect.max.x - inset).max(rect.min.x + inset), rect.max.y),
                )
            };
            let top = body.min.y + style.content_padding;
            let content = inner(Rect::from_min_max(
                Vec2::new(area.min.x, top),
                Vec2::new(area.max.x, area.max.y.max(top)),
            ));
            let used = ui.clipped(inner(body), |ui| add(ui, content));
            let used = if used.is_finite() { used.max(0.0) } else { 0.0 };
            if (used - self.state.content_height).abs() > 0.01 {
                // The panel takes another height: the next frame shows it at once.
                self.state.content_height = used;
                ui.request_redraw_at(now);
            }
        }
        // The frame, over what the content drew.
        self.draw_frame(ui, header, body);
        CollapsiblePanelResponse {
            header: response,
            toggled: response.pressed,
            height,
            trailing,
        }
    }

    /// The header's and the content's backgrounds, and for a first-level panel its frame.
    fn draw_box(&self, ui: &mut Ui<'_>, header: Rect, body: Rect, hovered: bool) {
        let panel = &ui.styles().panel;
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
    }

    /// The frame of a first-level panel, around its header and the content shown.
    fn draw_frame(&self, ui: &mut Ui<'_>, header: Rect, body: Rect) {
        let panel = &ui.styles().panel;
        if self.level != PanelLevel::First || panel.border_width <= 0.0 {
            return;
        }
        let frame = Rect::from_min_max(
            header.min,
            Vec2::new(header.max.x, body.max.y.max(header.max.y)),
        );
        ui.scene().push_rect(
            &RectPrimitive::new(frame, Color::TRANSPARENT)
                .with_corner_radius(panel.corner_radius)
                .with_border(panel.border_width, panel.border_color),
        );
    }

    /// The chevron, the icon and the title; returns the room kept for the caller's buttons.
    fn draw_header(&self, ui: &mut Ui<'_>, header: Rect) -> Option<Rect> {
        let style = &ui.styles().collapsible_panel;
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
        let width = self
            .trailing
            .as_ref()
            .map_or(0.0, |width| width.resolve(&ui.styles().tokens).max(0.0));
        // Room after the title's start, or none: never an inverted rectangle.
        let trailing = (width > 0.0 && end > x).then(|| {
            Rect::from_min_max(
                Vec2::new((end - width).max(x), header.min.y),
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
        fn panel(&mut self, panel: &mut CollapsiblePanelState) -> (CollapsiblePanelResponse, bool) {
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
        let mut panel = CollapsiblePanelState::new(true);
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
        let mut state = CollapsiblePanelState::new(true);
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
        let mut panel = CollapsiblePanelState::new(false);
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
        // Never measured: the frame that opens it measures the content, and asks for the next
        // frame at once.
        let (_, drawn) = harness.panel(&mut panel);
        assert!(drawn);
        assert_eq!(harness.state.redraw_at(), Some(Duration::ZERO));
        harness.input.set_time(duration / 4);
        let (opening, _) = harness.panel(&mut panel);
        assert!(opening.height > style.header_height);
        assert!(opening.height < style.header_height + 60.0);
    }

    #[test]
    fn nested_panels_settle_one_frame_each_then_ask_for_nothing() {
        let mut harness = Harness::new();
        let mut outer = CollapsiblePanelState::new(true);
        let mut inner = CollapsiblePanelState::new(true);
        let mut heights = Vec::new();
        for _ in 0..4 {
            let height = harness
                .frame(|ui| {
                    CollapsiblePanel::new(ID, &mut outer, "Transform")
                        .show(ui, area(), |ui, rect| {
                            CollapsiblePanel::new(WidgetId::new("delta"), &mut inner, "Delta")
                                .level(PanelLevel::Second)
                                .show(ui, rect, |_, _| 50.0)
                                .height
                        })
                        .height
                })
                .0;
            heights.push(height);
        }
        // Two levels: settled by the third frame, which asks for no other.
        assert_eq!(heights[2], heights[3]);
        assert!(heights[0] < heights[2]);
        assert_eq!(harness.state.redraw_at(), None);
    }

    #[test]
    fn the_hidden_part_of_a_folding_panel_takes_no_press() {
        let mut harness = Harness::new();
        let style = harness.styles.collapsible_panel;
        let mut panel = CollapsiblePanelState::new(true);
        let button = WidgetId::new("button");
        let show = |harness: &mut Harness, panel: &mut CollapsiblePanelState| {
            harness
                .frame(|ui| {
                    let mut pressed = false;
                    CollapsiblePanel::new(ID, panel, "Transform").show(ui, area(), |ui, rect| {
                        // A button at the bottom of 100 pixels of content.
                        let at = Rect::from_origin_size(
                            Vec2::new(rect.min.x, rect.min.y + 80.0),
                            Vec2::new(rect.width(), 20.0),
                        );
                        pressed = ui.interact(button, at).pressed;
                        100.0
                    });
                    pressed
                })
                .0
        };
        show(&mut harness, &mut panel);
        show(&mut harness, &mut panel);
        // Folding: at half way, the bottom of the content is hidden.
        panel.set_open(false);
        show(&mut harness, &mut panel);
        let (duration, _) = style.collapse;
        harness.input.set_time(duration / 2);
        show(&mut harness, &mut panel);
        let y = area().min.y + style.header_height + style.content_padding + 90.0;
        harness.input.primary_button(true, Vec2::new(100.0, y));
        assert!(!show(&mut harness, &mut panel));
    }

    #[test]
    fn narrow_headers_keep_no_inverted_room_and_infinite_content_is_ignored() {
        let mut harness = Harness::new();
        let mut panel = CollapsiblePanelState::new(true);
        let narrow = Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::new(8.0, 400.0));
        let response = harness
            .frame(|ui| {
                CollapsiblePanel::new(ID, &mut panel, "Transform")
                    .trailing(40.0)
                    .show(ui, narrow, |_, _| f32::INFINITY)
            })
            .0;
        assert!(
            response
                .trailing
                .is_none_or(|room| room.min.x <= room.max.x)
        );
        let again = harness
            .frame(|ui| {
                CollapsiblePanel::new(ID, &mut panel, "Transform").show(ui, narrow, |_, _| 10.0)
            })
            .0;
        assert!(again.height.is_finite());
    }
}
