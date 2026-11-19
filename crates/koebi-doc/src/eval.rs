//! Evaluation: the graph pulled into what the canvas draws, one [`Program`] per page (ADR 0028,
//! ADR 0029).
//!
//! - **Pulled** from the outputs a page needs, each node evaluated after its sources. A link
//!   that is invalid is ignored: its input's own value is used. Nodes of unknown kinds give
//!   nothing.
//! - **Cached by revision:** each output is kept with its node's revision and the versions of
//!   what it read, so an edit re-evaluates only what depends on it. Stroke outlines are kept by
//!   content (the geometry and the style, not the color), so recoloring a stroke outlines
//!   nothing; outlines placed in the world are kept by their outline and transform, so a
//!   recolored object keeps the very outline it had (the canvas keeps what it prepared from it).
//!   What reads an output is recorded once per structure of the graph, and a multi-input (a
//!   stack of thousands of layers) reads again only the sources that changed since.
//! - **World transforms** come from the chain each time: an object's space is its Parent input
//!   (its parent's world, by link), else its group's world, else the identity.
//! - **Layers** are a tree of draws, merges, blends and masks in world coordinates; a page's
//!   program flattens it into a DAG of steps, a layer shared by two consumers (an object used as
//!   a mask elsewhere) becoming one step.
//! - **Programs** are shared ([`Arc`]) and named by a revision: a page whose layers, size and
//!   background did not change keeps its program, revision included, without flattening it
//!   again, so the canvas sees at once that it has nothing to draw again.
//! - **Visibility:** an object hidden by itself or by a collection draws nothing.
//! - **Strokes in pixels** ([`StrokeUnits::Viewport`]) are not outlined here: their step,
//!   [`ViewStroke`], carries the geometry placed in the world and the style in pixels, and the
//!   canvas outlines it for each view's zoom.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use koebi_core::hash::{IdMap, IdSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use koebi_core::LinearRgba;
use koebi_geom::{
    Affine, Anchor, Cap, Contours, FillRule, Join, Path, Point, Rect, Spline, StrokeOffset,
    StrokeStyle, Subpath, Transform, Vector,
};

use crate::document::Document;
use crate::graph::{KindRef, LinkId, LinkStatus, MergeEntry, NodeId, Settings, SocketRef};
use crate::kinds::{
    blend, coverage, document, ellipse, fill, group_object, instance_object, layer_coverage, mask,
    merge, page, path_data, path_object, pattern_fill, polygon, rectangle, stroke, transform,
    transform_geometry,
};
use crate::transforms;
use crate::value::{
    BlendMode, PatternAnchor, PatternClip, SocketValue, StrokeAlign, StrokeCap, StrokeJoin,
    StrokeUnits, Value,
};

/// A filled outline in world coordinates, with its solid paint.
#[derive(Clone, Debug, PartialEq)]
pub struct Draw {
    /// The object it paints, which picking finds: an instance for what it shows of its source.
    pub object: Option<NodeId>,
    /// The outline.
    pub outline: Arc<Contours>,
    /// Its fill rule.
    pub rule: FillRule,
    /// Its color, linear with straight alpha.
    pub color: LinearRgba,
    /// Its opacity.
    pub opacity: f64,
}

/// A stroke whose lengths count the pixels of the view showing it: the canvas outlines its
/// geometry for each view, the style scaled by the view's document units per pixel, and fills
/// the outline with the non-zero rule.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewStroke {
    /// The object it paints, which picking finds: an instance for what it shows of its source.
    pub object: Option<NodeId>,
    /// The stroked geometry, in world coordinates.
    pub geometry: Arc<Contours>,
    /// The style, its lengths in pixels.
    pub style: Arc<StrokeStyle>,
    /// Its color, linear with straight alpha.
    pub color: LinearRgba,
    /// Its opacity.
    pub opacity: f64,
}

/// Where a pattern's motifs go: the points `origin + i·a + j·b` for every whole `i` and `j`,
/// each motif mapped by `motif` (its rotation and scale, and the world's when the lattice
/// follows the object) and moved to its point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lattice {
    /// The point of `i = j = 0`.
    pub origin: Point,
    /// The first axis: its direction and pitch.
    pub a: Vector,
    /// The second axis.
    pub b: Vector,
    /// What maps a motif from its own space, before it moves to its point.
    pub motif: Affine,
}

impl Lattice {
    /// This lattice mapped by `transform`: its points and axes, and its motifs with them.
    #[must_use]
    pub fn transformed(self, transform: Affine) -> Self {
        let linear = Affine::translate(-transform.translation()) * transform;
        Self {
            origin: transform.apply(self.origin),
            a: transform.apply_vector(self.a),
            b: transform.apply_vector(self.b),
            motif: linear * self.motif,
        }
    }
}

/// A motif repeated on a lattice over a region of the world: the fill of a pattern. Cut at an
/// outline, it is the content of a mask whose coverage is that outline.
#[derive(Clone, Debug, PartialEq)]
pub struct PatternValue {
    /// The object it fills, which picking finds.
    pub object: Option<NodeId>,
    /// The motif, in its own space.
    pub motif: Arc<LayerValue>,
    /// Where the motifs go, in world coordinates.
    pub lattice: Lattice,
    /// What it is pinned to: the object's transform moves the lattice, or only the region.
    pub anchor: PatternAnchor,
    /// The region to fill: motifs whose point lies in it are drawn.
    pub bounds: Rect,
}

/// Evaluated renderable content.
#[derive(Clone, Debug, PartialEq)]
pub enum LayerValue {
    /// Nothing.
    Empty,
    /// A filled outline.
    Draw(Draw),
    /// A stroke outlined for each view.
    ViewStroke(ViewStroke),
    /// A motif repeated on a lattice.
    Pattern(PatternValue),
    /// Layers composited in order, bottom first.
    Merge(Vec<(Arc<LayerValue>, MergeEntry)>),
    /// A source blended onto a backdrop.
    Blend {
        /// What lies below.
        backdrop: Arc<LayerValue>,
        /// What is blended.
        source: Arc<LayerValue>,
        /// How.
        mode: BlendMode,
        /// The source's opacity.
        opacity: f64,
        /// Whether the source composites on its own first.
        isolate: bool,
    },
    /// Content shown where a coverage is (or is not).
    Mask {
        /// The content.
        content: Arc<LayerValue>,
        /// Where it shows.
        coverage: Arc<CoverageValue>,
        /// Whether it shows outside instead.
        invert: bool,
    },
}

/// An evaluated mask.
#[derive(Clone, Debug, PartialEq)]
pub enum CoverageValue {
    /// Where an outline is filled, in world coordinates.
    Outline {
        /// The outline.
        outline: Arc<Contours>,
        /// Its fill rule.
        rule: FillRule,
    },
    /// A layer's opacity.
    Layer(Arc<LayerValue>),
}

