//! Where a field's text lies: the area inside the padding and after the icon, where a line
//! starts (aligned while it fits, scrolled when it does not), where the caret goes before a
//! character, which character boundary is nearest to a point, and how far to scroll to keep
//! the caret in view.

use koebi_core::{Rect, Vec2};
use koebi_text::TextLine;

use super::TextField;
use super::session::Session;
use crate::text::Align;
use crate::ui::Ui;

impl TextField<'_> {
    /// The rectangle the text lies in: inside the padding, after the icon.
    pub(super) fn text_area(&self, ui: &Ui<'_>, rect: Rect) -> Rect {
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
    pub(super) fn origin(&self, area: Rect, width: f32, scroll: f32) -> f32 {
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
    pub(super) fn offset_at(
        &self,
        ui: &mut Ui<'_>,
        rect: Rect,
        session: &Session,
        x: f32,
    ) -> usize {
        let area = self.text_area(ui, rect);
        let font = self.font.unwrap_or(&ui.styles().text.body);
        ui.lay_out(session.edit.text(), font);
        let line = &ui.state.line;
        let origin = self.origin(area, line.width(), session.scroll);
        offset_at(line, x - origin, session.edit.text().len())
    }
}

/// Where the caret before the character at byte `offset` lies on `line`, from its start.
pub(super) fn caret_x(line: &TextLine, offset: usize) -> f32 {
    line.glyphs()
        .iter()
        .find(|glyph| glyph.cluster as usize >= offset)
        .map_or(line.width(), |glyph| glyph.x)
}

/// The byte offset of the character boundary nearest to `x` (from the line's start) on
/// `line`, laid out from a text of `len` bytes.
pub(super) fn offset_at(line: &TextLine, x: f32, len: usize) -> usize {
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
pub(super) fn scroll_to(scroll: f32, caret: f32, width: f32, room: f32) -> f32 {
    // Fits, or a size that is not a number.
    if width.partial_cmp(&room) != Some(core::cmp::Ordering::Greater) {
        return 0.0;
    }
    // Not `clamp`: it panics on a NaN bound.
    let scroll = scroll.min(width - room).max(0.0);
    if caret - scroll > room {
        caret - room
    } else if caret < scroll {
        caret
    } else {
        scroll
    }
}
