//! The buttons of the gallery: every variant and color family, groups on a grid, content and
//! behaviours (press, double press, repeat), and checkboxes with descriptions.

use core::fmt::Write as _;

use koebi_core::{Color, Rect};
use koebi_ui::{
    Activation, Align, Button, ButtonGroup, Checkbox, GroupCell, Hue, Icon, PanelLevel, Response,
    Tone, Ui, Variant, WidgetId,
};

use super::{Metrics, cut_left, cut_top};

/// Rows of the panel: variants, two rows of color families, groups and content, two rows of
/// behaviours, two rows of checkboxes.
pub(super) const ROWS: u16 = 8;

/// The variants and their labels.
const VARIANTS: [(Variant, &str); 7] = [
    (Variant::Default, "Default"),
    (Variant::Accent, "Accent"),
    (Variant::Positive, "Positive"),
    (Variant::Negative, "Negative"),
    (Variant::Info, "Info"),
    (Variant::Neutral, "Neutral"),
    (Variant::Notice, "Notice"),
];

/// The color families and their labels.
const HUES: [(Hue, &str); 8] = [
    (Hue::Blue, "Blue"),
    (Hue::Cyan, "Cyan"),
    (Hue::Green, "Green"),
    (Hue::Yellow, "Yellow"),
    (Hue::Orange, "Orange"),
    (Hue::Red, "Red"),
    (Hue::Magenta, "Magenta"),
    (Hue::Purple, "Purple"),
];

/// The choices of the segmented group.
const MODES: [&str; 3] = ["Object", "Edit", "Sculpt"];

/// The values the panel edits.
#[derive(Debug)]
pub(super) struct Buttons {
    /// The family toggles.
    families: [bool; 8],
    /// The choice of the segmented group.
    mode: usize,
    /// The L and R modifier toggles.
    sides: [bool; 2],
    /// The counter the step buttons change.
    count: i32,
    /// The counter as text, rewritten in place when it changes.
    count_text: String,
    /// The checkboxes.
    snap: bool,
    overlays: bool,
}

impl Buttons {
    pub(super) fn new() -> Self {
        let mut buttons = Self {
            families: [false, true, false, true, false, true, false, true],
            mode: 0,
            sides: [true, false],
            count: 0,
            count_text: String::with_capacity(12),
            snap: true,
            overlays: false,
        };
        buttons.count_changed();
        buttons
    }

    fn count_changed(&mut self) {
        self.count_text.clear();
        // Writing to a String cannot fail.
        let _ = write!(self.count_text, "{}", self.count);
    }

    /// Draws the panel in `rect`.
    pub(super) fn show(&mut self, ui: &mut Ui<'_>, rect: Rect, metrics: Metrics) {
        const ID: WidgetId = WidgetId::new("buttons");
        let mut content = ui.panel(rect, PanelLevel::First);
        let (row, gap) = (metrics.row, metrics.row_gap());

        let mut line = cut_top(&mut content, row, gap);
        for (index, (variant, label)) in VARIANTS.into_iter().enumerate() {
            let button = Button::new(ID.with(index as u64))
                .label(label)
                .tone(Tone::new(variant));
            place(ui, &mut line, gap, button);
        }
        place(
            ui,
            &mut line,
            gap,
            Button::new(ID.with(7)).label("Disabled").enabled(false),
        );

        let mut line = cut_top(&mut content, row, gap);
        for (index, (hue, label)) in HUES.into_iter().enumerate() {
            let button = Button::new(ID.with(10 + index as u64))
                .label(label)
                .tone(Tone::ACCENT.with_hue(hue));
            place(ui, &mut line, gap, button);
        }
        let mut line = cut_top(&mut content, row, gap);
        for (index, ((hue, label), on)) in HUES.into_iter().zip(&mut self.families).enumerate() {
            let button = Button::new(ID.with(20 + index as u64))
                .label(label)
                .tone(Tone::DEFAULT.with_hue(hue))
                .toggle(on);
            place(ui, &mut line, gap, button);
        }

        let mut line = cut_top(&mut content, row, gap);
        self.groups_and_content(ui, &mut line, gap);

        let band = cut_top(&mut content, 2.0 * row + gap, gap);
        self.behaviours(ui, band, gap);

        let band = cut_top(&mut content, 2.0 * row + gap, gap);
        self.checkboxes(ui, band, gap);
    }

    /// A segmented choice, a toolbar group and a button with every kind of content.
    fn groups_and_content(&mut self, ui: &mut Ui<'_>, line: &mut Rect, gap: f32) {
        const ID: WidgetId = WidgetId::new("buttons-groups");
        let mut modes: [GroupCell<'_>; 3] = core::array::from_fn(|index| {
            GroupCell::new(
                Button::new(ID.with(index as u64))
                    .label(MODES[index])
                    .selected(self.mode == index),
            )
        });
        let group = ButtonGroup::horizontal();
        let width = group.size(ui, &modes).x;
        if let Some(index) = group
            .show(ui, cut_left(line, width, gap), &mut modes)
            .clicked
        {
            self.mode = index;
        }

