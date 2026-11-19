//! Stroke repeats and marks (ADR 0029, Carto's fence ticks and line marks): objects stamped
//! along a stroke, as part of its style.
//!
//! - **Repeats** ([`StrokeRepeat`]): runs of objects along the whole stroke, in groups, laid
//!   out by pitch, gap, count or density, trimmed at the ends, on either side of the line.
//! - **Marks** ([`StrokeMark`]): placed by hand at a point of one subpath. A mark may re-phase
//!   the dashes (a dash or a gap centered on it), and carries objects: shapes, or a gap that
//!   cuts the line open with its own caps and objects at its two ends.
//! - **Objects** composite with the stroke by their [`StampMode`]: fused into its outline
//!   (one drawing, a translucent stroke never doubles up), blended as draws of their own (in
//!   front of the stroke or behind it), or subtracted, cutting the stroke alone.
//! - **Placement:** at a point of the line, turned to its tangent. Rigid shapes (*Hard*), on
//!   the chord between the line's points at either end (*Chord*), or a rectangle that follows
//!   the line (*Follow*: the band of the line's stretch). Sides are judged walking from the
//!   start: right toward the stroke's normal, left away; inside and outside follow the fill on
//!   closed subpaths and the curvature on open ones.
//!
//! Sizes and offsets are shares of the stroke's width unless told in document units.

use koebi_core::LinearRgba;
use koebi_geom::{
    Affine, Align, Cap, Contour, Contours, DashAnchor, DashCenter, Dashes, FillRule, Join, Path,
    Point, Rect, Segment, StrokeOffset, StrokeStyle, Vector,
};

use crate::instanced::StampMode;
use crate::value::BlendMode;

/// The shape of a stamped object.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MarkShape {
    /// A circle of radius `size`.
    #[default]
    Circle,
    /// A rectangle `size` long either way along the line and `width` either way across.
    Rectangle,
    /// A diamond of half-diagonals `size` (along) and `width` (across).
    Diamond,
    /// An isosceles triangle of half-base `size` on the line and height `width` across it.
    Triangle,
    /// A half disc of radius `size`, its flat side on the line.
    HalfCircle,
    /// A segment across the line, `size` long from its start and `width` either way thick.
    Line,
    /// An opening cut in the line, `size` long: only for marks.
    Gap,
}

/// Which side of the line objects sit on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MarkSide {
    /// Across the line.
    #[default]
    Center,
    /// Away from the stroke's normal, walking from the start.
    Left,
    /// Toward the normal.
    Right,
    /// The fill's side on a closed subpath, the concave side on an open one.
    Inside,
    /// The other side.
    Outside,
}

/// How an object relates to the curve.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MarkBend {
    /// Rigid, turned to the tangent at its point.
    #[default]
    Hard,
    /// Rigid, on the chord between the line's points at its two ends along the line.
    Chord,
    /// Bending with the line: a rectangle becomes the band of the line's stretch it spans.
    Follow,
}

/// The ends of a gap's opening.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GapCap {
    /// Cut square.
    #[default]
    Butt,
    /// Rounded.
    Round,
    /// Square, half the width further.
    Square,
}

/// One object a mark or a repeat stamps.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkObject {
    /// Its shape.
    pub shape: MarkShape,
    /// How it composites with the stroke.
    pub mode: StampMode,
    /// How it bends with the line.
    pub bend: MarkBend,
    /// Its blend mode, blended.
    pub blend: BlendMode,
    /// Drawn in front of the stroke, or behind it, blended.
    pub front: bool,
    /// Its size, as its shape reads it.
    pub size: f64,
    /// Its width, as its shape reads it.
    pub width: f64,
    /// Whether its sizes, offsets and along shift are shares of the stroke's width (in
    /// percent) rather than document units.
    pub percent: bool,
    /// Its turn about its point.
    pub rotation: f64,
    /// How far along the line it moves from its point.
    pub along: f64,
    /// Its own side and offset across the line; the mark's without.
    pub side: Option<(MarkSide, f64)>,
    /// A gap's opening ends.
    pub gap_caps: [GapCap; 2],
    /// The objects at a gap's start and end.
    pub gap_objects: [Vec<MarkObject>; 2],
    /// Whether it takes the stroke's color rather than its own.
    pub stroke_color: bool,
    /// Its own color, linear with straight alpha.
    pub color: LinearRgba,
    /// Its opacity; subtracted, how much it erases.
    pub opacity: f64,
}

