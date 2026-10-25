//! The design system gallery: Koebi's interface until it has editors.
//!
//! - **Themes**: one card per theme, painted with that theme's tokens; a click activates it.
//! - **Components**: every component of `koebi-ui`, interactive.
//! - **Token editor** ([`editor`]): sliders and fields that edit tokens in every theme or in
//!   the active theme; every component that depends on them follows in the same frame.
//! - **Color vision** ([`editor`]): the simulation and correction modes, each previewed on
//!   four chart colors.
//! - **Grid**: hundreds of components, to show that styling costs nothing per component.
//!
//! Components have no labels until Koebi renders text: previews beside the editor's controls
//! show what each one edits.

mod editor;

use koebi_core::{Color, Oklch, Rect, Vec2};
use koebi_design::kind::Color as ColorKind;
use koebi_design::tier::Semantic;
use koebi_design::{DesignSystem, Scope, Token};
use koebi_tokens::semantic;
use koebi_ui::{Icon, PanelLevel, ThemePreview, Ui, WidgetId};

/// The twelve chart colors, shown as swatches and in the grid.
const PALETTE: [Token<ColorKind, Semantic>; 12] = [
    semantic::DATA_VIZ_CATEGORICAL_SHORT_1,
    semantic::DATA_VIZ_CATEGORICAL_SHORT_2,
    semantic::DATA_VIZ_CATEGORICAL_SHORT_3,
    semantic::DATA_VIZ_CATEGORICAL_SHORT_4,
    semantic::DATA_VIZ_CATEGORICAL_SHORT_5,
    semantic::DATA_VIZ_CATEGORICAL_SHORT_6,
    semantic::DATA_VIZ_CATEGORICAL_EXTENDED_7,
    semantic::DATA_VIZ_CATEGORICAL_EXTENDED_8,
    semantic::DATA_VIZ_CATEGORICAL_EXTENDED_9,
    semantic::DATA_VIZ_CATEGORICAL_EXTENDED_10,
    semantic::DATA_VIZ_CATEGORICAL_EXTENDED_11,
    semantic::DATA_VIZ_CATEGORICAL_EXTENDED_12,
];

/// The grid never holds more components than this.
const GRID_LIMIT: usize = 2000;

/// Below this width, panels stack in one column.
const TWO_COLUMNS: f32 = 900.0;

/// Spacing and sizes of the gallery, from tokens: the layout follows the design system too.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Metrics {
    /// Space around and between panels.
    gap: f32,
    /// Height of a row of controls.
    row: f32,
}

impl Metrics {
    fn resolve(design: &DesignSystem) -> Self {
        Self {
            gap: design.get(semantic::SPACING_EDITOR_INSET).max(0.0),
            row: design.get(semantic::SIZE_CONTROL_HEIGHT).max(8.0),
        }
    }

    /// Space between rows inside a panel.
    fn row_gap(self) -> f32 {
        (self.gap * 0.5).round()
    }

    /// The height of a panel holding `rows` rows of controls.
    fn panel_height(self, ui: &Ui<'_>, rows: u16) -> f32 {
        let panel = &ui.styles().panel;
        let rows = f32::from(rows);
        rows * self.row
            + (rows - 1.0).max(0.0) * self.row_gap()
            + 2.0 * (panel.padding + panel.border_width)
    }
}

/// Takes a band of `height` from the top of `rest`, and the gap below it.
fn cut_top(rest: &mut Rect, height: f32, gap: f32) -> Rect {
    let height = height.clamp(0.0, rest.height().max(0.0));
    let band = Rect::from_origin_size(rest.min, Vec2::new(rest.width(), height));
    rest.min.y = (rest.min.y + height + gap).min(rest.max.y);
    band
}

/// Takes a band of `width` from the left of `rest`, and the gap after it.
fn cut_left(rest: &mut Rect, width: f32, gap: f32) -> Rect {
    let width = width.clamp(0.0, rest.width().max(0.0));
    let band = Rect::from_origin_size(rest.min, Vec2::new(width, rest.height()));
    rest.min.x = (rest.min.x + width + gap).min(rest.max.x);
    band
}

