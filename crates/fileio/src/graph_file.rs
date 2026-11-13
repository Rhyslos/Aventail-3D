//! Graph files ([decision 0264](../../../docs/decisions/0264-starter-graphs.md)):
//! one material graph, with every one of your own boxes it uses, in a
//! small file. The starter graphs ship as graph files
//! (`crate::starter_graphs`), read through this same checked reader.
//!
//! A graph file is readable JSON: a format number, the graph (stored as
//! the project file stores graphs, `project::v95`) and its own boxes
//! (stored as `project::v98` stores them, each after those inside it).
//! Everything read is checked, as a project's graphs are.

use crate::limits::Bounded;
use crate::project::v95::GraphV95;
use crate::project::v98::{OwnBoxV98, own_box_of, own_boxes_from};
use matgraph::own::is_own;
use matgraph::{Graph, GraphId, GraphStore, OwnBox};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use words::{tr, trf};

/// The extension of graph files, without the dot. Generic on purpose: it
/// doesn't name the app, so it survives a rename (RENAMING.md).
pub const GRAPH_EXTENSION: &str = "matgraph";

/// The largest graph file read, in bytes.
pub const MAX_GRAPH_FILE: u64 = 16 << 20;

/// The newest version of the graph file format read. Newer files are
/// refused with a message rather than read wrongly. Version 2 (decision
/// 0281) may name a project's palette in a setting; version 3 (decision
/// 0282) may hold what is painted on a Painted box; version 4 (decision
/// 0284) may hold a photo brought in on an Image box; version 5 (decision
/// 0285) the small pictures on a Stamps box. A file holding none of these
/// is still written as version 1, so older apps keep reading it.
pub const GRAPH_FORMAT: u32 = 5;

/// The format a file holding `graphs` is written as: 5 when a box holds
/// stamps, 4 when a box holds a photo, 3 when a box holds a painting, 2 when a setting names a
/// palette (older apps can't read any of these), else 1.
pub(crate) fn format_for<'g>(graphs: impl IntoIterator<Item = &'g Graph>) -> u32 {
    let names_one = |g: &Graph| {
        g.boxes().any(|(_, b)| {
            b.settings
                .iter()
                .any(|(_, v)| matches!(v, matgraph::SettingValue::Palette(_)))
        })
    };
    let mut format = 1;
    for graph in graphs {
        if crate::project::v109::holds_stamps(graph) {
            return 5;
        }
        if crate::project::v108::holds_photo(graph) {
            format = 4;
        }
        if crate::project::v107::holds_painting(graph) {
            format = format.max(3);
        }
        if names_one(graph) {
            format = format.max(2);
        }
    }
    format
}

/// A material graph with the own boxes it uses: what a graph file holds.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphFile {
    /// The graph.
    pub graph: Graph,
    /// Every own box it uses (inside others too), each after those
    /// inside it.
    pub own: Vec<Arc<OwnBox>>,
}

#[derive(Serialize, Deserialize)]
struct Stored {
    format: u32,
    graph: GraphV95,
    #[serde(default = "Bounded::new")]
    own: Bounded<OwnBoxV98>,
}

impl GraphFile {
    /// `graph` with the own boxes it uses, found by `find` (the
    /// project's); `None` if one can't be found.
    pub fn of(graph: &Graph, find: impl Fn(&str) -> Option<Arc<OwnBox>>) -> Option<Self> {
        let mut found: BTreeMap<String, Arc<OwnBox>> = BTreeMap::new();
        let mut todo: Vec<String> = graph
            .boxes()
            .filter(|(_, b)| is_own(&b.kind))
            .map(|(_, b)| b.kind.clone())
            .collect();
        while let Some(id) = todo.pop() {
            if found.contains_key(&id) {
                continue;
            }
            let own = find(&id)?;
            todo.extend(own.uses());
            found.insert(id, own);
        }
        let mut own: Vec<Arc<OwnBox>> = Vec::new();
        let mut left: Vec<Arc<OwnBox>> = found.into_values().collect();
        while !left.is_empty() {
            let (ready, later): (Vec<_>, Vec<_>) = left
                .into_iter()
                .partition(|o| o.uses().iter().all(|u| own.iter().any(|b| &b.id == u)));
            if ready.is_empty() {
                return None;
            }
            own.extend(ready);
            left = later;
        }
        Some(Self {
            graph: graph.clone(),
            own,
        })
    }