/// An evaluated fill or stroke.
#[derive(Clone, Debug, PartialEq)]
pub enum PaintValue {
    /// A solid fill.
    Fill {
        /// Its color.
        color: LinearRgba,
        /// Its opacity.
        opacity: f64,
        /// Its fill rule.
        rule: FillRule,
        /// How it blends onto the paints below.
        blend: BlendMode,
    },
    /// A pattern: a motif on a lattice over the shape.
    Pattern {
        /// The motif, in its own space; `None` without one.
        motif: Option<Arc<LayerValue>>,
        /// The lattice's pitch along its two axes.
        spacing: Vector,
        /// Where its origin lies.
        phase: Vector,
        /// Its rotation.
        rotation: f64,
        /// Each motif's own rotation.
        motif_rotation: f64,
        /// Each motif's scale.
        scale: f64,
        /// Where it is cut.
        clip: PatternClip,
        /// What its lattice is pinned to.
        anchor: PatternAnchor,
        /// Its opacity.
        opacity: f64,
        /// How it blends onto the paints below.
        blend: BlendMode,
    },
    /// A stroke.
    Stroke {
        /// Its color.
        color: LinearRgba,
        /// Its opacity.
        opacity: f64,
        /// Its outline's style.
        style: StrokeStyle,
        /// What the style's lengths count.
        units: StrokeUnits,
        /// How it blends onto the paints below.
        blend: BlendMode,
    },
}

/// A step's index in its program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StepId(pub usize);

/// One layer of a merge step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MergeInput {
    /// The step drawn.
    pub step: StepId,
    /// Its blend mode.
    pub blend: BlendMode,
    /// Its opacity.
    pub opacity: f64,
}

/// What a mask step shows through.
#[derive(Clone, Debug, PartialEq)]
pub enum StepCoverage {
    /// Where an outline is filled.
    Outline {
        /// The outline, in world coordinates.
        outline: Arc<Contours>,
        /// Its fill rule.
        rule: FillRule,
    },
    /// A step's opacity.
    Layer(StepId),
}

/// A step of a program, over prepared geometry in f64 document coordinates.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    /// A filled outline.
    Draw(Draw),
    /// A stroke outlined for each view.
    ViewStroke(ViewStroke),
    /// The step `motif` repeated on a lattice over `bounds`.
    Pattern {
        /// The object it fills.
        object: Option<NodeId>,
        /// The motif's step, in its own space.
        motif: StepId,
        /// Where the motifs go, in world coordinates.
        lattice: Lattice,
        /// The region to fill: motifs whose point lies in it are drawn.
        bounds: Rect,
    },
    /// Steps composited in order, bottom first.
    Merge(Vec<MergeInput>),
    /// A source blended onto a backdrop.
    Blend {
        /// What lies below.
        backdrop: StepId,
        /// What is blended.
        source: StepId,
        /// How.
        mode: BlendMode,
        /// The source's opacity.
        opacity: f64,
        /// Whether the source composites on its own first.
        isolate: bool,
    },
    /// Content shown where a coverage is (or is not).
    Mask {
        /// The content.
        content: StepId,
        /// Where it shows.
        coverage: StepCoverage,
        /// Whether it shows outside instead.
        invert: bool,
    },
}

/// What the canvas draws for one page: a DAG of steps, each after the steps it reads.
///
/// A program never changes once made. Its [revision](Program::revision) names it: no two
/// programs made in the process share one, and an evaluator gives a page whose content did not
/// change the same program again. Equality compares the content, not the revision.
#[derive(Clone, Debug)]
pub struct Program {
    revision: u64,
    page: NodeId,
    size: Vector,
    background: LinearRgba,
    steps: Vec<Step>,
    root: Option<StepId>,
}

/// The next program's revision: revisions are unique in the process.
static REVISIONS: AtomicU64 = AtomicU64::new(1);

impl Program {
    /// The program of page `page` of `size` over `background`: `steps`, each after those it
    /// reads, and the step of the whole page (`None` when it draws nothing). It takes a new
    /// revision.
    pub fn new(
        page: NodeId,
        size: Vector,
        background: LinearRgba,
        steps: Vec<Step>,
        root: Option<StepId>,
    ) -> Self {
        Self {
            revision: REVISIONS.fetch_add(1, Ordering::Relaxed),
            page,
            size,
            background,
            steps,
            root,
        }
    }

    /// What names this program: no other program made in the process shares it.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The page node.
    pub fn page(&self) -> NodeId {
        self.page
    }

    /// The page's size, in document units.
    pub fn size(&self) -> Vector {
        self.size
    }

    /// The page's background color, linear with straight alpha.
    pub fn background(&self) -> LinearRgba {
        self.background
    }

    /// The steps, each after those it reads.
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// The step of the whole page; `None` when it draws nothing.
    pub fn root(&self) -> Option<StepId> {
        self.root
    }
}

impl PartialEq for Program {
    fn eq(&self, other: &Self) -> bool {
        self.page == other.page
            && self.size == other.size
            && self.background == other.background
            && self.steps == other.steps
            && self.root == other.root
    }
}

/// An evaluated output.
#[derive(Clone, Debug)]
enum Out {
    None,
    Value(Value),
    Geometry(Arc<Contours>),
    Paint(Arc<PaintValue>),
    Layer(Arc<LayerValue>),
    Coverage(Arc<CoverageValue>),
}

/// A cached output.
#[derive(Clone, Debug)]
struct Entry {
    revision: u64,
    /// Which computation gave it: one evaluator never gives two the same.
    version: u64,
    out: Out,
}

/// A multi-input's valid links, in order, and what each source gave when last read, with the
/// version of its output then: a source whose output kept its version is not read again.
#[derive(Clone, Debug, Default)]
struct Gathered {
    /// The graph structure the links were found in.
    structure: u64,
    links: Vec<(LinkId, SocketRef)>,
    outs: Vec<(u64, Out)>,
}

/// An output: a node and the output's index.
type Key = (NodeId, usize);

/// How many times things were evaluated, for tests and benchmarks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Outputs computed, by node kind id.
    pub evaluated: HashMap<String, u64>,
    /// Stroke outlines computed.
    pub outlines: u64,
}

impl Stats {
    /// Outputs computed for kind `id`.
    pub fn of(&self, id: &str) -> u64 {
        self.evaluated.get(id).copied().unwrap_or(0)
    }
}

/// Evaluates documents, keeping its caches from one evaluation to the next.
#[derive(Clone, Debug)]
pub struct Evaluator {
    cache: IdMap<Key, Entry>,
    /// The version of the next output computed.
    versions: u64,
    /// What reads each output, recorded as they are pulled: what to mark again when it changes.
    consumers: IdMap<Key, IdSet<Key>>,
    /// Outputs to compute again.
    dirty: IdSet<Key>,
    /// Each multi-input's sources as last read.
    gathered: IdMap<Key, Gathered>,
    /// The graph's clock at the last evaluation: nodes of a later revision changed since.
    seen: u64,
    /// The outputs being pulled: one met again lies on a cycle, and gives nothing.
    pulling: IdSet<Key>,
    /// The graph structure each output's readers were recorded for: links only change with it.
    registered: IdMap<Key, u64>,
    /// Stroke outlines, by geometry and style: the geometry kept, so its address names it.
    outlines: Kept<(usize, u64), (Arc<Contours>, Arc<Contours>)>,
    /// Outlines placed in the world, by the outline placed: it, the transform and the result.
    placed: PlacedOutlines,
    /// The links' statuses, and the graph structure they were computed for.
    statuses: Option<(u64, std::collections::BTreeMap<LinkId, LinkStatus>)>,
    /// What the last programs were evaluated from: the graph's clock and structure, and the
    /// collections.
    stamp: Option<((u64, u64), crate::collection::Collections)>,
    /// The last programs, in the document's order.
    programs: Vec<Arc<Program>>,
    /// Each page's last program and the layer, size and background it was made from.
    pages: IdMap<NodeId, PageProgram>,
    /// The steps the last page flattened: what the next one reserves.
    steps_hint: usize,
    /// The tolerance of stroke outlines, in document units.
    pub tolerance: f64,
    /// What was evaluated since the last [`Evaluator::reset_stats`].
    pub stats: Stats,
}

