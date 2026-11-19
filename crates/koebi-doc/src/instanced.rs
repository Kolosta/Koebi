//! Instanced fills: a region filled with generated shapes and families of parallel lines, laid
//! out on a lattice or scattered evenly from a seed (ADR 0029, Carto's instanced fill).
//!
//! - **The settings** of an instanced fill node ([`InstancedSettings`]): the elements it
//!   stamps ([`InstancedElement`]: a shape, its sizes, its color, how it composites) and its
//!   families of lines ([`LineSet`]). The layout's numbers are the node's inputs.
//! - **Generation** ([`generate`]): the layout's poses (a lattice jittered in position and
//!   rotation, or blue-noise points, `koebi_geom::scatter`), then each element stamped at
//!   every pose and each family drawn across the region, in the order the settings list them.
//!   Elements composite by their [`StampMode`]:
//!   - *Fusion*: every stamp of the element is one outline under the non-zero rule, painted
//!     once where stamps overlap, so a translucent field never darkens itself;
//!   - *Blend*: one draw per stamp, over the others;
//!   - *Subtract*: erases what the fill drew below it, the fill alone (it is a group of its
//!     own).

use std::sync::Arc;

use koebi_core::LinearRgba;
use koebi_geom::{
    Affine, Cap, Contour, Contours, Dashes, FillRule, Path, Point, Rect, Rng, Segment, Spacing,
    StrokeStyle, Vector,
};

use crate::eval::{Draw, LayerValue};
use crate::graph::{MergeEntry, NodeId};
use crate::value::BlendMode;

/// A shape an instanced fill stamps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InstancedShape {
    /// A circle of radius `size[0]`.
    #[default]
    Circle,
    /// A rectangle of half-width `size[0]` and half-height `size[1]`.
    Rectangle,
    /// A triangle of sides `size[0]`, `size[1]` and `size[2]`, centered on its centroid.
    Triangle,
    /// A diamond of half-diagonals `size[0]` (across) and `size[1]` (down).
    Diamond,
    /// The upper half of a circle of radius `size[0]`, its flat side through the center.
    HalfCircle,
}

/// How the stamps of an element, or the lines of a family, composite.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StampMode {
    /// All of them one outline: painted once where they overlap.
    #[default]
    Fusion,
    /// Each drawn over the others.
    Blend,
    /// Erasing what the fill drew below them.
    Subtract,
}

/// One element an instanced fill stamps at every pose of its layout.
#[derive(Clone, Debug, PartialEq)]
pub struct InstancedElement {
    /// Whether it is stamped.
    pub enabled: bool,
    /// Its shape.
    pub shape: InstancedShape,
    /// Its sizes, as its shape reads them, in document units.
    pub size: [f64; 3],
    /// Its orientation, before each pose's.
    pub rotation: f64,
    /// How its stamps composite.
    pub mode: StampMode,
    /// Whether it takes the fill's color rather than its own.
    pub fill_color: bool,
    /// Its own color, linear with straight alpha.
    pub color: LinearRgba,
    /// Its opacity.
    pub opacity: f64,
}

impl Default for InstancedElement {
    fn default() -> Self {
        Self {
            enabled: true,
            shape: InstancedShape::Circle,
            size: [2.0; 3],
            rotation: 0.0,
            mode: StampMode::Fusion,
            fill_color: true,
            color: LinearRgba::new(0.0, 0.0, 0.0, 1.0),
            opacity: 1.0,
        }
    }
}

/// How the spacing of a family of lines is measured.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LineSpacing {
    /// Axis to axis: the gap shrinks as the lines widen.
    #[default]
    Center,
    /// Edge to edge: the pitch is the spacing plus the width.
    Border,
}

/// A family of parallel lines across the region, stroked.
#[derive(Clone, Debug, PartialEq)]
pub struct LineSet {
    /// Whether it is drawn.
    pub enabled: bool,
    /// The lines' direction, before the layout's rotation.
    pub angle: f64,
    /// The distance between two lines, as `spacing_mode` measures it.
    pub spacing: f64,
    /// How the spacing is measured.
    pub spacing_mode: LineSpacing,
    /// How far across the lines are moved.
    pub phase: f64,
    /// How far each line is moved along itself from the one before, as a share of its dash
    /// pattern's length: 0.5 puts every other line half a pattern out of step.
    pub stagger: f64,
    /// The lines' width.
    pub width: f64,
    /// Their ends.
    pub cap: Cap,
    /// Their dashes.
    pub dashes: Dashes,
    /// How they composite.
    pub mode: StampMode,
    /// Whether they take the fill's color rather than their own.
    pub fill_color: bool,
    /// Their own color, linear with straight alpha.
    pub color: LinearRgba,
}