    /// This graph file's own boxes given new ids where `taken` says the
    /// project already has one by that id that isn't the same (changed
    /// there, or another box), with the graph and the boxes inside
    /// placing them by their new ids: so copying a graph into a project
    /// never changes, or is changed by, boxes already there. Own boxes
    /// the project holds exactly as they are keep their ids (and are
    /// shared). `None` if the renamed boxes no longer check, which can't
    /// happen for a file that was read.
    pub fn fitted_to(&self, existing: impl Fn(&str) -> Option<Arc<OwnBox>>) -> Option<Self> {
        let mut renamed: BTreeMap<String, String> = BTreeMap::new();
        let mut taken: BTreeSet<String> = BTreeSet::new();
        for own in &self.own {
            taken.insert(own.id.clone());
        }
        for own in &self.own {
            let clash = existing(&own.id).is_some_and(|there| *there != **own);
            // A box inside one renamed is renamed too (its kind changes).
            let inside_renamed = own.uses().iter().any(|u| renamed.contains_key(u));
            if clash || inside_renamed {
                let id = matgraph::own::fresh_id(&own.id, |id| {
                    taken.contains(id) || existing(id).is_some()
                });
                taken.insert(id.clone());
                renamed.insert(own.id.clone(), id);
            }
        }
        if renamed.is_empty() {
            return Some(self.clone());
        }
        let rename_kinds = |parts: &mut matgraph::GraphParts| {
            for (_, b) in &mut parts.boxes {
                if let Some(new) = renamed.get(&b.kind) {
                    b.kind = new.clone();
                }
            }
        };
        let mut own: Vec<Arc<OwnBox>> = Vec::with_capacity(self.own.len());
        for o in &self.own {
            let mut parts = o.to_parts();
            if let Some(new) = renamed.get(&o.id) {
                parts.id = new.clone();
            }
            rename_kinds(&mut parts.inner);
            let registry = matgraph::own::registry_with(&own);
            own.push(Arc::new(OwnBox::from_parts(parts, &registry).ok()?));
        }
        let mut parts = self.graph.to_parts();
        rename_kinds(&mut parts);
        let graph = Graph::from_parts(parts, &matgraph::own::registry_with(&own)).ok()?;
        Some(Self { graph, own })
    }
}

/// The text of a graph file holding `file`.
pub fn write_graph_file(file: &GraphFile) -> String {
    let format = format_for(std::iter::once(&file.graph).chain(file.own.iter().map(|o| &*o.inner)));
    let stored = Stored {
        format,
        graph: GraphV95::of(GraphId(0), &file.graph),
        own: file.own.iter().map(|o| own_box_of(o)).collect(),
    };
    serde_json::to_string_pretty(&stored).unwrap_or_default()
}

/// The graph file in `bytes`, checked; or why it can't be read, in plain
/// words.
pub fn read_graph_file(bytes: &[u8]) -> Result<GraphFile, String> {
    let unreadable = || tr("it isn't a graph file, or it is damaged").to_string();
    if bytes.len() as u64 > MAX_GRAPH_FILE {
        return Err(tr("it is too large to be a graph file").to_string());
    }
    // The format number first, so a newer file says so.
    #[derive(Deserialize)]
    struct Version {
        format: u32,
    }
    let version: Version = serde_json::from_slice(bytes).map_err(|_| unreadable())?;
    if version.format > GRAPH_FORMAT {
        return Err(tr("it was made by a newer version of the app").to_string());
    }
    let stored: Stored =
        crate::limits::within_budget(|| serde_json::from_slice(bytes)).map_err(|_| {
            if crate::limits::limit_hit() {
                tr(crate::limits::TOO_MUCH).to_string()
            } else {
                unreadable()
            }
        })?;
    let own = own_boxes_from(stored.own)?;
    let store = GraphStore::new()
        .with_own(own.clone())
        .ok_or_else(unreadable)?;
    let (_, parts) = stored.graph.into_parts()?;
    let graph = Graph::from_parts(parts, store.registry())
        .map_err(|why| trf!("the material graph is damaged: {}", why))?;
    let own: Vec<Arc<OwnBox>> = own.into_iter().map(Arc::new).collect();
    // Only the own boxes the graph uses are kept.
    let file = GraphFile::of(&graph, |id| own.iter().find(|o| o.id == id).cloned())
        .ok_or_else(unreadable)?;
    Ok(file)
}

#[cfg(test)]
mod tests;
