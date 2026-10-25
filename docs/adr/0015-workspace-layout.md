# 0015. Workspace layout: areas tiling an integer grid, edited by lines, joins that fill gaps

- Status: Accepted
- Date: 2026-10-25
- Deciders: @Kolosta

## Context

Koebi's workspace works like Blender's: rectangular areas tile the window, each area shows a
stack of editors (tabs), and the user resizes, splits, joins, swaps and closes areas and
docks editors by dragging. Resizing moves every edge aligned with the dragged one (the band
drawn over a gutter shows them), and joining two areas of different lengths must keep every
area rectangular by handing the leftovers to neighbours. The previous application drew these
previews but computed some results wrongly; the model must be exact and testable without
drawing.

## Decision

A new crate, `koebi-workspace`, holds the model (pure Rust, depends on `koebi-core` only).

- **Grid**: areas are rectangles with integer coordinates on a grid of `GRID` = 65,536 units
  per window side. The window maps the grid to pixels by rounding each coordinate, so shared
  edges map to the same pixel and areas tile the window exactly at any size. Resizing the
  window changes nothing in the model.
- **Invariant**: the areas tile the grid: no overlap, no gap, no empty area. Every operation
  is planned first and applied only if valid, so a refused operation changes nothing; tests
  check the invariant after every operation.
- **Lines**: the gutter between two areas belongs to a line: the longest run of edges at the
  same coordinate along which areas meet on both sides. Moving a line moves every edge on it,
  within the minimum sizes of the areas it touches.
- **Join**: a kept area absorbs a neighbour if its side along their shared edge lies within the
  neighbour's side; it then extends across the neighbour. The neighbour's leftover parts (at
  most two) are each filled by the areas along one of their sides, which must cover that side
  exactly; the side opposite the kept area is preferred, then the side towards it, then the far
  side. Otherwise the join is refused, and a preview shows it as not allowed.
- **Close**: a closed area is filled the same way by the neighbours along one of its sides,
  preferring the side with the fewest neighbours.
- **Editors**: each area holds a stack of editor identifiers with an active one. Docking an
  editor into a tab bar or onto a side (which splits the target in half) closes its area when
  it leaves it empty, or is refused if that area cannot close. Editor states live outside the
  model, keyed by editor identifier; area and editor identifiers are never reused.

## Consequences

- Any rectangular tiling is possible, including those no binary split tree can express, and
  every result is exact: no floating-point drift, no gaps.
- Previews (join, close, split) come from the same plans the operations apply.
- The model does not enforce minimum sizes when the window shrinks: areas scale with it.
- Persistence (#77) stores the grid rectangles and stacks as they are.

## Alternatives considered

- **Binary split tree** (VS Code, Graphite): simple resizing of one split, but lines spanning
  several splits, joins of unaligned areas and pinwheel layouts cannot be expressed.
- **Blender's vertices and edges**: the same expressiveness, with more bookkeeping (shared
  vertices, edge flags) and floating-point positions scaled at every resize.
- **Coordinates in pixels**: exact only at one window size; scaling accumulates rounding
  errors.