impl Default for LineSet {
    fn default() -> Self {
        Self {
            enabled: true,
            angle: 0.0,
            spacing: 20.0,
            spacing_mode: LineSpacing::Center,
            phase: 0.0,
            stagger: 0.0,
            width: 1.0,
            cap: Cap::default(),
            dashes: Dashes::default(),
            mode: StampMode::Fusion,
            fill_color: true,
            color: LinearRgba::new(0.0, 0.0, 0.0, 1.0),
        }
    }
}

/// The settings of an instanced fill node: what it stamps and draws.
#[derive(Clone, Debug, PartialEq)]
pub struct InstancedSettings {
    /// The elements, bottom first.
    pub elements: Vec<InstancedElement>,
    /// The families of lines, over the elements, bottom first.
    pub lines: Vec<LineSet>,
}

impl Default for InstancedSettings {
    /// One circle of radius 2, in the fill's color.
    fn default() -> Self {
        Self {
            elements: vec![InstancedElement::default()],
            lines: Vec::new(),
        }
    }
}

/// Where an instanced fill's poses come from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Layout {
    /// A lattice: two axes of their own pitches and angles, or three (a triangular lattice of
    /// the first pitch), each pose jittered by up to `jitter` units.
    Grid {
        /// The first axis's pitch and the second's (unused with three axes).
        spacing: Vector,
        /// The two axes' angles (the second unused with three axes).
        angles: Vector,
        /// Two axes, or three.
        triangular: bool,
        /// The most a pose moves from its lattice point.
        jitter: f64,
    },
    /// Blue-noise points over the region.
    Scatter {
        /// By count or by distance.
        spacing: Spacing,
        /// Whether whole shapes are kept from overlapping.
        avoid_collisions: bool,
    },
}

/// Everything an instanced fill generates from, but where: its layout, its stamps and lines,
/// and the fill's color.
#[derive(Clone, Debug, PartialEq)]
pub struct InstancedRecipe {
    /// Where its poses come from.
    pub layout: Layout,
    /// The most each pose turns, either way.
    pub rotation_jitter: f64,
    /// The seed of its randomness.
    pub seed: u64,
    /// The whole layout's rotation.
    pub rotation: f64,
    /// What it stamps and draws.
    pub settings: Arc<InstancedSettings>,
    /// The fill's color, for the elements and lines that take it.
    pub color: LinearRgba,
}

/// The most poses one instanced fill generates.
pub const MAX_POSES: usize = 100_000;
/// The most lines a family draws on either side of the region's center.
const MAX_LINES: i64 = 50_000;

/// The outline of `shape` of `size`, centered on the origin; `None` for sizes that make no
/// shape (a triangle whose sides cannot meet).
fn shape(shape: InstancedShape, size: [f64; 3]) -> Option<Contours> {
    let polygon = |points: &[Point]| {
        let segments = points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .map(|(&a, &b)| Segment::line(a, b))
            .collect();
        let mut contours = Contours::default();
        contours.push(Contour::new(segments, true));
        contours
    };
    let [a, b, c] = size.map(f64::abs);
    let contours = match shape {
        InstancedShape::Circle => Path::ellipse(Point::ORIGIN, Vector::new(a, a)).contours(),
        InstancedShape::Rectangle => {
            Path::rectangle(Rect::new(Point::new(-a, -b), Point::new(a, b))).contours()
        }
        InstancedShape::Diamond => polygon(&[
            Point::new(a, 0.0),
            Point::new(0.0, b),
            Point::new(-a, 0.0),
            Point::new(0.0, -b),
        ]),
        InstancedShape::Triangle => {
            // Sides a (opposite the first corner), b and c: the base of length c along x.
            let x = (b * b + c * c - a * a) / (2.0 * c);
            let height = (b * b - x * x).sqrt();
            if !(c > 0.0 && height.is_finite() && height > 0.0) {
                return None;
            }
            let corners = [
                Point::new(0.0, 0.0),
                Point::new(c, 0.0),
                Point::new(x, -height),
            ];
            let centroid = Vector::new((c + x) / 3.0, -height / 3.0);
            polygon(&corners.map(|corner| corner - centroid))
        }
        InstancedShape::HalfCircle => {
            // Two quarter circles as cubics, and the diameter.
            let k = 0.552_284_749_830_793_6 * a;
            let segments = vec![
                Segment::cubic(
                    Point::new(a, 0.0),
                    Point::new(a, -k),
                    Point::new(k, -a),
                    Point::new(0.0, -a),
                ),
                Segment::cubic(
                    Point::new(0.0, -a),
                    Point::new(-k, -a),
                    Point::new(-a, -k),
                    Point::new(-a, 0.0),
                ),
                Segment::line(Point::new(-a, 0.0), Point::new(a, 0.0)),
            ];
            let mut contours = Contours::default();
            contours.push(Contour::new(segments, true));
            contours
        }
    };
    (a > 0.0).then_some(contours)
}

