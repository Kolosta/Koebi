//! Rows and columns: a rectangle split into rectangles sized by tracks, the way button groups
//! size theirs (ADR 0024, `docs/adr/0024-layout.md`).
//!
//! Components receive their rectangle when they are shown; these functions hand rectangles out
//! without a tree, without allocation and without measuring anything themselves. A [`Track`]
//! is `Fixed`, a `Share` of what the others leave, or `Fit`: as large as its content asks,
//! growing when nothing shares and giving way first when room is short.
//!
//! ```
//! use koebi_core::{Rect, Vec2};
//! use koebi_ui::Track;
//! use koebi_ui::layout;
//!
//! let rect = Rect::from_origin_size(Vec2::ZERO, Vec2::new(212.0, 24.0));
//! // Two halves and a square button, 4 pixels apart.
//! let [name, shortcut, clear] =
//!     layout::row(rect, 4.0, [Track::Fit, Track::Fit, Track::Fixed(24.0)]);
//! assert_eq!((name.width(), shortcut.width(), clear.width()), (90.0, 90.0, 24.0));
//! assert_eq!(clear.max.x, 212.0);
//! ```

use koebi_core::{Rect, Vec2};

use crate::widgets::{MAX_TRACKS, Track, size_tracks};

/// Along which axis rectangles follow one another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    Horizontal,
    Vertical,
}

/// `rect` split along `axis` into `N` rectangles `gap` apart, sized by `tracks`, the natural
/// size of each in `natural` (what a `Fit` track's content asks).
fn split<const N: usize>(
    rect: Rect,
    axis: Axis,
    gap: f32,
    tracks: [Track; N],
    natural: [f32; N],
) -> [Rect; N] {
    const { assert!(N <= MAX_TRACKS, "a row or column holds at most 16 tracks") };
    let (start, length) = match axis {
        Axis::Horizontal => (rect.min.x, rect.width()),
        Axis::Vertical => (rect.min.y, rect.height()),
    };
    let gap = gap.max(0.0);
    #[allow(clippy::cast_precision_loss, reason = "at most 16 tracks")]
    let gaps = gap * N.saturating_sub(1) as f32;
    let mut sizes = [0.0_f32; MAX_TRACKS];
    for (size, (track, natural)) in sizes.iter_mut().zip(tracks.iter().zip(natural)) {
        if matches!(track, Track::Fit) {
            *size = natural.max(0.0);
        }
    }
    size_tracks(&mut sizes, N, &[], &tracks, Some((length - gaps).max(0.0)));
    let mut at = start;
    core::array::from_fn(|index| {
        let size = sizes[index].max(0.0);
        let part = match axis {
            Axis::Horizontal => {
                Rect::from_min_max(Vec2::new(at, rect.min.y), Vec2::new(at + size, rect.max.y))
            }
            Axis::Vertical => {
                Rect::from_min_max(Vec2::new(rect.min.x, at), Vec2::new(rect.max.x, at + size))
            }
        };
        at += size + gap;
        part
    })
}

/// `rect` split left to right into `N` rectangles `gap` apart, sized by `tracks`; `Fit` tracks
/// share the room left as if their content asked for nothing.
pub fn row<const N: usize>(rect: Rect, gap: f32, tracks: [Track; N]) -> [Rect; N] {
    split(rect, Axis::Horizontal, gap, tracks, [0.0; N])
}

/// [`row()`] where each `Fit` track starts at the width its content asks (a component's
/// `size().x`).
pub fn row_fitting<const N: usize>(rect: Rect, gap: f32, tracks: [(Track, f32); N]) -> [Rect; N] {
    split(
        rect,
        Axis::Horizontal,
        gap,
        tracks.map(|(track, _)| track),
        tracks.map(|(_, natural)| natural),
    )
}

/// `rect` split top to bottom into `N` rectangles `gap` apart, sized by `tracks`.
pub fn column<const N: usize>(rect: Rect, gap: f32, tracks: [Track; N]) -> [Rect; N] {
    split(rect, Axis::Vertical, gap, tracks, [0.0; N])
}

/// [`column()`] where each `Fit` track starts at the height its content asks.
pub fn column_fitting<const N: usize>(
    rect: Rect,
    gap: f32,
    tracks: [(Track, f32); N],
) -> [Rect; N] {
    split(
        rect,
        Axis::Vertical,
        gap,
        tracks.map(|(track, _)| track),
        tracks.map(|(_, natural)| natural),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn widths<const N: usize>(rects: [Rect; N]) -> [f32; N] {
        rects.map(|rect| rect.width())
    }

    fn rect(width: f32, height: f32) -> Rect {
        Rect::from_origin_size(Vec2::new(10.0, 20.0), Vec2::new(width, height))
    }

    #[test]
    fn fixed_tracks_keep_their_size_and_shares_split_the_rest() {
        let parts = row(
            rect(110.0, 20.0),
            5.0,
            [Track::Fixed(20.0), Track::Share(1.0), Track::Share(3.0)],
        );
        assert_eq!(widths(parts), [20.0, 20.0, 60.0]);
        // One after the other, the gap between, the row's height each.
        assert_eq!(parts[0].min, Vec2::new(10.0, 20.0));
        assert_eq!(parts[1].min.x, 35.0);
        assert_eq!(parts[2].max, Vec2::new(120.0, 40.0));
    }

    #[test]
    fn fitting_tracks_take_what_their_content_asks_then_give_way_first() {
        let tracks = [
            (Track::Fit, 30.0),
            (Track::Share(1.0), 0.0),
            (Track::Fixed(10.0), 0.0),
        ];
        assert_eq!(
            widths(row_fitting(rect(100.0, 10.0), 0.0, tracks)),
            [30.0, 60.0, 10.0]
        );
        // Too narrow: the fitting track shrinks before the fixed one.
        let tracks = [(Track::Fit, 30.0), (Track::Fixed(10.0), 0.0)];
        assert_eq!(
            widths(row_fitting(rect(25.0, 10.0), 0.0, tracks)),
            [15.0, 10.0]
        );
    }

    #[test]
    fn columns_split_the_height_and_never_go_negative() {
        let parts = column(rect(40.0, 50.0), 10.0, [Track::Fit, Track::Fit]);
        assert_eq!(parts.map(|part| part.height()), [20.0, 20.0]);
        assert_eq!(parts[1].min.y, 50.0);
        let crushed = column(
            rect(40.0, 4.0),
            10.0,
            [Track::Fixed(8.0), Track::Share(1.0)],
        );
        assert!(crushed.iter().all(|part| part.height() >= 0.0));
    }
}
