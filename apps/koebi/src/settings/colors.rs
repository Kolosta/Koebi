//! The Colors section of the Theme page: color settings above the tokens.
//!
//! Each palette (a color role, the neutral grays, a hue) is made again from one color, its
//! source, for the background of each theme it applies to: every state, part and component
//! aliasing its steps follows, and every step keeps its contrast (`koebi_tokens::palettes`,
//! `docs/design-system/color.md`). A row shows the palette as it lies on the theme's
//! background, each step's contrast with it, and what its main steps hold: a fill with its
//! content, text on a panel, a fill under the primary text. The reset dot gives the theme's own
//! palette back. The tokens stay in Customisation for finer changes.

use core::fmt::Write as _;
use koebi_i18n::tr;

use koebi_core::{Color, Rect, Vec2};
use koebi_design::{DesignSystem, Layers, ThemeId};
use koebi_render::RectPrimitive;
use koebi_tokens::palettes::{self, Editable, Family};
use koebi_tokens::semantic;
use koebi_ui::{
    Align, Button, ButtonGroup, CollapsiblePanel, CollapsiblePanelState, ColorField,
    ColorFieldState, GroupCell, Ui, WidgetId,
};

use super::Stack;

const ID: WidgetId = WidgetId::new("settings-colors");

/// The palettes of the first container: the roles and the neutral grays.
const MAIN: usize = 6;

/// What the section keeps between frames.
#[derive(Debug)]
pub(super) struct ColorsSection {
    palettes: Vec<Editable>,
    fields: Vec<ColorFieldState>,
    /// The palettes' contrast labels, written again when the design system changes.
    labels: Vec<[String; 16]>,
    revision: Option<u64>,
    every_theme: bool,
    hues: CollapsiblePanelState,
}

impl ColorsSection {
    pub(super) fn new() -> Self {
        let palettes: Vec<Editable> = palettes::editable().collect();
        Self {
            fields: palettes.iter().map(|_| ColorFieldState::new()).collect(),
            labels: palettes.iter().map(|_| Default::default()).collect(),
            palettes,
            revision: None,
            every_theme: false,
            hues: CollapsiblePanelState::new(false),
        }
    }

    /// The themes a change applies to.
    fn themes(&self, design: &DesignSystem) -> Vec<ThemeId> {
        if self.every_theme {
            design.themes().map(|(id, _)| id).collect()
        } else {
            vec![design.active_theme()]
        }
    }

