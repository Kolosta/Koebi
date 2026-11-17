//! Stroke outlines: a stroke turned into a fill, so that the canvas draws strokes with the
//! exact coverage of fills ([ADR 0029]).
//!
//! [`outline`] takes the [`Contours`] of a path and a [`StrokeStyle`], and gives contours to
//! fill with the non-zero rule ([`FillRule::NonZero`](crate::FillRule::NonZero)). They are
//! curves: lines, quadratic Béziers, exact arcs and the path's own segments.
//!
//! # The band
//!
//! A stroke is a band along its path: the points `Q(t) + d·n(t)`, where `n` is the unit normal
//! (the tangent turned a quarter from x toward y, which is the walker's right on screen) and `d`
//! runs between the band's two edges. A centered stroke of width `w` spans `-w/2..=w/2`. An
//! aligned one is shifted by its [`StrokeOffset`] to one side: the right or the left of the
//! walk, or the inside or the outside of the shape.
//!
//! # Exact, and united
//!
//! - Every part of the outline turns the same way (from x toward y), so the winding number is
//!   positive wherever the stroke is. Whatever overlaps is united by the non-zero rule: the inner
//!   side of a corner, a path crossing itself, dashes running into each other. A translucent
//!   stroke never darkens itself.
//! - The edges are offset curves, fitted with quadratic Béziers within the tolerance, checked
//!   along the curve's normals. Round joins and caps are exact arcs.
//! - Where the curve turns tighter than the band reaches on its concave side, the offset curve
//!   folds back on itself. There the curve is taken as a polyline within the tolerance, joined
//!   round: the band of a smooth curve, exact even then.
//! - Corners get the join on their outer side. On the inner side, a centered stroke pivots
//!   through the corner, where its two pieces overlap. An aligned stroke is cut where its two
//!   edges cross, so that nothing pokes out of the corner.
//!
//! # Sides
//!
//! Left and right are judged from the start, looking along the path. Inside and outside follow
//! the fill on closed subpaths (a hole's inside is the shape's). On open ones they follow the
//! curvature: inside is the concave side, a straight stretch keeps the side of the last turn,
//! and the stroke swaps sides where the path bends the other way, across its normal at an
//! inflection or along the bisector at a corner.
//!
//! Inside and outside on closed subpaths are exact whatever the shape's size against the band:
//! the region between two offsets of the shape, made by the booleans (a band wider than a thin
//! shape fills it, never more). Elsewhere, and for dashes, the band is cut where its edges cross
//! at corners, exact where the band is narrower than the parts of the path it runs along.
//!
//! # Dashes
//!
//! Dashes are laid along the path by arc length, and each one is stroked on its own, with real
//! caps and the joins of the corners it runs over. See [`Dashes`].
//!
//! [ADR 0029]: https://github.com/Kolosta/Koebi/blob/develop/docs/adr/0029-canvas-rendering-from-curves.md

mod dash;
mod offset;
mod run;
#[cfg(test)]
mod tests;

use crate::{Contour, Contours, Point, Segment, Vector, solve};

use run::{Params, Piece, Scratch};

/// Everything [`outline`] needs to know of a stroke.
#[derive(Clone, Debug, PartialEq)]
pub struct StrokeStyle {
    /// The width, in document units.
    pub width: f64,
    /// The side of the path the stroke lies on.
    pub align: Align,
    /// How far from the path an aligned stroke's middle lies. A centered stroke ignores it.
    pub offset: StrokeOffset,
    /// The ends of open subpaths and of dashes.
    pub cap: Cap,
    /// The corners.
    pub join: Join,
    /// The dashes: none with an empty pattern.
    pub dashes: Dashes,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            width: 1.0,
            align: Align::Center,
            offset: StrokeOffset::default(),
            cap: Cap::default(),
            join: Join::default(),
            dashes: Dashes::default(),
        }
    }
}

/// The side of the path a stroke lies on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Align {
    /// Across the path, half on each side.
    #[default]
    Center,
    /// The inside of the shape: the filled side of a closed subpath, the concave side of an
    /// open one.
    Inside,
    /// The outside of the shape.
    Outside,
    /// The left, looking along the path from its start: away from the normal.
    Left,
    /// The right, looking along the path from its start: toward the normal.
    Right,
}

