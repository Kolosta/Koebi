//! Collections: how the user organizes objects, outside the graph (ADR 0028).
//!
//! Collections form a tree. Membership is many to many: an object may be in several
//! collections, a collection holds several objects. An object is hidden when any collection it
//! is in is hidden, or any collection above one of them: Carto looked at direct membership only.

use std::collections::{BTreeMap, BTreeSet};

use crate::graph::NodeId;

/// A collection's id, never reused within a document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CollectionId(pub u64);

/// The colors a collection can be tagged with, as in Blender.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorTag {
    /// No color.
    #[default]
    None,
    /// Red.
    Red,
    /// Orange.
    Orange,
    /// Yellow.
    Yellow,
    /// Green.
    Green,
    /// Blue.
    Blue,
    /// Violet.
    Violet,
    /// Pink.
    Pink,
    /// Brown.
    Brown,
}

/// A collection.
#[derive(Clone, Debug, PartialEq)]
pub struct Collection {
    /// Its id.
    pub id: CollectionId,
    /// Its name.
    pub name: String,
    /// Its color tag.
    pub color: ColorTag,
    /// Whether its members show (with its parents').
    pub visible: bool,
    /// Whether its members can be selected in the Viewport.
    pub selectable: bool,
    /// The collection it lies in; `None` at the top.
    pub parent: Option<CollectionId>,
    /// Its objects.
    pub members: BTreeSet<NodeId>,
}

impl Collection {
    /// A visible, selectable collection named `name`, at the top, empty.
    pub fn new(id: CollectionId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            color: ColorTag::None,
            visible: true,
            selectable: true,
            parent: None,
            members: BTreeSet::new(),
        }
    }
}

/// The collections of a document.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Collections {
    collections: BTreeMap<CollectionId, Collection>,
}

impl Collections {
    /// The collection `id`.
    pub fn get(&self, id: CollectionId) -> Option<&Collection> {
        self.collections.get(&id)
    }

    /// Every collection, by id.
    pub fn iter(&self) -> impl Iterator<Item = &Collection> {
        self.collections.values()
    }

    /// The collections directly inside `parent` (`None`: at the top), by id.
    pub fn children(&self, parent: Option<CollectionId>) -> impl Iterator<Item = &Collection> {
        self.collections
            .values()
            .filter(move |c| c.parent == parent)
    }

    /// The collections `node` is directly in.
    pub fn of(&self, node: NodeId) -> impl Iterator<Item = &Collection> {
        self.collections
            .values()
            .filter(move |c| c.members.contains(&node))
    }

    /// Whether `node` is hidden: by a collection it is in, or any collection above one.
    pub fn hides(&self, node: NodeId) -> bool {
        self.of(node).any(|collection| !self.shows(collection.id))
    }

    /// Whether `node` cannot be selected: by a collection it is in, or any above one.
    pub fn locks(&self, node: NodeId) -> bool {
        self.of(node)
            .any(|collection| self.ancestry(collection.id).any(|c| !c.selectable))
    }

    /// Whether collection `id` and every collection above it are visible.
    pub fn shows(&self, id: CollectionId) -> bool {
        self.ancestry(id).all(|c| c.visible)
    }

    /// Collection `id`, then the collections above it, up to the top. A cycle (which edits
    /// refuse) would stop at the first repeat.
    fn ancestry(&self, id: CollectionId) -> impl Iterator<Item = &Collection> {
        let mut seen = BTreeSet::new();
        let mut next = Some(id);
        core::iter::from_fn(move || {
            let current = self.collections.get(&next?)?;
            if !seen.insert(current.id) {
                return None;
            }
            next = current.parent;
            Some(current)
        })
    }

    /// Whether `ancestor` is `id` or lies above it.
    pub fn contains(&self, ancestor: CollectionId, id: CollectionId) -> bool {
        self.ancestry(id).any(|c| c.id == ancestor)
    }

    pub(crate) fn insert(&mut self, collection: Collection) -> Option<Collection> {
        self.collections.insert(collection.id, collection)
    }

    pub(crate) fn remove(&mut self, id: CollectionId) -> Option<Collection> {
        self.collections.remove(&id)
    }

    pub(crate) fn get_mut(&mut self, id: CollectionId) -> Option<&mut Collection> {
        self.collections.get_mut(&id)
    }
}
