//! The Accessibility page: the color vision modes, as cards showing what each does to colors.

use koebi_core::{Rect, Vec2};
use koebi_design::{ColorVision, Deficiency, DesignSystem};
use koebi_render::RectPrimitive;
use koebi_tokens::semantic;
use koebi_ui::{
    Align, BarMapping, Button, CollapsiblePanel, CollapsiblePanelState, NumberField, Ui, WidgetId,
};

use super::Stack;

const ID: WidgetId = WidgetId::new("settings-accessibility");

/// The width of a card, at least.
const CARD_WIDTH: f32 = 200.0;
/// How high a card is, in rows.
const CARD_ROWS: f32 = 3.0;

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

const VISION: [Mode; 1] = [Mode {
    label: "Normal vision",
    detail: "Colors as the theme defines them",
    vision: ColorVision::Normal,
}];

const SIMULATIONS: [Mode; 4] = [
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
];

const CORRECTIONS: [Mode; 3] = [
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

/// The Accessibility page's state.
#[derive(Debug)]
pub(super) struct AccessibilityPage {
    panels: [CollapsiblePanelState; 3],
}

impl AccessibilityPage {
    pub(super) fn new() -> Self {
        Self {
            panels: [CollapsiblePanelState::new(true); 3],
        }
    }

    pub(super) fn show(&mut self, ui: &mut Ui<'_>, stack: &mut Stack, design: &mut DesignSystem) {
        let [vision, simulations, corrections] = &mut self.panels;
        let groups: [(&mut CollapsiblePanelState, &str, &[Mode]); 3] = [
            (vision, "Color vision", &VISION),
            (
                simulations,
                "See as someone with a color deficiency",
                &SIMULATIONS,
            ),
            (corrections, "Correct a color deficiency", &CORRECTIONS),
        ];
        for (index, (panel, title, modes)) in groups.into_iter().enumerate() {
            let id = ID.with(index as u64);
            let group = CollapsiblePanel::new(id, panel, title);
            stack.panel(ui, group, |ui, rect| {
                mode_cards(ui, rect, id, design, modes)
            });
        }
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

/// The cards of `modes` in `rect`, and how strong the chosen one is. Returns the height used.
fn mode_cards(
    ui: &mut Ui<'_>,
    rect: Rect,
    id: WidgetId,
    design: &mut DesignSystem,
    modes: &[Mode],
) -> f32 {
    let padding = ui.styles().panel.padding;
    let gap = design.get(semantic::SPACING_EDITOR_INSET);
    let row = design.get(semantic::SIZE_CONTROL_HEIGHT);
    let left = rect.min.x + padding;
    let width = rect.width() - 2.0 * padding;
    let (columns, card) = grid(width, gap, modes.len());
    let height = row * CARD_ROWS;
    let current = design.color_vision();
    let mut chosen = None;
    for (index, mode) in modes.iter().enumerate() {
        let (column, line) = (index % columns, index / columns);
        let at = Rect::from_origin_size(
            Vec2::new(
                left + (card + gap) * column as f32,
                rect.min.y + (height + gap) * line as f32,
            ),
            Vec2::new(card, height),
        );
        let selected = is_mode(current, mode.vision);
        if mode_card(ui, id.with(index as u64 + 1), at, design, mode, selected) {
            chosen = Some(mode.vision);
        }
    }
    let lines = modes.len().div_ceil(columns) as f32;
    let mut used = lines * height + (lines - 1.0).max(0.0) * gap;
    // How strong the mode chosen in this group is.
    if modes.iter().any(|mode| is_mode(current, mode.vision))
        && let Some(strength) = strength_of(current)
    {
        let label = Rect::from_origin_size(
            Vec2::new(left, rect.min.y + used + gap),
            Vec2::new(width * 0.3, row),
        );
        let font = ui.styles().text.label.clone();
        let color = design.get(semantic::TEXT_COLOR_PRIMARY_DEFAULT);
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
    used
}

/// A card: the colors of a chart as the mode shows them, its name and what it does. Returns
/// whether it was clicked.
fn mode_card(
    ui: &mut Ui<'_>,
    id: WidgetId,
    rect: Rect,
    design: &DesignSystem,
    mode: &Mode,
    selected: bool,
) -> bool {
    let clicked = Button::new(id).selected(selected).show(ui, rect).clicked;
    let row = design.get(semantic::SIZE_CONTROL_HEIGHT);
    let inset = (row * 0.25).round();
    let inner = rect.inset(inset);
    let strip = Rect::from_min_max(inner.min, Vec2::new(inner.max.x, inner.min.y + row * 0.75));
    let matrix = mode.vision.matrix();
    let samples = [
        semantic::DATA_VIZ_CATEGORICAL_SHORT_1,
        semantic::DATA_VIZ_CATEGORICAL_SHORT_2,
        semantic::DATA_VIZ_CATEGORICAL_SHORT_3,
        semantic::DATA_VIZ_CATEGORICAL_SHORT_4,
        semantic::DATA_VIZ_CATEGORICAL_SHORT_5,
        semantic::DATA_VIZ_CATEGORICAL_SHORT_6,
    ];
    let cell = strip.width() / samples.len() as f32;
    let view = design.view(
        design.active_theme(),
        koebi_design::Layers::THEME.union(koebi_design::Layers::OVERRIDES),
    );
    for (index, sample) in samples.into_iter().enumerate() {
        let min = strip.min + Vec2::new(cell * index as f32, 0.0);
        let color = view
            .as_ref()
            .map_or_else(|| design.get(sample), |view| view.get(sample));
        ui.scene().push_rect(&RectPrimitive::new(
            Rect::from_origin_size(min, Vec2::new(cell, strip.height())),
            matrix.apply(color),
        ));
    }
    let font = ui.styles().text.label.clone();
    let detail = ui.styles().text.detail.clone();
    let primary = design.get(semantic::TEXT_COLOR_PRIMARY_DEFAULT);
    let secondary = design.get(semantic::TEXT_COLOR_SECONDARY_DEFAULT);
    let name = Rect::from_origin_size(
        Vec2::new(inner.min.x, strip.max.y),
        Vec2::new(inner.width(), row),
    );
    ui.label(mode.label, name, &font, primary, Align::Start);
    let text = Rect::from_min_max(Vec2::new(inner.min.x, name.max.y), inner.max);
    ui.label(mode.detail, text, &detail, secondary, Align::Start);
    clicked
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
        let cards: Vec<ColorVision> = VISION
            .iter()
            .chain(&SIMULATIONS)
            .chain(&CORRECTIONS)
            .map(|mode| mode.vision)
            .collect();
        for (_, mode) in crate::design_options::COLOR_VISION_MODES {
            assert!(cards.iter().any(|card| is_mode(*card, mode)), "{mode:?}");
        }
    }
}
