//! The application bar: the title bar a window draws for itself.
//!
//! An [`AppBar`] fills the top of a window: the caller's items from the start (a [`Ui::menu_bar`]
//! usually), the window's title in the middle, and the window buttons at the end (minimize,
//! maximize or restore, close), flush with the window's corner and as high as the bar.
//!
//! The system moves and resizes the window: the bar only says where it is. Its background is a
//! widget ([`AppBarResponse::drag`]), and what lies over it takes the pointer
//! ([`UiState::covering`](crate::UiState::covering)): the application gives both to the platform.
//!
//! Glyphs are drawn on whole physical pixels, so that their straight lines stay sharp at every
//! scale ([`UiState::set_scale_factor`](crate::UiState::set_scale_factor)).

use koebi_core::{Color, Rect, Vec2};
use koebi_design::Resolve;
use koebi_render::{LinePrimitive, RectPrimitive};
use koebi_tokens::component;

use crate::bar::BarLayout;
use crate::id::WidgetId;
use crate::text::Align;
use crate::ui::{Response, Ui};
use crate::widgets::parts::shade;

/// Metrics and colors of application bars, from the `component.title-bar.*` tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppBarStyle {
    /// Background of the bar.
    pub background: Color,
    /// Color of the title.
    pub title: Color,
    /// Color of the window buttons' glyphs.
    pub icon: Color,
    /// Background of a window button under the pointer.
    pub button_hover: Color,
    /// Background of the close button under the pointer.
    pub close_hover: Color,
    /// Lightness added to a window button held down: negative, it darkens.
    pub pressed_lightness: f32,
    /// Space between the bar's edges and its items.
    pub padding: f32,
    /// Space between the items.
    pub gap: f32,
    /// Width of a window button.
    pub button_width: f32,
    /// Side of a window button's glyph.
    pub glyph_size: f32,
    /// Width of a glyph's lines.
    pub glyph_stroke: f32,
    /// How far the back square of the restore glyph lies from the front one.
    pub restore_offset: f32,
    /// How far from the window's edges a press resizes it.
    pub resize_border: f32,
    /// Space between the sides of a menu's title and its label.
    pub menu_padding: f32,
    /// Corner radius of a menu's title.
    pub menu_radius: f32,
    /// Color of a menu's title.
    pub menu_title: Color,
    /// Height of the items, one interface unit.
    pub item_height: f32,
}

impl AppBarStyle {
    /// Reads the style from `tokens`.
    pub fn resolve(tokens: &impl Resolve) -> Self {
        use component as c;
        Self {
            background: tokens.get(c::TITLE_BAR_BACKGROUND_COLOR_DEFAULT),
            title: tokens.get(c::TITLE_BAR_LABEL_COLOR_DEFAULT),
            icon: tokens.get(c::TITLE_BAR_ICON_COLOR_DEFAULT),
            button_hover: tokens.get(c::TITLE_BAR_BUTTON_BACKGROUND_COLOR_HOVER),
            close_hover: tokens.get(c::TITLE_BAR_CLOSE_BACKGROUND_COLOR_HOVER),
            pressed_lightness: tokens.get(c::TITLE_BAR_BUTTON_PRESSED_LIGHTNESS),
            padding: tokens.get(c::TITLE_BAR_PADDING_DEFAULT),
            gap: tokens.get(c::TITLE_BAR_GAP_DEFAULT),
            button_width: tokens.get(c::TITLE_BAR_BUTTON_WIDTH_DEFAULT),
            glyph_size: tokens.get(c::TITLE_BAR_GLYPH_SIZE_DEFAULT),
            glyph_stroke: tokens.get(c::TITLE_BAR_GLYPH_STROKE_DEFAULT),
            restore_offset: tokens.get(c::TITLE_BAR_GLYPH_RESTORE_OFFSET),
            resize_border: tokens.get(c::TITLE_BAR_RESIZE_BORDER_DEFAULT),
            menu_padding: tokens.get(c::TITLE_BAR_MENU_PADDING_DEFAULT),
            menu_radius: tokens.get(c::TITLE_BAR_MENU_CORNER_RADIUS_DEFAULT),
            menu_title: tokens.get(c::DROPDOWN_LABEL_COLOR_DEFAULT),
            item_height: tokens.get(component::DROPDOWN_SIZE_HEIGHT),
        }
    }

