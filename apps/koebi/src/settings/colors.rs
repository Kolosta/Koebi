//! The Colors section of the Theme page: color settings above the tokens.
//!
//! A theme is made from chosen colors by contrast rules (`koebi_tokens::generate`, ADR 0027,
//! `docs/design-system/color.md`), in one of three modes: every palette from its own color,
//! everything from the background, or from the background and the accent. The section shows the
//! mode, then the background, the roles and the hues: a field for each color the mode asks for,
//! the fill of the others, which follow the background. A chosen color is used as it is; when a
//! rule had to move it (a background in the mid-tones, a fill too close to the background), a
//! note under it says where and why.
//!
//! A row shows the palette as it lies on the theme's background, each step's contrast with it,
//! and what its main steps hold: a fill with its content, text on a panel, a fill under the
//! primary text. The reset dot gives the theme's own palette back. The tokens stay in
//! Customisation for finer changes.
//!
//! Every part is laid out from measured widths, so that nothing is cut whatever the language
//! or the window's width: the names' column is as wide as the longest name, descriptions
//! wrap, and the contrasts below a strip are written for as many steps as their width allows,
//! each swatch telling its step, contrast and color when the pointer rests on it.

use core::fmt::Write as _;
use koebi_i18n::tr;

use koebi_core::palette::HUE_STEPS;
use koebi_core::{Color, Rect, Vec2};
use koebi_design::{DesignSystem, ThemeId};
use koebi_render::RectPrimitive;
use koebi_tokens::generate::{self, BackgroundFit, Mode, ROLE_FILLS};
use koebi_tokens::palettes::{self, Editable, Family, NEUTRAL_STEPS, PaletteTokens};
use koebi_tokens::semantic;
use koebi_ui::{
    Align, Button, ButtonGroup, CollapsiblePanel, CollapsiblePanelState, ColorField,
    ColorFieldState, GroupCell, Ui, WidgetId,
};

use super::Stack;

const ID: WidgetId = WidgetId::new("settings-colors");

/// The palettes of the first container: the background (the neutral grays) and the roles.
const MAIN: usize = 6;

/// The mode buttons' labels, in the order of [`Mode::ALL`].
const MODE_LABELS: [&str; 3] = [
    koebi_i18n::n_!("Each palette"),
    koebi_i18n::n_!("The background"),
    koebi_i18n::n_!("Background and accent"),
];

/// What each mode does, in the order of [`Mode::ALL`].
const MODE_TEXTS: [&str; 3] = [
    koebi_i18n::n_!("Every palette from its own color: the theme's, or one you choose."),
    koebi_i18n::n_!(
        "One color makes the theme: the background. The accent takes its hue, the other colors lean to it."
    ),
    koebi_i18n::n_!(
        "Two colors make the theme: the background and the accent. The other colors lean to the background's hue."
    ),
];

/// What the section keeps between frames.
#[derive(Debug)]
pub(super) struct ColorsSection {
    palettes: Vec<Editable>,
    /// The tokens of each palette, found once in the design system: frames read them without
    /// writing their paths (`None` for a palette missing from the schema, not shown).
    tokens: Vec<Option<PaletteTokens>>,
    fields: Vec<ColorFieldState>,
    /// The palettes' contrast labels, written again when the design system changes.
    labels: Vec<[String; 16]>,
    /// What a rule did to each palette's chosen color (moved, used as the deepest surface),
    /// written again when the design system changes; empty when nothing.
    notes: Vec<String>,
    /// The palettes of the first container in the order shown: the background, then the roles.
    main: [usize; MAIN],
    revision: Option<u64>,
    every_theme: bool,
    hues: CollapsiblePanelState,
    /// The height of each palette's row, measured each frame for the section's width.
    heights: Vec<f32>,
    /// The text of the swatch tooltip showing: written while it shows, its storage kept.
    tip: String,
}

/// Where the parts of a palette's row lie, measured for the section's width.
#[derive(Clone, Copy, Debug)]
struct RowLayout {
    /// The column of the names, as wide as the longest.
    names: f32,
    /// The source's color field.
    field: f32,
    /// The samples at the end of the row.
    samples: f32,
}

