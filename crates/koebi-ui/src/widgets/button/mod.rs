//! Buttons: push buttons, toggle buttons and button groups.
//!
//! A [`Button`] is built, configured and shown:
//!
//! ```
//! # use koebi_core::{Rect, Vec2};
//! # use koebi_render::Scene;
//! # use koebi_ui::{Button, Icon, Input, Tone, Ui, UiState, WidgetId};
//! # let styles = koebi_ui::Styles::resolve(&koebi_design::DesignSystem::new(&koebi_tokens::KOEBI)?);
//! # let (mut scene, mut state, input) = (Scene::new(), UiState::new(), Input::new());
//! # let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
//! let rect = Rect::from_origin_size(Vec2::ZERO, Vec2::new(80.0, 20.0));
//! let response = Button::new(WidgetId::new("save"))
//!     .label("Save")
//!     .leading_icon(Icon::Check)
//!     .tone(Tone::ACCENT)
//!     .show(&mut ui, rect);
//! if response.clicked {
//!     // Save.
//! }
//! # Ok::<(), koebi_design::Error>(())
//! ```
//!
//! - **Look**: a [`Tone`] (variant and optional color family) picks the colors of every state;
//!   every property can be overridden with a token or a value ([`Prop`]).
//! - **Content**: leading icon, label or icon, status dot, trailing icon ([`Content`]).
//! - **Size**: [`Button::size`] measures the button: one interface unit high, as wide as its
//!   content within its minimum and maximum widths, unless its width is fixed.
//! - **Behaviour**: the action fires on release by default ([`Activation`]); a button can
//!   repeat while held, and toggle a value ([`Button::toggle`]).

mod group;
mod style;

#[cfg(test)]
mod tests;

pub use group::{ButtonGroup, GroupCell, GroupResponse, Track};
pub use style::ButtonStyle;

use koebi_core::{Color, Rect, Vec2};
use koebi_design::kind;
use koebi_render::{Corners, RectPrimitive};

use crate::icon::Icon;
use crate::id::WidgetId;
use crate::style::{CornerRadii, Prop};
use crate::text::Align;
use crate::ui::{Response, Ui};
use crate::widgets::parts::{Content, ContentMetrics, ControlState, Tone, ToneColors};

/// When a button's action fires.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Activation {
    /// When the pointer is released over the button after pressing it: moving away before
    /// releasing cancels (the default).
    #[default]
    Release,
    /// As soon as the button is pressed.
    Press,
    /// On the second press of a double click: within the double-click time of the first
    /// press, and near the same place ([`Response::double_clicked`]).
    DoublePress,
}

/// Color overrides of a call: per part (background, border, content) and per state.
type ColorOverrides = [[Option<Prop<kind::Color>>; 6]; 3];

const BACKGROUND: usize = 0;
const BORDER: usize = 1;
const CONTENT: usize = 2;

/// A button, configured then shown with [`Button::show`]. Every option has a default: no
/// content, the default tone, enabled, firing on release, as wide as its content.
///
/// A toggle button of the default tone ([`Button::toggle`]) takes the look of toggles
/// (`component.toggle.*`): its own colors, corner radius and border width.
#[derive(Debug)]
pub struct Button<'a> {
    id: WidgetId,
    content: Content<'a>,
    tone: Tone,
    enabled: bool,
    selected: bool,
    toggle: Option<&'a mut bool>,
    activation: Activation,
    repeat: bool,
    width: Option<Prop<kind::Dimension>>,
    min_width: Option<Prop<kind::Dimension>>,
    max_width: Option<Prop<kind::Dimension>>,
    height: Option<Prop<kind::Dimension>>,
    padding: Option<Prop<kind::Dimension>>,
    border_width: Option<Prop<kind::Dimension>>,
    radii: Option<CornerRadii>,
    colors: ColorOverrides,
}

