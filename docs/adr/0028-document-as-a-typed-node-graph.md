# 0028. The document is a typed node graph; layers and collections are views of it

- Status: Proposed
- Date: 2026-11-17
- Deciders: @Kolosta

## Context

Koebi edits vector documents: pages, shapes, paints, modifiers, and how they combine. The
previous C++ application (Carto) kept a **layer tree** as its truth and regenerated a
compositing graph from it on every compile. Its own documentation calls layers "a view of the
graph", but the graph could not hold anything the tree could not express:

- **Layers as the truth.** Each node had a parent, a z-order, an opacity and a blend mode. A
  blend applied to the siblings below it, and nothing else could be routed.
- **Patched edits.** Hand-wired inputs (`compInputs`) were a per-group patch over the
  generated graph. They were not saved, and their mute flag did nothing.
- **The compositor was known broken.** A Multiply leaked onto the siblings above it, an Erase
  wiped its whole group, and nested clips escaped. A redesign was specified but never verified
  on screen.
- **No sharing.** One object's color could not drive another's, and one object's shape could
  not mask an unrelated object.

The maintainer's requirements for Koebi:

- **The graph is the document.** Rendering never sees a stack of layers.
- **Anything can feed anything.**
  - Any object or property is an input elsewhere in the graph.
  - One color can feed several objects.
  - One object's shape, transformed on the way, can restrict a blend mode on an unrelated
    object.
- **Strict links.** Rust types decide what may connect. An incompatible link is never deleted
  but shown as invalid, as in Blender.
- **Usable without the graph.** The Viewport, the Outliner (Collections and Layers modes) and
  the property editors edit the document with familiar actions, and the graph follows them.
- **Layers mode is an abstraction** for users who come from layer-based tools, backed by the
  graph.
- **The Node Graph editor shows everything:** the document, pages, objects, modifiers, colors
  and operations.
- **Nodes are reusable components:** groups of nodes can be packaged and used again.

## Decision

A new crate, `koebi-doc` (std only; depends on `koebi-core` and `koebi-geom`, ADR 0029), holds
the document as a **typed dataflow graph**. Every other representation is computed from it or
edits it through named operations.

### Nodes, sockets, links

- **A node** is an instance of a *node kind*: a static definition with a stable string id
  (`koebi.object.path`), inputs, outputs, settings and an evaluation function. A node has:
  - a `NodeId`: 64 bits, never reused within a document;
  - its kind and a name;
  - one value per input, used while nothing is linked to it;
  - its settings: structured data that is not linkable, such as the marks of a stroke or the
    shapes of an instanced fill;
  - its placement in the Node Graph editor.
- **Socket types** form a closed enum, `SocketType`:
  - values: `Bool`, `Int`, `Float`, `Vector`, `Color` (linear, straight alpha);
  - `Transform` (a 2D affine in f64);
  - `Geometry` (paths in document units);
  - `Paint` (a fill or stroke piece);
  - `Layer` (renderable content with its compositing);
  - `Coverage` (a mask);
  - `Text`;
  - `Enum(EnumKind)`: blend modes, caps, joins and the like.

  A socket also carries a *quantity* (length, angle, percent, scalar) for units in the
  interface. It never changes the type.
- **Typed access.** A node kind declares its sockets as typed constants, for instance
  `const FILL_COLOR: In<Color>`, and reads them with `inputs.get(FILL_COLOR)`. A kind that reads
  a socket with the wrong type does not compile.
- **A link** goes from an output to an input. An input takes one link, except a *multi-input*
  (a Merge's layers, a Document's pages), which takes an ordered list. A link is **valid** when:
  - the types match or an explicit conversion exists (`Float → Color` as a gray,
    `Color → Float` as luminance, `Int ↔ Float`, `Bool → Float`);
  - and it creates no cycle.

  An invalid link stays in the document, is drawn in the editor's error color with a tooltip
  saying why, and evaluation uses the input's own value instead. Nothing removes a link but
  the user.
- **Pass-through outputs.** Every value input of a data node has an output of the same name
  that carries its resolved value. A fill's Color feeds another fill's Color directly. The Node
  Graph hides unused sockets until asked.

### Kinds of nodes

| Family | Kinds (first set) |
|---|---|
| Document | Document output (multi-input of pages) |
| Page | Page (size, background, content layer) |
| Objects | Path object, Group object, Instance object |
| Geometry | Path data (Bézier, NURBS, poly subpaths); Rectangle, Ellipse, Polygon, NURBS circle; Transform geometry; Boolean (union, subtract, intersect, xor); Offset |
| Paint | Fill (solid, pattern, instanced), Stroke (width, align, caps, joins, dashes, marks, repeats) |
| Composition | Merge (ordered layers, each with blend mode, opacity and visibility); Blend (backdrop, source, mode, opacity, isolate); Mask (content, coverage, invert); Coverage of a geometry or of a layer |
| Instancing | Array (transform, line, circle), Along path, Instance of a layer |
| Values | Color, Float, Vector, Transform, Math, Mix color, Swatch reference |
| Inspection | Preview: shows any socket's value (a layer, a geometry, a color, a number) in the Node Graph |

- **The blend modes** are those of W3C Compositing Level 1 and Erase, applied on premultiplied
  linear color (ADR 0029):
  - separable: Normal, Multiply, Screen, Overlay, Darken, Lighten, Color Dodge, Color Burn,
    Hard Light, Soft Light, Difference, Exclusion;
  - non-separable: Hue, Saturation, Color, Luminosity;
  - Erase, which removes the backdrop where the source lies (destination-out).
