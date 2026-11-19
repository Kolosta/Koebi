//! Operations: the application's actions as graph edits (ADR 0028). The Viewport, the Outliner
//! and the property editors call these, never the graph directly, so the user works without
//! seeing the graph and every action is one undo step.
//!
//! Each operation runs in a transaction (nothing changes when it fails) and leaves the clusters
//! it touches canonical: an object's geometry chain and paints wired the standard way, its
//! layer entering one stack. On a customized cluster it changes only what it owns, and says
//! what it cannot do ([`OpError::Customized`]).

use std::sync::Arc;

use koebi_geom::{Affine, Path, Transform, Vector};

use crate::collection::{Collection, CollectionId, ColorTag};
use crate::document::Document;
use crate::edit::{Edit, EditError, PlacedLink};
use crate::graph::{KindRef, Link, LinkId, MergeEntry, NodeId, Settings, SocketRef};
use crate::kind::NodeKind;
use crate::kinds::{
    document, ellipse, fill, group_object, instance_object, merge, page, path_data, path_object,
    pattern_fill, polygon, rectangle, stroke, transform_geometry,
};
use crate::transforms::{self, is_kind, placement};
use crate::value::{BlendMode, SocketValue, Value};

/// Why an operation did not apply. The document is unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpError {
    /// No such node.
    NoNode(NodeId),
    /// The node is not of a kind the operation acts on.
    WrongKind(NodeId),
    /// The cluster is customized where the operation would act: why.
    Customized(&'static str),
    /// It would make an object its own ancestor.
    Cycle,
    /// A transform that cannot be undone (a scale of 0).
    Degenerate,
    /// No such collection.
    NoCollection(CollectionId),
    /// An edit failed.
    Edit(EditError),
}

impl From<EditError> for OpError {
    fn from(error: EditError) -> Self {
        Self::Edit(error)
    }
}

/// Runs `f` in a transaction named `label`: committed when it succeeds, cancelled otherwise.
fn transaction<R>(
    doc: &mut Document,
    label: &str,
    f: impl FnOnce(&mut Document) -> Result<R, OpError>,
) -> Result<R, OpError> {
    doc.begin(label);
    match f(doc) {
        Ok(result) => {
            doc.commit();
            Ok(result)
        }
        Err(error) => {
            doc.cancel();
            Err(error)
        }
    }
}

/// Adds a node of `kind`; returns its id.
fn add(doc: &mut Document, kind: &'static NodeKind) -> Result<NodeId, OpError> {
    let node = doc.graph_mut().new_node(KindRef::Static(kind), kind.name);
    let id = node.id();
    doc.apply(Edit::AddNode(Box::new(node)))?;
    Ok(id)
}

/// Links `from` to `to` at `position` (appended when past the end), with `entry` for a Merge.
fn link(
    doc: &mut Document,
    from: SocketRef,
    to: SocketRef,
    position: Option<usize>,
    entry: Option<MergeEntry>,
) -> Result<LinkId, OpError> {
    let id = doc.graph_mut().new_link_id();
    let position = position.unwrap_or(usize::MAX);
    doc.apply(Edit::Link(PlacedLink {
        link: Link { id, from, to },
        position,
        entry,
    }))?;
    Ok(id)
}

fn set_input(doc: &mut Document, node: NodeId, index: usize, value: Value) -> Result<(), OpError> {
    Ok(doc.apply(Edit::SetInput { node, index, value })?)
}

fn set_parent_inverse(doc: &mut Document, object: NodeId, inverse: Affine) -> Result<(), OpError> {
    doc.apply(Edit::SetSettings {
        node: object,
        settings: Box::new(Settings::Object {
            parent_inverse: inverse,
        }),
    })?;
    Ok(())
}

fn set_local(doc: &mut Document, object: NodeId, local: Transform) -> Result<(), OpError> {
    let sockets = placement(doc.graph(), object).ok_or(OpError::WrongKind(object))?;
    set_input(doc, object, sockets.location, local.location.into_value())?;
    set_input(doc, object, sockets.rotation, local.rotation.into_value())?;
    set_input(doc, object, sockets.scale, local.scale.into_value())
}

/// The links of `object`'s layer into stacks (Merges): one, on a canonical cluster.
fn stack_entry(doc: &Document, object: NodeId) -> Result<(LinkId, NodeId), OpError> {
    let graph = doc.graph();
    let sockets = placement(graph, object).ok_or(OpError::WrongKind(object))?;
    let mut entries = graph
        .links_from(SocketRef::new(object, sockets.layer))
        .filter(|link| is_kind(graph, link.to.node, merge::KIND.id));
    let first = entries
        .next()
        .ok_or(OpError::Customized("the object is in no stack"))?;
    if entries.next().is_some() {
        return Err(OpError::Customized("the object enters several stacks"));
    }
    Ok((first.id, first.to.node))
}

