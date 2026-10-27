//! Text fields: one line of text edited in place.
//!
//! - **Editing** starts with a press on the field (selecting the whole text by default, see
//!   [`SelectOnFocus`]), with Tab from the previous field, or when the application asks
//!   ([`Ui::edit_text_field`]). One field edits at a time; while it does, it takes the
//!   keyboard ([`TEXT`]) and asks the platform for text input in its rectangle (on-screen
//!   keyboards, input methods).
//! - **Pointer**: a press places the caret (Shift extends the selection), a drag selects, a
//!   double press selects a word. The text scrolls to keep the caret in view.
//! - **Keys** ([`TEXT_ACTIONS`]): moves by character, word or to the ends, with Shift to
//!   select; deletion by character or word; select all; undo and redo; clipboard.
//! - **Ending**: Enter keeps the text, Escape restores the text the edit started with, Tab and
//!   Shift+Tab keep it and edit the next or previous field, and a press outside keeps it (or
//!   restores it, see [`OutsidePress`]). The text changes as the user types:
//!   [`TextFieldResponse`] says when it changed and how the edit ended.
//! - **Clipboard**: the interface does not reach the platform; it asks the application, which
//!   copies [`UiState::copied`](crate::UiState::copied) to the clipboard and answers
//!   [`UiState::paste_requested`](crate::UiState::paste_requested) with
//!   [`UiState::paste`](crate::UiState::paste).
//! - **Constraints**: a character filter (digits only...) and a length limit.

mod actions;
mod edit;
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
use koebi_core::{Color, Rect, Vec2};
use koebi_design::kind::{self, Kind};
use koebi_input::ActionDef;
use koebi_render::{Corners, RectPrimitive};
use koebi_text::TextLine;

use self::edit::{Select, Step, TextEdit};
use self::session::Session;
use crate::icon::Icon;
use crate::id::WidgetId;
use crate::input::{MAX_ACTIONS, PointerButton};
use crate::style::{CornerRadii, Prop};
use crate::text::{Align, FontStyle};
use crate::ui::{Response, Ui};
use crate::widgets::parts::ControlState;
use crate::widgets::snap;

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
    /// The text stays: Enter, Tab, a press outside, another field starting to edit, or the
    /// field no longer shown.
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
    /// The field edits text (after this frame).
    pub editing: bool,
    /// The edit started this frame.
    pub started: bool,
    /// The text changed this frame.
    pub changed: bool,
    /// How the edit ended, when it ended this frame.
    pub ended: Option<EditEnd>,
}

impl TextFieldResponse {
    /// `true` when the edit ended this frame keeping the text.
    pub fn confirmed(&self) -> bool {
        self.ended == Some(EditEnd::Confirmed)
    }

    /// `true` when the edit ended this frame restoring the text it started with.
    pub fn cancelled(&self) -> bool {
        self.ended == Some(EditEnd::Cancelled)
    }
}

/// The parts whose colors change per state.
type FieldColors = [[Option<Prop<kind::Color>>; 6]; 2];