/// How far a shape reaches from its center: what keeps whole shapes apart.
fn radius(element: &InstancedElement) -> f64 {
    let [a, b, c] = element.size.map(f64::abs);
    match element.shape {
        InstancedShape::Circle | InstancedShape::HalfCircle => a,
        InstancedShape::Rectangle => a.hypot(b),
        InstancedShape::Diamond => a.max(b),
        InstancedShape::Triangle => a.max(b).max(c),
    }
}

/// The merge entry of a layer that composites in `mode`.
fn entry(mode: StampMode) -> MergeEntry {
    MergeEntry {
        blend: if mode == StampMode::Subtract {
            BlendMode::Erase
        } else {
            BlendMode::Normal
        },
        ..MergeEntry::default()
    }
}

/// The layer `recipe` generates over `bounds`, its poses where `region` holds (the bounds
/// without one), in the space that `frame` maps to the world: a merge of its elements' draws,
/// then its lines', each draw naming `object`. `tolerance` is the outlines' tolerance, in
/// document units.
pub fn generate(
    recipe: &InstancedRecipe,
    frame: Affine,
    bounds: Rect,
    region: Option<&Contours>,
    object: Option<NodeId>,
    tolerance: f64,
) -> LayerValue {
    let inside = |point: Point| {
        region.map_or_else(
            || bounds.contains(point),
            |region| region.contains(point, FillRule::NonZero),
        )
    };
    let mut rng = Rng::new(recipe.seed);
    let settings = &recipe.settings;
    // The poses: points and their turns.
    let mut points = Vec::new();
    let reach = settings
        .elements
        .iter()
        .filter(|element| element.enabled)
        .map(radius)
        .fold(0.0, f64::max);
    match recipe.layout {
        Layout::Grid {
            spacing,
            angles,
            triangular,
            jitter,
        } => {
            let direction = |angle: f64| Vector::new(angle.cos(), angle.sin());
            let a = direction(recipe.rotation + angles.x) * spacing.x;
            let b = if triangular {
                direction(recipe.rotation + angles.x + core::f64::consts::FRAC_PI_3) * spacing.x
            } else {
                direction(recipe.rotation + angles.y) * spacing.y
            };
            koebi_geom::lattice(
                bounds.inflated(jitter.abs()),
                Point::ORIGIN,
                a,
                b,
                MAX_POSES,
                &mut points,
            );
            for point in &mut points {
                *point += Vector::new(rng.symmetric(), rng.symmetric()) * jitter;
            }
            points.retain(|point| inside(*point));
        }
        Layout::Scatter {
            spacing,
            avoid_collisions,
        } => {
            let clearance = if avoid_collisions { reach } else { 0.0 };
            koebi_geom::scatter(
                bounds,
                spacing,
                clearance,
                &mut rng,
                MAX_POSES,
                inside,
                &mut points,
            );
        }
    }
    let turns: Vec<f64> = points
        .iter()
        .map(|_| recipe.rotation + rng.symmetric() * recipe.rotation_jitter)
        .collect();
    let paint = |fill_color: bool, color: LinearRgba| if fill_color { recipe.color } else { color };
    let mut layers = Vec::new();
    for element in settings.elements.iter().filter(|element| element.enabled) {
        let Some(outline) = shape(element.shape, element.size) else {
            continue;
        };
        let stamp = |point: Point, turn: f64| {
            outline.transformed(
                frame
                    * Affine::translate(point - Point::ORIGIN)
                    * Affine::rotate(element.rotation + turn),
            )
        };
        let draw = |outline: Contours| {
            Arc::new(LayerValue::Draw(Draw {
                object,
                outline: Arc::new(outline),
                rule: FillRule::NonZero,
                color: paint(element.fill_color, element.color),
                opacity: element.opacity,
            }))
        };
        match element.mode {
            StampMode::Blend => {
                for (point, turn) in points.iter().zip(&turns) {
                    layers.push((draw(stamp(*point, *turn)), entry(StampMode::Blend)));
                }
            }
            mode => {
                let mut all = Contours::default();
                for (point, turn) in points.iter().zip(&turns) {
                    for contour in stamp(*point, *turn).contours() {
                        all.push(contour.clone());
                    }
                }
                if !all.is_empty() {
                    layers.push((draw(all), entry(mode)));
                }
            }
        }
    }
    for set in settings.lines.iter().filter(|set| set.enabled) {
        lines(
            recipe,
            set,
            frame,
            bounds,
            object,
            tolerance,
            paint(set.fill_color, set.color),
            &mut layers,
        );
    }
    LayerValue::Merge(layers)
}