- **Node groups** package a subgraph behind its own sockets. They are reusable components,
  saved with the document. Later they can be shared between documents.

### Objects as clusters, the canonical form

The user sees *objects*. In the graph, an object is an **object node** with the nodes that
feed it.

- **Inputs of an object node:**
  - its geometry: base path data, then the geometry modifiers in order;
  - its paint stack: a multi-input of fills and strokes, in paint order;
  - its local transform and a `Parent` transform;
  - opacity and visibility.
- **Outputs:**
  - `Layer` (placed in the world) and `Local layer`;
  - world and local geometry;
  - world transform;
  - bounds.
- **Placement in a page:** a page's content is a Merge, its *stack*. An object enters a stack
  through one of the Merge's inputs, which carries its blend mode and opacity. A group object
  holds a Merge of its children, and its transform applies to them.
- **Parenting** is a link from the parent's world transform to the child's `Parent` input. It
  is independent of the stacks, as in Carto.
- **Instances** (linked duplicates) take another object's `Local layer`, with their own
  transform. Copying the original's location, rotation or scale is a link.
- **Modifiers:**
  - Boolean, Offset and the like are geometry nodes in the object's geometry chain;
  - Array and Along path are instancing nodes between the object's layer and its stack.
- **The canonical form** of an object is that cluster wired the standard way. Automated
  operations (below) produce and maintain it. A cluster that differs is *customized*. It
  renders the same way, since rendering only reads the graph, and the views mark it.

### Views computed from the graph

- **Layers mode** (Outliner):
  - For each page, the page stack's inputs from top to bottom, groups nesting their own
    stacks, each object's paints below it.
  - An object whose layer also feeds something else, such as a mask on another object,
    appears again under that consumer: one object shown in two places, selected as one.
  - A customized cluster carries a badge, with *Reset to automatic*.
- **Collections mode** (Outliner): collections organize objects and stay outside the graph.
  - They form a tree, and membership is many to many.
  - Each has a name, a color tag, and visible and selectable flags.
  - Evaluation skips an object hidden by any collection, including through a parent
    collection: Carto checked only direct membership.
  - Parented objects nest under their parent.
- **The Properties, Fill, Stroke and Strokes & Fills editors** read the active object's
  cluster.
  - A linked input shows its source ("from *Shape 2* › Fill › Color") with a button to follow
    or unlink it.
- **The Node Graph editor** shows the graph itself, ADR 0030 and 0032.

### Operations: the application without the graph

`koebi_doc::ops` is a set of named operations. Each one turns an action into graph edits and
keeps clusters canonical. Examples:
- add a shape or a page;
- add, remove or reorder a fill or a stroke;
- set a property;
- set an object's blend mode, opacity or mask;
- group and ungroup, reorder in a stack;
- parent and unparent;
- duplicate, duplicate linked;
- add a modifier;
- move to a collection;
- *Use as mask of…*, *Drive from…*.

The Viewport, the Outliner and the property editors only call operations. On a customized
cluster, an operation edits only what it owns; when it cannot, it reports why.

### Edits, undo, evaluation

- **Every change is an `Edit`:**
  - add or remove a node (with its links);
  - link or unlink;
  - set an input's value or a node's settings;
  - rename;
  - move in the Node Graph;
  - collection changes.

  Applying an edit returns its inverse. A *transaction* is an ordered list of edits with a
  label: one undo step.
- **Continuous gestures** (a drag, a slider) keep one transaction open while the document
  shows its edits. They commit or revert at the end (ADR 0032).
- **The history** is a bounded list of transactions. Selection and mode changes are steps too,
  as in Carto and Blender, without marking the document as changed.
- **Every collection edit is undoable.** Carto's were not.
- **Evaluation** pulls from the outputs a view needs, in topological order.
  - Every node keeps a revision. An output is cached with the revisions of its inputs, so a
    color change re-evaluates its paint and nothing upstream.
  - Geometry results (outlines, curve bands) are cached by content.
- **The result per page is a `Program`:** a DAG of draw, merge, blend, mask and isolate steps
  over prepared geometry, in f64 document coordinates. The canvas renderer turns it into GPU
  work for each view (ADR 0029).

## Consequences

- The four kinds of use the maintainer described are ordinary links:
  - reusing a property;
  - one color driving several objects;
  - a transformed shape masking an unrelated blend;
  - layer-style stacking.
- No second truth to keep in step: Layers and Collections are computed. An invalid or
  customized structure is visible, never silently dropped.
- The node catalogue is the long-term surface of the application.
  - A new feature is a node kind and, where users expect it, an operation.
  - Kinds are identified by stable ids in files, so an unknown kind from a newer version is
    kept as an opaque node and saved back unchanged (ADR 0031).
- Automations must be tested as much as the nodes. Each operation gets tests:
  - its edits;
  - its inverse;
  - the canonical form it leaves;
  - its behavior on a customized cluster.
- The graph editor and the operations share one edit vocabulary, so undo works the same from
  every editor.

## Alternatives considered

- **Layer tree with a generated graph (Carto):** what the requirements rule out. Routing
  across objects needs patches that the tree cannot hold.
- **Dynamically typed sockets checked at run time only:** simpler node code. But a node
  reading the wrong type would fail in users' hands instead of at compile time.
- **One giant node per object with every property inside:** fewer nodes, but no property
  could feed another without special cases. Pass-through outputs give the same simplicity in
  the views, with full routing.
- **Collections as graph nodes:** they organize, they do not compute. In the graph they would
  only add links to maintain.
