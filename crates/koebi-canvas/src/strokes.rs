//! [`ViewStrokes`]: the outlines of strokes whose lengths count a view's pixels
//! ([`koebi_doc::eval::ViewStroke`]), made per zoom level and kept (ADR 0029).
//!
//! - **Levels** are quarters of an octave of zoom ([`Level`]). A stroke is outlined at its
//!   level's middle zoom: its style is scaled by the document units per pixel there, and its
//!   curves are kept within a sixteenth of a pixel at the level's largest zoom.
//!   - **Hysteresis:** a view changes level only once its zoom leaves the level by a further
//!     sixteenth of an octave. A stroke thus shows within 14% (2^(3/16)) of its width on screen,
//!     and a zoom hovering at a level's edge outlines nothing again.
//! - **Kept** by geometry (its address, the geometry kept alive), style and level. Every view at
//!   a level reuses what one outlined. A recolored or moved-back stroke keeps its outline, and
//!   so its cells in the store, which keys them by the outline's address.
//! - **Culled** before outlining: a stroke whose geometry's bounds, grown by its reach
//!   ([`koebi_geom::StrokeStyle::reach`]), miss the view is not outlined.
//! - **Forgotten** as cells are: what no render used for [`FORGET_AFTER`] renders.

use std::sync::Arc;

use koebi_core::hash::IdMap;
use koebi_geom::{Contours, Rect, StrokeStyle, Stroker};

use crate::store::FORGET_AFTER;

/// Levels a stroke keeps outlines for, per style.
const LEVELS: usize = 2;
/// Levels per octave of zoom.
const PER_OCTAVE: f64 = 4.0;
/// How far past its level a view's zoom goes before it changes level, in levels: a sixteenth
/// of an octave.
const HYSTERESIS: f64 = 0.25;

/// A quarter of an octave of zoom: level `n` holds zooms from `2^(n/4)` to `2^((n+1)/4)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Level(pub(crate) i32);

impl Level {
    /// The level a view at `zoom` (positive and finite) uses, coming from `last`: kept while
    /// the zoom stays within [`HYSTERESIS`] of it.
    pub(crate) fn follow(last: Option<Self>, zoom: f64) -> Self {
        let at = zoom.log2() * PER_OCTAVE;
        match last {
            Some(level)
                if at >= f64::from(level.0) - HYSTERESIS
                    && at < f64::from(level.0) + 1.0 + HYSTERESIS =>
            {
                level
            }
            #[allow(
                clippy::cast_possible_truncation,
                reason = "four times the binary exponent of an f64, within ±4400"
            )]
            _ => Self(at.floor() as i32),
        }
    }

    /// Document units per pixel at the level's middle zoom: what scales a style in pixels.
    pub(crate) fn units(self) -> f64 {
        (-(f64::from(self.0) + 0.5) / PER_OCTAVE).exp2()
    }

    /// The tolerance of its outlines, in document units: a sixteenth of a pixel at the largest
    /// zoom a view keeps the level for.
    fn tolerance(self) -> f64 {
        (-(f64::from(self.0) + 1.0 + HYSTERESIS) / PER_OCTAVE).exp2() / 16.0
    }
}

/// One outline of a stroke.
#[derive(Debug)]
struct Outlined {
    style: Arc<StrokeStyle>,
    level: Level,
    outline: Arc<Contours>,
    /// The last render that used it.
    used: u64,
}

/// What is kept of one stroked geometry.
#[derive(Debug)]
struct Kept {
    /// The geometry: kept, so that its address names it.
    geometry: Arc<Contours>,
    bounds: Rect,
    outlines: Vec<Outlined>,
    /// The last render that showed it, drawn or culled.
    used: u64,
}

/// The outlines of the strokes views show in pixels (see the module).
#[derive(Debug, Default)]
pub(crate) struct ViewStrokes {
    kept: IdMap<usize, Kept>,
    stroker: Stroker,
    /// Renders so far: the current one.
    clock: u64,
    /// The render that last forgot what was not used.
    swept: u64,
    /// Strokes outlined, for the stats.
    pub(crate) outlined: u64,
}

impl ViewStrokes {
    /// Starts a render: what no render used for [`FORGET_AFTER`] renders is forgotten.
    pub(crate) fn begin(&mut self) {
        self.clock += 1;
        if self.clock - self.swept >= FORGET_AFTER {
            let clock = self.clock;
            let unused = |used: u64| used + FORGET_AFTER <= clock;
            self.kept.retain(|_, kept| {
                kept.outlines.retain(|outlined| !unused(outlined.used));
                !unused(kept.used)
            });
            self.swept = clock;
        }
    }