/// A page made by [`add_page`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewPage {
    /// The page node.
    pub page: NodeId,
    /// Its stack, the Merge objects go into.
    pub stack: NodeId,
}

/// The document's output node, made if missing.
fn output(doc: &mut Document) -> Result<NodeId, OpError> {
    let found = doc
        .graph()
        .nodes()
        .find(|node| node.kind().id() == document::KIND.id)
        .map(crate::Node::id);
    match found {
        Some(id) => Ok(id),
        None => add(doc, document::KIND),
    }
}

/// Adds a page (A4 landscape on white) after the others, with its stack.
///
/// # Errors
///
/// When an edit fails.
pub fn add_page(doc: &mut Document) -> Result<NewPage, OpError> {
    transaction(doc, "Add page", |doc| {
        let out = output(doc)?;
        let page = add(doc, page::KIND)?;
        let stack = add(doc, merge::KIND)?;
        link(
            doc,
            SocketRef::new(stack, merge::LAYER.index()),
            SocketRef::new(page, page::CONTENT.index()),
            None,
            None,
        )?;
        link(
            doc,
            SocketRef::new(page, page::PAGE.index()),
            SocketRef::new(out, document::PAGES.index()),
            None,
            None,
        )?;
        Ok(NewPage { page, stack })
    })
}

/// The shapes [`add_shape`] makes.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// A rectangle of that size.
    Rectangle(Vector),
    /// An ellipse of those radii.
    Ellipse(Vector),
    /// An equilateral triangle of that radius.
    Triangle(f64),
    /// A path.
    Path(Path),
}

/// Adds a path object of `shape` with a fill, on top of `stack`; returns the object.
///
/// # Errors
///
/// When `stack` is not a Merge, or an edit fails.
pub fn add_shape(doc: &mut Document, stack: NodeId, shape: Shape) -> Result<NodeId, OpError> {
    if !is_kind(doc.graph(), stack, merge::KIND.id) {
        return Err(OpError::WrongKind(stack));
    }
    let label = match shape {
        Shape::Rectangle(_) => "Add rectangle",
        Shape::Ellipse(_) => "Add ellipse",
        Shape::Triangle(_) => "Add triangle",
        Shape::Path(_) => "Add path",
    };
    transaction(doc, label, |doc| {
        let (source, output) = match shape {
            Shape::Rectangle(size) => {
                let node = add(doc, rectangle::KIND)?;
                set_input(doc, node, rectangle::SIZE.index(), size.into_value())?;
                (node, rectangle::GEOMETRY.index())
            }
            Shape::Ellipse(radii) => {
                let node = add(doc, ellipse::KIND)?;
                set_input(doc, node, ellipse::RADII.index(), radii.into_value())?;
                (node, ellipse::GEOMETRY.index())
            }
            Shape::Triangle(radius) => {
                let node = add(doc, polygon::KIND)?;
                set_input(doc, node, polygon::RADIUS.index(), radius.into_value())?;
                set_input(doc, node, polygon::SIDES.index(), Value::Int(3))?;
                (node, polygon::GEOMETRY.index())
            }
            Shape::Path(path) => {
                let node = add(doc, path_data::KIND)?;
                doc.apply(Edit::SetSettings {
                    node,
                    settings: Box::new(Settings::Path(Arc::new(path))),
                })?;
                (node, path_data::GEOMETRY.index())
            }
        };
        let object = add(doc, path_object::KIND)?;
        link(
            doc,
            SocketRef::new(source, output),
            SocketRef::new(object, path_object::GEOMETRY.index()),
            None,
            None,
        )?;
        let paint = add(doc, fill::KIND)?;
        link(
            doc,
            SocketRef::new(paint, fill::PAINT.index()),
            SocketRef::new(object, path_object::PAINTS.index()),
            None,
            None,
        )?;
        link(
            doc,
            SocketRef::new(object, path_object::LAYER.index()),
            SocketRef::new(stack, merge::LAYERS.index()),
            None,
            Some(MergeEntry::default()),
        )?;
        Ok(object)
    })
}

fn check_path_object(doc: &Document, object: NodeId) -> Result<(), OpError> {
    if doc.graph().node(object).is_none() {
        return Err(OpError::NoNode(object));
    }
    if is_kind(doc.graph(), object, path_object::KIND.id) {
        Ok(())
    } else {
        Err(OpError::WrongKind(object))
    }
}

/// Adds a fill on top of `object`'s paints; returns it.
///
/// # Errors
///
/// When `object` is not a path object, or an edit fails.
pub fn add_fill(doc: &mut Document, object: NodeId) -> Result<NodeId, OpError> {
    add_paint(doc, object, fill::KIND, fill::PAINT.index(), "Add fill")
}

