# Feature parity with Carto

Carto is the previous C++ application (`ImGui_Vulkan2`, Dear ImGui and Vulkan). Koebi brings
back its **features**, never its code (ADRs 0028 to 0033). This page lists them by area, with
the Koebi phase that delivers each one ([roadmap](../architecture/document-and-canvas.md)).
Carto's defects are listed last, so that none of them returns.

Phase numbers refer to the roadmap: **1** foundations, **2** editing in the Viewport,
**3** editors, **4** Node Graph, **5** application, **L** later.

## Viewport: navigation and display

| Feature | Phase |
|---|---|
| Camera in f64 with no zoom limit; wheel zoom at the cursor (×1.15 per notch); middle drag pan | 1 |
| Ctrl+wheel and Shift+wheel pan, horizontal wheel, trackpad pinch and pan, touch | 1 |
| Frame Selected (Numpad .), Fit Document (Shift+C, every page), Reset View (Ctrl+Numpad0), animated | 1 |
| Dot grid, two levels that appear and fade with zoom, toggled in the overlays menu (new) | 1 |
| Default A4 landscape page at the origin (new) | 1 |
| Rulers top, bottom, left, right (top and left by default), per viewport; nice steps of 1, 2, 5 × 10ⁿ in display units; 5 minor ticks; cursor ticks | 1 |
| Corner unit button cycling *Follow document*, Metric, Imperial, Typographic, Pixel | 1 |
| Hover crosshair, document origin cross | 2 |
| Page frames; page name labels; active page; clicking a label activates, dragging moves (manual layout) | 2 |
| Page menu: Define Preview (whole page, zone…), Rename, Resize (A4, 1920×1080, 1280×720, square, 1080×1350, custom), Clip contents to page | 2 |
| Per-viewport page layouts: manual, left to right, right to left, top to bottom, bottom to top, grid, book (left, right), single page, single spread (left, right), hidden pages, show orphans | L |
| Print preview per viewport: normal, overprint, separations with C, M, Y, K toggles, flattener | L |
| Performance overlay (frame times, draws, caches) | exists (frame-time overlay) |

## Viewport: header

| Feature | Phase |
|---|---|
| Mode dropdown: Object, Edit (paths only), Line Mark, with Carto's tooltips | 2 |
| Fill and stroke chips: the default style, or the active object's; *none* drawn with a red diagonal; picker with alpha, stroke width, *No fill* / *No stroke*; applies to the selection, one undo step | 2 |
| Line Mark mode: mark shape picker (circle, rectangle, diamond, triangle, half circle, gap; *Empty mark*) and the Subtract toggle | 2 |
| Transform Orientation: Global, Local, View, Cursor, Parent (default Global), with tooltips | 2 |
| Transform Pivot Point: Bounding Box Center, 2D Cursor, Individual Origins, Median Point (default), Active Element | 2 |
| Snapping: magnet toggle fused to a dropdown; Snap To (Increment, Grid, Vertex, Edge, Face, Edge Center; icons); Snap Base (Closest, Pivot, Median, Active); Affect (Move, Rotate, Scale); Increment Move (10) and Rotate (5°); precisions with Shift (1, 1°); Ctrl inverts the magnet | 2 |
| Overlays dropdown: rulers ×4, viewport unit, 2D cursor (show, reset to origin, move to selection), print preview, grid, other overlays | 2 |
| Ruler space: viewport or page rulers (greyed in Carto) | L |

## Viewport: tools