/// How many cells of `cell` fit in `length`, with a `gap` between cells.
fn fit(length: f32, cell: f32, gap: f32) -> usize {
    let count = ((length + gap) / (cell + gap)).floor();
    if count.is_finite() && count > 0.0 {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a positive whole number of cells, a few hundred at most"
        )]
        let count = count.min(GRID_LIMIT as f32) as usize;
        count
    } else {
        0
    }
}

/// The values of the component sheet: the application owns them, components edit them.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Sheet {
    toggles: [bool; 2],
    checkboxes: [bool; 2],
    slider: f32,
    drag: f32,
    swatch: usize,
}

impl Default for Sheet {
    fn default() -> Self {
        Self {
            toggles: [false, true],
            checkboxes: [false, true],
            slider: 0.35,
            drag: 6.0,
            swatch: 0,
        }
    }
}

/// The gallery's state between frames.
#[derive(Debug)]
pub(crate) struct Gallery {
    /// Where the token editor writes: every theme ([`Scope::Global`]) or the active theme.
    scope: Scope,
    /// The accent being edited, kept in OKLCH so that the hue survives grays.
    accent: Oklch,
    /// The accent color `accent` was last read from or written to.
    accent_source: Option<Color>,
    sheet: Sheet,
    /// States of the grid's checkboxes, toggles and swatches.
    grid: Vec<bool>,
}

impl Gallery {
    pub(crate) fn new() -> Self {
        Self {
            scope: Scope::Global,
            accent: Oklch::new(0.0, 0.0, 0.0),
            accent_source: None,
            sheet: Sheet::default(),
            grid: Vec::new(),
        }
    }

    /// Draws the gallery in `area` and applies the changes made with it to `design`.
    pub(crate) fn show(&mut self, ui: &mut Ui<'_>, design: &mut DesignSystem, area: Rect) {
        let metrics = Metrics::resolve(design);
        let mut rest = area.inset(metrics.gap);
        if rest.is_empty() {
            return;
        }
        let cards = cut_top(&mut rest, (metrics.row * 4.5).round(), metrics.gap);
        theme_cards(ui, design, cards, metrics.gap);

        let sheet_height = metrics.panel_height(ui, 5);
        let editor_height = metrics.panel_height(ui, editor::ROWS);
        let vision_height = metrics.panel_height(ui, 1);
        if rest.width() >= TWO_COLUMNS {
            let mut top = cut_top(&mut rest, sheet_height.max(editor_height), metrics.gap);
            let half = ((top.width() - metrics.gap) * 0.5).round();
            let left = cut_left(&mut top, half, metrics.gap);
            self.components(ui, design, left, metrics);
            editor::tokens(self, ui, design, top, metrics);
        } else {
            let sheet = cut_top(&mut rest, sheet_height, metrics.gap);
            self.components(ui, design, sheet, metrics);
            let tokens = cut_top(&mut rest, editor_height, metrics.gap);
            editor::tokens(self, ui, design, tokens, metrics);
        }
        let vision = cut_top(&mut rest, vision_height, metrics.gap);
        editor::color_vision(ui, design, vision, metrics);
        self.grid(ui, design, rest, metrics);
    }

