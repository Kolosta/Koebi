//! The document: its graph and its collections, changed only through edits, grouped in
//! transactions kept in a bounded history (ADR 0028, ADR 0032).
//!
//! An open transaction is live: its edits show in the document as they apply. Committing it
//! makes it one undo step; cancelling it reverts its edits. A continuous gesture (a drag, a
//! slider) keeps one transaction open: setting the same thing again keeps only the first
//! inverse, so that undoing goes back to before the gesture in one step.

use std::collections::VecDeque;

use crate::collection::{CollectionId, Collections};
use crate::edit::{self, Edit, EditError, PlacedLink};
use crate::graph::{Graph, KindRef, Link, LinkId, NodeId, SocketRef};
use crate::kind::NodeKind;

/// How many steps the history keeps by default (Settings › General).
pub const HISTORY: usize = 256;

/// A transaction: a label, and the edits that revert it, in the order to apply them.
#[derive(Clone, Debug, PartialEq)]
pub struct Transaction {
    /// What it does, for the Edit menu ("Add rectangle").
    pub label: String,
    /// The edits that revert it, in order.
    revert: Vec<Edit>,
    /// Whether it changes the document (selection and mode steps do not).
    marks_changed: bool,
}

/// A document: a graph, collections and their history.
#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    graph: Graph,
    collections: Collections,
    open: Option<Transaction>,
    undo: VecDeque<Transaction>,
    redo: Vec<Transaction>,
    limit: usize,
    changed: bool,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    /// An empty document.
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            collections: Collections::default(),
            open: None,
            undo: VecDeque::new(),
            redo: Vec::new(),
            limit: HISTORY,
            changed: false,
        }
    }

    /// Its graph.
    pub const fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Its collections.
    pub const fn collections(&self) -> &Collections {
        &self.collections
    }

    /// The graph, for what edits do not cover: fresh ids, and node values made before they are
    /// added.
    pub(crate) const fn graph_mut(&mut self) -> &mut Graph {
        &mut self.graph
    }

    /// Adds a node of the application's `kind`, as an edit; returns its id.
    ///
    /// # Errors
    ///
    /// When the edit does not apply (it always does on a well-formed document).
    pub fn add_node(&mut self, kind: &'static NodeKind) -> Result<NodeId, EditError> {
        let node = self.graph.new_node(KindRef::Static(kind), kind.name);
        let id = node.id();
        self.apply(Edit::AddNode(Box::new(node)))?;
        Ok(id)
    }

    /// Links output `from` to input `to`, as edits: appended to a multi-input; a single
    /// input's link unlinked first. Returns the new link.
    ///
    /// # Errors
    ///
    /// When a node or socket does not exist.
    pub fn link(&mut self, from: SocketRef, to: SocketRef) -> Result<LinkId, EditError> {
        let multi = self
            .graph
            .node(to.node)
            .ok_or(EditError::NoNode(to.node))?
            .kind()
            .input(to.index)
            .map(|(_, multi)| multi);
        if multi == Some(false) {
            for old in self.graph.links_into(to).to_vec() {
                self.apply(Edit::Unlink(old))?;
            }
        }
        let id = self.graph.new_link_id();
        self.apply(Edit::Link(PlacedLink {
            link: Link { id, from, to },
            position: usize::MAX,
            entry: None,
        }))?;
        Ok(id)
    }

    /// A fresh collection id.
    pub fn new_collection_id(&mut self) -> CollectionId {
        CollectionId(self.graph.new_id())
    }

    /// Opens a transaction named `label`. One already open is committed first.
    pub fn begin(&mut self, label: impl Into<String>) {
        self.commit();
        self.open = Some(Transaction {
            label: label.into(),
            revert: Vec::new(),
            marks_changed: true,
        });
    }

    /// Opens a transaction that does not mark the document as changed: a selection or a mode
    /// step, which the history keeps as Carto and Blender do.
    pub fn begin_view(&mut self, label: impl Into<String>) {
        self.begin(label);
        if let Some(open) = &mut self.open {
            open.marks_changed = false;
        }
    }

    /// Applies `edit`: in the open transaction, or in one of its own.
    ///
    /// # Errors
    ///
    /// When the edit does not apply; the document is unchanged.
    pub fn apply(&mut self, edit: Edit) -> Result<(), EditError> {
        let own = self.open.is_none();
        if own {
            self.begin("Edit");
        }
        let target = edit.target();
        let result = edit::apply(&mut self.graph, &mut self.collections, edit);
        if let (Ok(inverse), Some(open)) = (&result, &mut self.open) {
            // A gesture setting the same thing again: the first inverse already reverts it.
            let again = target.is_some() && open.revert.iter().any(|e| e.target() == target);
            if !again {
                open.revert.insert(0, inverse.clone());
            }
        }
        if own {
            self.commit();
        }
        result.map(|_| ())
    }

    /// Whether a transaction is open.
    pub const fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// Ends the open transaction as one undo step, if it changed anything.
    pub fn commit(&mut self) {
        let Some(transaction) = self.open.take() else {
            return;
        };
        if transaction.revert.is_empty() {
            return;
        }
        self.changed |= transaction.marks_changed;
        self.redo.clear();
        self.undo.push_back(transaction);
        while self.undo.len() > self.limit {
            self.undo.pop_front();
        }
    }

    /// Ends the open transaction by reverting its edits.
    pub fn cancel(&mut self) {
        if let Some(transaction) = self.open.take() {
            self.toggle(transaction);
        }
    }

    /// Undoes the last step. False when there is none.
    pub fn undo(&mut self) -> bool {
        self.commit();
        let Some(transaction) = self.undo.pop_back() else {
            return false;
        };
        let marks = transaction.marks_changed;
        let redone = self.toggle(transaction);
        self.changed |= marks;
        self.redo.push(redone);
        true
    }

    /// Redoes the last undone step. False when there is none.
    pub fn redo(&mut self) -> bool {
        self.commit();
        let Some(transaction) = self.redo.pop() else {
            return false;
        };
        let marks = transaction.marks_changed;
        let undone = self.toggle(transaction);
        self.changed |= marks;
        self.undo.push_back(undone);
        true
    }

    /// Applies a transaction's edits, and returns the transaction that reverts them.
    fn toggle(&mut self, transaction: Transaction) -> Transaction {
        let mut back = Vec::with_capacity(transaction.revert.len());
        for edit in transaction.revert {
            // Inverses of edits that applied always apply: the document is as they left it.
            let inverse = edit::apply(&mut self.graph, &mut self.collections, edit)
                .expect("an inverse applies to the document its edit left");
            back.insert(0, inverse);
        }
        Transaction {
            label: transaction.label,
            revert: back,
            marks_changed: transaction.marks_changed,
        }
    }

    /// The labels of the steps that can be undone, oldest first.
    pub fn undo_labels(&self) -> impl Iterator<Item = &str> {
        self.undo.iter().map(|t| t.label.as_str())
    }

    /// The labels of the steps that can be redone, next first.
    pub fn redo_labels(&self) -> impl Iterator<Item = &str> {
        self.redo.iter().rev().map(|t| t.label.as_str())
    }

    /// Keeps at most `limit` steps (at least one).
    pub fn set_history_limit(&mut self, limit: usize) {
        self.limit = limit.max(1);
        while self.undo.len() > self.limit {
            self.undo.pop_front();
        }
    }

    /// Makes what the document holds its start, as a new or opened document's: nothing to undo
    /// or redo, nothing changed. A transaction still open stays so.
    pub fn reset_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.changed = false;
    }

    /// Whether the document changed since it was made, opened or saved.
    pub const fn is_changed(&self) -> bool {
        self.changed
    }

    /// Marks the document as saved.
    pub const fn mark_saved(&mut self) {
        self.changed = false;
    }

    /// The graph and collections alone, without history: for comparing documents.
    pub fn content(&self) -> (&Graph, &Collections) {
        (&self.graph, &self.collections)
    }
}
