//! [`Styles`]: what every component draws with, resolved from design tokens, and [`Prop`]: a
//! property a call gives a component, as a token or a value.
//!
//! Each component has a typed style (`ButtonStyle`, `PanelStyle`...) whose fields are read
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

use crate::pixel::whole_pixels;
use crate::text::TextStyles;
use crate::ui::WidgetState;
use crate::widgets::{
    AppBarStyle, ButtonStyle, CaptureFieldStyle, CheckboxStyle, CollapsiblePanelStyle,
    DropdownStyle, KeyCapStyle, ListStyle, MenuStyle, NumberFieldStyle, PanelStyle,
    PropertyGroupStyle, ResetDotStyle, ScrollStyle, SeparatorStyle, StatusBarStyle, SwatchStyle,
    TextFieldStyle, ThemeCardStyle, TooltipStyle,
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
    /// Stroke width of the chevrons that fold and open: fold arrows, panel headers, dropdowns.
    pub chevron_stroke_width: f32,
}

impl IconStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self {
            size: tokens.get(component::ICON_SIZE_DEFAULT),
            stroke_width: tokens.get(component::ICON_STROKE_WIDTH_DEFAULT),
            chevron_stroke_width: tokens.get(component::ICON_STROKE_WIDTH_CHEVRON),
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
    /// [`AppBar`](crate::AppBar) and [`Ui::menu_bar`](crate::Ui::menu_bar).
    pub app_bar: AppBarStyle,
    /// Line icons.
    pub icon: IconStyle,
    /// Typography and text colors.
    pub text: TextStyles,
    /// [`Button`](crate::Button) and [`ButtonGroup`](crate::ButtonGroup).
    pub button: ButtonStyle,
    /// [`Checkbox`](crate::Checkbox).
    pub checkbox: CheckboxStyle,
    /// [`NumberField`](crate::NumberField).
    pub number_field: NumberFieldStyle,
    /// [`PropertyGroup`](crate::PropertyGroup).
    pub property_group: PropertyGroupStyle,
    /// [`Ui::panel`](crate::Ui::panel).
    pub panel: PanelStyle,
    /// [`CollapsiblePanel`](crate::CollapsiblePanel).
    pub collapsible_panel: CollapsiblePanelStyle,
    /// [`Ui::separator`](crate::Ui::separator).
    pub separator: SeparatorStyle,
    /// [`Ui::swatch`](crate::Ui::swatch).
    pub swatch: SwatchStyle,
    /// [`Ui::theme_card`](crate::Ui::theme_card) and [`Ui::card`](crate::Ui::card).
    pub theme_card: ThemeCardStyle,
    /// [`Ui::reset_dot`](crate::Ui::reset_dot).
    pub reset_dot: ResetDotStyle,
    /// [`Ui::key_caps`](crate::Ui::key_caps).
    pub key_caps: KeyCapStyle,
    /// [`CaptureField`](crate::CaptureField).
    pub capture_field: CaptureFieldStyle,
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
    /// [`Ui::status_bar`](crate::Ui::status_bar).
    pub status_bar: StatusBarStyle,
}