/// How far from the path an aligned stroke's middle lies, toward its side. Half the width puts
/// one edge on the path; more moves the stroke away from it, and a negative offset sends it
/// across to the other side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StrokeOffset {
    /// A percentage of the stroke's width.
    Percent(f64),
    /// Document units.
    Units(f64),
}

impl Default for StrokeOffset {
    /// Half the width: the stroke beside the path, one edge on it.
    fn default() -> Self {
        Self::Percent(50.0)
    }
}

impl StrokeOffset {
    /// The offset in document units, for a stroke of `width`; 0 when not finite.
    pub fn resolve(self, width: f64) -> f64 {
        let offset = match self {
            Self::Percent(percent) => percent * 0.01 * width,
            Self::Units(units) => units,
        };
        if offset.is_finite() { offset } else { 0.0 }
    }
}

/// The end of an open subpath or of a dash.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cap {
    /// Cut across the end, and tilted by `tilt` radians (within ±45°) on solid strokes: the cut
    /// turns about the path's end point, so an edge at distance `d` from the path reaches
    /// `d·tan(tilt)` beyond the end. The same tilt reads the same way on both ends, each in its
    /// own direction of travel.
    Butt {
        /// The tilt, in radians; 0 for a square cut.
        tilt: f64,
    },
    /// A half disc around the end of the band's middle.
    Round,
    /// The band extended by half its width.
    Square,
    /// A triangle extending the band to a point.
    Taper {
        /// How far beyond the end the point is; 0 for twice the width.
        length: f64,
    },
}

impl Default for Cap {
    fn default() -> Self {
        Self::Butt { tilt: 0.0 }
    }
}

/// A corner's outer side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Join {
    /// The edges extended until they meet, unless the miter would be longer than `limit` times
    /// the width: then beveled, as in SVG.
    Miter {
        /// The longest miter, as a multiple of the width.
        limit: f64,
    },
    /// An arc around the corner.
    Round,
    /// A straight cut between the edges' ends.
    Bevel,
}

impl Default for Join {
    fn default() -> Self {
        Self::Miter { limit: 4.0 }
    }
}

/// The dashes of a stroke.
///
/// Without anchors the pattern runs from each subpath's start, `offset` into it, as in SVG.
/// Anchors center a dash or a gap at a point of a subpath; between two anchors, a whole number
/// of patterns is stretched to fit by the [`DashFit`], and beyond the outer ones the pattern
/// runs unscaled. On a closed subpath the anchors go round, and a dash over the start is one
/// dash.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dashes {
    /// Lengths along the path, dash and gap in turn, in document units. An odd count is
    /// repeated to make it even, as in SVG. A dash of length 0 is a dot, drawn by the caps.
    pub pattern: Vec<f64>,
    /// How far into the pattern each subpath starts, without anchors.
    pub offset: f64,
    /// How the pattern stretches between anchors.
    pub fit: DashFit,
    /// The dashes and gaps placed by hand.
    pub anchors: Vec<DashAnchor>,
}

/// How the pattern stretches to fit a whole number of times between two anchors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DashFit {
    /// Dashes and gaps alike.
    #[default]
    ScaleBoth,
    /// The dashes, keeping the gaps.
    AdaptDashes,
    /// The gaps, keeping the dashes.
    AdaptGaps,
}

/// A dash or a gap centered at a point of a subpath.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DashAnchor {
    /// The subpath: its index among the contours.
    pub contour: usize,
    /// Where on it, as a share of its length: 0 at the start, 1 at the end.
    pub at: f64,
    /// What is centered there.
    pub center: DashCenter,
    /// Its length; `None` for the pattern's first dash or first gap.
    pub size: Option<f64>,
}

/// What a [`DashAnchor`] centers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DashCenter {
    /// A dash.
    #[default]
    Dash,
    /// A gap.
    Gap,
}

