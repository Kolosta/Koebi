//! The outline of a run: a stretch of a subpath stroked in one piece (a whole subpath, or one
//! dash), as the region between its two edges.
//!
//! The band of each piece lies between the edges at distances `lo` and `hi` along its normal.
//! The outline goes along the `lo` edge, round the end cap, back along the `hi` edge, round the
//! start cap. That orientation gives every part of the stroke a positive winding number, so
//! that the non-zero rule unites whatever overlaps.

use crate::{Contour, Contours, Point, Segment, Vector};

use super::offset::{self, CORNER, Chain, Corner, line_crossing, offset_point, rotated, tangent};
use super::{Cap, Join};

/// A smooth piece of a subpath: no corner inside, no inflection.
#[derive(Clone, Copy, Debug)]
pub(super) struct Piece {
    pub(super) segment: Segment,
    /// Arc lengths along the subpath where it starts and ends.
    pub(super) start: f64,
    pub(super) end: f64,
    /// The side of the path its band lies on: 1 toward the normal, -1 away from it, 0 for a
    /// centered stroke.
    pub(super) side: f64,
}

/// What a run needs from the style.
#[derive(Clone, Copy, Debug)]
pub(super) struct Params {
    pub(super) half: f64,
    /// How far an aligned band's middle lies from the path, toward its side.
    pub(super) offset: f64,
    pub(super) join: Join,
    pub(super) tolerance: f64,
    pub(super) snap: f64,
}

impl Params {
    /// The band of `piece`: its edges' distances along the normal, `lo < hi`.
    fn band(&self, piece: &Piece) -> [f64; 2] {
        let middle = piece.side * self.offset;
        [middle - self.half, middle + self.half]
    }
}

/// How one edge goes from a piece to the next.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Link {
    /// A straight line, or nothing: the edges meet (a smooth junction), or step across the
    /// path's normal (a smooth change of side), or turn so little that it does not matter.
    Line,
    /// The join, on the outer side of a corner.
    Join,
    /// Through the corner point: the inner side of a centered stroke, where the two pieces
    /// overlap.
    Pivot,
    /// Each edge ends where they cross: the inner side of an aligned stroke.
    Cut,
    /// A change of side at a corner, along its bisector: how the arriving edge gets there, and
    /// how the leaving one leaves it.
    Swap(Half, Half),
}

/// One edge's half of a change of side at a corner.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Half {
    /// Half the join, on the outer side.
    Join,
    /// Cut where the edge crosses the bisector, on the inner side.
    Cut,
    /// Through the corner point, which is on the bisector.
    Pivot,
}

/// A junction between two pieces of a run.
#[derive(Clone, Copy, Debug)]
struct Junction {
    corner: Corner,
    links: [Link; 2],
}

/// The scratch of runs, kept between them.
#[derive(Clone, Debug, Default)]
pub(super) struct Scratch {
    bands: Vec<[f64; 2]>,
    junctions: Vec<Junction>,
    /// Per piece and edge, the parameters it runs between.
    ranges: Vec<[(f64, f64); 2]>,
    edges: [Vec<Segment>; 2],
    points: Vec<Point>,
}

/// Where an edge starts or ends: a piece and a parameter on it.
type Position = (usize, f64);