/// Adds a stroke on top of `object`'s paints; returns it.
///
/// # Errors
///
/// When `object` is not a path object, or an edit fails.
pub fn add_stroke(doc: &mut Document, object: NodeId) -> Result<NodeId, OpError> {
    add_paint(
        doc,
        object,
        stroke::KIND,
        stroke::PAINT.index(),
        "Add stroke",
    )
}

/// Adds a pattern fill to `object` whose motif is `motif`'s own layer, on top of its paints.
/// Returns the fill.
///
/// # Errors
///
/// When either is not a path object, or an edit fails.
pub fn add_pattern_fill(
    doc: &mut Document,
    object: NodeId,
    motif: NodeId,
) -> Result<NodeId, OpError> {
    check_path_object(doc, object)?;
    check_path_object(doc, motif)?;
    transaction(doc, "Add pattern fill", |doc| {
        let paint = add(doc, pattern_fill::KIND)?;
        link(
            doc,
            SocketRef::new(motif, path_object::LOCAL_LAYER.index()),
            SocketRef::new(paint, pattern_fill::MOTIF.index()),
            None,
            None,
        )?;
        link(
            doc,
            SocketRef::new(paint, pattern_fill::PAINT.index()),
            SocketRef::new(object, path_object::PAINTS.index()),
            None,
            None,
        )?;
        Ok(paint)
    })
}

fn add_paint(
    doc: &mut Document,
    object: NodeId,
    kind: &'static NodeKind,
    output: usize,
    label: &str,
) -> Result<NodeId, OpError> {
    check_path_object(doc, object)?;
    transaction(doc, label, |doc| {
        let paint = add(doc, kind)?;
        link(
            doc,
            SocketRef::new(paint, output),
            SocketRef::new(object, path_object::PAINTS.index()),
            None,
            None,
        )?;
        Ok(paint)
    })
}

/// The link of paint `paint` into `object`'s paints.
fn paint_link(doc: &Document, object: NodeId, paint: NodeId) -> Result<LinkId, OpError> {
    let paints = SocketRef::new(object, path_object::PAINTS.index());
    doc.graph()
        .links_into(paints)
        .iter()
        .copied()
        .find(|id| {
            doc.graph()
                .link_of(*id)
                .is_some_and(|l| l.from.node == paint)
        })
        .ok_or(OpError::Customized("the paint does not feed the object"))
}

/// Removes `paint` from `object`: the paint node, or only its link when it also feeds
/// something else (a shared paint).
///
/// # Errors
///
/// When the paint does not feed the object, or an edit fails.
pub fn remove_paint(doc: &mut Document, object: NodeId, paint: NodeId) -> Result<(), OpError> {
    check_path_object(doc, object)?;
    let id = paint_link(doc, object, paint)?;
    let shared = doc.graph().links().filter(|l| l.from.node == paint).count() > 1;
    transaction(doc, "Remove paint", |doc| {
        if shared {
            doc.apply(Edit::Unlink(id))?;
        } else {
            doc.apply(Edit::RemoveNode(paint))?;
        }
        Ok(())
    })
}

/// Moves `paint` to `position` among `object`'s paints (0: painted first, at the bottom).
///
/// # Errors
///
/// When the paint does not feed the object, or an edit fails.
pub fn move_paint(
    doc: &mut Document,
    object: NodeId,
    paint: NodeId,
    position: usize,
) -> Result<(), OpError> {
    check_path_object(doc, object)?;
    let id = paint_link(doc, object, paint)?;
    transaction(doc, "Move paint", |doc| relink_at(doc, id, position))
}

/// Puts link `id` back at `position` of its input's list, keeping its id and entry.
fn relink_at(doc: &mut Document, id: LinkId, position: usize) -> Result<(), OpError> {
    let entry = doc.graph().merge_entry(id);
    let link = *doc.graph().link_of(id).ok_or(EditError::NoLink(id))?;
    doc.apply(Edit::Unlink(id))?;
    doc.apply(Edit::Link(PlacedLink {
        link,
        position,
        entry,
    }))?;
    Ok(())
}

/// Sets input `index` of `node` to `value` (a property, from the property editors).
///
/// # Errors
///
/// When the input does not exist or the value is not of its type.
pub fn set_property(
    doc: &mut Document,
    node: NodeId,
    index: usize,
    value: Value,
) -> Result<(), OpError> {
    transaction(doc, "Set property", |doc| {
        set_input(doc, node, index, value)
    })
}

