//! The layout: areas tiling the grid, their lines, and splitting and resizing.

use std::fmt;

use crate::grid::{GRID, GridRect, Orientation, Side};
use crate::plan::{ClosePlan, Fill, JoinPlan};

/// An area of a layout. Identifiers are never reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AreaId(pub(crate) u32);

impl AreaId {
    /// An identifier read back from saved data.
    pub fn from_saved(id: u32) -> Self {
        Self(id)
    }

    /// The identifier as a number, for persistence.
    pub fn get(self) -> u32 {
        self.0
    }
}

/// Why a layout operation was refused. A refused operation changes nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LayoutError {
    /// No area has this identifier.
    UnknownArea(AreaId),
    /// An area cannot be joined with itself.
    SameArea,
    /// The areas do not share an edge.
    NotAdjacent,
    /// Leftover space that no row of neighbours covers exactly.
    CannotFill,
    /// The last area cannot be closed.
    LastArea,
    /// The split would make an area smaller than the minimum size.
    TooSmall,
    /// Rectangles that do not tile the grid.
    Invalid(String),
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownArea(area) => write!(f, "no area {}", area.0),
            Self::SameArea => f.write_str("an area cannot be joined with itself"),
            Self::NotAdjacent => f.write_str("the areas do not share an edge"),
            Self::CannotFill => f.write_str("no neighbours can fill the space left"),
            Self::LastArea => f.write_str("the last area cannot be closed"),
            Self::TooSmall => f.write_str("an area would be smaller than the minimum size"),
            Self::Invalid(reason) => write!(f, "invalid layout: {reason}"),
        }
    }
}

impl std::error::Error for LayoutError {}

/// Result of layout operations.
pub(crate) type Result<T> = std::result::Result<T, LayoutError>;

/// A run of edges at the same coordinate, along which areas meet on both sides: what moves
/// when the user drags a gutter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Line {
    /// Vertical lines have a constant x.
    pub orientation: Orientation,
    /// The constant coordinate.
    pub position: u32,
    /// Where the run starts along the line.
    pub start: u32,
    /// Where it ends.
    pub end: u32,
}

/// Areas tiling the grid: no overlap, no gap, no empty area.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    areas: Vec<(AreaId, GridRect)>,
    next: u32,
}

impl Default for Layout {
    fn default() -> Self {
        Self::new()
    }
}

impl Layout {
    /// One area covering the whole grid.
    pub fn new() -> Self {
        Self {
            areas: vec![(AreaId(0), GridRect::FULL)],
            next: 1,
        }
    }

    /// A layout from saved areas.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::Invalid`] unless the rectangles tile the grid with distinct
    /// identifiers.
    pub fn from_areas(areas: &[(u32, GridRect)]) -> Result<Self> {
        let layout = Self {
            areas: areas
                .iter()
                .map(|(id, rect)| (AreaId(*id), *rect))
                .collect(),
            next: areas.iter().map(|(id, _)| id + 1).max().unwrap_or(0),
        };
        layout.check().map_err(LayoutError::Invalid)?;
        Ok(layout)
    }

