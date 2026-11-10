//! The Accessibility page: the color vision modes, as cards showing the interface each makes.
//!
//! One container holds the three kinds of modes, a section each, which never fold. A card
//! paints the current theme's contrasts and colors as its mode shows them (`Ui::vision_card`):
//! text on the surfaces it lies on, the colors of states with their labels, and the colors of
//! charts side by side, where what a mode confuses shows as neighbours alike. Theme cards show
//! a miniature of the application instead; both are the same size, flat shapes and tiny text
//! that never seem to answer the pointer.

use koebi_core::{Rect, Vec2};
use koebi_design::{ColorVision, Deficiency, DesignSystem, Layers};
use koebi_i18n::n_;
use koebi_i18n::tr;
use koebi_tokens::semantic;
use koebi_ui::{Align, BarMapping, NumberField, ThemePreview, Ui, WidgetId};

use super::Stack;
use super::cards::Cards;

const ID: WidgetId = WidgetId::new("settings-accessibility");

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
        label: n_!("Normal vision"),
        detail: n_!("Colors as the theme defines them"),
        vision: ColorVision::Normal,
    },
    Mode {
        label: n_!("Protanopia"),
        detail: n_!("No red cones"),
        vision: simulate(Deficiency::Protan),
    },
    Mode {
        label: n_!("Deuteranopia"),
        detail: n_!("No green cones"),
        vision: simulate(Deficiency::Deutan),
    },
    Mode {
        label: n_!("Tritanopia"),
        detail: n_!("No blue cones"),
        vision: simulate(Deficiency::Tritan),
    },
    Mode {
        label: n_!("Achromatopsia"),
        detail: n_!("No colors"),
        vision: ColorVision::SimulateAchromatopsia { severity: 1.0 },
    },
    Mode {
        label: n_!("Protan correction"),
        detail: n_!("Separates reds from greens"),
        vision: correct(Deficiency::Protan),
    },
    Mode {
        label: n_!("Deutan correction"),
        detail: n_!("Separates greens from reds"),
        vision: correct(Deficiency::Deutan),
    },
    Mode {
        label: n_!("Tritan correction"),
        detail: n_!("Separates blues from yellows"),
        vision: correct(Deficiency::Tritan),
    },
];