/// Sets how `object` composites in its stack.
///
/// # Errors
///
/// When the object is not in exactly one stack, or an edit fails.
pub fn set_entry(doc: &mut Document, object: NodeId, entry: MergeEntry) -> Result<(), OpError> {
    let (id, stack) = stack_entry(doc, object)?;
    let Some(Settings::Merge(entries)) = doc.graph().node(stack).map(|n| n.settings.clone()) else {
        return Err(OpError::Customized("the stack has no entries"));
    };
    let mut entries = entries;
    entries.insert(id, entry);
    transaction(doc, "Set blending", |doc| {
        doc.apply(Edit::SetSettings {
            node: stack,
            settings: Box::new(Settings::Merge(entries)),
        })?;
        Ok(())
    })
}

/// Sets `object`'s blend mode in its stack.
///
/// # Errors
///
/// As [`set_entry`].
pub fn set_blend(doc: &mut Document, object: NodeId, blend: BlendMode) -> Result<(), OpError> {
    let (id, _) = stack_entry(doc, object)?;
    let entry = doc.graph().merge_entry(id).unwrap_or_default();
    set_entry(doc, object, MergeEntry { blend, ..entry })
}

/// Sets `object`'s opacity in its stack.
///
/// # Errors
///
/// As [`set_entry`].
pub fn set_opacity(doc: &mut Document, object: NodeId, opacity: f64) -> Result<(), OpError> {
    let (id, _) = stack_entry(doc, object)?;
    let entry = doc.graph().merge_entry(id).unwrap_or_default();
    set_entry(doc, object, MergeEntry { opacity, ..entry })
}

/// Moves `object` to `position` in its stack (0: at the bottom).
///
/// # Errors
///
/// When the object is not in exactly one stack, or an edit fails.
pub fn move_in_stack(doc: &mut Document, object: NodeId, position: usize) -> Result<(), OpError> {
    let (id, _) = stack_entry(doc, object)?;
    transaction(doc, "Move in stack", |doc| relink_at(doc, id, position))
}

/// Groups `objects`, which share one stack, into a group object in their place; returns the
/// group. Its origin is the middle of its members' origins; their world placements stay.
///
/// # Errors
///
/// When the objects are not all in one stack, or an edit fails.
pub fn group(doc: &mut Document, objects: &[NodeId]) -> Result<NodeId, OpError> {
    let first = *objects
        .first()
        .ok_or(OpError::Customized("nothing to group"))?;
    let (_, stack) = stack_entry(doc, first)?;
    let mut members = Vec::with_capacity(objects.len());
    for &object in objects {
        let (id, other) = stack_entry(doc, object)?;
        if other != stack {
            return Err(OpError::Customized("the objects are in different stacks"));
        }
        let position = doc.graph().position_of(id).unwrap_or(0);
        members.push((position, object, id));
    }
    members.sort_unstable_by_key(|(position, ..)| *position);
    members.dedup_by_key(|(_, object, _)| *object);
    transaction(doc, "Group", |doc| {
        // In the stack, the group takes the place of its topmost member.
        let top = members.last().map_or(0, |(position, ..)| *position);
        let below = top + 1 - members.len();
        let origins: Vec<Vector> = members
            .iter()
            .map(|(_, object, _)| {
                let world = transforms::world(doc.graph(), *object);
                world.apply(koebi_geom::Point::ORIGIN).to_vector()
            })
            .collect();
        #[allow(clippy::cast_precision_loss, reason = "a count of objects")]
        let count = origins.len() as f64;
        let middle = origins.iter().fold(Vector::ZERO, |sum, v| sum + *v) / count;
        let space = transforms::space(doc.graph(), first);
        let content = add(doc, merge::KIND)?;
        let group = add(doc, group_object::KIND)?;
        // The group's own location, in the space its members had.
        let inverse_space = space.inverse().ok_or(OpError::Degenerate)?;
        let location = inverse_space.apply(middle.to_point()).to_vector();
        set_input(
            doc,
            group,
            group_object::LOCATION.index(),
            location.into_value(),
        )?;
        link(
            doc,
            SocketRef::new(content, merge::LAYER.index()),
            SocketRef::new(group, group_object::CONTENT.index()),
            None,
            None,
        )?;
        let group_world = transforms::world(doc.graph(), group);
        for (_, object, id) in &members {
            let entry = doc.graph().merge_entry(*id);
            let (world_before, parented) = (
                transforms::world(doc.graph(), *object),
                transforms::parent(doc.graph(), *object).is_some(),
            );
            doc.apply(Edit::Unlink(*id))?;
            let sockets = placement(doc.graph(), *object).ok_or(OpError::WrongKind(*object))?;
            link(
                doc,
                SocketRef::new(*object, sockets.layer),
                SocketRef::new(content, merge::LAYERS.index()),
                None,
                entry,
            )?;
            if !parented {
                keep_world(doc, *object, world_before, group_world)?;
            }
        }
        link(
            doc,
            SocketRef::new(group, group_object::LAYER.index()),
            SocketRef::new(stack, merge::LAYERS.index()),
            Some(below),
            Some(MergeEntry::default()),
        )?;
        Ok(group)
    })
}

