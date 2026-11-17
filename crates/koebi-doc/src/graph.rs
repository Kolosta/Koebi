//! The graph: nodes, the links between their sockets, and node groups.
//!
//! Links are never refused for their types or for a cycle: an invalid link stays, with the
//! reason [`Graph::statuses`] gives, and evaluation uses the input's own value instead. Only
//! the structure is checked when linking: both nodes and sockets must exist. A single input
//! takes one link (a new one replaces it, as in Blender); a multi-input takes an ordered list.

use core::marker::PhantomData;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use koebi_geom::{Affine, Dashes, Path, Point};

use crate::kind::{In, Kind, NodeKind, Out};
use crate::value::{
    BlendMode, Conversion, Quantity, Socket, SocketType, SocketValue, Stored, Value,
};

/// A node's id: 64 bits, never reused within a document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u64);

/// A link's id, never reused within a document. Ids grow, so they order links by creation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LinkId(pub u64);

/// A node group's id, never reused within a document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupId(pub u64);

/// A socket of a node: the node, and the socket's index among its inputs or outputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SocketRef {
    /// The node.
    pub node: NodeId,
    /// The socket's index.
    pub index: usize,
}

impl SocketRef {
    /// Socket `index` of `node`.
    pub const fn new(node: NodeId, index: usize) -> Self {
        Self { node, index }
    }
}

/// A link from an output to an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Link {
    /// Its id.
    pub id: LinkId,
    /// The output it leaves.
    pub from: SocketRef,
    /// The input it enters.
    pub to: SocketRef,
}

/// A socket of a node group's interface.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupSocket {
    /// Its name.
    pub name: String,
    /// Its type.
    pub ty: SocketType,
    /// What its number means.
    pub quantity: Quantity,
}

/// The sockets of a node group, as its users see them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Interface {
    /// Its inputs.
    pub inputs: Vec<GroupSocket>,
    /// Its outputs.
    pub outputs: Vec<GroupSocket>,
}

/// A node of a kind this version does not know (from a newer one): kept with its sockets and
/// data, and saved back unchanged.
#[derive(Clone, Debug, PartialEq)]
pub struct Opaque {
    /// The kind's id.
    pub id: String,
    /// Its inputs' types.
    pub inputs: Vec<SocketType>,
    /// Its outputs' types.
    pub outputs: Vec<SocketType>,
    /// Its data, as the file held it.
    pub data: Vec<u8>,
}

/// What a node is an instance of.
#[derive(Clone, Debug, PartialEq)]
pub enum KindRef {
    /// A kind of the application.
    Static(&'static NodeKind),
    /// A use of a node group, with a copy of its interface.
    Group {
        /// The group.
        group: GroupId,
        /// Its interface.
        interface: Arc<Interface>,
    },
    /// Inside a group: the group's inputs, as outputs.
    GroupInput(Arc<Interface>),
    /// Inside a group: the group's outputs, as inputs.
    GroupOutput(Arc<Interface>),
    /// A kind this version does not know.
    Opaque(Arc<Opaque>),
}

impl KindRef {
    /// The type of input `index`, and whether it takes several links.
    pub fn input(&self, index: usize) -> Option<(SocketType, bool)> {
        match self {
            Self::Static(kind) => kind.inputs.get(index).map(|i| (i.ty, i.multi)),
            Self::Group { interface, .. } => interface.inputs.get(index).map(|s| (s.ty, false)),
            Self::GroupOutput(interface) => interface.outputs.get(index).map(|s| (s.ty, false)),
            Self::GroupInput(_) => None,
            Self::Opaque(opaque) => opaque.inputs.get(index).map(|ty| (*ty, false)),
        }
    }

    /// The type of output `index`.
    pub fn output(&self, index: usize) -> Option<SocketType> {
        match self {
            Self::Static(kind) => kind.output_type(index),
            Self::Group { interface, .. } => interface.outputs.get(index).map(|s| s.ty),
            Self::GroupInput(interface) => interface.inputs.get(index).map(|s| s.ty),
            Self::GroupOutput(_) => None,
            Self::Opaque(opaque) => opaque.outputs.get(index).copied(),
        }
    }

    /// The number of inputs.
    pub fn input_count(&self) -> usize {
        match self {
            Self::Static(kind) => kind.inputs.len(),
            Self::Group { interface, .. } => interface.inputs.len(),
            Self::GroupOutput(interface) => interface.outputs.len(),
            Self::GroupInput(_) => 0,
            Self::Opaque(opaque) => opaque.inputs.len(),
        }
    }

