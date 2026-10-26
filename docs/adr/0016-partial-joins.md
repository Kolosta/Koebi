# 0016. Joins of partly aligned areas

- Status: Accepted
- Date: 2026-10-26
- Deciders: @Kolosta
- Supersedes: the join item of [0015](0015-workspace-layout.md)

## Context

ADR 0015 lets a kept area absorb a neighbour only when the kept area's side along their
shared edge lies within the neighbour's: two areas whose edges only partly meet, or a kept
area longer than the neighbour, could not be joined, neither from a corner drag nor from the
area menu (the item was disabled). Users expect, as in recent Blender versions, to join any
two areas that touch: the joined area takes the stretch they share, and the neighbours close
the gaps.

## Decision

- **Join**: the kept area keeps only the stretch of the shared edge that both areas cover and
  extends across the removed area. It shrinks along the edge where the two areas are not
  aligned and grows in the direction of the join.
- The leftovers of both areas outside that stretch (at most one at each end, belonging to
  whichever area reaches further) are each filled by the areas along one of their sides, which
  must cover that side exactly: the side away from the joined area first, then the side facing
  the other area's column, then the far side.
- An area may fill one leftover only, and the outcome is checked against the tiling invariant
  before the plan is accepted. Otherwise the join is refused (`LayoutError::CannotFill`), and
  the preview shows it as not allowed.
- The preview shows the part of the kept area that stays lighter, the part of the removed area
  it takes darker, every leftover darker still, and outlines the joined area. The arrow cursor
  points the way the join goes, from the kept area towards the removed one.

## Consequences

- Any two adjacent areas can be joined whenever their neighbours can close the gaps; the area
  menu offers the join in those cases.
- A join may resize areas on both sides of the joined pair, not only the removed area's
  neighbours; the preview shows every part that changes hands.
- `LayoutError::Unaligned` is gone: the only reasons left are unknown or identical areas, areas
  that do not touch, and leftovers no neighbours can take.

## Alternatives considered

- **Keep the 0015 rule**: simple, but it refuses joins users perform every day in Blender.
- **Grow the kept area to the union and push neighbours away**: neighbours would have to move
  lines they do not share, changing areas far from the join.
- **Split leftovers among several sides**: more joins would succeed, but the result would be
  hard to predict from the preview; one side per leftover keeps it readable.
