//! The commands for material graphs ([decision 0257](../../../../docs/decisions/0257-material-graphs-core.md)):
//! adding, replacing and removing a project's graphs, and making one
//! change to a graph (`matgraph::GraphEdit`), each one undo step; and the
//! project's own boxes ([decision 0263](../../../../docs/decisions/0263-your-own-boxes.md)):
//! adding, replacing and removing one, and changing the boxes inside it.

use super::Run;
use crate::tree::{Edit, SceneError};
use matgraph::{Graph, GraphEdit, GraphId, OwnBox, Refusal, Registry};
use std::collections::BTreeSet;
use std::sync::Arc;
use words::{tr, trf};

/// `graph` checked through its parts, as a file's would be, or why it
/// can't be kept.
fn sound(graph: &Graph, registry: &Registry) -> Result<Graph, SceneError> {
    Graph::from_parts(graph.to_parts(), registry)
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
        let after = sound(graph, &self.scene.graphs().registry().clone())?;
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
        let registry = self.scene.graphs().registry().clone();
        let after = graph.map(|g| sound(g, &registry)).transpose()?;
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
        let registry = self.scene.graphs().registry().clone();
        after.apply(&registry, edit).map_err(SceneError::Graph)?;
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

/// A refusal for an own box that isn't there.
fn no_such_own_box() -> SceneError {
    SceneError::Graph(Refusal::new(tr(
        "That box of your own isn't in the project any more.",
    )))
}

/// The own boxes `id` holds, through every own box inside it, with
/// `own`'s definition used for `own.id`.
fn inside_of(scene: &crate::Scene, own: &OwnBox) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut todo: Vec<String> = own.uses().into_iter().collect();
    while let Some(id) = todo.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let uses = if id == own.id {
            own.uses()
        } else {
            scene
                .graphs()
                .own(&id)
                .map(|o| o.uses())
                .unwrap_or_default()
        };
        todo.extend(uses);
    }
    seen
}

impl Run<'_> {
    /// [`super::Command::SetOwnBox`].
    pub(super) fn set_own_box(&mut self, id: &str, own: Option<&OwnBox>) -> Result<(), SceneError> {
        let store = self.scene.graphs();
        let before = store.own(id).cloned();
        let refuse = |why: String| Err(SceneError::Graph(Refusal::new(why)));
        let Some(own) = own else {
            let Some(before) = before else {
                return Err(no_such_own_box());
            };
            let (graphs, owns) = store.users_of(id);
            if !graphs.is_empty() || !owns.is_empty() {
                let mut names: Vec<String> = graphs
                    .iter()
                    .filter_map(|g| store.get(*g).map(|g| g.name().to_string()))
                    .collect();
                names.extend(owns);
                return refuse(trf!(
                    "{} is still used in {}. Remove the boxes of it there first (or open them \
                     up with Shift + G), then delete it.",
                    before.name,
                    names.join(", ")
                ));
            }
            return self.edit(Edit::SetOwnBox {
                id: id.to_string(),
                before: Some(before),
                after: None,
            });
        };
        if own.id != id || !matgraph::own::is_own_id(id) {
            return refuse(tr("That box of your own has a damaged id.").to_string());
        }
        if before.is_none() && store.own_count() >= matgraph::own::MAX_OWN_BOXES {
            return refuse(trf!(
                "A project can hold {} boxes of your own; delete one first.",
                matgraph::own::MAX_OWN_BOXES
            ));
        }
        if inside_of(self.scene, own).contains(id) {
            return refuse(trf!(
                "{} can't hold itself, even through other boxes of yours.",
                own.name
            ));
        }
        let registry = store.registry().clone();
        let inner = sound(&own.inner, &registry)?;
        let mut after = own.clone();
        after.inner = Arc::new(inner);
        let after = after.normalised(&registry);
        after.check(&registry).map_err(|why| {
            SceneError::Graph(Refusal::new(trf!(
                "That box of your own can't be kept: {}.",
                why
            )))
        })?;
        // Placed boxes keep their wires: a box in use keeps its plugs.
        if let Some(before) = &before {
            let plugs = |o: &OwnBox| {
                let ins = o.inputs.iter().map(|p| (p.key.clone(), p.kind));
                let outs = o.outputs.iter().map(|p| (p.key.clone(), p.kind));
                (ins.collect::<Vec<_>>(), outs.collect::<Vec<_>>())
            };
            let (graphs, owns) = store.users_of(id);
            if plugs(before) != plugs(&after) && (!graphs.is_empty() || !owns.is_empty()) {
                return refuse(trf!(
                    "{} is placed in graphs, so its plugs stay as they are; their names can \
                     change.",
                    before.name
                ));
            }
            if **before == after {
                return Ok(());
            }
        }
        self.edit(Edit::SetOwnBox {
            id: id.to_string(),
            before,
            after: Some(Arc::new(after)),
        })
    }

    /// [`super::Command::EditOwnBox`].
    pub(super) fn edit_own_box(&mut self, id: &str, edit: &GraphEdit) -> Result<(), SceneError> {
        let store = self.scene.graphs();
        let before = store.own(id).cloned().ok_or_else(no_such_own_box)?;
        let registry = store.registry().clone();
        if let GraphEdit::AddBox { kind, .. } = edit {
            let refuse = |why: String| Err(SceneError::Graph(Refusal::new(why)));
            if let Some(info) = registry.get(kind).map(|k| k.info()) {
                if info.one_per_graph || info.outputs.is_empty() {
                    return refuse(trf!(
                        "{} can't go inside your own box: it is where a graph's material comes \
                         out.",
                        info.name.show()
                    ));
                }
                if matgraph::model::is_from_model(&registry, kind) {
                    return refuse(trf!(
                        "{} can't go inside your own box yet: it reads the model's own maps \
                         in the painter. Put it outside and plug it into your box.",
                        info.name.show()
                    ));
                }
            }
            let holds_this = kind == id
                || store
                    .own(kind)
                    .is_some_and(|o| inside_of(self.scene, o).contains(id));
            if holds_this {
                return refuse(trf!(
                    "{} can't go inside itself, even through other boxes of yours.",
                    before.name
                ));
            }
        }
        let mut inner = (*before.inner).clone();
        inner.apply(&registry, edit).map_err(SceneError::Graph)?;
        if inner == *before.inner {
            return Ok(());
        }
        let mut after = (*before).clone();
        after.inner = Arc::new(inner);
        let after = after.normalised(&registry);
        self.edit(Edit::SetOwnBox {
            id: id.to_string(),
            before: Some(before),
            after: Some(Arc::new(after)),
        })
    }
}
