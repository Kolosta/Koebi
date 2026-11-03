//! Cards in rows, as the Theme and Accessibility pages show them: all the same size, as many as
//! fit a row, each with its name below. A card alone in its section is no larger than the
//! others.

use koebi_core::{Rect, Vec2};

/// The width of a card; narrower only when not even one fits.
pub(super) const CARD_WIDTH: f32 = 144.0;
/// How wide a card is for its height.
pub(super) const CARD_RATIO: f32 = 1.6;

/// Where `count` cards lie from `origin`, in a section `width` wide.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Cards {
    origin: Vec2,
    columns: usize,
    count: usize,
    card: Vec2,
    gap: f32,
    /// The height of the name below each card.
    name: f32,
}

impl Cards {
    /// Cards `gap` apart, each with a name `name` high below it. `snap` rounds lengths to
    /// whole pixels ([`koebi_ui::Ui::whole_pixels`]).
    pub(super) fn new(
        origin: Vec2,
        width: f32,
        count: usize,
        gap: f32,
        name: f32,
        snap: impl Fn(f32) -> f32,
    ) -> Self {
        let fitting = ((width + gap) / (CARD_WIDTH + gap)).floor();
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a few cards, a positive whole number"
        )]
        let columns = (fitting.max(1.0) as usize).min(count.max(1));
        let card = snap(CARD_WIDTH.min(width).max(0.0));
        Self {
            origin,
            columns,
            count,
            card: Vec2::new(card, snap(card / CARD_RATIO)),
            gap,
            name,
        }
    }

    /// How many cards a row holds.
    #[cfg(test)]
    pub(super) fn columns(&self) -> usize {
        self.columns
    }

    /// The card at `index`.
    pub(super) fn card(&self, index: usize) -> Rect {
        let (column, line) = (index % self.columns, index / self.columns);
        Rect::from_origin_size(
            Vec2::new(
                self.origin.x + (self.card.x + self.gap) * column as f32,
                self.origin.y + (self.card.y + self.name + self.gap) * line as f32,
            ),
            self.card,
        )
    }

    /// The name below the card at `index`.
    pub(super) fn name(&self, index: usize) -> Rect {
        let card = self.card(index);
        Rect::from_origin_size(
            Vec2::new(card.min.x, card.max.y),
            Vec2::new(card.width(), self.name),
        )
    }

    /// The height the cards take, their names included.
    pub(super) fn height(&self) -> f32 {
        let lines = self.count.div_ceil(self.columns) as f32;
        lines * (self.card.y + self.name) + (lines - 1.0).max(0.0) * self.gap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cards(width: f32, count: usize) -> Cards {
        Cards::new(Vec2::ZERO, width, count, 10.0, 20.0, f32::round)
    }

    #[test]
    fn cards_keep_their_size_and_wrap_as_they_fit() {
        let wide = cards(1000.0, 4);
        assert_eq!(wide.columns(), 4);
        assert_eq!(wide.card(3).width(), CARD_WIDTH);
        // Alone, a card is as large as when it has neighbours.
        assert_eq!(cards(1000.0, 1).card(0).size(), wide.card(0).size());
        // Fewer fit: they wrap, the same size.
        let narrow = cards(2.0 * CARD_WIDTH + 10.0, 3);
        assert_eq!(narrow.columns(), 2);
        assert_eq!(
            narrow.card(2).min,
            Vec2::new(0.0, narrow.card(0).height() + 30.0)
        );
        assert_eq!(
            narrow.height(),
            2.0 * (narrow.card(0).height() + 20.0) + 10.0
        );
        // Too narrow for one: it takes what there is.
        assert_eq!(cards(100.0, 4).card(0).width(), 100.0);
        let name = narrow.name(1);
        assert_eq!(name.min.y, narrow.card(1).max.y);
        assert_eq!(name.height(), 20.0);
    }
}
