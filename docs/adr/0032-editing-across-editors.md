# 0032. Editing across editors: one session, operations as live transactions, tool visuals where the pointer is

- Status: Accepted
- Date: 2026-11-17
- Deciders: @Kolosta
- Settles #17 with ADR 0028

## Context

Several editors show one document at once:
- Viewports;
- Outliners in their two modes;
- Properties, Fill, Stroke and Strokes & Fills;
- the Node Graph;
- Info.

They can live in several windows (ADR 0019 and the floating windows).

The maintainer's requirements:

- **An action in one editor shows in every editor at once:** an object moved or resized in one
  Viewport moves in all of them during the drag, not after it.
- **What is not in the document shows only in the editor that has the pointer:** shortcuts
  hints, tool visuals, handles, hover highlights.
- **It is done cleanly**, without workarounds.
- **Every tool of Carto returns, with its visuals, behavior and shortcuts:**
  - modes (Object, Edit, Line Mark);
  - selection, 2D cursor, shapes, curves (Bézier, NURBS, poly, circles, free), follow curve;
  - modal move, rotate and scale with axis constraints, pivots and orientations;
  - snapping in all its modes;
  - the Add menu and context menus;
  - Edit mode and its handle types;
  - line marks;
  - the N panel;
  - the cursor that wraps at the editor's edges during a drag.

What Carto taught (inventory in `docs/development/feature-parity.md`):

- **Its sharing worked:** one edit context shared by all editors, per-viewport cameras and
  overlays, gesture ownership by the viewport that started it.
- **Previews leaked:** shape and pen previews were temporary document nodes, and blanking the
  pen's stroke was document-wide, so other viewports lost that stroke. A frame-scoped style
  rewrote the document during rendering.
- **Ownership was incomplete.** Every viewport drained the modal motion accumulator with *its
  own* zoom, and confirm keys were polled in every viewport.
- **The wrapped cursor**, once rebuilt on SDL's relative mouse mode with a virtual cursor, had
  no drift by construction.

## Decision

### The session

`koebi_doc::Session` holds one open document and everything shared about editing it:

- the document and its history (ADR 0028);
- **the selection:**
  - selected and active objects;
  - the last active object, kept after a deselect for the N panel;
  - Edit mode's selected elements: subpath, anchor, point or in/out handle;
  - Line Mark mode's selected marks;
- **the edit context:**
  - mode (Object, Edit, Line Mark);
  - pivot point (bounding box center, 2D cursor, individual origins, median point, active
    element);
  - transform orientation (global, local, view, cursor, parent);
  - snapping (on/off, targets, base, affected transforms, increments and precisions);
  - the 2D cursor (position, rotation, placed or not);
  - the default fill and stroke for new objects;
  - the last tool per mode and each multi-tool's variant.

Editors read the session each frame. The interface is immediate mode, so every editor shows the
current state without notifications. Caches (the evaluated program, the Outliner's flattened
rows) key on the document's and the selection's revisions.

### Editor state

Each editor instance owns what is about its view only:
- Viewports: camera, unit, rulers, overlay options, print preview, N panel;
- Outliners: mode, filters, search, folds, synchronized viewport;
- Node Graphs: framing and breadcrumb.

It is saved with the workspace (`apps/koebi/src/saved.rs`). Two Viewports never share a camera.
Two Outliners never share a filter.

### Operations and transactions

