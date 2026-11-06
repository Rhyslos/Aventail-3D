//! A project's material graphs, by id.
//!
//! Each graph is shared ([`Arc`]), so an undo step holding the graph
//! before and after a change costs only the graph that changed. Ids are
//! given out in order and never reused, so a painter layer or a material
//! can name a graph and keep naming it.

use crate::graph::Graph;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The most graphs a project may hold.
pub const MAX_GRAPHS: usize = 1000;

/// A graph's id in its project: never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphId(pub u64);

/// A project's material graphs.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphStore {
    graphs: BTreeMap<GraphId, Arc<Graph>>,
    next_id: u64,
}

impl Default for GraphStore {
    fn default() -> Self {
        Self {
            graphs: BTreeMap::new(),
            next_id: 1,
        }
    }
}

impl GraphStore {
    /// No graphs.
    pub fn new() -> Self {
        Self::default()
    }

    /// Graph `id`.
    pub fn get(&self, id: GraphId) -> Option<&Arc<Graph>> {
        self.graphs.get(&id)
    }

    /// Every graph, by id.
    pub fn iter(&self) -> impl Iterator<Item = (GraphId, &Arc<Graph>)> {
        self.graphs.iter().map(|(id, g)| (*id, g))
    }

    /// How many graphs it holds.
    pub fn len(&self) -> usize {
        self.graphs.len()
    }

    /// Whether it holds none.
    pub fn is_empty(&self) -> bool {
        self.graphs.is_empty()
    }

    /// The id the next graph added gets.
    pub fn next_id(&self) -> GraphId {
        GraphId(self.next_id)
    }

    /// Puts `graph` in as graph `id` (`None` takes it out). The next id is
    /// raised past `id`, so it is never given out again.
    pub fn put(&mut self, id: GraphId, graph: Option<Arc<Graph>>) {
        self.next_id = self.next_id.max(id.0.saturating_add(1));
        match graph {
            Some(g) => {
                self.graphs.insert(id, g);
            }
            None => {
                self.graphs.remove(&id);
            }
        }
    }

    /// Rebuilds a store from a file: ids unique and not 0, at most
    /// [`MAX_GRAPHS`]; `next_id` is raised past every id.
    pub fn from_graphs(
        graphs: impl IntoIterator<Item = (GraphId, Graph)>,
        next_id: u64,
    ) -> Option<Self> {
        let mut store = Self {
            graphs: BTreeMap::new(),
            next_id: next_id.max(1),
        };
        for (id, graph) in graphs {
            if id.0 == 0 || id.0 == u64::MAX || store.graphs.contains_key(&id) {
                return None;
            }
            if store.graphs.len() >= MAX_GRAPHS {
                return None;
            }
            store.put(id, Some(Arc::new(graph)));
        }
        Some(store)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_never_given_out_twice() {
        let mut store = GraphStore::new();
        let first = store.next_id();
        store.put(first, Some(Arc::new(Graph::new("Stone"))));
        let second = store.next_id();
        assert_ne!(first, second);
        store.put(first, None);
        assert_eq!(store.next_id(), second);
        assert!(store.get(first).is_none());
    }

    #[test]
    fn stores_from_files_are_checked() {
        let g = || Graph::new("Wood");
        assert!(GraphStore::from_graphs([(GraphId(0), g())], 5).is_none());
        assert!(GraphStore::from_graphs([(GraphId(2), g()), (GraphId(2), g())], 5).is_none());
        let store = GraphStore::from_graphs([(GraphId(7), g())], 3).unwrap();
        assert_eq!(store.next_id(), GraphId(8));
    }
}