    /// The kind's stable id.
    pub fn id(&self) -> &str {
        match self {
            Self::Static(kind) => kind.id,
            Self::Group { .. } => "koebi.group",
            Self::GroupInput(_) => "koebi.group.input",
            Self::GroupOutput(_) => "koebi.group.output",
            Self::Opaque(opaque) => &opaque.id,
        }
    }
}

/// A Merge's entry: how one of its layers composites.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MergeEntry {
    /// The blend mode.
    pub blend: BlendMode,
    /// The opacity, from 0 to 1.
    pub opacity: f64,
    /// Whether it shows.
    pub visible: bool,
}

impl Default for MergeEntry {
    fn default() -> Self {
        Self {
            blend: BlendMode::Normal,
            opacity: 1.0,
            visible: true,
        }
    }
}

/// A node's settings: structured data that is not linkable.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Settings {
    /// None.
    #[default]
    None,
    /// Path data: the path.
    Path(Arc<Path>),
    /// A stroke: its dashes.
    Stroke(Dashes),
    /// A Merge: each entry's compositing, by link.
    Merge(BTreeMap<LinkId, MergeEntry>),
    /// An object: the inverse of its parent's world transform when it was parented.
    Object {
        /// `world = space × parent_inverse × local`.
        parent_inverse: Affine,
    },
}

/// A node.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    id: NodeId,
    kind: KindRef,
    inputs: Vec<Value>,
    /// Its name.
    pub name: String,
    /// Its settings.
    pub settings: Settings,
    /// Its place in the Node Graph editor.
    pub position: Point,
}

impl Node {
    /// Its id.
    pub const fn id(&self) -> NodeId {
        self.id
    }

    /// Its kind.
    pub const fn kind(&self) -> &KindRef {
        &self.kind
    }

    /// The value input `index` holds (used while nothing valid is linked to it).
    pub fn input(&self, index: usize) -> Option<&Value> {
        self.inputs.get(index)
    }
}

/// A node seen as its kind `K`, to read its inputs by `K`'s typed handles.
#[derive(Clone, Copy, Debug)]
pub struct NodeRef<'a, K> {
    node: &'a Node,
    marker: PhantomData<fn() -> K>,
}

impl<'a, K: Kind> NodeRef<'a, K> {
    /// The node.
    pub const fn node(&self) -> &'a Node {
        self.node
    }

    /// The value `socket` holds.
    ///
    /// ```
    /// use koebi_doc::{Graph, kinds::fill};
    ///
    /// let mut graph = Graph::new();
    /// let id = graph.add_kind(fill::KIND);
    /// let node = graph.typed::<fill::Fill>(id).expect("a fill");
    /// let opacity: f64 = node.get(fill::OPACITY);
    /// assert_eq!(opacity, 1.0);
    /// ```
    ///
    /// Reading a socket as another type does not compile:
    ///
    /// ```compile_fail
    /// use koebi_doc::{Graph, kinds::fill};
    ///
    /// let mut graph = Graph::new();
    /// let id = graph.add_kind(fill::KIND);
    /// let node = graph.typed::<fill::Fill>(id).expect("a fill");
    /// let opacity: koebi_core::LinearRgba = node.get(fill::OPACITY);
    /// ```
    ///
    /// Nor does reading another kind's socket.
    ///
    /// # Panics
    ///
    /// Never: a node seen as kind `K` holds a `T` at the index of every `In<K, T>`, since its
    /// kind's declarations are built from those handles and its inputs are only set by type.
    ///
    /// Reading another kind's socket, as said, does not compile:
    ///
    /// ```compile_fail
    /// use koebi_doc::{Graph, kinds::{fill, stroke}};
    ///
    /// let mut graph = Graph::new();
    /// let id = graph.add_kind(fill::KIND);
    /// let node = graph.typed::<fill::Fill>(id).expect("a fill");
    /// let width = node.get(stroke::WIDTH);
    /// ```
    pub fn get<T: Stored>(&self, socket: In<K, T>) -> T {
        self.node
            .inputs
            .get(socket.index())
            .and_then(T::from_value)
            .expect("a node of kind K holds a T at the index of K's In<K, T>: its declaration is that handle's, and its inputs are only set by type")
    }

    /// The value `socket` holds, for the types an input may hold no value of (geometry, text).
    pub fn value<T: SocketValue>(&self, socket: In<K, T>) -> Option<T> {
        self.node.inputs.get(socket.index()).and_then(T::from_value)
    }
}

