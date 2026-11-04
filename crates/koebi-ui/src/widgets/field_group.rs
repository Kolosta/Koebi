//! [`FieldGroup`]: fields side by side or stacked as one control, as Blender groups the
//! components of a vector or a color: they share their borders, and only the group's outer
//! corners are rounded.

use koebi_core::{Rect, Vec2};
use koebi_render::Corners;

/// Where the fields of a group lie, and their corners.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldGroup {
    rect: Rect,
    count: usize,
    radius: f32,
    border: f32,
    vertical: bool,
}

impl FieldGroup {
    /// `count` fields filling `rect` from left to right, the group rounded by `radius`, each
    /// field's `border` laid over its neighbour's so that they share one line.
    pub fn horizontal(rect: Rect, count: usize, radius: f32, border: f32) -> Self {
        Self {
            rect,
            count: count.max(1),
            radius: radius.max(0.0),
            border: border.max(0.0),
            vertical: false,
        }
    }

    /// `count` fields filling `rect` from top to bottom, as [`FieldGroup::horizontal`].
    pub fn vertical(rect: Rect, count: usize, radius: f32, border: f32) -> Self {
        Self {
            vertical: true,
            ..Self::horizontal(rect, count, radius, border)
        }
    }

    /// How many fields the group holds.
    pub fn count(&self) -> usize {
        self.count
    }

    /// The rectangle of the field at `index`, and its corners: rounded only at the group's
    /// ends.
    pub fn cell(&self, index: usize) -> (Rect, Corners) {
        let count = self.count as f32;
        let shared = self.border * (count - 1.0);
        let length = if self.vertical {
            self.rect.height()
        } else {
            self.rect.width()
        };
        let size = ((length + shared) / count).max(0.0);
        let index = index.min(self.count - 1);
        let (first, last) = (index == 0, index + 1 == self.count);
        let corner = |rounded: bool| if rounded { self.radius } else { 0.0 };
        if self.vertical {
            let y = self.rect.min.y + index as f32 * (size - self.border);
            let rect = Rect::from_min_max(
                Vec2::new(self.rect.min.x, y),
                Vec2::new(self.rect.max.x, (y + size).min(self.rect.max.y)),
            );
            return (
                rect,
                Corners {
                    top_left: corner(first),
                    top_right: corner(first),
                    bottom_left: corner(last),
                    bottom_right: corner(last),
                },
            );
        }
        let x = self.rect.min.x + index as f32 * (size - self.border);
        let rect = Rect::from_min_max(
            Vec2::new(x, self.rect.min.y),
            Vec2::new((x + size).min(self.rect.max.x), self.rect.max.y),
        );
        (
            rect,
            Corners {
                top_left: corner(first),
                bottom_left: corner(first),
                top_right: corner(last),
                bottom_right: corner(last),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_share_their_borders_and_only_the_ends_are_rounded() {
        let rect = Rect::from_origin_size(Vec2::new(10.0, 0.0), Vec2::new(201.0, 20.0));
        let group = FieldGroup::horizontal(rect, 4, 4.0, 1.0);
        let cells: Vec<(Rect, Corners)> = (0..4).map(|index| group.cell(index)).collect();
        // The group's ends are its rectangle's.
        assert_eq!(cells[0].0.min.x, 10.0);
        assert!((cells[3].0.max.x - 211.0).abs() < 1e-4);
        // Neighbours overlap by a border: one line between them.
        for pair in cells.windows(2) {
            assert!((pair[0].0.max.x - pair[1].0.min.x - 1.0).abs() < 1e-4);
        }
        assert_eq!(cells[0].1.top_left, 4.0);
        assert_eq!(cells[0].1.top_right, 0.0);
        assert_eq!(cells[1].1, Corners::ZERO);
        assert_eq!(cells[3].1.bottom_right, 4.0);
        // One field is rounded on both sides.
        let alone = FieldGroup::horizontal(rect, 1, 4.0, 1.0).cell(0);
        assert_eq!(alone.1, Corners::all(4.0));
        // Stacked: rounded at the top of the first and the bottom of the last.
        let tall = Rect::from_origin_size(Vec2::ZERO, Vec2::new(100.0, 61.0));
        let stack = FieldGroup::vertical(tall, 3, 4.0, 1.0);
        let (first, top) = stack.cell(0);
        let (last, bottom) = stack.cell(2);
        assert_eq!((first.min.y, first.width()), (0.0, 100.0));
        assert!((last.max.y - 61.0).abs() < 1e-4);
        assert!((stack.cell(1).0.min.y - (first.max.y - 1.0)).abs() < 1e-4);
        assert_eq!((top.top_left, top.bottom_left), (4.0, 0.0));
        assert_eq!((bottom.bottom_right, bottom.top_right), (4.0, 0.0));
    }
}
