//! Icons: the vector icons of `koebi-icons` painted with their zones recolored
//! ([`paint_icon`], [`Ui::icon`]), placed in the square they take ([`icon_bounds`]), and the
//! line marks of controls ([`draw_mark`]), a check or a dash whose weight is a token.
//!
//! Components take any [`IconId`]: the interface's glyphs are `koebi_icons::interface::*` (check,
//! plus, chevrons, search...) and `koebi_icons::input::*` (keys and mouse buttons).

use koebi_core::{Color, Rect, Vec2};
use koebi_icons::{IconId, Role};
use koebi_render::{ICON_ZONES, IconColors, IconPrimitive, LinePrimitive, Scene};

use crate::ui::Ui;

/// The color of each role of a vector icon's zones.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconInk {
    /// Zones of the main color: most of an icon.
    pub primary: Color,
    /// Zones of a second color.
    pub secondary: Color,
    /// Zones of a third color.
    pub tertiary: Color,
    /// Accent zones.
    pub accent: Color,
}

impl IconInk {
    /// Every zone in `color`: a disabled icon, an icon on an accent background.
    pub const fn uniform(color: Color) -> Self {
        Self {
            primary: color,
            secondary: color,
            tertiary: color,
            accent: color,
        }
    }

    /// The color of the zones of `role`.
    pub fn of(self, role: Role) -> Color {
        match role {
            Role::Primary => self.primary,
            Role::Secondary => self.secondary,
            Role::Tertiary => self.tertiary,
            Role::Accent => self.accent,
        }
    }
}

/// How a vector icon is painted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IconPaint {
    /// The colors of its SVG file: logos, illustrations, symbols whose colors mean something.
    Original,
    /// Each zone in the color of its role: an interface icon following its component's state
    /// ([`IconStyle::ink`](crate::IconStyle::ink)).
    Ink(IconInk),
}

/// Paints `icon` in the square `bounds` (use [`icon_bounds`] to place one), turned clockwise
/// by `rotation` radians about its center.
pub fn paint_icon(scene: &mut Scene, bounds: Rect, icon: IconId, paint: IconPaint, rotation: f32) {
    let colors = match paint {
        IconPaint::Original => IconColors::Original,
        IconPaint::Ink(ink) => {
            let mut zones = [None; ICON_ZONES];
            for (zone, &role) in zones.iter_mut().zip(icon.roles()) {
                *zone = Some(ink.of(role));
            }
            IconColors::Zones(zones)
        }
    };
    scene.push_icon(&IconPrimitive {
        rect: bounds,
        icon: icon.index(),
        colors,
        opacity: 1.0,
        rotation,
    });
}

/// The square of side `size` centered in `area`, shrunk to fit it, with its corner on a whole
/// logical pixel: where an icon of `size` goes.
pub fn icon_bounds(area: Rect, size: f32) -> Rect {
    let side = size.min(area.width()).min(area.height()).max(0.0);
    let center = area.center();
    let min = Vec2::new(
        (center.x - side * 0.5).round(),
        (center.y - side * 0.5).round(),
    );
    Rect::from_origin_size(min, Vec2::splat(side))
}

impl Ui<'_> {
    /// Paints `icon` in the square `bounds` in `color`, the icon color of its component's state:
    /// its zones by role ([`IconStyle::ink`](crate::IconStyle::ink)).
    pub fn icon(&mut self, bounds: Rect, icon: IconId, color: Color) {
        self.icon_rotated(bounds, icon, color, 0.0);
    }

    /// Paints `icon` like [`Ui::icon`], turned clockwise by `angle` radians about its center: a
    /// fold arrow turning as its fold opens.
    pub fn icon_rotated(&mut self, bounds: Rect, icon: IconId, color: Color, angle: f32) {
        let ink = self.styles().icon.ink(color);
        paint_icon(self.scene(), bounds, icon, IconPaint::Ink(ink), angle);
    }
}

/// A mark of a control, drawn with round-capped lines of a width its component's tokens set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mark {
    /// A check mark: the control is on.
    Check,
    /// A dash: the control is partly on (a checkbox over a mixed selection).
    Dash,
}

/// Marks are designed on a 16 × 16 grid, then scaled to the box they are drawn in.
const GRID: f32 = 16.0;

/// Draws `mark` scaled to the square `bounds`, its lines `width` wide whatever its size.
pub fn draw_mark(scene: &mut Scene, bounds: Rect, mark: Mark, color: Color, width: f32) {
    let scale = bounds.width().min(bounds.height()) / GRID;
    let point = |[x, y]: [f32; 2]| bounds.min + Vec2::new(x, y) * scale;
    match mark {
        Mark::Check => {
            // Both arms in one stroke: the corner is covered once and keeps their weight.
            let line = LinePrimitive::new(point([3.5, 8.5]), point([6.5, 11.5]), width, color);
            scene.push_line(&line.then(point([12.5, 4.5])));
        }
        Mark::Dash => {
            scene.push_line(&LinePrimitive::new(
                point([3.0, 8.0]),
                point([13.0, 8.0]),
                width,
                color,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_scale_to_their_bounds_in_one_stroke() {
        let bounds = Rect::from_origin_size(Vec2::new(10.0, 20.0), Vec2::splat(32.0));
        for mark in [Mark::Check, Mark::Dash] {
            let mut scene = Scene::new();
            draw_mark(&mut scene, bounds, mark, Color::WHITE, 2.0);
            assert_eq!(scene.line_count(), 1, "{mark:?}");
        }
    }

    #[test]
    fn icons_are_painted_with_their_zones_inked_by_role() {
        let mut scene = Scene::new();
        let bounds = Rect::from_origin_size(Vec2::ZERO, Vec2::splat(16.0));
        let ink = IconInk::uniform(Color::WHITE);
        paint_icon(
            &mut scene,
            bounds,
            koebi_icons::interface::CHECK,
            IconPaint::Ink(ink),
            0.0,
        );
        paint_icon(
            &mut scene,
            bounds,
            koebi_icons::app::ICON,
            IconPaint::Original,
            0.0,
        );
        assert_eq!(scene.icon_count(), 2);
        assert_eq!(scene.draw_count(), 1);
        // The pressed button of a mouse is an accent zone.
        assert!(
            koebi_icons::input::MOUSE_LEFT
                .roles()
                .contains(&Role::Accent)
        );
    }

    #[test]
    fn icon_bounds_fit_and_snap() {
        let area = Rect::from_origin_size(Vec2::new(0.0, 0.0), Vec2::new(41.0, 24.0));
        let icon = icon_bounds(area, 16.0);
        assert_eq!(icon.size(), Vec2::splat(16.0));
        assert_eq!(icon.min, Vec2::new(13.0, 4.0));
        let small = icon_bounds(area, 100.0);
        assert_eq!(small.size(), Vec2::splat(24.0));
    }
}