impl Default for MarkObject {
    fn default() -> Self {
        Self {
            shape: MarkShape::Circle,
            mode: StampMode::Fusion,
            bend: MarkBend::Hard,
            blend: BlendMode::Normal,
            front: true,
            size: 100.0,
            width: 100.0,
            percent: true,
            rotation: 0.0,
            along: 0.0,
            side: None,
            gap_caps: [GapCap::Butt; 2],
            gap_objects: [Vec::new(), Vec::new()],
            stroke_color: true,
            color: LinearRgba::new(0.0, 0.0, 0.0, 1.0),
            opacity: 1.0,
        }
    }
}

/// What a mark makes of the dashes there.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MarkPhase {
    /// Nothing.
    #[default]
    Neutral,
    /// A dash centered on it.
    Dash,
    /// A gap centered on it.
    Gap,
}

/// A mark placed by hand on one subpath.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StrokeMark {
    /// The subpath: its index among the contours.
    pub contour: usize,
    /// Where on it, as a share of its length: 0 at the start, 1 at the end.
    pub at: f64,
    /// What it makes of the dashes.
    pub phase: MarkPhase,
    /// The length of the dash or gap it centers; the pattern's own without.
    pub anchor_size: Option<f64>,
    /// The side its objects sit on, and how far.
    pub side: MarkSide,
    /// That offset.
    pub offset: f64,
    /// Its objects.
    pub objects: Vec<MarkObject>,
}

/// How a repeat's groups are laid out along the line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Distribute {
    /// A fixed distance between groups' centers.
    Pitch(f64),
    /// A fixed distance between groups' edges.
    Gap(f64),
    /// Exactly this many groups over the line.
    Count(usize),
    /// This many groups per 100 units.
    Density(f64),
}

/// What a repeat's trims are measured to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TrimMeasure {
    /// The first and last groups' centers.
    #[default]
    Center,
    /// Their outer edges.
    Outside,
}

/// A run of objects repeated along the whole stroke.
#[derive(Clone, Debug, PartialEq)]
pub struct StrokeRepeat {
    /// Whether it is drawn.
    pub enabled: bool,
    /// The object stamped: its shape, size, mode and color. Its bend is rigid.
    pub object: MarkObject,
    /// Its side across the line, and how far, as the object's sizes are measured.
    pub side: MarkSide,
    /// That offset.
    pub offset: f64,
    /// How its groups are laid out.
    pub distribute: Distribute,
    /// How far into the line the run starts.
    pub phase: f64,
    /// Objects per group.
    pub group_count: usize,
    /// The distance between objects' centers within a group.
    pub group_pitch: f64,
    /// The length left out at each end.
    pub trims: [f64; 2],
    /// What the trims are measured to.
    pub trim_measure: TrimMeasure,
}

impl Default for StrokeRepeat {
    /// The fence tick: rectangles across the line every 8 units.
    fn default() -> Self {
        Self {
            enabled: true,
            object: MarkObject {
                shape: MarkShape::Rectangle,
                size: 25.0,
                width: 100.0,
                ..MarkObject::default()
            },
            side: MarkSide::Center,
            offset: 50.0,
            distribute: Distribute::Pitch(8.0),
            phase: 0.0,
            group_count: 1,
            group_pitch: 2.0,
            trims: [0.0; 2],
            trim_measure: TrimMeasure::Center,
        }
    }
}

