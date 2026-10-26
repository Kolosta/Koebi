//! The components: one module per component, holding its style and its builder or drawing
//! function, and the parts they share ([`parts`]).
//!
//! A component is a builder shown in a rectangle ([`Button`]) or a method of
//! [`Ui`](crate::Ui) ([`Ui::menu`](crate::Ui::menu)): it interacts, draws with its style and
//! the call's overrides, and returns what happened. Styles are plain data resolved from
//! tokens, so a component never reads the design system while drawing.

mod button;
mod checkbox;
mod drag_value;
mod dropdown;
mod list;
mod menu;
mod panel;
mod parts;
mod scroll;
mod separator;
mod slider;
mod swatch;
mod theme_card;
mod tooltip;

pub use button::{Activation, Button, ButtonGroup, ButtonStyle, GroupCell, GroupResponse, Track};
pub use checkbox::{Checkbox, CheckboxStyle};
pub use drag_value::DragValueStyle;
pub use dropdown::{Dropdown, DropdownStyle};
pub use list::{
    DropPlace, ItemMark, LIST, LIST_ACTIONS, LIST_DRAG, LIST_DRAG_CANCEL, LIST_EXTEND_NEXT,
    LIST_EXTEND_PREVIOUS, LIST_FOLD, LIST_NEXT, LIST_PREVIOUS, LIST_SELECT_ALL, LIST_UNFOLD,
    ListContextMenu, ListDelegate, ListItem, ListOptions, ListResponse, ListSelection, ListState,
    ListStyle,
};
pub use menu::{
    MENU, MENU_ACTIONS, MENU_CHOOSE, MENU_CLOSE, MENU_CLOSE_SUBMENU, MENU_NEXT, MENU_OPEN_SUBMENU,
    MENU_PREVIOUS, Mark, MenuAction, MenuChoice, MenuItem, MenuState, MenuStyle, Submenu,
};
pub use panel::{PanelLevel, PanelStyle};
pub use parts::{
    Alignment, Content, ControlState, Hue, HueColors, Placed, Placement, Shades, Side, States,
    Tone, ToneColors, Variant,
};
pub use scroll::{SCROLL, ScrollAxes, ScrollContent, ScrollState, ScrollStyle};
pub use separator::SeparatorStyle;
pub use slider::SliderStyle;
pub use swatch::SwatchStyle;
pub use theme_card::{ThemeCardStyle, ThemePreview};
pub(crate) use tooltip::TooltipState;
pub use tooltip::TooltipStyle;

use koebi_core::input::Key;
use koebi_core::{Rect, Vec2};
use koebi_input::{KeyTrigger, Shortcut, Trigger};

/// The press of `key`, repeating while it is held: moving through items.
pub(crate) const fn repeating(key: Key) -> Shortcut {
    Shortcut::new(Trigger::Key {
        key,
        when: KeyTrigger::Press,
        repeat: true,
    })
}

/// `rect` with its origin and size rounded to whole logical pixels, so that its edges stay
/// crisp and its size does not depend on where it lands.
pub(crate) fn snap(rect: Rect) -> Rect {
    let round = |v: Vec2| Vec2::new(v.x.round(), v.y.round());
    Rect::from_origin_size(round(rect.min), round(rect.size()))
}