| Tool or behavior | Shortcut | Phase |
|---|---|---|
| Tool palette: groups, 32 px buttons, variant triangle, right-click variant menu, tooltips with shortcut; per-mode memory of the last tool | | 2 |
| Select: click (4 px), Shift toggle, click on selected makes it active, box select (intersecting bounds; Shift extends), click-drag move with 3 px threshold (legacy) | W | 2 |
| Selection overlays: contour of paths (active 2 px orange, selected 1.5 px), bounds of groups and instances, origin dot, dashed parent line | | 2 |
| 2D Cursor tool; Shift+RMB places it under any tool; ring, axis ticks, rotation with R (legacy) | E | 2 |
| Shape tool: Rectangle, Ellipse, Triangle, Free; drag a box or click for a 160 × 160 preset; Shift square and Alt from center (new) | B | 2 |
| Curve tool: Bézier (symmetric handles; Mapper-style asymmetric as an option), Bézier Circle, NURBS Path, NURBS Circle, Poly Line, Free; Backspace removes a point; Enter or double-click finishes; click within 9 px of the first point closes; Esc and RMB cancel; continue an endpoint and branch (legacy); snapping while drawing (new) | C | 2 |
| Draw-on-create cursor: crosshair with halo and the tool's glyph | | 2 |
| Follow curve: hold Shift with the pen; diamond at the nearest curve point (24 px); lock, trace across the seam of closed curves, freeze on click | Shift | 2 |
| Add menu: Shapes (Rectangle, Ellipse, Triangle, Free), Curves (Bézier, Bézier Circle, NURBS Path, NURBS Circle, Poly Line) at the 2D cursor; preview placement option | Shift+A | 2 |
| Modal Move, Rotate, Scale; X and Y constraints in the orientation's basis; Shift precision ×0.1; Ctrl inverts snapping; LMB or Enter confirm; Esc or RMB cancel; pivot cross, axis lines, dashed guide; typed values (new); redo panel (legacy) | G, R, S | 2 |
| Duplicate (+20, +20), Duplicate & Move, Duplicate Linked & Move | Ctrl+D, Shift+D, Alt+D | 2 |
| Select All, Deselect All | A, Alt+A | 2 |
| Delete (objects; vertices in Edit mode; marks in Line Mark mode) | X, Delete | 2 |
| Group, Ungroup | Ctrl+G, Ctrl+Alt+G | 2 |
| Parent to Active, Clear Parent | Ctrl+P, Alt+P | 2 |
| Snap pie: Selection to Cursor, to Active, to Grid; Cursor to Active, to Selected, to World Origin, to Page Origin, to Grid | Shift+S | 2 |
| Set Origin: Origin to Geometry, Geometry to Origin, Origin to 2D Cursor; Apply Scale | | 2 |
| Copy, Cut, Paste objects (legacy) | Ctrl+C, Ctrl+X, Ctrl+V | 2 |
| Context menus: Object, Vertex, Viewport (Add ▸, Select All, Deselect All, 2D Cursor to Origin / to Selection), driven by the selection | RMB | 2 |
| Object eyedropper for pickers in property editors | | 2 |

### Edit mode

| Feature | Shortcut | Phase |
|---|---|---|
| Edit the active path, and every selected path (legacy) | Tab | 2 |
| Elements: point, in handle, out handle, selected separately; handles shown for touched anchors; picking 9 px, handles first | | 2 |
| Drag handles keeping the anchor's kind (symmetric mirrors, smooth keeps collinear); drag points (legacy) | | 2 |
| Box select over points (and visible handles, legacy); Shift extends | | 2 |
| G, R, S on elements; a point carries its handles; Individual Origins per element | G, R, S | 2 |
| Handle types: Free, Aligned, Mirrored, Aligned + Mirrored, Vector, Remove Handles; handles seeded at ¼ of the neighbour distance (legacy) | V | 2 |
| Colors: corner blue, smooth amber, symmetric green; NURBS control hull | | 2 |
| Merge: At Center, At 2D Cursor, At First, At Last, By Distance | M | 2 |
| Set Spline Type (Bézier, NURBS, Poly); Toggle Cyclic | | 2 |
| Extrude: cyclic outward, insert, branch with junctions | E | 2 |
| Select More / Less, Linked, Grouped (Children, Immediate Children, Parent, Siblings, Type, Collection, Color) | Shift+Numpad ±, Shift+L, Shift+G | 2 |
| Hide, Reveal; Join | H, Alt+H, Ctrl+J | 2 |
| Vertex, Edge, Face element modes for closed shapes (legacy) | 1, 2, 3 | 2 |

### Line Mark mode

| Feature | Shortcut | Phase |
|---|---|---|
| Enter and leave | Shift+Tab | 2 |
| Hover highlights the nearest stroked line; ghost of the result within 14 px; click places a mark with the header's shape; Ctrl+click places a dash tick; *Empty mark* | | 2 |
| Handles: dash (violet diamond), gap (green square), neutral (blue dot); hover and selected rings | | 2 |
| Alt+click deletes, Ctrl+click cycles phase, Shift+click toggles; click-drag slides along the curve | | 2 |
| Mark box select | | 2 |
| Slide, rotate objects, scale objects (X length, Y width) | G, R, S | 2 |
| Cycle side, cycle phase | V, P | 2 |
| Copy and paste marks with a following preview | Ctrl+C, Ctrl+V | 2 |
| Mark fields (N panel): position, side and distance, dash phase and anchor size, repeats and pitch, objects (circle, rectangle, diamond, instance, gap, triangle, half circle) with mode (fusion, blend, cut), size, rotation, along offset, bend (hard, bend, follow, chord), blend mode and order, color; gaps with caps, cut objects, start and end markers | | 2 |

