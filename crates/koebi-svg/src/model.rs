//! A compiled icon: layers of quadratic curves in the unit square of the icon, each filled with
//! a paint whose colors belong to color zones.

use crate::color::Rgba;
use crate::geometry::Point;

/// How a layer decides what lies inside its curves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FillRule {
    /// Inside where the curves wind around a point at least once, either way.
    #[default]
    NonZero,
    /// Inside where a ray from the point crosses the curves an odd number of times.
    EvenOdd,
}

/// What a zone stands for, from the `class` of what it colors (`ds-primary`, ...); what the
/// interface colors it with when it recolors the icon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// The main color: most of an icon.
    #[default]
    Primary,
    /// A second color.
    Secondary,
    /// A third color.
    Tertiary,
    /// The accent.
    Accent,
}

impl Role {
    /// The role a `class` attribute names: the first of `primary` (not `secondary`),
    /// `secondary`, `tertiary`, `accent` its classes contain, as the previous application's
    /// icons wrote them (`ds-primary`).
    pub(crate) fn of_class(class: &str) -> Option<Self> {
        let mut best: Option<Self> = None;
        for name in class.split_whitespace() {
            let role = if name.contains("secondary") {
                Some(Self::Secondary)
            } else if name.contains("primary") {
                Some(Self::Primary)
            } else if name.contains("tertiary") {
                Some(Self::Tertiary)
            } else if name.contains("accent") {
                Some(Self::Accent)
            } else {
                None
            };
            if let Some(role) = role {
                best = Some(best.map_or(role, |kept| kept.min(role)));
            }
        }
        best
    }
}

/// A color zone: the original color the zone groups, and its role.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zone {
    /// The color the SVG gives, opaque (opacity stays with what it colors).
    pub color: Rgba,
    /// What it stands for.
    pub role: Role,
}

/// A color of a paint, and the zone it belongs to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stop {
    /// Where along the gradient, from 0 to 1 (0 for a plain color).
    pub offset: f32,
    /// The color as the SVG gives it, opacities included.
    pub color: Rgba,
    /// Its zone, by index in [`Icon::zones`].
    pub zone: u8,
}

/// What a layer is filled with.
#[derive(Clone, Debug, PartialEq)]
pub enum Paint {
    /// One color.
    Solid(Stop),
    /// A gradient: `to_gradient` takes a point of the icon's square to the gradient's space,
    /// where a linear gradient's position is `x` and a radial one's the distance from the
    /// origin; the stops go along it.
    Gradient {
        /// Linear or radial.
        radial: bool,
        /// From the icon's square to the gradient's space.
        to_gradient: [f32; 6],
        /// The stops, in order, two at least.
        stops: Vec<Stop>,
    },
}

/// A filled region of an icon.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    /// Its outline, as quadratic Béziers in the icon's unit square (y down), closed.
    pub curves: Vec<[Point; 3]>,
    /// How its inside is decided.
    pub rule: FillRule,
    /// What fills it.
    pub paint: Paint,
}

/// A compiled icon.
#[derive(Clone, Debug, PartialEq)]
pub struct Icon {
    /// The width and height of its view box, in SVG units: what one unit of its square is.
    pub view_size: [f32; 2],
    /// Its layers, painted in order.
    pub layers: Vec<Layer>,
    /// Its color zones: those with a role first, in order of role, then by first use.
    pub zones: Vec<Zone>,
}

impl Icon {
    /// How many curves the icon holds.
    pub fn curve_count(&self) -> usize {
        self.layers.iter().map(|layer| layer.curves.len()).sum()
    }
}