    /// Every component, interactive.
    fn components(&mut self, ui: &mut Ui<'_>, design: &DesignSystem, rect: Rect, metrics: Metrics) {
        const ID: WidgetId = WidgetId::new("components");
        let mut content = ui.panel(rect, PanelLevel::First);
        let (row, gap) = (metrics.row, metrics.row_gap());
        let sheet = &mut self.sheet;

        let mut line = cut_top(&mut content, row, gap);
        ui.button(ID.with(0), cut_left(&mut line, 4.0 * row, gap), None);
        for (index, icon) in [Icon::Plus, Icon::Minus, Icon::Close]
            .into_iter()
            .enumerate()
        {
            ui.button(
                ID.with(1 + index as u64),
                cut_left(&mut line, row, gap),
                Some(icon),
            );
        }
        let icons = [Icon::ChevronDown, Icon::Check];
        for (index, (on, icon)) in sheet.toggles.iter_mut().zip(icons).enumerate() {
            ui.toggle(
                ID.with(4 + index as u64),
                cut_left(&mut line, row, gap),
                on,
                Some(icon),
            );
        }
        for (index, checked) in sheet.checkboxes.iter_mut().enumerate() {
            ui.checkbox(
                ID.with(6 + index as u64),
                cut_left(&mut line, row, gap),
                checked,
            );
        }

        let mut line = cut_top(&mut content, row, gap);
        let half = ((line.width() - gap) * 0.5).round();
        ui.slider(
            ID.with(8),
            cut_left(&mut line, half, gap),
            &mut sheet.slider,
            0.0..=1.0,
        );
        ui.drag_value(ID.with(9), line, &mut sheet.drag, 0.0..=10.0, 0.5);

        ui.separator(cut_top(&mut content, row, gap));

        let mut line = cut_top(&mut content, row, gap);
        for (index, token) in PALETTE.into_iter().enumerate() {
            let cell = cut_left(&mut line, row, gap);
            let selected = sheet.swatch == index;
            if ui
                .swatch(
                    ID.with(10 + index as u64),
                    cell,
                    design.get(token),
                    selected,
                )
                .clicked
            {
                sheet.swatch = index;
            }
        }

        // The three panel levels side by side.
        let mut line = cut_top(&mut content, row, gap);
        let third = ((line.width() - 2.0 * gap) / 3.0).round();
        for level in [PanelLevel::First, PanelLevel::Second, PanelLevel::Third] {
            ui.panel(cut_left(&mut line, third, gap), level);
        }
    }

    /// As many components as fit, in a grid: checkboxes, toggles, buttons and swatches.
    fn grid(&mut self, ui: &mut Ui<'_>, design: &DesignSystem, rect: Rect, metrics: Metrics) {
        const ID: WidgetId = WidgetId::new("grid");
        if rect.height() < metrics.row * 2.0 {
            return;
        }
        let content = ui.panel(rect, PanelLevel::First);
        let (cell, gap) = (metrics.row, metrics.row_gap());
        let columns = fit(content.width(), cell, gap);
        let rows = fit(content.height(), cell, gap);
        let count = (columns * rows).min(GRID_LIMIT);
        if self.grid.len() < count {
            self.grid.resize(count, false);
        }
        for (index, state) in self.grid.iter_mut().take(count).enumerate() {
            let (column, row) = (index % columns, index / columns);
            let origin =
                content.min + Vec2::new(column as f32 * (cell + gap), row as f32 * (cell + gap));
            let rect = Rect::from_origin_size(origin, Vec2::splat(cell));
            let id = ID.with(index as u64);
            match index % 4 {
                0 => {
                    ui.checkbox(id, rect, state);
                }
                1 => {
                    ui.toggle(id, rect, state, Some(Icon::Check));
                }
                2 => {
                    let icon = if *state { Icon::Minus } else { Icon::Plus };
                    if ui.button(id, rect, Some(icon)).clicked {
                        *state = !*state;
                    }
                }
                _ => {
                    let color = design.get(PALETTE[index / 4 % PALETTE.len()]);
                    if ui.swatch(id, rect, color, *state).clicked {
                        *state = !*state;
                    }
                }
            }
        }
    }
}