/// A stroke node's settings: its dashes, repeats and marks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StrokeSettings {
    /// The dashes.
    pub dashes: Dashes,
    /// The repeats, in order.
    pub repeats: Vec<StrokeRepeat>,
    /// The marks.
    pub marks: Vec<StrokeMark>,
}

impl StrokeSettings {
    /// The dashes with the marks' anchors: a dash or a gap centered on each mark that asks.
    pub fn dashes(&self) -> Dashes {
        let mut dashes = self.dashes.clone();
        for mark in &self.marks {
            let center = match mark.phase {
                MarkPhase::Neutral => continue,
                MarkPhase::Dash => DashCenter::Dash,
                MarkPhase::Gap => DashCenter::Gap,
            };
            dashes.anchors.push(DashAnchor {
                contour: mark.contour,
                at: mark.at,
                center,
                size: mark.anchor_size,
            });
        }
        dashes
    }
}

/// What a stroke's objects add to it: the outline fused into the stroke's, the draws over or
/// under it, and the outlines that cut it.
#[derive(Clone, Debug, Default)]
pub struct Decorations {
    /// Fused into the stroke's outline.
    pub fused: Contours,
    /// Drawn on their own: the outline, color, opacity, blend mode, and in front or behind.
    pub drawn: Vec<(Contours, LinearRgba, f64, BlendMode, bool)>,
    /// Cutting the stroke: the outline and how much it erases.
    pub cut: Vec<(Contours, f64)>,
}

impl Decorations {
    /// Whether nothing was added.
    pub fn is_empty(&self) -> bool {
        self.fused.is_empty() && self.drawn.is_empty() && self.cut.is_empty()
    }
}

/// A point of the line: where, its unit tangent, and its right normal (toward the stroke's
/// normal).
#[derive(Clone, Copy, Debug)]
struct Frame {
    point: Point,
    tangent: Vector,
    right: Vector,
}

/// The frame of `contour` at arc length `at`, and the side its curvature turns to there
/// (positive: right).
fn frame(contour: &Contour, at: f64) -> Option<(Frame, f64)> {
    let mut remaining = at.max(0.0);
    let last = contour.segments().len().checked_sub(1)?;
    for (index, segment) in contour.segments().iter().enumerate() {
        let own = segment.arc_length();
        if remaining <= own || index == last {
            let t = segment.param_at_length(remaining, own.max(1.0) * 1e-12);
            let tangent = segment
                .tangent(t)
                .or_else(|| (segment.end() - segment.start()).normalized())
                .unwrap_or(Vector::new(1.0, 0.0));
            let right = Vector::new(-tangent.y, tangent.x);
            return Some((
                Frame {
                    point: segment.eval(t),
                    tangent,
                    right,
                },
                segment.curvature(t),
            ));
        }
        remaining -= own;
    }
    None
}

/// The signed area of `contour`, flattened: positive when its inside lies to its right.
fn signed_area(contour: &Contour) -> f64 {
    let mut points = Vec::new();
    for segment in contour.segments() {
        segment.flatten(0.1, &mut points);
    }
    let mut area = 0.0;
    for (a, b) in points.iter().zip(points.iter().cycle().skip(1)) {
        area += a.x * b.y - b.x * a.y;
    }
    0.5 * area
}

/// The unit vector across the line toward `side` at `frame` of `contour` (`bend` the
/// curvature's sign there); the left normal for the center.
fn across(side: MarkSide, frame: Frame, contour: &Contour, bend: f64) -> Vector {
    let left = -frame.right;
    let inside_right = || {
        if contour.is_closed() {
            signed_area(contour) > 0.0
        } else {
            bend > 0.0
        }
    };
    match side {
        MarkSide::Center | MarkSide::Left => left,
        MarkSide::Right => frame.right,
        MarkSide::Inside => {
            if inside_right() {
                frame.right
            } else {
                left
            }
        }
        MarkSide::Outside => {
            if inside_right() {
                left
            } else {
                frame.right
            }
        }
    }
}

