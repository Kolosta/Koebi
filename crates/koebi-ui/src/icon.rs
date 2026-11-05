//! Icons: vector icons of `koebi-icons` painted with their zones recolored ([`paint_icon`]),
//! and [`Icon`], small line icons drawn with round-capped strokes.

use koebi_core::{Color, Rect, Vec2};
use koebi_icons::IconId;
use koebi_icons::Role;
use koebi_render::{ICON_ZONES, IconColors, IconPrimitive, LinePrimitive, Scene};

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

/// Paints `icon` in the square `bounds` (use [`Icon::centered`] to place one), turned
/// clockwise by `rotation` radians about its center.
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

/// Icons are designed on a 16 x 16 grid, then scaled to the box they are drawn in.
const GRID: f32 = 16.0;

/// The body of a mouse, rounded at both ends.
const MOUSE: &[[f32; 2]] = &[
    [8.0, 2.0],
    [10.5, 2.5],
    [11.8, 4.0],
    [12.0, 6.5],
    [12.0, 10.0],
    [11.5, 12.2],
    [10.0, 13.5],
    [8.0, 14.0],
    [6.0, 13.5],
    [4.5, 12.2],
    [4.0, 10.0],
    [4.0, 6.5],
    [4.2, 4.0],
    [5.5, 2.5],
    [8.0, 2.0],
];

/// The line between a mouse's buttons and its palm, and between its two buttons.
const MOUSE_SPLIT: &[[f32; 2]] = &[[4.0, 7.5], [12.0, 7.5]];

/// A line icon. Strokes keep their width whatever the icon's size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    /// A check mark.
    Check,
    /// A plus sign.
    Plus,
    /// A minus sign.
    Minus,
    /// A cross.
    Close,
    /// A chevron pointing left.
    ChevronLeft,
    /// A chevron pointing right.
    ChevronRight,
    /// A chevron pointing up.
    ChevronUp,
    /// A chevron pointing down.
    ChevronDown,
    /// A magnifier: search.
    Search,
    /// An arrow going back around a circle: bring back the default.
    Reset,
    /// A triangle with an exclamation mark: something needs attention.
    Warning,
    /// A circle holding an i: something to know, which needs nothing done.
    Info,
    /// A pipette: pick a color from the screen.
    Eyedropper,
    /// The Command key of Apple keyboards.
    Command,
    /// The Windows key.
    Windows,
    /// The Shift key: a hollow arrow up.
    Shift,
    /// The Tab key: an arrow to a bar.
    Tab,
    /// A mouse, its left button marked.
    MouseLeft,
    /// A mouse, its wheel button marked.
    MouseMiddle,
    /// A mouse, its right button marked.
    MouseRight,
}

impl Icon {
    /// Every icon, for galleries and tests.
    pub const ALL: [Self; 20] = [
        Self::Check,
        Self::Plus,
        Self::Minus,
        Self::Close,
        Self::ChevronLeft,
        Self::ChevronRight,
        Self::ChevronUp,
        Self::ChevronDown,
        Self::Search,
        Self::Reset,
        Self::Warning,
        Self::Info,
        Self::Eyedropper,
        Self::Command,
        Self::Windows,
        Self::Shift,
        Self::Tab,
        Self::MouseLeft,
        Self::MouseMiddle,
        Self::MouseRight,
    ];