impl RowLayout {
    /// The width of a palette's description: under its name and its field.
    fn description(&self, gap: f32) -> f32 {
        self.names + gap + self.field
    }
}

impl ColorsSection {
    pub(super) fn new() -> Self {
        let palettes: Vec<Editable> = palettes::editable().collect();
        Self {
            fields: palettes.iter().map(|_| ColorFieldState::new()).collect(),
            labels: palettes.iter().map(|_| Default::default()).collect(),
            notes: palettes.iter().map(|_| String::new()).collect(),
            main: main_order(&palettes),
            tokens: Vec::new(),
            palettes,
            revision: None,
            every_theme: false,
            hues: CollapsiblePanelState::new(false),
            heights: Vec::new(),
            tip: String::new(),
        }
    }

    /// The themes a change applies to: every theme, or the active one.
    fn themes<'d>(&self, design: &'d DesignSystem) -> impl Iterator<Item = ThemeId> + 'd {
        let (every, active) = (self.every_theme, design.active_theme());
        design
            .themes()
            .map(|(id, _)| id)
            .filter(move |&id| every || id == active)
    }

    /// Writes the contrast of every step of every palette with the active theme's background.
    fn refresh_labels(&mut self, design: &DesignSystem) {
        if self.revision == Some(design.revision()) {
            return;
        }
        self.revision = Some(design.revision());
        if self.tokens.len() != self.palettes.len() {
            self.tokens = self
                .palettes
                .iter()
                .map(|palette| palette.tokens(design))
                .collect();
        }
        self.refresh_notes(design);
        let background = design.get(semantic::BACKGROUND_BASE_DEFAULT);
        for (tokens, labels) in self.tokens.iter().zip(&mut self.labels) {
            let Some(tokens) = tokens else { continue };
            for (label, color) in labels.iter_mut().zip(tokens.steps(design)) {
                label.clear();
                let ratio = color.contrast_ratio(background);
                let _ = if ratio < 10.0 {
                    write!(label, "{ratio:.1}")
                } else {
                    write!(label, "{ratio:.0}")
                };
            }
        }
    }

    /// Writes what the rules did to the active theme's chosen colors
    /// ([`generate::report`]).
    fn refresh_notes(&mut self, design: &DesignSystem) {
        let report = generate::report(design, design.active_theme());
        for (palette, note) in self.palettes.iter().zip(&mut self.notes) {
            note.clear();
            let hex = |color: Color| {
                let [r, g, b, _] = color.to_rgba8();
                (r, g, b)
            };
            if palette.family == Family::Neutral {
                match report.background {
                    Some(BackgroundFit::Moved(color)) => {
                        let (r, g, b) = hex(color);
                        koebi_i18n::fill_into(
                            note,
                            tr("Moved to {color}: in the mid-tones, no text reads well enough."),
                            &[("color", &format_args!("#{r:02X}{g:02X}{b:02X}"))],
                        );
                    }
                    Some(BackgroundFit::Deepest) => note.push_str(tr(
                        "So close to black or white that no surface fits beyond it: it is the deepest surface, and the background lies a step away.",
                    )),
                    Some(BackgroundFit::Exact) | None => {}
                }
            } else if let Some(&(_, color)) =
                report.moved.iter().find(|(key, _)| *key == palette.key)
            {
                let (r, g, b) = hex(color);
                koebi_i18n::fill_into(
                    note,
                    tr("Moved to {color}: a fill keeps the theme's contrast with the background."),
                    &[("color", &format_args!("#{r:02X}{g:02X}{b:02X}"))],
                );
            }
        }
    }

    /// The parts of a row for a section `width` wide: the names' column holds the longest
    /// name, and the longest description on one line beside the field below it, as far as a
    /// third of the width; past it, descriptions wrap.
    fn layout(&self, ui: &mut Ui<'_>, width: f32) -> RowLayout {
        let row = ui.styles().button.height;
        let gap = ui.styles().property_group.row_gap;
        let label = ui.styles().text.label.clone();
        let detail = ui.styles().text.detail.clone();
        let field = 3.0 * row;
        let longest = self
            .palettes
            .iter()
            .map(|palette| {
                let name = ui.text_width(tr(palette.name), &label);
                let description = ui.text_width(tr(palette.description), &detail);
                name.max(description - gap - field)
            })
            .fold(0.0_f32, f32::max);
        let names = longest
            .ceil()
            .min((width * 0.35).max(4.0 * row))
            .max(4.0 * row);
        RowLayout {
            names,
            field,
            samples: 3.0 * 2.0 * row + 2.0 * gap,
        }
    }

    /// The height of the palette at `index`'s row: its name over its description, wrapped,
    /// its strip over the contrasts.
    fn row_height(&self, ui: &mut Ui<'_>, index: usize, layout: RowLayout) -> f32 {
        let row = ui.styles().button.height;
        let gap = ui.styles().property_group.row_gap;
        let detail = ui.styles().text.detail.clone();
        let description = tr(self.palettes[index].description);
        let mut lines = ui.paragraph_height(description, layout.description(gap), &detail);
        let note = &self.notes[index];
        if !note.is_empty() {
            lines += gap + ui.paragraph_height(note, layout.description(gap), &detail);
        }
        row + gap + lines.max(row)
    }

    /// The mode's buttons, each label as wide as it asks.
    fn mode_cells(mode: Mode) -> [GroupCell<'static>; 3] {
        let cell = |index: usize| {
            GroupCell::new(
                Button::new(ID.with(20 + index as u64))
                    .label(tr(MODE_LABELS[index]))
                    .selected(Mode::ALL[index] == mode),
            )
            .at(u16::try_from(index).unwrap_or(0), 0)
        };
        [cell(0), cell(1), cell(2)]
    }

    /// Where the mode's row puts its buttons and its text, for a row `width` wide: side by
    /// side when the buttons take three fifths of it at most, else the text below them.
    fn mode_layout(ui: &mut Ui<'_>, width: f32, mode: Mode) -> (f32, bool) {
        let natural = ButtonGroup::horizontal()
            .size(ui, &Self::mode_cells(mode))
            .x
            .ceil();
        (natural.min(width), natural <= width * 0.6)
    }

    /// The height of the mode's row: its buttons, and what the mode does, wrapped.
    fn mode_height(ui: &mut Ui<'_>, width: f32, mode: Mode) -> f32 {
        let row = ui.styles().button.height;
        let gap = ui.styles().property_group.row_gap;
        let label = ui.styles().text.label.clone();
        let (buttons, beside) = Self::mode_layout(ui, width, mode);
        let text = MODE_TEXTS[Mode::ALL.iter().position(|&m| m == mode).unwrap_or(0)];
        if beside {
            ui.paragraph_height(tr(text), (width - buttons - gap).max(1.0), &label)
                .max(row)
        } else {
            row + gap + ui.paragraph_height(tr(text), width, &label)
        }
    }

    /// The choice of the mode: what the chosen colors make, for the themes of the scope.
    fn mode_row(&mut self, ui: &mut Ui<'_>, rect: Rect, design: &mut DesignSystem, mode: Mode) {
        let row = ui.styles().button.height;
        let gap = ui.styles().property_group.row_gap;
        let label = ui.styles().text.label.clone();
        let secondary = ui.styles().text.secondary;
        let (width, beside) = Self::mode_layout(ui, rect.width(), mode);
        let (buttons, text) = if beside {
            (
                Rect::from_min_max(
                    Vec2::new(rect.max.x - width, rect.min.y),
                    Vec2::new(rect.max.x, rect.min.y + row),
                ),
                Rect::from_min_max(rect.min, Vec2::new(rect.max.x - width - gap, rect.max.y)),
            )
        } else {
            (
                Rect::from_origin_size(rect.min, Vec2::new(width, row)),
                Rect::from_min_max(Vec2::new(rect.min.x, rect.min.y + row + gap), rect.max),
            )
        };
        let index = Mode::ALL.iter().position(|&m| m == mode).unwrap_or(0);
        ui.paragraph(tr(MODE_TEXTS[index]), text, &label, secondary);
        let mut cells = Self::mode_cells(mode);
        if let Some(chosen) = ButtonGroup::horizontal()
            .show(ui, buttons, &mut cells)
            .clicked
            .and_then(|index| Mode::ALL.get(index).copied())
            && chosen != mode
        {
            let themes: Vec<ThemeId> = self.themes(design).collect();
            for theme in themes {
                if let Err(error) = generate::set_mode(design, theme, chosen) {
                    koebi_core::warn!("cannot change how the theme is made: {error}");
                }
            }
        }
    }

    /// The text beside the choice of scope.
    fn scope_text() -> &'static str {
        tr("The colors chosen here make the theme again, every step keeping its contrast.")
    }

    /// The width of the choice of scope's buttons, in a row `width` wide.
    fn scope_buttons(width: f32, row: f32) -> f32 {
        (width * 0.5).min(14.0 * row)
    }

    /// The height of the scope's row: its text wrapped beside the buttons.
    fn scope_height(ui: &mut Ui<'_>, width: f32) -> f32 {
        let row = ui.styles().button.height;
        let gap = ui.styles().property_group.row_gap;
        let label = ui.styles().text.label.clone();
        let text = (width - Self::scope_buttons(width, row) - gap).max(1.0);
        ui.paragraph_height(Self::scope_text(), text, &label)
            .max(row)
    }

    /// Shows the section below what `stack` holds.
    pub(super) fn show(&mut self, ui: &mut Ui<'_>, stack: &mut Stack, design: &mut DesignSystem) {
        self.refresh_labels(design);
        let row = ui.styles().button.height;
        let gap = ui.styles().property_group.row_gap;
        let bounds = stack.bounds;
        // Every row measured first, for the section's width.
        let width = stack.content(ui).width();
        let layout = self.layout(ui, width);
        let scope = Self::scope_height(ui, width);
        let mode = Mode::of(design, design.active_theme());
        let mode_height = Self::mode_height(ui, width, mode);
        self.heights.clear();
        for index in 0..self.palettes.len() {
            let height = self.row_height(ui, index, layout);
            self.heights.push(height);
        }
        let rows: f32 = self
            .main
            .iter()
            .map(|&index| self.heights[index] + gap)
            .sum();
        let height = row + gap + scope + gap + mode_height + gap + rows;
        stack.container(ui, height, |ui, content| {
            let title = Rect::from_origin_size(content.min, Vec2::new(content.width(), row));
            ui.section_title(title, tr("Colors"));
            let at = Rect::from_origin_size(
                Vec2::new(content.min.x, title.max.y + gap),
                Vec2::new(content.width(), scope),
            );
            self.scope(ui, at);
            let at = Rect::from_origin_size(
                Vec2::new(content.min.x, at.max.y + gap),
                Vec2::new(content.width(), mode_height),
            );
            self.mode_row(ui, at, design, mode);
            let mut y = at.max.y + gap;
            for index in self.main {
                let line = self.heights[index] + gap;
                let at = Rect::from_min_max(
                    Vec2::new(content.min.x, y),
                    Vec2::new(content.max.x, y + line),
                );
                self.palette_row(ui, at, index, design, bounds, layout);
                y += line;
            }
        });
        let mut state = core::mem::replace(&mut self.hues, CollapsiblePanelState::new(false));
        stack.panel(
            ui,
            CollapsiblePanel::new(ID.with(1), &mut state, tr("Hue palettes")),
            |ui, rect| {
                let mut y = rect.min.y;
                let main = self.main;
                for index in (0..self.palettes.len()).filter(|index| !main.contains(index)) {
                    let line = self.heights[index] + gap;
                    let at = Rect::from_min_max(
                        Vec2::new(rect.min.x, y),
                        Vec2::new(rect.max.x, y + line),
                    );
                    self.palette_row(ui, at, index, design, bounds, layout);
                    y += line;
                }
                y - rect.min.y
            },
        );
        self.hues = state;
    }

    /// The choice of the themes a change applies to: its text wraps beside the buttons.
    fn scope(&mut self, ui: &mut Ui<'_>, rect: Rect) {
        let row = ui.styles().button.height;
        let gap = ui.styles().property_group.row_gap;
        let label = ui.styles().text.label.clone();
        let secondary = ui.styles().text.secondary;
        let width = Self::scope_buttons(rect.width(), row);
        let text = Rect::from_min_max(rect.min, Vec2::new(rect.max.x - width - gap, rect.max.y));
        ui.paragraph(Self::scope_text(), text, &label, secondary);
        let at = Rect::from_min_max(
            Vec2::new(rect.max.x - width, rect.min.y),
            Vec2::new(rect.max.x, rect.min.y + row),
        );
        let mut cells = [
            GroupCell::new(
                Button::new(ID.with(10))
                    .label(tr("This theme"))
                    .selected(!self.every_theme),
            ),
            GroupCell::new(
                Button::new(ID.with(11))
                    .label(tr("Every theme"))
                    .selected(self.every_theme),
            )
            .at(1, 0),
        ];
        let columns = [koebi_ui::Track::Share(1.0); 2];
        match ButtonGroup::horizontal()
            .columns(&columns)
            .show(ui, at, &mut cells)
            .clicked
        {
            Some(0) => self.every_theme = false,
            Some(1) => self.every_theme = true,
            _ => {}
        }
    }

    /// The row of the palette at `index` in `rect`, its parts where `layout` puts them.
    fn palette_row(
        &mut self,
        ui: &mut Ui<'_>,
        rect: Rect,
        index: usize,
        design: &mut DesignSystem,
        bounds: Rect,
        layout: RowLayout,
    ) {
        let palette = self.palettes[index];
        let Some(tokens) = self.tokens.get(index).copied().flatten() else {
            return;
        };
        let id = ID.with(100 + index as u64 * 4);
        let row = ui.styles().button.height;
        let gap = ui.styles().property_group.row_gap;
        let active = design.active_theme();
        // In the background modes, the roles follow the background: no color of their own.
        let derived = !Mode::of(design, active).takes(&palette);
        // The reset dot, beside the name: the palette shows other colors than the theme's own.
        let changed = !derived
            && self
                .themes(design)
                .any(|theme| tokens.is_changed(design, theme));
        let dot = Rect::from_origin_size(rect.min, Vec2::splat(row));
        if changed
            && ui
                .reset_dot(id.with(1), dot, tr("Back to the theme's own palette"))
                .clicked
        {
            let themes: Vec<ThemeId> = self.themes(design).collect();
            for &theme in &themes {
                if let Err(error) = palettes::reset(design, theme, &palette) {
                    koebi_core::warn!("cannot reset the {} palette: {error}", palette.key);
                }
            }
        }
        let label = ui.styles().text.label.clone();
        let detail = ui.styles().text.detail.clone();
        let (primary, secondary) = (ui.styles().text.primary, ui.styles().text.secondary);
        let name = Rect::from_origin_size(
            Vec2::new(dot.max.x + gap, rect.min.y),
            Vec2::new(layout.names, row),
        );
        ui.label(tr(palette.name), name, &label, primary, Align::Start);
        // Under the name and the field, wrapped: the row is as high as it needs.
        let description = Rect::from_min_max(
            Vec2::new(name.min.x, name.max.y + gap),
            Vec2::new(name.min.x + layout.description(gap), rect.max.y),
        );
        let lines = ui.paragraph(tr(palette.description), description, &detail, secondary);
        if !self.notes[index].is_empty() {
            let note = Rect::from_min_max(
                Vec2::new(description.min.x, description.min.y + lines + gap),
                description.max,
            );
            ui.paragraph(&self.notes[index], note, &detail, primary);
        }

        // The source, in a color field.
        let field = Rect::from_origin_size(
            Vec2::new(name.max.x + gap, rect.min.y),
            Vec2::new(layout.field, row),
        );
        if derived {
            Self::derived_fill(ui, id.with(2), field, design, &palette);
        } else {
            self.source_field(
                ui,
                id.with(2),
                field,
                index,
                design,
                bounds,
                &palette,
                &tokens,
            );
        }

        // The palette on the theme's background, each step's contrast below it, then samples.
        let samples = layout.samples;
        let strip = Rect::from_min_max(
            Vec2::new(field.max.x + 2.0 * gap, rect.min.y),
            Vec2::new(
                (rect.max.x - samples - gap).max(field.max.x + 2.0 * gap),
                rect.min.y + row,
            ),
        );
        let background = design.get(semantic::BACKGROUND_BASE_DEFAULT);
        let steps = tokens.steps(design);
        ui.scene()
            .push_rect(&RectPrimitive::new(strip, background).with_corner_radius(row * 0.2));
        let width = strip.width() / steps.len().max(1) as f32;
        // The contrasts of as many steps as their width allows: every step, or every second,
        // third... from the first. The swatch under the pointer tells its own.
        let widest = self.labels[index]
            .iter()
            .map(|label| ui.text_width(label, &detail))
            .fold(0.0_f32, f32::max)
            .ceil()
            + 2.0;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a whole number of steps, between 1 and 16"
        )]
        let every = ((widest / width.max(1.0)).ceil() as usize).clamp(1, steps.len().max(1));
        let numbers = match palette.family {
            Family::Neutral => &NEUTRAL_STEPS,
            Family::Role | Family::Hue => &HUE_STEPS,
        };
        for (step, color) in steps.iter().enumerate() {
            let x = strip.min.x + step as f32 * width;
            let swatch = Rect::from_min_max(
                Vec2::new(x + 1.0, strip.min.y + 2.0),
                Vec2::new(x + width - 1.0, strip.max.y - 2.0),
            );
            ui.scene()
                .push_rect(&RectPrimitive::new(swatch, *color).with_corner_radius(2.0));
            let key = id.with(40 + step as u64);
            if ui.interact(key, swatch).hovered {
                let [r, g, b, _] = color.to_rgba8();
                let hex = format_args!("#{r:02X}{g:02X}{b:02X}");
                koebi_i18n::fill_into(
                    &mut self.tip,
                    tr("Step {step}: {contrast}:1 against the background, {color}"),
                    &[
                        ("step", &numbers.get(step).copied().unwrap_or_default()),
                        ("contrast", &self.labels[index][step]),
                        ("color", &hex),
                    ],
                );
                ui.tooltip(key, &self.tip, swatch);
            }
            if step % every == 0 {
                // Centred under its swatch, inside the strip.
                let left = (x + width * 0.5 - widest * 0.5)
                    .clamp(strip.min.x, (strip.max.x - widest).max(strip.min.x));
                let under = Rect::from_origin_size(
                    Vec2::new(left, strip.max.y + gap),
                    Vec2::new(widest, row),
                );
                ui.label(
                    &self.labels[index][step],
                    under,
                    &detail,
                    secondary,
                    Align::Center,
                );
            }
        }
        let at = Vec2::new(strip.max.x + gap, rect.min.y);
        samples_of(ui, at, row, gap, design, &palette, &steps);
    }
}

