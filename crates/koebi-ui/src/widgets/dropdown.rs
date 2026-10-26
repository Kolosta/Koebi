//! Dropdowns: a control showing the current choice that opens a menu of choices beside it.
//!
//! A [`Dropdown`] is a menu button ([`Button::menu`]) drawn with the dropdowns' tokens
//! (`component.dropdown.*`, darker than buttons), ending with a chevron that turns while its
//! menu is open. The menu opens below by default, at least as wide as the control, and flips
//! when the window has no room ([`Placement`]); a placement beside the control makes a
//! vertical dropdown. Where menu and control meet, both square their corners. To fuse buttons
//! on its sides, put a menu button in a [`ButtonGroup`](crate::ButtonGroup).

use koebi_core::{Rect, Vec2};
use koebi_design::Resolve;
use koebi_design::kind;
use koebi_tokens::component;

use crate::icon::Icon;
use crate::id::WidgetId;
use crate::style::{CornerRadii, Prop};
use crate::text::Align;
use crate::ui::Ui;
use crate::widgets::button::Button;
use crate::widgets::menu::{MenuChoice, MenuItem, MenuState};
use crate::widgets::parts::{ControlState, Placement, Side, States, Tone, ToneColors};

/// Colors and metrics of dropdowns, from the `component.dropdown.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropdownStyle {
    /// Colors in every state; selected is open.
    pub colors: ToneColors,
    /// Height.
    pub height: f32,
    /// Smallest width of a dropdown fitting its content.
    pub min_width: f32,
    /// Space between the sides and the content.
    pub padding: f32,
    /// Space above and below the content, at least.
    pub padding_vertical: f32,
    /// Space between the parts of the content.
    pub gap: f32,
    /// Side of icons.
    pub icon_size: f32,
    /// Side of the chevron.
    pub chevron_size: f32,
    /// Corner radius.
    pub corner_radius: f32,
    /// Border width.
    pub border_width: f32,
}

impl DropdownStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        let get = |token| tokens.get(token);
        let open = get(c::DROPDOWN_BACKGROUND_COLOR_OPEN);
        let border_open = get(c::DROPDOWN_BORDER_COLOR_OPEN);
        let border_hover = get(c::DROPDOWN_BORDER_COLOR_HOVER);
        Self {
            colors: ToneColors {
                background: States {
                    normal: get(c::DROPDOWN_BACKGROUND_COLOR_DEFAULT),
                    hovered: get(c::DROPDOWN_BACKGROUND_COLOR_HOVER),
                    pressed: get(c::DROPDOWN_BACKGROUND_COLOR_PRESSED),
                    selected: open,
                    selected_hovered: open,
                    disabled: get(c::DROPDOWN_BACKGROUND_COLOR_DISABLED),
                },
                border: States {
                    normal: get(c::DROPDOWN_BORDER_COLOR_DEFAULT),
                    hovered: border_hover,
                    pressed: border_hover,
                    selected: border_open,
                    selected_hovered: border_open,
                    disabled: get(c::DROPDOWN_BORDER_COLOR_DISABLED),
                },
                content: States {
                    disabled: get(c::DROPDOWN_LABEL_COLOR_DISABLED),
                    ..States::all(get(c::DROPDOWN_LABEL_COLOR_DEFAULT))
                },
                icon: States {
                    disabled: get(c::DROPDOWN_ICON_COLOR_DISABLED),
                    ..States::all(get(c::DROPDOWN_ICON_COLOR_DEFAULT))
                },
            },
            height: tokens.get(c::DROPDOWN_SIZE_HEIGHT),
            min_width: tokens.get(c::DROPDOWN_MIN_WIDTH_DEFAULT),
            padding: tokens.get(c::DROPDOWN_PADDING_HORIZONTAL_DEFAULT),
            padding_vertical: tokens.get(c::DROPDOWN_PADDING_VERTICAL_DEFAULT),
            gap: tokens.get(c::DROPDOWN_GAP_DEFAULT),
            icon_size: tokens.get(c::DROPDOWN_ICON_SIZE),
            chevron_size: tokens.get(c::DROPDOWN_CHEVRON_SIZE),
            corner_radius: tokens.get(c::DROPDOWN_CORNER_RADIUS_DEFAULT),
            border_width: tokens.get(c::DROPDOWN_BORDER_WIDTH_DEFAULT),
        }
    }
}