    /// The strokes on the 16 x 16 grid: polylines, as their points `[x, y]`.
    fn strokes(self) -> &'static [&'static [[f32; 2]]] {
        match self {
            Self::Check => &[&[[3.5, 8.5], [6.5, 11.5], [12.5, 4.5]]],
            Self::Plus => &[&[[8.0, 3.0], [8.0, 13.0]], &[[3.0, 8.0], [13.0, 8.0]]],
            Self::Minus => &[&[[3.0, 8.0], [13.0, 8.0]]],
            Self::Close => &[&[[4.0, 4.0], [12.0, 12.0]], &[[12.0, 4.0], [4.0, 12.0]]],
            Self::ChevronLeft => &[&[[10.0, 3.5], [5.5, 8.0], [10.0, 12.5]]],
            Self::ChevronRight => &[&[[6.0, 3.5], [10.5, 8.0], [6.0, 12.5]]],
            Self::ChevronUp => &[&[[3.5, 10.0], [8.0, 5.5], [12.5, 10.0]]],
            Self::ChevronDown => &[&[[3.5, 6.0], [8.0, 10.5], [12.5, 6.0]]],
            // An arrow head, and the circle it turns along.
            Self::Reset => &[
                &[[2.0, 2.5], [2.0, 6.5], [6.0, 6.5]],
                &[
                    [2.0, 6.5],
                    [4.32, 4.32],
                    [5.89, 3.25],
                    [7.75, 2.81],
                    [9.63, 3.06],
                    [11.3, 3.98],
                    [12.53, 5.44],
                    [13.14, 7.24],
                    [13.07, 9.15],
                    [12.32, 10.9],
                    [10.99, 12.26],
                    [9.25, 13.05],
                    [7.35, 13.16],
                    [5.54, 12.58],
                    [4.05, 11.38],
                    [3.1, 9.74],
                ],
            ],
            // A circle of twelve segments and a handle.
            Self::Search => &[
                &[
                    [11.0, 6.75],
                    [10.43, 8.88],
                    [8.88, 10.43],
                    [6.75, 11.0],
                    [4.63, 10.43],
                    [3.07, 8.88],
                    [2.5, 6.75],
                    [3.07, 4.62],
                    [4.62, 3.07],
                    [6.75, 2.5],
                    [8.88, 3.07],
                    [10.43, 4.62],
                    [11.0, 6.75],
                ],
                &[[10.06, 10.06], [13.0, 13.0]],
            ],
            // A triangle, the stroke of the mark and its dot.
            Self::Warning => &[
                &[[8.0, 2.0], [14.5, 13.5], [1.5, 13.5], [8.0, 2.0]],
                &[[8.0, 6.0], [8.0, 9.5]],
                &[[8.0, 11.75], [8.0, 11.75]],
            ],
            // A circle, the dot of the i and its stroke.
            // Its tube toward the lower left, the collar across it, the bulb at the top right.
            Self::Eyedropper => &[
                &[
                    [2.5, 13.5],
                    [2.5, 11.5],
                    [8.0, 6.0],
                    [10.0, 8.0],
                    [4.5, 13.5],
                    [2.5, 13.5],
                ],
                &[[6.5, 4.5], [11.5, 9.5]],
                &[[9.0, 5.0], [11.5, 2.5], [13.5, 4.5], [11.0, 7.0]],
            ],
            Self::Info => &[
                &[
                    [14.00, 8.00],
                    [13.54, 10.30],
                    [12.24, 12.24],
                    [10.30, 13.54],
                    [8.00, 14.00],
                    [5.70, 13.54],
                    [3.76, 12.24],
                    [2.46, 10.30],
                    [2.00, 8.00],
                    [2.46, 5.70],
                    [3.76, 3.76],
                    [5.70, 2.46],
                    [8.00, 2.00],
                    [10.30, 2.46],
                    [12.24, 3.76],
                    [13.54, 5.70],
                    [14.00, 8.00],
                ],
                &[[8.0, 5.0], [8.0, 5.0]],
                &[[8.0, 7.5], [8.0, 11.0]],
            ],
            // Two bars each way, looped at their ends: one closed stroke.
            Self::Command => &[&[
                [6.0, 4.0],
                [5.41, 2.59],
                [4.0, 2.0],
                [2.59, 2.59],
                [2.0, 4.0],
                [2.59, 5.41],
                [4.0, 6.0],
                [12.0, 6.0],
                [13.41, 5.41],
                [14.0, 4.0],
                [13.41, 2.59],
                [12.0, 2.0],
                [10.59, 2.59],
                [10.0, 4.0],
                [10.0, 12.0],
                [10.59, 13.41],
                [12.0, 14.0],
                [13.41, 13.41],
                [14.0, 12.0],
                [13.41, 10.59],
                [12.0, 10.0],
                [4.0, 10.0],
                [2.59, 10.59],
                [2.0, 12.0],
                [2.59, 13.41],
                [4.0, 14.0],
                [5.41, 13.41],
                [6.0, 12.0],
                [6.0, 4.0],
            ]],
            // Four panes.
            Self::Windows => &[
                &[
                    [3.0, 3.0],
                    [7.25, 3.0],
                    [7.25, 7.25],
                    [3.0, 7.25],
                    [3.0, 3.0],
                ],
                &[
                    [8.75, 3.0],
                    [13.0, 3.0],
                    [13.0, 7.25],
                    [8.75, 7.25],
                    [8.75, 3.0],
                ],
                &[
                    [3.0, 8.75],
                    [7.25, 8.75],
                    [7.25, 13.0],
                    [3.0, 13.0],
                    [3.0, 8.75],
                ],
                &[
                    [8.75, 8.75],
                    [13.0, 8.75],
                    [13.0, 13.0],
                    [8.75, 13.0],
                    [8.75, 8.75],
                ],
            ],
            Self::Shift => &[&[
                [8.0, 2.5],
                [13.5, 8.0],
                [10.5, 8.0],
                [10.5, 13.5],
                [5.5, 13.5],
                [5.5, 8.0],
                [2.5, 8.0],
                [8.0, 2.5],
            ]],
            Self::Tab => &[
                &[[2.5, 8.0], [12.0, 8.0]],
                &[[8.5, 4.5], [12.0, 8.0], [8.5, 11.5]],
                &[[13.5, 3.5], [13.5, 12.5]],
            ],
            Self::MouseLeft => &[MOUSE, MOUSE_SPLIT, &[[6.0, 4.75], [6.0, 4.75]]],
            Self::MouseMiddle => &[MOUSE, MOUSE_SPLIT, &[[8.0, 4.0], [8.0, 6.0]]],
            Self::MouseRight => &[MOUSE, MOUSE_SPLIT, &[[10.0, 4.75], [10.0, 4.75]]],
        }
    }

    /// Pushes the strokes, each point placed by `point`: the segments of a polyline go by
    /// pairs joined at their shared point, so that a chevron's or a check mark's corner is
    /// covered once and keeps the weight of its arms. Longer polylines (the circles of
    /// `Search` and `Reset`) meet one pair from the next at every other corner, where a
    /// translucent color blends twice.
    fn stroke(
        self,
        scene: &mut Scene,
        color: Color,
        stroke_width: f32,
        point: impl Fn([f32; 2]) -> Vec2,
    ) {
        for polyline in self.strokes() {
            let mut at = 0;
            while at + 1 < polyline.len() {
                let line = LinePrimitive::new(
                    point(polyline[at]),
                    point(polyline[at + 1]),
                    stroke_width,
                    color,
                );
                if let Some(&next) = polyline.get(at + 2) {
                    scene.push_line(&line.then(point(next)));
                    at += 2;
                } else {
                    scene.push_line(&line);
                    at += 1;
                }
            }
        }
    }

    /// Draws the icon scaled to the square `bounds` (use [`Icon::centered`] to place one).
    pub fn draw(self, scene: &mut Scene, bounds: Rect, color: Color, stroke_width: f32) {
        let scale = bounds.width().min(bounds.height()) / GRID;
        self.stroke(scene, color, stroke_width, |[x, y]| {
            bounds.min + Vec2::new(x, y) * scale
        });
    }

    /// Draws the icon like [`Icon::draw`], turned clockwise by `angle` radians around the
    /// center of `bounds`: a fold arrow turning as its fold opens.
    pub(crate) fn draw_rotated(
        self,
        scene: &mut Scene,
        bounds: Rect,
        color: Color,
        stroke_width: f32,
        angle: f32,
    ) {
        let scale = bounds.width().min(bounds.height()) / GRID;
        let center = bounds.center();
        let (sin, cos) = angle.sin_cos();
        self.stroke(scene, color, stroke_width, |[x, y]| {
            let local = Vec2::new(x - GRID * 0.5, y - GRID * 0.5) * scale;
            center + Vec2::new(local.x * cos - local.y * sin, local.x * sin + local.y * cos)
        });
    }

    /// The square of side `size` centered in `area`, shrunk to fit it, with its corner on a
    /// whole logical pixel.
    pub fn centered(area: Rect, size: f32) -> Rect {
        let side = size.min(area.width()).min(area.height()).max(0.0);
        let center = area.center();
        let min = Vec2::new(
            (center.x - side * 0.5).round(),
            (center.y - side * 0.5).round(),
        );
        Rect::from_origin_size(min, Vec2::splat(side))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strokes_stay_inside_the_grid() {
        for icon in Icon::ALL {
            for polyline in icon.strokes() {
                assert!(polyline.len() >= 2, "{icon:?}");
                let inside = polyline.iter().flatten().all(|v| (0.0..=GRID).contains(v));
                assert!(inside, "{icon:?}");
            }
        }
    }

    #[test]
    fn icons_scale_to_their_bounds() {
        let mut scene = Scene::new();
        let bounds = Rect::from_origin_size(Vec2::new(10.0, 20.0), Vec2::splat(32.0));
        Icon::Plus.draw(&mut scene, bounds, Color::WHITE, 2.0);
        assert_eq!(scene.line_count(), 2);
        assert_eq!(scene.draw_count(), 1);
    }

    #[test]
    fn polylines_join_their_segments_by_pairs() {
        let bounds = Rect::from_origin_size(Vec2::ZERO, Vec2::splat(16.0));
        let count = |icon: Icon| {
            let mut scene = Scene::new();
            icon.draw(&mut scene, bounds, Color::WHITE, 1.5);
            scene.line_count()
        };
        // Two segments joined in one stroke.
        assert_eq!(count(Icon::ChevronRight), 1);
        assert_eq!(count(Icon::Check), 1);
        // Two separate strokes.
        assert_eq!(count(Icon::Close), 2);
        // An arrow head, and fifteen segments of the circle: seven pairs and one alone.
        assert_eq!(count(Icon::Reset), 1 + 8);
        // Twelve segments of the circle, six pairs, and the handle.
        assert_eq!(count(Icon::Search), 6 + 1);
    }

    #[test]
    fn centered_boxes_fit_and_snap() {
        let area = Rect::from_origin_size(Vec2::new(0.0, 0.0), Vec2::new(41.0, 24.0));
        let icon = Icon::centered(area, 16.0);
        assert_eq!(icon.size(), Vec2::splat(16.0));
        assert_eq!(icon.min, Vec2::new(13.0, 4.0));
        let small = Icon::centered(area, 100.0);
        assert_eq!(small.size(), Vec2::splat(24.0));
    }
}
