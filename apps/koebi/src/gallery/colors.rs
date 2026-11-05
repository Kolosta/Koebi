//! The colors of the gallery: color fields opening their picker as a dropdown and as a panel of
//! their own, a horizontal strip, and a picker shown in place; above them, the settings to try:
//! the picker's shape, how each half of the fields shows transparency, its title, its palette
//! and the order of its parts.

use koebi_core::{Color, Rect, Vec2};
use koebi_render::HueModel;
use koebi_ui::{
    Align, Checkbox, ColorField, ColorFieldState, ColorPicker, ColorPickerState, ColorStrip,
    ColorStripState, Dropdown, MenuItem, MenuState, PanelLevel, PickerSection, PickerShape,
    StripFill, Transparency, Ui, WidgetId,
};

use super::{Metrics, cut_left, cut_top};

/// The parts of the picker with its fields first.
const VALUES_FIRST: [PickerSection; 5] = [
    PickerSection::Title,
    PickerSection::Values,
    PickerSection::Modes,
    PickerSection::Area,
    PickerSection::Palette,
];

/// A palette to offer.
const PALETTE: [Color; 8] = [
    Color::from_hex(0xE0_3E_3E),
    Color::from_hex(0xF0_8A_24),
    Color::from_hex(0xF2_C9_4C),
    Color::from_hex(0x4C_B8_6A),
    Color::from_hex(0x3D_8B_E0),
    Color::from_hex(0x8E_5C_D9),
    Color::from_hex(0xFF_FF_FF),
    Color::from_hex(0x1E_1E_1E),
];

/// What the panel keeps: the colors, the fields' and the strip's states, and the settings.
#[derive(Debug)]
pub(super) struct Colors {
    dropdown: (Color, ColorFieldState),
    panel: (Color, ColorFieldState),
    inline: (Color, ColorPickerState),
    hue: (f32, ColorStripState),
    shape: (PickerShape, MenuState),
    left: (Transparency, MenuState),
    right: (Transparency, MenuState),
    titled: bool,
    palette: bool,
    values_first: bool,
}

impl Colors {
    pub(super) fn new() -> Self {
        Self {
            dropdown: (Color::new(0.24, 0.53, 0.88, 0.7), ColorFieldState::new()),
            panel: (Color::from_hex(0x4C_B8_6A), ColorFieldState::new()),
            inline: (Color::new(0.88, 0.45, 0.2, 0.85), ColorPickerState::new()),
            hue: (0.6, ColorStripState::new()),
            shape: (PickerShape::SquareSv, MenuState::new()),
            left: (Transparency::Opaque, MenuState::new()),
            right: (Transparency::Checker, MenuState::new()),
            titled: true,
            palette: true,
            values_first: false,
        }
    }

    /// The height of the panel: two rows of settings and fields, then the picker in place.
    pub(super) fn height(&self, ui: &Ui<'_>, metrics: Metrics) -> f32 {
        let panel = &ui.styles().panel;
        let picker = self.picker_size(ui);
        2.0 * metrics.row
            + 2.0 * metrics.row_gap()
            + picker.y
            + 2.0 * (panel.padding + panel.border_width)
    }

    /// The size of the picker shown in place, with the settings chosen.
    fn picker_size(&self, ui: &Ui<'_>) -> Vec2 {
        let mut state = ColorPickerState::new();
        let mut color = self.inline.0;
        let picker = ColorPicker::new(WidgetId::new("measure"), &mut state, &mut color);
        configured(
            picker,
            self.shape.0,
            self.titled,
            self.palette,
            self.values_first,
        )
        .size(ui)
    }