impl ColorsSection {
    /// The field of a palette's chosen color: a change makes the theme again, for every theme
    /// of the scope.
    #[allow(
        clippy::too_many_arguments,
        reason = "the parts of one row, laid out by its caller"
    )]
    fn source_field(
        &mut self,
        ui: &mut Ui<'_>,
        id: WidgetId,
        field: Rect,
        index: usize,
        design: &mut DesignSystem,
        bounds: Rect,
        palette: &Editable,
        tokens: &PaletteTokens,
    ) {
        let active = design.active_theme();
        let mut color = tokens.source(design, active).unwrap_or(Color::BLACK);
        let own = tokens.own_source(design, active).unwrap_or(color);
        // A palette is made from an opaque color: no opacity to edit.
        if ColorField::new(id, &mut color, &mut self.fields[index])
            .dropdown(true)
            .opacity(false)
            .original(own)
            .title(tr(palette.name))
            .show(ui, field, bounds)
            .changed
        {
            let themes: Vec<ThemeId> = self.themes(design).collect();
            for &theme in &themes {
                if let Err(error) = palettes::seed(design, theme, palette, color) {
                    koebi_core::warn!("cannot make the {} palette: {error}", palette.key);
                }
            }
        }
    }

    /// In place of a field, the fill of a role that follows the background, and why in a
    /// tooltip.
    fn derived_fill(
        ui: &mut Ui<'_>,
        id: WidgetId,
        field: Rect,
        design: &DesignSystem,
        palette: &Editable,
    ) {
        let row = field.height();
        let fill = ROLE_FILLS
            .iter()
            .find(|fills| fills.key == palette.key)
            .and_then(|fills| match design.value(fills.default) {
                Some(koebi_design::value::Value::Color(color)) => Some(*color),
                _ => None,
            })
            .unwrap_or(Color::TRANSPARENT);
        let border = design.get(semantic::BORDER_COLOR_DEFAULT);
        ui.scene().push_rect(
            &RectPrimitive::new(field, fill)
                .with_corner_radius(row * 0.2)
                .with_border(1.0, border),
        );
        let tip = if palette.key == "accent" {
            tr("The accent takes the background's hue.")
        } else {
            tr("This color leans to the background's hue.")
        };
        ui.interact(id, field);
        ui.tooltip(id, tip, field);
    }
}

