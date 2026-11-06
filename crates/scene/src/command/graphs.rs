//! The commands for material graphs ([decision 0257](../../../../docs/decisions/0257-material-graphs-core.md)):
//! adding, replacing and removing a project's graphs, and making one
//! change to a graph (`matgraph::GraphEdit`), each one undo step.

use super::Run;
use crate::tree::{Edit, SceneError};
use matgraph::{Graph, GraphEdit, GraphId, Refusal, Registry};
use std::sync::Arc;
use words::{tr, trf};

/// `graph` checked through its parts, as a file's would be, or why it
/// can't be kept.
fn sound(graph: &Graph) -> Result<Graph, SceneError> {
    Graph::from_parts(graph.to_parts(), Registry::standard())
        .map_err(|why| SceneError::Graph(Refusal::new(trf!("That graph can't be kept: {}.", why))))
}

/// A refusal for a graph that isn't there.
fn no_such_graph() -> SceneError {
    SceneError::Graph(Refusal::new(tr(
        "That material graph isn't in the project any more.",
    )))
}

impl Run<'_> {
    /// [`super::Command::AddGraph`].
    pub(super) fn add_graph(&mut self, graph: &Graph) -> Result<(), SceneError> {
        if self.scene.graphs().len() >= matgraph::MAX_GRAPHS {
            return Err(SceneError::Graph(Refusal::new(trf!(
                "A project can hold {} material graphs; delete one first.",
                matgraph::MAX_GRAPHS
            ))));
        }
        let after = sound(graph)?;
        let id = self.scene.graphs().next_id();
        self.edit(Edit::SetGraph {
            id,
            before: None,
            after: Some(Arc::new(after)),
        })
    }

    /// [`super::Command::SetGraph`].
    pub(super) fn set_graph(
        &mut self,
        id: GraphId,
        graph: Option<&Graph>,
    ) -> Result<(), SceneError> {
        let before = self
            .scene
            .graphs()
            .get(id)
            .cloned()
            .ok_or_else(no_such_graph)?;
        let after = graph.map(sound).transpose()?;
        if after.as_ref() == Some(&*before) {
            return Ok(());
        }
        self.edit(Edit::SetGraph {
            id,
            before: Some(before),
            after: after.map(Arc::new),
        })
    }

    /// [`super::Command::EditGraph`].
    pub(super) fn edit_graph(&mut self, id: GraphId, edit: &GraphEdit) -> Result<(), SceneError> {
        let before = self
            .scene
            .graphs()
            .get(id)
            .cloned()
            .ok_or_else(no_such_graph)?;
        let mut after = (*before).clone();
        after
            .apply(Registry::standard(), edit)
            .map_err(SceneError::Graph)?;
        if after == *before {
            return Ok(());
        }
        self.edit(Edit::SetGraph {
            id,
            before: Some(before),
            after: Some(Arc::new(after)),
        })
    }
}
