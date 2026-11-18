//! [`DotGrid`]: the dots under a canvas that pans and zooms, as in Blender's node editor: two
//! levels drawn by one fill ([`Fill::Dots`]), their spacing chosen by zoom (ADR 0029).
//!
//! Dots lie every `step` canvas units times a power of `subdivisions`: the power that keeps the
//! coarse dots at least `min_distance` apart on screen, and less than `subdivisions` times that.
//! A finer level, `subdivisions` dots per coarse step, fades in as its dots near `min_distance`:
//! zooming in, it reaches full strength just as it becomes the coarse level, and the grid never
//! jumps.

use koebi_core::{Color, Rect, Vec2};

use crate::fill::{Fill, FillPrimitive};

/// A grid of dots under a canvas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DotGrid {
    /// Canvas units between dots at the finest power: dots lie every `step` times a power of
    /// `subdivisions`.
    pub step: f64,
    /// How many finer steps one coarse step holds, 2 at least.
    pub subdivisions: u32,
    /// The smallest distance between coarse dots on screen, in logical pixels.
    pub min_distance: f32,
    /// The diameter of a dot, in logical pixels.
    pub size: f32,
    /// Their color.
    pub color: Color,
}

/// What a [`DotGrid`] shows at one zoom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DotLevels {
    /// Canvas units between coarse dots.
    pub spacing: f64,
    /// Logical pixels between coarse dots on screen: at least the grid's `min_distance`, less
    /// than `subdivisions` times it.
    pub on_screen: f64,
    /// How much the finer level shows, 0 to 1.
    pub fine: f32,
}

impl DotGrid {
    /// The levels shown at `zoom` logical pixels per canvas unit; `None` when no dot can be
    /// placed (a zoom or a step that is not positive and finite).
    pub fn levels(&self, zoom: f64) -> Option<DotLevels> {
        let base = self.step * zoom;
        let min = f64::from(self.min_distance).max(f64::MIN_POSITIVE);
        if !(base > 0.0 && base.is_finite() && min.is_finite()) {
            return None;
        }
        let factor = f64::from(self.subdivisions.max(2));
        // The power of `factor` bringing `base` into [min, min × factor), corrected for the
        // rounding of the logarithm.
        let mut power = (min / base).log(factor).ceil();
        let mut on_screen = base * factor.powf(power);
        while on_screen < min {
            power += 1.0;
            on_screen = base * factor.powf(power);
        }
        while on_screen / factor >= min {
            power -= 1.0;
            on_screen = base * factor.powf(power);
        }
        if !on_screen.is_finite() {
            return None;
        }
        // From nothing, its dots `min / factor` apart, to full, `min` apart.
        #[allow(clippy::cast_possible_truncation, reason = "a share between 0 and 1")]
        let fine = ((on_screen - min) / (min * factor - min)).clamp(0.0, 1.0) as f32;
        Some(DotLevels {
            spacing: self.step * factor.powf(power),
            on_screen,
            fine,
        })
    }

