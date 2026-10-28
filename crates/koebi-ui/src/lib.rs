//! Koebi's interface components, drawn from design tokens.
//!
//! - **Components**: a component is configured by a builder, then shown in its rectangle:
//!   `Button::new(id).label("Save").tone(Tone::ACCENT).show(ui, rect)`. Every property has a
//!   default read from the component's tokens, and a call can replace it with another token or
//!   a plain value ([`Prop`]), state by state where it changes with the state
//!   ([`ControlState`]). Behaviours are options as well: when a button fires
//!   ([`Activation`]), whether it repeats while held, whether it toggles a value. Families of
//!   controls share their roles and colors ([`Tone`]: a [`Variant`] and an optional [`Hue`]).
//!   Simpler components are methods of [`Ui`] ([`Ui::slider`], [`Ui::panel`]...). ADR 0017
//!   records the design of the library.
//! - **Floating content**: menus and their submenus ([`Ui::menu`]), dropdowns ([`Dropdown`]),
//!   popups ([`Ui::popup`]) and tooltips ([`Ui::tooltip`]) draw above everything, placed beside
//!   their control by a [`Placement`] that flips when the window has no room.
//! - **Styles**: each component draws with a typed style ([`ButtonStyle`], [`SliderStyle`]...)
//!   whose fields come from its `component.*` tokens. [`Styles::resolve`] reads every style
//!   from the active theme or from any other theme ([`koebi_design::Resolve`]), and
//!   [`StyleCache`] resolves again only when the design system changes: frames read plain
//!   fields, however many components they draw.
//! - **Immediate mode**: a [`Ui`] lives for one frame. Components react to the [`Input`], draw
//!   into a [`koebi_render::Scene`] and return a [`Response`] (hovered, pressed, clicked,
//!   dragged, changed). [`UiState`] keeps what outlives a frame: the widget that holds the
//!   pointer, the last click of a double click.
//! - **Actions**: the application turns events into `koebi_input` actions through the chain
//!   of contexts under the pointer ([`UiState::chain_at`]); components read them with
//!   [`Ui::triggered`], and an open menu or an operation in progress takes the keyboard with
//!   [`Ui::grab`].
//! - **Layout**: callers give every component its rectangle, in logical pixels. A layout
//!   engine comes with the widget tree.
//! - **Text**: labels come with the components that need them (menus, tabs, lists); line
//!   [`Icon`]s stand in elsewhere.
//! - **Lists**: [`Ui::list`] draws every list and tree of the application, folding,
//!   selecting and reordering rows the application identifies by its own keys.
//!
//! # Example
//!
//! ```
//! use koebi_core::{Rect, Vec2};
//! use koebi_design::DesignSystem;
//! use koebi_render::Scene;
//! use koebi_ui::{Button, Icon, Input, StyleCache, Ui, UiState, WidgetId};
//!
//! let design = DesignSystem::new(&koebi_tokens::KOEBI)?;
//! let mut styles = StyleCache::new(&design);
//! let (mut scene, mut state, mut input) = (Scene::new(), UiState::new(), Input::new());
//! let toggle = Rect::from_origin_size(Vec2::ZERO, Vec2::new(24.0, 24.0));
//! let mut grid = false;
//!
//! // One frame: components draw, and react to the input against the previous frame.
//! let mut frame = |input: &mut Input, grid: &mut bool| {
//!     scene.clear();
//!     let mut ui = Ui::new(&mut scene, styles.update(&design), input, &mut state);
//!     let response = Button::new(WidgetId::new("grid"))
//!         .icon(Icon::Plus)
//!         .toggle(grid)
//!         .show(&mut ui, toggle);
//!     drop(ui);
//!     input.end_frame();
//!     response
//! };
//!
//! frame(&mut input, &mut grid);
//! // A click on the toggle, between two frames.
//! input.primary_button(true, Vec2::new(10.0, 10.0));
//! input.primary_button(false, Vec2::new(10.0, 10.0));
//! let response = frame(&mut input, &mut grid);
//!
//! assert!(response.changed && grid);
//! # Ok::<(), koebi_design::Error>(())
//! ```

