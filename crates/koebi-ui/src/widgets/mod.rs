//! The components: one module per component, holding its style and its builder or drawing
//! function, and the parts they share ([`parts`]).
//!
//! A component is a builder shown in a rectangle ([`Button`]) or a method of
//! [`Ui`](crate::Ui) ([`Ui::menu`](crate::Ui::menu)): it interacts, draws with its style and
//! the call's overrides, and returns what happened. Styles are plain data resolved from
//! tokens, so a component never reads the design system while drawing.

mod app_bar;
mod button;
mod capture_field;
mod checkbox;
mod collapsible;
mod dropdown;
mod key_caps;
mod list;
mod menu;
mod menu_bar;
mod number_field;
mod panel;
mod parts;
mod property_group;
mod reset_dot;
mod scroll;
mod scrollbar;
mod separator;
mod swatch;
mod text_field;
mod theme_card;
mod tooltip;

pub use app_bar::{AppBar, AppBarResponse, AppBarStyle, WindowCommand, WindowGlyph};
pub use button::{Activation, Button, ButtonGroup, ButtonStyle, GroupCell, GroupResponse, Track};
pub use capture_field::{
    CAPTURE, Capture, CaptureField, CaptureFieldStyle, CaptureResponse, CaptureState,
};
pub use checkbox::{Checkbox, CheckboxStyle};
pub use collapsible::{
    CollapsiblePanel, CollapsiblePanelResponse, CollapsiblePanelState, CollapsiblePanelStyle,
};
pub use dropdown::{Dropdown, DropdownStyle};
pub use key_caps::{Cap, CapLabel, KEY_SIGNS, KeyCapStyle, Keyboard, ShortcutCaps, key_cap};
pub use list::{
    DropPlace, ItemMark, LIST, LIST_ACTIONS, LIST_DRAG, LIST_DRAG_CANCEL, LIST_EXTEND_NEXT,
    LIST_EXTEND_PREVIOUS, LIST_FOLD, LIST_NEXT, LIST_PREVIOUS, LIST_SELECT_ALL, LIST_UNFOLD,
    ListContextMenu, ListDelegate, ListItem, ListOptions, ListResponse, ListSelection, ListState,
    ListStyle, TreeGuide,
};
pub use menu::{
    MENU, MENU_ACTIONS, MENU_CHOOSE, MENU_CLOSE, MENU_CLOSE_SUBMENU, MENU_NEXT, MENU_OPEN_SUBMENU,
    MENU_PREVIOUS, MenuAction, MenuChoice, MenuItem, MenuMark, MenuState, MenuStyle, Submenu,
};
pub use menu_bar::{MenuBarEntry, MenuBarState};
pub(crate) use number_field::NumberState;
pub use number_field::{
    BarMapping, FineMode, NUMBER, NUMBER_ACTIONS, NUMBER_CANCEL, NUMBER_CONFIRM, NumberField,
    NumberFieldResponse, NumberFieldStyle, Numeric,
};
pub use panel::{PanelLevel, PanelStyle};
pub use parts::{
    Content, ControlState, Hue, HueColors, PanelAlign, Placed, Placement, Shades, Side, States,
    Tone, ToneColors, Variant,
};
pub use property_group::{PropertyGroup, PropertyGroupStyle};
pub use reset_dot::ResetDotStyle;
pub use scroll::{SCROLL, ScrollAxes, ScrollContent, ScrollState, ScrollStyle};
pub use scrollbar::{ScrollAxis, Scrollbar, ScrollbarResponse, ScrollbarState};
pub use separator::SeparatorStyle;
pub use swatch::SwatchStyle;
pub(crate) use text_field::TextState;
pub use text_field::{
    EditEnd, OutsidePress, SelectOnFocus, TEXT, TEXT_ACTIONS, TEXT_CANCEL, TEXT_CONFIRM, TEXT_COPY,
    TEXT_CUT, TEXT_DELETE_BACKWARD, TEXT_DELETE_FORWARD, TEXT_DELETE_WORD_BACKWARD,
    TEXT_DELETE_WORD_FORWARD, TEXT_END, TEXT_LEFT, TEXT_NEXT_FIELD, TEXT_PASTE,
    TEXT_PREVIOUS_FIELD, TEXT_REDO, TEXT_RIGHT, TEXT_SELECT_ALL, TEXT_SELECT_END, TEXT_SELECT_LEFT,
    TEXT_SELECT_RIGHT, TEXT_SELECT_START, TEXT_SELECT_WORD_LEFT, TEXT_SELECT_WORD_RIGHT,
    TEXT_START, TEXT_UNDO, TEXT_WORD_LEFT, TEXT_WORD_RIGHT, TextField, TextFieldResponse,
    TextFieldStyle,
};
pub use theme_card::{CardResponse, ThemeCardStyle, ThemePreview};
pub(crate) use tooltip::TooltipState;
pub use tooltip::TooltipStyle;

use koebi_core::Rect;
use koebi_core::input::Key;
use koebi_input::{KeyTrigger, Shortcut, Trigger};

/// The press of `key`, repeating while it is held: moving through items.
pub(crate) const fn repeating(key: Key) -> Shortcut {
    Shortcut::new(Trigger::Key {
        key,
        when: KeyTrigger::Press,
        repeat: true,
    })
}

/// `rect` with its origin on physical pixel edges and its size whole physical pixels at
/// `scale`, so that its edges lie where they are drawn and its size does not depend on where it
/// lands.
pub(crate) fn snap(rect: Rect, scale: f32) -> Rect {
    crate::pixel::snap_rect(rect, scale)
}