/// Why a link is invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Invalid {
    /// Its types do not connect, and no conversion exists.
    Types {
        /// The output's type.
        from: SocketType,
        /// The input's type.
        to: SocketType,
    },
    /// It closes a cycle: the newest link of a cycle is the invalid one.
    Cycle,
}

/// Whether a link is valid, and how its value goes through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkStatus {
    /// It carries its value, converted as said.
    Valid(Conversion),
    /// It is kept, and drawn as invalid; evaluation uses the input's own value instead.
    Invalid(Invalid),
}

/// Why a link could not be made at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkError {
    /// No such node.
    NoNode(NodeId),
    /// No such output on the source node.
    NoOutput(SocketRef),
    /// No such input on the target node.
    NoInput(SocketRef),
    /// The node is not of the kind the typed handle belongs to.
    WrongKind(NodeId),
}

/// A link made: its id, and the link it replaced on a single input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Linked {
    /// The new link.
    pub id: LinkId,
    /// The link it replaced.
    pub replaced: Option<Link>,
}

/// A node removed, with the links it had.
#[derive(Clone, Debug, PartialEq)]
pub struct Removed {
    /// The node.
    pub node: Node,
    /// Its links, in and out.
    pub links: Vec<Link>,
}

/// A node group: a subgraph behind its own sockets, a reusable component.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeGroup {
    /// Its name.
    pub name: String,
    /// Its sockets.
    pub interface: Arc<Interface>,
    /// Its nodes, with a Group input node and a Group output node.
    pub graph: Graph,
}

/// A document's graph.
#[derive(Clone, Debug, Default)]
pub struct Graph {
    nodes: BTreeMap<NodeId, Node>,
    links: BTreeMap<LinkId, Link>,
    /// The links into each input, in order.
    into: BTreeMap<SocketRef, Vec<LinkId>>,
    groups: BTreeMap<GroupId, NodeGroup>,
    /// The next id, for nodes, links and groups alike: never reused.
    next: u64,
}

/// Graphs are equal when their nodes, links and groups are: the next id, which never goes
/// back, is not part of what they hold.
impl PartialEq for Graph {
    fn eq(&self, other: &Self) -> bool {
        self.nodes == other.nodes
            && self.links == other.links
            && self.into == other.into
            && self.groups == other.groups
    }
}

