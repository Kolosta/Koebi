# 0013. Layered hit testing against the previous frame, and zones as shortcut contexts

- Status: Accepted
- Date: 2026-10-25
- Deciders: @Kolosta
- Supersedes: the hit testing item of [ADR 0011](0011-token-driven-components.md)

## Context

ADR 0011 gave each component its rectangle and let the first component drawn take a press,
so components could not overlap. The workspace needs overlap everywhere: menus over areas,
tab bars appearing over top bars, drag previews over everything, buttons inside zones that
accept shortcuts on their whole surface. A click must reach what is on top and nothing
behind it, and the shortcuts that apply must be those of the zones under the pointer.

An immediate-mode frame cannot know, while it draws a component, whether a later component
will cover it.

## Decision

- Each frame registers **regions** in drawing order: widgets (`Ui::interact`), **zones**
  (`Ui::zone`, carrying a `koebi_input::Context` and nesting) and blockers (`Ui::block`, the
  background of a floating panel). Regions live on **layers** (`Layer::BASE`, `FLOATING`,
  `OVERLAY`) and are clipped to the enclosing clip rectangle.
- The next frame resolves the pointer and the press against the **previous frame's regions**:
  the highest layer wins, then the region registered last. Only the widget on top is hovered,
  pressed or clicked; zones and blockers on top shield what lies under them.
- The chain of zones under the pointer, innermost first, is the chain of shortcut contexts
  (`UiState::contexts_at`); the cursor comes from the widget holding the pointer, or from the
  region on top and its zones (`UiState::cursor`).

## Consequences

- Components may overlap; menus and previews need no special casing to block clicks.
- Interaction lags the layout by one frame. Frames follow input, so the previous frame is the
  layout on screen when the input arrives; tests lay a frame out before interacting.
- A frame drawn twice (after a change of the design system) resolves against its first pass,
  which has the same layout.

## Alternatives considered

- **Hit testing while drawing, in reverse order**: components would have to be drawn back to
  front and handled front to back, which immediate mode cannot do in one pass.
- **A retained tree with z-order**: the right structure eventually (#11), not needed for
  correct hit testing.
- **Each component checking every later rectangle**: quadratic, and blind to components drawn
  later in the frame.