impl Scratch {
    /// Adds the outline of the run `pieces` to `out`: a closed one makes two contours (its
    /// edges, one reversed), an open one a contour with its caps.
    pub(super) fn run(
        &mut self,
        pieces: &[Piece],
        closed: bool,
        caps: Cap,
        params: &Params,
        out: &mut Contours,
    ) {
        let count = pieces.len();
        if count == 0 {
            return;
        }
        self.bands.clear();
        self.bands
            .extend(pieces.iter().map(|piece| params.band(piece)));
        self.junctions.clear();
        let junctions = if closed { count } else { count - 1 };
        for index in 0..junctions {
            let next = (index + 1) % count;
            let junction = self.junction(&pieces[index], &pieces[next], index, next);
            self.junctions.push(junction);
        }
        self.cut(pieces);

        if closed {
            // Two loops: the lo edge, and the hi edge run backward. An edge whose offset folds
            // all round lies farther than every radius of curvature on its side: everything
            // on that side is within its distance, so it bounds nothing and is left out.
            for edge in 0..2 {
                let folded = pieces
                    .iter()
                    .zip(&self.bands)
                    .all(|(piece, band)| offset::folds_throughout(&piece.segment, band[edge]));
                if folded {
                    continue;
                }
                let first = (0, self.ranges[0][edge].0);
                let last = (count - 1, self.ranges[count - 1][edge].1);
                let mut segments = Vec::new();
                self.edge(pieces, edge, first, last, true, params, &mut segments);
                if edge == 1 {
                    reverse(&mut segments);
                }
                out.push(Contour::new(segments, true));
            }
            self.connectors(pieces, params, out);
            return;
        }

        let tilt = match caps {
            Cap::Butt { tilt } if tilt != 0.0 && tilt.is_finite() => tilt
                .clamp(-core::f64::consts::FRAC_PI_4, core::f64::consts::FRAC_PI_4)
                .tan(),
            _ => 0.0,
        };
        let mut ends = [((0, 0.0), (0, 0.0)); 2];
        for (edge, end) in ends.iter_mut().enumerate() {
            let first = (0, self.ranges[0][edge].0);
            let last = (count - 1, self.ranges[count - 1][edge].1);
            *end = if tilt == 0.0 {
                (first, last)
            } else {
                (
                    self.tilted(pieces, edge, tilt, false).unwrap_or(first),
                    self.tilted(pieces, edge, tilt, true).unwrap_or(last),
                )
            };
        }
        for (edge, (first, last)) in ends.into_iter().enumerate() {
            let mut segments = core::mem::take(&mut self.edges[edge]);
            segments.clear();
            self.edge(pieces, edge, first, last, false, params, &mut segments);
            self.edges[edge] = segments;
        }

        // The contour: the lo edge, the end cap, the hi edge backward, the start cap.
        let [lo, hi] = &self.edges;
        let lo_start = edge_start(lo, pieces, &self.bands, ends[0].0, 0);
        let mut segments = Vec::with_capacity(lo.len() + hi.len() + 8);
        let mut chain = Chain::new(&mut segments, lo_start, params.snap);
        for segment in lo {
            chain.push(*segment);
        }
        let last = &pieces[count - 1];
        let hi_end = edge_end(hi, pieces, &self.bands, ends[1].1, 1);
        cap(
            &mut chain,
            caps,
            tilt,
            CapEnd {
                vertex: last.segment.end(),
                forward: tangent(&last.segment, 1.0),
                band: self.bands[count - 1],
                to: hi_end,
                start: false,
            },
            params,
        );
        for segment in hi.iter().rev() {
            chain.push(segment.reversed());
        }
        let first = &pieces[0];
        cap(
            &mut chain,
            caps,
            tilt,
            CapEnd {
                vertex: first.segment.start(),
                forward: tangent(&first.segment, 0.0),
                band: self.bands[0],
                to: lo_start,
                start: true,
            },
            params,
        );
        chain.close();
        out.push(Contour::new(segments, true));
        self.connectors(pieces, params, out);
    }

    /// Where the band changes sides (inside and outside on an open subpath), the step across
    /// the path that joins the two: the band's middle, the spine, runs from one side to the
    /// other along the normal at an inflection, along the bisector at a corner, and the step
    /// is the centered stroke of that stretch of spine, joined to the spine on either side by
    /// the stroke's join.
    fn connectors(&self, pieces: &[Piece], params: &Params, out: &mut Contours) {
        let count = pieces.len();
        for (index, junction) in self.junctions.iter().enumerate() {
            let next = (index + 1) % count;
            let (before, after) = (self.bands[index], self.bands[next]);
            if before == after {
                continue;
            }
            let corner = junction.corner;
            // Where spines at distance c cross the bisector: c / cos(turn / 2) along it, the
            // cosine kept from vanishing at a turn back on itself.
            let along =
                rotated(corner.from, corner.turn * 0.5) / (corner.turn * 0.5).cos().max(0.25);
            let middle = |band: [f64; 2]| corner.point + along * band[0].midpoint(band[1]);
            let (from, to) = (middle(before), middle(after));
            let into = tangent(&pieces[index].segment, 1.0);
            let out_of = tangent(&pieces[next].segment, 0.0);
            let spine = [
                from - into * params.half,
                from,
                to,
                to + out_of * params.half,
            ];
            let mut steps = [None; 3];
            let mut length = 0.0;
            for (step, pair) in steps.iter_mut().zip(spine.windows(2)) {
                let span = pair[0].distance(pair[1]);
                if span > params.snap {
                    *step = Some(Piece {
                        segment: Segment::line(pair[0], pair[1]),
                        start: length,
                        end: length + span,
                        side: 0.0,
                    });
                    length += span;
                }
            }
            let steps: Vec<Piece> = steps.into_iter().flatten().collect();
            let centered = Params {
                offset: 0.0,
                ..*params
            };
            Self::default().run(&steps, false, Cap::Butt { tilt: 0.0 }, &centered, out);
        }
    }