/// The raw offset of the closed `contour` at distance `distance` along its normal: one loop,
/// the join on the outer side of corners, through the corner point on the inner side, and the
/// offset curve itself where it folds back. Its ears and swallowtails wind the other way, so
/// the points it winds positively are the contour's region offset (as Clipper offsets).
/// `None` for a contour of no length, or one whose offset folds all round.
pub(crate) fn raw_offset(
    contour: &Contour,
    distance: f64,
    join: Join,
    tolerance: f64,
) -> Option<Contour> {
    let mut pieces_of = Vec::new();
    pieces(contour, tolerance * 1e-3, false, &mut pieces_of);
    // Folded all round, the offset lies beyond every radius of curvature: everything on that
    // side is within the distance, so the region shrinks to nothing there (or a hole fills in).
    // Its raw offset would turn the right way round, as an antipodal copy: left out.
    if pieces_of
        .iter()
        .all(|piece| offset::folds_throughout(&piece.segment, distance))
    {
        return None;
    }
    let first = pieces_of.first()?;
    let mut segments = Vec::new();
    let mut points = Vec::new();
    let start = offset::offset_point(&first.segment, 0.0, distance);
    let mut chain = offset::Chain::new(&mut segments, start, tolerance * 1e-3);
    let count = pieces_of.len();
    for (index, piece) in pieces_of.iter().enumerate() {
        offset::raw(&mut chain, &piece.segment, distance, tolerance, &mut points);
        let next = &pieces_of[(index + 1) % count];
        let (into, out_of) = (
            offset::tangent(&piece.segment, 1.0),
            offset::tangent(&next.segment, 0.0),
        );
        let corner = offset::Corner {
            point: piece.segment.end(),
            from: into.perpendicular(),
            to: out_of.perpendicular(),
            turn: offset::turn(into, out_of),
        };
        if corner.turn.abs() > offset::CORNER {
            if corner.is_outer(distance) {
                corner.join(&mut chain, distance, join);
            } else {
                chain.line_to(corner.point);
            }
        }
        chain.line_to(offset::offset_point(&next.segment, 0.0, distance));
    }
    chain.close();
    Some(Contour::new(segments, true))
}

/// Fits `segment` with quadratics within `tolerance`, into `out`, as offsets are fitted. False
/// when it cannot be (a degenerate curve).
pub(crate) fn fit_quadratics(segment: &Segment, tolerance: f64, out: &mut Vec<Segment>) -> bool {
    offset::fit_curve(segment, tolerance, out)
}

/// The outline of the stroke of `contours` in `style`, within `tolerance` (document units) of
/// the exact one: contours to fill with the non-zero rule.
pub fn outline(contours: &Contours, style: &StrokeStyle, tolerance: f64) -> Contours {
    let mut out = Contours::default();
    Stroker::default().outline_into(contours, style, tolerance, &mut out);
    out
}

/// Outlines strokes, keeping its scratch from one to the next: what a document with many
/// strokes uses.
#[derive(Clone, Debug, Default)]
pub struct Stroker {
    /// The smooth pieces of the subpath being stroked.
    pieces: Vec<Piece>,
    /// The pieces of one dash.
    dash: Vec<Piece>,
    layout: dash::Layout,
    scratch: Scratch,
}

impl Stroker {
    /// A stroker with no scratch yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// [`outline`] into `out`, replacing what it held.
    pub fn outline_into(
        &mut self,
        contours: &Contours,
        style: &StrokeStyle,
        tolerance: f64,
        out: &mut Contours,
    ) {
        out.clear();
        let width = style.width;
        if !(width.is_finite() && width > 0.0) {
            return;
        }
        let tolerance = if tolerance.is_finite() && tolerance > 0.0 {
            tolerance
        } else {
            width * 1e-3
        };
        let join = match style.join {
            Join::Miter { limit } if !limit.is_finite() => Join::Miter { limit: 4.0 },
            join => join,
        };
        let params = Params {
            half: width * 0.5,
            offset: match style.align {
                Align::Center => 0.0,
                _ => style.offset.resolve(width),
            },
            join,
            tolerance,
            snap: tolerance * 1e-3,
        };
        // Inside and outside on closed subpaths are cut against the shape itself; the rest is
        // the band.
        let exact = matches!(style.align, Align::Inside | Align::Outside)
            && style.dashes.pattern.is_empty();
        for (index, contour) in contours.contours().iter().enumerate() {
            if !(exact && contour.is_closed()) {
                self.contour(contours, index, style, &params, out);
            }
        }
        if exact && contours.contours().iter().any(Contour::is_closed) {
            aligned(contours, style, &params, out);
        }
    }