const BACKGROUND: usize = 0;
const BORDER: usize = 1;

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
    text_color: Option<Prop<kind::Color>>,
    placeholder_color: Option<Prop<kind::Color>>,
    icon_color: Option<Prop<kind::Color>>,
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
            text_color: None,
            placeholder_color: None,
            icon_color: None,
            selection_color: None,
            caret_color: None,
        }
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

    /// Makes the field available or not; a disabled field stops editing.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Places the text at the start (the default), center or end while it fits.
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

    /// The color of the text of an enabled field.
    #[must_use]
    pub fn text_color(mut self, color: impl Into<Prop<kind::Color>>) -> Self {
        self.text_color = Some(color.into());
        self
    }

    /// The color of the placeholder.
    #[must_use]
    pub fn placeholder_color(mut self, color: impl Into<Prop<kind::Color>>) -> Self {
        self.placeholder_color = Some(color.into());
        self
    }

    /// The color of the icon of an enabled field.
    #[must_use]
    pub fn icon_color(mut self, color: impl Into<Prop<kind::Color>>) -> Self {
        self.icon_color = Some(color.into());
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
        let texts = &mut ui.state.text;
        texts.shown.push(id);
        if texts.ended.is_some_and(|ended| ended.id == id) {
            texts.ended = None;
            out.ended = Some(EditEnd::Confirmed);
        }
        let mut session = texts.session.take_if(|session| session.id == id);
        if !self.enabled {
            if session.take().is_some() {
                out.ended = Some(EditEnd::Confirmed);
            }
        } else if session.is_none() && (response.pressed || texts.focus == Some(id)) {
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
        texts.end_away();
        if texts.focus == Some(self.id) {
            texts.focus = None;
        }
        let mut session = Session {
            id: self.id,
            edit: TextEdit::new(self.text),
            original: self.text.clone(),
            scroll: 0.0,
            dragging: false,
            select_after_threshold: false,
            since: ui.input().time(),
            seen: true,
        };
        let pointer = ui.input().pointer().filter(|_| by_press);
        match (self.select_on_focus, pointer) {
            (SelectOnFocus::Pointer, Some(pointer)) => {
                let at = self.offset_at(ui, rect, &session, pointer.x);
                session.edit.place(at, Select::Collapse);
                session.dragging = true;
            }
            (SelectOnFocus::End, _) => {}
            (_, pointer) => {
                session.edit.select_all();
                session.dragging = pointer.is_some();
                session.select_after_threshold = pointer.is_some();
            }
        }
        session
    }

    /// Applies this frame's pointer, keys and typed text to the edit, and writes the text
    /// back when it changed. Returns `false` when the edit ended.
    fn edit(
        &mut self,
        ui: &mut Ui<'_>,
        rect: Rect,
        session: &mut Session,
        out: &mut TextFieldResponse,
    ) -> bool {
        ui.grab(TEXT);
        ui.text_input(rect);
        // The application changed the text during the edit (an undo of the document): the
        // edit continues from it, and can undo the change.
        if session.edit.text() != self.text.as_str() {
            session.edit.set_text(self.text);
        }
        let before = (session.edit.caret(), session.edit.selection());
        let outside = [
            PointerButton::Primary,
            PointerButton::Secondary,
            PointerButton::Middle,
        ]
        .into_iter()
        .filter_map(|button| ui.input().button_pressed_at(button))
        .any(|at| !rect.contains(at));
        if outside {
            return match self.outside_press {
                OutsidePress::Confirm => {
                    out.ended = Some(EditEnd::Confirmed);
                    false
                }
                OutsidePress::Cancel => {
                    self.cancel(session, out);
                    false
                }
            };
        }
        self.point(ui, rect, session, out);
        let mut changed = Self::keys(ui, session);
        changed |= self.type_text(ui, session);
        if changed {
            self.text.clear();
            self.text.push_str(session.edit.text());
            out.changed = true;
            out.response.changed = true;
        }
        if changed || before != (session.edit.caret(), session.edit.selection()) {
            session.since = ui.input().time();
        }
        // The keys that end the edit come last: what was typed before them stays.
        // Each is used here: the field that Tab starts in this frame does not see it.
        if ui.triggered(&TEXT_CANCEL) {
            ui.consume(&TEXT_CANCEL);
            self.cancel(session, out);
            return false;
        }
        let next = [(&TEXT_NEXT_FIELD, true), (&TEXT_PREVIOUS_FIELD, false)]
            .into_iter()
            .find(|(action, _)| ui.triggered(action));
        if let Some((action, forward)) = next {
            ui.consume(action);
            let texts = &mut ui.state.text;
            texts.focus = texts.neighbour(self.id, forward);
            out.ended = Some(EditEnd::Confirmed);
            return false;
        }
        if ui.triggered(&TEXT_CONFIRM) {
            ui.consume(&TEXT_CONFIRM);
            out.ended = Some(EditEnd::Confirmed);
            return false;
        }
        true
    }

    /// The pointer: a press places the caret or selects a word, a drag selects.
    fn point(&self, ui: &mut Ui<'_>, rect: Rect, session: &mut Session, out: &TextFieldResponse) {
        let response = &out.response;
        let pointer = ui.input().pointer();
        if response.pressed
            && !out.started
            && let Some(pointer) = pointer
        {
            let at = self.offset_at(ui, rect, session, pointer.x);
            if response.double_clicked {
                session.edit.select_word(at);
                session.dragging = false;
            } else {
                let select = if ui.input().modifiers().shift() {
                    Select::Extend
                } else {
                    Select::Collapse
                };
                session.edit.place(at, select);
                session.dragging = true;
            }
            session.select_after_threshold = false;
        }
        if !response.held {
            session.dragging = false;
            return;
        }
        let (true, Some(pointer)) = (session.dragging, pointer) else {
            return;
        };
        if session.select_after_threshold {
            if response.drag_delta.length() < ui.styles().input.drag_threshold {
                return;
            }
            // A drag from the press that selected everything selects from the press.
            session.select_after_threshold = false;
            let press = pointer - response.drag_delta;
            let start = self.offset_at(ui, rect, session, press.x);
            session.edit.place(start, Select::Collapse);
        }
        let at = self.offset_at(ui, rect, session, pointer.x);
        session.edit.place(at, Select::Extend);
    }

    /// The keys that move the caret and change the text. Returns `true` when the text
    /// changed.
    fn keys(ui: &mut Ui<'_>, session: &mut Session) -> bool {
        const MOVES: [(&ActionDef, bool, Step, Select); 12] = [
            (&TEXT_LEFT, false, Step::Character, Select::Collapse),
            (&TEXT_RIGHT, true, Step::Character, Select::Collapse),
            (&TEXT_WORD_LEFT, false, Step::Word, Select::Collapse),
            (&TEXT_WORD_RIGHT, true, Step::Word, Select::Collapse),
            (&TEXT_START, false, Step::Line, Select::Collapse),
            (&TEXT_END, true, Step::Line, Select::Collapse),
            (&TEXT_SELECT_LEFT, false, Step::Character, Select::Extend),
            (&TEXT_SELECT_RIGHT, true, Step::Character, Select::Extend),
            (&TEXT_SELECT_WORD_LEFT, false, Step::Word, Select::Extend),
            (&TEXT_SELECT_WORD_RIGHT, true, Step::Word, Select::Extend),
            (&TEXT_SELECT_START, false, Step::Line, Select::Extend),
            (&TEXT_SELECT_END, true, Step::Line, Select::Extend),
        ];
        const DELETES: [(&ActionDef, bool, Step); 4] = [
            (&TEXT_DELETE_BACKWARD, false, Step::Character),
            (&TEXT_DELETE_FORWARD, true, Step::Character),
            (&TEXT_DELETE_WORD_BACKWARD, false, Step::Word),
            (&TEXT_DELETE_WORD_FORWARD, true, Step::Word),
        ];
        let edit = &mut session.edit;
        let mut changed = false;
        // In the order they came: several keys between two frames apply one after the other,
        // repeats included. What the field uses is consumed once it has gone through them all.
        let mut used = [None; MAX_ACTIONS];
        for (slot, action) in used.iter_mut().zip(ui.input().actions()) {
            if !ui.triggered(action) {
                continue;
            }
            let id = action.id;
            if let Some(&(_, forward, step, select)) = MOVES.iter().find(|(a, ..)| a.id == id) {
                edit.step(forward, step, select);
            } else if let Some(&(_, forward, step)) = DELETES.iter().find(|(a, ..)| a.id == id) {
                changed |= edit.delete(forward, step);
            } else if id == TEXT_SELECT_ALL.id {
                edit.select_all();
            } else if id == TEXT_COPY.id || id == TEXT_CUT.id {
                if !edit.selected().is_empty() {
                    ui.copy(edit.selected());
                    if id == TEXT_CUT.id {
                        changed |= edit.delete(false, Step::Character);
                    }
                }
            } else if id == TEXT_PASTE.id {
                ui.request_paste();
            } else if id == TEXT_UNDO.id {
                changed |= edit.undo();
            } else if id == TEXT_REDO.id {
                changed |= edit.redo();
            } else {
                continue;
            }
            *slot = Some(action);
        }
        // Used here: the components after the field do not see them.
        for action in used.into_iter().flatten() {
            ui.consume(action);
        }
        changed
    }

    /// The text typed this frame, and the clipboard's text when it arrived, within the filter
    /// and the length limit. Returns `true` when the text changed.
    fn type_text(&self, ui: &mut Ui<'_>, session: &mut Session) -> bool {
        let mut changed = false;
        for c in ui.input().text().chars() {
            // Tab and line breaks come from keys the field handles as actions.
            if matches!(c, '\t' | '\n' | '\r') || !self.accepts(c) || self.room(session) == 0 {
                continue;
            }
            let mut bytes = [0; 4];
            changed |= session.edit.insert(c.encode_utf8(&mut bytes));
        }
        if let Some(pasted) = ui.state.text.pasted.take() {
            let room = self.room(session);
            let kept: String = pasted
                .chars()
                .map(|c| {
                    if matches!(c, '\n' | '\r' | '\t') {
                        ' '
                    } else {
                        c
                    }
                })
                .filter(|c| self.accepts(*c))
                .take(room)
                .collect();
            changed |= session.edit.paste(&kept);
        }
        changed
    }

    /// `true` when the filter keeps `c`.
    fn accepts(&self, c: char) -> bool {
        !c.is_control() && self.filter.is_none_or(|accepts| accepts(c))
    }

    /// How many characters can still be inserted, replacing the selection.
    fn room(&self, session: &Session) -> usize {
        let Some(limit) = self.max_chars else {
            return usize::MAX;
        };
        let kept = session.edit.text().chars().count() - session.edit.selected().chars().count();
        limit.saturating_sub(kept)
    }

    /// Ends the edit restoring the text it started with.
    fn cancel(&mut self, session: &Session, out: &mut TextFieldResponse) {
        if *self.text != session.original {
            self.text.clone_from(&session.original);
            out.changed = true;
            out.response.changed = true;
        }
        out.ended = Some(EditEnd::Cancelled);
    }

    /// The rectangle the text lies in: inside the padding, after the icon.
    fn text_area(&self, ui: &Ui<'_>, rect: Rect) -> Rect {
        let style = &ui.styles().text_field;
        let padding = Self::prop(ui, self.padding.as_ref(), style.padding);
        let icon = if self.icon.is_some() {
            style.icon_size + style.gap
        } else {
            0.0
        };
        let min_x = (rect.min.x + padding + icon).min(rect.max.x);
        Rect::from_min_max(
            Vec2::new(min_x, rect.min.y),
            Vec2::new((rect.max.x - padding).max(min_x), rect.max.y),
        )
    }

    /// Where a line of `width` starts in `area`: placed by the alignment while it fits,
    /// scrolled by `scroll` when it does not.
    fn origin(&self, area: Rect, width: f32, scroll: f32) -> f32 {
        let room = area.width() - width;
        if room < 0.0 {
            return area.min.x - scroll;
        }
        match self.align {
            Align::Start => area.min.x,
            Align::Center => area.min.x + room * 0.5,
            Align::End => area.max.x - width,
        }
    }

    /// The text position nearest to `x`, against the edit's text as the last frame showed it.
    fn offset_at(&self, ui: &mut Ui<'_>, rect: Rect, session: &Session, x: f32) -> usize {
        let area = self.text_area(ui, rect);
        let font = self.font.unwrap_or(&ui.styles().text.body);
        ui.lay_out(session.edit.text(), font);
        let line = &ui.state.line;
        let origin = self.origin(area, line.width(), session.scroll);
        offset_at(line, x - origin, session.edit.text().len())
    }

    /// The color of `part` in `state`: the call's, or the style's.
    fn color(&self, ui: &Ui<'_>, part: usize, state: ControlState) -> Color {
        let style = &ui.styles().text_field;
        let fallback = if part == BACKGROUND {
            style.background
        } else {
            style.border
        }
        .get(state);
        Self::prop(ui, self.colors[part][state.index()].as_ref(), fallback)
    }

    /// Draws the field: its box, icon and text, and while it edits, the selection and caret.
    fn draw(
        &self,
        ui: &mut Ui<'_>,
        rect: Rect,
        session: Option<&mut Session>,
        out: &TextFieldResponse,
    ) {
        let style = &ui.styles().text_field;
        let state = if !self.enabled {
            ControlState::Disabled
        } else if session.is_some() {
            ControlState::Selected
        } else if out.response.hovered || out.response.held {
            ControlState::Hovered
        } else {
            ControlState::Normal
        };
        let radii = self
            .radii
            .as_ref()
            .map_or(Corners::all(style.corner_radius), |radii| {
                radii.resolve(&ui.styles().tokens)
            });
        let border_width = Self::prop(ui, self.border_width.as_ref(), style.border_width);
        let (background, border) = (
            self.color(ui, BACKGROUND, state),
            self.color(ui, BORDER, state),
        );
        ui.scene().push_rect(
            &RectPrimitive::new(snap(rect), background)
                .with_corner_radii(radii)
                .with_border(border_width, border),
        );
        // What lies inside the box stays inside it, even in a rectangle smaller than asked.
        ui.clipped(rect, |ui| self.draw_content(ui, rect, session));
    }

    /// Draws the icon and the text, and while the field edits, the selection and caret.
    fn draw_content(&self, ui: &mut Ui<'_>, rect: Rect, session: Option<&mut Session>) {
        let style = &ui.styles().text_field;
        let area = self.text_area(ui, rect);
        if let Some(icon) = self.icon {
            let color = if self.enabled {
                Self::prop(ui, self.icon_color.as_ref(), style.icon)
            } else {
                style.icon_disabled
            };
            let slot = Rect::from_origin_size(
                Vec2::new(area.min.x - style.gap - style.icon_size, rect.min.y),
                Vec2::new(style.icon_size, rect.height()),
            );
            let stroke = ui.styles().icon.stroke_width;
            icon.draw(
                ui.scene(),
                Icon::centered(slot, style.icon_size),
                color,
                stroke,
            );
        }
        let font = self.font.unwrap_or(&ui.styles().text.body);
        let text_color = if self.enabled {
            Self::prop(ui, self.text_color.as_ref(), style.text)
        } else {
            style.text_disabled
        };
        let placeholder = Self::prop(ui, self.placeholder_color.as_ref(), style.placeholder);
        match session {
            Some(session) => self.draw_editing(ui, area, session, text_color, placeholder),
            None if self.text.is_empty() => {
                if let Some(text) = self.placeholder {
                    ui.label(text, area, font, placeholder, self.align);
                }
            }
            None => {
                ui.label(self.text, area, font, text_color, self.align);
            }
        }
    }

    /// Draws the text being edited, scrolled to keep the caret in view, with its selection
    /// and caret.
    fn draw_editing(
        &self,
        ui: &mut Ui<'_>,
        area: Rect,
        session: &mut Session,
        text_color: Color,
        placeholder: Color,
    ) {
        let style = &ui.styles().text_field;
        let font = self.font.unwrap_or(&ui.styles().text.body);
        let selection_color = Self::prop(ui, self.selection_color.as_ref(), style.selection);
        let caret_color = Self::prop(ui, self.caret_color.as_ref(), style.caret);
        let caret_width = style.caret_width;
        let has_fonts = ui.lay_out(session.edit.text(), font);
        let line = &ui.state.line;
        let width = line.width();
        let caret = caret_x(line, session.edit.caret());
        session.scroll = scroll_to(session.scroll, caret, width, area.width());
        let origin = self.origin(area, width, session.scroll);
        let (baseline, top, bottom) = if has_fonts {
            let baseline = line.centered_baseline(area.min.y, area.height());
            (
                baseline,
                baseline - line.ascent(),
                baseline + line.descent(),
            )
        } else {
            (area.center().y, area.min.y, area.max.y)
        };
        let selection = session.edit.selection();
        let selected = (!selection.is_empty())
            .then(|| (caret_x(line, selection.start), caret_x(line, selection.end)));
        // The caret shows at the edges of the area.
        let clip = Rect::from_min_max(
            Vec2::new(area.min.x - caret_width, area.min.y),
            Vec2::new(area.max.x + caret_width, area.max.y),
        );
        let visible = Self::caret_visible(ui, session);
        ui.clipped(clip, |ui| {
            if let Some((start, end)) = selected {
                ui.scene().push_rect(&RectPrimitive::new(
                    Rect::from_min_max(
                        Vec2::new(origin + start, top),
                        Vec2::new(origin + end, bottom),
                    ),
                    selection_color,
                ));
            }
            if session.edit.text().is_empty() {
                if let Some(text) = self.placeholder {
                    ui.label(text, area, font, placeholder, Align::Start);
                }
            } else {
                ui.scene
                    .push_text(&ui.state.line, Vec2::new(origin, baseline), text_color);
            }
            if visible {
                let x = (origin + caret - caret_width * 0.5).round();
                ui.scene().push_rect(&RectPrimitive::new(
                    Rect::from_min_max(Vec2::new(x, top), Vec2::new(x + caret_width, bottom)),
                    caret_color,
                ));
            }
        });
    }

    /// `true` when the caret shows this frame: always, or in the shown half of its blink,
    /// asking for the frame that turns it.
    fn caret_visible(ui: &mut Ui<'_>, session: &Session) -> bool {
        let blink = ui.styles().text_field.caret_blink;
        if blink.is_zero() {
            return true;
        }
        let elapsed = ui.input().time().saturating_sub(session.since);
        let halves = elapsed.as_nanos() / blink.as_nanos();
        let next = session
            .since
            .saturating_add(blink.saturating_mul(u32::try_from(halves + 1).unwrap_or(u32::MAX)));
        ui.request_redraw_at(next);
        halves.is_multiple_of(2)
    }
}