impl Graph {
    /// An empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    fn fresh(&mut self) -> u64 {
        self.next += 1;
        self.next
    }

    /// Adds a node of `kind` named `name`, its inputs at their initial values.
    pub fn add(&mut self, kind: KindRef, name: impl Into<String>) -> NodeId {
        let id = NodeId(self.fresh());
        let (inputs, settings) = match &kind {
            KindRef::Static(definition) => (
                definition
                    .inputs
                    .iter()
                    .map(|i| i.initial.value())
                    .collect(),
                settings_of(definition),
            ),
            other => (
                (0..other.input_count())
                    .map(|index| other.input(index).map_or(Value::None, |(ty, _)| zero(ty)))
                    .collect(),
                Settings::None,
            ),
        };
        self.nodes.insert(
            id,
            Node {
                id,
                kind,
                inputs,
                name: name.into(),
                settings,
                position: Point::ORIGIN,
            },
        );
        id
    }

    /// Adds a node of the application's `kind`, named after it.
    pub fn add_kind(&mut self, kind: &'static NodeKind) -> NodeId {
        self.add(KindRef::Static(kind), kind.name)
    }

    /// The node `id`.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// The node `id`, to change its name, settings or place.
    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(&id)
    }

    /// The nodes, by id.
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    /// The node `id` seen as kind `K`, if it is one.
    pub fn typed<K: Kind>(&self, id: NodeId) -> Option<NodeRef<'_, K>> {
        let node = self.nodes.get(&id)?;
        // Kinds are told apart by id: a const's address is not unique.
        matches!(node.kind, KindRef::Static(kind) if kind.id == K::KIND.id).then_some(NodeRef {
            node,
            marker: PhantomData,
        })
    }

    /// Sets `socket` of node `id`, of kind `K`, to `value`. Returns the value it held, or
    /// `None` when the node is not of kind `K`.
    pub fn set<K: Kind, T: SocketValue>(
        &mut self,
        id: NodeId,
        socket: In<K, T>,
        value: T,
    ) -> Option<Value> {
        self.typed::<K>(id)?;
        let node = self.nodes.get_mut(&id)?;
        Some(core::mem::replace(
            &mut node.inputs[socket.index()],
            value.into_value(),
        ))
    }

    /// Sets input `index` of node `id` to `value`, when it is of the input's type (or
    /// [`Value::None`] for data that only links carry). Returns the value it held.
    pub fn set_input(&mut self, id: NodeId, index: usize, value: Value) -> Option<Value> {
        let node = self.nodes.get_mut(&id)?;
        let (ty, _) = node.kind.input(index)?;
        // A value of the input's type, or none for data that only links carry.
        let fits = match value.socket_type() {
            Some(found) => found == ty,
            None => matches!(zero(ty), Value::None),
        };
        fits.then(|| core::mem::replace(&mut node.inputs[index], value))
    }

    /// Removes node `id` and its links.
    pub fn remove(&mut self, id: NodeId) -> Option<Removed> {
        let node = self.nodes.remove(&id)?;
        let touching: Vec<LinkId> = self
            .links
            .values()
            .filter(|link| link.from.node == id || link.to.node == id)
            .map(|link| link.id)
            .collect();
        let links = touching
            .into_iter()
            .filter_map(|link| self.unlink(link))
            .collect();
        Some(Removed { node, links })
    }

    /// Links output `from` to input `to`: appended to a multi-input, replacing the link of a
    /// single one. Types and cycles do not refuse a link (see [`Self::statuses`]).
    ///
    /// # Errors
    ///
    /// When a node or a socket does not exist.
    pub fn link(&mut self, from: SocketRef, to: SocketRef) -> Result<Linked, LinkError> {
        let position = self.into.get(&to).map_or(0, Vec::len);
        self.link_at(from, to, position)
    }

    /// Links `from`, an output of a node of kind `A`, to `to`, an input of a node of kind `B`,
    /// of the same type `T`: a link that cannot be invalid for its types, checked at compile
    /// time. What the operations wire clusters with.
    ///
    /// # Errors
    ///
    /// When a node does not exist or is not of its handle's kind.
    pub fn connect<A: Kind, B: Kind, T: Socket>(
        &mut self,
        (source, from): (NodeId, Out<A, T>),
        (target, to): (NodeId, In<B, T>),
    ) -> Result<Linked, LinkError> {
        self.typed::<A>(source)
            .ok_or(LinkError::WrongKind(source))?;
        self.typed::<B>(target)
            .ok_or(LinkError::WrongKind(target))?;
        self.link(
            SocketRef::new(source, from.index()),
            SocketRef::new(target, to.index()),
        )
    }

    /// [`Self::link`], at `position` among a multi-input's links.
    ///
    /// # Errors
    ///
    /// When a node or a socket does not exist.
    pub fn link_at(
        &mut self,
        from: SocketRef,
        to: SocketRef,
        position: usize,
    ) -> Result<Linked, LinkError> {
        let source = self
            .nodes
            .get(&from.node)
            .ok_or(LinkError::NoNode(from.node))?;
        source
            .kind
            .output(from.index)
            .ok_or(LinkError::NoOutput(from))?;
        let target = self.nodes.get(&to.node).ok_or(LinkError::NoNode(to.node))?;
        let (_, multi) = target.kind.input(to.index).ok_or(LinkError::NoInput(to))?;
        let replaced = if multi {
            None
        } else {
            self.into
                .get(&to)
                .and_then(|links| links.first().copied())
                .and_then(|old| self.unlink(old))
        };
        let id = LinkId(self.fresh());
        self.links.insert(id, Link { id, from, to });
        let list = self.into.entry(to).or_default();
        list.insert(position.min(list.len()), id);
        Ok(Linked { id, replaced })
    }

    /// Removes link `id`.
    pub fn unlink(&mut self, id: LinkId) -> Option<Link> {
        let link = self.links.remove(&id)?;
        if let Some(list) = self.into.get_mut(&link.to) {
            list.retain(|other| *other != id);
            if list.is_empty() {
                self.into.remove(&link.to);
            }
        }
        if let Some(node) = self.nodes.get_mut(&link.to.node)
            && let Settings::Merge(entries) = &mut node.settings
        {
            entries.remove(&id);
        }
        Some(link)
    }

    /// The link `id`.
    pub fn link_of(&self, id: LinkId) -> Option<&Link> {
        self.links.get(&id)
    }

    /// The links, by id.
    pub fn links(&self) -> impl Iterator<Item = &Link> {
        self.links.values()
    }

    /// The links into input `to`, in order.
    pub fn links_into(&self, to: SocketRef) -> &[LinkId] {
        self.into.get(&to).map_or(&[], Vec::as_slice)
    }

    /// The status of every link: valid (and how it converts) or invalid (and why). Links are
    /// taken in the order they were made, so that of the links closing a cycle, the newest is
    /// the invalid one.
    pub fn statuses(&self) -> BTreeMap<LinkId, LinkStatus> {
        let mut statuses = BTreeMap::new();
        let mut downstream: BTreeMap<NodeId, BTreeSet<NodeId>> = BTreeMap::new();
        for link in self.links.values() {
            let status = self.typed_status(link).unwrap_or_else(|| {
                let reaches = |from: NodeId, target: NodeId| {
                    let mut stack = vec![from];
                    let mut seen = BTreeSet::new();
                    while let Some(node) = stack.pop() {
                        if node == target {
                            return true;
                        }
                        if seen.insert(node)
                            && let Some(next) = downstream.get(&node)
                        {
                            stack.extend(next.iter().copied());
                        }
                    }
                    false
                };
                if reaches(link.to.node, link.from.node) {
                    LinkStatus::Invalid(Invalid::Cycle)
                } else {
                    downstream
                        .entry(link.from.node)
                        .or_default()
                        .insert(link.to.node);
                    LinkStatus::Valid(self.conversion(link).unwrap_or(Conversion::Same))
                }
            });
            statuses.insert(link.id, status);
        }
        statuses
    }

    /// The status of link `id` (see [`Self::statuses`]).
    pub fn status(&self, id: LinkId) -> Option<LinkStatus> {
        self.statuses().get(&id).copied()
    }

    /// The conversion of a link, `None` when its types do not connect.
    fn conversion(&self, link: &Link) -> Option<Conversion> {
        let from = self
            .nodes
            .get(&link.from.node)?
            .kind
            .output(link.from.index)?;
        let (to, _) = self.nodes.get(&link.to.node)?.kind.input(link.to.index)?;
        from.conversion(to)
    }

    /// `Some(invalid)` when a link's types do not connect; `None` when they do, and the cycle
    /// is left to check.
    fn typed_status(&self, link: &Link) -> Option<LinkStatus> {
        let from = self
            .nodes
            .get(&link.from.node)?
            .kind
            .output(link.from.index)?;
        let (to, _) = self.nodes.get(&link.to.node)?.kind.input(link.to.index)?;
        from.conversion(to)
            .is_none()
            .then_some(LinkStatus::Invalid(Invalid::Types { from, to }))
    }

    /// A node of `kind` named `name`, with a fresh id, not yet in the graph: what an
    /// [`crate::Edit::AddNode`] adds.
    ///
    /// # Panics
    ///
    /// Never: the node is made, then taken out at once.
    pub fn new_node(&mut self, kind: KindRef, name: impl Into<String>) -> Node {
        let id = self.add(kind, name);
        self.nodes.remove(&id).expect("just added")
    }

    /// A copy of node `id` with a fresh id, not yet in the graph: same kind, values, settings
    /// and place, unlinked.
    pub fn copy_node(&mut self, id: NodeId) -> Option<Node> {
        let mut copy = self.nodes.get(&id)?.clone();
        copy.id = NodeId(self.fresh());
        Some(copy)
    }

    /// A fresh link id, for an [`crate::Edit::Link`].
    pub fn new_link_id(&mut self) -> LinkId {
        LinkId(self.fresh())
    }

    /// A fresh id for something outside the graph, such as a collection.
    pub(crate) fn new_id(&mut self) -> u64 {
        self.fresh()
    }

    /// Puts `node` in, with its own id. False when the id is taken.
    pub(crate) fn insert_node(&mut self, node: Node) -> bool {
        if self.nodes.contains_key(&node.id) {
            return false;
        }
        self.next = self.next.max(node.id.0);
        self.nodes.insert(node.id, node);
        true
    }

    /// Puts `link` in with its own id, at `position` in its input's list. False when its id
    /// is taken, a node or socket is missing, or a single input already has a link.
    pub(crate) fn insert_link(&mut self, link: Link, position: usize) -> bool {
        let fits = !self.links.contains_key(&link.id)
            && self
                .nodes
                .get(&link.from.node)
                .and_then(|node| node.kind.output(link.from.index))
                .is_some()
            && self
                .nodes
                .get(&link.to.node)
                .and_then(|node| node.kind.input(link.to.index))
                .is_some_and(|(_, multi)| multi || !self.into.contains_key(&link.to));
        if fits {
            self.next = self.next.max(link.id.0);
            self.links.insert(link.id, link);
            let list = self.into.entry(link.to).or_default();
            list.insert(position.min(list.len()), link.id);
        }
        fits
    }

    /// Where link `id` sits in its input's list.
    pub fn position_of(&self, id: LinkId) -> Option<usize> {
        let link = self.links.get(&id)?;
        self.into
            .get(&link.to)?
            .iter()
            .position(|other| *other == id)
    }

    /// The links leaving output `from`.
    pub fn links_from(&self, from: SocketRef) -> impl Iterator<Item = &Link> {
        self.links.values().filter(move |link| link.from == from)
    }

    /// The Merge entry of link `id`, when it enters a Merge that has one.
    pub fn merge_entry(&self, id: LinkId) -> Option<MergeEntry> {
        let link = self.links.get(&id)?;
        match &self.nodes.get(&link.to.node)?.settings {
            Settings::Merge(entries) => entries.get(&id).copied(),
            _ => None,
        }
    }

    /// Adds a node group named `name` with `interface`: a graph holding its Group input and
    /// Group output nodes.
    pub fn add_group(&mut self, name: impl Into<String>, interface: Interface) -> GroupId {
        let id = GroupId(self.fresh());
        let interface = Arc::new(interface);
        let mut graph = Self::new();
        graph.add(KindRef::GroupInput(interface.clone()), "Group input");
        graph.add(KindRef::GroupOutput(interface.clone()), "Group output");
        self.groups.insert(
            id,
            NodeGroup {
                name: name.into(),
                interface,
                graph,
            },
        );
        id
    }

    /// The node group `id`.
    pub fn group(&self, id: GroupId) -> Option<&NodeGroup> {
        self.groups.get(&id)
    }

    /// The node group `id`, to edit its nodes.
    pub fn group_mut(&mut self, id: GroupId) -> Option<&mut NodeGroup> {
        self.groups.get_mut(&id)
    }

    /// Adds a node using the node group `id`.
    pub fn use_group(&mut self, id: GroupId) -> Option<NodeId> {
        let group = self.groups.get(&id)?;
        let (name, interface) = (group.name.clone(), group.interface.clone());
        Some(self.add(
            KindRef::Group {
                group: id,
                interface,
            },
            name,
        ))
    }
}