        let icons = [Icon::Plus, Icon::Minus, Icon::Close];
        let mut tools: [GroupCell<'_>; 3] = core::array::from_fn(|index| {
            GroupCell::new(Button::new(ID.with(10 + index as u64)).icon(icons[index]))
        });
        let group = ButtonGroup::horizontal();
        let width = group.size(ui, &tools).x;
        group.show(ui, cut_left(line, width, gap), &mut tools);

        place(
            ui,
            line,
            gap,
            Button::new(ID.with(20))
                .leading_icon(Icon::Plus)
                .label("Add")
                .status_dot(Color::from_hex(0x27_AE_60))
                .trailing_icon(Icon::ChevronDown),
        );
    }

    /// Modifier toggles on a grid, step buttons that repeat while held, a counter, and
    /// buttons firing on press and on double press.
    fn behaviours(&mut self, ui: &mut Ui<'_>, mut band: Rect, gap: f32) {
        const ID: WidgetId = WidgetId::new("buttons-behaviours");
        let [left, right] = &mut self.sides;
        let mut modifiers = [
            GroupCell::new(Button::new(ID.with(0)).label("Ctrl"))
                .at(0, 0)
                .span(2, 1),
            GroupCell::new(Button::new(ID.with(1)).label("L").toggle(left)).at(0, 1),
            GroupCell::new(Button::new(ID.with(2)).label("R").toggle(right)).at(1, 1),
        ];
        let group = ButtonGroup::horizontal();
        let width = group.size(ui, &modifiers).x;
        group.show(ui, cut_left(&mut band, width, gap), &mut modifiers);

        let mut steps = [
            GroupCell::new(Button::new(ID.with(3)).icon(Icon::ChevronUp).repeat(true)),
            GroupCell::new(Button::new(ID.with(4)).icon(Icon::ChevronDown).repeat(true)),
        ];
        let group = ButtonGroup::vertical();
        let width = group.size(ui, &steps).x;
        let clicked = group
            .show(ui, cut_left(&mut band, width, gap), &mut steps)
            .clicked;
        let step = match clicked {
            Some(0) => 1,
            Some(_) => -1,
            None => 0,
        };

        let styles = ui.styles();
        let height = band.height();
        let number = cut_left(&mut band, 2.0 * height, gap);
        ui.label(
            &self.count_text,
            number,
            &styles.text.label,
            styles.text.primary,
            Align::Center,
        );

        // Two buttons stacked, as wide as the wider.
        let more = Button::new(ID.with(5))
            .label("+10 on press")
            .activation(Activation::Press);
        let reset = Button::new(ID.with(6))
            .label("Reset on double click")
            .tone(Tone::NEGATIVE)
            .activation(Activation::DoublePress);
        let width = more.size(ui).x.max(reset.size(ui).x);
        let mut column = cut_left(&mut band, width, gap);
        let half = ((column.height() - gap) * 0.5).max(0.0);
        let ten = more.show(ui, cut_top(&mut column, half, gap)).clicked;
        let reset = reset.show(ui, column).clicked;

        let count = if reset {
            0
        } else {
            self.count
                .saturating_add(step)
                .saturating_add(if ten { 10 } else { 0 })
        };
        if count != self.count {
            self.count = count;
            self.count_changed();
        }
    }

    /// Checkboxes with descriptions, one mixed, one disabled.
    fn checkboxes(&mut self, ui: &mut Ui<'_>, mut band: Rect, gap: f32) {
        const ID: WidgetId = WidgetId::new("buttons-checkboxes");
        let snap = Checkbox::new(ID.with(0), &mut self.snap)
            .label("Snap to grid")
            .description("Moves items by whole cells");
        let width = snap.size(ui).x;
        snap.show(ui, cut_left(&mut band, width, 2.0 * gap));
        let mixed = !self.overlays;
        let overlays = Checkbox::new(ID.with(1), &mut self.overlays)
            .label("Overlays")
            .description("Some guides are shown")
            .mixed(mixed);
        let width = overlays.size(ui).x;
        overlays.show(ui, cut_left(&mut band, width, 2.0 * gap));
        let mut locked = true;
        let locked = Checkbox::new(ID.with(2), &mut locked)
            .label("Locked")
            .description("Cannot change here")
            .enabled(false);
        let width = locked.size(ui).x;
        locked.show(ui, cut_left(&mut band, width, 2.0 * gap));
    }
}

/// Shows `button` at its own width at the left of `line`, and takes that width and a gap.
fn place(ui: &mut Ui<'_>, line: &mut Rect, gap: f32, button: Button<'_>) -> Response {
    let width = button.size(ui).x;
    button.show(ui, cut_left(line, width, gap))
}