impl Default for Evaluator {
    fn default() -> Self {
        Self {
            cache: IdMap::default(),
            versions: 1,
            consumers: IdMap::default(),
            dirty: IdSet::default(),
            gathered: IdMap::default(),
            seen: 0,
            pulling: IdSet::default(),
            registered: IdMap::default(),
            outlines: Kept::default(),
            placed: PlacedOutlines::default(),
            statuses: None,
            stamp: None,
            programs: Vec::new(),
            pages: IdMap::default(),
            steps_hint: 0,
            tolerance: 1e-3,
            stats: Stats::default(),
        }
    }
}

/// The context of one evaluation: the document and its links' statuses.
struct Context<'a> {
    doc: &'a Document,
    statuses: std::collections::BTreeMap<LinkId, LinkStatus>,
}

impl Evaluator {
    /// An evaluator with empty caches.
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets the counts in [`Self::stats`].
    pub fn reset_stats(&mut self) {
        self.stats = Stats::default();
    }

    /// The program of every page of `doc`, in the document's order. Asking again when nothing
    /// changed costs a comparison and allocates nothing; a page whose content did not change
    /// keeps its program.
    pub fn programs(&mut self, doc: &Document) -> &[Arc<Program>] {
        let graph = doc.graph();
        let stamp = (graph.clock(), graph.structure());
        let unchanged = self
            .stamp
            .as_ref()
            .is_some_and(|(last, collections)| *last == stamp && collections == doc.collections());
        if !unchanged {
            self.update(doc);
        }
        &self.programs
    }

    /// Evaluates `doc` again after a change.
    fn update(&mut self, doc: &Document) {
        let graph = doc.graph();
        let stamp = (graph.clock(), graph.structure());
        // What changed since: nodes of a later revision; links whose status changed with the
        // structure; objects, when collections did.
        let changed: Vec<NodeId> = graph
            .nodes()
            .filter(|node| node.revision() > self.seen)
            .map(crate::Node::id)
            .collect();
        for node in changed {
            self.invalidate_node(node);
        }
        let statuses = match self.statuses.take() {
            Some((structure, statuses)) if structure == graph.structure() => statuses,
            Some((_, old)) => {
                let new = graph.statuses();
                let moved: Vec<NodeId> = new
                    .iter()
                    .filter(|(id, status)| old.get(id) != Some(status))
                    .filter_map(|(id, _)| graph.link_of(*id).map(|link| link.to.node))
                    .collect();
                for node in moved {
                    self.invalidate_node(node);
                }
                new
            }
            None => graph.statuses(),
        };
        if self
            .stamp
            .as_ref()
            .is_some_and(|(_, collections)| collections != doc.collections())
        {
            let objects: Vec<NodeId> = graph
                .nodes()
                .map(crate::Node::id)
                .filter(|node| transforms::placement(graph, *node).is_some())
                .collect();
            for object in objects {
                self.invalidate_node(object);
            }
        }
        self.seen = graph.clock();
        let context = Context { doc, statuses };
        self.evaluate(&context);
        self.statuses = Some((graph.structure(), context.statuses));
        self.stamp = Some((stamp, doc.collections().clone()));
        // What no output holds any more: outlines of geometry gone, pages removed.
        self.outlines
            .forget(|(geometry, _)| Arc::strong_count(geometry) == 1);
        self.placed
            .outlines
            .forget(|(outline, _, _)| Arc::strong_count(outline) == 1);
        let programs = &self.programs;
        self.pages
            .retain(|page, _| programs.iter().any(|program| program.page == *page));
    }

    fn evaluate(&mut self, context: &Context<'_>) {
        self.programs.clear();
        let doc = context.doc;
        let Some(output) = doc
            .graph()
            .nodes()
            .find(|node| node.kind().id() == document::KIND.id)
            .map(crate::Node::id)
        else {
            return;
        };
        let pages: Vec<NodeId> =
            valid_links(context, SocketRef::new(output, document::PAGES.index()))
                .filter(|(_, from)| is(context, from.node, page::KIND.id))
                .map(|(_, from)| from.node)
                .collect();
        for page_node in pages {
            let program = self.program(context, page_node);
            self.programs.push(program);
        }
    }

    /// The program of page `node`: its last one when its layer, size and background are the
    /// same.
    fn program(&mut self, context: &Context<'_>, node: NodeId) -> Arc<Program> {
        let size = match self.input(context, node, page::SIZE.index()) {
            Out::Value(Value::Vector(size)) => size,
            _ => Vector::new(297.0, 210.0),
        };
        let background = match self.input(context, node, page::BACKGROUND.index()) {
            Out::Value(Value::Color(color)) => color,
            _ => LinearRgba::new(1.0, 1.0, 1.0, 1.0),
        };
        let layer = match self.pull(context, node, page::PAGE.index()).0 {
            Out::Layer(layer) => Some(layer),
            _ => None,
        };
        if let Some(last) = self.pages.get(&node)
            && last.size == size
            && last.background == background
            && match (&last.layer, &layer) {
                (Some(old), Some(new)) => Arc::ptr_eq(old, new),
                (None, None) => true,
                _ => false,
            }
        {
            return Arc::clone(&last.program);
        }
        let mut steps = Vec::with_capacity(self.steps_hint);
        let mut seen = IdMap::default();
        seen.reserve(self.steps_hint * 2);
        let root = layer
            .as_ref()
            .and_then(|layer| flatten(layer, &mut steps, &mut seen));
        self.steps_hint = steps.len();
        let program = Arc::new(Program::new(node, size, background, steps, root));
        self.pages.insert(
            node,
            PageProgram {
                layer,
                size,
                background,
                program: Arc::clone(&program),
            },
        );
        program
    }

    /// Marks every output of `node`, and what reads them, to compute again.
    fn invalidate_node(&mut self, node: NodeId) {
        // Its outputs: the declared ones, and as many pass-through ones as inputs at most.
        let mut stack: Vec<Key> = (0..64)
            .map(|output| (node, output))
            .filter(|key| self.cache.contains_key(key))
            .collect();
        while let Some(key) = stack.pop() {
            if self.dirty.insert(key)
                && let Some(readers) = self.consumers.get(&key)
            {
                stack.extend(readers.iter().copied());
            }
        }
    }

    /// Output `output` of `node`: from the cache unless something it reads changed.
    fn pull(&mut self, context: &Context<'_>, node: NodeId, output: usize) -> (Out, u64) {
        let key = (node, output);
        let Some(current) = context.doc.graph().node(node) else {
            return (Out::None, 0);
        };
        let revision = current.revision();
        if let Some(entry) = self.cache.get(&key)
            && entry.revision == revision
            && !self.dirty.contains(&key)
        {
            return (entry.out.clone(), 0);
        }
        if !self.pulling.insert(key) {
            return (Out::None, 0);
        }
        // Who reads what: its sources, and what reaches it outside links (a group's world).
        // Links change only with the graph's structure: recorded once for each.
        let structure = context.doc.graph().structure();
        if self.registered.get(&key) != Some(&structure) {
            let reads = dependencies(context, node, output);
            for index in reads.inputs {
                for (_, from) in
                    valid_links(context, SocketRef::new(node, index)).collect::<Vec<_>>()
                {
                    self.consumers
                        .entry((from.node, from.index))
                        .or_default()
                        .insert(key);
                }
            }
            if reads.world
                && let Some(group) = transforms::group_of(context.doc.graph(), node)
            {
                let world = (group, group_object::WORLD.index());
                self.consumers.entry(world).or_default().insert(key);
            }
            self.registered.insert(key, structure);
        }
        let out = self.compute(context, node, output);
        self.pulling.remove(&key);
        *self
            .stats
            .evaluated
            .entry(current.kind().id().to_owned())
            .or_default() += 1;
        self.dirty.remove(&key);
        let version = self.versions;
        self.versions += 1;
        self.cache.insert(
            key,
            Entry {
                revision,
                version,
                out: out.clone(),
            },
        );
        (out, 0)
    }