/// Where the caret before the character at byte `offset` lies on `line`, from its start.
fn caret_x(line: &TextLine, offset: usize) -> f32 {
    line.glyphs()
        .iter()
        .find(|glyph| glyph.cluster as usize >= offset)
        .map_or(line.width(), |glyph| glyph.x)
}

/// The byte offset of the character boundary nearest to `x` (from the line's start) on
/// `line`, laid out from a text of `len` bytes.
fn offset_at(line: &TextLine, x: f32, len: usize) -> usize {
    let glyphs = line.glyphs();
    for (index, glyph) in glyphs.iter().enumerate() {
        let end = glyphs.get(index + 1).map_or(line.width(), |next| next.x);
        if x < f32::midpoint(glyph.x, end) {
            return glyph.cluster as usize;
        }
    }
    len
}

/// The scroll that keeps the caret at `caret` in view in an area `room` wide, moving the last
/// `scroll` as little as possible; none while the text (`width`) fits.
fn scroll_to(scroll: f32, caret: f32, width: f32, room: f32) -> f32 {
    if width <= room {
        return 0.0;
    }
    let scroll = scroll.clamp(0.0, width - room);
    if caret - scroll > room {
        caret - room
    } else if caret < scroll {
        caret
    } else {
        scroll
    }
}

impl Ui<'_> {
    /// Edits the text field `id` when it is next shown, its text selected: a name to type at
    /// once (a new item, a rename shortcut).
    pub fn edit_text_field(&mut self, id: WidgetId) {
        self.state.text.focus = Some(id);
        let now = self.input().time();
        self.request_redraw_at(now);
    }

    /// Puts `text` on the clipboard: the application copies
    /// [`UiState::copied`](crate::UiState::copied) after the frame.
    pub fn copy(&mut self, text: &str) {
        self.state.text.copied = Some(text.to_owned());
    }

    /// Asks the application for the clipboard's text, which arrives in the next frame
    /// ([`UiState::paste`](crate::UiState::paste)).
    pub fn request_paste(&mut self) {
        self.state.text.paste_requested = true;
    }
}