/// Sets `object`'s parent inverse so that its world transform stays `world` in a new `space`.
fn keep_world(
    doc: &mut Document,
    object: NodeId,
    world: Affine,
    space: Affine,
) -> Result<(), OpError> {
    let local = transforms::local(doc.graph(), object).ok_or(OpError::WrongKind(object))?;
    let inverse = space.inverse().ok_or(OpError::Degenerate)?
        * world
        * local.to_affine().inverse().ok_or(OpError::Degenerate)?;
    set_parent_inverse(doc, object, inverse)
}

/// Ungroups `group`: its members take its place in its stack, their world placements kept,
/// and the group goes. Returns the members, bottom first.
///
/// # Errors
///
/// When `group` is not a group object with a stack of its own, or an edit fails.
pub fn ungroup(doc: &mut Document, group: NodeId) -> Result<Vec<NodeId>, OpError> {
    if !is_kind(doc.graph(), group, group_object::KIND.id) {
        return Err(OpError::WrongKind(group));
    }
    let (entry_id, stack) = stack_entry(doc, group)?;
    let content_links = doc
        .graph()
        .links_into(SocketRef::new(group, group_object::CONTENT.index()))
        .to_vec();
    let content = content_links
        .first()
        .and_then(|id| doc.graph().link_of(*id))
        .map(|link| link.from.node)
        .filter(|node| is_kind(doc.graph(), *node, merge::KIND.id))
        .ok_or(OpError::Customized("the group's content is not a stack"))?;
    transaction(doc, "Ungroup", |doc| {
        let position = doc.graph().position_of(entry_id).unwrap_or(0);
        let outer = transforms::space(doc.graph(), group);
        let members_links = doc
            .graph()
            .links_into(SocketRef::new(content, merge::LAYERS.index()))
            .to_vec();
        let mut members = Vec::new();
        for (offset, id) in members_links.iter().enumerate() {
            let link_value = *doc.graph().link_of(*id).ok_or(EditError::NoLink(*id))?;
            let object = link_value.from.node;
            let entry = doc.graph().merge_entry(*id);
            let world_before = transforms::world(doc.graph(), object);
            let parented = transforms::parent(doc.graph(), object).is_some();
            doc.apply(Edit::Unlink(*id))?;
            link(
                doc,
                link_value.from,
                SocketRef::new(stack, merge::LAYERS.index()),
                Some(position + offset),
                entry,
            )?;
            if !parented {
                keep_world(doc, object, world_before, outer)?;
            }
            members.push(object);
        }
        for collection in doc
            .collections()
            .of(group)
            .map(|c| c.id)
            .collect::<Vec<_>>()
        {
            edit_collection(doc, collection, |c| {
                c.members.remove(&group);
            })?;
        }
        doc.apply(Edit::RemoveNode(group))?;
        doc.apply(Edit::RemoveNode(content))?;
        Ok(members)
    })
}

/// Parents `child` to `parent`, as Blender's Ctrl+P: the child keeps its place, its own
/// location, rotation and scale unchanged, through its parent inverse.
///
/// # Errors
///
/// When either is not an object, the parent lies below the child, or an edit fails.
pub fn parent(doc: &mut Document, child: NodeId, parent: NodeId) -> Result<(), OpError> {
    let sockets = placement(doc.graph(), child).ok_or(OpError::WrongKind(child))?;
    let parent_sockets = placement(doc.graph(), parent).ok_or(OpError::WrongKind(parent))?;
    // No cycle: the child must not lie above the parent.
    let mut above = Some(parent);
    while let Some(node) = above {
        if node == child {
            return Err(OpError::Cycle);
        }
        above = transforms::parent(doc.graph(), node);
    }
    transaction(doc, "Parent", |doc| {
        let world_before = transforms::world(doc.graph(), child);
        for id in doc
            .graph()
            .links_into(SocketRef::new(child, sockets.parent))
            .to_vec()
        {
            doc.apply(Edit::Unlink(id))?;
        }
        link(
            doc,
            SocketRef::new(parent, parent_sockets.world),
            SocketRef::new(child, sockets.parent),
            None,
            None,
        )?;
        let parent_world = transforms::world(doc.graph(), parent);
        keep_world(doc, child, world_before, parent_world)
    })
}

/// The ways of clearing a parent, as Blender's Alt+P.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClearParent {
    /// The child takes its local transform in its new space (its group's, or none).
    Clear,
    /// The child keeps its world placement, baked into its location, rotation and scale.
    KeepTransform,
    /// The parent stays; its inverse goes, so the child moves by its parent's transform.
    Inverse,
}