/// A closed polygon through `points`.
fn polygon(points: &[Point]) -> Contours {
    let segments = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(&a, &b)| Segment::line(a, b))
        .collect();
    let mut contours = Contours::default();
    contours.push(Contour::new(segments, true));
    contours
}

/// The outline of a rigid `shape` of `size` and `width`, in its own frame: x along the line,
/// y across it toward its side, centered on the origin (a line starting there).
fn rigid(shape: MarkShape, size: f64, width: f64) -> Option<Contours> {
    let (size, width) = (size.abs(), width.abs());
    if size <= 0.0 || !size.is_finite() {
        return None;
    }
    Some(match shape {
        MarkShape::Circle => Path::ellipse(Point::ORIGIN, Vector::new(size, size)).contours(),
        MarkShape::Rectangle => Path::rectangle(Rect::new(
            Point::new(-size, -width),
            Point::new(size, width),
        ))
        .contours(),
        MarkShape::Diamond => polygon(&[
            Point::new(size, 0.0),
            Point::new(0.0, width),
            Point::new(-size, 0.0),
            Point::new(0.0, -width),
        ]),
        MarkShape::Triangle => polygon(&[
            Point::new(-size, 0.0),
            Point::new(size, 0.0),
            Point::new(0.0, width),
        ]),
        MarkShape::HalfCircle => {
            let k = 0.552_284_749_830_793_6 * size;
            let segments = vec![
                Segment::cubic(
                    Point::new(size, 0.0),
                    Point::new(size, k),
                    Point::new(k, size),
                    Point::new(0.0, size),
                ),
                Segment::cubic(
                    Point::new(0.0, size),
                    Point::new(-k, size),
                    Point::new(-size, k),
                    Point::new(-size, 0.0),
                ),
                Segment::line(Point::new(-size, 0.0), Point::new(size, 0.0)),
            ];
            let mut contours = Contours::default();
            contours.push(Contour::new(segments, true));
            contours
        }
        MarkShape::Line => {
            Path::rectangle(Rect::new(Point::new(-width, 0.0), Point::new(width, size))).contours()
        }
        MarkShape::Gap => return None,
    })
}

/// Everything placing objects on one subpath takes.
struct Line<'a> {
    contour: &'a Contour,
    length: f64,
    width: f64,
    stroke_color: LinearRgba,
    tolerance: f64,
}

