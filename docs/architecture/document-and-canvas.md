# The document, its canvas and the editors

How Koebi's documents are modelled, rendered and edited, and the order the work is done in.
The decisions are ADRs 0028 to 0033. What the previous C++ application did, feature by feature,
is in [`docs/development/feature-parity.md`](../development/feature-parity.md).

## The model in one page

```text
                       ┌────────────────────────── koebi-doc ─────────────────────────────┐
  Viewport ─┐          │ Session                                                          │
  Outliner ─┤ ops ───► │   Document = typed node graph ──evaluate──► Program per page    │
  Properties┤          │   History (transactions)      (revisions,    (draw, merge,      │
  Node Graph┘          │   Selection, edit context      caches)        blend, mask)      │
     ▲                 └──────────────────────────────────────┬───────────────────────────┘
     │ immediate mode: every editor reads the session         │ Program + View
     │ each frame                                             ▼
     │                                         koebi-canvas: curves on the GPU, compositing
     │                                                        │ view target (Rgba16Float)
     └── koebi-ui (spaces, components) ◄── image primitive ───┘
```

- **The document is a typed node graph** (ADR 0028). Objects, pages, paints, modifiers and
  their composition are nodes. Any output can feed any compatible input; invalid links are kept
  and shown. Layers and Collections are views of it.
- **Operations** (`koebi_doc::ops`) are how the application edits the graph without the user
  touching it: add a shape, set a blend mode, group, parent, add a modifier. Every editor calls
  them, and the graph stays in canonical form.
- **A modal gesture is a live transaction** (ADR 0032): the document is in its intermediate
  state, so every editor shows the gesture. Only the owner editor draws the tool's visuals.
- **The canvas** is drawn from the curves of every outline, with coverage computed per pixel
  (ADR 0029): exact at every zoom, with no tessellation. Compositing runs the evaluated program
  with every blend mode, Erase and masks.
- **The interface scales in spaces** (ADR 0030): the Node Graph's nodes are ordinary
  components inside a zoomed space.
- **Files** are `.koebi` binary containers with an embedded preview (ADR 0031), known to
  Windows (icon, open, thumbnails), Linux (type, thumbnailer), macOS, iOS and Android (icon,
  type).
- **Modules** specialize the application (ADR 0033): IOF and Typographie, empty at first.

## Crates