/// Clears `child`'s parent in the way `mode` says.
///
/// # Errors
///
/// When `child` is not an object, or an edit fails.
pub fn clear_parent(doc: &mut Document, child: NodeId, mode: ClearParent) -> Result<(), OpError> {
    let sockets = placement(doc.graph(), child).ok_or(OpError::WrongKind(child))?;
    transaction(doc, "Clear parent", |doc| {
        let world_before = transforms::world(doc.graph(), child);
        if mode != ClearParent::Inverse {
            for id in doc
                .graph()
                .links_into(SocketRef::new(child, sockets.parent))
                .to_vec()
            {
                doc.apply(Edit::Unlink(id))?;
            }
        }
        set_parent_inverse(doc, child, Affine::IDENTITY)?;
        if mode == ClearParent::KeepTransform {
            let space = transforms::space(doc.graph(), child);
            let local = space.inverse().ok_or(OpError::Degenerate)? * world_before;
            set_local(doc, child, local.decompose().transform)?;
        }
        Ok(())
    })
}

/// What [`apply_transform`] bakes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Apply {
    /// The location.
    Location,
    /// The rotation.
    Rotation,
    /// The scale.
    Scale,
    /// All three.
    All,
}

/// Bakes `what` of `object`'s transform into its geometry and resets it, as Blender's Ctrl+A:
/// the object looks the same, and its children keep their world placements.
///
/// A path object's geometry gets a Transform geometry node, exact for any geometry. A group
/// bakes into its members.
///
/// # Errors
///
/// When `object` is not a path or group object, or an edit fails.
pub fn apply_transform(doc: &mut Document, object: NodeId, what: Apply) -> Result<(), OpError> {
    let is_path = is_kind(doc.graph(), object, path_object::KIND.id);
    if !is_path && !is_kind(doc.graph(), object, group_object::KIND.id) {
        return Err(OpError::WrongKind(object));
    }
    let local = transforms::local(doc.graph(), object).ok_or(OpError::WrongKind(object))?;
    let mut kept = local;
    match what {
        Apply::Location => kept.location = Vector::ZERO,
        Apply::Rotation => kept.rotation = 0.0,
        Apply::Scale => kept.scale = Vector::new(1.0, 1.0),
        Apply::All => kept = Transform::IDENTITY,
    }
    // kept × baked = local: the object's look is unchanged.
    let baked = kept.to_affine().inverse().ok_or(OpError::Degenerate)? * local.to_affine();
    transaction(doc, "Apply transform", |doc| {
        // Who must keep their place: children by parenting, and a group's members.
        let graph = doc.graph();
        let mut dependents: Vec<NodeId> = graph
            .nodes()
            .map(crate::Node::id)
            .filter(|node| transforms::parent(graph, *node) == Some(object))
            .collect();
        if !is_path {
            dependents.extend(graph.nodes().map(crate::Node::id).filter(|node| {
                transforms::parent(graph, *node).is_none()
                    && transforms::group_of(graph, *node) == Some(object)
            }));
        }
        if is_path {
            let geometry = SocketRef::new(object, path_object::GEOMETRY.index());
            if let Some(&id) = doc.graph().links_into(geometry).first() {
                let source = doc.graph().link_of(id).ok_or(EditError::NoLink(id))?.from;
                doc.apply(Edit::Unlink(id))?;
                let moved = add(doc, transform_geometry::KIND)?;
                set_input(
                    doc,
                    moved,
                    transform_geometry::TRANSFORM.index(),
                    baked.into_value(),
                )?;
                link(
                    doc,
                    source,
                    SocketRef::new(moved, transform_geometry::GEOMETRY.index()),
                    None,
                    None,
                )?;
                link(
                    doc,
                    SocketRef::new(moved, transform_geometry::RESULT.index()),
                    geometry,
                    None,
                    None,
                )?;
            }
        }
        set_local(doc, object, kept)?;
        for dependent in dependents {
            let inverse = transforms::parent_inverse(doc.graph(), dependent);
            set_parent_inverse(doc, dependent, baked * inverse)?;
        }
        Ok(())
    })
}

/// Duplicates `object` (Shift+D): its node, its geometry chain and its paints copied, what
/// feeds them from outside still linked; the copy enters the same stack just above it, and the
/// same collections. Returns the copy.
///
/// # Errors
///
/// When `object` is not a path object in one stack, or an edit fails.
pub fn duplicate(doc: &mut Document, object: NodeId) -> Result<NodeId, OpError> {
    check_path_object(doc, object)?;
    let (entry_id, stack) = stack_entry(doc, object)?;
    transaction(doc, "Duplicate", |doc| {
        // The geometry chain and the paints are the object's own: copied.
        let copy = copy_with_inputs(doc, object, &|_, input| {
            let own = input.to.index == path_object::GEOMETRY.index()
                || input.to.index == path_object::PAINTS.index();
            own.then_some(input.from)
        })?;
        enter_above(doc, object, copy, entry_id, stack)?;
        Ok(copy)
    })
}

