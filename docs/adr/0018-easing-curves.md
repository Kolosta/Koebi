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
  the keys may leave `0..1` (overshoots, bounces), within -4..=4 like the easings' control
  points, so that every valid curve eases to finite values. `Curve::ease(t)` evaluates it; a
  curve of one segment eases exactly like its Bézier (`Curve::from(bezier)`). The last key's
  easing is never used: `Curve::new` makes it linear, so that curves that ease the same
  compare equal in every encoding.
- The curve is fixed-size and `Copy` (an array of keys and a length, about 200 bytes), so
  styles read and evaluate curves without allocating. Being large, it is stored by pointer:
  token tables hold `&'static Curve` literals and run-time values an `Arc<Curve>`, so that
  every other token value stays 24 bytes.
- Token tables write curves with `curve(&[key(x, y, bezier(...)), ...])`, checked while
  compiling like every literal; the design system checks overrides and imports with the same
  rule.
- **Interchange**: the Design Tokens format has no such type. Curves export as Koebi's own
  `"$type": "curve"`, an array of keys `{ "x", "y", "easing": [x1, y1, x2, y2] }` (the last key
  without easing). Tools that do not know the type cannot read these tokens; Koebi reads them
  back exactly. Saved state stores the keys as numbers under a new kind code (8), after the
  existing ones, which are pinned: older files keep their meaning, and an older build reading a
  newer file reports it damaged and falls back to its backup.
- **Tokens**: primitive curves (`curve.linear`, `ease-in`, `ease-out`, `ease-in-out`,
  `settle`, `bounce`), semantic curves by intent (`animation.curve.enter`, `exit`, `expand`,
  `collapse`, `grow`, `move`), and component curves beside their durations
  (`list-row.expand.curve`, `list-row.collapse.curve`, `scrollbar.overlay.grow.curve`).
  `CubicBezier` tokens stay, for interchange with other tools.
- **Components** animate with a duration and a curve from tokens. A small `Tween` holds a
  value from 0 to 1 moving towards a target; a new target restarts from where the running
  animation is at that moment, so that a target moving every frame is followed, and the
  duration is the time of a whole change, a shorter change taking proportionally less. Folds
  hold their openness within 0 and 1: an overshooting curve stays at the ends there.

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