    /// The height of a bar holding its items.
    pub fn height(&self) -> f32 {
        self.item_height + 2.0 * self.padding
    }
}

/// What the window buttons ask of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowCommand {
    /// Minimize it.
    Minimize,
    /// Maximize it, or bring it back: from fullscreen too.
    ToggleMaximize,
    /// Close it.
    Close,
}

/// The glyph of a window button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowGlyph {
    /// A bar across the middle.
    Minimize,
    /// A square.
    Maximize,
    /// A square in front of another.
    Restore,
    /// A cross, each branch as long as the others.
    Close,
}

/// What happened to an application bar this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppBarResponse<R> {
    /// What the caller's items returned.
    pub items: R,
    /// What a window button asked for.
    pub command: Option<WindowCommand>,
    /// The widget of the bar's background: the region the window moves from, in the frame's
    /// regions, under what lies over it.
    pub drag: WidgetId,
    /// The widget of the maximize button, and where it lies: the platform may have to know.
    pub maximize: Option<(WidgetId, Rect)>,
    /// Where the secondary button went down on the bar's background, where the platform leaves
    /// it to the application: open the window's system menu there.
    pub system_menu: Option<Vec2>,
}

/// An application bar, configured then shown with [`AppBar::show`].
#[derive(Clone, Copy, Debug)]
pub struct AppBar<'a> {
    id: WidgetId,
    title: &'a str,
    filled: bool,
    buttons: bool,
    leading: f32,
    maximize_state: (bool, bool),
}

/// The widget keys of the bar's parts.
const BUTTONS: [u64; 3] = [1, 2, 3];

impl<'a> AppBar<'a> {
    /// A bar identified by `id`, showing `title`, with window buttons.
    pub const fn new(id: WidgetId, title: &'a str) -> Self {
        Self {
            id,
            title,
            filled: false,
            buttons: true,
            leading: 0.0,
            maximize_state: (false, false),
        }
    }

    /// The window is maximized or fullscreen: its middle button brings it back.
    #[must_use]
    pub const fn filled(mut self, filled: bool) -> Self {
        self.filled = filled;
        self
    }

    /// Draws the window buttons, or not: the system draws them on macOS, and mobile platforms
    /// have none.
    #[must_use]
    pub const fn window_buttons(mut self, shown: bool) -> Self {
        self.buttons = shown;
        self
    }

    /// Shows the maximize button hovered or pressed, as the system reports it where the pointer
    /// over it does not reach the application (Windows, for its snap layouts).
    #[must_use]
    pub const fn maximize_state(mut self, hovered: bool, pressed: bool) -> Self {
        self.maximize_state = (hovered, pressed);
        self
    }

    /// Leaves `width` free at the start of the bar, for the system's window buttons.
    #[must_use]
    pub const fn leading_inset(mut self, width: f32) -> Self {
        self.leading = width;
        self
    }