impl<'a> Button<'a> {
    /// A button identified by `id`, without content yet.
    pub fn new(id: WidgetId) -> Self {
        Self {
            id,
            content: Content::default(),
            tone: Tone::DEFAULT,
            enabled: true,
            selected: false,
            toggle: None,
            activation: Activation::Release,
            repeat: false,
            width: None,
            min_width: None,
            max_width: None,
            height: None,
            padding: None,
            border_width: None,
            radii: None,
            colors: ColorOverrides::default(),
        }
    }

    /// Shows `label`.
    #[must_use]
    pub fn label(mut self, label: &'a str) -> Self {
        self.content.label = Some(label);
        self
    }

    /// Shows `icon` in place of the label, or after it.
    #[must_use]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.content.icon = Some(icon);
        self
    }

    /// Shows `icon` before everything.
    #[must_use]
    pub fn leading_icon(mut self, icon: Icon) -> Self {
        self.content.leading = Some(icon);
        self
    }

    /// Shows `icon` after everything.
    #[must_use]
    pub fn trailing_icon(mut self, icon: Icon) -> Self {
        self.content.trailing = Some(icon);
        self
    }

    /// Shows a status dot of `color` after the label.
    #[must_use]
    pub fn status_dot(mut self, color: Color) -> Self {
        self.content.dot = Some(color);
        self
    }

    /// Replaces the whole content.
    #[must_use]
    pub fn content(mut self, content: Content<'a>) -> Self {
        self.content = content;
        self
    }

    /// Places the content at the start, center (the default) or end of the button.
    #[must_use]
    pub fn align(mut self, align: Align) -> Self {
        self.content.align = align;
        self
    }

    /// Colors the button by `tone`.
    #[must_use]
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Makes the button available or not: a disabled button shows dimmed and ignores the
    /// pointer.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows the button selected (switched on, the current choice).
    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// A toggle button: each activation flips `value`, and the button shows selected while
    /// it is `true`.
    #[must_use]
    pub fn toggle(mut self, value: &'a mut bool) -> Self {
        self.toggle = Some(value);
        self
    }

    /// When the action fires.
    #[must_use]
    pub fn activation(mut self, activation: Activation) -> Self {
        self.activation = activation;
        self
    }

    /// Fires on press, then repeats while held under the pointer, after the repeat delay of
    /// the input settings (step buttons); repeats pause while the pointer is off the button.
    #[must_use]
    pub fn repeat(mut self, repeat: bool) -> Self {
        self.repeat = repeat;
        self
    }

    /// A fixed width for [`Button::size`], in place of the width of the content.
    #[must_use]
    pub fn width(mut self, width: impl Into<Prop<kind::Dimension>>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// The smallest width a fitting button measures.
    #[must_use]
    pub fn min_width(mut self, width: impl Into<Prop<kind::Dimension>>) -> Self {
        self.min_width = Some(width.into());
        self
    }

    /// The largest width a fitting button measures; its label is cut beyond.
    #[must_use]
    pub fn max_width(mut self, width: impl Into<Prop<kind::Dimension>>) -> Self {
        self.max_width = Some(width.into());
        self
    }

    /// The height [`Button::size`] measures.
    #[must_use]
    pub fn height(mut self, height: impl Into<Prop<kind::Dimension>>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// The space between the sides and the content.
    #[must_use]
    pub fn padding(mut self, padding: impl Into<Prop<kind::Dimension>>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    /// The same radius for every corner. The last of this and [`Button::corner_radii`]
    /// applies.
    #[must_use]
    pub fn corner_radius(mut self, radius: impl Into<Prop<kind::Dimension>>) -> Self {
        self.radii = Some(CornerRadii::all(radius));
        self
    }

    /// A radius per corner, as tokens or values (`Corners` converts). The last of this and
    /// [`Button::corner_radius`] applies.
    #[must_use]
    pub fn corner_radii(mut self, radii: impl Into<CornerRadii>) -> Self {
        self.radii = Some(radii.into());
        self
    }

    /// The border width.
    #[must_use]
    pub fn border_width(mut self, width: impl Into<Prop<kind::Dimension>>) -> Self {
        self.border_width = Some(width.into());
        self
    }

    /// The background in `state`.
    #[must_use]
    pub fn background(mut self, state: ControlState, color: impl Into<Prop<kind::Color>>) -> Self {
        self.colors[BACKGROUND][state.index()] = Some(color.into());
        self
    }

    /// The border color in `state`.
    #[must_use]
    pub fn border_color(
        mut self,
        state: ControlState,
        color: impl Into<Prop<kind::Color>>,
    ) -> Self {
        self.colors[BORDER][state.index()] = Some(color.into());
        self
    }

    /// The color of the content (label and icons) in `state`.
    #[must_use]
    pub fn content_color(
        mut self,
        state: ControlState,
        color: impl Into<Prop<kind::Color>>,
    ) -> Self {
        self.colors[CONTENT][state.index()] = Some(color.into());
        self
    }

    /// The identifier of the button.
    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// `true` for a toggle of the default tone, drawn with the look of toggles.
    fn is_plain_toggle(&self) -> bool {
        self.toggle.is_some() && self.tone == Tone::DEFAULT
    }

    /// The colors the button draws with: its tone's (or the toggles' for a toggle of the
    /// default tone), with the overrides of the call.
    pub fn colors(&self, ui: &Ui<'_>) -> ToneColors {
        let styles = ui.styles();
        let mut colors = if self.is_plain_toggle() {
            styles.button.toggle
        } else {
            *styles.button.colors(self.tone)
        };
        for (part, overrides) in self.colors.iter().enumerate() {
            let target = match part {
                BACKGROUND => &mut colors.background,
                BORDER => &mut colors.border,
                _ => &mut colors.content,
            };
            for (state, prop) in ControlState::ALL.into_iter().zip(overrides) {
                if let Some(prop) = prop {
                    target.set(state, prop.resolve(&styles.tokens));
                }
            }
        }
        colors
    }

    /// The metrics of the content.
    fn metrics<'s>(ui: &Ui<'s>) -> ContentMetrics<'s> {
        let styles = ui.styles();
        let style = &styles.button;
        ContentMetrics {
            gap: style.gap,
            icon_size: style.icon_size,
            stroke_width: style.stroke_width,
            dot_size: style.dot_size,
            font: &styles.text.label,
        }
    }

    /// Resolves a dimension given at the call, or takes the style's.
    fn dimension(ui: &Ui<'_>, prop: Option<&Prop<kind::Dimension>>, fallback: f32) -> f32 {
        prop.map_or(fallback, |prop| prop.resolve(&ui.styles().tokens))
    }

    /// The size the button asks for: one interface unit high (or its height), never lower than
    /// its content; as wide as its content, paddings and side borders within its minimum and
    /// maximum widths, or its fixed width.
    pub fn size(&self, ui: &mut Ui<'_>) -> Vec2 {
        let border = self.border(ui);
        self.measure(ui, border)
    }

    /// [`Button::size`] with side borders of `border` (groups draw their own).
    pub(crate) fn measure(&self, ui: &mut Ui<'_>, border: f32) -> Vec2 {
        let style = &ui.styles().button;
        let metrics = Self::metrics(ui);
        let height = Self::dimension(ui, self.height.as_ref(), style.height)
            .max(self.content.height(ui, &metrics))
            .ceil();
        let width = if let Some(width) = &self.width {
            width.resolve(&ui.styles().tokens)
        } else {
            let content = self.content.width(ui, &metrics);
            // A button showing an icon alone is square.
            let natural = if self.content.is_icon_only() {
                height.max(content + 2.0 * border)
            } else {
                let padding = Self::dimension(ui, self.padding.as_ref(), style.padding);
                content + 2.0 * (padding + border)
            };
            let min = Self::dimension(ui, self.min_width.as_ref(), style.min_width);
            let fitted = natural.ceil().max(min);
            self.max_width
                .as_ref()
                .map_or(fitted, |max| fitted.min(max.resolve(&ui.styles().tokens)))
        };
        Vec2::new(width, height)
    }

    /// The border width: the call's, or the style's.
    fn border(&self, ui: &Ui<'_>) -> f32 {
        let style = &ui.styles().button;
        let fallback = if self.is_plain_toggle() {
            style.toggle_border_width
        } else {
            style.border_width
        };
        Self::dimension(ui, self.border_width.as_ref(), fallback)
    }

    /// The corner radii: the call's, or the style's.
    fn radii(&self, ui: &Ui<'_>) -> Corners {
        let styles = ui.styles();
        match &self.radii {
            Some(radii) => radii.resolve(&styles.tokens),
            None if self.is_plain_toggle() => Corners::all(styles.button.toggle_corner_radius),
            None => Corners::all(styles.button.corner_radius),
        }
    }

    /// Shows the button in `rect` and returns what happened: `clicked` when its action fired
    /// (per its [`Activation`], or a repeat), `changed` when it toggled its value.
    pub fn show(mut self, ui: &mut Ui<'_>, rect: Rect) -> Response {
        // A disabled button still keeps the pointer from what lies under it.
        let raw = ui.interact(self.id, rect);
        let response = self.react(ui, raw);
        let state = ControlState::of(&response, self.is_selected(), self.enabled);
        self.draw(ui, rect, state);
        response
    }

    /// What the button does with the pointer events of `response`: nothing when disabled;
    /// otherwise its action fires per its activation (or repeats), and a toggle flips its
    /// value. Returns the response with `clicked` meaning "fired".
    pub(crate) fn react(&mut self, ui: &mut Ui<'_>, mut response: Response) -> Response {
        if !self.enabled {
            return Response::default();
        }
        let fired = if self.repeat {
            response.pressed || ui.repeats(&response)
        } else {
            match self.activation {
                Activation::Release => response.clicked,
                Activation::Press => response.pressed,
                Activation::DoublePress => response.double_clicked,
            }
        };
        response.clicked = fired;
        if fired && let Some(value) = self.toggle.as_deref_mut() {
            *value = !*value;
            response.changed = true;
        }
        response
    }

    /// Draws the button in `rect` in `state`, without interaction: what [`Button::show`] and
    /// groups draw.
    pub(crate) fn draw(&self, ui: &mut Ui<'_>, rect: Rect, state: ControlState) {
        let colors = self.colors(ui);
        let radii = self.radii(ui);
        let border = self.border(ui);
        ui.scene().push_rect(
            &RectPrimitive::new(rect, colors.background.get(state))
                .with_corner_radii(radii)
                .with_border(border, colors.border.get(state)),
        );
        self.draw_content(ui, rect, (border, border), colors.content.get(state));
    }

    /// Draws the content in `color` inside `rect`, centered vertically, clear of the side
    /// borders (`sides`: the left and right widths) and within the paddings; content that does
    /// not fit is clipped to `rect`.
    pub(crate) fn draw_content(
        &self,
        ui: &mut Ui<'_>,
        rect: Rect,
        sides: (f32, f32),
        color: Color,
    ) {
        let padding = Self::dimension(ui, self.padding.as_ref(), ui.styles().button.padding);
        let left = rect.min.x + sides.0 + padding;
        let content = Rect::from_min_max(
            Vec2::new(left, rect.min.y),
            Vec2::new((rect.max.x - sides.1 - padding).max(left), rect.max.y),
        );
        let metrics = Self::metrics(ui);
        self.content.draw(ui, content, rect, color, &metrics);
    }

    /// Whether the button is enabled.
    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Whether the button shows selected.
    pub(crate) fn is_selected(&self) -> bool {
        self.toggle.as_deref().copied().unwrap_or(self.selected)
    }
}
