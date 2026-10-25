//! [`BarLayout`]: places items along a bar, from the left, from the right and in the centre.

use koebi_core::{Rect, Vec2};

/// Hands out the rectangles of a bar's items: a group packed from the left, a group packed
/// from the right, and items centred in the bar. Items are as tall as the bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarLayout {
    rect: Rect,
    gap: f32,
    /// Where the next left item starts, and where the next right item ends.
    left: f32,
    right: f32,
}

impl BarLayout {
    /// A bar filling `rect`, with `gap` between items.
    pub fn new(rect: Rect, gap: f32) -> Self {
        Self {
            rect,
            gap,
            left: rect.min.x,
            right: rect.max.x,
        }
    }

    /// The whole bar.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// The next item of the left group, `width` wide; empty once the groups meet.
    pub fn left(&mut self, width: f32) -> Rect {
        let width = width.min((self.right - self.left).max(0.0));
        let item = Rect::from_min_max(
            Vec2::new(self.left, self.rect.min.y),
            Vec2::new(self.left + width, self.rect.max.y),
        );
        self.left += width + self.gap;
        item
    }

    /// The next item of the right group, `width` wide, right to left; empty once the groups
    /// meet.
    pub fn right(&mut self, width: f32) -> Rect {
        let width = width.min((self.right - self.left).max(0.0));
        let item = Rect::from_min_max(
            Vec2::new(self.right - width, self.rect.min.y),
            Vec2::new(self.right, self.rect.max.y),
        );
        self.right -= width + self.gap;
        item
    }

    /// An item `width` wide in the middle of the bar.
    pub fn center(&self, width: f32) -> Rect {
        Rect::from_center_size(self.rect.center(), Vec2::new(width, self.rect.height()))
    }

    /// The space left between the two groups.
    pub fn remaining(&self) -> Rect {
        Rect::from_min_max(
            Vec2::new(self.left.min(self.right), self.rect.min.y),
            Vec2::new(self.right.max(self.left), self.rect.max.y),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_fill_from_both_ends_until_they_meet() {
        let mut bar = BarLayout::new(
            Rect::from_min_max(Vec2::new(0.0, 10.0), Vec2::new(100.0, 30.0)),
            4.0,
        );
        assert_eq!(
            bar.left(20.0),
            Rect::from_min_max(Vec2::new(0.0, 10.0), Vec2::new(20.0, 30.0))
        );
        assert_eq!(bar.left(10.0).min.x, 24.0);
        assert_eq!(bar.right(30.0).min.x, 70.0);
        assert_eq!(
            bar.remaining(),
            Rect::from_min_max(Vec2::new(38.0, 10.0), Vec2::new(66.0, 30.0))
        );
        assert_eq!(bar.center(10.0).min.x, 45.0);
        // Past the meeting point, items are empty.
        assert_eq!(bar.left(50.0).width(), 28.0);
        assert_eq!(bar.right(10.0).width(), 0.0);
    }
}
