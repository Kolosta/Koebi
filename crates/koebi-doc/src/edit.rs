//! Edits: every change to a document, each returning its inverse when applied (ADR 0028).
//!
//! Ids are kept through edits: removing a node and putting it back gives the same node, with
//! the same links at the same places in their inputs' lists. Redoing an undone edit therefore
//! gives the same document, not one with new ids.

use koebi_geom::Point;

use crate::collection::{Collection, CollectionId, Collections};
use crate::graph::{Graph, Link, LinkId, MergeEntry, Node, NodeId, Settings};
use crate::value::Value;

/// A link with its place in its input's list, and the Merge entry it has when it enters one.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedLink {
    /// The link, with its id.
    pub link: Link,
    /// Its place among the links of its input.
    pub position: usize,
    /// Its entry in the Merge it enters, if any.
    pub entry: Option<MergeEntry>,
}

/// A change to a document.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    /// Adds a node, with its id, unlinked.
    AddNode(Box<Node>),
    /// Removes a node, with its links.
    RemoveNode(NodeId),
    /// Puts a node back with its links, each at its place: the inverse of removing it.
    RestoreNode {
        /// The node.
        node: Box<Node>,
        /// Its links, in and out.
        links: Vec<PlacedLink>,
    },
    /// Adds a link at its place, with its Merge entry.
    Link(PlacedLink),
    /// Removes a link.
    Unlink(LinkId),
    /// Sets the value of an input, which must be of its type.
    SetInput {
        /// The node.
        node: NodeId,
        /// The input.
        index: usize,
        /// The value.
        value: Value,
    },
    /// Sets a node's settings.
    SetSettings {
        /// The node.
        node: NodeId,
        /// The settings.
        settings: Box<Settings>,
    },
    /// Renames a node.
    Rename {
        /// The node.
        node: NodeId,
        /// Its new name.
        name: String,
    },
    /// Moves a node in the Node Graph editor.
    Move {
        /// The node.
        node: NodeId,
        /// Its new place.
        position: Point,
    },
    /// Adds a collection, as given.
    AddCollection(Box<Collection>),
    /// Removes a collection, which holds no other collection.
    RemoveCollection(CollectionId),
    /// Sets everything of a collection: name, color, flags, parent and members.
    SetCollection(Box<Collection>),
}

impl Edit {
    /// What the edit changes, for merging the edits of a continuous gesture: two edits with the
    /// same target set the same thing, and only the first one's inverse matters.
    pub(crate) fn target(&self) -> Option<(u8, u64, usize)> {
        match self {
            Self::SetInput { node, index, .. } => Some((0, node.0, *index)),
            Self::SetSettings { node, .. } => Some((1, node.0, 0)),
            Self::Rename { node, .. } => Some((2, node.0, 0)),
            Self::Move { node, .. } => Some((3, node.0, 0)),
            Self::SetCollection(collection) => Some((4, collection.id.0, 0)),
            _ => None,
        }
    }
}

/// Why an edit could not apply. The document is unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    /// No such node.
    NoNode(NodeId),
    /// The node's id is taken.
    NodeTaken(NodeId),
    /// The link does not fit: its id is taken, a node or socket is missing, or its single
    /// input already has a link.
    LinkDoesNotFit(LinkId),
    /// No such link.
    NoLink(LinkId),
    /// No such input, or a value of another type.
    BadInput(NodeId, usize),
    /// No such collection, or its id is taken.
    Collection(CollectionId),
    /// The collection holds other collections, or would lie inside itself.
    CollectionTree(CollectionId),
}

