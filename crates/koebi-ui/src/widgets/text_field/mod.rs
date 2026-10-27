//! Text fields: one line of text edited in place.
//!
//! - **Editing** starts with a press on the field (selecting the whole text by default, see
//!   [`SelectOnFocus`]), with Tab from the previous field, or when the application asks
//!   ([`Ui::edit_text_field`]). One field edits at a time; while it does, it takes the whole
//!   keyboard ([`TEXT`], [`Ui::grab_keyboard`]): no shortcut of another context fires, and it
//!   asks the platform for text input in its rectangle (on-screen keyboards, input methods).
//! - **Pointer** ([`editing`]): a press places the caret (Shift extends the selection), a drag
//!   selects, a double press selects a word. The text scrolls to keep the caret in view.
//! - **Keys** ([`TEXT_ACTIONS`]): moves by character, word or to the ends, with Shift to
//!   select; deletion by character or word; select all; undo and redo; clipboard. Keys and
//!   typed text apply in the order they came, even between two frames.
//! - **Ending**: Enter keeps the text, Escape restores the text the edit started with, Tab and
//!   Shift+Tab keep it and edit the next or previous enabled field, and a press outside keeps
//!   it (or restores it, see [`OutsidePress`]). The text changes as the user types:
//!   [`TextFieldResponse`] says when it changed and how the edit ended.
//! - **Clipboard**: the interface does not reach the platform; it asks the application, which
//!   copies [`UiState::copied_text`](crate::UiState::copied_text) to the clipboard and answers
//!   [`UiState::paste_requested`](crate::UiState::paste_requested) with
//!   [`UiState::set_pasted_text`](crate::UiState::set_pasted_text).
//! - **Constraints**: a character filter (digits only...) and a length limit.
//! - **Drawing** ([`draw`]) and **geometry** ([`geometry`]): the box, the icon, the text or
//!   its placeholder, the selection and the caret.

mod actions;
mod draw;
mod edit;
mod editing;
mod geometry;
mod session;
mod style;

#[cfg(test)]
mod tests;

pub use actions::{
    TEXT, TEXT_ACTIONS, TEXT_CANCEL, TEXT_CONFIRM, TEXT_COPY, TEXT_CUT, TEXT_DELETE_BACKWARD,
    TEXT_DELETE_FORWARD, TEXT_DELETE_WORD_BACKWARD, TEXT_DELETE_WORD_FORWARD, TEXT_END, TEXT_LEFT,
    TEXT_NEXT_FIELD, TEXT_PASTE, TEXT_PREVIOUS_FIELD, TEXT_REDO, TEXT_RIGHT, TEXT_SELECT_ALL,
    TEXT_SELECT_END, TEXT_SELECT_LEFT, TEXT_SELECT_RIGHT, TEXT_SELECT_START, TEXT_SELECT_WORD_LEFT,
    TEXT_SELECT_WORD_RIGHT, TEXT_START, TEXT_UNDO, TEXT_WORD_LEFT, TEXT_WORD_RIGHT,
};
pub use style::TextFieldStyle;

pub(crate) use session::TextState;

use koebi_core::input::CursorShape;
use koebi_core::{Rect, Vec2};
use koebi_design::kind::{self, Kind};

use self::edit::Select;
use self::session::{Away, Session};
use crate::icon::Icon;
use crate::id::WidgetId;
use crate::style::{CornerRadii, Prop};
use crate::text::{Align, FontStyle};
use crate::ui::{Response, Ui};
use crate::widgets::parts::ControlState;

/// What a press on a field that is not editing selects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectOnFocus {
    /// The whole text, ready to be replaced (the default); a drag from the press selects
    /// from there instead.
    #[default]
    All,
    /// Nothing: the caret goes to the end of the text.
    End,
    /// Nothing: the caret goes where the press is.
    Pointer,
}

/// What a press outside a field being edited does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutsidePress {
    /// Ends the edit and keeps the text (the default).
    #[default]
    Confirm,
    /// Ends the edit and restores the text it started with.
    Cancel,
}

