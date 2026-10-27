# 0018. Easing curves of several segments as a token kind

- Status: Accepted
- Date: 2026-10-27
- Deciders: @Kolosta
- Extends: [0008](0008-design-token-system.md), [0010](0010-design-token-interchange.md)

## Context

Components animate: folds of lists and trees open and close, scrollbars grow under the
pointer, and collapsible panels will slide. Each animation needs a duration and an easing
curve that themes and users can change. Tokens already have the `CubicBezier` kind, the Design
Tokens format's easing type, but one cubic Bézier cannot express a bounce, an overshoot that
settles back, or a hold: those need several segments. Components also hard-coded their easing
(a cubic ease-out in folds, an exponential follow in scrollbars).

## Decision

- A new kind, **`Curve`** (`value::Curve`): an easing curve through 2 to 8 keys from `(0, 0)`
  to `(1, 1)`, keys in time order, each segment eased by its own cubic Bézier. Values between
  the keys may leave `0..1` (overshoots, bounces). `Curve::ease(t)` evaluates it; a curve of
  one segment eases exactly like its Bézier (`Curve::from(bezier)`).
- The value is fixed-size and `Copy` (an array of keys and a length), so styles resolve and
  evaluate curves without allocating.
- Token tables write curves with `curve(&[key(x, y, bezier(...)), ...])`, checked while
  compiling like every literal; the design system checks overrides and imports with the same
  rule.
- **Interchange**: the Design Tokens format has no such type. Curves export as Koebi's own
  `"$type": "curve"`, an array of keys `{ "x", "y", "easing": [x1, y1, x2, y2] }` (the last key
  without easing). Tools that do not know the type cannot read these tokens; Koebi reads them
  back exactly. Saved state stores the keys as numbers, the kind appended after the existing
  ones so that older files keep their meaning.
- **Tokens**: primitive curves (`curve.linear`, `ease-in`, `ease-out`, `ease-in-out`,
  `settle`, `bounce`), semantic curves by intent (`animation.curve.enter`, `exit`, `expand`,
  `collapse`, `grow`, `move`), and component curves beside their durations
  (`list-row.expand.curve`, `list-row.collapse.curve`, `scrollbar.overlay.grow.curve`).
  `CubicBezier` tokens stay, for interchange with other tools.
- **Components** animate with a duration and a curve from tokens. A small `Tween` holds a
  value from 0 to 1 moving towards a target; a new target restarts from the current value,
  and the duration is the time of a whole change, a shorter change taking proportionally less.

## Consequences

- Themes and users change how every animation moves, not only how long it lasts.
- One evaluation path (`Curve::ease`) serves single Béziers and multi-segment curves.
- A curve editor can come later on top of the keys and their Béziers.
- Exported themes holding curves are Koebi-specific on those tokens.

## Alternatives considered

- **Only cubic Béziers**: standard, but no bounce, settle or hold.
- **Spring physics** (stiffness, damping): natural motion, but durations become implicit and
  the parameters are harder to edit and to compare across themes.
- **Arbitrary sampled curves** (a table of values): any shape, but large, not `Copy`, and hard
  to edit by hand or in a file.