/// Applies `edit` to `graph` and `collections`, and returns its inverse.
///
/// # Errors
///
/// When the edit does not apply; nothing has changed then.
pub(crate) fn apply(
    graph: &mut Graph,
    collections: &mut Collections,
    edit: Edit,
) -> Result<Edit, EditError> {
    match edit {
        Edit::AddNode(node) => {
            let id = node.id();
            if graph.insert_node(*node) {
                Ok(Edit::RemoveNode(id))
            } else {
                Err(EditError::NodeTaken(id))
            }
        }
        Edit::RemoveNode(id) => {
            if graph.node(id).is_none() {
                return Err(EditError::NoNode(id));
            }
            // Each link's place and entry, read before anything is removed.
            let mut links: Vec<PlacedLink> = graph
                .links()
                .filter(|link| link.from.node == id || link.to.node == id)
                .map(|link| PlacedLink {
                    link: *link,
                    position: graph.position_of(link.id).unwrap_or(0),
                    entry: graph.merge_entry(link.id),
                })
                .collect();
            // Put back in order of place, so that each lands where it was.
            links.sort_by_key(|placed| placed.position);
            let removed = graph.remove(id).ok_or(EditError::NoNode(id))?;
            Ok(Edit::RestoreNode {
                node: Box::new(removed.node),
                links,
            })
        }
        Edit::RestoreNode { node, links } => {
            let id = node.id();
            if !graph.insert_node(*node) {
                return Err(EditError::NodeTaken(id));
            }
            for placed in links {
                place(graph, &placed).inspect_err(|_| {
                    graph.remove(id);
                })?;
            }
            Ok(Edit::RemoveNode(id))
        }
        Edit::Link(placed) => {
            let id = placed.link.id;
            place(graph, &placed)?;
            Ok(Edit::Unlink(id))
        }
        Edit::Unlink(id) => {
            let position = graph.position_of(id).ok_or(EditError::NoLink(id))?;
            let entry = graph.merge_entry(id);
            let link = graph.unlink(id).ok_or(EditError::NoLink(id))?;
            Ok(Edit::Link(PlacedLink {
                link,
                position,
                entry,
            }))
        }
        Edit::SetInput { node, index, value } => {
            let old = graph
                .set_input(node, index, value)
                .ok_or(EditError::BadInput(node, index))?;
            Ok(Edit::SetInput {
                node,
                index,
                value: old,
            })
        }
        Edit::SetSettings { node, settings } => {
            let target = graph.node_mut(node).ok_or(EditError::NoNode(node))?;
            let old = core::mem::replace(&mut target.settings, *settings);
            Ok(Edit::SetSettings {
                node,
                settings: Box::new(old),
            })
        }
        Edit::Rename { node, name } => {
            let target = graph.node_mut(node).ok_or(EditError::NoNode(node))?;
            let old = core::mem::replace(&mut target.name, name);
            Ok(Edit::Rename { node, name: old })
        }
        Edit::Move { node, position } => {
            let target = graph.node_mut(node).ok_or(EditError::NoNode(node))?;
            let old = core::mem::replace(&mut target.position, position);
            Ok(Edit::Move {
                node,
                position: old,
            })
        }
        Edit::AddCollection(collection) => {
            let id = collection.id;
            if collections.get(id).is_some() {
                return Err(EditError::Collection(id));
            }
            check_parent(collections, &collection)?;
            collections.insert(*collection);
            Ok(Edit::RemoveCollection(id))
        }
        Edit::RemoveCollection(id) => {
            if collections.children(Some(id)).next().is_some() {
                return Err(EditError::CollectionTree(id));
            }
            let removed = collections.remove(id).ok_or(EditError::Collection(id))?;
            Ok(Edit::AddCollection(Box::new(removed)))
        }
        Edit::SetCollection(collection) => {
            let id = collection.id;
            if collections.get(id).is_none() {
                return Err(EditError::Collection(id));
            }
            check_parent(collections, &collection)?;
            let slot = collections.get_mut(id).ok_or(EditError::Collection(id))?;
            let old = core::mem::replace(slot, *collection);
            Ok(Edit::SetCollection(Box::new(old)))
        }
    }
}

/// Puts a link in at its place, with its Merge entry.
fn place(graph: &mut Graph, placed: &PlacedLink) -> Result<(), EditError> {
    if !graph.insert_link(placed.link, placed.position) {
        return Err(EditError::LinkDoesNotFit(placed.link.id));
    }
    if let Some(entry) = placed.entry
        && let Some(node) = graph.node_mut(placed.link.to.node)
        && let Settings::Merge(entries) = &mut node.settings
    {
        entries.insert(placed.link.id, entry);
    }
    Ok(())
}

/// Refuses a parent that is missing, or that lies inside the collection.
fn check_parent(collections: &Collections, collection: &Collection) -> Result<(), EditError> {
    match collection.parent {
        Some(parent) if collections.get(parent).is_none() => Err(EditError::Collection(parent)),
        Some(parent) if collections.contains(collection.id, parent) => {
            Err(EditError::CollectionTree(collection.id))
        }
        None | Some(_) => Ok(()),
    }
}