    /// The junction between `arriving` and `leaving`, and how each edge goes through it.
    fn junction(&self, arriving: &Piece, leaving: &Piece, index: usize, next: usize) -> Junction {
        let (into, out_of) = (
            tangent(&arriving.segment, 1.0),
            tangent(&leaving.segment, 0.0),
        );
        let corner = Corner {
            point: arriving.segment.end(),
            from: into.perpendicular(),
            to: out_of.perpendicular(),
            turn: offset::turn(into, out_of),
        };
        let (before, after) = (self.bands[index], self.bands[next]);
        let symmetric = before[0] == -before[1];
        let links = core::array::from_fn(|edge| {
            let (d, d_after) = (before[edge], after[edge]);
            if corner.turn.abs() <= CORNER {
                Link::Line
            } else if before != after {
                let half = |d: f64| {
                    if d == 0.0 {
                        Half::Pivot
                    } else if corner.is_outer(d) {
                        Half::Join
                    } else {
                        Half::Cut
                    }
                };
                Link::Swap(half(d), half(d_after))
            } else if d == 0.0 {
                Link::Line
            } else if corner.is_outer(d) {
                Link::Join
            } else if symmetric {
                Link::Pivot
            } else {
                Link::Cut
            }
        });
        Junction { corner, links }
    }

    /// Finds where the cut edges end, sets every edge's range on every piece, and falls back to
    /// a pivot for a cut that is not found, or that would leave an edge with nothing of its
    /// piece.
    fn cut(&mut self, pieces: &[Piece]) {
        let count = pieces.len();
        self.ranges.clear();
        self.ranges.resize(count, [(0.0, 1.0); 2]);
        for index in 0..self.junctions.len() {
            let next = (index + 1) % count;
            let junction = self.junctions[index];
            for edge in 0..2 {
                let (d, d_after) = (self.bands[index][edge], self.bands[next][edge]);
                let (arriving, leaving) = (&pieces[index].segment, &pieces[next].segment);
                match junction.links[edge] {
                    Link::Cut => {
                        match offset::crossing(arriving, leaving, d, junction.corner.turn) {
                            Some((t, u)) => {
                                self.ranges[index][edge].1 = t;
                                self.ranges[next][edge].0 = u;
                            }
                            None => self.junctions[index].links[edge] = Link::Pivot,
                        }
                    }
                    Link::Swap(mut into, mut out_of) => {
                        let side = junction.corner.bisector_side();
                        if into == Half::Cut {
                            match line_crossing(arriving, (0.0, 1.0), d, &side, true) {
                                Some(t) => self.ranges[index][edge].1 = t,
                                None => into = Half::Pivot,
                            }
                        }
                        if out_of == Half::Cut {
                            match line_crossing(leaving, (0.0, 1.0), d_after, &side, false) {
                                Some(u) => self.ranges[next][edge].0 = u,
                                None => out_of = Half::Pivot,
                            }
                        }
                        self.junctions[index].links[edge] = Link::Swap(into, out_of);
                    }
                    Link::Line | Link::Join | Link::Pivot => {}
                }
            }
        }
        // Two cuts that cross on a piece leave its edge nothing: both fall back to pivots,
        // which is the band without cuts.
        for edge in 0..2 {
            for index in 0..count {
                let (start, end) = self.ranges[index][edge];
                if start < end {
                    continue;
                }
                self.ranges[index][edge] = (0.0, 1.0);
                let before = if index > 0 {
                    Some(index - 1)
                } else {
                    (self.junctions.len() == count).then_some(count - 1)
                };
                if let Some(before) = before {
                    self.uncut(before, edge, false);
                }
                if index < self.junctions.len() {
                    self.uncut(index, edge, true);
                }
            }
        }
    }

    /// Makes the cut of `edge` at junction `index` a pivot, on the arriving piece's side
    /// (`arriving`) or the leaving one's.
    fn uncut(&mut self, index: usize, edge: usize, arriving: bool) {
        let count = self.ranges.len();
        let next = (index + 1) % count;
        let link = &mut self.junctions[index].links[edge];
        match link {
            Link::Cut => {
                *link = Link::Pivot;
                self.ranges[index][edge].1 = 1.0;
                self.ranges[next][edge].0 = 0.0;
            }
            Link::Swap(into, _) if arriving && *into == Half::Cut => {
                *into = Half::Pivot;
                self.ranges[index][edge].1 = 1.0;
            }
            Link::Swap(_, out_of) if !arriving && *out_of == Half::Cut => {
                *out_of = Half::Pivot;
                self.ranges[next][edge].0 = 0.0;
            }
            _ => {}
        }
    }

