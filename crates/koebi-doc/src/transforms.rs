//! Objects' transforms: `world = space × parent_inverse × local` (ADR 0028), computed from the
//! chain each time, never stored and patched.
//!
//! An object's *space* is its parent's world transform when it has a parent (a valid link into
//! its Parent input from another object's world transform); otherwise its group's world
//! transform when it lies in a group; otherwise the identity. Its *local* transform is its
//! location, rotation and scale: the values its inputs hold (inputs driven by links are
//! evaluated by #261).

use std::collections::BTreeSet;

use koebi_geom::{Affine, Transform, Vector};

use crate::graph::{Graph, KindRef, LinkStatus, NodeId, Settings, SocketRef};
use crate::kinds::{group_object, instance_object, merge, path_object};

/// The input indexes of an object's placement: location, rotation, scale, parent.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Placement {
    pub(crate) location: usize,
    pub(crate) rotation: usize,
    pub(crate) scale: usize,
    pub(crate) parent: usize,
    /// Its opacity input.
    pub(crate) opacity: usize,
    /// Its visibility input.
    pub(crate) visible: usize,
    /// Its world transform output.
    pub(crate) world: usize,
    /// Its layer output.
    pub(crate) layer: usize,
}

/// The placement sockets of `node`, when it is an object.
pub(crate) fn placement(graph: &Graph, node: NodeId) -> Option<Placement> {
    let KindRef::Static(kind) = graph.node(node)?.kind() else {
        return None;
    };
    if kind.id == path_object::KIND.id {
        Some(Placement {
            location: path_object::LOCATION.index(),
            rotation: path_object::ROTATION.index(),
            scale: path_object::SCALE.index(),
            parent: path_object::PARENT.index(),
            opacity: path_object::OPACITY.index(),
            visible: path_object::VISIBLE.index(),
            world: path_object::WORLD.index(),
            layer: path_object::LAYER.index(),
        })
    } else if kind.id == group_object::KIND.id {
        Some(Placement {
            location: group_object::LOCATION.index(),
            rotation: group_object::ROTATION.index(),
            scale: group_object::SCALE.index(),
            parent: group_object::PARENT.index(),
            opacity: group_object::OPACITY.index(),
            visible: group_object::VISIBLE.index(),
            world: group_object::WORLD.index(),
            layer: group_object::LAYER.index(),
        })
    } else if kind.id == instance_object::KIND.id {
        Some(Placement {
            location: instance_object::LOCATION.index(),
            rotation: instance_object::ROTATION.index(),
            scale: instance_object::SCALE.index(),
            parent: instance_object::PARENT.index(),
            opacity: instance_object::OPACITY.index(),
            visible: instance_object::VISIBLE.index(),
            world: instance_object::WORLD.index(),
            layer: instance_object::LAYER.index(),
        })
    } else {
        None
    }
}

/// An object's local transform, from its location, rotation and scale.
pub fn local(graph: &Graph, object: NodeId) -> Option<Transform> {
    let sockets = placement(graph, object)?;
    let node = graph.node(object)?;
    let vector = |index: usize| match node.input(index) {
        Some(crate::Value::Vector(v)) => Some(*v),
        _ => None,
    };
    let rotation = match node.input(sockets.rotation) {
        Some(crate::Value::Float(angle)) => *angle,
        _ => return None,
    };
    Some(Transform {
        location: vector(sockets.location)?,
        rotation,
        scale: vector(sockets.scale)?,
    })
}

/// An object's parent inverse: the inverse of its parent's world transform when it was
/// parented.
pub fn parent_inverse(graph: &Graph, object: NodeId) -> Affine {
    match graph.node(object).map(|node| &node.settings) {
        Some(Settings::Object { parent_inverse }) => *parent_inverse,
        _ => Affine::IDENTITY,
    }
}

/// The object `object` has as parent: the source of a valid link into its Parent input.
pub fn parent(graph: &Graph, object: NodeId) -> Option<NodeId> {
    let sockets = placement(graph, object)?;
    let statuses = graph.statuses();
    graph
        .links_into(SocketRef::new(object, sockets.parent))
        .iter()
        .filter(|id| matches!(statuses.get(id), Some(LinkStatus::Valid(_))))
        .filter_map(|id| graph.link_of(*id))
        .map(|link| link.from.node)
        .find(|source| placement(graph, *source).is_some())
}

/// The group object whose stack `object`'s layer enters, if any.
pub fn group_of(graph: &Graph, object: NodeId) -> Option<NodeId> {
    let sockets = placement(graph, object)?;
    graph
        .links_from(SocketRef::new(object, sockets.layer))
        .filter_map(|link| {
            let stack = link.to.node;
            is_kind(graph, stack, merge::KIND.id).then_some(stack)
        })
        .flat_map(|stack| graph.links_from(SocketRef::new(stack, merge::LAYER.index())))
        .map(|link| link.to.node)
        .find(|group| is_kind(graph, *group, group_object::KIND.id))
}

pub(crate) fn is_kind(graph: &Graph, node: NodeId, id: &str) -> bool {
    graph.node(node).is_some_and(|n| n.kind().id() == id)
}

/// An object's space: its parent's world transform, else its group's, else the identity.
pub fn space(graph: &Graph, object: NodeId) -> Affine {
    space_within(graph, object, &mut BTreeSet::new())
}

/// An object's world transform: `space × parent_inverse × local`. The identity for a node that
/// is not an object, or one on a cycle of parents.
pub fn world(graph: &Graph, object: NodeId) -> Affine {
    world_within(graph, object, &mut BTreeSet::new())
}

fn world_within(graph: &Graph, object: NodeId, seen: &mut BTreeSet<NodeId>) -> Affine {
    let Some(local) = local(graph, object) else {
        return Affine::IDENTITY;
    };
    if !seen.insert(object) {
        return Affine::IDENTITY;
    }
    let world =
        space_within(graph, object, seen) * parent_inverse(graph, object) * local.to_affine();
    seen.remove(&object);
    world
}

fn space_within(graph: &Graph, object: NodeId, seen: &mut BTreeSet<NodeId>) -> Affine {
    if let Some(parent) = parent(graph, object) {
        return world_within(graph, parent, seen);
    }
    group_of(graph, object).map_or(Affine::IDENTITY, |group| world_within(graph, group, seen))
}

/// Whether two transforms place everything within `tolerance` of each other.
pub fn close(a: Affine, b: Affine, tolerance: f64) -> bool {
    let probe = |affine: Affine| {
        [
            Vector::new(0.0, 0.0),
            Vector::new(1.0, 0.0),
            Vector::new(0.0, 1.0),
        ]
        .map(|v| affine.apply(v.to_point()))
    };
    probe(a)
        .iter()
        .zip(probe(b))
        .all(|(p, q)| p.distance(q) <= tolerance)
}