mod bar;
mod icon;
mod id;
mod input;
mod routing;
mod style;
mod text;
mod ui;
mod widgets;
mod workspace;

pub use bar::BarLayout;
pub use icon::Icon;
pub use id::WidgetId;
pub use input::{Input, PointerButton};
pub use routing::Layer;
pub use style::{CornerRadii, IconStyle, InputStyle, Prop, StateColors, StyleCache, Styles};
pub use text::{Align, FontStyle, TextStyles};
pub use ui::{Response, Ui, UiState, WidgetState};
pub use widgets::{
    Activation, BarMapping, Button, ButtonGroup, ButtonStyle, Checkbox, CheckboxStyle, Content,
    ControlState, DropPlace, Dropdown, DropdownStyle, EditEnd, GroupCell, GroupResponse, Hue,
    HueColors, ItemMark, LIST, LIST_ACTIONS, LIST_DRAG, LIST_DRAG_CANCEL, LIST_EXTEND_NEXT,
    LIST_EXTEND_PREVIOUS, LIST_FOLD, LIST_NEXT, LIST_PREVIOUS, LIST_SELECT_ALL, LIST_UNFOLD,
    ListContextMenu, ListDelegate, ListItem, ListOptions, ListResponse, ListSelection, ListState,
    ListStyle, MENU, MENU_ACTIONS, MENU_CHOOSE, MENU_CLOSE, MENU_CLOSE_SUBMENU, MENU_NEXT,
    MENU_OPEN_SUBMENU, MENU_PREVIOUS, MenuAction, MenuChoice, MenuItem, MenuMark, MenuState,
    MenuStyle, NUMBER, NUMBER_ACTIONS, NUMBER_CANCEL, NUMBER_CONFIRM, NumberField,
    NumberFieldResponse, NumberFieldStyle, Numeric, OutsidePress, PanelAlign, PanelLevel,
    PanelStyle, Placed, Placement, PropertyGroup, PropertyGroupStyle, SCROLL, ScrollAxes,
    ScrollContent, ScrollState, ScrollStyle, SelectOnFocus, SeparatorStyle, Shades, Side,
    SliderStyle, States, Submenu, SwatchStyle, TEXT, TEXT_ACTIONS, TEXT_CANCEL, TEXT_CONFIRM,
    TEXT_COPY, TEXT_CUT, TEXT_DELETE_BACKWARD, TEXT_DELETE_FORWARD, TEXT_DELETE_WORD_BACKWARD,
    TEXT_DELETE_WORD_FORWARD, TEXT_END, TEXT_LEFT, TEXT_NEXT_FIELD, TEXT_PASTE,
    TEXT_PREVIOUS_FIELD, TEXT_REDO, TEXT_RIGHT, TEXT_SELECT_ALL, TEXT_SELECT_END, TEXT_SELECT_LEFT,
    TEXT_SELECT_RIGHT, TEXT_SELECT_START, TEXT_SELECT_WORD_LEFT, TEXT_SELECT_WORD_RIGHT,
    TEXT_START, TEXT_UNDO, TEXT_WORD_LEFT, TEXT_WORD_RIGHT, TextField, TextFieldResponse,
    TextFieldStyle, ThemeCardStyle, ThemePreview, Tone, ToneColors, TooltipStyle, Track, Variant,
};
pub use workspace::{
    EditorHost, OPERATION_CANCEL, OPERATION_CONFIRM, WORKSPACE, WORKSPACE_ACTIONS,
    WORKSPACE_OPERATION, WorkspaceStyle, WorkspaceView,
};

/// The styles of Koebi's base theme, for unit tests.
#[cfg(test)]
pub(crate) fn test_styles() -> Styles {
    let design =
        koebi_design::DesignSystem::new(&koebi_tokens::KOEBI).expect("Koebi's schema is valid");
    Styles::resolve(&design)
}