### Cursor during operations

| Feature | Phase |
|---|---|
| Relative mouse mode with the system's speed; a virtual cursor folded into the editor's rect; guides past the edge; one warp at the end | 2 |
| Used by modal transforms, pans, handle and point drags, mark slides (Carto used it for transforms only) | 2 |
| Cursor glyphs: move, rotate (tangent), scale (along the guide), 28 px, tinted from tokens | 2 |

### N panel

| Feature | Phase |
|---|---|
| Three stages: closed handle, tabs only, full panel; N toggles; drag from the handle; resize grip; width 320 by default, 70 % at most | 2 |
| Vertical tabs with rotated labels; the active tab remembered by name; translucent over the canvas; animated (new) | 2 |
| Item tab: location, dimensions (read only), rotation, scale in the viewport's unit, padlocks per group, one undo step per drag | 2 |
| Marks tab; module tabs (IOF symbols); Pages tab for single-page layouts (legacy) | 2, L |

## Outliner

| Feature | Phase |
|---|---|
| Two modes: Collections (default) and Layers; per-tab state saved with the workspace | 3 |
| Top bar: mode, search, viewport sync button, filter dropdown | 3 |
| Rows: active dot, indent, chevron, icon, name, collapsed summary with counts, badges, eye; zebra stripes; state colors (hover, selected, active) | 3 |
| Guide lines under expanded containers (collection color, solid for groups and pages, dotted for parenting), recomputed during drags | 3 |
| Collections mode: project root; collections (child collections first, members alphabetical, many to many); loose objects; parented objects nested; pages (legacy) | 3 |
| Object child rows: modifiers, linked data of instances, fills and strokes, children by parenting; line marks (legacy) | 3 |
| Layers mode: per page, the stack top first, nesting groups, clip and mask badges, customized badge, live previews instead of icons (×2.4 rows) | 3 |
| Search: case-insensitive on objects, collections and pages; matches in green, their ancestors kept; non-matches hidden | 3 |
| Filter: Objects, Groups, Pages, Collections; State All, Visible, Selected, Active, Selectable; Invert | 3 |
| Eye on objects, groups, collections, pages; hidden rows dimmed, and their contents; lock and selectable toggles (missing in Carto's UI) | 3 |
| Selection: click, Ctrl toggle, Shift range from the active row, Alt add without activating; RMB never selects; double-click renames; click below the rows deselects | 3 |
| Rename inline: Enter commits, Esc cancels; collections and pages too | 3 |
| Expand and collapse; Shift-click recursive, arrows, expand all (new); Numpad . reveals and centers the active row | 3 |
| Drag and drop: objects and groups, collections, modifiers (copied onto paths); single-row drag with sliding rows, multi-row pill with counts; drop zones above, below, into; Collections and Layers semantics (mask on the preview, clip in the middle, reorder at the edges) | 3 |
| Viewport sync: arm, hover preview on a viewport, click to bind; the Outliner shows what that viewport shows | 3 |
| Context menus — Collection: Add Collection, Rename, Icon Color ▸ (Default, ten hues, Custom…), Delete, Delete Hierarchy | 3 |
| Context menus — Object (Collections): Select, Deselect, Copy, Cut, Paste, Duplicate, Delete, Move to Collection ▸, Remove from Collections, Unparent, New Collection from Selection | 3 |
| Context menus — Object (Layers): Select, Deselect, Group, Ungroup, Blend Mode ▸, Clip to First Child, Reset Node Graph to Automatic, Rename, Duplicate, Delete | 3 |
| Context menus — Linked Data: Select Original, Select Linked, Make Single; Page: Define Preview ▸, Rename, Resize, Clip Contents; Background: Add Collection | 3 |

## Properties and paint editors

| Feature | Phase |
|---|---|
| Properties tabs: Object, Paint, Modifiers, Document; header with name and type | 3 |
| Object: Location, Rotation, Scale with padlocks; Apply Scale; Curve (spline, cyclic, NURBS order, endpoint, Bézier, weights); Compositing (opacity, blend mode, isolate, clip, mask); Instance (target, copy location, rotation, scale) | 3 |
| Paint: fills rail and strokes rail (vignettes, +, drag to reorder, disabled crossed), paint order rail | 3 |
| Fill: enabled; solid (color, rule), pattern (motif, spacing, offset, angle, motif angle, scale, clip, anchor), instanced (grid 2 or 3 axes, scatter by count or distance, jitters, seed, rotation, shapes, line sets); opacity; blend | 3 |
| Stroke: enabled; color; blend; width in document or viewport units; align (center, inner, outer, left, right) and offset; caps (butt with angle, round, square, taper with length); joins and miter limit; dashes (pattern, offset, fit); repeats (shape, mode, size, incline, orient, side, offset, distribution, phase, groups, trims, fit, color, opacity) | 3 |
| Modifiers: Add Modifier; panels with enabled, close, drag to reorder; Array (transform, line, circle), Along Path (content, distribution, placement), Boolean (operation, operand) | 3 |
| Document: unit system, color mode, print technique | 3 |
| Color rows: picker with original and current, over white and black, RGBA, HSVA, CMYK, hex, document swatches | 3 |
| Object pickers with search and eyedropper | 3 |
| Fill and Stroke editors (the same panels for the selection, or the defaults with nothing selected), Strokes & Fills editor (paint order) | 3 |
| Gradients, image paints | L |
| Palette (swatches with CMYK, spot inks, print order, overprint) and Colour Usage editors | L |

## Node Graph

| Feature | Phase |
|---|---|
| Follows the active object, breadcrumb, *Reset to automatic* | 4 |
| Nodes with headers colored by family, sockets colored by type, live previews (64 to 1024 px), Preview node | 4 |
| Pan, zoom, select, box select, move, collapse (H), mute (M), delete (X), previews (P), Add (Shift+A) with search | 4 |
| Blender's link rules: grabbing a linked socket takes its link; drop on empty space unlinks; incompatible links shown, never deleted | 4 |
| Node groups as reusable components | 4 |

## Files, application, modules

| Feature | Phase |
|---|---|
| `.koebi` file, preview at save, preview zone | 1 |
| File menu: New, Open, Save, Save As, Quit; unsaved-changes dialog (Save, Don't Save, Cancel) on every path, New included | 1, 5 |
| Recent files (10, missing ones dropped) | 1 |
| Windows: icon, open verb, Explorer thumbnails | 1 |
| Splash screen: image, logo, version; New File presets General, Layout, Data; page formats; recent files with previews, pin, remove, scroll; modules; link | 5 |
| Modules: API with capabilities, layout, editors, add menu, tools, N-panel tabs, policies, numpad sequences, lifecycle; module id in the file | 5 |
| IOF module (empty first): A4 landscape in mm, CMYK pinned; later its 124 ISOM 2017-2 symbols, 30 print layers, theme tools, Symbol Viewer, Map Settings, course planning | 5, L |
| Typographie module (empty first): em square, pt; later Font Atlas, Font Editor, Font Info, Preview, Variations, glyph layers | 5, L |
| Info log of operations | 5 |
| Undo history bound set in Settings › General (256) | 2 |
| Unit expressions in number fields (exist in Koebi) | — |

## Carto defects not to repeat

- **Compositing:**
  - blends leaking onto siblings above;
  - Erase wiping the parent;
  - nested clips escaping;
  - aliased isolated edges;
  - full-screen targets per group.
- **Deep zoom:**
  - meshes in node-local f32;
  - flatten tolerance ignoring world scale;
  - zoom tiers clamped at 2^±24;
  - 120 ms re-tessellation spikes;
  - picking with a 0.01-unit floor;
  - the axis guide as a fixed ±100 000-unit line.
- **Strokes:**
  - translucent strokes darkening where they overlap themselves;
  - picking ignoring inside and outside alignment;
  - fills picked with no tolerance.
- **Ownership:**
  - every viewport draining the modal motion with its own zoom;
  - confirm keys polled everywhere;
  - document-wide pen blanking hiding strokes in other viewports;
  - RMB opening a menu in the middle of a drag.
- **Persistence:**
  - Node Graph edits not saved;
  - Map Settings not saved;
  - no open verb and argv ignored.
- **Undo:**
  - collection operations;
  - Compositing rows;
  - the Node Graph's isolate;
  - Delete Hierarchy without undo.
- **Outliner:**
  - every group member showing the clip badge;
  - Rename renaming the active object instead of the clicked row;
  - search walking the wrong relation;
  - *Collections* off hiding every member;
  - Alt making the row active;
  - shortcut hints that do nothing there;
  - Delete from the menu deleting vertices or marks in other modes.
- **Visibility:**
  - hiding a collection not hiding its child collections' members.
- **Other:**
  - Individual Origins ignored in Object mode;
  - Scale constraints in world axes;
  - Mark Rotate and Scale collapsing several objects into the first;
  - entering Line Mark mode leaving pivot and orientation changed;
  - the 2D cursor's visibility global though shown per viewport;
  - the splash screen's clipped recent files;
  - New without the unsaved-changes guard.