    /// Strokes contour `index`.
    fn contour(
        &mut self,
        contours: &Contours,
        index: usize,
        style: &StrokeStyle,
        params: &Params,
        out: &mut Contours,
    ) {
        let contour = &contours.contours()[index];
        let tilted = matches!(style.cap, Cap::Butt { tilt } if tilt != 0.0);
        let measure = tilted || !style.dashes.pattern.is_empty();
        pieces(contour, params.snap, measure, &mut self.pieces);
        let closed = contour.is_closed();
        let Some(last) = self.pieces.last() else {
            // A subpath of no length: a dot, for the caps that draw one.
            if let (false, Some(first)) = (closed, contour.segments().first()) {
                let side = default_side(style.align);
                run::dot(
                    first.start(),
                    Vector::new(1.0, 0.0),
                    side,
                    style.cap,
                    params,
                    out,
                );
            }
            return;
        };
        let total = last.end;
        let interior = match (style.align, closed) {
            (Align::Inside | Align::Outside, true) => Some(interior_side(contours, index)),
            _ => None,
        };
        sides(&mut self.pieces, style.align, interior);

        let dashed = !style.dashes.pattern.is_empty()
            && self.layout.run(&style.dashes, index, total, closed);
        if !dashed {
            self.scratch
                .run(&self.pieces, closed, style.cap, params, out);
            return;
        }
        // Only the ends of the path itself are tilted.
        let cap = match style.cap {
            Cap::Butt { .. } => Cap::Butt { tilt: 0.0 },
            cap => cap,
        };
        let accuracy = params.tolerance * 1e-2;
        for &(from, to) in &self.layout.dashes {
            if closed && to - from >= total {
                self.scratch.run(&self.pieces, true, style.cap, params, out);
            } else if to <= from {
                let (point, forward, side) = locate(&self.pieces, from, accuracy);
                run::dot(point, forward, side, cap, params, out);
            } else {
                self.dash.clear();
                slice(&self.pieces, from, to.min(total), accuracy, &mut self.dash);
                if to > total {
                    slice(&self.pieces, 0.0, to - total, accuracy, &mut self.dash);
                }
                self.scratch.run(&self.dash, false, cap, params, out);
            }
        }
    }
}

/// The inside or outside stroke of the closed subpaths of `contours`, exactly: with `a..b`
/// the band's distances toward its side, the points of that side between distances `a` and
/// `b` of the outline, each distance measured as the join measures it, whatever the shape's
/// size against the band. With `O(s)` the shape grown by `s` (shrunk when negative), the
/// inside stroke is `O(-a) ∖ O(-b)` and the outside one `O(b) ∖ O(a)`.
fn aligned(contours: &Contours, style: &StrokeStyle, params: &Params, out: &mut Contours) {
    use crate::boolean::grown;
    use crate::{BooleanOp, FillRule, boolean, simplify};
    let mut closed = Contours::default();
    for contour in contours.contours().iter().filter(|c| c.is_closed()) {
        closed.push(contour.clone());
    }
    let shape = simplify(&closed, FillRule::NonZero);
    let offset_by = |distance: f64| grown(&shape, distance, params.join, params.tolerance);
    let (a, b) = (params.offset - params.half, params.offset + params.half);
    let (outer, inner) = if style.align == Align::Inside {
        (offset_by(-a), offset_by(-b))
    } else {
        (offset_by(b), offset_by(a))
    };
    let rule = FillRule::NonZero;
    for contour in boolean(&outer, rule, &inner, rule, BooleanOp::Subtract).contours() {
        out.push(contour.clone());
    }
}