/// The palettes of the first container in the order shown: the background, then the roles.
fn main_order(palettes: &[Editable]) -> [usize; MAIN] {
    let mut order = [0; MAIN];
    order[0] = palettes
        .iter()
        .position(|palette| palette.family == Family::Neutral)
        .unwrap_or(0);
    let roles = (0..palettes.len()).filter(|&index| palettes[index].family == Family::Role);
    for (slot, index) in order[1..].iter_mut().zip(roles) {
        *slot = index;
    }
    order
}

/// What the palette's main steps hold, from `at`: a fill with its content (step 900 and the
/// content on colored fills), text in the palette on a panel (1400), a fill under the primary
/// text (500). For the neutral grays: text on a panel, on a field, and a border.
fn samples_of(
    ui: &mut Ui<'_>,
    at: Vec2,
    row: f32,
    gap: f32,
    design: &DesignSystem,
    palette: &Editable,
    steps: &[Color],
) {
    let panel = design.get(semantic::BACKGROUND_RAISED_DEFAULT);
    let text = design.get(semantic::TEXT_COLOR_PRIMARY_DEFAULT);
    let get = |index: usize| steps.get(index).copied().unwrap_or(Color::BLACK);
    let on = ROLE_FILLS
        .iter()
        .find(|fills| fills.key == palette.key)
        .and_then(|fills| match design.value(fills.content) {
            Some(koebi_design::value::Value::Color(color)) => Some(*color),
            _ => None,
        })
        .unwrap_or_else(|| {
            if Color::WHITE.contrast_ratio(get(8)) >= Color::BLACK.contrast_ratio(get(8)) {
                Color::WHITE
            } else {
                Color::BLACK
            }
        });
    let chips: [(Color, Color, Option<Color>); 3] = if palette.family == Family::Neutral {
        // Text on a panel, on a field, and a border around a field.
        [
            (get(4), get(14), None),
            (get(7), get(14), None),
            (panel, text, Some(get(9))),
        ]
    } else {
        [
            (get(8), on, None),
            (panel, get(13), None),
            (get(4), text, None),
        ]
    };
    let label = ui.styles().text.label.clone();
    for (index, (fill, ink, border)) in chips.into_iter().enumerate() {
        let chip = Rect::from_origin_size(
            Vec2::new(at.x + index as f32 * (2.0 * row + gap), at.y),
            Vec2::new(2.0 * row, row),
        );
        let mut rect = RectPrimitive::new(chip, fill).with_corner_radius(row * 0.2);
        if let Some(border) = border {
            rect = rect.with_border(1.0, border);
        }
        ui.scene().push_rect(&rect);
        ui.label(tr("Aa"), chip, &label, ink, Align::Center);
    }
}

#[cfg(test)]
mod tests {
    use super::super::Page;
    use super::super::tests::Harness;

    #[test]
    fn the_theme_page_lists_every_palette_with_its_source() {
        let mut harness = Harness::new();
        harness.settings.page = Page::Theme;
        drop(harness.frame());
        let section = &harness.settings.theme.colors;
        assert_eq!(section.palettes.len(), 24);
        // Every row knows the contrast of each of its steps.
        assert!(
            section
                .labels
                .iter()
                .all(|labels| labels.iter().all(|label| !label.is_empty()))
        );
    }
}
