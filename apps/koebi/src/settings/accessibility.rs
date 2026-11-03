//! The Accessibility page: the color vision modes, as cards showing the interface each makes.
//!
//! One panel holds the three kinds of modes, each a panel of cards. A card paints, as its mode
//! shows them, real components of the current theme (an accent button, a checked box, a filled
//! bar), the main colors of the interface by name and the colors of a chart: the differences a
//! mode makes are seen where they matter.

use koebi_core::{Rect, Vec2};
use koebi_design::{ColorVision, Deficiency, DesignSystem, Layers, Resolve};
use koebi_render::RectPrimitive;
use koebi_tokens::semantic;
use koebi_ui::{
    Align, BarMapping, Button, Checkbox, CollapsiblePanel, CollapsiblePanelState, NumberField,
    Styles, Tone, Ui, UiState, WidgetId,
};

use super::Stack;

const ID: WidgetId = WidgetId::new("settings-accessibility");

/// The width of a card, at least.
const CARD_WIDTH: f32 = 260.0;
/// How high a card's preview is, in rows: its name and what it does, the components, the main
/// colors with their names, and the chart.
const CARD_ROWS: f32 = 6.5;

/// A color vision mode offered, at full strength.
#[derive(Clone, Copy, Debug)]
struct Mode {
    label: &'static str,
    detail: &'static str,
    vision: ColorVision,
}

const fn simulate(deficiency: Deficiency) -> ColorVision {
    ColorVision::Simulate {
        deficiency,
        severity: 1.0,
    }
}

const fn correct(deficiency: Deficiency) -> ColorVision {
    ColorVision::Correct {
        deficiency,
        strength: 1.0,
    }
}

/// Every mode, in the order of their groups ([`GROUPS`]).
const MODES: [Mode; 8] = [
    Mode {
        label: "Normal vision",
        detail: "Colors as the theme defines them",
        vision: ColorVision::Normal,
    },
    Mode {
        label: "Protanopia",
        detail: "No red cones",
        vision: simulate(Deficiency::Protan),
    },
    Mode {
        label: "Deuteranopia",
        detail: "No green cones",
        vision: simulate(Deficiency::Deutan),
    },
    Mode {
        label: "Tritanopia",
        detail: "No blue cones",
        vision: simulate(Deficiency::Tritan),
    },
    Mode {
        label: "Achromatopsia",
        detail: "No colors",
        vision: ColorVision::SimulateAchromatopsia { severity: 1.0 },
    },
    Mode {
        label: "Protan correction",
        detail: "Separates reds from greens",
        vision: correct(Deficiency::Protan),
    },
    Mode {
        label: "Deutan correction",
        detail: "Separates greens from reds",
        vision: correct(Deficiency::Deutan),
    },
    Mode {
        label: "Tritan correction",
        detail: "Separates blues from yellows",
        vision: correct(Deficiency::Tritan),
    },
];

/// The groups of modes: their titles and the range of [`MODES`] they hold.
const GROUPS: [(&str, core::ops::Range<usize>); 3] = [
    ("Normal vision", 0..1),
    ("See as someone with a color deficiency", 1..5),
    ("Correct a color deficiency", 5..8),
];

/// The main colors a card names: the button tones of the interface.
const MAIN_COLORS: [(&str, Tone); 5] = [
    ("Accent", Tone::ACCENT),
    ("Positive", Tone::POSITIVE),
    ("Negative", Tone::NEGATIVE),
    ("Notice", Tone::NOTICE),
    ("Info", Tone::INFO),
];

/// `true` when `current` is `mode`, at any strength.
fn is_mode(current: ColorVision, mode: ColorVision) -> bool {
    match (current, mode) {
        (
            ColorVision::Simulate { deficiency: a, .. },
            ColorVision::Simulate { deficiency: b, .. },
        )
        | (
            ColorVision::Correct { deficiency: a, .. },
            ColorVision::Correct { deficiency: b, .. },
        ) => a == b,
        (ColorVision::SimulateAchromatopsia { .. }, ColorVision::SimulateAchromatopsia { .. })
        | (ColorVision::Normal, ColorVision::Normal) => true,
        _ => false,
    }
}