/// Appends the draws of family `set` across `bounds`, mapped by `frame`.
#[allow(
    clippy::too_many_arguments,
    reason = "the parts of one family's drawing"
)]
fn lines(
    recipe: &InstancedRecipe,
    set: &LineSet,
    frame: Affine,
    bounds: Rect,
    object: Option<NodeId>,
    tolerance: f64,
    color: LinearRgba,
    layers: &mut Vec<(Arc<LayerValue>, MergeEntry)>,
) {
    let width = set.width.abs();
    let pitch = set.spacing.abs()
        + if set.spacing_mode == LineSpacing::Border {
            width
        } else {
            0.0
        };
    if !(pitch > 0.0 && pitch.is_finite()) || bounds.is_empty() {
        return;
    }
    let angle = recipe.rotation + set.angle;
    let along = Vector::new(angle.cos(), angle.sin());
    let across = Vector::new(-along.y, along.x);
    let center = bounds.center();
    // Long enough to cross the bounds, with room for the stagger and the caps.
    let period: f64 = set.dashes.pattern.iter().map(|length| length.abs()).sum();
    let half = 0.5 * bounds.width().hypot(bounds.height()) + period + width;
    let reach = 0.5 * bounds.width().hypot(bounds.height()) + width;
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a count of lines across a region, far below 2^31"
    )]
    let count = ((reach / pitch).ceil() as i64).min(MAX_LINES);
    let style = StrokeStyle {
        width,
        cap: set.cap,
        dashes: set.dashes.clone(),
        ..StrokeStyle::default()
    };
    let line = |k: i64| {
        #[allow(clippy::cast_precision_loss, reason = "a small line index")]
        let k = k as f64;
        let middle = center + across * (k * pitch + set.phase) + along * (k * set.stagger * period);
        let mut contours = Contours::default();
        contours.push(Contour::new(
            vec![Segment::line(middle - along * half, middle + along * half)],
            false,
        ));
        contours
    };
    let draw = |outline: Contours| {
        Arc::new(LayerValue::Draw(Draw {
            object,
            outline: Arc::new(outline.transformed(frame)),
            rule: FillRule::NonZero,
            color,
            opacity: 1.0,
        }))
    };
    match set.mode {
        StampMode::Blend => {
            for k in -count..=count {
                let outline = koebi_geom::outline(&line(k), &style, tolerance);
                layers.push((draw(outline), entry(StampMode::Blend)));
            }
        }
        mode => {
            let mut all = Contours::default();
            for k in -count..=count {
                for contour in line(k).contours() {
                    all.push(contour.clone());
                }
            }
            let outline = koebi_geom::outline(&all, &style, tolerance);
            if !outline.is_empty() {
                layers.push((draw(outline), entry(mode)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_are_centered_and_sized() {
        let circle = shape(InstancedShape::Circle, [3.0, 0.0, 0.0]).unwrap();
        assert!((circle.bounds().width() - 6.0).abs() < 1e-9);
        let rectangle = shape(InstancedShape::Rectangle, [2.0, 1.0, 0.0]).unwrap();
        assert!((rectangle.bounds().height() - 2.0).abs() < 1e-9);
        let diamond = shape(InstancedShape::Diamond, [2.0, 3.0, 0.0]).unwrap();
        assert!(diamond.contains(Point::new(0.0, 2.9), FillRule::NonZero));
        assert!(!diamond.contains(Point::new(1.5, 1.5), FillRule::NonZero));
        // A 3-4-5 triangle, centered on its centroid; sides that cannot meet make none.
        let triangle = shape(InstancedShape::Triangle, [3.0, 4.0, 5.0]).unwrap();
        assert!(triangle.contains(Point::ORIGIN, FillRule::NonZero));
        assert!((triangle.bounds().width() - 5.0).abs() < 1e-9);
        assert!(shape(InstancedShape::Triangle, [1.0, 1.0, 5.0]).is_none());
        let half = shape(InstancedShape::HalfCircle, [2.0, 0.0, 0.0]).unwrap();
        assert!(half.contains(Point::new(0.0, -1.0), FillRule::NonZero));
        assert!(!half.contains(Point::new(0.0, 1.0), FillRule::NonZero));
        assert!(shape(InstancedShape::Circle, [0.0; 3]).is_none());
    }
}