impl Line<'_> {
    /// A length in units: `value` as is, or a share of the stroke's width in percent.
    fn units(&self, value: f64, percent: bool) -> f64 {
        if percent {
            value * 0.01 * self.width
        } else {
            value
        }
    }

    /// The outline of `object` at arc length `at`, on `side` at `offset` units across.
    fn outline(
        &self,
        object: &MarkObject,
        at: f64,
        side: MarkSide,
        offset: f64,
    ) -> Option<Contours> {
        let size = self.units(object.size, object.percent);
        let width = self.units(object.width, object.percent);
        let at = at + self.units(object.along, object.percent);
        let (frame, bend) = frame(self.contour, at)?;
        let toward = across(side, frame, self.contour, bend);
        let offset = if side == MarkSide::Center {
            0.0
        } else {
            offset
        };
        if object.bend == MarkBend::Follow && object.shape == MarkShape::Rectangle {
            // The band of the line's stretch the rectangle spans, moved across by the offset.
            let stretch = self.contour.between(at - size, at + size);
            let signed = if toward.dot(frame.right) > 0.0 {
                offset
            } else {
                -offset
            };
            let style = StrokeStyle {
                width: 2.0 * width,
                align: if signed >= 0.0 {
                    Align::Right
                } else {
                    Align::Left
                },
                offset: StrokeOffset::Units(signed.abs()),
                cap: Cap::Butt { tilt: 0.0 },
                join: Join::Round,
                dashes: Dashes::default(),
            };
            let mut wrapped = Contours::default();
            wrapped.push(stretch);
            return Some(koebi_geom::outline(&wrapped, &style, self.tolerance));
        }
        let shape = rigid(object.shape, size, width)?;
        // Chord: the shape spans straight between the line's points at its two ends.
        let (center, direction) = if object.bend == MarkBend::Chord {
            let ends = (
                frame_point(self.contour, at - size),
                frame_point(self.contour, at + size),
            );
            match ends {
                (Some(a), Some(b)) => {
                    (a.midpoint(b), (b - a).normalized().unwrap_or(frame.tangent))
                }
                _ => (frame.point, frame.tangent),
            }
        } else {
            (frame.point, frame.tangent)
        };
        // The shape's y goes toward its side: mirrored when that is the left.
        let toward_right = toward.dot(frame.right) > 0.0;
        let normal = if toward_right {
            Vector::new(-direction.y, direction.x)
        } else {
            Vector::new(direction.y, -direction.x)
        };
        let place = Affine::new([
            direction.x,
            direction.y,
            normal.x,
            normal.y,
            center.x + toward.x * offset,
            center.y + toward.y * offset,
        ]) * Affine::rotate(object.rotation);
        Some(shape.transformed(place))
    }

    /// Adds `object` at `at` to `decorations`, on `side` at `offset` units across.
    fn add(
        &self,
        object: &MarkObject,
        at: f64,
        side: MarkSide,
        offset: f64,
        decorations: &mut Decorations,
    ) {
        let (side, offset) = object.side.map_or((side, offset), |(side, offset)| {
            (side, self.units(offset, object.percent))
        });
        if object.shape == MarkShape::Gap {
            self.gap(object, at, side, offset, decorations);
            return;
        }
        let Some(outline) = self.outline(object, at, side, offset) else {
            return;
        };
        let color = if object.stroke_color {
            self.stroke_color
        } else {
            object.color
        };
        match object.mode {
            StampMode::Fusion => {
                for contour in outline.contours() {
                    decorations.fused.push(contour.clone());
                }
            }
            StampMode::Blend => {
                decorations.drawn.push((
                    outline,
                    color,
                    object.opacity,
                    object.blend,
                    object.front,
                ));
            }
            StampMode::Subtract => decorations.cut.push((outline, object.opacity)),
        }
    }

    /// Cuts the line open over `object.size` centered on `at`, with its caps, and adds the
    /// objects at its two ends.
    fn gap(
        &self,
        object: &MarkObject,
        at: f64,
        side: MarkSide,
        offset: f64,
        decorations: &mut Decorations,
    ) {
        let length = self.units(object.size, object.percent).abs();
        let (from, to) = (at - length * 0.5, at + length * 0.5);
        if length > 0.0 {
            // The opening: the line's stretch, wider than the stroke, each end of its cap.
            let mut stretch = Contours::default();
            stretch.push(self.contour.between(from, to));
            let cap = |cap: GapCap| match cap {
                GapCap::Butt => Cap::Butt { tilt: 0.0 },
                GapCap::Round => Cap::Round,
                GapCap::Square => Cap::Square,
            };
            let style = StrokeStyle {
                width: self.width * 2.0 + 2.0 * offset.abs(),
                cap: cap(object.gap_caps[0]),
                join: Join::Round,
                ..StrokeStyle::default()
            };
            let mut opening = koebi_geom::outline(&stretch, &style, self.tolerance);
            if object.gap_caps[1] != object.gap_caps[0] {
                // The end's cap differs: the opening's second half outlined with it.
                let end = StrokeStyle {
                    cap: cap(object.gap_caps[1]),
                    ..style.clone()
                };
                let mut second = Contours::default();
                second.push(self.contour.between(at, to));
                let mut first = Contours::default();
                first.push(self.contour.between(from, at));
                opening = koebi_geom::outline(&first, &style, self.tolerance);
                for contour in koebi_geom::outline(&second, &end, self.tolerance).contours() {
                    opening.push(contour.clone());
                }
            }
            decorations.cut.push((opening, object.opacity));
        }
        for (end, objects) in [from, to].into_iter().zip(&object.gap_objects) {
            for marker in objects {
                if marker.shape != MarkShape::Gap {
                    self.add(marker, end, side, offset, decorations);
                }
            }
        }
    }
}

