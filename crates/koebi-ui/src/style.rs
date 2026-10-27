//! [`Styles`]: what every component draws with, resolved from design tokens, and [`Prop`]: a
//! property a call gives a component, as a token or a value.
//!
//! Each component has a typed style (`ButtonStyle`, `SliderStyle`...) whose fields are read
//! from its `component.*` tokens by a `resolve` function (ADR 0017). Resolving reads a few
//! hundred values, so it happens when the design system changes, not per component: drawing
//! reads plain fields. [`Styles::tokens`] keeps a snapshot of every token, so that a token a
//! call gives a component is read in constant time.

use core::time::Duration;

use koebi_core::Color;
use koebi_design::kind::{self, Kind};
use koebi_design::tier::Tier;
use koebi_design::{DesignSystem, Resolve, Snapshot, Token, TokenId};
use koebi_render::Corners;
use koebi_tokens::{component, semantic};

use crate::text::TextStyles;
use crate::ui::WidgetState;
use crate::widgets::{
    ButtonStyle, CheckboxStyle, DropdownStyle, ListStyle, MenuStyle, NumberFieldStyle, PanelStyle,
    ScrollStyle, SeparatorStyle, SliderStyle, SwatchStyle, TextFieldStyle, ThemeCardStyle,
    TooltipStyle,
};
use crate::workspace::WorkspaceStyle;

/// One color per interaction state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StateColors {
    /// At rest.
    pub normal: Color,
    /// Under the pointer.
    pub hovered: Color,
    /// Held down.
    pub pressed: Color,
}

impl StateColors {
    /// The color of `state`.
    pub fn pick(&self, state: WidgetState) -> Color {
        match state {
            WidgetState::Normal => self.normal,
            WidgetState::Hovered => self.hovered,
            WidgetState::Pressed => self.pressed,
        }
    }
}

/// A property a call gives a component: a token of the design system, or a value.
///
/// Builders take `impl Into<Prop<K>>`, so a call writes either:
///
/// ```
/// # use koebi_ui::Prop;
/// # use koebi_design::kind;
/// let radius: Prop<kind::Dimension> = 0.0.into();
/// let themed: Prop<kind::Dimension> = koebi_tokens::semantic::RADIUS_M.into();
/// # let _ = (radius, themed);
/// ```
pub enum Prop<K: Kind> {
    /// The value of a token, read in [`Styles::tokens`].
    Token(TokenId),
    /// A value.
    Value(K::Value),
}

impl<K: Kind> Clone for Prop<K> {
    fn clone(&self) -> Self {
        match self {
            Self::Token(token) => Self::Token(*token),
            Self::Value(value) => Self::Value(value.clone()),
        }
    }
}

impl<K: Kind> PartialEq for Prop<K> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Token(a), Self::Token(b)) => a == b,
            (Self::Value(a), Self::Value(b)) => a == b,
            _ => false,
        }
    }
}

impl<K: Kind> core::fmt::Debug for Prop<K> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Token(token) => f.debug_tuple("Token").field(token).finish(),
            Self::Value(value) => f.debug_tuple("Value").field(value).finish(),
        }
    }
}

impl<K: Kind> Prop<K> {
    /// The value of the property in `tokens`.
    pub fn resolve(&self, tokens: &Snapshot) -> K::Value {
        match self {
            Self::Token(token) => tokens
                .value(*token)
                .and_then(K::from_value)
                .unwrap_or_else(K::fallback),
            Self::Value(value) => value.clone(),
        }
    }
}

impl<K: Kind, T: Tier> From<Token<K, T>> for Prop<K> {
    fn from(token: Token<K, T>) -> Self {
        Self::Token(token.id())
    }
}

impl From<Color> for Prop<kind::Color> {
    fn from(color: Color) -> Self {
        Self::Value(color)
    }
}

impl From<f32> for Prop<kind::Dimension> {
    fn from(value: f32) -> Self {
        Self::Value(value)
    }
}