    /// The fill drawing the grid over `area` (logical pixels) for a view at `zoom` logical
    /// pixels per canvas unit, the canvas's origin at `origin` on screen. The origin is kept in
    /// `f64`: a view far from it still places its dots exactly. `None` when nothing can be
    /// drawn.
    pub fn fill(&self, area: Rect, origin: [f64; 2], zoom: f64) -> Option<FillPrimitive> {
        let levels = self.levels(zoom)?;
        let spacing = levels.on_screen;
        // A dot near the area's corner: the shader then works with small numbers.
        let near = |origin: f64, start: f32| {
            let start = f64::from(start);
            #[allow(
                clippy::cast_possible_truncation,
                reason = "a position within a spacing of the area"
            )]
            let near = (start + (origin - start).rem_euclid(spacing)) as f32;
            near
        };
        let origin = Vec2::new(near(origin[0], area.min.x), near(origin[1], area.min.y));
        #[allow(
            clippy::cast_possible_truncation,
            reason = "a spacing on screen, below the subdivisions times the minimum distance"
        )]
        let spacing = spacing as f32;
        Some(FillPrimitive::new(
            area,
            Fill::Dots {
                origin,
                spacing,
                subdivisions: self.subdivisions.max(2),
                size: self.size,
                color: self.color,
                fine: levels.fine,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> DotGrid {
        DotGrid {
            step: 1.0,
            subdivisions: 4,
            min_distance: 24.0,
            size: 2.0,
            color: Color::WHITE,
        }
    }

    #[test]
    fn coarse_dots_stay_between_the_minimum_and_subdivisions_times_it() {
        let grid = grid();
        let mut zoom = 1e-6;
        while zoom < 1e9 {
            let levels = grid.levels(zoom).expect("a zoom");
            assert!(
                (24.0..96.0 + 1e-9).contains(&levels.on_screen),
                "{zoom}: {levels:?}"
            );
            // A power of the subdivisions times the step.
            let power = levels.spacing.log(4.0);
            assert!((power - power.round()).abs() < 1e-9, "{zoom}: {levels:?}");
            assert!((levels.spacing * zoom - levels.on_screen).abs() < 1e-6 * levels.on_screen);
            zoom *= 1.37;
        }
    }

    #[test]
    fn the_finer_level_fades_in_and_the_grid_never_jumps() {
        let grid = grid();
        // Exactly at the minimum, the finer dots lie a quarter of it apart: hidden.
        let at_min = grid.levels(24.0).expect("a zoom");
        assert!((at_min.on_screen - 24.0).abs() < 1e-9);
        assert!(at_min.fine.abs() < 1e-6);
        // Half way, half shown.
        let half = grid.levels(60.0).expect("a zoom");
        assert!((half.fine - 0.5).abs() < 1e-6);
        // Just before the finer level takes over, it shows in full: what follows shows the
        // same dots, the fine ones now coarse.
        let before = grid.levels(96.0 * (1.0 - 1e-9)).expect("a zoom");
        let after = grid.levels(96.0 * (1.0 + 1e-9)).expect("a zoom");
        assert!(before.fine > 0.999 && after.fine < 0.001);
        assert!((before.spacing / 4.0 - after.spacing).abs() < 1e-12);
    }

    #[test]
    fn nothing_is_placed_without_a_zoom_or_a_step() {
        for zoom in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(grid().levels(zoom), None);
        }
        let flat = DotGrid {
            step: 0.0,
            ..grid()
        };
        assert_eq!(flat.levels(1.0), None);
        let area = Rect::from_origin_size(Vec2::ZERO, Vec2::splat(100.0));
        assert_eq!(flat.fill(area, [0.0, 0.0], 1.0), None);
    }

    #[test]
    fn far_views_place_their_dots_near_the_area() {
        let area = Rect::from_origin_size(Vec2::new(10.0, 20.0), Vec2::splat(200.0));
        // The canvas's origin a billion pixels away, the dots still exactly on its lattice.
        let origin = [1.0e9 + 5.0, -3.0e9 + 7.25];
        let fill = grid().fill(area, origin, 32.0).expect("a grid");
        let Fill::Dots {
            origin: near,
            spacing,
            ..
        } = fill.fill
        else {
            panic!("a dot grid");
        };
        assert!((spacing - 32.0).abs() < 1e-6);
        assert!(near.x >= area.min.x && near.x < area.min.x + spacing);
        assert!(near.y >= area.min.y && near.y < area.min.y + spacing);
        let off_lattice = |near: f32, origin: f64| {
            let steps = (f64::from(near) - origin) / f64::from(spacing);
            (steps - steps.round()).abs()
        };
        assert!(off_lattice(near.x, origin[0]) < 1e-6);
        assert!(off_lattice(near.y, origin[1]) < 1e-6);
    }
}
