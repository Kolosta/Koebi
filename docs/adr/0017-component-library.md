# 0017. Component library: theme values declared once, overridable per call, configurable behaviours

- Status: Accepted
- Date: 2026-10-26
- Deciders: @Kolosta
- Extends: [0011](0011-token-driven-components.md)

## Context

`koebi-ui` must become a real component library, the way Blender's interface toolkit sits on
its GPU module: the same base could serve applications with very different needs. Developers
must be able to configure every component on every point (sizes, colors, borders, content,
when an action fires and what each event does), while Koebi itself styles everything from its
design tokens and lets users edit them.

Where theme values enter components can be decided three ways:

1. **A library below the tokens**: components carry built-in default values, replaced by token
   values, then by the values a call gives.
2. **One whole**: each component is declared together with its tokens; its defaults *are* the
   resolved tokens, with no second layer that overwrites them.
3. Something in between.

The rendering side must also give components what they need (layers for floating content,
radii per corner, borders per side), and families of components must restyle together (all
control borders, but not container borders).

## Decision

**One whole, declared once, overridable at the call.**

- **Every visual property is a component token.** A component's style (`ButtonStyle`, ...) is
  plain data resolved from its `component.*` tokens by its `resolve` function; the library
  holds no literal default. A property exists even when the current theme does not use it: a
  button has a border width (0 in the dark theme) and border colors for every state, so a
  theme or a user can give it a border.
- **Families through semantic roles.** Component tokens alias semantic tokens that name a role
  shared by several components (control backgrounds, control borders, container borders,
  primary text...). Editing a role restyles its family and leaves the others; overriding a
  component token detaches that component from its family.
- **Per-call overrides.** Components are builders, configured by a call and then shown in a
  rectangle. They take every property as a `Prop`: a token of the design system or a literal
  value (`.corner_radius(semantic::RADIUS_M)`, `.corner_radius(0.0)`), state by state for the
  properties that change with the state. `Styles` keeps a snapshot of every token's value, so a
  token given at a call is read in constant time, and previews of other themes keep working.
  Every style field is public: an application can also build styles from values.
- **Roles and colors.** A control's colors come from its tone: a variant naming its role
  (default, accent, positive, negative, info, neutral, notice) and an optional color family
  (blue, cyan, green, yellow, orange, red, magenta, purple). Families derive their hovered,
  pressed and disabled colors from three base colors per family and three shading tokens, so
  that every variant has every family without a token per combination. Toggle buttons of the
  default role have a look of their own (`component.toggle.*`), as in Blender's theme.
- **States.** Colors vary with the state: normal, hovered, pressed, selected, selected and
  hovered, disabled. Components are tested in every state, and with every numeric token at
  both ends of its range, not only with the current theme.
- **Behaviours are options.** Responses report the events of the frame (hovered, pressed,
  held, released, clicked, double clicked, secondary press, drag); options choose when the
  main action fires (release by default, press, double press), whether it repeats while held,
  and whether the control toggles a value. Defaults follow the [actions and shortcuts
  conventions](../development/actions-and-shortcuts.md).
- **Content.** Controls with content show, in order, a leading icon, a label or an icon, a
  status dot and a trailing icon, each optional; a label too long is cut with an ellipsis.
- **Sizes.** Components are one interface unit high by default (`semantic.size.control-height`
  through their own height token), never lower than their content; their width fits their
  content within optional minimum and maximum widths, or is fixed. They measure themselves, so
  that callers and layout helpers can place them, and at their own size they never draw
  outside their rectangle.
- **Groups.** Fused controls (button groups) lie on a grid whose cells may span several
  columns or rows. Neighbours share one border, colored by the cell of highest state priority,
  and only the group's outer corners are rounded.
- **Reuse elsewhere.** Components depend on their style structs, filled by `resolve` functions
  that read token paths. Another application provides its own token set and themes for the same
  paths, or builds styles from values.

## Consequences

- One source of truth: the token tables. Themes, overrides, color vision and user edits reach
  every component without code, and no default can drift from its token.
- The token set grows: each component declares all its properties per state (button variants
  and color families, group borders, labels and descriptions...).
- Per-call overrides cost a lookup in the snapshot or nothing; steady frames still read plain
  fields and allocate nothing.
- Adding a component means its tokens (aliasing roles), its style and `resolve`, its builder,
  and tests across states and extreme values (see [components](../design-system/components.md)).
- Older components drawn by `Ui` methods move to builders as they are reworked.

## Alternatives considered

- **A library below the tokens with literal defaults**: every default written twice (in code
  and in tokens), the literal layer dead in Koebi, and the two drift apart unnoticed.
- **Reading tokens while drawing**: no styles of other themes for previews, and every
  component repeats its token list.
- **Style sheets with selectors (cascading)**: flexible, but untyped, resolved at run time per
  component, and hard to test.
- **A token per variant and family combination**: 56 sets of state colors to keep consistent
  by hand; deriving the states from three colors per family keeps families editable in one
  place.