/// One card per theme, painted with that theme's tokens; a click activates the theme.
fn theme_cards(ui: &mut Ui<'_>, design: &mut DesignSystem, mut rest: Rect, gap: f32) {
    let count = design.themes().count();
    if count == 0 || rest.is_empty() {
        return;
    }
    let height = rest.height();
    let fitting = (rest.width() - gap * (count - 1) as f32) / count as f32;
    let width = (height * 1.6).min(fitting).round().max(0.0);
    let mut chosen = None;
    for (theme, info) in design.themes() {
        let Some(preview) = ThemePreview::of(design, theme) else {
            continue;
        };
        let card = cut_left(&mut rest, width, gap);
        let selected = theme == design.active_theme();
        if ui
            .theme_card(WidgetId::new(info.key()), card, &preview, selected)
            .clicked
        {
            chosen = Some(theme);
        }
    }
    if let Some(theme) = chosen
        && let Err(error) = design.set_active_theme(theme)
    {
        koebi_core::warn!("cannot activate the theme: {error}");
    }
}

#[cfg(test)]
mod tests {
    use koebi_render::Scene;
    use koebi_tokens::KOEBI;
    use koebi_ui::{Input, StyleCache, UiState};

    use super::*;

    fn design() -> DesignSystem {
        DesignSystem::new(&KOEBI).expect("Koebi's schema is valid")
    }

    /// One frame of the gallery in a window of `size`, with `input`.
    fn frame(gallery: &mut Gallery, design: &mut DesignSystem, input: &Input, size: Vec2) -> Scene {
        let styles = StyleCache::new(design);
        let mut scene = Scene::new();
        let mut state = UiState::new();
        let mut ui = Ui::new(&mut scene, styles.styles(), input, &mut state);
        gallery.show(&mut ui, design, Rect::from_origin_size(Vec2::ZERO, size));
        drop(ui);
        scene
    }

    #[test]
    fn layout_helpers_cut_bands_and_fit_cells() {
        let mut rest = Rect::from_origin_size(Vec2::ZERO, Vec2::new(100.0, 50.0));
        let top = cut_top(&mut rest, 10.0, 4.0);
        assert_eq!((top.height(), rest.min.y), (10.0, 14.0));
        let left = cut_left(&mut rest, 200.0, 4.0);
        assert_eq!((left.width(), rest.width()), (100.0, 0.0));
        assert_eq!(fit(100.0, 20.0, 4.0), 4);
        assert_eq!(fit(10.0, 20.0, 4.0), 0);
        assert_eq!(fit(f32::NAN, 20.0, 4.0), 0);
    }

    #[test]
    fn the_gallery_draws_hundreds_of_components_without_changing_the_design() {
        let mut design = design();
        let mut gallery = Gallery::new();
        let revision = design.revision();
        let scene = frame(
            &mut gallery,
            &mut design,
            &Input::new(),
            Vec2::new(1280.0, 800.0),
        );
        assert!(
            gallery.grid.len() > 300,
            "{} grid cells",
            gallery.grid.len()
        );
        assert!(
            scene.rect_count() > 600,
            "{} rectangles",
            scene.rect_count()
        );
        assert_eq!(design.revision(), revision, "drawing alone changes nothing");
        // Narrow windows stack the panels.
        frame(
            &mut gallery,
            &mut design,
            &Input::new(),
            Vec2::new(400.0, 900.0),
        );
        frame(
            &mut gallery,
            &mut design,
            &Input::new(),
            Vec2::new(20.0, 20.0),
        );
    }

    #[test]
    fn a_click_on_a_theme_card_activates_the_theme() {
        let mut design = design();
        let mut gallery = Gallery::new();
        // The second card (light) starts after the first card and the gap.
        let metrics = Metrics::resolve(&design);
        let height = (metrics.row * 4.5).round();
        let x = metrics.gap + height * 1.6 + metrics.gap + 20.0;
        let at = Vec2::new(x, metrics.gap + 20.0);
        let mut input = Input::new();
        input.primary_button(true, at);
        input.primary_button(false, at);
        frame(&mut gallery, &mut design, &input, Vec2::new(1280.0, 800.0));
        let active = design.theme(design.active_theme()).expect("live theme");
        assert_eq!(active.key(), "light");
    }
}