/// How strong `vision` is, from 0 to 1, for the modes that have a strength.
const fn strength_of(vision: ColorVision) -> Option<f32> {
    match vision {
        ColorVision::Normal | ColorVision::Custom(_) => None,
        ColorVision::Simulate { severity, .. }
        | ColorVision::SimulateAchromatopsia { severity } => Some(severity),
        ColorVision::Correct { strength, .. } => Some(strength),
    }
}

/// `vision` at `strength`.
const fn with_strength(vision: ColorVision, strength: f32) -> ColorVision {
    match vision {
        ColorVision::Simulate { deficiency, .. } => ColorVision::Simulate {
            deficiency,
            severity: strength,
        },
        ColorVision::SimulateAchromatopsia { .. } => {
            ColorVision::SimulateAchromatopsia { severity: strength }
        }
        ColorVision::Correct { deficiency, .. } => ColorVision::Correct {
            deficiency,
            strength,
        },
        ColorVision::Normal | ColorVision::Custom(_) => vision,
    }
}

/// The vision a card of `mode` shows when `current` is set: the setting itself on the card of
/// its mode (at its strength), the mode at full strength on the others.
fn shown_vision(mode: ColorVision, current: ColorVision) -> ColorVision {
    if is_mode(current, mode) {
        current
    } else {
        mode
    }
}

/// The styles of the current theme as each mode shows them, resolved again only when the
/// design system changes.
#[derive(Debug)]
struct Previews {
    /// The revision of the design system the styles were resolved from.
    revision: Option<u64>,
    /// One per mode of [`MODES`].
    styles: Vec<Styles>,
    /// The state of the interface the previews draw with.
    state: UiState,
}

impl Previews {
    fn new() -> Self {
        Self {
            revision: None,
            styles: Vec::new(),
            state: UiState::new(),
        }
    }

    /// Resolves the styles again if `design` changed since.
    fn refresh(&mut self, design: &DesignSystem) {
        if self.revision == Some(design.revision()) && self.styles.len() == MODES.len() {
            return;
        }
        let Some(view) = design.view(design.active_theme(), Layers::ALL) else {
            return;
        };
        let current = design.color_vision();
        self.styles.clear();
        self.styles.extend(
            MODES
                .iter()
                .map(|mode| Styles::resolve(&view.seen_as(shown_vision(mode.vision, current)))),
        );
        self.revision = Some(design.revision());
    }
}

/// The Accessibility page's state.
#[derive(Debug)]
pub(super) struct AccessibilityPage {
    /// The panel holding the groups, and each group.
    panel: CollapsiblePanelState,
    groups: [CollapsiblePanelState; 3],
    previews: Previews,
}

impl AccessibilityPage {
    pub(super) fn new() -> Self {
        Self {
            panel: CollapsiblePanelState::new(true),
            groups: [CollapsiblePanelState::new(true); 3],
            previews: Previews::new(),
        }
    }

    pub(super) fn show(&mut self, ui: &mut Ui<'_>, stack: &mut Stack, design: &mut DesignSystem) {
        let Self {
            panel,
            groups,
            previews,
        } = self;
        previews.refresh(design);
        let container = CollapsiblePanel::new(ID, panel, "Color vision");
        stack.panel(ui, container, |ui, rect| {
            let gap = ui.styles().collapsible_panel.gap;
            let mut y = rect.min.y;
            for (index, (state, (title, modes))) in groups.iter_mut().zip(GROUPS).enumerate() {
                let id = ID.with(index as u64 + 1);
                let at = Rect::from_min_max(Vec2::new(rect.min.x, y), rect.max);
                let shown = CollapsiblePanel::new(id, state, title).show(ui, at, |ui, rect| {
                    mode_cards(ui, rect, id, design, previews, modes.clone())
                });
                y += shown.height + if index + 1 < GROUPS.len() { gap } else { 0.0 };
            }
            y - rect.min.y
        });
    }
}