impl Styles {
    /// Reads every style from `tokens` (the active theme, [`DesignSystem`], or a
    /// [`View`](koebi_design::View) of any theme) for a window of one physical pixel per
    /// logical pixel.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        Self::resolve_at(tokens, 1.0)
    }

    /// Reads every style from `tokens` for a window of `scale` physical pixels per logical
    /// pixel: layout lengths become whole physical pixels ([`whole_pixels`]), so that an inset
    /// or a gap is the same number of pixels wherever it lies.
    pub fn resolve_at(tokens: &impl Resolve, scale: f32) -> Self {
        let mut styles = Self::resolve_with(tokens, tokens.snapshot());
        styles.snap_lengths(scale);
        styles
    }

    /// Reads every style again from `tokens` at `scale`, reusing the memory of the token
    /// snapshot: what a frame that changed the design system costs.
    pub fn refresh_at(&mut self, tokens: &impl Resolve, scale: f32) {
        let mut snapshot = core::mem::take(&mut self.tokens);
        tokens.snapshot_into(&mut snapshot);
        *self = Self::resolve_with(tokens, snapshot);
        self.snap_lengths(scale);
    }

    /// Rounds the lengths that lay components out (heights, insets, gaps, paddings, indents)
    /// to whole physical pixels at `scale`. Strokes, borders, radii and font sizes keep their
    /// values: the renderer anti-aliases or rounds them itself.
    fn snap_lengths(&mut self, scale: f32) {
        let snap = |lengths: &mut [&mut f32]| {
            for length in lengths {
                **length = whole_pixels(**length, scale);
            }
        };
        let panel = &mut self.collapsible_panel;
        let control = whole_pixels(panel.header_height - 2.0 * panel.header_inset, scale);
        snap(&mut [
            &mut panel.header_inset,
            &mut panel.header_padding,
            &mut panel.header_gap,
            &mut panel.indent,
            &mut panel.content_padding,
            &mut panel.gap,
        ]);
        panel.header_height = control + 2.0 * panel.header_inset;
        let bar = &mut self.app_bar;
        snap(&mut [
            &mut bar.padding,
            &mut bar.gap,
            &mut bar.button_width,
            &mut bar.menu_padding,
            &mut bar.item_height,
        ]);
        let button = &mut self.button;
        snap(&mut [
            &mut button.height,
            &mut button.min_width,
            &mut button.padding,
            &mut button.gap,
        ]);
        let capture = &mut self.capture_field;
        snap(&mut [&mut capture.padding, &mut capture.min_width]);
        let checkbox = &mut self.checkbox;
        snap(&mut [
            &mut checkbox.box_size,
            &mut checkbox.gap,
            &mut checkbox.line_height,
        ]);
        let dropdown = &mut self.dropdown;
        snap(&mut [
            &mut dropdown.height,
            &mut dropdown.min_width,
            &mut dropdown.padding,
            &mut dropdown.padding_vertical,
            &mut dropdown.gap,
        ]);
        let caps = &mut self.key_caps;
        snap(&mut [&mut caps.padding, &mut caps.inset, &mut caps.gap]);
        snap(&mut [&mut self.panel.padding, &mut self.theme_card.padding]);
        let group = &mut self.property_group;
        snap(&mut [
            &mut group.row_height,
            &mut group.row_gap,
            &mut group.group_gap,
            &mut group.column_gap,
        ]);
        snap(&mut [&mut self.scroll.margin, &mut self.scroll.padding]);
        let tooltip = &mut self.tooltip;
        snap(&mut [
            &mut tooltip.padding_horizontal,
            &mut tooltip.padding_vertical,
            &mut tooltip.offset,
        ]);
        let list = &mut self.list;
        snap(&mut [
            &mut list.row_height,
            &mut list.indent,
            &mut list.padding,
            &mut list.gap,
            &mut list.highlight_margin_vertical,
            &mut list.highlight_margin_horizontal,
        ]);
        let menu = &mut self.menu;
        snap(&mut [
            &mut menu.padding_vertical,
            &mut menu.padding_horizontal,
            &mut menu.item_height,
            &mut menu.item_gap,
            &mut menu.item_margin,
            &mut menu.item_padding,
            &mut menu.column_gap,
            &mut menu.icon_gap,
        ]);
        let number = &mut self.number_field;
        snap(&mut [
            &mut number.height,
            &mut number.min_width,
            &mut number.padding,
            &mut number.step_width,
        ]);
        let text = &mut self.text_field;
        snap(&mut [
            &mut text.height,
            &mut text.min_width,
            &mut text.padding,
            &mut text.gap,
        ]);
        let bar = &mut self.status_bar;
        let control = whole_pixels(bar.height - 2.0 * bar.inset, scale);
        snap(&mut [&mut bar.inset, &mut bar.padding, &mut bar.gap]);
        bar.height = control + 2.0 * bar.inset;
        let workspace = &mut self.workspace;
        snap(&mut [
            &mut workspace.gap,
            &mut workspace.top_bar_height,
            &mut workspace.top_bar_padding,
            &mut workspace.top_bar_gap,
            &mut workspace.tab_bar_height,
            &mut workspace.tab_gap,
            &mut workspace.tab_bar_padding,
            &mut workspace.tab_padding,
            &mut workspace.drop_center_inset,
        ]);
    }

    /// Every style from `tokens`, with `snapshot` already captured from them.
    fn resolve_with(tokens: &impl Resolve, snapshot: Snapshot) -> Self {
        let text = TextStyles::resolve(tokens);
        let key_caps = KeyCapStyle::resolve(tokens, &text.label);
        Self {
            tokens: snapshot,
            input: InputStyle::resolve(tokens),
            background: tokens.get(component::WINDOW_BACKGROUND_COLOR_DEFAULT),
            app_bar: AppBarStyle::resolve(tokens),
            icon: IconStyle::resolve(tokens),
            text,
            button: ButtonStyle::resolve(tokens),
            checkbox: CheckboxStyle::resolve(tokens),
            number_field: NumberFieldStyle::resolve(tokens),
            property_group: PropertyGroupStyle::resolve(tokens),
            panel: PanelStyle::resolve(tokens),
            collapsible_panel: CollapsiblePanelStyle::resolve(tokens),
            separator: SeparatorStyle::resolve(tokens),
            swatch: SwatchStyle::resolve(tokens),
            theme_card: ThemeCardStyle::resolve(tokens),
            reset_dot: ResetDotStyle::resolve(tokens),
            key_caps,
            capture_field: CaptureFieldStyle::resolve(tokens),
            menu: MenuStyle::resolve(tokens),
            dropdown: DropdownStyle::resolve(tokens),
            text_field: TextFieldStyle::resolve(tokens),
            workspace: WorkspaceStyle::resolve(tokens),
            scroll: ScrollStyle::resolve(tokens),
            list: ListStyle::resolve(tokens),
            tooltip: TooltipStyle::resolve(tokens),
            status_bar: StatusBarStyle::resolve(tokens),
        }
    }
}

