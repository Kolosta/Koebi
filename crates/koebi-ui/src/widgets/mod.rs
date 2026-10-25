//! The components: one module per component, holding its style and its drawing function.
//!
//! A component is a method of [`Ui`](crate::Ui): it interacts, draws with its style and
//! returns a [`Response`](crate::Response). Styles are plain data resolved from tokens, so a
//! component never reads a token while drawing.

mod button;
mod checkbox;
mod drag_value;
mod dropdown;
mod menu;
mod panel;
mod scroll;
mod separator;
mod slider;
mod swatch;
mod theme_card;
mod toggle;

pub use button::ButtonStyle;
pub use checkbox::CheckboxStyle;
pub use drag_value::DragValueStyle;
pub use dropdown::DropdownStyle;
pub use menu::{
    MENU, MENU_ACTIONS, MENU_CHOOSE, MENU_CLOSE, MENU_NEXT, MENU_PREVIOUS, MenuAction, MenuItem,
    MenuState, MenuStyle,
};
pub use panel::{PanelLevel, PanelStyle};
pub use scroll::{SCROLL, ScrollAxes, ScrollContent, ScrollState, ScrollStyle};
pub use separator::SeparatorStyle;
pub use slider::SliderStyle;
pub use swatch::SwatchStyle;
pub use theme_card::{ThemeCardStyle, ThemePreview};
pub use toggle::ToggleStyle;

use koebi_core::{Rect, Vec2};

/// `rect` with its origin and size rounded to whole logical pixels, so that its edges stay
/// crisp and its size does not depend on where it lands.
pub(crate) fn snap(rect: Rect) -> Rect {
    let round = |v: Vec2| Vec2::new(v.x.round(), v.y.round());
    Rect::from_origin_size(round(rect.min), round(rect.size()))
}
