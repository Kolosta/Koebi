//! The Theme page: a container whose section holds a card per theme, painted with that theme's
//! tokens, and its name.

use koebi_core::{Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_tokens::semantic;
use koebi_ui::{Align, ThemePreview, Ui, WidgetId};

use super::Stack;
use super::cards::Cards;

/// The Theme page's state: where its cards lay on the last frame.
#[derive(Debug)]
pub(super) struct ThemePage {
    cards: Option<Cards>,
}

impl ThemePage {
    pub(super) fn new() -> Self {
        Self { cards: None }
    }

    pub(super) fn show(&mut self, ui: &mut Ui<'_>, stack: &mut Stack, design: &mut DesignSystem) {
        let row = ui.styles().button.height;
        let gap = ui.whole_pixels(design.get(semantic::SPACING_EDITOR_INSET));
        let content = stack.content(ui);
        let cards = Cards::new(
            Vec2::new(content.min.x, content.min.y + row + gap),
            content.width(),
            design.themes().count(),
            gap,
            row,
            |length| ui.whole_pixels(length),
        );
        self.cards = Some(cards);
        stack.container(ui, row + gap + cards.height(), |ui, content| {
            let title = Rect::from_origin_size(content.min, Vec2::new(content.width(), row));
            ui.section_title(title, "Themes");
            theme_cards(ui, &cards, design);
        });
    }

    /// The card of the theme at `index`, where the last frame drew it.
    #[cfg(test)]
    pub(super) fn card(&self, index: usize) -> Option<Rect> {
        self.cards.map(|cards| cards.card(index))
    }
}

/// The cards of the themes, laid by `cards`; a click activates a theme.
fn theme_cards(ui: &mut Ui<'_>, cards: &Cards, design: &mut DesignSystem) {
    let font = ui.styles().text.label.clone();
    let primary = ui.styles().text.primary;
    let secondary = ui.styles().text.secondary;
    let mut chosen = None;
    let active = design.active_theme();
    for (index, (theme, info)) in design.themes().enumerate() {
        let Some(preview) = ThemePreview::of(design, theme) else {
            continue;
        };
        let id = WidgetId::new(info.key()).with(1);
        if ui
            .theme_card(id, cards.card(index), &preview, theme == active)
            .clicked
        {
            chosen = Some(theme);
        }
        let color = if theme == active { primary } else { secondary };
        ui.label(info.name(), cards.name(index), &font, color, Align::Center);
    }
    if let Some(theme) = chosen
        && let Err(error) = design.set_active_theme(theme)
    {
        koebi_core::warn!("cannot activate the theme: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::Harness;

    #[test]
    fn a_click_on_a_card_activates_its_theme() {
        let mut harness = Harness::new();
        harness.settle();
        let first = harness.design.active_theme();
        let second = harness.settings.theme.card(1).expect("cards laid out");
        harness.click(second.center());
        harness.settle();
        assert_ne!(harness.design.active_theme(), first);
    }
}