/// How many scale factors a [`StyleCache`] keeps styles for: one per kind of display the
/// windows lie on, which is rarely more than two.
const SCALES_KEPT: usize = 4;

/// [`Styles`] that follow a design system's active theme, at each scale factor windows draw
/// at, resolved again only when the design system changed: every theme switch, override or
/// color vision change shows on the next frame, and frames without changes read no token at
/// all.
#[derive(Clone, Debug)]
pub struct StyleCache {
    /// The styles at each scale factor (its bits), with the revision of the design system
    /// they show.
    entries: Vec<(u32, u64, Styles)>,
    /// The entry last asked for.
    last: usize,
}

impl StyleCache {
    /// Resolves the styles of the active theme, at one physical pixel per logical pixel.
    pub fn new(design: &DesignSystem) -> Self {
        Self {
            entries: vec![(
                1.0_f32.to_bits(),
                design.revision(),
                Styles::resolve(design),
            )],
            last: 0,
        }
    }

    /// The styles of the active theme at `scale` physical pixels per logical pixel, resolved
    /// again if `design` changed since they were.
    pub fn update(&mut self, design: &DesignSystem, scale: f32) -> &Styles {
        let key = scale.to_bits();
        let index = match self.entries.iter().position(|entry| entry.0 == key) {
            Some(index) => index,
            None if self.entries.len() < SCALES_KEPT => {
                self.entries
                    .push((key, design.revision(), Styles::resolve_at(design, scale)));
                self.entries.len() - 1
            }
            // Full: a scale not shown last gives its memory to the new one.
            None => {
                let index = usize::from(self.last == 0);
                let entry = &mut self.entries[index];
                entry.0 = key;
                entry.1 = design.revision();
                entry.2.refresh_at(design, scale);
                index
            }
        };
        self.last = index;
        let (_, revision, styles) = &mut self.entries[index];
        if *revision != design.revision() {
            *revision = design.revision();
            styles.refresh_at(design, scale);
        }
        styles
    }