/// The point of `contour` at arc length `at`, clamped.
fn frame_point(contour: &Contour, at: f64) -> Option<Point> {
    contour.point_at_length(at).map(|(point, _)| point)
}

/// The arc lengths of a repeat's object centers on a line of `length`, whose object reaches
/// `half` either way along it.
fn positions(repeat: &StrokeRepeat, length: f64, half: f64) -> Vec<f64> {
    let count = repeat.group_count.max(1);
    #[allow(clippy::cast_precision_loss, reason = "a small count of objects")]
    let span = (count - 1) as f64 * repeat.group_pitch.abs();
    let reach = span * 0.5 + half;
    let measure = if repeat.trim_measure == TrimMeasure::Outside {
        reach
    } else {
        0.0
    };
    let (start, end) = (
        repeat.trims[0].max(0.0) + measure,
        length - repeat.trims[1].max(0.0) - measure,
    );
    if end < start || !end.is_finite() {
        return Vec::new();
    }
    let room = end - start;
    let pitch = match repeat.distribute {
        Distribute::Pitch(pitch) => pitch.abs(),
        Distribute::Gap(gap) => gap.abs() + 2.0 * reach,
        #[allow(clippy::cast_precision_loss, reason = "a count of groups")]
        Distribute::Count(groups) => {
            if groups <= 1 {
                f64::INFINITY
            } else {
                room / (groups - 1) as f64
            }
        }
        Distribute::Density(per_hundred) => 100.0 / per_hundred.abs(),
    };
    let mut centers = Vec::new();
    if pitch.is_finite() && pitch > 0.0 {
        // Pinned to the start when trimmed there, else centered in its first cell.
        let pinned = repeat.trims[0] > 0.0 || matches!(repeat.distribute, Distribute::Count(_));
        let first = if pinned { 0.0 } else { pitch * 0.5 };
        let mut at = start + (first + repeat.phase).rem_euclid(pitch);
        while at <= end + 1e-9 && centers.len() < 100_000 {
            centers.push(at);
            at += pitch;
        }
    } else if room >= 0.0 {
        centers.push(start + room * 0.5);
    }
    // Each group's objects about its center.
    let mut all = Vec::with_capacity(centers.len() * count);
    for center in centers {
        for k in 0..count {
            #[allow(clippy::cast_precision_loss, reason = "a small index")]
            let shift = k as f64 * repeat.group_pitch.abs() - span * 0.5;
            all.push(center + shift);
        }
    }
    all
}

/// The decorations of a stroke of `style` on `geometry`: its repeats along every subpath and
/// its marks, in `stroke_color` when they take the stroke's, outlined within `tolerance`.
pub fn decorate(
    geometry: &Contours,
    style: &StrokeStyle,
    settings: &StrokeSettings,
    stroke_color: LinearRgba,
    tolerance: f64,
) -> Decorations {
    let mut decorations = Decorations::default();
    let width = style.width.abs();
    for contour in geometry.contours() {
        let line = Line {
            contour,
            length: contour.length(),
            width,
            stroke_color,
            tolerance,
        };
        for repeat in settings.repeats.iter().filter(|repeat| repeat.enabled) {
            let object = MarkObject {
                bend: MarkBend::Hard,
                ..repeat.object.clone()
            };
            let half = line.units(object.size, object.percent);
            let offset = line.units(repeat.offset, object.percent);
            for at in positions(repeat, line.length, half) {
                line.add(&object, at, repeat.side, offset, &mut decorations);
            }
        }
    }
    for mark in &settings.marks {
        let Some(contour) = geometry.contours().get(mark.contour) else {
            continue;
        };
        let line = Line {
            contour,
            length: contour.length(),
            width,
            stroke_color,
            tolerance,
        };
        let at = mark.at.clamp(0.0, 1.0) * line.length;
        for object in &mark.objects {
            let offset = line.units(mark.offset, object.percent);
            line.add(object, at, mark.side, offset, &mut decorations);
        }
    }
    // Fused objects and the stroke make one region: united, they wind once.
    if !decorations.fused.is_empty() {
        decorations.fused = koebi_geom::simplify(&decorations.fused, FillRule::NonZero);
    }
    decorations
}