impl From<f32> for Prop<kind::Number> {
    fn from(value: f32) -> Self {
        Self::Value(value)
    }
}

impl From<Duration> for Prop<kind::Duration> {
    fn from(value: Duration) -> Self {
        Self::Value(value)
    }
}

/// The radius of each corner of a control, as tokens or values ([`Prop`]).
#[derive(Clone, Debug, PartialEq)]
pub struct CornerRadii {
    /// Top left.
    pub top_left: Prop<kind::Dimension>,
    /// Top right.
    pub top_right: Prop<kind::Dimension>,
    /// Bottom right.
    pub bottom_right: Prop<kind::Dimension>,
    /// Bottom left.
    pub bottom_left: Prop<kind::Dimension>,
}

impl CornerRadii {
    /// Each corner its radius, clockwise from the top left.
    pub fn new(
        top_left: impl Into<Prop<kind::Dimension>>,
        top_right: impl Into<Prop<kind::Dimension>>,
        bottom_right: impl Into<Prop<kind::Dimension>>,
        bottom_left: impl Into<Prop<kind::Dimension>>,
    ) -> Self {
        Self {
            top_left: top_left.into(),
            top_right: top_right.into(),
            bottom_right: bottom_right.into(),
            bottom_left: bottom_left.into(),
        }
    }

    /// The same radius for every corner.
    pub fn all(radius: impl Into<Prop<kind::Dimension>>) -> Self {
        let radius = radius.into();
        Self {
            top_left: radius.clone(),
            top_right: radius.clone(),
            bottom_right: radius.clone(),
            bottom_left: radius,
        }
    }

    /// The radii in `tokens`.
    pub fn resolve(&self, tokens: &Snapshot) -> Corners {
        Corners {
            top_left: self.top_left.resolve(tokens),
            top_right: self.top_right.resolve(tokens),
            bottom_right: self.bottom_right.resolve(tokens),
            bottom_left: self.bottom_left.resolve(tokens),
        }
    }
}

impl From<Corners> for CornerRadii {
    fn from(corners: Corners) -> Self {
        Self::new(
            corners.top_left,
            corners.top_right,
            corners.bottom_right,
            corners.bottom_left,
        )
    }
}

/// Timings and distances of interaction, from the `semantic.input.*` tokens: the input
/// settings a theme or the user may change.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputStyle {
    /// How far the pointer moves before a press becomes a drag.
    pub drag_threshold: f32,
    /// Longest time between the two presses of a double click.
    pub double_click: Duration,
    /// How long a held button waits before repeating.
    pub repeat_delay: Duration,
    /// Time between two repeats.
    pub repeat_interval: Duration,
    /// How long the pointer rests before a tooltip shows.
    pub tooltip_delay: Duration,
    /// How long the pointer rests before the tooltip of the next control shows while a
    /// tooltip is shown.
    pub tooltip_delay_short: Duration,
    /// How long the pointer rests on a hover-revealed element before it appears.
    pub hover_delay: Duration,
    /// How long the pointer rests on the item of a submenu before the submenu opens.
    pub submenu_delay: Duration,
    /// How long an open submenu waits for the pointer heading towards it.
    pub submenu_grace: Duration,
    /// How far the pointer goes from the open menus before they close; 0 never.
    pub menu_close_distance: f32,
}

impl InputStyle {
    /// Reads the settings from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            drag_threshold: tokens.get(semantic::INPUT_DRAG_THRESHOLD),
            double_click: tokens.get(semantic::INPUT_DOUBLE_CLICK_TIME),
            repeat_delay: tokens.get(semantic::INPUT_REPEAT_DELAY),
            repeat_interval: tokens.get(semantic::INPUT_REPEAT_INTERVAL),
            tooltip_delay: tokens.get(semantic::INPUT_TOOLTIP_DELAY),
            tooltip_delay_short: tokens.get(semantic::INPUT_TOOLTIP_DELAY_SHORT),
            hover_delay: tokens.get(semantic::INPUT_HOVER_DELAY),
            submenu_delay: tokens.get(semantic::INPUT_SUBMENU_DELAY),
            submenu_grace: tokens.get(semantic::INPUT_SUBMENU_GRACE),
            menu_close_distance: tokens.get(semantic::INPUT_MENU_CLOSE_DISTANCE),
        }
    }
}