/// The chevron of a dropdown whose menu opens on `side`: pointing there while closed, and
/// back towards the control from the side the menu opened on (`open`) while it is open.
pub(crate) fn chevron(side: Side, open: Option<Side>) -> Icon {
    let pointing = |side: Side| match side {
        Side::Below => Icon::ChevronDown,
        Side::Above => Icon::ChevronUp,
        Side::Right => Icon::ChevronRight,
        Side::Left => Icon::ChevronLeft,
    };
    match open {
        Some(side) => pointing(side.opposite()),
        None => pointing(side),
    }
}

/// The widget key of a dropdown's menu.
const MENU_KEY: u64 = u64::MAX - 1;

/// A dropdown, configured then shown with [`Dropdown::show`]: a control showing the current
/// choice (mark it in the items with [`MenuItem::selected`]) that opens the menu of its items.
#[derive(Debug)]
pub struct Dropdown<'a> {
    id: WidgetId,
    button: Button<'a>,
    items: &'a [MenuItem<'a>],
}

impl<'a> Dropdown<'a> {
    /// A dropdown identified by `id`, opening `items` with the menu `state`.
    pub fn new(id: WidgetId, state: &'a mut MenuState, items: &'a [MenuItem<'a>]) -> Self {
        Self {
            id,
            button: Button::new(id)
                .dropdown_look()
                .align(Align::Start)
                .trailing_icon(Icon::ChevronDown)
                .menu(state, Placement::BELOW),
            items,
        }
    }

    /// Applies `change` to the control.
    fn with(mut self, change: impl FnOnce(Button<'a>) -> Button<'a>) -> Self {
        self.button = change(self.button);
        self
    }

    /// Shows `label`: the current choice.
    #[must_use]
    pub fn label(self, label: &'a str) -> Self {
        self.with(|button| button.label(label))
    }

    /// Shows `icon` before the label.
    #[must_use]
    pub fn leading_icon(self, icon: Icon) -> Self {
        self.with(|button| button.leading_icon(icon))
    }

    /// Shows `icon` in place of the label, or after it.
    #[must_use]
    pub fn icon(self, icon: Icon) -> Self {
        self.with(|button| button.icon(icon))
    }

    /// Places the content at the start (the default), center or end.
    #[must_use]
    pub fn align(self, align: Align) -> Self {
        self.with(|button| button.align(align))
    }

    /// Where the menu opens; [`Placement::RIGHT`] makes a vertical dropdown.
    #[must_use]
    pub fn placement(self, placement: Placement) -> Self {
        self.with(|button| button.menu_placement_to(placement))
    }

    /// Colors the control by `tone` in place of the dropdowns' colors.
    #[must_use]
    pub fn tone(self, tone: Tone) -> Self {
        self.with(|button| button.tone(tone))
    }

    /// Makes the dropdown available or not.
    #[must_use]
    pub fn enabled(self, enabled: bool) -> Self {
        self.with(|button| button.enabled(enabled))
    }

    /// A fixed width in place of the width of the content.
    #[must_use]
    pub fn width(self, width: impl Into<Prop<kind::Dimension>>) -> Self {
        self.with(|button| button.width(width))
    }

    /// The smallest width a fitting dropdown measures.
    #[must_use]
    pub fn min_width(self, width: impl Into<Prop<kind::Dimension>>) -> Self {
        self.with(|button| button.min_width(width))
    }

    /// The largest width a fitting dropdown measures; its label is cut beyond.
    #[must_use]
    pub fn max_width(self, width: impl Into<Prop<kind::Dimension>>) -> Self {
        self.with(|button| button.max_width(width))
    }

    /// The height [`Dropdown::size`] measures.
    #[must_use]
    pub fn height(self, height: impl Into<Prop<kind::Dimension>>) -> Self {
        self.with(|button| button.height(height))
    }

    /// The space between the sides and the content.
    #[must_use]
    pub fn padding(self, padding: impl Into<Prop<kind::Dimension>>) -> Self {
        self.with(|button| button.padding(padding))
    }

    /// The same radius for every corner.
    #[must_use]
    pub fn corner_radius(self, radius: impl Into<Prop<kind::Dimension>>) -> Self {
        self.with(|button| button.corner_radius(radius))
    }

    /// A radius per corner.
    #[must_use]
    pub fn corner_radii(self, radii: impl Into<CornerRadii>) -> Self {
        self.with(|button| button.corner_radii(radii))
    }

    /// The border width.
    #[must_use]
    pub fn border_width(self, width: impl Into<Prop<kind::Dimension>>) -> Self {
        self.with(|button| button.border_width(width))
    }

    /// The background in `state` (selected: open).
    #[must_use]
    pub fn background(self, state: ControlState, color: impl Into<Prop<kind::Color>>) -> Self {
        self.with(|button| button.background(state, color))
    }

    /// The border color in `state` (selected: open).
    #[must_use]
    pub fn border_color(self, state: ControlState, color: impl Into<Prop<kind::Color>>) -> Self {
        self.with(|button| button.border_color(state, color))
    }

    /// The color of the label and icons in `state`.
    #[must_use]
    pub fn content_color(self, state: ControlState, color: impl Into<Prop<kind::Color>>) -> Self {
        self.with(|button| button.content_color(state, color))
    }

    /// The color of the icons and the chevron in `state`.
    #[must_use]
    pub fn icon_color(self, state: ControlState, color: impl Into<Prop<kind::Color>>) -> Self {
        self.with(|button| button.icon_color(state, color))
    }

    /// The size the dropdown asks for: one interface unit high, as wide as its content and
    /// chevron within its limits.
    pub fn size(&self, ui: &mut Ui<'_>) -> Vec2 {
        self.button.size(ui)
    }

    /// Shows the dropdown in `rect` and its menu when open, kept inside `bounds` (the
    /// window); returns the item chosen this frame.
    pub fn show(self, ui: &mut Ui<'_>, rect: Rect, bounds: Rect) -> Option<MenuChoice> {
        let (_, state) = self.button.show_with_menu(ui, rect);
        ui.menu(self.id.with(MENU_KEY), state?, self.items, bounds)
    }
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;
    use koebi_text::Fonts;

    use super::*;
    use crate::{Input, Styles, UiState};

    const ID: WidgetId = WidgetId::new("editor type");

    fn bounds() -> Rect {
        Rect::from_origin_size(Vec2::ZERO, Vec2::new(400.0, 300.0))
    }

    fn control() -> Rect {
        Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::new(120.0, 20.0))
    }

    fn items() -> [MenuItem<'static>; 3] {
        [
            MenuItem::action("Viewport").selected(true),
            MenuItem::action("Outliner"),
            MenuItem::action("Timeline"),
        ]
    }

    struct Harness {
        state: UiState,
        menu: MenuState,
        input: Input,
        styles: Styles,
    }

    impl Harness {
        fn new() -> Self {
            let mut state = UiState::new();
            state.set_fonts(Fonts::with_embedded());
            Self {
                state,
                menu: MenuState::new(),
                input: Input::new(),
                styles: crate::test_styles(),
            }
        }

        fn frame(&mut self, placement: Placement, at: Rect) -> (Option<MenuChoice>, Scene) {
            let mut scene = Scene::new();
            let items = items();
            let chosen = {
                let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
                Dropdown::new(ID, &mut self.menu, &items)
                    .label("Viewport")
                    .placement(placement)
                    .show(&mut ui, at, bounds())
            };
            self.input.end_frame();
            (chosen, scene)
        }

        fn click(&mut self, at: Vec2) {
            self.input.primary_button(true, at);
            self.input.primary_button(false, at);
        }
    }

    #[test]
    fn a_click_opens_the_menu_below_and_a_choice_closes_it() {
        let mut harness = Harness::new();
        let (_, closed) = harness.frame(Placement::BELOW, control());
        // The control, its chevron and its label.
        assert_eq!((closed.rect_count(), closed.line_count()), (1, 2));
        assert!(closed.glyph_count() >= 8);
        harness.click(Vec2::new(50.0, 20.0));
        let (_, open) = harness.frame(Placement::BELOW, control());
        assert!(harness.menu.is_open());
        assert!(open.rect_count() > closed.rect_count());
        // The menu lies below the control, at least as wide: the second item ("Outliner").
        let placed = harness.menu.placed().expect("placed");
        assert_eq!(placed.rect.min, Vec2::new(10.0, 30.0));
        assert!(placed.rect.width() >= control().width());
        let style = &harness.styles.menu;
        let second = Vec2::new(
            50.0,
            placed.rect.min.y
                + style.border_width
                + style.padding_vertical
                + style.item_height * 1.5,
        );
        harness.click(second);
        let (chosen, _) = harness.frame(Placement::BELOW, control());
        assert_eq!(chosen.map(|choice| choice.index()), Some(1));
        assert!(!harness.menu.is_open());
    }

    #[test]
    fn clicking_the_control_again_closes_the_menu() {
        let mut harness = Harness::new();
        harness.frame(Placement::BELOW, control());
        harness.click(Vec2::new(50.0, 20.0));
        harness.frame(Placement::BELOW, control());
        harness.frame(Placement::BELOW, control());
        assert!(harness.menu.is_open());
        harness.click(Vec2::new(50.0, 20.0));
        harness.frame(Placement::BELOW, control());
        assert!(!harness.menu.is_open());
    }

    #[test]
    fn the_chevron_points_where_the_menu_opens_and_turns_while_open() {
        assert_eq!(chevron(Side::Below, None), Icon::ChevronDown);
        assert_eq!(chevron(Side::Below, Some(Side::Below)), Icon::ChevronUp);
        // Flipped above: it points down, back to the control.
        assert_eq!(chevron(Side::Below, Some(Side::Above)), Icon::ChevronDown);
        assert_eq!(chevron(Side::Right, None), Icon::ChevronRight);
        assert_eq!(chevron(Side::Right, Some(Side::Left)), Icon::ChevronRight);
    }

    #[test]
    fn menus_follow_their_control_and_flip_near_the_edges() {
        let mut harness = Harness::new();
        harness.frame(Placement::BELOW, control());
        harness.click(Vec2::new(50.0, 20.0));
        harness.frame(Placement::BELOW, control());
        // The layout moves the control (and the pointer on it) to the bottom of the window: the
        // menu follows and opens above.
        let low = Rect::from_origin_size(Vec2::new(10.0, 270.0), Vec2::new(120.0, 20.0));
        harness.input.pointer_moved(low.center());
        harness.frame(Placement::BELOW, low);
        harness.frame(Placement::BELOW, low);
        let placed = harness.menu.placed().expect("placed");
        assert_eq!(placed.side, Side::Above);
        assert_eq!(placed.rect.max.y, low.min.y);
    }

    #[test]
    fn vertical_dropdowns_open_beside_the_control() {
        let mut harness = Harness::new();
        harness.frame(Placement::RIGHT, control());
        harness.click(Vec2::new(50.0, 20.0));
        harness.frame(Placement::RIGHT, control());
        let placed = harness.menu.placed().expect("placed");
        assert_eq!(placed.side, Side::Right);
        assert_eq!(placed.rect.min, Vec2::new(control().max.x, control().min.y));
    }

    #[test]
    fn open_dropdowns_show_their_open_colors() {
        let colors = crate::test_styles().dropdown.colors;
        assert_ne!(
            colors.background.get(ControlState::Selected),
            colors.background.get(ControlState::Normal)
        );
    }
}