    /// Where `edge` crosses the tilted cut of a butt cap, at the run's end or its start: the
    /// line through the path's end point, turned from its normal so that the edge at distance
    /// `d` reaches `d·tilt` beyond the end. `None` where the edge reaches beyond the end (it is
    /// extended by the cap), or when the cut is not found on the run.
    fn tilted(&self, pieces: &[Piece], edge: usize, tilt: f64, at_end: bool) -> Option<Position> {
        let count = pieces.len();
        let piece = if at_end {
            &pieces[count - 1]
        } else {
            &pieces[0]
        };
        let d = self.bands[if at_end { count - 1 } else { 0 }][edge];
        if d * tilt >= 0.0 {
            return None;
        }
        let t = if at_end { 1.0 } else { 0.0 };
        let vertex = piece.segment.eval(t);
        let forward = tangent(&piece.segment, t);
        let normal = forward.perpendicular();
        let outward = if at_end { forward } else { -forward };
        // Kept where non-negative: behind the tilted line.
        let side = move |p: Point| tilt * (p - vertex).dot(normal) - (p - vertex).dot(outward);
        // The cut lies within |d·tilt| of the end, along the path: look a little further.
        let reach = (d * tilt).abs() * 4.0 + 1e-9;
        for step in 0..count {
            let index = if at_end { count - 1 - step } else { step };
            let piece = &pieces[index];
            let behind = if at_end {
                pieces[count - 1].end - piece.end
            } else {
                piece.start - pieces[0].start
            };
            if behind > reach {
                return None;
            }
            let range = self.ranges[index][edge];
            let band_d = self.bands[index][edge];
            let far = if at_end { range.0 } else { range.1 };
            if side(offset_point(&piece.segment, far, band_d)) < 0.0 {
                continue;
            }
            return line_crossing(&piece.segment, range, band_d, side, at_end).map(|t| (index, t));
        }
        None
    }

    /// Writes `edge` from `first` to `last` into `segments`: each piece's offset, joined at the
    /// junctions; closed round the run's own junction when `closed`.
    #[allow(
        clippy::too_many_arguments,
        reason = "the parts of one run, all needed"
    )]
    fn edge(
        &mut self,
        pieces: &[Piece],
        edge: usize,
        first: Position,
        last: Position,
        closed: bool,
        params: &Params,
        segments: &mut Vec<Segment>,
    ) {
        let start = offset_point(&pieces[first.0].segment, first.1, self.bands[first.0][edge]);
        let mut chain = Chain::new(segments, start, params.snap);
        let mut index = first.0;
        loop {
            let from = if index == first.0 {
                first.1
            } else {
                self.ranges[index][edge].0
            };
            let to = if index == last.0 {
                last.1
            } else {
                self.ranges[index][edge].1
            };
            offset::offset(
                &mut chain,
                &pieces[index].segment,
                (from, to),
                self.bands[index][edge],
                params.tolerance,
                &mut self.points,
            );
            if index == last.0 {
                break;
            }
            self.link(&mut chain, pieces, index, edge, params);
            index += 1;
        }
        if closed {
            self.link(&mut chain, pieces, pieces.len() - 1, edge, params);
            chain.close();
        }
    }

    /// Takes `edge` through junction `index`, from the end of the arriving piece's edge (the
    /// cursor) to the start of the leaving one's.
    fn link(
        &self,
        chain: &mut Chain<'_>,
        pieces: &[Piece],
        index: usize,
        edge: usize,
        params: &Params,
    ) {
        let next = (index + 1) % pieces.len();
        let junction = &self.junctions[index];
        let corner = junction.corner;
        let (d, d_after) = (self.bands[index][edge], self.bands[next][edge]);
        let target = offset_point(&pieces[next].segment, self.ranges[next][edge].0, d_after);
        match junction.links[edge] {
            Link::Line | Link::Cut => {}
            Link::Join => corner.join(chain, d, params.join),
            Link::Pivot => chain.line_to(corner.point),
            Link::Swap(into, out_of) => {
                match into {
                    Half::Join => corner.join_to_bisector(chain, d, params.join),
                    Half::Pivot => chain.line_to(corner.point),
                    Half::Cut => {}
                }
                match out_of {
                    Half::Join => {
                        chain.line_to(corner.bisector_point(d_after, params.join));
                        corner.join_from_bisector(chain, d_after, params.join);
                    }
                    Half::Pivot => chain.line_to(corner.point),
                    Half::Cut => {}
                }
            }
        }
        chain.line_to(target);
    }
}