    /// The outline of `geometry` stroked in `style` (lengths in pixels) at `level`, outlined
    /// unless kept; `None` when it cannot reach `area`.
    pub(crate) fn outline(
        &mut self,
        geometry: &Arc<Contours>,
        style: &Arc<StrokeStyle>,
        level: Level,
        area: Rect,
    ) -> Option<Arc<Contours>> {
        let clock = self.clock;
        let kept = self
            .kept
            .entry(Arc::as_ptr(geometry) as usize)
            .or_insert_with(|| Kept {
                geometry: Arc::clone(geometry),
                bounds: geometry.bounds(),
                outlines: Vec::new(),
                used: clock,
            });
        debug_assert!(Arc::ptr_eq(&kept.geometry, geometry));
        kept.used = clock;
        let units = level.units();
        if !kept.bounds.inflated(style.reach() * units).intersects(area) {
            return None;
        }
        let same =
            |outlined: &Outlined| Arc::ptr_eq(&outlined.style, style) || *outlined.style == **style;
        if let Some(outlined) = kept
            .outlines
            .iter_mut()
            .find(|outlined| outlined.level == level && same(outlined))
        {
            outlined.used = clock;
            return Some(Arc::clone(&outlined.outline));
        }
        let mut outline = Contours::default();
        self.stroker.outline_into(
            geometry,
            &style.scaled(units),
            level.tolerance(),
            &mut outline,
        );
        let outline = Arc::new(outline);
        self.outlined += 1;
        // In place of the style's outline used longest ago once it has enough.
        let levels = kept
            .outlines
            .iter()
            .enumerate()
            .filter(|(_, outlined)| same(outlined));
        let oldest = if levels.clone().count() >= LEVELS {
            levels
                .min_by_key(|(_, outlined)| outlined.used)
                .map(|(at, _)| at)
        } else {
            None
        };
        let outlined = Outlined {
            style: Arc::clone(style),
            level,
            outline: Arc::clone(&outline),
            used: clock,
        };
        match oldest {
            Some(at) => kept.outlines[at] = outlined,
            None => kept.outlines.push(outlined),
        }
        Some(outline)
    }
}

#[cfg(test)]
mod tests {
    use koebi_geom::{Path, Point, Vector};

    use super::*;

    #[test]
    fn levels_follow_the_zoom_with_hysteresis() {
        assert_eq!(Level::follow(None, 1.0), Level(0));
        assert_eq!(Level::follow(None, 2.0), Level(4));
        assert_eq!(Level::follow(None, 0.5), Level(-4));
        // Just past the level's top, kept; past it by more than a sixteenth of an octave, left.
        let top = (1.0 / PER_OCTAVE).exp2();
        assert_eq!(Level::follow(Some(Level(0)), top * 1.01), Level(0));
        assert_eq!(Level::follow(Some(Level(0)), top * 1.05), Level(1));
        assert_eq!(Level::follow(Some(Level(0)), 0.99), Level(0));
        assert_eq!(Level::follow(Some(Level(0)), 0.95), Level(-1));
        // A pixel at the middle of the level, and the width within 14% across what it covers.
        for level in [-9, 0, 13] {
            let level = Level(level);
            let low = ((f64::from(level.0) - HYSTERESIS) / PER_OCTAVE).exp2();
            let high = ((f64::from(level.0) + 1.0 + HYSTERESIS) / PER_OCTAVE).exp2();
            for zoom in [low, high] {
                let pixels = level.units() * zoom;
                assert!((0.87..1.15).contains(&pixels), "{pixels}");
            }
            assert!(level.tolerance() * high <= 1.0 / 16.0 + 1e-12);
        }
    }

    #[test]
    fn strokes_are_outlined_once_per_level_and_culled_out_of_sight() {
        let mut strokes = ViewStrokes::default();
        let line = Arc::new(
            Path::rectangle(Rect::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0))).contours(),
        );
        let hairline = Arc::new(StrokeStyle {
            width: 1.0,
            ..StrokeStyle::default()
        });
        let area = Rect::new(Point::new(-10.0, -10.0), Point::new(60.0, 60.0));
        strokes.begin();
        let first = strokes
            .outline(&line, &hairline, Level(0), area)
            .expect("in sight");
        // The same style anew: the same outline.
        let again = Arc::new((*hairline).clone());
        let kept = strokes
            .outline(&line, &again, Level(0), area)
            .expect("in sight");
        assert!(Arc::ptr_eq(&first, &kept));
        assert_eq!(strokes.outlined, 1);
        // Its width is a pixel at the level's middle zoom.
        let width = first.bounds().width() - 100.0;
        assert!((width - Level(0).units()).abs() < 1e-6, "{width}");
        // Two levels kept; a third replaces the one used longest ago.
        strokes
            .outline(&line, &hairline, Level(8), area)
            .expect("in sight");
        strokes.begin();
        strokes
            .outline(&line, &hairline, Level(8), area)
            .expect("in sight");
        strokes
            .outline(&line, &hairline, Level(-8), area)
            .expect("in sight");
        strokes
            .outline(&line, &hairline, Level(8), area)
            .expect("in sight");
        assert_eq!(strokes.outlined, 3);
        strokes
            .outline(&line, &hairline, Level(0), area)
            .expect("in sight");
        assert_eq!(strokes.outlined, 4);
        // Out of sight: not outlined, though a wide stroke reaches it.
        let far = Rect::new(Point::new(110.0, 0.0), Point::new(120.0, 10.0));
        assert!(strokes.outline(&line, &hairline, Level(0), far).is_none());
        let wide = Arc::new(StrokeStyle {
            width: 40.0,
            ..StrokeStyle::default()
        });
        assert!(strokes.outline(&line, &wide, Level(0), far).is_some());
        assert_eq!(strokes.outlined, 5);
        // Forgotten once unused.
        let other = Arc::new(Path::ellipse(Point::new(0.0, 0.0), Vector::new(5.0, 5.0)).contours());
        for _ in 0..2 * FORGET_AFTER {
            strokes.begin();
            strokes.outline(&other, &hairline, Level(0), area);
        }
        assert_eq!(strokes.kept.len(), 1);
    }
}