    /// Shows the bar in `rect`. `add` draws its items with the layout it gets, inside the bar's
    /// padding; they lie over the bar's background and take the pointer there.
    pub fn show<R>(
        self,
        ui: &mut Ui<'_>,
        rect: Rect,
        add: impl FnOnce(&mut Ui<'_>, &mut BarLayout) -> R,
    ) -> AppBarResponse<R> {
        let style = ui.styles().app_bar;
        ui.scene()
            .push_rect(&RectPrimitive::new(rect, style.background));
        let drag = self.id;
        let background = ui.interact(drag, rect);
        let system_menu = background.secondary_pressed.then(|| {
            ui.input()
                .button_pressed_at(crate::PointerButton::Secondary)
                .unwrap_or_default()
        });
        let mut end = rect.max.x;
        let mut command = None;
        let mut maximize = None;
        if self.buttons {
            let glyphs = [
                (WindowGlyph::Close, WindowCommand::Close),
                (
                    if self.filled {
                        WindowGlyph::Restore
                    } else {
                        WindowGlyph::Maximize
                    },
                    WindowCommand::ToggleMaximize,
                ),
                (WindowGlyph::Minimize, WindowCommand::Minimize),
            ];
            for ((glyph, asked), key) in glyphs.into_iter().zip(BUTTONS) {
                let button = Rect::from_min_max(
                    Vec2::new(end - style.button_width, rect.min.y),
                    Vec2::new(end, rect.max.y),
                );
                end = button.min.x;
                let id = self.id.with(key);
                let reported = if asked == WindowCommand::ToggleMaximize {
                    maximize = Some((id, button));
                    self.maximize_state
                } else {
                    (false, false)
                };
                if Self::window_button(ui, id, button, glyph, reported).clicked {
                    command = Some(asked);
                }
            }
        }
        let padding = style.padding;
        let items = Rect::from_min_max(
            Vec2::new(rect.min.x + padding + self.leading, rect.min.y + padding),
            Vec2::new(end - padding, rect.max.y - padding),
        );
        let mut layout = BarLayout::new(items, style.gap);
        let result = add(ui, &mut layout);
        self.draw_title(ui, rect, &layout);
        AppBarResponse {
            items: result,
            command,
            drag,
            maximize,
            system_menu,
        }
    }

    /// The title in the middle of the bar, when it fits between the items.
    fn draw_title(&self, ui: &mut Ui<'_>, rect: Rect, layout: &BarLayout) {
        let style = ui.styles().app_bar;
        let font = ui.styles().text.label.clone();
        let width = ui.text_width(self.title, &font);
        let room = layout.remaining();
        let title = Rect::from_center_size(rect.center(), Vec2::new(width, rect.height()));
        if title.min.x >= room.min.x && title.max.x <= room.max.x {
            ui.label(self.title, title, &font, style.title, Align::Center);
        }
    }

    /// A window button: it acts when released over itself, as the system's do.
    fn window_button(
        ui: &mut Ui<'_>,
        id: WidgetId,
        rect: Rect,
        glyph: WindowGlyph,
        (hovered, pressed): (bool, bool),
    ) -> Response {
        let style = ui.styles().app_bar;
        let response = ui.interact(id, rect);
        let hovered = hovered || response.hovered;
        if hovered || response.held {
            let hover = if glyph == WindowGlyph::Close {
                style.close_hover
            } else {
                style.button_hover
            };
            let background = if pressed || (response.held && response.hovered) {
                shade(hover, style.pressed_lightness)
            } else {
                hover
            };
            ui.scene().push_rect(&RectPrimitive::new(rect, background));
        }
        ui.window_glyph(glyph, rect.center(), style.icon);
        response
    }
}

impl Ui<'_> {
    /// Draws the window button glyph `glyph` around `center`, on whole physical pixels.
    pub fn window_glyph(&mut self, glyph: WindowGlyph, center: Vec2, color: Color) {
        let style = self.styles().app_bar;
        let scale = self.state.scale_factor();
        // Whole pixels: what the logical values below round back to in the shader.
        let side = (style.glyph_size * scale).round().max(1.0);
        let stroke = (style.glyph_stroke * scale).floor().max(1.0);
        let offset = (style.restore_offset * scale).floor();
        let left = (center.x * scale - side * 0.5).round();
        let top = (center.y * scale - side * 0.5).round();
        let (right, bottom) = (left + side, top + side);
        let mut rect = |x0: f32, y0: f32, x1: f32, y1: f32| {
            let primitive = RectPrimitive::new(
                Rect::from_min_max(Vec2::new(x0, y0) / scale, Vec2::new(x1, y1) / scale),
                color,
            );
            self.scene.push_rect(&primitive);
        };
        match glyph {
            WindowGlyph::Minimize => {
                let middle = (center.y * scale).floor();
                rect(left, middle, right, middle + stroke);
            }
            WindowGlyph::Maximize => {
                outline(&mut rect, [left, top, right, bottom], stroke);
            }
            WindowGlyph::Restore => {
                // The front square; of the back one, only what shows above and beside it.
                outline(
                    &mut rect,
                    [left, top + offset, right - offset, bottom],
                    stroke,
                );
                rect(left + offset, top, right, top + stroke);
                rect(right - stroke, top, right, bottom - offset);
            }
            WindowGlyph::Close => {
                // From pixel centre to pixel centre: the round caps end on the box's edges, and
                // every branch is as long as the others.
                let half = stroke * 0.5;
                let (x0, x1) = (left + half, right - half);
                let (y0, y1) = (top + half, bottom - half);
                for (from, to) in [
                    (Vec2::new(x0, y0), Vec2::new(x1, y1)),
                    (Vec2::new(x0, y1), Vec2::new(x1, y0)),
                ] {
                    self.scene.push_line(&LinePrimitive::new(
                        from / scale,
                        to / scale,
                        stroke / scale,
                        color,
                    ));
                }
            }
        }
    }
}

