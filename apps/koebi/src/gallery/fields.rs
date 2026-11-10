//! The fields of the gallery: text fields with text, with a placeholder and an icon, centered,
//! accepting digits only, too narrow for their text, and disabled; number fields in a unit,
//! with linear and logarithmic bars, of integers, and disabled.

use koebi_core::Rect;
use koebi_core::expr::MILLIMETER;
use koebi_i18n::tr;
use koebi_ui::{
    Align, BarMapping, EditEnd, NumberField, PanelLevel, SelectOnFocus, TextField, Ui, WidgetId,
};

use super::{Metrics, cut_left, cut_top};

/// Rows of the panel: the text fields, the number fields, their step buttons on and off, then
/// what the last edit did.
pub(super) const ROWS: u16 = 4;

/// The texts the panel edits.
#[derive(Debug)]
pub(super) struct Fields {
    name: String,
    search: String,
    centered: String,
    digits: String,
    long: String,
    disabled: String,
    width: f64,
    opacity: f64,
    frequency: f64,
    samples: i32,
    locked: f64,
    bar_with_buttons: f64,
    plain_without_buttons: f64,
    /// What the last edit did.
    last: &'static str,
}

impl Fields {
    pub(super) fn new() -> Self {
        Self {
            name: "Cube.001".to_owned(),
            search: String::new(),
            centered: tr("Centered").to_owned(),
            digits: "2026".to_owned(),
            long: tr("A name far too long for its field, which scrolls while editing").to_owned(),
            disabled: tr("Disabled").to_owned(),
            width: 12.5,
            opacity: 0.8,
            frequency: 440.0,
            samples: 64,
            locked: 1.0,
            bar_with_buttons: 0.5,
            plain_without_buttons: 3.5,
            last: tr("Click a field to edit it; Tab moves to the next one"),
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
            .placeholder(tr("Search"))
            .icon(koebi_icons::interface::SEARCH)
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
        let mut line = cut_top(&mut content, row, gap);
        let fifth = ((line.width() - 4.0 * gap) / 5.0).floor();
        let numbers = [
            NumberField::new(ID.with(10), &mut self.width)
                .label(tr("Width"))
                .unit(&MILLIMETER)
                .show(ui, cut_left(&mut line, fifth, gap))
                .ended,
            NumberField::new(ID.with(11), &mut self.opacity)
                .label(tr("Opacity"))
                .range(0.0, 1.0)
                .bar(BarMapping::Linear)
                .show(ui, cut_left(&mut line, fifth, gap))
                .ended,
            NumberField::new(ID.with(12), &mut self.frequency)
                .label(tr("Frequency"))
                .range(20.0, 20_000.0)
                .decimals(0)
                .step(10.0)
                .bar(BarMapping::Logarithmic)
                .show(ui, cut_left(&mut line, fifth, gap))
                .ended,
            NumberField::new(ID.with(13), &mut self.samples)
                .label(tr("Samples"))
                .range(1, 256)
                .show(ui, cut_left(&mut line, fifth, gap))
                .ended,
            NumberField::new(ID.with(14), &mut self.locked)
                .label(tr("Locked"))
                .enabled(false)
                .show(ui, line)
                .ended,
        ];
        // Step buttons: on a field that has none by default, and off one that has them.
        let mut line = cut_top(&mut content, row, gap);
        let half = ((line.width() - gap) * 0.5).floor();
        NumberField::new(ID.with(15), &mut self.bar_with_buttons)
            .label(tr("Bar with step buttons"))
            .range(0.0, 1.0)
            .bar(BarMapping::Linear)
            .step_buttons(true)
            .show(ui, cut_left(&mut line, half, gap));
        NumberField::new(ID.with(16), &mut self.plain_without_buttons)
            .label(tr("Without step buttons"))
            .step_buttons(false)
            .show(ui, line);
        match ended.into_iter().chain(numbers).flatten().next() {
            Some(EditEnd::Confirmed) => self.last = tr("Kept the text"),
            Some(EditEnd::Cancelled) => self.last = tr("Restored the text"),
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