- **Every change goes through an operation** (Blender's *operator*): a named type with an
  action id for the keymap (ADR 0012), a poll (can it run here), and either a one-shot `run` or
  a *modal* lifecycle.
- **A modal operation opens a transaction** on the session's document. Every update applies
  real edits through `koebi_doc::ops`, so the document *is* in its intermediate state:
  - every editor, every window and the file preview see it, during the gesture;
  - confirming commits the transaction as one undo step;
  - cancelling reverts it exactly.

  There are no preview nodes and no frame-scoped document rewrites.
- **A transaction is owned by the editor that started the operation.** Only that editor:
  - receives its input (motion, axis keys, confirm and cancel);
  - integrates the motion, with its own camera and snapping radius;
  - draws its tool visuals.

  Another editor's hover or keys cannot reach it. This fixes Carto's accumulator and
  confirm-key bugs by construction.

### What draws where

| What | Where |
|---|---|
| The document, including an open transaction's edits | every editor |
| The selection: object outlines, origin dots, Edit mode's points and handles of selected anchors, selected marks, the 2D cursor; selected rows in Outliners; selected nodes in Node Graphs | **every editor**, as in Blender and Carto: session state, not tool visuals |
| Previews of the action in progress: a modal operation's pivot cross, axis line and dashed guides (scale, rotate); the transform cursor; snapping candidates and the engaged marker; construction lines; the pen's rubber band; the line mark ghost; box selection | **the editor the action runs from**, only |
| Hover highlights; a tool's cursor and glyph | the hovered editor's |
| Status-bar shortcut hints, tooltips | the hovered editor's |

This is the maintainer's rule, from Blender and Carto: what the session holds shows everywhere,
what an action in progress shows only where the action runs.

### Tools and modes

- **A tool** is a type implementing `Tool`:
  - id, icon, name, shortcut;
  - the modes it belongs to, its variants (a multi-tool), and the settings it shows in the
    header;
  - how it answers press, drag, release, hover and keys, usually by starting operations;
  - what it draws in the owning editor.
- **The tool palette** is a column of tool buttons, with a variant menu on right-click and the
  bound shortcut in tooltips. It is built from the registry for the current mode, with module
  tools below the core ones (ADR 0033).
- **Modes** decide the tools, keymap contexts (ADR 0012's context chain:
  window → editor → region → mode → tool) and what clicking selects.
- **Carto's tool set returns**, listed in `feature-parity.md` with each tool's shortcuts and
  visuals.
- **Carto's missing behaviors are added:**
  - click-drag to move objects and Edit mode points;
  - Shift and Alt constraints on shape drags;
  - snapping while drawing;
  - typed numeric input during modal operations;
  - the operator redo panel of the legacy build.

### The wrapped cursor

During a modal transform, a pan, a slider drag (already done for number fields) or a handle
drag:
- the platform layer switches to relative mouse mode, with the system's pointer speed kept;
- the operation integrates raw motion into a virtual cursor, unbounded, in the owner's view;
- the drawn cursor is the virtual cursor folded into the owner editor's rect: it teleports at
  the edges while guides keep pointing past them;
- on confirm or cancel, the real pointer is warped once to the drawn position.

Cursor glyphs (move, rotate tangent to the motion, scale along the guide) come from the icon
set, drawn by the interface at the cursor's size and tinted from tokens.

### Picking modes

Pickers in the property editors (an object, a motif, an operand) are *pick operations*:
- their eyedropper cursor works in any Viewport or Outliner, and in any window;
- clicking delivers the object.

This reuses the eyedropper operation of the color picker (#211), generalized from pixels to
objects.

## Consequences

- **Synchronization is a property of the model:** the transaction makes the document
  everyone's truth during a gesture, and ownership confines input and tool visuals. No editor
  special-cases another.
- **Live edits cost an evaluation per frame** of the changed part only (ADR 0028's revisions).
  Transform-only edits re-place prepared geometry without preparing it again (ADR 0029).
- **Tests:**
  - operations are unit-tested headless: run, modal updates, commit, cancel, undo;
  - multi-editor tests open two Viewports and an Outliner in one harness, drive a gesture in
    one, and check what each draws.
- **Every tool is a separate type** with its tests, so the Carto parity list becomes a checklist
  of types.

## Alternatives considered

- **Preview nodes and overlays for in-progress edits (Carto):** previews must then be added to
  every editor separately, leak into other views, and need cleanup paths.
- **Copy-on-write document per gesture**, swapped in on commit: other editors would not see the
  gesture live, which the requirement asks for.
- **Message passing between editors:** a second channel of truth beside the document. Immediate
  mode plus revisions already gives consistency for free.