#[cfg(test)]
mod tests {
    use super::*;

    fn straight(length: f64) -> Contours {
        let mut contours = Contours::default();
        contours.push(Contour::new(
            vec![Segment::line(Point::new(0.0, 0.0), Point::new(length, 0.0))],
            false,
        ));
        contours
    }

    fn style(width: f64) -> StrokeStyle {
        StrokeStyle {
            width,
            ..StrokeStyle::default()
        }
    }

    const BLACK: LinearRgba = LinearRgba::new(0.0, 0.0, 0.0, 1.0);

    #[test]
    fn a_repeat_lays_its_groups_out_by_pitch_count_and_gap() {
        let repeat = |distribute: Distribute| StrokeRepeat {
            distribute,
            ..StrokeRepeat::default()
        };
        // Every 8 along 40, centered in their cells: 4, 12, 20, 28, 36.
        let at = positions(&repeat(Distribute::Pitch(8.0)), 40.0, 1.0);
        assert_eq!(at, [4.0, 12.0, 20.0, 28.0, 36.0]);
        // Five over the line, its ends included.
        let at = positions(&repeat(Distribute::Count(5)), 40.0, 1.0);
        assert_eq!(at, [0.0, 10.0, 20.0, 30.0, 40.0]);
        // A gap of 2 between objects reaching 1 either way: a pitch of 4.
        let at = positions(&repeat(Distribute::Gap(2.0)), 16.0, 1.0);
        assert_eq!(at, [2.0, 6.0, 10.0, 14.0]);
        // Groups of three 2 apart, trimmed 5 at the start, measured to their outer edge.
        let grouped = StrokeRepeat {
            distribute: Distribute::Pitch(20.0),
            group_count: 3,
            group_pitch: 2.0,
            trims: [5.0, 0.0],
            trim_measure: TrimMeasure::Outside,
            ..StrokeRepeat::default()
        };
        let at = positions(&grouped, 60.0, 1.0);
        // The first group's outer edge at 5: its center at 5 + 2 + 1 = 8.
        assert_eq!(&at[..3], [6.0, 8.0, 10.0]);
    }

    #[test]
    fn fused_ticks_join_the_stroke_blended_and_cut_ones_stand_apart() {
        let line = straight(40.0);
        let mut settings = StrokeSettings {
            repeats: vec![StrokeRepeat::default()],
            ..StrokeSettings::default()
        };
        // Fence ticks across a stroke 2 wide: 25% long either way (0.5), 100% across (2).
        let fused = decorate(&line, &style(2.0), &settings, BLACK, 1e-3);
        assert_eq!(fused.fused.contours().len(), 5);
        let tick = fused.fused.contours()[0]
            .segments()
            .iter()
            .fold(Rect::EMPTY, |bounds, segment| {
                bounds.union(segment.bounds())
            });
        assert!(
            (tick.width() - 1.0).abs() < 1e-6 && (tick.height() - 4.0).abs() < 1e-6,
            "{tick:?}"
        );
        settings.repeats[0].object.mode = StampMode::Blend;
        let blended = decorate(&line, &style(2.0), &settings, BLACK, 1e-3);
        assert!(blended.fused.is_empty() && blended.drawn.len() == 5);
        settings.repeats[0].object.mode = StampMode::Subtract;
        assert_eq!(
            decorate(&line, &style(2.0), &settings, BLACK, 1e-3)
                .cut
                .len(),
            5
        );
    }