/// The first point of an edge written from `first`, or where it would start.
fn edge_start(
    edge: &[Segment],
    pieces: &[Piece],
    bands: &[[f64; 2]],
    first: Position,
    index: usize,
) -> Point {
    edge.first().map_or_else(
        || offset_point(&pieces[first.0].segment, first.1, bands[first.0][index]),
        Segment::start,
    )
}

/// The last point of an edge written up to `last`, or where it would end.
fn edge_end(
    edge: &[Segment],
    pieces: &[Piece],
    bands: &[[f64; 2]],
    last: Position,
    index: usize,
) -> Point {
    edge.last().map_or_else(
        || offset_point(&pieces[last.0].segment, last.1, bands[last.0][index]),
        Segment::end,
    )
}

/// Reverses a closed edge in place: the same loop, run the other way.
fn reverse(segments: &mut [Segment]) {
    segments.reverse();
    for segment in segments.iter_mut() {
        *segment = segment.reversed();
    }
}

/// One end of an open run, for its cap.
#[derive(Clone, Copy)]
struct CapEnd {
    /// The path's end point, and its direction of travel there.
    vertex: Point,
    forward: Vector,
    band: [f64; 2],
    /// Where the cap ends: the other edge's end.
    to: Point,
    /// The start of the run, rather than its end.
    start: bool,
}

/// The cap from the cursor, the end of one edge, to the end of the other. At the end of the
/// run it goes from the `lo` edge to the `hi` one; at the start, back from `hi` to `lo`.
fn cap(chain: &mut Chain<'_>, cap: Cap, tilt: f64, end: CapEnd, params: &Params) {
    let normal = end.forward.perpendicular();
    let outward = if end.start { -end.forward } else { end.forward };
    let half = params.half;
    let middle = end.vertex + normal * end.band[0].midpoint(end.band[1]);
    match cap {
        Cap::Butt { .. } if tilt != 0.0 => {
            // Each edge at distance d reaches d·tilt beyond the end; those that stop short
            // were cut there already.
            let (from_d, to_d) = if end.start {
                (end.band[1], end.band[0])
            } else {
                (end.band[0], end.band[1])
            };
            let reach = |d: f64| (d * tilt).max(0.0);
            let from = chain.cursor();
            chain.line_to(from + outward * reach(from_d));
            chain.line_to(end.to + outward * reach(to_d));
        }
        Cap::Butt { .. } => {}
        Cap::Square => {
            let from = chain.cursor();
            chain.line_to(from + outward * half);
            chain.line_to(end.to + outward * half);
        }
        Cap::Round => {
            // From one edge to the other through the point ahead, turning from x toward y.
            let from = if end.start { normal } else { -normal };
            chain.arc(middle, half, from, core::f64::consts::PI);
        }
        Cap::Taper { length } => {
            let length = if length > 0.0 && length.is_finite() {
                length
            } else {
                4.0 * half
            };
            chain.line_to(middle + outward * length);
        }
    }
    chain.line_to(end.to);
}

/// A run of no length, a dot: the caps alone, back to back, around `vertex`.
pub(super) fn dot(
    vertex: Point,
    forward: Vector,
    side: f64,
    cap: Cap,
    params: &Params,
    out: &mut Contours,
) {
    let normal = forward.perpendicular();
    let half = params.half;
    let middle = vertex + normal * (side * params.offset);
    let corners = |along: f64| {
        [
            middle - forward * along - normal * half,
            middle + forward * along - normal * half,
            middle + forward * along + normal * half,
            middle - forward * along + normal * half,
        ]
    };
    let mut segments = Vec::new();
    let polygon = |segments: &mut Vec<Segment>, points: &[Point]| {
        let mut chain = Chain::new(segments, points[0], params.snap);
        for point in &points[1..] {
            chain.line_to(*point);
        }
        chain.close();
    };
    match cap {
        Cap::Butt { .. } => return,
        Cap::Square => polygon(&mut segments, &corners(half)),
        Cap::Round => {
            let mut chain = Chain::new(&mut segments, middle - normal * half, params.snap);
            chain.arc(middle, half, -normal, core::f64::consts::TAU);
            chain.close();
        }
        Cap::Taper { length } => {
            let length = if length > 0.0 && length.is_finite() {
                length
            } else {
                4.0 * half
            };
            polygon(
                &mut segments,
                &[
                    middle - normal * half,
                    middle + forward * length,
                    middle + normal * half,
                    middle - forward * length,
                ],
            );
        }
    }
    out.push(Contour::new(segments, true));
}