    /// The styles last asked for.
    pub fn styles(&self) -> &Styles {
        &self.entries[self.last].2
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
            styles.number_field.fill.get(crate::ControlState::Normal),
            design.get(component::NUMBER_FIELD_FILL_COLOR_DEFAULT)
        );
        assert_eq!(styles.checkbox.box_size, 16.0);
        assert_eq!(
            styles.icon,
            IconStyle {
                size: 16.0,
                stroke_width: 1.5,
                chevron_stroke_width: 1.25,
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
        assert_eq!(accent(cache.update(&design, 1.0)), before);

        let red = Color::from_hex(0xFF_00_00);
        design
            .set_value(Scope::Global, semantic::BACKGROUND_ACCENT_DEFAULT, red)
            .unwrap();
        // The accent button aliases the semantic token: the change flows to its style.
        assert_eq!(accent(cache.update(&design, 1.0)), red);
    }

    #[test]
    fn layout_lengths_are_whole_physical_pixels_at_each_scale() {
        let design = design();
        let whole = |length: f32, scale: f32| {
            let pixels = length * scale;
            (pixels - pixels.round()).abs() < 1e-4
        };
        for scale in [1.0, 1.25, 1.5, 1.75, 2.25] {
            let styles = Styles::resolve_at(&design, scale);
            let panel = styles.collapsible_panel;
            for length in [
                panel.header_height,
                panel.header_inset,
                panel.gap,
                panel.indent,
                panel.content_padding,
                styles.button.height,
                styles.menu.item_height,
                styles.workspace.tab_bar_height,
                styles.property_group.row_gap,
            ] {
                assert!(whole(length, scale), "{length} at {scale}");
            }
            // What a header holds keeps its own height, the inset around it whole too.
            let control = design.get(component::PANEL_HEADER_HEIGHT_DEFAULT);
            assert!(
                (panel.header_height - 2.0 * panel.header_inset - whole_pixels(control, scale))
                    .abs()
                    < 1e-4
            );
            // Strokes stay as the tokens say.
            assert_eq!(
                styles.icon.chevron_stroke_width,
                Styles::resolve(&design).icon.chevron_stroke_width
            );
        }
    }

    #[test]
    fn the_cache_keeps_styles_for_each_scale_it_is_asked_for() {
        let mut design = design();
        let mut cache = StyleCache::new(&design);
        let inset = |styles: &Styles| styles.collapsible_panel.header_inset;
        let at_one = inset(cache.update(&design, 1.0));
        let at_five_quarters = inset(cache.update(&design, 1.25));
        assert!(
            (at_five_quarters * 1.25 - 2.0).abs() < 1e-4,
            "{at_five_quarters}"
        );
        assert_eq!(inset(cache.update(&design, 1.0)), at_one);
        assert_eq!(inset(cache.styles()), at_one);
        // More scales than kept: the oldest ones give way, the one asked for is right.
        for scale in [1.5, 1.75, 2.0, 2.25, 2.5] {
            assert!((inset(cache.update(&design, scale)) * scale).fract().abs() < 1e-4);
        }
        assert!(cache.entries.len() <= SCALES_KEPT);
        // A change shows at every scale.
        let red = Color::from_hex(0xFF_00_00);
        design
            .set_value(Scope::Global, semantic::BACKGROUND_ACCENT_DEFAULT, red)
            .unwrap();
        let accent = |styles: &Styles| styles.button.colors(Tone::ACCENT).background.normal;
        assert_eq!(accent(cache.update(&design, 1.25)), red);
        assert_eq!(accent(cache.update(&design, 2.5)), red);
    }
}