/// How many cards go in a row of `width`, and how wide they are.
fn grid(width: f32, gap: f32, count: usize) -> (usize, f32) {
    let fitting = ((width + gap) / (CARD_WIDTH + gap)).floor();
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a few cards, a positive whole number"
    )]
    let columns = (fitting.max(1.0) as usize).min(count.max(1));
    let card = ((width - gap * (columns - 1) as f32) / columns as f32).clamp(0.0, CARD_WIDTH * 1.5);
    (columns, card)
}

/// The cards of the modes at `modes` of [`MODES`] in `rect`, and how strong the chosen one is.
/// Returns the height used.
fn mode_cards(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    design: &mut DesignSystem,
    previews: &mut Previews,
    modes: core::ops::Range<usize>,
) -> f32 {
    let padding = ui.styles().panel.padding;
    let gap = design.get(semantic::SPACING_EDITOR_INSET);
    let row = design.get(semantic::SIZE_CONTROL_HEIGHT);
    let left = rect.min.x + padding;
    let width = rect.width() - 2.0 * padding;
    let count = modes.len();
    let (columns, card) = grid(width, gap, count);
    let height = (row * CARD_ROWS).round();
    let current = design.color_vision();
    let mut chosen = None;
    for (index, mode_index) in modes.clone().enumerate() {
        let mode = &MODES[mode_index];
        let (column, line) = (index % columns, index / columns);
        let at = Rect::from_origin_size(
            Vec2::new(
                left + (card + gap) * column as f32,
                rect.min.y + padding + (height + gap) * line as f32,
            ),
            Vec2::new(card, height),
        );
        let selected = is_mode(current, mode.vision);
        let Some(styles) = previews.styles.get(mode_index) else {
            continue;
        };
        let clicked = mode_card(
            ui,
            id.with(index as u64 + 1),
            at,
            mode,
            styles,
            &mut previews.state,
            selected,
        );
        if clicked {
            chosen = Some(mode.vision);
        }
    }
    let lines = count.div_ceil(columns) as f32;
    let mut used = padding + lines * height + (lines - 1.0).max(0.0) * gap;
    // How strong the mode chosen in this group is.
    if MODES[modes]
        .iter()
        .any(|mode| is_mode(current, mode.vision))
        && let Some(strength) = strength_of(current)
    {
        let label = Rect::from_origin_size(
            Vec2::new(left, rect.min.y + used + gap),
            Vec2::new(width * 0.3, row),
        );
        let font = ui.styles().text.label.clone();
        let color = ui.styles().text.primary;
        ui.label("Strength", label, &font, color, Align::Start);
        let field = Rect::from_min_max(
            Vec2::new(label.max.x, label.min.y),
            Vec2::new(left + width, label.max.y),
        );
        let mut percent = f64::from(strength) * 100.0;
        if NumberField::new(id.with(100), &mut percent)
            .range(0.0, 100.0)
            .decimals(0)
            .bar(BarMapping::Linear)
            .show(ui, field)
            .changed
        {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "a share from 0 to 1, which f32 holds"
            )]
            let strength = (percent / 100.0) as f32;
            chosen = Some(with_strength(current, strength));
        }
        used += gap + row;
    }
    if let Some(vision) = chosen
        && let Err(error) = design.set_color_vision(vision)
    {
        koebi_core::warn!("cannot change the color vision setting: {error}");
    }
    used + padding
}

/// A card: the interface as `mode` shows it, painted with `styles`. Returns whether it was
/// clicked.
fn mode_card(
    ui: &mut Ui<'_>,
    id: WidgetId,
    rect: Rect,
    mode: &Mode,
    styles: &Styles,
    state: &mut UiState,
    selected: bool,
) -> bool {
    let card = ui.card(id, rect, selected);
    if !card.content.is_empty() {
        ui.preview(styles, state, |ui| {
            mode_preview(ui, card.content, card.corner_radius, mode);
        });
    }
    card.response.clicked
}

