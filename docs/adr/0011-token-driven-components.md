# 0011. Token-driven components: typed styles and immediate-mode interaction

- Status: Accepted
- Date: 2026-10-25
- Deciders: @Kolosta

## Context

The design system (ADR 0008) resolves tokens; something must turn them into pixels. The
first components (button, toggle, checkbox, slider, drag value, panel, separator, color
swatch, theme card) exist to prove that the design system works in the application: a theme
switch, an override or a color vision change must restyle hundreds of components on the next
frame, and a slider must edit a token every frame without slowing anything down. The widget
tree, the layout engine and text are later milestones (#11, v0.3.0), so these components
must not lock in their design.

## Decision

`koebi-ui` draws components from **typed styles** and handles input in **immediate mode**.

- **Styles**: each component has a style struct of plain values (`ButtonStyle { background:
  StateColors, corner_radius: f32, .. }`) and a `resolve(&impl Resolve)` function that reads
  its `component.*` tokens. `Resolve` (in `koebi-design`) is implemented by the active theme
  (`DesignSystem`) and by any `View`, so one function styles the live interface and the
  previews of other themes. Every field comes from a token; components hold no literal.
- **Caching**: `Styles` gathers every style; `StyleCache` resolves it again only when the
  design system's revision changes. Drawing reads fields, never tokens.
- **Immediate mode**: a `Ui` lives for one frame. Components are its methods: they hit-test
  their rectangle, draw into the `Scene` and return a `Response` (hovered, pressed, held,
  clicked, released, changed, drag delta). `UiState` keeps the only state that outlives a
  frame: which widget holds the pointer, where it was pressed and the value it had then.
  The application owns the values (`&mut bool`, `&mut f32`).
- **Layout and text**: callers pass a rectangle per component; line icons stand in for
  labels.

## Consequences

- Switching themes, editing a token or changing the color vision costs one resolution of a
  few hundred values, then nothing until the next change; frames stay allocation-free.
- Components cannot drift from the tokens: a style field without a token does not compile
  into `resolve`, and pixel tests check that token values reach the screen.
- Adding a component means adding its tokens, a style and a `Ui` method (see
  [components](../design-system/components.md)).
- Components must not overlap: the first one drawn takes the press. The widget tree (#11)
  will bring hit testing in depth order, focus and keyboard navigation (#12), and may keep
  these styles while replacing how components are laid out and stored.

## Alternatives considered

- **Reading tokens while drawing** (`design.get` per component): each read is an array
  access, but styles of other themes (previews) would need a view per component, and every
  component would repeat its token list.
- **A retained widget tree now**: the right long-term structure for layout, focus and
  accessibility, but it needs the layout engine and text first; doing it here would decide
  #11 without its ADR.
- **Untyped style maps** (`HashMap<&str, Value>`): flexible, but loses compile-time checks
  of names and kinds, and allocates.