/// Duplicates `object` linked (Alt+D): an instance object of its local layer, placed as it
/// is, entering the same stack just above it and the same collections. Returns the instance.
///
/// # Errors
///
/// When `object` is not a path object in one stack, or an edit fails.
pub fn duplicate_linked(doc: &mut Document, object: NodeId) -> Result<NodeId, OpError> {
    check_path_object(doc, object)?;
    let (entry_id, stack) = stack_entry(doc, object)?;
    transaction(doc, "Duplicate linked", |doc| {
        let instance = add(doc, instance_object::KIND)?;
        let from = placement(doc.graph(), object).ok_or(OpError::WrongKind(object))?;
        let to = placement(doc.graph(), instance).ok_or(OpError::WrongKind(instance))?;
        let source = doc
            .graph()
            .node(object)
            .ok_or(OpError::NoNode(object))?
            .clone();
        for (a, b) in [
            (from.location, to.location),
            (from.rotation, to.rotation),
            (from.scale, to.scale),
            (
                path_object::OPACITY.index(),
                instance_object::OPACITY.index(),
            ),
            (
                path_object::VISIBLE.index(),
                instance_object::VISIBLE.index(),
            ),
        ] {
            let value = source.input(a).cloned().unwrap_or(Value::None);
            set_input(doc, instance, b, value)?;
        }
        doc.apply(Edit::SetSettings {
            node: instance,
            settings: Box::new(source.settings.clone()),
        })?;
        for id in doc
            .graph()
            .links_into(SocketRef::new(object, from.parent))
            .to_vec()
        {
            let parent_link = *doc.graph().link_of(id).ok_or(EditError::NoLink(id))?;
            link(
                doc,
                parent_link.from,
                SocketRef::new(instance, to.parent),
                None,
                None,
            )?;
        }
        link(
            doc,
            SocketRef::new(object, path_object::LOCAL_LAYER.index()),
            SocketRef::new(instance, instance_object::SOURCE.index()),
            None,
            None,
        )?;
        enter_above(doc, object, instance, entry_id, stack)?;
        Ok(instance)
    })
}

/// Puts `copy` in `stack` just above `original` (whose entry is `entry_id`), with the same
/// entry, and in the same collections.
fn enter_above(
    doc: &mut Document,
    original: NodeId,
    copy: NodeId,
    entry_id: LinkId,
    stack: NodeId,
) -> Result<(), OpError> {
    let position = doc.graph().position_of(entry_id).unwrap_or(0);
    let entry = doc.graph().merge_entry(entry_id);
    let layer = placement(doc.graph(), copy)
        .ok_or(OpError::WrongKind(copy))?
        .layer;
    link(
        doc,
        SocketRef::new(copy, layer),
        SocketRef::new(stack, merge::LAYERS.index()),
        Some(position + 1),
        entry,
    )?;
    for collection in doc
        .collections()
        .of(original)
        .map(|c| c.id)
        .collect::<Vec<_>>()
    {
        edit_collection(doc, collection, |c| {
            c.members.insert(copy);
        })?;
    }
    Ok(())
}

/// Copies `node` with its incoming links: the inputs `chain` names are fed by copies of their
/// sources (recursively), the others by the same sources. Returns the copy.
fn copy_with_inputs(
    doc: &mut Document,
    node: NodeId,
    chain: &dyn Fn(&Document, &Link) -> Option<SocketRef>,
) -> Result<NodeId, OpError> {
    let copy = doc
        .graph_mut()
        .copy_node(node)
        .ok_or(OpError::NoNode(node))?;
    let id = copy.id();
    doc.apply(Edit::AddNode(Box::new(copy)))?;
    let incoming: Vec<Link> = doc
        .graph()
        .links()
        .filter(|link| link.to.node == node)
        .copied()
        .collect();
    let mut ordered: Vec<(usize, Link)> = incoming
        .into_iter()
        .map(|link| (doc.graph().position_of(link.id).unwrap_or(0), link))
        .collect();
    ordered.sort_by_key(|(position, link)| (link.to.index, *position));
    for (_, incoming) in ordered {
        let source = match chain(doc, &incoming) {
            Some(from) => {
                // A geometry chain or a paint: copied, its own sources linked as they are,
                // geometry chains copied all the way up.
                let upstream = copy_with_inputs(doc, from.node, &|doc, input| {
                    let geometry = doc
                        .graph()
                        .node(input.from.node)
                        .and_then(|n| n.kind().output(input.from.index))
                        == Some(crate::SocketType::Geometry);
                    geometry.then_some(input.from)
                })?;
                SocketRef::new(upstream, from.index)
            }
            None => incoming.from,
        };
        let entry = doc.graph().merge_entry(incoming.id);
        link(
            doc,
            source,
            SocketRef::new(id, incoming.to.index),
            None,
            entry,
        )?;
    }
    Ok(id)
}

