# Actions and shortcuts

Rules every feature follows, so that Koebi behaves the same everywhere. The engine is
`koebi-input` ([ADR 0012](../adr/0012-actions-and-shortcuts.md)).

## Declaring an action

Declare each action once, as a `static ActionDef` in an `actions` module of its feature, and
list it in the feature's `ActionGroup`:

```rust
use koebi_core::input::Key;
use koebi_input::{ActionDef, ActionGroup, Context, Shortcut};

/// The canvas of the viewport.
pub const CANVAS: Context = Context::new("viewport.canvas");

/// Frames the whole document.
pub static VIEW_ALL: ActionDef = ActionDef {
    id: "viewport.view-all",
    label: "Frame all",
    description: "Zooms and pans so that the whole document is visible.",
    context: CANVAS,
    defaults: &[Shortcut::key(Key::Home)],
};

pub static GROUP: ActionGroup = ActionGroup { name: "Viewport", actions: &[&VIEW_ALL] };
```

- **Identifiers** are `feature.action` in lowercase kebab case. Saved keymaps refer to them:
  never rename one; deprecate it and add another.
- **Contexts** are `feature.zone`. Pick the narrowest zone where the action makes sense: the
  canvas rather than the whole viewport editor, the gutter rather than the area.
- **Standard shortcuts** (undo, copy, save) use `Shortcut::primary()`: Command on Apple
  platforms, Ctrl elsewhere. Do not take shortcuts the system reserves (Alt+F4, Command+Q,
  Command+Tab).
- A shortcut is unique within its context; an inner context may reuse an outer context's
  shortcut on purpose (the canvas's Ctrl+Z undoes view changes, the application's undoes
  edits). `Keymap::new` refuses conflicting defaults, and a test builds the keymap.

## When actions run

| Trigger | Use it for | Examples |
|---|---|---|
| Press | Immediate and harmless actions, starting an operation from a key, context menus | select with the keyboard, open the area menu on right press, start grabbing with G |
| Click (release without drag) | Buttons, and items whose button also drags | push a button, select a list row when the same button drags rows |
| Release | The end of something held | stop a temporary tool |
| Drag | Operations driven by the pointer | resize a gutter, move an editor, box select |
| Double click | Opening, renaming, framing | open an item, rename a tab |
| Repeat (press held) | Stepping only | arrows in lists, +/- on values |

Buttons react on release, so that moving away before releasing cancels them; pressing a
button never runs its action.

## Modal operations

Operations that last (drags, previews) follow the same conventions:

- **Escape, right click and Delete cancel** and restore *exactly* the previous state (Delete
  only when the operation does not delete). **Enter, and the release or click that ends the
  drag, confirm.**
- **Shift gives precision**: slower movement (one tenth), finer steps, one more decimal.
  **Ctrl gives coarse steps or snapping**: bigger increments, one decimal less, snapping to
  the grid or to round values. Both apply as soon as they are pressed, also during the
  operation.
- The operation takes over the input (`Keymap::cancel_presses`), so the release that ends a
  drag is not a click for what is under the pointer.
- Previews show the result before it happens; nothing changes until the confirmation.

## Hover context

Shortcuts act on what is under the pointer, not on a focused window: the chain of zones under
the pointer (application, area, editor, top bar, content, element) decides which actions
apply, innermost first. Text fields, once they exist, take the keyboard while editing.

## Wiring

- The application builds one keymap from every feature's group (`GROUPS` in
  `apps/koebi/src/actions.rs`): list a new group there, and a test checks that its defaults do
  not conflict.
- For each input event, the keymap receives the chain of `UiState::chain_at`: the contexts
  that grab the keyboard, then the zones under the pointer. Triggered actions reach the next
  frame through `Input::trigger`.
- Components read them with `Ui::triggered(&ACTION)`. A component that must keep the keyboard
  whatever the pointer is over (an open menu, an operation in progress) calls
  `Ui::grab(CONTEXT)` every frame it lasts.

## Input settings

Thresholds and delays are design tokens, so that a theme or the user can change them:
`semantic.input.drag-threshold`, `semantic.input.double-click.time`,
`semantic.input.tooltip.delay` (and `-short`, `-stationary`), `semantic.input.hover.delay`,
`semantic.input.repeat.delay` and `semantic.input.repeat.interval`. Never hard-code a
duration or a distance of interaction.

## Conventions to come

Blender conventions Koebi will adopt with the components that need them:

- typing a number during a modal operation enters the value; X, Y or Z (or the middle
  button) constrains a move to an axis;
- over a field: Backspace restores its default, Ctrl+C and Ctrl+V copy and paste its value,
  Ctrl+wheel steps it, double click edits it, Alt applies an edit to every selected item;
- dragging across toggles toggles them all;
- Ctrl+Space maximizes the area under the pointer; F3 searches every action by name.