    /// The value of single input `index` of `node`: its valid link's source, converted, or its
    /// own value.
    fn input(&mut self, context: &Context<'_>, node: NodeId, index: usize) -> Out {
        let link = valid_links(context, SocketRef::new(node, index)).next();
        if let Some((id, from)) = link {
            let out = self.pull(context, from.node, from.index).0;
            return match (out, context.statuses.get(&id)) {
                (Out::Value(value), Some(LinkStatus::Valid(conversion))) => {
                    conversion.apply(&value).map_or(Out::None, Out::Value)
                }
                (other, _) => other,
            };
        }
        match context.doc.graph().node(node).and_then(|n| n.input(index)) {
            Some(Value::None) | None => Out::None,
            Some(Value::Geometry(path)) => Out::Geometry(Arc::new(path.0.contours())),
            Some(value) => Out::Value(value.clone()),
        }
    }

    /// The values of multi-input `index` of `node`, from its valid links in order. Only the
    /// sources that changed since the last read are read again: a stack of thousands of layers
    /// where one changed reads one.
    fn inputs(&mut self, context: &Context<'_>, node: NodeId, index: usize) -> Vec<(LinkId, Out)> {
        let structure = context.doc.graph().structure();
        let mut gathered = match self.gathered.remove(&(node, index)) {
            Some(gathered) if gathered.structure == structure => gathered,
            _ => Gathered {
                structure,
                links: valid_links(context, SocketRef::new(node, index)).collect(),
                outs: Vec::new(),
            },
        };
        let known = gathered.outs.len() == gathered.links.len();
        if !known {
            gathered.outs.clear();
        }
        for (at, &(_, from)) in gathered.links.iter().enumerate() {
            let key = (from.node, from.index);
            let version = |this: &Self| this.cache.get(&key).map(|entry| entry.version);
            if known && !self.dirty.contains(&key) && version(self) == Some(gathered.outs[at].0) {
                continue;
            }
            let out = self.pull(context, from.node, from.index).0;
            let read = (version(self).unwrap_or(0), out);
            if known {
                gathered.outs[at] = read;
            } else {
                gathered.outs.push(read);
            }
        }
        let values = gathered
            .links
            .iter()
            .zip(&gathered.outs)
            .map(|(&(id, _), (_, out))| (id, out.clone()))
            .collect();
        self.gathered.insert((node, index), gathered);
        values
    }

    fn value<T: SocketValue>(
        &mut self,
        context: &Context<'_>,
        node: NodeId,
        index: usize,
    ) -> Option<T> {
        match self.input(context, node, index) {
            Out::Value(value) => T::from_value(&value),
            _ => None,
        }
    }

    fn compute(&mut self, context: &Context<'_>, node: NodeId, output: usize) -> Out {
        let Some(current) = context.doc.graph().node(node) else {
            return Out::None;
        };
        let KindRef::Static(kind) = current.kind() else {
            return Out::None;
        };
        // A pass-through output carries its input's value.
        if output >= kind.outputs.len() {
            let input = kind
                .inputs
                .iter()
                .enumerate()
                .filter(|(_, i)| i.pass_through)
                .nth(output - kind.outputs.len())
                .map(|(index, _)| index);
            return input.map_or(Out::None, |index| self.input(context, node, index));
        }
        let id = kind.id;
        let float = |this: &mut Self, index: usize, default: f64| {
            this.value::<f64>(context, node, index).unwrap_or(default)
        };
        let vector = |this: &mut Self, index: usize, default: Vector| {
            this.value::<Vector>(context, node, index)
                .unwrap_or(default)
        };
        if id == transform::KIND.id {
            let local = Transform {
                location: vector(self, transform::LOCATION.index(), Vector::ZERO),
                rotation: float(self, transform::ROTATION.index(), 0.0),
                scale: vector(self, transform::SCALE.index(), Vector::new(1.0, 1.0)),
            };
            return Out::Value(Value::Transform(local.to_affine()));
        }
        if id == rectangle::KIND.id {
            let size = vector(self, rectangle::SIZE.index(), Vector::new(100.0, 100.0));
            let radius = float(self, rectangle::CORNER_RADIUS.index(), 0.0);
            return Out::Geometry(Arc::new(rounded_rectangle(size, radius).contours()));
        }
        if id == ellipse::KIND.id {
            let radii = vector(self, ellipse::RADII.index(), Vector::new(50.0, 50.0));
            let exact = self
                .value::<bool>(context, node, ellipse::EXACT.index())
                .unwrap_or(false);
            let path = if exact {
                Path::exact_ellipse(Point::ORIGIN, radii)
            } else {
                Path::ellipse(Point::ORIGIN, radii)
            };
            return Out::Geometry(Arc::new(path.contours()));
        }
        if id == polygon::KIND.id {
            let radius = float(self, polygon::RADIUS.index(), 50.0);
            let sides = self
                .value::<i64>(context, node, polygon::SIDES.index())
                .unwrap_or(6);
            let rotation = float(self, polygon::ROTATION.index(), 0.0);
            let sides = usize::try_from(sides.clamp(3, 1024)).unwrap_or(3);
            return Out::Geometry(Arc::new(
                Path::polygon(Point::ORIGIN, radius, sides, rotation).contours(),
            ));
        }
        if id == path_data::KIND.id {
            return match &current.settings {
                Settings::Path(path) => Out::Geometry(Arc::new(path.contours())),
                _ => Out::Geometry(Arc::new(Contours::default())),
            };
        }
        if id == transform_geometry::KIND.id {
            let geometry = self.input(context, node, transform_geometry::GEOMETRY.index());
            let affine = self
                .value::<Affine>(context, node, transform_geometry::TRANSFORM.index())
                .unwrap_or(Affine::IDENTITY);
            return match geometry {
                Out::Geometry(contours) => Out::Geometry(Arc::new(contours.transformed(affine))),
                _ => Out::None,
            };
        }
        if id == fill::KIND.id {
            return Out::Paint(Arc::new(PaintValue::Fill {
                color: self
                    .value(context, node, fill::COLOR.index())
                    .unwrap_or(LinearRgba::new(0.0, 0.0, 0.0, 1.0)),
                opacity: float(self, fill::OPACITY.index(), 1.0),
                rule: self
                    .value(context, node, fill::RULE.index())
                    .unwrap_or_default(),
                blend: self
                    .value(context, node, fill::BLEND.index())
                    .unwrap_or_default(),
            }));
        }
        if id == pattern_fill::KIND.id {
            let motif = match self.input(context, node, pattern_fill::MOTIF.index()) {
                Out::Layer(layer) => Some(layer),
                _ => None,
            };
            return Out::Paint(Arc::new(PaintValue::Pattern {
                motif,
                spacing: vector(self, pattern_fill::SPACING.index(), Vector::new(40.0, 40.0)),
                phase: vector(self, pattern_fill::PHASE.index(), Vector::ZERO),
                rotation: float(self, pattern_fill::ROTATION.index(), 0.0),
                motif_rotation: float(self, pattern_fill::MOTIF_ROTATION.index(), 0.0),
                scale: float(self, pattern_fill::SCALE.index(), 1.0),
                clip: self
                    .value(context, node, pattern_fill::CLIP.index())
                    .unwrap_or_default(),
                anchor: self
                    .value(context, node, pattern_fill::ANCHOR.index())
                    .unwrap_or_default(),
                opacity: float(self, pattern_fill::OPACITY.index(), 1.0),
                blend: self
                    .value(context, node, pattern_fill::BLEND.index())
                    .unwrap_or_default(),
            }));
        }
        if id == stroke::KIND.id {
            return Out::Paint(Arc::new(self.stroke(context, node)));
        }
        if id == merge::KIND.id {
            let entries = match &current.settings {
                Settings::Merge(entries) => Some(entries),
                _ => None,
            };
            let entry = |link: &LinkId| {
                entries
                    .and_then(|entries| entries.get(link).copied())
                    .unwrap_or_default()
            };
            let layers = self
                .inputs(context, node, merge::LAYERS.index())
                .into_iter()
                .filter_map(|(link, out)| match out {
                    Out::Layer(layer) => Some((layer, entry(&link))),
                    _ => None,
                })
                .collect();
            return Out::Layer(Arc::new(LayerValue::Merge(layers)));
        }
        if id == blend::KIND.id {
            let layer = |this: &mut Self, index: usize| match this.input(context, node, index) {
                Out::Layer(layer) => layer,
                _ => Arc::new(LayerValue::Empty),
            };
            return Out::Layer(Arc::new(LayerValue::Blend {
                backdrop: layer(self, blend::BACKDROP.index()),
                source: layer(self, blend::SOURCE.index()),
                mode: self
                    .value(context, node, blend::MODE.index())
                    .unwrap_or_default(),
                opacity: float(self, blend::OPACITY.index(), 1.0),
                isolate: self
                    .value(context, node, blend::ISOLATE.index())
                    .unwrap_or(false),
            }));
        }
        if id == mask::KIND.id {
            let content = match self.input(context, node, mask::CONTENT.index()) {
                Out::Layer(layer) => layer,
                _ => Arc::new(LayerValue::Empty),
            };
            let invert = self
                .value(context, node, mask::INVERT.index())
                .unwrap_or(false);
            return match self.input(context, node, mask::COVERAGE.index()) {
                Out::Coverage(coverage) => Out::Layer(Arc::new(LayerValue::Mask {
                    content,
                    coverage,
                    invert,
                })),
                // No coverage: nothing shows, or everything when inverted.
                _ if invert => Out::Layer(content),
                _ => Out::Layer(Arc::new(LayerValue::Empty)),
            };
        }
        if id == coverage::KIND.id {
            let rule = self
                .value(context, node, coverage::RULE.index())
                .unwrap_or_default();
            return match self.input(context, node, coverage::GEOMETRY.index()) {
                Out::Geometry(outline) => {
                    Out::Coverage(Arc::new(CoverageValue::Outline { outline, rule }))
                }
                _ => Out::None,
            };
        }
        if id == layer_coverage::KIND.id {
            return match self.input(context, node, layer_coverage::LAYER.index()) {
                Out::Layer(layer) => Out::Coverage(Arc::new(CoverageValue::Layer(layer))),
                _ => Out::None,
            };
        }
        if id == page::KIND.id {
            return self.input(context, node, page::CONTENT.index());
        }
        if transforms::placement(context.doc.graph(), node).is_some() {
            return self.object(context, node, output);
        }
        Out::None
    }

