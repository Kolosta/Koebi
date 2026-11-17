# 0033. Modules specialize the application; the splash screen starts documents and modules

- Status: Proposed
- Date: 2026-11-17
- Deciders: @Kolosta

## Context

The maintainer wants *modules* back.

- A module is like a plugin: it uses the main application's tools and editors, presents them
  differently, and adds features of its own for a particular use.
- Two modules start empty: **IOF** (orienteering maps after the International Orienteering
  Federation's ISOM) and **Typographie** (type design).

Carto's modules (`src/Modules/ModuleAPI.h`) were compiled into the application. A module:
- declared an id, name, description, icon and version;
- built its workspace layout and restricted which editors a zone may show;
- switched core features on or off (*capabilities*: the core Add menu, pages, Edit mode, the
  curve tool, forced locks, a locked Outliner tree, the document unit, the color mode, a symbol
  scale);
- set the default page size and seeded new documents;
- replaced the Add menu, added N-panel tabs, tools and editors, and drew on the canvas;
- vetoed Outliner reparenting;
- received Shift + numpad digit sequences;
- ran per frame.

A document stored its module id, and opening the file activated the module again. External
plugin DLLs were planned, never built.

The splash screen started everything:
- *New File* presets (General, Layout, Data) built a workspace;
- *Recent Files* reopened documents;
- *Modules* opened a new document in a module.

It had no scrolling and no thumbnails, and its fixed height clipped long lists.

## Decision

### Modules

A module is a type implementing `Module`, compiled into the application and registered at
start-up. Every hook has a default, so an empty module only states its identity:

| Hook | What it gives |
|---|---|
| `info` | id, name, description, icon, version |
| `capabilities` | the core features it keeps or turns off, as a typed struct: the Add menu's core items, pages, modes, tools, forced property locks, Outliner structure locked, document unit and color mode (pinned or not) |
| `workspace` | its layout preset, a tree of splits and editors, and the editors a zone may show |
| `new_document` | the default page (size, unit) and the document it starts from |
| `add_menu` | items for the Add menu, beside or instead of the core ones |
| `tools` | tools for the palette, below the core tools (ADR 0032) |
| `panels` | tabs for the Viewport's N panel |
| `editors` | editors of its own, registered like the core ones |
| `nodes` | node kinds and node groups for the graph (ADR 0028) |
| `policies` | vetoes on structural operations (reparenting, deleting managed objects) |
| `sequences` | Shift + numpad digit runs |
| `on_activate`, `on_deactivate`, `on_frame` | lifecycle |

- **Services:** a module reaches the application through `ModuleHost`:
  - the session and its operations, so module edits share the undo history;
  - placement of an object that follows the pointer until a click;
  - drawing with the core pen in a style of its own;
  - offscreen previews of objects;
  - the info log.

  It never touches editors' internals.
- **Activation:** the active module is part of the document (`META`, ADR 0031), so opening a
  file restores it. A file whose module is not available opens in the Classic mode, and saving
  keeps the module's id.
- **External modules** (dynamic libraries) are not part of this decision: Rust has no stable ABI.
  They will need their own ADR when wanted.
- **IOF and Typographie** arrive as empty modules:
  - their identity;
  - a workspace preset;
  - a default page: A4 landscape in millimetres for IOF, a 1000 × 1000 em square in points
    for Typographie;
  - the capabilities that frame them: the CMYK color mode pinned for IOF; no pages for
    Typographie.

  Their real content comes in their own issues. Carto's symbol catalogue (124 ISOM elements,
  30 print layers with their CMYK values), its theme tools and its symbol viewer are recorded
  in `docs/development/feature-parity.md` for then.

### Workspaces and new documents

- **Presets:** a workspace preset is a named layout tree.
  - General: Viewport over Timeline, beside Outliner over Properties. This is today's default.
  - Layout: a large Viewport beside Outliner over Properties.
  - Data: a large Outliner beside a small Viewport over Properties and Info.
  - One per module.
- **A new document** gets a default A4 landscape page centered at the origin: Koebi's default
  cube. It also gets the document unit of its preset or module.

### The splash screen

A floating panel built from `koebi-ui` components, laid out by measured widths (#247) and
scrolling where needed:

- **The image band** at the top, with the application's logo and version.
- **New File:**
  - the presets, each with its icon and a one-line description;
  - the page formats of the old *New Document* popup: A4 portrait and landscape, 1920 × 1080,
    1280 × 720, square, 1080 × 1350, custom.
- **Recent Files:**
  - each document's embedded preview (ADR 0031), name, folder and date;
  - pinned files first;
  - pin, remove from the list, clear the list;
  - a missing file shown greyed with *Locate…* and *Remove*;
  - a scroll area for long lists.
- **Modules:** each with its icon, name and description; a click starts a new document in it.
- **Links:** the project's repository and the documentation.

Behavior:
- It opens at launch (an option in Settings › General) and from the application menu.
- It closes with Esc, a click outside it, or any action.
- Keyboard navigation moves through its rows.
- Unsaved changes are asked about first (*Save*, *Don't Save*, *Cancel*), then the action runs.

## Consequences

- **Modules specialize without forking:** every module goes through the same session,
  operations and graph. A module's document opens anywhere, with or without the module.
- **New features stay in their own code:** a capability flag is a typed field, and a feature
  that should be switchable reads it. IOF's needs (pinned CMYK, locked symbol styles, numpad
  codes) are already covered by hooks, so filling the module adds code without changing the
  application.
- **The splash screen is an ordinary set of components:** it follows the theme, scales, scrolls
  and is tested like any page of the settings (cut labels, steady-frame allocations, golden
  images).

## Alternatives considered

- **Dynamic plugins now:** an unstable ABI across compiler versions, or a C ABI that hides
  every Rust type. Nothing asks for third-party modules yet.
- **Modules as document templates only:** no tools, panels or policies, which IOF needs.