    /// Writes the contrast of every step of every palette with the active theme's background.
    fn refresh_labels(&mut self, design: &DesignSystem) {
        if self.revision == Some(design.revision()) {
            return;
        }
        self.revision = Some(design.revision());
        let background = design.get(semantic::BACKGROUND_BASE_DEFAULT);
        for (palette, labels) in self.palettes.iter().zip(&mut self.labels) {
            for (label, color) in labels.iter_mut().zip(steps(design, palette)) {
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

    /// The height of a palette's row: its name over its description, its strip over the
    /// contrasts.
    fn row_height(ui: &Ui<'_>) -> f32 {
        let row = ui.styles().button.height;
        2.0 * row + ui.styles().property_group.row_gap
    }

    /// Shows the section below what `stack` holds.
    pub(super) fn show(&mut self, ui: &mut Ui<'_>, stack: &mut Stack, design: &mut DesignSystem) {
        self.refresh_labels(design);
        let row = ui.styles().button.height;
        let gap = ui.styles().property_group.row_gap;
        let line = Self::row_height(ui) + gap;
        let bounds = stack.bounds;
        let height = 2.0 * (row + gap) + MAIN as f32 * line;
        stack.container(ui, height, |ui, content| {
            let title = Rect::from_origin_size(content.min, Vec2::new(content.width(), row));
            ui.section_title(title, tr("Colors"));
            let scope = Rect::from_origin_size(
                Vec2::new(content.min.x, title.max.y + gap),
                Vec2::new(content.width(), row),
            );
            self.scope(ui, scope);
            let mut y = scope.max.y + gap;
            for index in 0..MAIN {
                let at = Rect::from_min_max(
                    Vec2::new(content.min.x, y),
                    Vec2::new(content.max.x, y + line),
                );
                self.palette_row(ui, at, index, design, bounds);
                y += line;
            }
        });
        let hues = self.palettes.len() - MAIN;
        let mut state = core::mem::replace(&mut self.hues, CollapsiblePanelState::new(false));
        stack.panel(
            ui,
            CollapsiblePanel::new(ID.with(1), &mut state, tr("Hue palettes")),
            |ui, rect| {
                for index in 0..hues {
                    let y = rect.min.y + index as f32 * line;
                    let at = Rect::from_min_max(
                        Vec2::new(rect.min.x, y),
                        Vec2::new(rect.max.x, y + line),
                    );
                    self.palette_row(ui, at, MAIN + index, design, bounds);
                }
                hues as f32 * line
            },
        );
        self.hues = state;
    }

    /// The choice of the themes a change applies to.
    fn scope(&mut self, ui: &mut Ui<'_>, rect: Rect) {
        let label = ui.styles().text.label.clone();
        let secondary = ui.styles().text.secondary;
        let width = (rect.width() * 0.5).min(14.0 * rect.height());
        let text = Rect::from_min_max(rect.min, Vec2::new(rect.max.x - width, rect.max.y));
        ui.label(
            tr("One color makes a palette: its steps keep their contrast."),
            text,
            &label,
            secondary,
            Align::Start,
        );
        let at = Rect::from_min_max(Vec2::new(rect.max.x - width, rect.min.y), rect.max);
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

    /// The row of the palette at `index` in `rect`.
    fn palette_row(
        &mut self,
        ui: &mut Ui<'_>,
        rect: Rect,
        index: usize,
        design: &mut DesignSystem,
        bounds: Rect,
    ) {
        let palette = self.palettes[index];
        let id = ID.with(100 + index as u64 * 4);
        let row = ui.styles().button.height;
        let gap = ui.styles().property_group.row_gap;
        let active = design.active_theme();
        let themes = self.themes(design);
        // The reset dot, beside the name.
        let seeded = themes
            .iter()
            .any(|&theme| palettes::is_seeded(design, theme, &palette));
        let dot = Rect::from_origin_size(rect.min, Vec2::splat(row));
        if seeded
            && ui
                .reset_dot(id.with(1), dot, tr("Back to the theme's own palette"))
                .clicked
        {
            for &theme in &themes {
                if let Err(error) = palettes::reset(design, theme, &palette) {
                    koebi_core::warn!("cannot reset the {} palette: {error}", palette.key);
                }
            }
        }
        let label = ui.styles().text.label.clone();
        let detail = ui.styles().text.detail.clone();
        let (primary, secondary) = (ui.styles().text.primary, ui.styles().text.secondary);
        let names = 12.0 * row;
        let name = Rect::from_origin_size(
            Vec2::new(dot.max.x + gap, rect.min.y),
            Vec2::new(names, row),
        );
        ui.label(palette.name, name, &label, primary, Align::Start);
        let description = Rect::from_origin_size(
            Vec2::new(name.min.x, name.max.y + gap),
            Vec2::new(names, row),
        );
        ui.label(
            palette.description,
            description,
            &detail,
            secondary,
            Align::Start,
        );

        // The source, in a color field.
        let field = Rect::from_origin_size(
            Vec2::new(name.max.x + gap, rect.min.y),
            Vec2::new(3.0 * row, row),
        );
        let mut color = palettes::source(design, active, &palette).unwrap_or(Color::BLACK);
        let own = design
            .view(active, Layers::THEME)
            .and_then(|view| {
                design
                    .find(&palette.source_path())
                    .and_then(|token| view.value(token))
            })
            .and_then(|value| match value {
                koebi_design::value::Value::Color(color) => Some(color),
                _ => None,
            })
            .unwrap_or(color);
        if ColorField::new(id.with(2), &mut color, &mut self.fields[index])
            .dropdown(true)
            .original(own)
            .title(palette.name)
            .show(ui, field, bounds)
            .changed
        {
            for &theme in &themes {
                if let Err(error) = palettes::seed(design, theme, &palette, color) {
                    koebi_core::warn!("cannot make the {} palette: {error}", palette.key);
                }
            }
        }

        // The palette on the theme's background, each step's contrast below it, then samples.
        let samples = 3.0 * 2.0 * row + 2.0 * gap;
        let strip = Rect::from_min_max(
            Vec2::new(field.max.x + 2.0 * gap, rect.min.y),
            Vec2::new(
                (rect.max.x - samples - gap).max(field.max.x + 2.0 * gap),
                rect.min.y + row,
            ),
        );
        let background = design.get(semantic::BACKGROUND_BASE_DEFAULT);
        let steps = steps(design, &palette);
        ui.scene()
            .push_rect(&RectPrimitive::new(strip, background).with_corner_radius(row * 0.2));
        let width = strip.width() / steps.len() as f32;
        for (step, color) in steps.iter().enumerate() {
            let x = strip.min.x + step as f32 * width;
            let swatch = Rect::from_min_max(
                Vec2::new(x + 1.0, strip.min.y + 2.0),
                Vec2::new(x + width - 1.0, strip.max.y - 2.0),
            );
            ui.scene()
                .push_rect(&RectPrimitive::new(swatch, *color).with_corner_radius(2.0));
            let under =
                Rect::from_origin_size(Vec2::new(x, strip.max.y + gap), Vec2::new(width, row));
            ui.label(
                &self.labels[index][step],
                under,
                &detail,
                secondary,
                Align::Center,
            );
        }
        let at = Vec2::new(strip.max.x + gap, rect.min.y);
        samples_of(ui, at, row, gap, design, &palette, &steps);
    }
}

/// The steps of `palette` in the active theme.
fn steps(design: &DesignSystem, palette: &Editable) -> Vec<Color> {
    palette
        .step_paths()
        .iter()
        .filter_map(|path| design.find(path))
        .filter_map(|token| match design.value(token) {
            Some(koebi_design::value::Value::Color(color)) => Some(*color),
            _ => None,
        })
        .collect()
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
    let on = design.get(semantic::TEXT_COLOR_ON_ACCENT_DEFAULT);
    let get = |index: usize| steps.get(index).copied().unwrap_or(Color::BLACK);
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