    fn stroke(&mut self, context: &Context<'_>, node: NodeId) -> PaintValue {
        let float = |this: &mut Self, index: usize, default: f64| {
            this.value::<f64>(context, node, index).unwrap_or(default)
        };
        let width = float(self, stroke::WIDTH.index(), 1.0);
        let cap = match self
            .value::<StrokeCap>(context, node, stroke::CAP.index())
            .unwrap_or_default()
        {
            StrokeCap::Butt => Cap::Butt {
                tilt: float(self, stroke::CAP_TILT.index(), 0.0),
            },
            StrokeCap::Round => Cap::Round,
            StrokeCap::Square => Cap::Square,
            StrokeCap::Taper => Cap::Taper {
                length: float(self, stroke::TAPER.index(), 0.0),
            },
        };
        let join = match self
            .value::<StrokeJoin>(context, node, stroke::JOIN.index())
            .unwrap_or_default()
        {
            StrokeJoin::Miter => Join::Miter {
                limit: float(self, stroke::MITER_LIMIT.index(), 4.0),
            },
            StrokeJoin::Round => Join::Round,
            StrokeJoin::Bevel => Join::Bevel,
        };
        let align = match self
            .value::<StrokeAlign>(context, node, stroke::ALIGN.index())
            .unwrap_or_default()
        {
            StrokeAlign::Center => koebi_geom::Align::Center,
            StrokeAlign::Inside => koebi_geom::Align::Inside,
            StrokeAlign::Outside => koebi_geom::Align::Outside,
            StrokeAlign::Left => koebi_geom::Align::Left,
            StrokeAlign::Right => koebi_geom::Align::Right,
        };
        let dashes = match context.doc.graph().node(node).map(|n| &n.settings) {
            Some(Settings::Stroke(dashes)) => dashes.clone(),
            _ => koebi_geom::Dashes::default(),
        };
        PaintValue::Stroke {
            color: self
                .value(context, node, stroke::COLOR.index())
                .unwrap_or(LinearRgba::new(0.0, 0.0, 0.0, 1.0)),
            opacity: float(self, stroke::OPACITY.index(), 1.0),
            style: StrokeStyle {
                width,
                align,
                offset: StrokeOffset::Percent(float(self, stroke::OFFSET.index(), 0.5) * 100.0),
                cap,
                join,
                dashes,
            },
            units: self
                .value(context, node, stroke::UNITS.index())
                .unwrap_or_default(),
            blend: self
                .value(context, node, stroke::BLEND.index())
                .unwrap_or_default(),
        }
    }

    /// An object's outputs.
    fn object(&mut self, context: &Context<'_>, node: NodeId, output: usize) -> Out {
        let graph = context.doc.graph();
        let Some(sockets) = transforms::placement(graph, node) else {
            return Out::None;
        };
        let is_path = is(context, node, path_object::KIND.id);
        let is_group = is(context, node, group_object::KIND.id);
        if output == sockets.world {
            return Out::Value(Value::Transform(self.world(context, node)));
        }
        if is_path && output == path_object::LOCAL_GEOMETRY.index() {
            return self.input(context, node, path_object::GEOMETRY.index());
        }
        if is_path && output == path_object::WORLD_GEOMETRY.index() {
            let world = self.world(context, node);
            return match self.input(context, node, path_object::GEOMETRY.index()) {
                Out::Geometry(contours) => Out::Geometry(Arc::new(contours.transformed(world))),
                other => other,
            };
        }
        if is_path && output == path_object::LOCAL_LAYER.index() {
            return Out::Layer(self.local_layer(context, node));
        }
        if output != sockets.layer {
            return Out::None;
        }
        // The layer, placed in the world: nothing when hidden.
        let visible = self
            .value::<bool>(context, node, sockets.visible)
            .unwrap_or(true);
        if !visible || context.doc.collections().hides(node) {
            return Out::Layer(Arc::new(LayerValue::Empty));
        }
        let opacity = self
            .value::<f64>(context, node, sockets.opacity)
            .unwrap_or(1.0);
        let placed = if is_group {
            // The members are placed in the world already, the group's transform with them.
            match self.input(context, node, group_object::CONTENT.index()) {
                Out::Layer(layer) => layer,
                _ => Arc::new(LayerValue::Empty),
            }
        } else {
            let world = self.world(context, node);
            // An instance's draws are its own: picking what it shows finds it.
            let (local, object) = if is_path {
                (self.local_layer(context, node), None)
            } else {
                match self.input(context, node, instance_object::SOURCE.index()) {
                    Out::Layer(layer) => (layer, Some(node)),
                    _ => (Arc::new(LayerValue::Empty), None),
                }
            };
            Arc::new(transformed(&local, world, object, &mut self.placed))
        };
        if (opacity - 1.0).abs() < f64::EPSILON {
            Out::Layer(placed)
        } else {
            let entry = MergeEntry {
                opacity,
                ..MergeEntry::default()
            };
            Out::Layer(Arc::new(LayerValue::Merge(vec![(placed, entry)])))
        }
    }

