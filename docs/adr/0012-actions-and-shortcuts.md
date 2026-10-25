# 0012. Actions and shortcuts: an action-centred keymap resolved through pointer contexts

- Status: Accepted
- Date: 2026-10-25
- Deciders: @Kolosta

## Context

Koebi's interface is driven like Blender's: most work happens with shortcuts, and they act on
what is under the pointer. The keymap must support several shortcuts per action, remapping
and restoring them, conflicts that are impossible within a zone but allowed across zones,
keyboard and mouse triggers (press, release, click, double click, repeat, drags in a
direction from a distance, wheel, pinch), and modifiers that tell left and right apart. No
dependency may be added (ADR 0003), and the keymap must stay maintainable as the application
grows to hundreds of actions.

## Decision

`koebi-input` holds an **action-centred** keymap, independent of SDL and of drawing.

- **Actions** are `static ActionDef`s declared in the module of their feature (identifier,
  label, description, context, default shortcuts) and gathered in `ActionGroup`s: the code
  triggers actions, users choose their shortcuts, and no flat keymap file exists.
- **Shortcuts** are values: a trigger (key, button, wheel, pinch, pointer move, with *when*:
  press, release, click, double click, repeat, drag with a direction and an optional
  distance) and a modifier rule (each of Ctrl, Shift, Alt, GUI up, down on a side, or
  ignored; a platform-aware primary modifier). Their text form (`Ctrl+Shift+Z`,
  `LeftMouse drag north 12`) is used by saved keymaps.
- **Contexts**: each input event comes with the chain of contexts under the pointer,
  innermost first (application, area, editor, top bar, content...), provided by the UI's
  zones. The first context with a matching shortcut wins. A button or key held keeps the
  chain of its press, so drags and releases stay with the zone where they started.
- **Conflicts**: two shortcuts overlap when one input could trigger both; overlapping
  shortcuts are refused within a context and allowed across contexts. Every change is checked,
  and the built-in defaults are checked when the keymap is built.
- **Overrides** replace an action's shortcuts. An action is modified when its shortcuts
  differ from its defaults (as a set), whatever the edits that led there. Saved overrides
  list modified actions only; loading reports what it cannot apply and never leaves a
  conflict.
- **Gestures**: presses become drags once the pointer moves beyond the drag threshold (or the
  shortcut's own distance), and are then no longer clicks; key double clicks use the
  double-click time; key repeats come from the system's repeat rate; mouse double clicks
  from the platform's click count. Thresholds and delays are design tokens
  (`semantic.input.*`).

## Consequences

- Features own their shortcuts next to their code; the shortcut editor lists groups and
  restores them per action, per group or all at once.
- The UI must report the zones under the pointer (#12) so that the chain of contexts is right;
  modal operations take over the input (`Keymap::cancel_presses`) and resolve their own
  confirm and cancel actions in their own contexts, so those keys are remappable too.
- Rotation and smart-zoom gestures are not reported by SDL yet; they will join the triggers
  when the platform reports them.

## Alternatives considered

- **Shortcut-centred map** (shortcut → action): several shortcuts per action and per-action
  reset become awkward, and conflicts are implicit.
- **One keymap file for the whole application**: hard to edit and to review, far from the
  code it serves.
- **Event handlers in components**: no remapping, no conflict detection, no shortcut editor.
- **Modifiers without sides** (Blender's): simpler, but cannot express left and right keys,
  which the previous application supported.