/// The four lines of a square, as `[left, top, right, bottom]`, in physical pixels.
fn outline(rect: &mut impl FnMut(f32, f32, f32, f32), [x0, y0, x1, y1]: [f32; 4], stroke: f32) {
    rect(x0, y0, x1, y0 + stroke);
    rect(x0, y1 - stroke, x1, y1);
    rect(x0, y0 + stroke, x0 + stroke, y1 - stroke);
    rect(x1 - stroke, y0 + stroke, x1, y1 - stroke);
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;
    use koebi_text::Fonts;

    use super::*;
    use crate::{Input, MenuBarEntry, MenuBarState, MenuItem, Styles, UiState};

    const ID: WidgetId = WidgetId::new("bar");

    fn bar() -> Rect {
        Rect::from_origin_size(Vec2::ZERO, Vec2::new(600.0, 24.0))
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

        fn frame(&mut self, app_bar: AppBar<'_>) -> (AppBarResponse<()>, Scene) {
            let mut scene = Scene::new();
            let response = {
                let mut ui = Ui::new(&mut scene, &self.styles, &self.input, &mut self.state);
                app_bar.show(&mut ui, bar(), |_, _| ())
            };
            self.input.end_frame();
            (response, scene)
        }

        fn click(&mut self, at: Vec2, app_bar: AppBar<'_>) -> Option<WindowCommand> {
            self.frame(app_bar);
            self.input.pointer_moved(at);
            self.frame(app_bar);
            self.input.primary_button(true, at);
            let pressed = self.frame(app_bar).0.command;
            assert_eq!(pressed, None, "a window button acts on the release");
            self.input.primary_button(false, at);
            self.frame(app_bar).0.command
        }

        fn button(&self, index: f32) -> Vec2 {
            let width = self.styles.app_bar.button_width;
            Vec2::new(600.0 - width * (index + 0.5), 12.0)
        }
    }

    fn app_bar() -> AppBar<'static> {
        AppBar::new(ID, "Koebi")
    }

    #[test]
    fn window_buttons_ask_for_what_they_show_on_their_release() {
        let mut harness = Harness::new();
        assert_eq!(
            harness.click(harness.button(0.0), app_bar()),
            Some(WindowCommand::Close)
        );
        assert_eq!(
            harness.click(harness.button(1.0), app_bar()),
            Some(WindowCommand::ToggleMaximize)
        );
        assert_eq!(
            harness.click(harness.button(2.0), app_bar()),
            Some(WindowCommand::Minimize)
        );
    }

    #[test]
    fn a_press_leaving_a_window_button_does_not_act() {
        let mut harness = Harness::new();
        let at = harness.button(0.0);
        harness.frame(app_bar());
        harness.input.pointer_moved(at);
        harness.frame(app_bar());
        harness.input.primary_button(true, at);
        harness.frame(app_bar());
        harness.input.pointer_moved(Vec2::new(300.0, 100.0));
        harness.frame(app_bar());
        harness.input.primary_button(false, Vec2::new(300.0, 100.0));
        assert_eq!(harness.frame(app_bar()).0.command, None);
    }

    #[test]
    fn the_maximize_button_shows_what_the_system_reports() {
        let mut harness = Harness::new();
        let (plain, _) = harness.frame(app_bar());
        let (id, rect) = plain.maximize.expect("a maximize button");
        assert!(rect.contains(harness.button(1.0)));
        assert_ne!(id, plain.drag);
        // The pointer lies elsewhere: only the report can show it hovered, with one more rectangle.
        let (_, idle) = harness.frame(app_bar());
        let (_, hovered) = harness.frame(app_bar().maximize_state(true, false));
        assert_eq!(hovered.rect_count(), idle.rect_count() + 1);
        // Nothing to report without the button.
        assert_eq!(
            harness.frame(app_bar().window_buttons(false)).0.maximize,
            None
        );
    }

    #[test]
    fn the_system_draws_its_buttons_on_macos() {
        let mut harness = Harness::new();
        let bare = app_bar().window_buttons(false);
        assert_eq!(harness.click(harness.button(0.0), bare), None);
    }

    #[test]
    fn what_lies_on_the_bar_covers_its_background() {
        let mut harness = Harness::new();
        let (response, _) = harness.frame(app_bar());
        let mut covered = Vec::new();
        harness.state.covering(response.drag, &mut covered);
        // The three window buttons, nothing else: the rest of the bar moves the window.
        assert_eq!(covered.len(), 3);
        let width = harness.styles.app_bar.button_width;
        assert!(
            covered
                .iter()
                .all(|rect| (rect.width() - width).abs() < 1e-3)
        );
        assert!(
            covered
                .iter()
                .all(|rect| (rect.height() - 24.0).abs() < 1e-3)
        );
        assert!(
            covered
                .iter()
                .any(|rect| rect.contains(Vec2::new(599.0, 1.0)))
        );
    }

    #[test]
    fn items_and_floating_menus_cover_the_bar_too() {
        let mut harness = Harness::new();
        let mut menus = MenuBarState::default();
        let items = [MenuItem::action("Quit")];
        let entries = [MenuBarEntry {
            title: "File",
            items: &items,
        }];
        let window = Rect::from_origin_size(Vec2::ZERO, Vec2::new(600.0, 400.0));
        let mut frame = |harness: &mut Harness| {
            let mut scene = Scene::new();
            let drag = {
                let mut ui = Ui::new(
                    &mut scene,
                    &harness.styles,
                    &harness.input,
                    &mut harness.state,
                );
                app_bar()
                    .show(&mut ui, bar(), |ui, layout| {
                        ui.menu_bar(WidgetId::new("menus"), &mut menus, &entries, layout, window);
                    })
                    .drag
            };
            harness.input.end_frame();
            let mut covered = Vec::new();
            harness.state.covering(drag, &mut covered);
            covered
        };
        // The title of the menu and the three buttons.
        assert_eq!(frame(&mut harness).len(), 4);
        let title = Vec2::new(10.0, 12.0);
        harness.input.pointer_moved(title);
        frame(&mut harness);
        harness.input.primary_button(true, title);
        harness.input.primary_button(false, title);
        frame(&mut harness);
        // Open: what catches the presses outside it covers the whole bar.
        let covered = frame(&mut harness);
        assert!(
            covered
                .iter()
                .any(|rect| rect.contains(Vec2::new(300.0, 12.0)))
        );
    }

    /// The rectangles and lines of a glyph, as drawn at `scale`.
    fn glyph_scene(glyph: WindowGlyph, scale: f32) -> Scene {
        let mut harness = Harness::new();
        harness.state.set_scale_factor(scale);
        let mut scene = Scene::new();
        {
            let mut ui = Ui::new(
                &mut scene,
                &harness.styles,
                &harness.input,
                &mut harness.state,
            );
            ui.window_glyph(glyph, Vec2::new(23.4, 12.0), Color::WHITE);
        }
        scene
    }

    #[test]
    fn glyphs_use_the_primitives_that_keep_them_sharp() {
        // Lines of whole pixels for the straight glyphs, segments for the cross alone.
        let counts = |glyph| {
            let scene = glyph_scene(glyph, 1.25);
            (scene.rect_count(), scene.line_count())
        };
        assert_eq!(counts(WindowGlyph::Minimize), (1, 0));
        assert_eq!(counts(WindowGlyph::Maximize), (4, 0));
        assert_eq!(counts(WindowGlyph::Restore), (6, 0));
        assert_eq!(counts(WindowGlyph::Close), (0, 2));
    }
}