    /// A path object's paints over its geometry, in its own space.
    fn local_layer(&mut self, context: &Context<'_>, node: NodeId) -> Arc<LayerValue> {
        let Out::Geometry(geometry) = self.input(context, node, path_object::GEOMETRY.index())
        else {
            return Arc::new(LayerValue::Empty);
        };
        let paints = self.inputs(context, node, path_object::PAINTS.index());
        // The widest stroke in document units: where a pattern cut at a stroke's edge meets it.
        let widest = paints
            .iter()
            .filter_map(|(_, paint)| match paint {
                Out::Paint(paint) => match paint.as_ref() {
                    PaintValue::Stroke {
                        style,
                        units: StrokeUnits::Document,
                        ..
                    } => Some(style.clone()),
                    _ => None,
                },
                _ => None,
            })
            .max_by(|a, b| a.width.abs().total_cmp(&b.width.abs()));
        let mut layers = Vec::with_capacity(paints.len());
        for (_, paint) in paints {
            let Out::Paint(paint) = paint else {
                continue;
            };
            let mut opacity_of_layer = 1.0;
            let (layer, blend) = match paint.as_ref() {
                PaintValue::Fill {
                    color,
                    opacity,
                    rule,
                    blend,
                } => (
                    LayerValue::Draw(Draw {
                        object: Some(node),
                        outline: geometry.clone(),
                        rule: *rule,
                        color: *color,
                        opacity: *opacity,
                    }),
                    *blend,
                ),
                PaintValue::Stroke {
                    color,
                    opacity,
                    style,
                    units: StrokeUnits::Document,
                    blend,
                } => (
                    LayerValue::Draw(Draw {
                        object: Some(node),
                        outline: self.outline(&geometry, style),
                        rule: FillRule::NonZero,
                        color: *color,
                        opacity: *opacity,
                    }),
                    *blend,
                ),
                // Outlined by the canvas, at each view's zoom.
                PaintValue::Stroke {
                    color,
                    opacity,
                    style,
                    units: StrokeUnits::Viewport,
                    blend,
                } => (
                    LayerValue::ViewStroke(ViewStroke {
                        object: Some(node),
                        geometry: geometry.clone(),
                        style: Arc::new(style.clone()),
                        color: *color,
                        opacity: *opacity,
                    }),
                    *blend,
                ),
                PaintValue::Pattern { motif: None, .. } => continue,
                PaintValue::Pattern {
                    motif: Some(motif),
                    spacing,
                    phase,
                    rotation,
                    motif_rotation,
                    scale,
                    clip,
                    anchor,
                    opacity,
                    blend,
                } => {
                    opacity_of_layer = *opacity;
                    let region = self.pattern_region(&geometry, *clip, widest.as_ref());
                    let (sin, cos) = rotation.sin_cos();
                    let pattern = PatternValue {
                        object: Some(node),
                        motif: Arc::clone(motif),
                        lattice: Lattice {
                            origin: Point::new(phase.x, phase.y),
                            a: Vector::new(cos, sin) * spacing.x,
                            b: Vector::new(-sin, cos) * spacing.y,
                            motif: Affine::rotate(rotation + motif_rotation)
                                * Affine::scale(*scale, *scale),
                        },
                        anchor: *anchor,
                        bounds: region
                            .as_ref()
                            .map_or_else(|| geometry.bounds(), |region| region.bounds()),
                    };
                    // Cut at an outline: the content of a mask that outline covers.
                    let layer = match region {
                        Some(region) => LayerValue::Mask {
                            content: Arc::new(LayerValue::Pattern(pattern)),
                            coverage: Arc::new(CoverageValue::Outline {
                                outline: region,
                                rule: FillRule::NonZero,
                            }),
                            invert: false,
                        },
                        None => LayerValue::Pattern(pattern),
                    };
                    (layer, *blend)
                }
            };
            let entry = MergeEntry {
                blend,
                opacity: opacity_of_layer,
                ..MergeEntry::default()
            };
            layers.push((Arc::new(layer), entry));
        }
        Arc::new(LayerValue::Merge(layers))
    }

    /// Where a pattern on `geometry` is cut by `clip`: nowhere at its bounds, else its outline,
    /// or that outline grown to the inner or outer edge of `widest`, the shape's widest stroke
    /// (its outline without one).
    fn pattern_region(
        &self,
        geometry: &Arc<Contours>,
        clip: PatternClip,
        widest: Option<&StrokeStyle>,
    ) -> Option<Arc<Contours>> {
        let inner = match clip {
            PatternClip::Bounds => return None,
            PatternClip::Contour => return Some(Arc::clone(geometry)),
            PatternClip::StrokeInner => true,
            PatternClip::StrokeOuter => false,
        };
        let Some(style) = widest else {
            return Some(Arc::clone(geometry));
        };
        let width = style.width.abs();
        // The band's middle, from the outline toward its outside.
        let middle = match style.align {
            koebi_geom::Align::Inside => -style.offset.resolve(width),
            koebi_geom::Align::Outside => style.offset.resolve(width),
            _ => 0.0,
        };
        let edge = if inner {
            middle - width * 0.5
        } else {
            middle + width * 0.5
        };
        Some(Arc::new(koebi_geom::offset_region(
            geometry,
            FillRule::NonZero,
            edge,
            Join::Round,
            self.tolerance,
        )))
    }

    /// The outline of `geometry` stroked in `style`, from the cache when the same geometry was
    /// stroked in the same style (the color does not count).
    fn outline(&mut self, geometry: &Arc<Contours>, style: &StrokeStyle) -> Arc<Contours> {
        let key = (
            Arc::as_ptr(geometry) as usize,
            style_key(style, self.tolerance),
        );
        if let Some((kept, outline)) = self.outlines.entries.get(&key)
            && Arc::ptr_eq(kept, geometry)
        {
            return outline.clone();
        }
        self.stats.outlines += 1;
        let outline = Arc::new(koebi_geom::outline(geometry, style, self.tolerance));
        self.outlines
            .entries
            .insert(key, (geometry.clone(), outline.clone()));
        outline
    }