    /// Draws the panel in `rect`; its menus and pickers open inside `window`.
    pub(super) fn show(&mut self, ui: &mut Ui<'_>, rect: Rect, window: Rect, metrics: Metrics) {
        const ID: WidgetId = WidgetId::new("colors");
        let mut content = ui.panel(rect, PanelLevel::First);
        let (row, gap) = (metrics.row, metrics.row_gap());

        // The settings.
        let mut line = cut_top(&mut content, row, gap);
        let fifth = ((line.width() - 4.0 * gap) / 5.0).floor();
        let shapes = PickerShape::ALL.map(|shape| MenuItem::action(shape.label()));
        if let Some(choice) = Dropdown::new(ID.with(1), &mut self.shape.1, &shapes)
            .label(self.shape.0.label())
            .show(ui, cut_left(&mut line, fifth, gap), window)
            && let Some(&shape) = PickerShape::ALL.get(choice.index())
        {
            self.shape.0 = shape;
        }
        let ways = Transparency::ALL.map(|way| MenuItem::action(way.label()));
        for (index, half) in [&mut self.left, &mut self.right].into_iter().enumerate() {
            let id = ID.with(2 + index as u64);
            let at = cut_left(&mut line, fifth, gap);
            if let Some(choice) = Dropdown::new(id, &mut half.1, &ways)
                .label(half.0.label())
                .show(ui, at, window)
                && let Some(&way) = Transparency::ALL.get(choice.index())
            {
                half.0 = way;
            }
            let tip = if index == 0 {
                "How the left half of the fields shows transparency"
            } else {
                "How the right half of the fields shows transparency"
            };
            ui.tooltip(id, tip, at);
        }
        let third = ((line.width() - 2.0 * gap) / 3.0).floor();
        Checkbox::new(ID.with(4), &mut self.titled)
            .label("Title")
            .show(ui, cut_left(&mut line, third, gap));
        Checkbox::new(ID.with(5), &mut self.palette)
            .label("Palette")
            .show(ui, cut_left(&mut line, third, gap));
        Checkbox::new(ID.with(6), &mut self.values_first)
            .label("Values first")
            .show(ui, line);

        // A field opening its picker as a dropdown, one as a panel, a horizontal strip.
        let mut line = cut_top(&mut content, row, gap);
        let quarter = ((line.width() - 3.0 * gap) / 4.0).floor();
        let font = ui.styles().text.label.clone();
        let secondary = ui.styles().text.secondary;
        let (left, right) = (self.left.0, self.right.0);
        for (index, ((color, state), label)) in [
            (&mut self.dropdown, "As a dropdown"),
            (&mut self.panel, "As a panel"),
        ]
        .into_iter()
        .enumerate()
        {
            let mut cell = cut_left(&mut line, quarter, gap);
            let caption = cut_left(&mut cell, quarter * 0.45, gap);
            ui.label(label, caption, &font, secondary, Align::Start);
            let mut field = ColorField::new(ID.with(10 + index as u64), color, state)
                .halves(left, right)
                .dropdown(index == 0)
                .shape(self.shape.0);
            if self.titled {
                field = field.title(label);
            }
            if self.palette {
                field = field.palette(&PALETTE);
            }
            field.show(ui, cell, window);
        }
        let mut strip = line;
        let caption = cut_left(&mut strip, quarter * 0.45, gap);
        ui.label(
            "A horizontal strip",
            caption,
            &font,
            secondary,
            Align::Start,
        );
        let strip = Rect::from_min_max(
            Vec2::new(strip.min.x, strip.center().y - row * 0.3),
            Vec2::new(strip.max.x, strip.center().y + row * 0.3),
        );
        let (hue, state) = &mut self.hue;
        ColorStrip::new(
            ID.with(20),
            state,
            hue,
            StripFill::Hues {
                model: HueModel::Hsv,
                lightness: 0.0,
                chroma: 0.0,
            },
        )
        .vertical(false)
        .show(ui, strip);

        // The picker in place, with the settings chosen.
        let size = self.picker_size(ui);
        let at = Rect::from_origin_size(content.min, size);
        let (color, state) = &mut self.inline;
        let picker = ColorPicker::new(ID.with(30), state, color).eyedropper(true);
        let picker = configured(
            picker,
            self.shape.0,
            self.titled,
            self.palette,
            self.values_first,
        );
        picker.show(ui, at);
    }
}

/// `picker` with the settings given.
fn configured(
    picker: ColorPicker<'_>,
    shape: PickerShape,
    titled: bool,
    palette: bool,
    values_first: bool,
) -> ColorPicker<'_> {
    let mut picker = picker.shape(shape).original(Color::from_hex(0x3D_8B_E0));
    if titled {
        picker = picker.title("Accent");
    }
    if palette {
        picker = picker.palette(&PALETTE);
    }
    if values_first {
        picker = picker.sections(&VALUES_FIRST);
    }
    picker
}
