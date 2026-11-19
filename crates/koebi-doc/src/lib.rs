//! Koebi's document, as a typed dataflow graph ([ADR 0028]).
//!
//! The graph is the document: pages, objects, paints, modifiers and how they composite are
//! nodes whose sockets are linked. Anything can feed anything its type allows: one color can
//! drive several fills, an object's geometry can become another's mask. Layers and collections
//! are views computed from the graph, and every action edits the graph.
//!
//! - [`SocketType`]: a closed set of types, each with a Rust type ([`value`]). An input holds a
//!   [`Value`] while nothing valid is linked to it.
//! - [`NodeKind`]: a kind's static definition, with a stable id that files store. Its sockets
//!   are typed handles, `In<K, T>` and `Out<K, T>`, from which its declarations are built and
//!   checked at compile time; reading a node through them ([`NodeRef::get`]) with another type
//!   or another kind's handle does not compile. The first kinds are in [`kinds`]; the
//!   [`Registry`] finds them by id.
//! - [`Graph`]: nodes, links and node groups. A link that does not connect (its types, or a
//!   cycle) is kept, with the reason [`Graph::statuses`] gives, and never removed but by the
//!   user. Value inputs of data nodes have pass-through outputs.
//!
//! ```
//! use koebi_doc::{Graph, LinkStatus, SocketRef, kinds::{color, fill}};
//!
//! // One color drives a fill: the Color node's output is its input, passed through.
//! let mut graph = Graph::new();
//! let paint = graph.add_kind(fill::KIND);
//! let swatch = graph.add_kind(color::KIND);
//! let link = graph
//!     .link(SocketRef::new(swatch, 0), SocketRef::new(paint, fill::COLOR.index()))
//!     .expect("both sockets exist");
//! assert!(matches!(graph.status(link.id), Some(LinkStatus::Valid(_))));
//! ```
//!
//! - [`Document`]: the graph and its [`collection`]s, changed only by [`Edit`]s, each
//!   returning its inverse, grouped in transactions kept in a bounded history.
//!
//! - [`eval::Evaluator`]: the graph evaluated into one [`eval::Program`] per page, a DAG of
//!   draw, merge, blend and mask steps in document coordinates, an edit re-evaluating only what
//!   depends on it.
//!
//! [ADR 0028]: https://github.com/Kolosta/Koebi/blob/develop/docs/adr/0028-document-as-a-typed-node-graph.md

pub mod collection;
mod document;
mod edit;
pub mod eval;
mod graph;
pub mod instanced;
pub mod kind;
pub mod kinds;
pub mod ops;
pub mod transforms;
pub mod value;

pub use collection::{Collection, CollectionId, Collections, ColorTag};
pub use document::{Document, HISTORY, Transaction};
pub use edit::{Edit, EditError, PlacedLink};
pub use graph::{
    Graph, GroupId, GroupSocket, Interface, Invalid, KindRef, Link, LinkError, LinkId, LinkStatus,
    Linked, MergeEntry, Node, NodeGroup, NodeId, NodeRef, Opaque, Removed, Settings, SocketRef,
};
pub use kind::{Family, In, Initial, InputDecl, Kind, NodeKind, Out, OutputDecl, Registry};
pub use value::{
    BlendMode, Conversion, EnumKind, Geometry, InstancedLayout, PatternAnchor, PatternClip,
    Quantity, ScatterMode, Socket, SocketEnum, SocketType, SocketValue, Stored, StrokeAlign,
    StrokeCap, StrokeJoin, StrokeUnits, Text, Value,
};