    /// The areas and their rectangles, in creation order.
    pub fn areas(&self) -> impl ExactSizeIterator<Item = (AreaId, GridRect)> + '_ {
        self.areas.iter().copied()
    }

    /// Number of areas.
    pub fn len(&self) -> usize {
        self.areas.len()
    }

    /// Always `false`: a layout has at least one area.
    pub fn is_empty(&self) -> bool {
        self.areas.is_empty()
    }

    /// The rectangle of `area`.
    pub fn rect(&self, area: AreaId) -> Option<GridRect> {
        self.areas
            .iter()
            .find(|(id, _)| *id == area)
            .map(|(_, rect)| *rect)
    }

    /// The area at (`x`, `y`).
    pub fn area_at(&self, x: u32, y: u32) -> Option<AreaId> {
        self.areas
            .iter()
            .find(|(_, rect)| rect.contains(x, y))
            .map(|(id, _)| *id)
    }

    fn require(&self, area: AreaId) -> Result<GridRect> {
        self.rect(area).ok_or(LayoutError::UnknownArea(area))
    }

    fn set_rect(&mut self, area: AreaId, rect: GridRect) {
        if let Some(entry) = self.areas.iter_mut().find(|(id, _)| *id == area) {
            entry.1 = rect;
        }
    }

    fn remove(&mut self, area: AreaId) {
        self.areas.retain(|(id, _)| *id != area);
    }

    /// Splits `area` along a line of `orientation` at `position`: the area keeps the part
    /// before the line (left or top), and the new area, returned, takes the rest. Both parts
    /// must be at least `min` grid units across.
    ///
    /// # Errors
    ///
    /// [`LayoutError::UnknownArea`], or [`LayoutError::TooSmall`] when a part would be
    /// smaller than `min` (or empty).
    pub fn split(
        &mut self,
        area: AreaId,
        orientation: Orientation,
        position: u32,
        min: u32,
    ) -> Result<AreaId> {
        let rect = self.require(area)?;
        let (start, end) = match orientation {
            Orientation::Vertical => (rect.x0, rect.x1),
            Orientation::Horizontal => (rect.y0, rect.y1),
        };
        let min = min.max(1);
        if position < start.saturating_add(min) || position.saturating_add(min) > end {
            return Err(LayoutError::TooSmall);
        }
        let (kept, added) = match orientation {
            Orientation::Vertical => (
                rect.with_edge(Side::Right, position),
                rect.with_edge(Side::Left, position),
            ),
            Orientation::Horizontal => (
                rect.with_edge(Side::Bottom, position),
                rect.with_edge(Side::Top, position),
            ),
        };
        let id = AreaId(self.next);
        self.next += 1;
        self.set_rect(area, kept);
        self.areas.push((id, added));
        Ok(id)
    }

    /// The areas touching `area` across its `side`.
    pub fn neighbours(&self, area: AreaId, side: Side) -> Vec<AreaId> {
        let Some(rect) = self.rect(area) else {
            return Vec::new();
        };
        self.areas_along(rect, side, &[area])
            .into_iter()
            .map(|(id, _)| id)
            .collect()
    }

    /// The areas, other than `excluded`, whose edge lies on `rect`'s `side` and that share a
    /// stretch of it.
    pub(crate) fn areas_along(
        &self,
        rect: GridRect,
        side: Side,
        excluded: &[AreaId],
    ) -> Vec<(AreaId, GridRect)> {
        let edge = rect.edge(side);
        let (start, end) = rect.span(side.orientation());
        self.areas
            .iter()
            .filter(|(id, _)| !excluded.contains(id))
            .filter(|(_, other)| {
                let (other_start, other_end) = other.span(side.orientation());
                other.edge(side.opposite()) == edge && other_start < end && start < other_end
            })
            .copied()
            .collect()
    }

    /// The line `area`'s `side` lies on, or `None` on the border of the grid.
    pub fn line(&self, area: AreaId, side: Side) -> Option<Line> {
        let rect = self.rect(area)?;
        let orientation = side.orientation();
        let position = rect.edge(side);
        if position == 0 || position == GRID {
            return None;
        }
        // Where areas end on the line, and where areas start on it.
        let (end_side, start_side) = match orientation {
            Orientation::Vertical => (Side::Right, Side::Left),
            Orientation::Horizontal => (Side::Bottom, Side::Top),
        };
        let spans = |edge_side: Side| {
            merge(
                self.areas
                    .iter()
                    .filter(|(_, other)| other.edge(edge_side) == position)
                    .map(|(_, other)| other.span(orientation))
                    .collect(),
            )
        };
        let boundary = intersect(&spans(end_side), &spans(start_side));
        let (own_start, _) = rect.span(orientation);
        boundary
            .into_iter()
            .find(|(start, end)| *start <= own_start && own_start < *end)
            .map(|(start, end)| Line {
                orientation,
                position,
                start,
                end,
            })
    }

    /// The areas on both sides of `line`: those ending on it, then those starting on it.
    pub fn line_areas(&self, line: &Line) -> (Vec<AreaId>, Vec<AreaId>) {
        let on = |side: Side| {
            self.areas
                .iter()
                .filter(|(_, rect)| {
                    let (start, end) = rect.span(line.orientation);
                    rect.edge(side) == line.position && line.start <= start && end <= line.end
                })
                .map(|(id, _)| *id)
                .collect::<Vec<_>>()
        };
        match line.orientation {
            Orientation::Vertical => (on(Side::Right), on(Side::Left)),
            Orientation::Horizontal => (on(Side::Bottom), on(Side::Top)),
        }
    }

    /// How far `line` can move while every area it touches stays at least `min` across.
    pub fn line_limits(&self, line: &Line, min: u32) -> (u32, u32) {
        let (before, after) = self.line_areas(line);
        let (start_side, end_side) = match line.orientation {
            Orientation::Vertical => (Side::Left, Side::Right),
            Orientation::Horizontal => (Side::Top, Side::Bottom),
        };
        let low = before
            .iter()
            .filter_map(|area| self.rect(*area))
            .map(|rect| rect.edge(start_side).saturating_add(min))
            .max()
            .unwrap_or(0);
        let high = after
            .iter()
            .filter_map(|area| self.rect(*area))
            .map(|rect| rect.edge(end_side).saturating_sub(min))
            .min()
            .unwrap_or(GRID);
        if low <= high {
            (low, high)
        } else {
            // The areas are already below the minimum: the line stays.
            (line.position, line.position)
        }
    }

    /// Moves `line` towards `position`, within its limits for `min`, and returns where it
    /// landed. Every edge on the line moves.
    pub fn move_line(&mut self, line: &Line, position: u32, min: u32) -> u32 {
        let (low, high) = self.line_limits(line, min);
        let position = position.clamp(low, high);
        let (before, after) = self.line_areas(line);
        let (end_side, start_side) = match line.orientation {
            Orientation::Vertical => (Side::Right, Side::Left),
            Orientation::Horizontal => (Side::Bottom, Side::Top),
        };
        for (areas, side) in [(before, end_side), (after, start_side)] {
            for area in areas {
                if let Some(rect) = self.rect(area) {
                    self.set_rect(area, rect.with_edge(side, position));
                }
            }
        }
        position
    }

    /// What joining `removed` into `kept` would do, without doing it: `kept` keeps the stretch
    /// of their shared edge both cover and extends across `removed`; the leftovers of both go
    /// to neighbours.
    ///
    /// # Errors
    ///
    /// [`LayoutError::UnknownArea`], [`LayoutError::SameArea`], [`LayoutError::NotAdjacent`]
    /// or [`LayoutError::CannotFill`].
    pub fn plan_join(&self, kept: AreaId, removed: AreaId) -> Result<JoinPlan> {
        crate::plan::join(self, kept, removed)
    }

    /// Joins `removed` into `kept` (see [`Layout::plan_join`]).
    ///
    /// # Errors
    ///
    /// See [`Layout::plan_join`]; nothing changes on error.
    pub fn join(&mut self, kept: AreaId, removed: AreaId) -> Result<JoinPlan> {
        let plan = self.plan_join(kept, removed)?;
        self.apply_join(&plan);
        Ok(plan)
    }

    /// Applies a join planned on this layout.
    pub(crate) fn apply_join(&mut self, plan: &JoinPlan) {
        self.set_rect(plan.kept, plan.result);
        self.remove(plan.removed);
        for fill in &plan.fills {
            self.apply_fill(fill);
        }
    }

    /// What closing `area` would do, without doing it.
    ///
    /// # Errors
    ///
    /// [`LayoutError::UnknownArea`], [`LayoutError::LastArea`] or
    /// [`LayoutError::CannotFill`].
    pub fn plan_close(&self, area: AreaId) -> Result<ClosePlan> {
        crate::plan::close(self, area)
    }

    /// Closes `area`: its neighbours along one side take its space (see
    /// [`Layout::plan_close`]).
    ///
    /// # Errors
    ///
    /// See [`Layout::plan_close`]; nothing changes on error.
    pub fn close(&mut self, area: AreaId) -> Result<ClosePlan> {
        let plan = self.plan_close(area)?;
        self.remove(area);
        self.apply_fill(&plan.fill);
        Ok(plan)
    }

    fn apply_fill(&mut self, fill: &Fill) {
        for (area, rect) in &fill.areas {
            self.set_rect(*area, *rect);
        }
    }

    /// Checks that the areas tile the grid: inside it, not empty, not overlapping, covering
    /// it all, with distinct identifiers.
    ///
    /// # Errors
    ///
    /// The first problem found, described.
    pub fn check(&self) -> std::result::Result<(), String> {
        if self.areas.is_empty() {
            return Err("no area".into());
        }
        let mut surface = 0_u64;
        for (index, (id, rect)) in self.areas.iter().enumerate() {
            if rect.is_empty() || rect.x1 > GRID || rect.y1 > GRID {
                return Err(format!("area {} has rectangle {rect:?}", id.0));
            }
            if id.0 >= self.next {
                return Err(format!("area {} is beyond the next identifier", id.0));
            }
            for (other_id, other) in &self.areas[index + 1..] {
                if other_id == id {
                    return Err(format!("area {} appears twice", id.0));
                }
                if rect.overlaps(*other) {
                    return Err(format!("areas {} and {} overlap", id.0, other_id.0));
                }
            }
            surface += rect.surface();
        }
        if surface == GridRect::FULL.surface() {
            Ok(())
        } else {
            Err(format!(
                "the areas cover {surface} of {}",
                GridRect::FULL.surface()
            ))
        }
    }
}