/// The settings a node of `kind` starts with.
fn settings_of(kind: &NodeKind) -> Settings {
    use crate::kinds;
    let is = |other: &NodeKind| kind.id == other.id;
    if is(kinds::path_data::KIND) {
        Settings::Path(Arc::new(Path::default()))
    } else if is(kinds::stroke::KIND) {
        Settings::Stroke(Dashes::default())
    } else if is(kinds::merge::KIND) {
        Settings::Merge(BTreeMap::new())
    } else if is(kinds::path_object::KIND)
        || is(kinds::group_object::KIND)
        || is(kinds::instance_object::KIND)
    {
        Settings::Object {
            parent_inverse: Affine::IDENTITY,
        }
    } else {
        Settings::None
    }
}

/// The value an input of type `ty` holds when nothing says otherwise.
fn zero(ty: SocketType) -> Value {
    use koebi_core::LinearRgba;
    use koebi_geom::Vector;
    match ty {
        SocketType::Bool => Value::Bool(false),
        SocketType::Int => Value::Int(0),
        SocketType::Float => Value::Float(0.0),
        SocketType::Vector => Value::Vector(Vector::ZERO),
        SocketType::Color => Value::Color(LinearRgba::new(0.0, 0.0, 0.0, 1.0)),
        SocketType::Transform => Value::Transform(Affine::IDENTITY),
        SocketType::Enum(kind) => Value::Enum(kind, 0),
        SocketType::Geometry
        | SocketType::Paint
        | SocketType::Layer
        | SocketType::Coverage
        | SocketType::Text
        | SocketType::Any => Value::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::{color, document, fill, float, merge, page, path_object, preview, stroke};
    use crate::value::{EnumKind, SocketType};
    use koebi_core::LinearRgba;

    fn socket(node: NodeId, index: usize) -> SocketRef {
        SocketRef::new(node, index)
    }

    #[test]
    fn links_convert_or_stay_invalid() {
        let mut graph = Graph::new();
        let (number, paint) = (graph.add_kind(float::KIND), graph.add_kind(fill::KIND));
        // A number feeds a color as a gray, and an opacity as itself.
        let gray = graph.link(socket(number, 0), socket(paint, fill::COLOR.index()));
        let same = graph.link(socket(number, 0), socket(paint, fill::OPACITY.index()));
        let statuses = graph.statuses();
        assert_eq!(
            statuses[&gray.unwrap().id],
            LinkStatus::Valid(Conversion::Gray)
        );
        assert_eq!(
            statuses[&same.unwrap().id],
            LinkStatus::Valid(Conversion::Same)
        );
        // A paint does not feed a fill rule: kept, invalid, with why.
        let wrong = graph
            .link(
                socket(paint, fill::PAINT.index()),
                socket(paint, fill::RULE.index()),
            )
            .unwrap();
        let expected = Invalid::Types {
            from: SocketType::Paint,
            to: SocketType::Enum(EnumKind::FillRule),
        };
        assert_eq!(graph.status(wrong.id), Some(LinkStatus::Invalid(expected)));
        assert_eq!(graph.links().count(), 3);
    }

    #[test]
    fn of_the_links_closing_a_cycle_the_newest_is_invalid() {
        let mut graph = Graph::new();
        let (a, b, c) = (
            graph.add_kind(float::KIND),
            graph.add_kind(float::KIND),
            graph.add_kind(float::KIND),
        );
        let ab = graph.link(socket(a, 0), socket(b, 0)).unwrap().id;
        let bc = graph.link(socket(b, 0), socket(c, 0)).unwrap().id;
        let ca = graph.link(socket(c, 0), socket(a, 0)).unwrap().id;
        let statuses = graph.statuses();
        assert!(matches!(statuses[&ab], LinkStatus::Valid(_)));
        assert!(matches!(statuses[&bc], LinkStatus::Valid(_)));
        assert_eq!(statuses[&ca], LinkStatus::Invalid(Invalid::Cycle));
        // A node feeding itself.
        let paint = graph.add_kind(fill::KIND);
        let own = graph
            .link(
                socket(paint, fill::KIND.pass_through_of(0).unwrap()),
                socket(paint, 0),
            )
            .unwrap()
            .id;
        assert_eq!(graph.status(own), Some(LinkStatus::Invalid(Invalid::Cycle)));
        // Removing the oldest link of the cycle makes the newest valid.
        graph.unlink(ab);
        assert!(matches!(graph.status(ca), Some(LinkStatus::Valid(_))));
    }

    #[test]
    fn single_inputs_take_one_link_and_multi_inputs_an_ordered_list() {
        let mut graph = Graph::new();
        let (first, second) = (graph.add_kind(color::KIND), graph.add_kind(color::KIND));
        let paint = graph.add_kind(fill::KIND);
        let color_in = socket(paint, fill::COLOR.index());
        let old = graph.link(socket(first, 0), color_in).unwrap();
        let new = graph.link(socket(second, 0), color_in).unwrap();
        assert_eq!(new.replaced.map(|link| link.id), Some(old.id));
        assert_eq!(graph.links_into(color_in), &[new.id]);
        // A merge's layers: appended, or inserted where asked.
        let stack = graph.add_kind(merge::KIND);
        let objects: Vec<NodeId> = (0..3).map(|_| graph.add_kind(path_object::KIND)).collect();
        let layers = socket(stack, merge::LAYERS.index());
        let a = graph.link(socket(objects[0], 0), layers).unwrap().id;
        let b = graph.link(socket(objects[1], 0), layers).unwrap().id;
        let c = graph.link_at(socket(objects[2], 0), layers, 1).unwrap().id;
        assert_eq!(graph.links_into(layers), &[a, c, b]);
        assert_eq!(
            graph.link(socket(stack, 9), layers),
            Err(LinkError::NoOutput(socket(stack, 9)))
        );
    }

    #[test]
    fn removing_a_node_takes_its_links() {
        let mut graph = Graph::new();
        let (swatch, paint) = (graph.add_kind(color::KIND), graph.add_kind(fill::KIND));
        graph.link(socket(swatch, 0), socket(paint, 0)).unwrap();
        let removed = graph.remove(swatch).unwrap();
        assert_eq!(removed.links.len(), 1);
        assert_eq!(graph.links().count(), 0);
        assert_eq!(graph.links_into(socket(paint, 0)), &[] as &[LinkId]);
    }

    #[test]
    fn typed_access_reads_and_writes_by_kind() {
        let mut graph = Graph::new();
        let paint = graph.add_kind(stroke::KIND);
        let red = LinearRgba::new(1.0, 0.0, 0.0, 1.0);
        assert!(graph.set(paint, stroke::COLOR, red).is_some());
        assert!(graph.set(paint, stroke::WIDTH, 2.5).is_some());
        let node = graph.typed::<stroke::Stroke>(paint).unwrap();
        assert_eq!(node.get(stroke::COLOR), red);
        assert!((node.get(stroke::WIDTH) - 2.5).abs() < f64::EPSILON);
        assert_eq!(node.get(stroke::JOIN), crate::StrokeJoin::Miter);
        // Not a fill: no typed view, no typed write.
        assert!(graph.typed::<fill::Fill>(paint).is_none());
        assert!(graph.set(paint, fill::OPACITY, 0.5).is_none());
        // Untyped writes check the type.
        assert!(
            graph
                .set_input(paint, stroke::WIDTH.index(), Value::Bool(true))
                .is_none()
        );
        assert!(
            graph
                .set_input(paint, stroke::WIDTH.index(), Value::Float(3.0))
                .is_some()
        );
        // A page starts as A4 landscape on white.
        let sheet = graph.add_kind(page::KIND);
        let sheet = graph.typed::<page::Page>(sheet).unwrap();
        assert_eq!(sheet.get(page::SIZE), koebi_geom::Vector::new(297.0, 210.0));
    }

    #[test]
    fn a_preview_shows_anything() {
        let mut graph = Graph::new();
        let shown = graph.add_kind(preview::KIND);
        let sources = [
            (graph.add_kind(fill::KIND), fill::PAINT.index()),
            (graph.add_kind(page::KIND), page::PAGE.index()),
            (graph.add_kind(float::KIND), 0),
        ];
        for (node, output) in sources {
            let link = graph.link(socket(node, output), socket(shown, 0)).unwrap();
            assert!(matches!(graph.status(link.id), Some(LinkStatus::Valid(_))));
        }
        // It feeds nothing: it has no output.
        assert!(
            graph
                .link(socket(shown, 0), socket(sources[2].0, 0))
                .is_err()
        );
        let _ = document::KIND;
    }

    #[test]
    fn node_groups_are_used_through_their_interface() {
        let mut graph = Graph::new();
        let interface = Interface {
            inputs: vec![GroupSocket {
                name: "Tint".into(),
                ty: SocketType::Color,
                quantity: Quantity::Scalar,
            }],
            outputs: vec![GroupSocket {
                name: "Paint".into(),
                ty: SocketType::Paint,
                quantity: Quantity::Scalar,
            }],
        };
        let group = graph.add_group("Tinted fill", interface);
        // Inside: the group's input feeds a fill, whose paint is the group's output.
        let inner = &mut graph.group_mut(group).unwrap().graph;
        let ids: Vec<NodeId> = inner.nodes().map(Node::id).collect();
        let (input, output) = (ids[0], ids[1]);
        let paint = inner.add_kind(fill::KIND);
        let tint = inner
            .link(socket(input, 0), socket(paint, fill::COLOR.index()))
            .unwrap();
        let out = inner
            .link(socket(paint, fill::PAINT.index()), socket(output, 0))
            .unwrap();
        assert!(matches!(inner.status(tint.id), Some(LinkStatus::Valid(_))));
        assert!(matches!(inner.status(out.id), Some(LinkStatus::Valid(_))));
        // Outside: a node using it, with its sockets.
        let used = graph.use_group(group).unwrap();
        let object = graph.add_kind(path_object::KIND);
        let paints = socket(object, path_object::PAINTS.index());
        let link = graph.link(socket(used, 0), paints).unwrap();
        assert!(matches!(graph.status(link.id), Some(LinkStatus::Valid(_))));
        let swatch = graph.add_kind(color::KIND);
        let tinted = graph.link(socket(swatch, 0), socket(used, 0)).unwrap();
        assert!(matches!(
            graph.status(tinted.id),
            Some(LinkStatus::Valid(_))
        ));
    }

    #[test]
    fn unknown_kinds_are_kept_with_their_sockets() {
        let mut graph = Graph::new();
        let opaque = Opaque {
            id: "koebi.future.sparkle".into(),
            inputs: vec![SocketType::Layer],
            outputs: vec![SocketType::Layer],
            data: vec![1, 2, 3],
        };
        let node = graph.add(KindRef::Opaque(Arc::new(opaque)), "Sparkle");
        assert_eq!(
            graph.node(node).unwrap().kind().id(),
            "koebi.future.sparkle"
        );
        let stack = graph.add_kind(merge::KIND);
        let link = graph.link(socket(node, 0), socket(stack, 0)).unwrap();
        assert!(matches!(graph.status(link.id), Some(LinkStatus::Valid(_))));
    }
}