| Crate | Depends on | Holds |
|---|---|---|
| `koebi-geom` | core | paths (Bézier, NURBS, poly), measures, hit tests, stroke outlines, booleans, curve preparation |
| `koebi-image` | — | PNG encoding and decoding (from `koebi-testkit`), later image decoding (#20) |
| `koebi-doc` | core, geom, i18n | the graph, node kinds, edits and history, operations, evaluation, session, the `.koebi` codec |
| `koebi-canvas` | gpu, geom, doc | curve coverage, paints, compositing, views |
| `koebi-thumbnails` | image (Windows only) | the Explorer thumbnail provider (COM, FFI) |
| `koebi` (app) | all | editors, tools, operations' interface, modules, splash screen |

`koebi-render` gains image primitives and the dot grid. `koebi-ui` gains spaces. `render`,
`ui` and `canvas` never see SDL. `doc` and `geom` never see the GPU.

## Phases

Each item becomes an issue under the epic once the ADRs are accepted. Phases are ordered; items
inside a phase can overlap.

### 1. Foundations

1. `koebi-geom`: paths, NURBS (de Boor), builders, measures, hit tests.
2. `koebi-geom`: stroke outlines with every align, cap, join, dash and fit; overlaps united.
3. `koebi-geom`: booleans on curves; cubic-to-quadratic preparation by zoom octave.
4. `koebi-doc`: graph, socket types and conversions, links and validity, node registry, the
   first node kinds.
5. `koebi-doc`: edits, transactions, history; the first operations; collections.
6. `koebi-doc`: evaluation with revisions and caches, producing programs.
7. `koebi-ui`: spaces, pointer reads through `Ui`, the clippy rule.
8. `koebi-render`: image primitives; the dot grid fill.
9. `koebi-canvas`: outline coverage, solid paints, views, steady state, picking.
10. `koebi-canvas`: compositing (merge, the 16 blend modes and Erase, masks), patterns,
    instanced fills, repeats and marks.
11. Viewport:
    - an infinite camera with every navigation gesture;
    - the dot grid with its levels, toggled from the overlays menu;
    - the default A4 landscape page;
    - rulers on four sides with units and the corner button.
12. `koebi-image` and the `.koebi` file:
    - sections, graph records, preview and preview zone;
    - New, Open, Save, Save As and recent files;
    - unsaved-changes dialog;
    - `koebi file.koebi` and `--thumbnail`.
13. The systems:
    - Windows association and the thumbnail DLL;
    - Linux type, thumbnailer and desktop file;
    - macOS and iOS document type;
    - Android intent filters.

### 2. Editing in the Viewport

1. Session, operations, transactions with ownership, the overlay policy; keymap contexts by
   mode and tool; status-bar hints.
2. Relative mouse mode, the wrapped virtual cursor and the operation cursors.
3. Select tool:
   - click, Shift, box, click-drag move;
   - selection overlays and the origin and parent lines.
4. Modal Move, Rotate and Scale:
   - axis constraints, precision, typed values;
   - five pivots and five orientations;
   - the operator redo panel.
5. Snapping:
   - increment, grid, vertex, edge, face, edge center;
   - bases, affected transforms, increments;
   - candidates and engaged markers.
6. The 2D cursor tool and the Shift+S pie.
7. The Viewport's header:
   - mode, the fill and stroke chips, orientation, pivot, snapping;
   - the overlays menu: rulers, unit, 2D cursor, print preview, grid, other overlays.
8. Shape tool with its variants and constraints; the Add menu; context menus.
9. Curve tool:
   - Bézier, Bézier circle, NURBS path and circle, poly line, free;
   - closing, finishing, removing points, continuing and branching.
10. Follow curve.
11. Edit mode:
    - selection of elements, handle kinds, point and handle drags, element transforms;
    - handle type menu, delete, merge, spline type, cyclic, extrude;
    - more, less, linked and grouped selections; hide and reveal; join.
12. Line Mark mode: marks, gaps, repeats; the mark tool and its keys; copy and paste.
13. The N panel: three stages, tabs, animated; the Item and Marks tabs; module tabs.
14. Clipboard, duplicate, duplicate linked, group, ungroup, parent, apply scale, set origin.

### 3. Editors

1. Outliner, Collections mode:
   - rows, guides, collapsed summaries;
   - search with Blender's colors;
   - filters (Objects, Groups, Pages, Collections) and states (All, Visible, Selected, Active,
     Selectable) with invert;
   - eye, lock and selectable toggles;
   - selection rules and rename;
   - drag and drop of every row kind, with its rules and animations;
   - context menus with every submenu;
   - viewport synchronization with its effect and preview;
   - frame selected.
2. Outliner, Layers mode:
   - the stacks;
   - live previews;
   - clip, mask and customized badges;
   - drops onto masks, clips and positions;
   - a consumer's rows.
3. Properties:
   - Object: transform with padlocks, curve, compositing, instance;
   - Paint: fills, strokes, paint order;
   - Modifiers: Array, Along Path, Boolean;
   - Document;
   - linked inputs shown with their source.
4. Fill, Stroke and Strokes & Fills editors, with mixed values across a selection.

### 4. Node Graph

1. Nodes as components in a zoomed space:
   - sockets colored by type;
   - links with Blender's grab rules, compatibility colors, and invalid links kept;
   - box selection, moving, collapsing, muting.
2. Add menu with search; node groups; the Preview node; live previews; automatic layout of
   canonical clusters; breadcrumb; *Reset to automatic*.

### 5. Application

1. The splash screen: presets, page formats, recent files with previews, modules.
2. Modules: the `Module` API and host; IOF and Typographie as empty modules; workspace presets.
3. The file menu and the application menu; the Info log of operations.

### Later

- Gradients, images (#20), text objects.
- Export to PNG, PDF and SVG; print proofing (overprint, separations, flattener).
- The swatch table (Palette) and Colour Usage editors.
- IOF and Typographie content.
- GPU picking, external modules.