/// How an edit ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditEnd {
    /// The text stays: Enter, Tab, a press outside, another field starting to edit, the field
    /// disabled or no longer shown.
    Confirmed,
    /// The text the edit started with is back: Escape, or a press outside with
    /// [`OutsidePress::Cancel`].
    Cancelled,
}

/// What happened to a text field this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextFieldResponse {
    /// The pointer's interaction with the field.
    pub response: Response,
    /// The field still edits when it has been drawn (a field drawn after it in the same frame
    /// may yet take over).
    pub editing: bool,
    /// The edit started this frame.
    pub started: bool,
    /// The text changed this frame: follow this to apply the text as it is typed.
    pub changed: bool,
    /// How the edit ended, when it ended this frame or, away from the field (another field
    /// pressed, the field not shown), in the frame before. Commit the text on
    /// [`EditEnd::Confirmed`] when only final texts matter.
    pub ended: Option<EditEnd>,
}

impl TextFieldResponse {
    /// `true` when the edit ended keeping the text.
    pub fn confirmed(&self) -> bool {
        self.ended == Some(EditEnd::Confirmed)
    }

    /// `true` when the edit ended restoring the text it started with.
    pub fn cancelled(&self) -> bool {
        self.ended == Some(EditEnd::Cancelled)
    }
}

/// The parts whose colors the call can set per state.
type FieldColors = [[Option<Prop<kind::Color>>; 6]; 4];

const BACKGROUND: usize = 0;
const BORDER: usize = 1;
const TEXT_PART: usize = 2;
const ICON: usize = 3;

/// A text field, configured then shown with [`TextField::show`]. Every property comes from
/// the text field tokens (`component.text-field.*`) unless the call gives another token or a
/// value.
#[derive(Debug)]
pub struct TextField<'a> {
    id: WidgetId,
    text: &'a mut String,
    placeholder: Option<&'a str>,
    icon: Option<Icon>,
    enabled: bool,
    align: Align,
    select_on_focus: SelectOnFocus,
    outside_press: OutsidePress,
    filter: Option<fn(char) -> bool>,
    max_chars: Option<usize>,
    font: Option<&'a FontStyle>,
    height: Option<Prop<kind::Dimension>>,
    min_width: Option<Prop<kind::Dimension>>,
    padding: Option<Prop<kind::Dimension>>,
    radii: Option<CornerRadii>,
    border_width: Option<Prop<kind::Dimension>>,
    colors: FieldColors,
    placeholder_color: Option<Prop<kind::Color>>,
    selection_color: Option<Prop<kind::Color>>,
    caret_color: Option<Prop<kind::Color>>,
}

impl<'a> TextField<'a> {
    /// A field identified by `id` showing and editing `text`.
    pub fn new(id: WidgetId, text: &'a mut String) -> Self {
        Self {
            id,
            text,
            placeholder: None,
            icon: None,
            enabled: true,
            align: Align::Start,
            select_on_focus: SelectOnFocus::default(),
            outside_press: OutsidePress::default(),
            filter: None,
            max_chars: None,
            font: None,
            height: None,
            min_width: None,
            padding: None,
            radii: None,
            border_width: None,
            colors: FieldColors::default(),
            placeholder_color: None,
            selection_color: None,
            caret_color: None,
        }
    }

