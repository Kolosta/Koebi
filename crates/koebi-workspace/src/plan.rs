//! Plans of joins and closes: what each area becomes, for the operation and its preview.
//!
//! A kept area extends across the area it absorbs when its side along their shared edge lies
//! within the other's; the absorbed area's leftovers, and a closed area, are filled by the
//! neighbours along one of their sides, which must cover that side exactly so that every area
//! stays rectangular.

use crate::grid::{GridRect, Orientation, Side};
use crate::layout::{AreaId, Layout, LayoutError, Result};

/// Space handed to neighbours: the areas along one of its sides extend across it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fill {
    /// The space to fill.
    pub piece: GridRect,
    /// The side of the piece the filling areas come from.
    pub from: Side,
    /// The filling areas and their rectangles once extended.
    pub areas: Vec<(AreaId, GridRect)>,
}

/// What joining two areas does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JoinPlan {
    /// The area that stays.
    pub kept: AreaId,
    /// The area that goes.
    pub removed: AreaId,
    /// The kept area's rectangle after the join.
    pub result: GridRect,
    /// The removed area's leftovers, given to neighbours (none when the areas are aligned).
    pub fills: Vec<Fill>,
}

/// What closing an area does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosePlan {
    /// The area that goes.
    pub closed: AreaId,
    /// Who takes its space.
    pub fill: Fill,
}

pub(crate) fn join(layout: &Layout, kept: AreaId, removed: AreaId) -> Result<JoinPlan> {
    if kept == removed {
        return Err(LayoutError::SameArea);
    }
    let k = layout.rect(kept).ok_or(LayoutError::UnknownArea(kept))?;
    let r = layout
        .rect(removed)
        .ok_or(LayoutError::UnknownArea(removed))?;
    // The side of `kept` the removed area lies on.
    let side = Side::ALL
        .into_iter()
        .find(|side| {
            let (k_start, k_end) = k.span(side.orientation());
            let (r_start, r_end) = r.span(side.orientation());
            k.edge(*side) == r.edge(side.opposite()) && k_start < r_end && r_start < k_end
        })
        .ok_or(LayoutError::NotAdjacent)?;
    let orientation = side.orientation();
    let (k_start, k_end) = k.span(orientation);
    let (r_start, r_end) = r.span(orientation);
    if k_start < r_start || k_end > r_end {
        return Err(LayoutError::Unaligned);
    }
    // The kept area extends across the removed one, keeping its own extent along the edge.
    let result = k.with_edge(side, r.edge(side));
    // Leftovers before and after the kept area along the shared edge.
    let (before_side, after_side) = match orientation {
        Orientation::Vertical => (Side::Top, Side::Bottom),
        Orientation::Horizontal => (Side::Left, Side::Right),
    };
    let leftovers = [
        (r.with_edge(after_side, k_start), before_side),
        (r.with_edge(before_side, k_end), after_side),
    ];
    let excluded = [kept, removed];
    let mut fills = Vec::new();
    for (piece, opposite) in leftovers {
        if piece.is_empty() {
            continue;
        }
        // Opposite the kept area first, then towards it, then the far side.
        let preference = [opposite, side.opposite(), side];
        let fill = fill(layout, piece, &preference, &excluded).ok_or(LayoutError::CannotFill)?;
        fills.push(fill);
    }
    Ok(JoinPlan {
        kept,
        removed,
        result,
        fills,
    })
}

pub(crate) fn close(layout: &Layout, area: AreaId) -> Result<ClosePlan> {
    let rect = layout.rect(area).ok_or(LayoutError::UnknownArea(area))?;
    if layout.len() == 1 {
        return Err(LayoutError::LastArea);
    }
    // Sides with the fewest neighbours first: one neighbour taking the space is the tidiest.
    let mut sides = Side::ALL;
    sides.sort_by_key(|side| {
        let count = layout.areas_along(rect, *side, &[area]).len();
        if count == 0 { usize::MAX } else { count }
    });
    let fill = fill(layout, rect, &sides, &[area]).ok_or(LayoutError::CannotFill)?;
    Ok(ClosePlan { closed: area, fill })
}

/// The neighbours along the first side, in `preference` order, that cover that side of
/// `piece` exactly, each extended across the piece.
fn fill(
    layout: &Layout,
    piece: GridRect,
    preference: &[Side],
    excluded: &[AreaId],
) -> Option<Fill> {
    preference.iter().find_map(|side| {
        let mut along = layout.areas_along(piece, *side, excluded);
        let orientation = side.orientation();
        along.sort_by_key(|(_, rect)| rect.span(orientation));
        let (start, end) = piece.span(orientation);
        // The neighbours must lie within the side and cover it without gaps.
        let mut covered = start;
        for (_, rect) in &along {
            let (rect_start, rect_end) = rect.span(orientation);
            if rect_start != covered || rect_end > end {
                return None;
            }
            covered = rect_end;
        }
        if along.is_empty() || covered != end {
            return None;
        }
        let areas = along
            .into_iter()
            .map(|(id, rect)| {
                (
                    id,
                    rect.with_edge(side.opposite(), piece.edge(side.opposite())),
                )
            })
            .collect();
        Some(Fill {
            piece,
            from: *side,
            areas,
        })
    })
}