/// Sorts intervals and merges those that touch or overlap.
fn merge(mut spans: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    spans.sort_unstable();
    let mut merged: Vec<(u32, u32)> = Vec::with_capacity(spans.len());
    for (start, end) in spans {
        match merged.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }
    merged
}

/// The intersection of two sorted lists of disjoint intervals, merged.
fn intersect(a: &[(u32, u32)], b: &[(u32, u32)]) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        let start = a[i].0.max(b[j].0);
        let end = a[i].1.min(b[j].1);
        if start < end {
            out.push((start, end));
        }
        if a[i].1 < b[j].1 {
            i += 1;
        } else {
            j += 1;
        }
    }
    merge(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intervals_merge_and_intersect() {
        assert_eq!(
            merge(vec![(5, 8), (0, 2), (2, 4), (7, 9)]),
            [(0, 4), (5, 9)]
        );
        assert_eq!(
            intersect(&[(0, 4), (5, 9)], &[(1, 6), (8, 12)]),
            [(1, 4), (5, 6), (8, 9)]
        );
        // Touching pieces of an intersection form one run.
        assert_eq!(intersect(&[(0, 10)], &[(0, 5), (5, 10)]), [(0, 10)]);
        assert_eq!(intersect(&[(0, 4)], &[(4, 8)]), []);
    }
}