    /// The field's identity.
    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Shows `placeholder` in a quieter color while the text is empty.
    #[must_use]
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = Some(placeholder);
        self
    }

    /// Shows `icon` before the text (a search field's magnifier).
    #[must_use]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Makes the field available or not; a disabled field stops editing and Tab skips it.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Places the text and the placeholder at the start (the default), center or end while
    /// they fit.
    #[must_use]
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// What a press on the field selects when the edit starts.
    #[must_use]
    pub fn select_on_focus(mut self, select: SelectOnFocus) -> Self {
        self.select_on_focus = select;
        self
    }

    /// What a press outside the field does while it edits.
    #[must_use]
    pub fn outside_press(mut self, outside: OutsidePress) -> Self {
        self.outside_press = outside;
        self
    }

    /// Accepts only the characters `accepts` keeps, typed or pasted.
    #[must_use]
    pub fn filter(mut self, accepts: fn(char) -> bool) -> Self {
        self.filter = Some(accepts);
        self
    }

    /// Keeps the text to `count` characters at most.
    #[must_use]
    pub fn max_chars(mut self, count: usize) -> Self {
        self.max_chars = Some(count);
        self
    }

    /// Sets the text in `font` in place of the body font.
    #[must_use]
    pub fn font(mut self, font: &'a FontStyle) -> Self {
        self.font = Some(font);
        self
    }

    /// The height [`TextField::size`] measures.
    #[must_use]
    pub fn height(mut self, height: impl Into<Prop<kind::Dimension>>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// The width [`TextField::size`] measures.
    #[must_use]
    pub fn min_width(mut self, width: impl Into<Prop<kind::Dimension>>) -> Self {
        self.min_width = Some(width.into());
        self
    }

    /// The space between the sides and the text.
    #[must_use]
    pub fn padding(mut self, padding: impl Into<Prop<kind::Dimension>>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    /// The same radius for every corner.
    #[must_use]
    pub fn corner_radius(mut self, radius: impl Into<Prop<kind::Dimension>>) -> Self {
        self.radii = Some(CornerRadii::all(radius));
        self
    }

    /// A radius per corner.
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

    /// The background in `state` (selected: editing).
    #[must_use]
    pub fn background(mut self, state: ControlState, color: impl Into<Prop<kind::Color>>) -> Self {
        self.colors[BACKGROUND][state.index()] = Some(color.into());
        self
    }

    /// The border color in `state` (selected: editing).
    #[must_use]
    pub fn border_color(
        mut self,
        state: ControlState,
        color: impl Into<Prop<kind::Color>>,
    ) -> Self {
        self.colors[BORDER][state.index()] = Some(color.into());
        self
    }

    /// The color of the text in `state` (selected: editing).
    #[must_use]
    pub fn text_color(mut self, state: ControlState, color: impl Into<Prop<kind::Color>>) -> Self {
        self.colors[TEXT_PART][state.index()] = Some(color.into());
        self
    }

    /// The color of the icon in `state` (selected: editing).
    #[must_use]
    pub fn icon_color(mut self, state: ControlState, color: impl Into<Prop<kind::Color>>) -> Self {
        self.colors[ICON][state.index()] = Some(color.into());
        self
    }

    /// The color of the placeholder.
    #[must_use]
    pub fn placeholder_color(mut self, color: impl Into<Prop<kind::Color>>) -> Self {
        self.placeholder_color = Some(color.into());
        self
    }

    /// The background of the selected text.
    #[must_use]
    pub fn selection_color(mut self, color: impl Into<Prop<kind::Color>>) -> Self {
        self.selection_color = Some(color.into());
        self
    }

    /// The color of the caret.
    #[must_use]
    pub fn caret_color(mut self, color: impl Into<Prop<kind::Color>>) -> Self {
        self.caret_color = Some(color.into());
        self
    }

    /// Resolves a property given at the call, or takes the style's value.
    fn prop<K: Kind>(ui: &Ui<'_>, prop: Option<&Prop<K>>, fallback: K::Value) -> K::Value {
        prop.map_or(fallback, |prop| prop.resolve(&ui.styles().tokens))
    }

    /// The size the field asks for: its smallest width (with room for its padding and icon),
    /// one control high or a line of its font if taller. Layouts usually give it more width.
    pub fn size(&self, ui: &mut Ui<'_>) -> Vec2 {
        let style = &ui.styles().text_field;
        let font = self.font.unwrap_or(&ui.styles().text.body);
        let padding = Self::prop(ui, self.padding.as_ref(), style.padding);
        let icon = if self.icon.is_some() {
            style.icon_size + style.gap
        } else {
            0.0
        };
        let width =
            Self::prop(ui, self.min_width.as_ref(), style.min_width).max(2.0 * padding + icon);
        let height = Self::prop(ui, self.height.as_ref(), style.height);
        Vec2::new(width.ceil(), height.max(ui.line_height(font)).ceil())
    }

    /// Shows the field in `rect`, editing its text when it should.
    pub fn show(mut self, ui: &mut Ui<'_>, rect: Rect) -> TextFieldResponse {
        let id = self.id;
        let response = if self.enabled {
            ui.interact_with_cursor(id, rect, CursorShape::Text)
        } else {
            ui.interact(id, rect)
        };
        let mut out = TextFieldResponse {
            response,
            ..TextFieldResponse::default()
        };
        // An edit that ended away from the field: another field was pressed, or this one was
        // not shown.
        if let Some(ended) = ui.state.text.take_ended(id) {
            out.ended = Some(ended.end);
            if let Some(original) = ended.restore
                && *self.text != original
            {
                *self.text = original;
                out.changed = true;
                out.response.changed = true;
            }
        }
        let texts = &mut ui.state.text;
        let focused = texts.is_focused(id);
        let mut session = texts.session.take_if(|session| session.id == id);
        if self.enabled {
            texts.shown.push(id);
        } else {
            texts.clear_focus(id);
            if session.take().is_some() {
                out.ended = Some(EditEnd::Confirmed);
            }
        }
        if session.is_some() {
            // Asked to edit while it does: nothing to start.
            texts.clear_focus(id);
        } else if self.enabled && (response.pressed || focused) {
            session = Some(self.start(ui, rect, response.pressed));
            out.started = true;
        }
        match session {
            Some(mut session) => {
                session.seen = true;
                if self.edit(ui, rect, &mut session, &mut out) {
                    self.draw(ui, rect, Some(&mut session), &out);
                    ui.state.text.session = Some(session);
                    out.editing = true;
                } else {
                    self.draw(ui, rect, None, &out);
                }
            }
            None => self.draw(ui, rect, None, &out),
        }
        out
    }

    /// Starts editing: another field's edit ends, and the press (if one started it) selects.
    fn start(&self, ui: &mut Ui<'_>, rect: Rect, by_press: bool) -> Session {
        let texts = &mut ui.state.text;
        texts.end_away(if by_press {
            Away::Pressed
        } else {
            Away::Replaced
        });
        texts.clear_focus(self.id);
        let mut session = Session::new(self.id, self.text, ui.input().time(), self.outside_press);
        // Where the press happened, not where the pointer went since.
        let press = ui.input().pressed_at().filter(|_| by_press);
        match (self.select_on_focus, press) {
            (SelectOnFocus::Pointer, Some(press)) => {
                let at = self.offset_at(ui, rect, &session, press.x);
                session.edit.place(at, Select::Collapse);
                session.dragging = true;
            }
            (SelectOnFocus::End, _) => {}
            (_, press) => {
                session.edit.select_all();
                session.dragging = press.is_some();
                session.select_after_threshold = press.is_some();
            }
        }
        session
    }
}

impl Ui<'_> {
    /// Edits the text field `id` when it is next shown, in this frame or the next one: a name
    /// to type at once (a new item, a rename shortcut). The field selects as its
    /// [`SelectOnFocus`] says when no press places the caret: everything, or the end.
    pub fn edit_text_field(&mut self, id: WidgetId) {
        self.state.text.set_focus(id);
        let now = self.input().time();
        self.request_redraw_at(now);
    }

    /// Leaves `text` for the application to put on the clipboard after the frame
    /// ([`UiState::copied_text`](crate::UiState::copied_text)).
    pub fn copy_text(&mut self, text: &str) {
        self.state.text.copied = Some(text.to_owned());
    }

    /// Asks the application for the clipboard's text, which arrives in the next frame
    /// ([`UiState::set_pasted_text`](crate::UiState::set_pasted_text)).
    pub fn request_paste(&mut self) {
        self.state.text.paste_requested = true;
    }
}
