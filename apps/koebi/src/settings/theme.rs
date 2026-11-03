//! The Theme page: a card per theme, painted with that theme's tokens, and its name.

use koebi_core::{Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_tokens::semantic;
use koebi_ui::{Align, CollapsiblePanel, CollapsiblePanelState, ThemePreview, Ui, WidgetId};

use super::Stack;

const ID: WidgetId = WidgetId::new("settings-theme");

/// The width of a card, at least.
const CARD_WIDTH: f32 = 150.0;
/// How wide a card is for its height.
const CARD_RATIO: f32 = 1.6;

/// The Theme page's state.
#[derive(Debug)]
pub(super) struct ThemePage {
    panel: CollapsiblePanelState,
}

impl ThemePage {
    pub(super) fn new() -> Self {
        Self {
            panel: CollapsiblePanelState::new(true),
        }
    }

    pub(super) fn show(&mut self, ui: &mut Ui<'_>, stack: &mut Stack, design: &mut DesignSystem) {
        let group = CollapsiblePanel::new(ID, &mut self.panel, "Theme");
        stack.panel(ui, group, |ui, rect| cards(ui, rect, design));
    }
}

/// How many cards go in a row of `width`, and how wide they are: as many as fit, filling it.
fn grid(width: f32, gap: f32, count: usize) -> (usize, f32) {
    let fitting = ((width + gap) / (CARD_WIDTH + gap)).floor();
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a few cards, a positive whole number"
    )]
    let columns = (fitting.max(1.0) as usize).min(count.max(1));
    let card = ((width - gap * (columns - 1) as f32) / columns as f32).min(CARD_WIDTH * 1.25);
    (columns, card.max(0.0))
}

/// The cards in `rect`; a click activates a theme. Returns the height used.
fn cards(ui: &mut Ui<'_>, rect: Rect, design: &mut DesignSystem) -> f32 {
    let padding = ui.styles().panel.padding;
    let gap = design.get(semantic::SPACING_EDITOR_INSET);
    let row = design.get(semantic::SIZE_CONTROL_HEIGHT);
    let font = ui.styles().text.label.clone();
    let primary = design.get(semantic::TEXT_COLOR_PRIMARY_DEFAULT);
    let secondary = design.get(semantic::TEXT_COLOR_SECONDARY_DEFAULT);
    let left = rect.min.x + padding;
    let width = rect.width() - 2.0 * padding;
    let count = design.themes().count();
    let (columns, card) = grid(width, gap, count);
    let height = (card / CARD_RATIO).round();
    let mut chosen = None;
    let active = design.active_theme();
    for (index, (theme, info)) in design.themes().enumerate() {
        let Some(preview) = ThemePreview::of(design, theme) else {
            continue;
        };
        let (column, line) = (index % columns, index / columns);
        let origin = Vec2::new(
            left + (card + gap) * column as f32,
            rect.min.y + (height + row + gap) * line as f32,
        );
        let at = Rect::from_origin_size(origin, Vec2::new(card, height));
        if ui
            .theme_card(
                WidgetId::new(info.key()).with(1),
                at,
                &preview,
                theme == active,
            )
            .clicked
        {
            chosen = Some(theme);
        }
        let name = Rect::from_origin_size(Vec2::new(origin.x, at.max.y), Vec2::new(card, row));
        let color = if theme == active { primary } else { secondary };
        ui.label(info.name(), name, &font, color, Align::Center);
    }
    if let Some(theme) = chosen
        && let Err(error) = design.set_active_theme(theme)
    {
        koebi_core::warn!("cannot activate the theme: {error}");
    }
    let lines = count.div_ceil(columns) as f32;
    lines * (height + row) + (lines - 1.0).max(0.0) * gap
}

#[cfg(test)]
mod tests {
    use koebi_ui::Styles;

    use super::super::tests::Harness;
    use super::*;

    #[test]
    fn cards_fill_the_row_as_they_fit() {
        assert_eq!(grid(630.0, 10.0, 4), (4, 150.0));
        // Fewer fit: they wrap.
        assert_eq!(grid(310.0, 10.0, 4), (2, 150.0));
        // Too narrow for one: it takes what there is.
        assert_eq!(grid(100.0, 10.0, 4), (1, 100.0));
        // Few cards are not stretched without limit.
        let (columns, card) = grid(2000.0, 10.0, 2);
        assert_eq!(columns, 2);
        assert!((card - 187.5).abs() < 1e-3);
    }

    #[test]
    fn a_click_on_a_card_activates_its_theme() {
        let mut harness = Harness::new();
        harness.settle();
        let styles = Styles::resolve(&harness.design);
        let panel = styles.collapsible_panel;
        let first = harness.design.active_theme();
        // The second card of the row: the first page holds them at the top of the content.
        let margin = harness
            .design
            .get(koebi_tokens::component::SIDE_PANEL_MARGIN_DEFAULT);
        let width = harness
            .design
            .get(koebi_tokens::component::SIDE_PANEL_WIDTH_DEFAULT);
        let gap = harness.design.get(semantic::SPACING_EDITOR_INSET);
        let content = Harness::window().width() - width - margin - 2.0 * styles.panel.padding;
        let (_, card) = grid(content - 2.0 * styles.panel.border_width, gap, 4);
        let x = width + styles.panel.padding + styles.panel.border_width + card + gap + card * 0.5;
        let y = margin + panel.header_height + panel.content_padding + 20.0;
        harness.click(Vec2::new(x, y));
        harness.settle();
        assert_ne!(harness.design.active_theme(), first);
    }
}