/// The groups of modes: their titles and the range of [`MODES`] they hold.
const GROUPS: [(&str, core::ops::Range<usize>); 3] = [
    (n_!("No transformation"), 0..1),
    (n_!("See as someone with a color deficiency"), 1..5),
    (n_!("Correct a color deficiency"), 5..8),
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

/// The miniatures of the current theme as each mode shows it, resolved again only when the
/// design system changes.
#[derive(Debug)]
struct Previews {
    /// The revision of the design system they were resolved from.
    revision: Option<u64>,
    /// One per mode of [`MODES`].
    previews: Vec<ThemePreview>,
}

impl Previews {
    fn new() -> Self {
        Self {
            revision: None,
            previews: Vec::new(),
        }
    }

    /// Resolves the miniatures again if `design` changed since.
    fn refresh(&mut self, design: &DesignSystem) {
        if self.revision == Some(design.revision()) && self.previews.len() == MODES.len() {
            return;
        }
        let Some(view) = design.view(design.active_theme(), Layers::ALL) else {
            return;
        };
        let current = design.color_vision();
        self.previews.clear();
        self.previews.extend(
            MODES.iter().map(|mode| {
                ThemePreview::resolve(&view.seen_as(shown_vision(mode.vision, current)))
            }),
        );
        self.revision = Some(design.revision());
    }
}

/// Where a section of the container lies: its title, its cards, the strength of the mode set
/// when it is one of the section's.
#[derive(Clone, Copy, Debug)]
struct Section {
    title: Rect,
    cards: Cards,
    strength: Option<Rect>,
}

/// The sections in `content`, the mode set being `current`; and the height they take.
fn layout(
    ui: &Ui<'_>,
    content: Rect,
    gap: f32,
    current: ColorVision,
) -> ([Option<Section>; GROUPS.len()], f32) {
    let row = ui.styles().button.height;
    let mut sections = [None; GROUPS.len()];
    let mut y = content.min.y;
    for (index, (_, modes)) in GROUPS.iter().enumerate() {
        if index > 0 {
            // Sections stand apart more than the rows they hold.
            y += 2.0 * gap;
        }
        let title =
            Rect::from_origin_size(Vec2::new(content.min.x, y), Vec2::new(content.width(), row));
        y += row + gap;
        let cards = Cards::new(
            Vec2::new(content.min.x, y),
            content.width(),
            modes.len(),
            gap,
            row,
            |length| ui.whole_pixels(length),
        );
        y += cards.height();
        let chosen = MODES[modes.clone()]
            .iter()
            .any(|mode| is_mode(current, mode.vision));
        let strength = (chosen && strength_of(current).is_some()).then(|| {
            y += gap;
            let at = Rect::from_origin_size(
                Vec2::new(content.min.x, y),
                Vec2::new(content.width(), row),
            );
            y += row;
            at
        });
        sections[index] = Some(Section {
            title,
            cards,
            strength,
        });
    }
    (sections, y - content.min.y)
}

/// The Accessibility page's state.
#[derive(Debug)]
pub(super) struct AccessibilityPage {
    previews: Previews,
    /// Where the sections lay on the last frame.
    sections: [Option<Section>; GROUPS.len()],
}

impl AccessibilityPage {
    pub(super) fn new() -> Self {
        Self {
            previews: Previews::new(),
            sections: [None; GROUPS.len()],
        }
    }

    pub(super) fn show(&mut self, ui: &mut Ui<'_>, stack: &mut Stack, design: &mut DesignSystem) {
        self.previews.refresh(design);
        let gap = ui.whole_pixels(design.get(semantic::SPACING_EDITOR_INSET));
        let current = design.color_vision();
        let (sections, height) = layout(ui, stack.content(ui), gap, current);
        self.sections = sections;
        let previews = &self.previews;
        let mut chosen = None;
        stack.container(ui, height, |ui, _| {
            for (index, (section, (title, modes))) in sections.iter().zip(GROUPS).enumerate() {
                let Some(section) = section else {
                    continue;
                };
                ui.section_title(section.title, tr(title));
                let id = ID.with(index as u64 + 1);
                if let Some(vision) = mode_cards(ui, id, section, previews, current, modes) {
                    chosen = Some(vision);
                }
            }
        });
        if let Some(vision) = chosen
            && let Err(error) = design.set_color_vision(vision)
        {
            koebi_core::warn!("cannot change the color vision setting: {error}");
        }
    }

    /// The card of the mode at `index` of the section at `section`, where the last frame drew
    /// it.
    #[cfg(test)]
    fn card(&self, section: usize, index: usize) -> Option<Rect> {
        self.sections
            .get(section)
            .copied()
            .flatten()
            .map(|section| section.cards.card(index))
    }
}

/// The cards of the modes at `modes` of [`MODES`] in `section`, and how strong the mode set is
/// when it is one of them. Returns the vision chosen.
fn mode_cards(
    ui: &mut Ui<'_>,
    id: WidgetId,
    section: &Section,
    previews: &Previews,
    current: ColorVision,
    modes: core::ops::Range<usize>,
) -> Option<ColorVision> {
    let font = ui.styles().text.label.clone();
    let (primary, secondary) = (ui.styles().text.primary, ui.styles().text.secondary);
    let mut chosen = None;
    for (index, mode_index) in modes.enumerate() {
        let mode = &MODES[mode_index];
        let Some(preview) = previews.previews.get(mode_index) else {
            continue;
        };
        let selected = is_mode(current, mode.vision);
        let card = section.cards.card(index);
        let key = id.with(index as u64 + 1);
        if ui.vision_card(key, card, preview, selected).clicked {
            chosen = Some(mode.vision);
        }
        ui.tooltip(key, tr(mode.detail), card);
        let color = if selected { primary } else { secondary };
        ui.label(
            tr(mode.label),
            section.cards.name(index),
            &font,
            color,
            Align::Center,
        );
    }
    // How strong the mode set is.
    if let Some(at) = section.strength
        && let Some(strength) = strength_of(current)
    {
        let label = Rect::from_origin_size(at.min, Vec2::new(at.width() * 0.3, at.height()));
        ui.label(tr("Strength"), label, &font, primary, Align::Start);
        let field = Rect::from_min_max(Vec2::new(label.max.x, at.min.y), at.max);
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
    }
    chosen
}

#[cfg(test)]
mod tests {
    use koebi_tokens::component;

    use super::super::Page;
    use super::super::tests::Harness;
    use super::*;

    #[test]
    fn a_click_on_a_card_sets_its_mode() {
        let mut harness = Harness::new();
        let at = harness.page_button(Page::Accessibility);
        harness.click(at);
        harness.settle();
        assert_eq!(harness.design.color_vision(), ColorVision::Normal);
        // Protanopia, the first card of the second section.
        let card = harness
            .settings
            .accessibility
            .card(1, 0)
            .expect("cards laid out");
        harness.click(card.center());
        harness.settle();
        let vision = harness.design.color_vision();
        assert!(is_mode(vision, simulate(Deficiency::Protan)), "{vision:?}");
        assert_eq!(strength_of(vision), Some(1.0));
    }

    #[test]
    fn every_card_has_the_size_of_the_others_alone_or_not() {
        let mut harness = Harness::new();
        let at = harness.page_button(Page::Accessibility);
        harness.click(at);
        harness.settle();
        let page = &harness.settings.accessibility;
        let alone = page.card(0, 0).expect("the normal vision card");
        for (section, (_, modes)) in GROUPS.iter().enumerate() {
            for index in 0..modes.len() {
                let card = page.card(section, index).expect("laid out");
                assert_eq!(card.size(), alone.size());
            }
        }
        assert_eq!(alone.width(), super::super::cards::CARD_WIDTH);
    }

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
        assert_eq!(previews.previews.len(), MODES.len());
        let accent = |previews: &Previews, index: usize| previews.previews[index].accent;
        // Normal vision shows the theme's colors; achromatopsia shows them gray.
        assert_eq!(
            accent(&previews, 0),
            design.get(component::BUTTON_ACCENT_BACKGROUND_COLOR_DEFAULT)
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
                .get(component::BUTTON_ACCENT_BACKGROUND_COLOR_DEFAULT)
        );
    }
}
