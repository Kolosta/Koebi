# 0024. Layout: rectangles handed out in immediate mode, no retained tree

- Status: Accepted
- Date: 2026-11-11
- Deciders: @Kolosta
- Replaces the goal of issue #11 (a retained widget tree with a layout engine)

## Context

Issue #11 planned a retained widget tree with a flex-like layout engine. Since then the toolkit
has taken another shape:

- **Immediate mode.** Components are values built and shown each frame
  ([ADR 0011](0011-token-driven-components.md), [ADR 0017](0017-component-library.md)). They are
  hit-tested against the previous frame ([ADR 0013](0013-layered-hit-testing.md)), so a
  component needs its rectangle when it is shown, and nothing else.
- **Every component measures itself.** `Button::size`, `Checkbox::size`, `Dropdown::size`,
  `NumberField::size` and `TextField::size` ask nothing of the frame being built.
- **Layout helpers exist where the interface needed them:**
  - `PropertyGroup` rows: label and input columns, groups, vectors;
  - `ButtonGroup` tracks: `Fit`, `Fixed`, `Share`, spans, shrinking;
  - `BarLayout`: left, right and centred items;
  - `CollapsiblePanel`, which measures its content's height;
  - the workspace's own tiling of areas (ADR 0015).
- **Ad hoc arithmetic remains.** The rest of the code computes rectangles by hand, for
  instance a search field and a capture field sharing a row with a square button. That is
  where widths go wrong and pixels get lost.

The question asked: is a layout engine needed, how does Blender do it, what does it cost, and
what helps developers?

### How Blender lays out its interface

- **Built and solved every redraw.** Blender's `uiLayout` is built anew at each redraw of a
  region, by C or Python code that reads as immediate (`layout.row()`, `col.prop(...)`). Each
  call appends a layout item or a button to the region's `uiBlock`, a tree kept only until the
  block is drawn.
- **Two passes.** `UI_block_layout_resolve`:
  1. estimates sizes bottom-up (`ui_item_size`, `ui_layout_estimate`);
  2. hands out positions top-down (`ui_item_layout`). A row splits its width among its items
     in proportion to their estimates, with `scale_x`, `ui_units_x`, `alignment` and fixed
     sizes, then the buttons receive their rectangles.
- **Panels** take the height their layout resolved to; regions scroll on the total.
- **State is matched back.** Buttons are matched with those of the previous block
  (`ui_but_update_from_old_block`) to keep what the user is doing: an active button, an edit
  under way.

So Blender has a layout engine, but not a retained tree: it rebuilds the tree on each redraw,
which costs an allocation per item, and matches it with the previous one. It needs the two
passes because its buttons cannot measure themselves before they are placed in a block.

## Decision

No retained widget tree and no deferred layout engine. Components keep receiving their
rectangle when they are shown, and `koebi-ui` provides the solver the interface already uses,
in a form any code can call.

- **`koebi_ui::layout::row` and `column`** split a rectangle into `N` rectangles along an
  axis, `gap` apart, with the tracks of button groups:
  - `Track::Fixed(size)` is that size;
  - `Track::Share(weight)` takes a share of what the others leave;
  - `Track::Fit` takes what its content asks.

  When there is too little room, fitting tracks give way first. `row_fitting` and
  `column_fitting` take the size each `Fit` track's content asks: the component's `size()`,
  measured first.
- It is the same solver `ButtonGroup` sizes its tracks with, so a group and the row around it
  agree on every rule.
- **The patterns stay as they are:**
  - property rows (`PropertyGroup`), bars (`BarLayout`) and measured panels
    (`CollapsiblePanel`), each with its own conventions;
  - the workspace's tiling (ADR 0015).
- **Height measured on the previous frame.** A container whose height depends on its content
  (a panel, a settings page) takes the height measured on the previous frame, and asks for one
  more frame when it changed, as today. The interface settles in one frame, the same delay as
  hit testing.

## Consequences

- No allocation and no tree per frame. Layout costs a few additions per row (`N` is at most
  16, on the stack).
- Developers write `let [name, shortcut, clear] = layout::row(rect, gap, [Track::Fit,
  Track::Fit, Track::Fixed(row)]);` instead of computing widths by hand. Components measure
  themselves when a track should fit them.
- Layouts that need the size of content drawn later (a row whose width depends on a panel
  below it) are not expressible in one pass. None exists in Koebi's interface. Should one
  appear, the content can be measured first, since every component's `size()` is free of side
  effects.
- Issue #11 is re-scoped to these helpers and closed by them.

## Alternatives considered

- **A retained widget tree with diffing**, the original goal of #11:
  - ownership of widget state moves into the tree, against the immediate-mode API of ADRs 0011
    and 0017;
  - invalidation and diffing must be written and kept right;
  - every component would be rewritten. All for layouts the current code already expresses.
- **Blender's deferred two-pass layout:**
  - each frame records items and closures, resolves, then draws;
  - in Rust the deferred drawing closures borrow application state mutably while the layout
    resolves, and the recorded items allocate;
  - its benefit, measuring before placing, is already available because components measure
    themselves.
- **A flexbox or grid engine** (Taffy, Morphorm): a dependency (ADR 0003), and a model built for
  documents more than for dense editor panels.
