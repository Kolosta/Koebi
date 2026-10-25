//! The active view, materialized: every token's value in the active theme with every layer.
//!
//! Reading a token is an array access. When a definition changes, only the token and the
//! tokens that may depend on it are resolved again: a reverse dependency graph (compressed
//! rows of dependents) lists, for each token, the tokens whose definition in some layer of
//! the active theme aliases it. Buffers are reused, so updates do not allocate once warm.

use crate::graph::Graph;
use crate::layers::Layers;
use crate::token::TokenId;
use crate::value::{Definition, Value};

#[derive(Debug)]
pub(crate) struct Active {
    values: Vec<Value>,
    /// Dependents of token `t`: `edges[offsets[t]..offsets[t + 1]]`.
    offsets: Vec<u32>,
    edges: Vec<TokenId>,
    /// Write position of each row while the rows are filled.
    cursor: Vec<u32>,
    stack: Vec<TokenId>,
    visited: Vec<u64>,
    touched: Vec<TokenId>,
    refreshed: usize,
}

impl Active {
    pub(crate) fn new(graph: &Graph) -> Self {
        let mut active = Self {
            values: Vec::with_capacity(graph.len()),
            offsets: Vec::new(),
            edges: Vec::new(),
            cursor: Vec::new(),
            stack: Vec::new(),
            visited: vec![0; graph.len().div_ceil(64)],
            touched: Vec::new(),
            refreshed: 0,
        };
        active.rebuild(graph);
        active
    }

    pub(crate) fn value(&self, token: TokenId) -> Option<&Value> {
        self.values.get(token.index())
    }

    /// Number of tokens resolved by the last refresh.
    pub(crate) fn refreshed(&self) -> usize {
        self.refreshed
    }

    /// Rebuilds the dependency graph and every value (theme switch).
    pub(crate) fn rebuild(&mut self, graph: &Graph) {
        self.rebuild_dependents(graph);
        self.refresh_all(graph);
    }

    /// Rebuilds the reverse dependency graph of the active theme.
    pub(crate) fn rebuild_dependents(&mut self, graph: &Graph) {
        let len = graph.len();
        let slot = graph.active_slot();
        let targets = |index: usize| {
            let token = TokenId::from_index(u16::try_from(index).unwrap_or(u16::MAX));
            let mut found = [None; 4];
            let candidates = [
                Some(&graph.base[index]),
                slot.definition.get(token),
                slot.overrides.get(token),
                graph.global.get(token),
            ];
            for (cell, candidate) in found.iter_mut().zip(candidates) {
                *cell = candidate.and_then(Definition::alias_target);
            }
            found
        };

        self.offsets.clear();
        self.offsets.resize(len + 1, 0);
        for index in 0..len {
            let found = targets(index);
            for (i, target) in found.iter().enumerate() {
                if let Some(target) = target
                    && !found[..i].contains(&Some(*target))
                {
                    self.offsets[target.index() + 1] += 1;
                }
            }
        }
        for i in 0..len {
            self.offsets[i + 1] += self.offsets[i];
        }
        self.edges.clear();
        self.edges
            .resize(self.offsets[len] as usize, TokenId::from_index(0));
        self.cursor.clear();
        self.cursor.extend_from_slice(&self.offsets[..len]);
        for index in 0..len {
            let found = targets(index);
            for (i, target) in found.iter().enumerate() {
                if let Some(target) = target
                    && !found[..i].contains(&Some(*target))
                {
                    let slot = &mut self.cursor[target.index()];
                    self.edges[*slot as usize] =
                        TokenId::from_index(u16::try_from(index).unwrap_or(u16::MAX));
                    *slot += 1;
                }
            }
        }
    }

    /// Resolves every token again (color vision change).
    pub(crate) fn refresh_all(&mut self, graph: &Graph) {
        let len = graph.len();
        self.values.clear();
        self.values.extend((0..len).map(|index| {
            let token = TokenId::from_index(u16::try_from(index).unwrap_or(u16::MAX));
            graph.value(token, graph.active, Layers::ALL)
        }));
        self.refreshed = len;
    }

    /// Resolves `token` and every token that may depend on it again.
    pub(crate) fn refresh_from(&mut self, graph: &Graph, token: TokenId) {
        self.stack.clear();
        self.touched.clear();
        self.mark(token);
        self.stack.push(token);
        while let Some(current) = self.stack.pop() {
            self.values[current.index()] = graph.value(current, graph.active, Layers::ALL);
            let (start, end) = (
                self.offsets[current.index()] as usize,
                self.offsets[current.index() + 1] as usize,
            );
            for edge in start..end {
                let dependent = self.edges[edge];
                if !self.is_marked(dependent) {
                    self.mark(dependent);
                    self.stack.push(dependent);
                }
            }
        }
        self.refreshed = self.touched.len();
        for touched in self.touched.drain(..) {
            let index = touched.index();
            self.visited[index / 64] &= !(1 << (index % 64));
        }
    }

    fn mark(&mut self, token: TokenId) {
        let index = token.index();
        self.visited[index / 64] |= 1 << (index % 64);
        self.touched.push(token);
    }

    fn is_marked(&self, token: TokenId) -> bool {
        let index = token.index();
        self.visited[index / 64] & (1 << (index % 64)) != 0
    }
}