/// The smooth pieces of `contour`: its segments split at inflections (and at a cubic's cusp),
/// without the ones of no length. A closed contour that does not end where it starts is closed
/// by a line. Their arc lengths are measured when `measure` (dashes and tilted caps need them);
/// otherwise the lengths of their control polygons stand in.
fn pieces(contour: &Contour, snap: f64, measure: bool, out: &mut Vec<Piece>) {
    out.clear();
    let mut length = 0.0;
    let mut push = |segment: Segment, out: &mut Vec<Piece>| {
        let (cuts, count) = inflections(&segment);
        let mut t0 = 0.0;
        for &t1 in cuts[..count].iter().chain(core::iter::once(&1.0)) {
            let part = segment.subsegment(t0, t1);
            let hull = part.hull_length();
            let part_length = if measure && hull > snap {
                part.arc_length()
            } else {
                hull
            };
            if hull > snap {
                out.push(Piece {
                    segment: part,
                    start: length,
                    end: length + part_length,
                    side: 0.0,
                });
                length += part_length;
            }
            t0 = t1;
        }
    };
    let segments = contour.segments();
    for segment in segments {
        if segment.is_finite() {
            push(*segment, out);
        }
    }
    if contour.is_closed()
        && let (Some(first), Some(last)) = (segments.first(), segments.last())
        && last.end().distance(first.start()) > snap
    {
        push(Segment::line(last.end(), first.start()), out);
    }
}

/// Where the curvature of `segment` changes sign, in `(0, 1)`, ascending: at most 4.
fn inflections(segment: &Segment) -> ([f64; 4], usize) {
    let mut cuts = [0.0; 4];
    let mut count = 0;
    let mut push = |t: f64| {
        if t > 1e-9 && t < 1.0 - 1e-9 && count < cuts.len() {
            cuts[count] = t;
            count += 1;
        }
    };
    match segment {
        // Conics (a rational quadratic) turn one way only, as quadratics do.
        Segment::Line(_) | Segment::Quad(_) => {}
        Segment::Rational(r) if r.degree() <= 2 => {}
        Segment::Cubic([p0, p1, p2, p3]) => {
            // B' ∝ a + 2bt + ct², B'' ∝ b + ct: their cross product is quadratic in t.
            let a = *p1 - *p0;
            let b = (*p2 - *p1) - a;
            let c = (*p3 - *p0) - (*p2 - *p1) * 3.0;
            for &t in solve::quadratic(b.cross(c), a.cross(c), a.cross(b)).as_slice() {
                push(t);
            }
        }
        Segment::Rational(_) => {
            const STEPS: u32 = 32;
            let sign = |t: f64| {
                segment
                    .derivative(t)
                    .cross(segment.second_derivative(t))
                    .signum()
            };
            let mut previous: (f64, f64) = (0.0, sign(0.0));
            for step in 1..=STEPS {
                let t = f64::from(step) / f64::from(STEPS);
                let current = sign(t);
                if current != previous.1 && current != 0.0 && previous.1 != 0.0 {
                    let (mut lo, mut hi) = (previous.0, t);
                    for _ in 0..60 {
                        let middle = lo.midpoint(hi);
                        if sign(middle) == previous.1 {
                            lo = middle;
                        } else {
                            hi = middle;
                        }
                    }
                    push(lo.midpoint(hi));
                }
                previous = (t, current);
            }
        }
    }
    cuts[..count].sort_unstable_by(f64::total_cmp);
    (cuts, count)
}

/// The side of a stroke that has nothing to go by: an open subpath that never turns.
const fn default_side(align: Align) -> f64 {
    match align {
        Align::Center => 0.0,
        Align::Inside | Align::Right => 1.0,
        Align::Outside | Align::Left => -1.0,
    }
}