    #[test]
    fn marks_stamp_their_objects_on_their_side_and_cut_gaps() {
        let line = straight(40.0);
        // A circle on the left of the line, at its middle, 2 units across.
        let settings = StrokeSettings {
            marks: vec![StrokeMark {
                at: 0.5,
                side: MarkSide::Left,
                offset: 2.0,
                objects: vec![MarkObject {
                    size: 1.0,
                    percent: false,
                    ..MarkObject::default()
                }],
                ..StrokeMark::default()
            }],
            ..StrokeSettings::default()
        };
        let made = decorate(&line, &style(1.0), &settings, BLACK, 1e-3);
        let bounds = made.fused.bounds();
        // Left of a line going right is up, y down: centered at (20, -2).
        assert!((bounds.center().x - 20.0).abs() < 1e-6 && (bounds.center().y + 2.0).abs() < 1e-6);
        // A gap 6 long cuts the line, its ends' objects at 17 and 23.
        let gap = StrokeSettings {
            marks: vec![StrokeMark {
                at: 0.5,
                objects: vec![MarkObject {
                    shape: MarkShape::Gap,
                    size: 6.0,
                    percent: false,
                    gap_objects: [
                        vec![MarkObject {
                            size: 0.5,
                            percent: false,
                            ..MarkObject::default()
                        }],
                        vec![MarkObject {
                            size: 0.5,
                            percent: false,
                            ..MarkObject::default()
                        }],
                    ],
                    ..MarkObject::default()
                }],
                ..StrokeMark::default()
            }],
            ..StrokeSettings::default()
        };
        let made = decorate(&line, &style(1.0), &gap, BLACK, 1e-3);
        assert_eq!(made.cut.len(), 1);
        let opening = made.cut[0].0.bounds();
        assert!((opening.min.x - 17.0).abs() < 1e-6 && (opening.max.x - 23.0).abs() < 1e-6);
        let ends = made.fused.bounds();
        assert!((ends.min.x - 16.5).abs() < 1e-6 && (ends.max.x - 23.5).abs() < 1e-6);
        // A mark that asks for a dash adds an anchor to the dashes.
        let phased = StrokeSettings {
            marks: vec![StrokeMark {
                at: 0.25,
                phase: MarkPhase::Gap,
                ..StrokeMark::default()
            }],
            ..StrokeSettings::default()
        };
        assert_eq!(phased.dashes().anchors.len(), 1);
    }

    #[test]
    fn a_following_rectangle_bends_with_the_line() {
        // A quarter circle of radius 20: a rectangle following it spans its stretch.
        let k = 0.552_284_749_830_793_6 * 20.0;
        let mut arc = Contours::default();
        arc.push(Contour::new(
            vec![Segment::cubic(
                Point::new(20.0, 0.0),
                Point::new(20.0, k),
                Point::new(k, 20.0),
                Point::new(0.0, 20.0),
            )],
            false,
        ));
        let settings = StrokeSettings {
            marks: vec![StrokeMark {
                at: 0.5,
                objects: vec![MarkObject {
                    shape: MarkShape::Rectangle,
                    bend: MarkBend::Follow,
                    size: 8.0,
                    width: 1.0,
                    percent: false,
                    ..MarkObject::default()
                }],
                ..StrokeMark::default()
            }],
            ..StrokeSettings::default()
        };
        let made = decorate(&arc, &style(1.0), &settings, BLACK, 1e-3);
        // Every point of the band lies within 1 of the circle of radius 20, give or take the
        // cubic's own departure from the circle (0.03%).
        for contour in made.fused.contours() {
            for segment in contour.segments() {
                for t in [0.0, 0.5, 1.0] {
                    let radius = (segment.eval(t) - Point::ORIGIN).length();
                    assert!((radius - 20.0).abs() <= 1.01, "{radius}");
                }
            }
        }
    }
}