    /// An object's world transform: its space (its Parent input when linked, else its group's
    /// world, else the identity), its parent inverse and its own transform.
    fn world(&mut self, context: &Context<'_>, node: NodeId) -> Affine {
        let graph = context.doc.graph();
        let Some(sockets) = transforms::placement(graph, node) else {
            return Affine::IDENTITY;
        };
        let local = Transform {
            location: self
                .value(context, node, sockets.location)
                .unwrap_or(Vector::ZERO),
            rotation: self.value(context, node, sockets.rotation).unwrap_or(0.0),
            scale: self
                .value(context, node, sockets.scale)
                .unwrap_or(Vector::new(1.0, 1.0)),
        };
        let parented = valid_links(context, SocketRef::new(node, sockets.parent))
            .next()
            .is_some();
        let space = if parented {
            self.value::<Affine>(context, node, sockets.parent)
                .unwrap_or(Affine::IDENTITY)
        } else if let Some(group) = transforms::group_of(graph, node) {
            match self.pull(context, group, group_object::WORLD.index()).0 {
                Out::Value(Value::Transform(world)) => world,
                _ => Affine::IDENTITY,
            }
        } else {
            Affine::IDENTITY
        };
        space * transforms::parent_inverse(graph, node) * local.to_affine()
    }
}

/// What an output reads.
struct Reads {
    /// Its node's inputs.
    inputs: Vec<usize>,
    /// Its object's world transform, so its group's.
    world: bool,
}

/// What output `output` of `node` reads: all its node's inputs, except for objects, whose world
/// transform reads only their placement (a group's layer would read its members, which read
/// its world), and whose own layers and geometry read what they use.
fn dependencies(context: &Context<'_>, node: NodeId, output: usize) -> Reads {
    let graph = context.doc.graph();
    let count = graph.node(node).map_or(0, |n| n.kind().input_count());
    let all = Reads {
        inputs: (0..count).collect(),
        world: false,
    };
    let Some(sockets) = transforms::placement(graph, node) else {
        return all;
    };
    let placement = vec![
        sockets.location,
        sockets.rotation,
        sockets.scale,
        sockets.parent,
    ];
    let is_group = transforms::is_kind(graph, node, group_object::KIND.id);
    if output == sockets.world {
        Reads {
            inputs: placement,
            world: true,
        }
    } else if output == sockets.layer && is_group {
        Reads {
            inputs: vec![
                group_object::CONTENT.index(),
                sockets.opacity,
                sockets.visible,
            ],
            world: false,
        }
    } else if output == sockets.layer {
        Reads { world: true, ..all }
    } else if output == path_object::LOCAL_LAYER.index()
        || output == path_object::LOCAL_GEOMETRY.index()
    {
        Reads {
            inputs: vec![path_object::GEOMETRY.index(), path_object::PAINTS.index()],
            world: false,
        }
    } else {
        Reads { world: true, ..all }
    }
}

/// The valid links into `to`, in order, with their sources.
fn valid_links<'a>(
    context: &'a Context<'_>,
    to: SocketRef,
) -> impl Iterator<Item = (LinkId, SocketRef)> + 'a {
    let graph = context.doc.graph();
    graph
        .links_into(to)
        .iter()
        .filter(move |id| matches!(context.statuses.get(id), Some(LinkStatus::Valid(_))))
        .filter_map(move |id| graph.link_of(*id).map(|link| (*id, link.from)))
}

fn is(context: &Context<'_>, node: NodeId, id: &str) -> bool {
    transforms::is_kind(context.doc.graph(), node, id)
}

/// A cache whose dead entries are forgotten in amortized constant time: a sweep runs once the
/// cache has doubled since the last one left it.
#[derive(Clone, Debug)]
struct Kept<K, V> {
    entries: IdMap<K, V>,
    /// The entries the last sweep kept.
    kept: usize,
}

impl<K, V> Default for Kept<K, V> {
    fn default() -> Self {
        Self {
            entries: IdMap::default(),
            kept: 0,
        }
    }
}

impl<K: Eq + Hash, V> Kept<K, V> {
    /// Forgets the entries `dead` names, when the cache has doubled since the last sweep.
    fn forget(&mut self, dead: impl Fn(&V) -> bool) {
        if self.entries.len() > 2 * self.kept + 64 {
            self.entries.retain(|_, value| !dead(value));
            self.kept = self.entries.len();
        }
    }
}

/// Outlines placed in the world, by the outline placed (its address, the outline kept so that
/// the address names it): the transform and the placed outline. An outline placed again by the
/// same transform gives the very outline it gave.
#[derive(Clone, Debug, Default)]
struct PlacedOutlines {
    outlines: Kept<usize, (Arc<Contours>, Affine, Arc<Contours>)>,
}

impl PlacedOutlines {
    /// `outline` moved by `transform`: the very outline when the transform moves nothing.
    fn get(&mut self, outline: &Arc<Contours>, transform: Affine) -> Arc<Contours> {
        if transform == Affine::IDENTITY {
            return Arc::clone(outline);
        }
        let key = Arc::as_ptr(outline) as usize;
        if let Some((kept, kept_transform, placed)) = self.outlines.entries.get(&key)
            && Arc::ptr_eq(kept, outline)
            && *kept_transform == transform
        {
            return Arc::clone(placed);
        }
        let placed = Arc::new(outline.transformed(transform));
        self.outlines
            .entries
            .insert(key, (Arc::clone(outline), transform, Arc::clone(&placed)));
        placed
    }
}

/// A page's last program and what it was made from.
#[derive(Clone, Debug)]
struct PageProgram {
    /// The page's layer: the program flattens it.
    layer: Option<Arc<LayerValue>>,
    size: Vector,
    background: LinearRgba,
    program: Arc<Program>,
}

/// `layer` with every outline moved by `transform`, through `placed`, its draws given to
/// `object` when there is one.
fn transformed(
    layer: &LayerValue,
    transform: Affine,
    object: Option<NodeId>,
    placed: &mut PlacedOutlines,
) -> LayerValue {
    if transform == Affine::IDENTITY && object.is_none() {
        return layer.clone();
    }
    let mut again =
        |layer: &Arc<LayerValue>| Arc::new(transformed(layer, transform, object, placed));
    match layer {
        LayerValue::Empty => LayerValue::Empty,
        LayerValue::Draw(draw) => LayerValue::Draw(Draw {
            object: object.or(draw.object),
            outline: placed.get(&draw.outline, transform),
            ..draw.clone()
        }),
        // The geometry moves; the style's pixels stay pixels.
        LayerValue::ViewStroke(stroke) => LayerValue::ViewStroke(ViewStroke {
            object: object.or(stroke.object),
            geometry: placed.get(&stroke.geometry, transform),
            ..stroke.clone()
        }),
        // The region moves; the lattice with it when pinned to the object, the motif never
        // (the lattice maps it).
        LayerValue::Pattern(pattern) => LayerValue::Pattern(PatternValue {
            object: object.or(pattern.object),
            lattice: match pattern.anchor {
                PatternAnchor::Object => pattern.lattice.transformed(transform),
                PatternAnchor::Document => pattern.lattice,
            },
            bounds: pattern.bounds.transformed(transform),
            ..pattern.clone()
        }),
        LayerValue::Merge(layers) => LayerValue::Merge(
            layers
                .iter()
                .map(|(layer, entry)| (again(layer), *entry))
                .collect(),
        ),
        LayerValue::Blend {
            backdrop,
            source,
            mode,
            opacity,
            isolate,
        } => LayerValue::Blend {
            backdrop: again(backdrop),
            source: again(source),
            mode: *mode,
            opacity: *opacity,
            isolate: *isolate,
        },
        LayerValue::Mask {
            content,
            coverage,
            invert,
        } => {
            let content = again(content);
            let coverage = Arc::new(match coverage.as_ref() {
                CoverageValue::Outline { outline, rule } => CoverageValue::Outline {
                    outline: placed.get(outline, transform),
                    rule: *rule,
                },
                CoverageValue::Layer(layer) => {
                    CoverageValue::Layer(Arc::new(transformed(layer, transform, object, placed)))
                }
            });
            LayerValue::Mask {
                content,
                coverage,
                invert: *invert,
            }
        }
    }
}