/// Size and stroke of line icons, from the `component.icon.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconStyle {
    /// Width and height of an icon, in logical pixels.
    pub size: f32,
    /// Stroke width.
    pub stroke_width: f32,
}

impl IconStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            size: tokens.get(component::ICON_SIZE_DEFAULT),
            stroke_width: tokens.get(component::ICON_STROKE_WIDTH_DEFAULT),
        }
    }
}

/// The style of every component, resolved together.
#[derive(Clone, Debug, PartialEq)]
pub struct Styles {
    /// Every token's value when the styles were resolved: where tokens given at a call are
    /// read ([`Prop`]).
    pub tokens: Snapshot,
    /// Input settings: drag threshold, double click, repeats, delays.
    pub input: InputStyle,
    /// Window background: the clear color of the scene.
    pub background: Color,
    /// Line icons.
    pub icon: IconStyle,
    /// Typography and text colors.
    pub text: TextStyles,
    /// [`Button`](crate::Button) and [`ButtonGroup`](crate::ButtonGroup).
    pub button: ButtonStyle,
    /// [`Checkbox`](crate::Checkbox).
    pub checkbox: CheckboxStyle,
    /// [`Ui::slider`](crate::Ui::slider).
    pub slider: SliderStyle,
    /// [`NumberField`](crate::NumberField).
    pub number_field: NumberFieldStyle,
    /// [`Ui::panel`](crate::Ui::panel).
    pub panel: PanelStyle,
    /// [`Ui::separator`](crate::Ui::separator).
    pub separator: SeparatorStyle,
    /// [`Ui::swatch`](crate::Ui::swatch).
    pub swatch: SwatchStyle,
    /// [`Ui::theme_card`](crate::Ui::theme_card).
    pub theme_card: ThemeCardStyle,
    /// [`Ui::menu`](crate::Ui::menu).
    pub menu: MenuStyle,
    /// [`Dropdown`](crate::Dropdown).
    pub dropdown: DropdownStyle,
    /// [`TextField`](crate::TextField).
    pub text_field: TextFieldStyle,
    /// [`Ui::workspace`](crate::Ui::workspace).
    pub workspace: WorkspaceStyle,
    /// [`Ui::scroll_area`](crate::Ui::scroll_area).
    pub scroll: ScrollStyle,
    /// [`Ui::list`](crate::Ui::list).
    pub list: ListStyle,
    /// [`Ui::tooltip`](crate::Ui::tooltip).
    pub tooltip: TooltipStyle,
}

impl Styles {
    /// Reads every style from `tokens`: the active theme ([`DesignSystem`]) or a
    /// [`View`](koebi_design::View) of any theme.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self::resolve_with(tokens, tokens.snapshot())
    }

    /// Reads every style again from `tokens`, reusing the memory of the token snapshot: what
    /// a frame that changed the design system costs.
    pub fn refresh(&mut self, tokens: &impl Resolve) {
        let mut snapshot = core::mem::take(&mut self.tokens);
        tokens.snapshot_into(&mut snapshot);
        *self = Self::resolve_with(tokens, snapshot);
    }

    /// Every style from `tokens`, with `snapshot` already captured from them.
    fn resolve_with(tokens: &impl Resolve, snapshot: Snapshot) -> Self {
        Self {
            tokens: snapshot,
            input: InputStyle::resolve(tokens),
            background: tokens.get(component::WINDOW_BACKGROUND_COLOR_DEFAULT),
            icon: IconStyle::resolve(tokens),
            text: TextStyles::resolve(tokens),
            button: ButtonStyle::resolve(tokens),
            checkbox: CheckboxStyle::resolve(tokens),
            slider: SliderStyle::resolve(tokens),
            number_field: NumberFieldStyle::resolve(tokens),
            panel: PanelStyle::resolve(tokens),
            separator: SeparatorStyle::resolve(tokens),
            swatch: SwatchStyle::resolve(tokens),
            theme_card: ThemeCardStyle::resolve(tokens),
            menu: MenuStyle::resolve(tokens),
            dropdown: DropdownStyle::resolve(tokens),
            text_field: TextFieldStyle::resolve(tokens),
            workspace: WorkspaceStyle::resolve(tokens),
            scroll: ScrollStyle::resolve(tokens),
            list: ListStyle::resolve(tokens),
            tooltip: TooltipStyle::resolve(tokens),
        }
    }
}