/// Sets the side of each piece's band. `interior` is the filled side of a closed subpath (1:
/// toward the normal), for inside and outside.
fn sides(pieces: &mut [Piece], align: Align, interior: Option<f64>) {
    let fixed = match (align, interior) {
        (Align::Center | Align::Left | Align::Right, _) => Some(default_side(align)),
        (Align::Inside, Some(interior)) => Some(interior),
        (Align::Outside, Some(interior)) => Some(-interior),
        (Align::Inside | Align::Outside, None) => None,
    };
    if let Some(side) = fixed {
        for piece in pieces.iter_mut() {
            piece.side = side;
        }
        return;
    }
    // An open subpath: the curvature decides. Inside is the side a piece or a corner turns
    // toward, outside the other; a straight piece keeps the side of the last turn.
    let toward = |turn: f64| {
        if align == Align::Inside {
            turn.signum()
        } else {
            -turn.signum()
        }
    };
    let mut current = 0.0;
    for index in 0..pieces.len() {
        if index > 0 {
            let into = offset::tangent(&pieces[index - 1].segment, 1.0);
            let turn = offset::turn(into, offset::tangent(&pieces[index].segment, 0.0));
            if turn.abs() > offset::CORNER {
                current = toward(turn);
            }
        }
        let segment = &pieces[index].segment;
        let bend = segment.curvature(0.5);
        if bend != 0.0 && bend.abs() * segment.hull_length() > 1e-9 {
            current = toward(bend);
        }
        pieces[index].side = current;
    }
    // Before the first turn, the side of the first turn; with no turn at all, the default.
    let first = pieces
        .iter()
        .map(|piece| piece.side)
        .find(|side| *side != 0.0)
        .unwrap_or_else(|| default_side(align));
    for piece in pieces.iter_mut().take_while(|piece| piece.side == 0.0) {
        piece.side = first;
    }
}

/// The filled side of closed contour `index` among `contours` under the non-zero rule: 1
/// toward its normal, -1 away from it. A hole's filled side is away from the hole. Where the
/// fill does not tell (both sides filled, or neither), the contour's own orientation does.
fn interior_side(contours: &Contours, index: usize) -> f64 {
    let contour = &contours.contours()[index];
    let area = signed_area(contour);
    let own = if area >= 0.0 { 1.0 } else { -1.0 };
    let Some(probe) = contour.segments().first().map(|segment| segment.eval(0.5)) else {
        return own;
    };
    let others: i32 = contours
        .contours()
        .iter()
        .enumerate()
        .filter(|(other, _)| *other != index)
        .map(|(_, other)| other.winding(probe))
        .sum();
    // Toward the normal of a positive area lies its inside, wound +1; away from the normal of
    // a negative one, -1.
    let toward = others + i32::from(area > 0.0);
    let away = others - i32::from(area < 0.0);
    match (toward != 0, away != 0) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => own,
    }
}

/// The signed area enclosed by a closed contour: positive when its inside is toward its
/// normal.
fn signed_area(contour: &Contour) -> f64 {
    contour
        .segments()
        .iter()
        .map(|segment| {
            if let Segment::Line([a, b]) = segment {
                return 0.5 * a.to_vector().cross(b.to_vector());
            }
            // Only the sign of the whole is needed: a moderate accuracy does.
            let scale = segment.hull_length();
            let integrand = |t: f64| 0.5 * segment.eval(t).to_vector().cross(segment.derivative(t));
            solve::integrate(&integrand, 0.0, 1.0, scale * scale * 1e-6)
        })
        .sum()
}

/// Appends the parts of `pieces` between arc lengths `from` and `to`.
fn slice(pieces: &[Piece], from: f64, to: f64, accuracy: f64, out: &mut Vec<Piece>) {
    for piece in pieces {
        if piece.end <= from || piece.start >= to {
            continue;
        }
        let at = |length: f64| {
            piece
                .segment
                .param_at_length(length - piece.start, accuracy)
        };
        let t0 = if from > piece.start { at(from) } else { 0.0 };
        let t1 = if to < piece.end { at(to) } else { 1.0 };
        if t1 > t0 {
            out.push(Piece {
                segment: piece.segment.subsegment(t0, t1),
                start: from.max(piece.start),
                end: to.min(piece.end),
                side: piece.side,
            });
        }
    }
}

/// The point at arc length `at`, the direction of travel there, and the side of the band.
fn locate(pieces: &[Piece], at: f64, accuracy: f64) -> (Point, Vector, f64) {
    let piece = pieces
        .iter()
        .find(|piece| at < piece.end)
        .or(pieces.last())
        .copied();
    let Some(piece) = piece else {
        return (Point::ORIGIN, Vector::new(1.0, 0.0), 0.0);
    };
    let t = piece.segment.param_at_length(at - piece.start, accuracy);
    (
        piece.segment.eval(t),
        offset::tangent(&piece.segment, t),
        piece.side,
    )
}