/// Applies `change` to a copy of collection `id`, as one edit.
fn edit_collection(
    doc: &mut Document,
    id: CollectionId,
    change: impl FnOnce(&mut Collection),
) -> Result<(), OpError> {
    let mut collection = doc
        .collections()
        .get(id)
        .cloned()
        .ok_or(OpError::NoCollection(id))?;
    change(&mut collection);
    doc.apply(Edit::SetCollection(Box::new(collection)))?;
    Ok(())
}

/// Adds a collection named `name` inside `parent` (`None`: at the top); returns it.
///
/// # Errors
///
/// When `parent` does not exist.
pub fn add_collection(
    doc: &mut Document,
    name: &str,
    parent: Option<CollectionId>,
) -> Result<CollectionId, OpError> {
    transaction(doc, "Add collection", |doc| {
        let id = doc.new_collection_id();
        let mut collection = Collection::new(id, name);
        collection.parent = parent;
        doc.apply(Edit::AddCollection(Box::new(collection)))?;
        Ok(id)
    })
}

/// Removes collection `id`: the collections inside it move up to its parent; its objects stay
/// in the document.
///
/// # Errors
///
/// When it does not exist.
pub fn remove_collection(doc: &mut Document, id: CollectionId) -> Result<(), OpError> {
    let parent = doc
        .collections()
        .get(id)
        .ok_or(OpError::NoCollection(id))?
        .parent;
    transaction(doc, "Remove collection", |doc| {
        let inside: Vec<CollectionId> =
            doc.collections().children(Some(id)).map(|c| c.id).collect();
        for child in inside {
            edit_collection(doc, child, |c| c.parent = parent)?;
        }
        doc.apply(Edit::RemoveCollection(id))?;
        Ok(())
    })
}

/// Renames collection `id`.
///
/// # Errors
///
/// When it does not exist.
pub fn rename_collection(doc: &mut Document, id: CollectionId, name: &str) -> Result<(), OpError> {
    transaction(doc, "Rename collection", |doc| {
        edit_collection(doc, id, |c| name.clone_into(&mut c.name))
    })
}

/// Shows or hides collection `id` (and so its members, and the collections inside it).
///
/// # Errors
///
/// When it does not exist.
pub fn set_collection_visible(
    doc: &mut Document,
    id: CollectionId,
    visible: bool,
) -> Result<(), OpError> {
    transaction(doc, "Show or hide collection", |doc| {
        edit_collection(doc, id, |c| c.visible = visible)
    })
}

/// Lets collection `id`'s members be selected or not.
///
/// # Errors
///
/// When it does not exist.
pub fn set_collection_selectable(
    doc: &mut Document,
    id: CollectionId,
    selectable: bool,
) -> Result<(), OpError> {
    transaction(doc, "Collection selectability", |doc| {
        edit_collection(doc, id, |c| c.selectable = selectable)
    })
}

/// Tags collection `id` with `color`.
///
/// # Errors
///
/// When it does not exist.
pub fn set_collection_color(
    doc: &mut Document,
    id: CollectionId,
    color: ColorTag,
) -> Result<(), OpError> {
    transaction(doc, "Collection color", |doc| {
        edit_collection(doc, id, |c| c.color = color)
    })
}

/// Moves collection `id` inside `parent` (`None`: to the top).
///
/// # Errors
///
/// When either does not exist, or `parent` lies inside `id`.
pub fn move_collection(
    doc: &mut Document,
    id: CollectionId,
    parent: Option<CollectionId>,
) -> Result<(), OpError> {
    transaction(doc, "Move collection", |doc| {
        edit_collection(doc, id, |c| c.parent = parent)
    })
}

/// Puts `object` in collection `id` (it stays in its others).
///
/// # Errors
///
/// When the collection does not exist.
pub fn link_to_collection(
    doc: &mut Document,
    id: CollectionId,
    object: NodeId,
) -> Result<(), OpError> {
    transaction(doc, "Link to collection", |doc| {
        edit_collection(doc, id, |c| {
            c.members.insert(object);
        })
    })
}

/// Takes `object` out of collection `id`.
///
/// # Errors
///
/// When the collection does not exist.
pub fn unlink_from_collection(
    doc: &mut Document,
    id: CollectionId,
    object: NodeId,
) -> Result<(), OpError> {
    transaction(doc, "Unlink from collection", |doc| {
        edit_collection(doc, id, |c| {
            c.members.remove(&object);
        })
    })
}