/// [`Styles`] that follow a design system's active theme, resolved again only when the
/// design system changed: every theme switch, override or color vision change shows on the
/// next frame, and frames without changes read no token at all.
#[derive(Clone, Debug)]
pub struct StyleCache {
    revision: u64,
    styles: Styles,
}

impl StyleCache {
    /// Resolves the styles of the active theme.
    pub fn new(design: &DesignSystem) -> Self {
        Self {
            revision: design.revision(),
            styles: Styles::resolve(design),
        }
    }

    /// The styles of the active theme, resolved again if `design` changed since the last
    /// call.
    pub fn update(&mut self, design: &DesignSystem) -> &Styles {
        if design.revision() != self.revision {
            self.revision = design.revision();
            self.styles.refresh(design);
        }
        &self.styles
    }

    /// The styles as last resolved.
    pub fn styles(&self) -> &Styles {
        &self.styles
    }
}

#[cfg(test)]
mod tests {
    use koebi_design::{Layers, Scope};
    use koebi_tokens::{KOEBI, semantic};

    use super::*;
    use crate::widgets::Tone;

    fn design() -> DesignSystem {
        DesignSystem::new(&KOEBI).expect("Koebi's schema is valid")
    }

    #[test]
    fn styles_read_component_tokens() {
        let design = design();
        let styles = Styles::resolve(&design);
        assert_eq!(
            styles.button.colors(Tone::DEFAULT).background.normal,
            design.get(component::BUTTON_BACKGROUND_COLOR_DEFAULT)
        );
        assert_eq!(
            styles.slider.grab.pressed,
            design.get(component::SLIDER_GRAB_COLOR_PRESSED)
        );
        assert_eq!(styles.checkbox.box_size, 16.0);
        assert_eq!(
            styles.icon,
            IconStyle {
                size: 16.0,
                stroke_width: 1.5
            }
        );
    }

    #[test]
    fn styles_of_another_theme_come_from_its_view() {
        let design = design();
        let light = design.find_theme("light").expect("built-in theme");
        let view = design.view(light, Layers::ALL).expect("live theme");
        let styles = Styles::resolve(&view);
        assert_eq!(
            styles.background,
            view.get(component::WINDOW_BACKGROUND_COLOR_DEFAULT)
        );
        assert_ne!(styles.background, Styles::resolve(&design).background);
    }

    #[test]
    fn the_cache_resolves_again_after_a_change_only() {
        let mut design = design();
        let mut cache = StyleCache::new(&design);
        let accent = |styles: &Styles| styles.button.colors(Tone::ACCENT).background.normal;
        let before = accent(cache.styles());
        assert_eq!(accent(cache.update(&design)), before);

        let red = Color::from_hex(0xFF_00_00);
        design
            .set_value(Scope::Global, semantic::BACKGROUND_ACCENT_DEFAULT, red)
            .unwrap();
        // The accent button aliases the semantic token: the change flows to its style.
        assert_eq!(accent(cache.update(&design)), red);
    }
}
