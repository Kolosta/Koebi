//! The fields of the gallery: text fields with text, with a placeholder and an icon, centered,
//! accepting digits only, too narrow for their text, and disabled.

use koebi_core::Rect;
use koebi_ui::{Align, EditEnd, Icon, PanelLevel, SelectOnFocus, TextField, Ui, WidgetId};

use super::{Metrics, cut_left, cut_top};

/// Rows of the panel: the fields, then what the last edit did.
pub(super) const ROWS: u16 = 2;

/// The texts the panel edits.
#[derive(Debug)]
pub(super) struct Fields {
    name: String,
    search: String,
    centered: String,
    digits: String,
    long: String,
    disabled: String,
    /// What the last edit did.
    last: &'static str,
}

impl Fields {
    pub(super) fn new() -> Self {
        Self {
            name: "Cube.001".to_owned(),
            search: String::new(),
            centered: "Centered".to_owned(),
            digits: "2026".to_owned(),
            long: "A name far too long for its field, which scrolls while editing".to_owned(),
            disabled: "Disabled".to_owned(),
            last: "Click a field to edit it; Tab moves to the next one",
        }
    }

    /// Draws the panel in `rect`.
    pub(super) fn show(&mut self, ui: &mut Ui<'_>, rect: Rect, metrics: Metrics) {
        const ID: WidgetId = WidgetId::new("fields");
        let mut content = ui.panel(rect, PanelLevel::First);
        let (row, gap) = (metrics.row, metrics.row_gap());

        let mut line = cut_top(&mut content, row, gap);
        let sixth = ((line.width() - 5.0 * gap) / 6.0).floor();
        let mut ended = [None; 6];
        ended[0] = TextField::new(ID.with(0), &mut self.name)
            .show(ui, cut_left(&mut line, sixth, gap))
            .ended;
        ended[1] = TextField::new(ID.with(1), &mut self.search)
            .placeholder("Search")
            .icon(Icon::Search)
            .select_on_focus(SelectOnFocus::End)
            .show(ui, cut_left(&mut line, sixth, gap))
            .ended;
        ended[2] = TextField::new(ID.with(2), &mut self.centered)
            .align(Align::Center)
            .show(ui, cut_left(&mut line, sixth, gap))
            .ended;
        ended[3] = TextField::new(ID.with(3), &mut self.digits)
            .filter(|c| c.is_ascii_digit())
            .max_chars(4)
            .align(Align::End)
            .show(ui, cut_left(&mut line, sixth, gap))
            .ended;
        ended[4] = TextField::new(ID.with(4), &mut self.long)
            .select_on_focus(SelectOnFocus::Pointer)
            .show(ui, cut_left(&mut line, sixth, gap))
            .ended;
        ended[5] = TextField::new(ID.with(5), &mut self.disabled)
            .enabled(false)
            .show(ui, line)
            .ended;
        match ended.into_iter().flatten().next() {
            Some(EditEnd::Confirmed) => self.last = "Kept the text",
            Some(EditEnd::Cancelled) => self.last = "Restored the text",
            None => {}
        }

        let style = &ui.styles().text;
        ui.label(
            self.last,
            cut_top(&mut content, row, gap),
            &style.body,
            style.secondary,
            Align::Start,
        );
    }
}