/// What a card shows of the interface, drawn in `rect` with the preview's styles: the mode's
/// name and what it does, real components, the main colors by name and a chart.
fn mode_preview(ui: &mut Ui<'_>, rect: Rect, corner_radius: f32, mode: &Mode) {
    let styles = ui.styles();
    let text = styles.text.clone();
    let background = styles.panel.backgrounds[0];
    let row = styles.button.height;
    let inset = styles.panel.padding;
    ui.scene()
        .push_rect(&RectPrimitive::new(rect, background).with_corner_radius(corner_radius));
    let inner = rect.inset(inset);
    let (left, width) = (inner.min.x, inner.width());
    let line =
        |y: f32, height: f32| Rect::from_origin_size(Vec2::new(left, y), Vec2::new(width, height));

    // Its name and what it does.
    let mut y = inner.min.y;
    ui.label(
        mode.label,
        line(y, row),
        &text.label,
        text.primary,
        Align::Start,
    );
    y += row;
    let detail = (row * 0.8).round();
    ui.label(
        mode.detail,
        line(y, detail),
        &text.detail,
        text.secondary,
        Align::Start,
    );
    y += detail + inset * 0.5;

    // Components of the theme: an accent button, a checked box, a filled bar.
    let id = WidgetId::new("color-vision-preview");
    let button = Button::new(id.with(1)).label("Apply").tone(Tone::ACCENT);
    let button_width = button.size(ui).x.min(width * 0.3);
    button.show(
        ui,
        Rect::from_origin_size(Vec2::new(left, y), Vec2::new(button_width, row)),
    );
    let mut checked = true;
    let check = Rect::from_origin_size(
        Vec2::new(left + button_width + inset, y),
        Vec2::new(width * 0.25, row),
    );
    Checkbox::new(id.with(2), &mut checked)
        .label("On")
        .show(ui, check);
    let mut share = 0.6;
    let bar = Rect::from_min_max(
        Vec2::new(check.max.x + inset, y),
        Vec2::new(inner.max.x, y + row),
    );
    if bar.width() > row {
        NumberField::new(id.with(3), &mut share)
            .range(0.0, 1.0)
            .decimals(2)
            .bar(BarMapping::Linear)
            .show(ui, bar);
    }
    y += row + inset;

    // The main colors, by name.
    let styles = ui.styles();
    let cell = width / MAIN_COLORS.len() as f32;
    let swatch = (row * 0.6).round();
    for (index, (name, tone)) in MAIN_COLORS.into_iter().enumerate() {
        let x = left + cell * index as f32;
        let color = styles.button.colors(tone).background.normal;
        let chip = Rect::from_origin_size(
            Vec2::new(x + 2.0, y),
            Vec2::new((cell - 4.0).max(0.0), swatch),
        );
        ui.scene().push_rect(
            &RectPrimitive::new(chip, color).with_corner_radius(styles.button.corner_radius),
        );
        let label = Rect::from_origin_size(Vec2::new(x, y + swatch), Vec2::new(cell, detail));
        ui.label(name, label, &text.detail, text.secondary, Align::Center);
    }
    y += swatch + detail + inset * 0.5;

    // A chart: the colors data takes, each bar as high as its share of the tallest.
    let bars = [
        (semantic::DATA_VIZ_CATEGORICAL_SHORT_1, 0.55),
        (semantic::DATA_VIZ_CATEGORICAL_SHORT_2, 0.85),
        (semantic::DATA_VIZ_CATEGORICAL_SHORT_3, 0.4),
        (semantic::DATA_VIZ_CATEGORICAL_SHORT_4, 0.7),
        (semantic::DATA_VIZ_CATEGORICAL_SHORT_5, 1.0),
        (semantic::DATA_VIZ_CATEGORICAL_SHORT_6, 0.6),
    ];
    let chart = Rect::from_min_max(Vec2::new(left, y), Vec2::new(inner.max.x, inner.max.y));
    if chart.height() > 0.0 {
        let bar = chart.width() / bars.len() as f32;
        for (index, (token, share)) in bars.into_iter().enumerate() {
            let color = ui.styles().tokens.get(token);
            let height = (chart.height() * share).round();
            let column = Rect::from_min_max(
                Vec2::new(chart.min.x + bar * index as f32 + 2.0, chart.max.y - height),
                Vec2::new(chart.min.x + bar * (index + 1) as f32 - 2.0, chart.max.y),
            );
            ui.scene()
                .push_rect(&RectPrimitive::new(column, color).with_corner_radius(2.0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_are_recognized_at_any_strength() {
        let weak = with_strength(simulate(Deficiency::Protan), 0.4);
        assert!(is_mode(weak, simulate(Deficiency::Protan)));
        assert!(!is_mode(weak, simulate(Deficiency::Deutan)));
        assert!(!is_mode(weak, correct(Deficiency::Protan)));
        assert!(is_mode(ColorVision::Normal, ColorVision::Normal));
        assert_eq!(strength_of(ColorVision::Normal), None);
        assert_eq!(strength_of(weak), Some(0.4));
        assert_eq!(with_strength(ColorVision::Normal, 0.5), ColorVision::Normal);
    }

    #[test]
    fn every_mode_of_the_command_line_has_a_card() {
        for (_, mode) in crate::design_options::COLOR_VISION_MODES {
            assert!(
                MODES.iter().any(|card| is_mode(card.vision, mode)),
                "{mode:?}"
            );
        }
    }

    #[test]
    fn the_groups_hold_every_mode_once() {
        let mut held: Vec<usize> = GROUPS.iter().flat_map(|(_, range)| range.clone()).collect();
        held.dedup();
        assert_eq!(held, (0..MODES.len()).collect::<Vec<_>>());
    }

    #[test]
    fn the_card_of_the_mode_set_shows_it_at_its_strength() {
        let weak = with_strength(simulate(Deficiency::Deutan), 0.3);
        assert_eq!(shown_vision(simulate(Deficiency::Deutan), weak), weak);
        assert_eq!(
            shown_vision(simulate(Deficiency::Protan), weak),
            simulate(Deficiency::Protan)
        );
    }

    #[test]
    fn previews_paint_each_mode_and_follow_the_design() {
        let mut design = DesignSystem::new(&koebi_tokens::KOEBI).unwrap();
        let mut previews = Previews::new();
        previews.refresh(&design);
        assert_eq!(previews.styles.len(), MODES.len());
        let accent = |previews: &Previews, index: usize| {
            previews.styles[index]
                .button
                .colors(Tone::ACCENT)
                .background
                .normal
        };
        // Normal vision shows the theme's colors; achromatopsia shows them gray.
        assert_eq!(
            accent(&previews, 0),
            design.get(koebi_tokens::component::BUTTON_ACCENT_BACKGROUND_COLOR_DEFAULT)
        );
        let gray = accent(&previews, 4);
        assert!((gray.r - gray.g).abs() < 2e-3 && (gray.g - gray.b).abs() < 2e-3);
        // Unchanged, nothing is resolved again; a change of the design is followed.
        let revision = previews.revision;
        previews.refresh(&design);
        assert_eq!(previews.revision, revision);
        design
            .set_color_vision(simulate(Deficiency::Tritan))
            .unwrap();
        previews.refresh(&design);
        assert_ne!(previews.revision, revision);
        // The normal card still shows the theme's own colors.
        assert_eq!(
            accent(&previews, 0),
            design
                .view(design.active_theme(), Layers::DEFINITIONS)
                .unwrap()
                .get(koebi_tokens::component::BUTTON_ACCENT_BACKGROUND_COLOR_DEFAULT)
        );
    }
}