/// Appends the steps of `layer` to `steps`, each after those it reads, a layer met twice (the
/// same `Arc`) giving one step; returns its step, `None` when it draws nothing.
fn flatten(
    layer: &Arc<LayerValue>,
    steps: &mut Vec<Step>,
    seen: &mut IdMap<*const LayerValue, Option<StepId>>,
) -> Option<StepId> {
    let key = Arc::as_ptr(layer);
    if let Some(step) = seen.get(&key) {
        return *step;
    }
    let step = match layer.as_ref() {
        LayerValue::Empty => None,
        LayerValue::Draw(draw) => Some(Step::Draw(draw.clone())),
        LayerValue::ViewStroke(stroke) => Some(Step::ViewStroke(stroke.clone())),
        LayerValue::Pattern(pattern) => {
            flatten(&pattern.motif, steps, seen).map(|motif| Step::Pattern {
                object: pattern.object,
                motif,
                lattice: pattern.lattice,
                bounds: pattern.bounds,
            })
        }
        // One layer at full opacity, blended normally, is that layer: no step, nothing gathered.
        LayerValue::Merge(layers)
            if matches!(layers.as_slice(), [(_, entry)] if entry.visible
                && entry.blend == BlendMode::Normal
                && (entry.opacity - 1.0).abs() < f64::EPSILON) =>
        {
            let single = flatten(&layers[0].0, steps, seen);
            seen.insert(key, single);
            return single;
        }
        LayerValue::Merge(layers) => {
            let inputs: Vec<MergeInput> = layers
                .iter()
                .filter(|(_, entry)| entry.visible)
                .filter_map(|(layer, entry)| {
                    flatten(layer, steps, seen).map(|step| MergeInput {
                        step,
                        blend: entry.blend,
                        opacity: entry.opacity,
                    })
                })
                .collect();
            match inputs.as_slice() {
                [] => None,
                // One layer at full opacity, blended normally, is that layer.
                [single]
                    if single.blend == BlendMode::Normal
                        && (single.opacity - 1.0).abs() < f64::EPSILON =>
                {
                    seen.insert(key, Some(single.step));
                    return Some(single.step);
                }
                _ => Some(Step::Merge(inputs)),
            }
        }
        LayerValue::Blend {
            backdrop,
            source,
            mode,
            opacity,
            isolate,
        } => {
            let backdrop = flatten(backdrop, steps, seen);
            let source = flatten(source, steps, seen);
            match (backdrop, source) {
                (Some(backdrop), Some(source)) => Some(Step::Blend {
                    backdrop,
                    source,
                    mode: *mode,
                    opacity: *opacity,
                    isolate: *isolate,
                }),
                (backdrop, None) => {
                    seen.insert(key, backdrop);
                    return backdrop;
                }
                (None, Some(source)) => Some(Step::Merge(vec![MergeInput {
                    step: source,
                    blend: *mode,
                    opacity: *opacity,
                }])),
            }
        }
        LayerValue::Mask {
            content,
            coverage,
            invert,
        } => match flatten(content, steps, seen) {
            None => None,
            Some(content) => {
                let coverage = match coverage.as_ref() {
                    CoverageValue::Outline { outline, rule } => StepCoverage::Outline {
                        outline: outline.clone(),
                        rule: *rule,
                    },
                    CoverageValue::Layer(layer) => match flatten(layer, steps, seen) {
                        Some(step) => StepCoverage::Layer(step),
                        // An empty coverage: nothing shows, or all of it when inverted.
                        None if *invert => {
                            seen.insert(key, Some(content));
                            return Some(content);
                        }
                        None => {
                            seen.insert(key, None);
                            return None;
                        }
                    },
                };
                Some(Step::Mask {
                    content,
                    coverage,
                    invert: *invert,
                })
            }
        },
    };
    let id = step.map(|step| {
        steps.push(step);
        StepId(steps.len() - 1)
    });
    seen.insert(key, id);
    id
}

/// A rectangle of `size` centered on the origin, its corners rounded by `radius` (quarter
/// circles as cubics, as the editable ellipse's).
fn rounded_rectangle(size: Vector, radius: f64) -> Path {
    let (half_w, half_h) = (size.x.abs() * 0.5, size.y.abs() * 0.5);
    let bounds = Rect::new(Point::new(-half_w, -half_h), Point::new(half_w, half_h));
    let radius = radius.clamp(0.0, half_w.min(half_h));
    if radius <= 0.0 {
        return Path::rectangle(bounds);
    }
    // The handle of a quarter circle as a cubic.
    let handle = radius * 0.552_284_749_830_793_4;
    let corner = |x: f64, y: f64, handle_in: Vector, handle_out: Vector| Anchor {
        handle_in: Some(handle_in),
        handle_out: Some(handle_out),
        ..Anchor::corner(Point::new(x, y))
    };
    let (left, right, top, bottom) = (-half_w, half_w, -half_h, half_h);
    let anchors = vec![
        corner(left + radius, top, Vector::new(-handle, 0.0), Vector::ZERO),
        corner(right - radius, top, Vector::ZERO, Vector::new(handle, 0.0)),
        corner(right, top + radius, Vector::new(0.0, -handle), Vector::ZERO),
        corner(
            right,
            bottom - radius,
            Vector::ZERO,
            Vector::new(0.0, handle),
        ),
        corner(
            right - radius,
            bottom,
            Vector::new(handle, 0.0),
            Vector::ZERO,
        ),
        corner(
            left + radius,
            bottom,
            Vector::ZERO,
            Vector::new(-handle, 0.0),
        ),
        corner(
            left,
            bottom - radius,
            Vector::new(0.0, handle),
            Vector::ZERO,
        ),
        corner(left, top + radius, Vector::ZERO, Vector::new(0.0, -handle)),
    ];
    Path {
        subpaths: vec![Subpath {
            anchors,
            closed: true,
            spline: Spline::Bezier,
        }],
    }
}

/// A key for a stroke style and a tolerance: everything its outline depends on.
fn style_key(style: &StrokeStyle, tolerance: f64) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let mut add = |tag: u64, value: f64| {
        tag.hash(&mut hasher);
        value.to_bits().hash(&mut hasher);
    };
    add(0, style.width);
    add(1, tolerance);
    add(2, style.offset.resolve(1.0));
    add(3, f64::from(style.align as u8));
    match style.cap {
        Cap::Butt { tilt } => add(4, tilt),
        Cap::Round => add(5, 0.0),
        Cap::Square => add(6, 0.0),
        Cap::Taper { length } => add(7, length),
    }
    match style.join {
        Join::Miter { limit } => add(8, limit),
        Join::Round => add(9, 0.0),
        Join::Bevel => add(10, 0.0),
    }
    add(11, style.dashes.offset);
    add(12, f64::from(style.dashes.fit as u8));
    for length in &style.dashes.pattern {
        add(13, *length);
    }
    for anchor in &style.dashes.anchors {
        #[allow(clippy::cast_precision_loss, reason = "a contour's index")]
        add(14, anchor.contour as f64);
        add(15, anchor.at);
        add(16, f64::from(anchor.center as u8));
        add(17, anchor.size.unwrap_or(-1.0));
    }
    hasher.finish()
}
