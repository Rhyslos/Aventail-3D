//! Material graphs in the scene (decision 0257): added, edited, replaced
//! and removed as undo steps; refusals keep their reasons; ids are never
//! reused; slider drags merge into one step.

use crate::{Command, Document, SceneError};
use glam::Vec2;
use matgraph::{BoxId, Graph, GraphEdit, GraphId, PlugRef, SettingValue};

fn add_box(kind: &str) -> GraphEdit {
    GraphEdit::AddBox {
        kind: kind.into(),
        at: Vec2::ZERO,
    }
}

fn edit(doc: &mut Document, id: GraphId, edit: GraphEdit) -> Result<(), SceneError> {
    doc.execute(Command::EditGraph { id, edit })
}

#[test]
fn a_graph_is_added_edited_and_each_change_undone() {
    let mut doc = Document::new();
    let id = doc.scene().graphs().next_id();
    doc.execute(Command::AddGraph(Graph::new("Stone"))).unwrap();
    assert_eq!(doc.undo_label(), Some("Add a material graph"));
    edit(&mut doc, id, add_box("noise")).unwrap();
    edit(&mut doc, id, add_box("levels")).unwrap();
    edit(
        &mut doc,
        id,
        GraphEdit::Connect {
            from: PlugRef::new(BoxId(1), "height"),
            to: PlugRef::new(BoxId(2), "in"),
        },
    )
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Connect boxes"));
    let graph = doc.scene().graphs().get(id).unwrap().clone();
    assert_eq!((graph.box_count(), graph.wires().len()), (2, 1));
    doc.undo().unwrap();
    assert!(doc.scene().graphs().get(id).unwrap().wires().is_empty());
    doc.redo().unwrap();
    assert_eq!(doc.scene().graphs().get(id).unwrap(), &graph);
    for _ in 0..4 {
        doc.undo().unwrap();
    }
    assert!(doc.scene().graphs().is_empty());
    // Ids are never given out again, even after undo.
    doc.execute(Command::AddGraph(Graph::new("Wood"))).unwrap();
    assert!(doc.scene().graphs().get(id).is_none());
    assert_eq!(doc.scene().graphs().len(), 1);
}

#[test]
fn a_refused_edit_keeps_its_reason_and_changes_nothing() {
    let mut doc = Document::new();
    let id = doc.scene().graphs().next_id();
    doc.execute(Command::AddGraph(Graph::new("Stone"))).unwrap();
    edit(&mut doc, id, add_box("colour_fill")).unwrap();
    edit(&mut doc, id, add_box("levels")).unwrap();
    let steps = doc.undo_steps();
    let error = edit(
        &mut doc,
        id,
        GraphEdit::Connect {
            from: PlugRef::new(BoxId(1), "colour"),
            to: PlugRef::new(BoxId(2), "in"),
        },
    )
    .unwrap_err();
    let SceneError::Graph(refusal) = &error else {
        panic!("{error:?}")
    };
    assert!(error.to_string().contains("high and low"), "{error}");
    assert!(!refusal.fixes.is_empty() || refusal.reason.contains("Levels"));
    assert_eq!(doc.undo_steps(), steps);
    // A graph that isn't there.
    assert!(edit(&mut doc, GraphId(99), add_box("noise")).is_err());
    assert!(
        doc.execute(Command::SetGraph {
            id: GraphId(99),
            graph: None
        })
        .is_err()
    );
}

#[test]
fn a_graph_is_replaced_and_removed() {
    let mut doc = Document::new();
    let id = doc.scene().graphs().next_id();
    doc.execute(Command::AddGraph(Graph::new("Stone"))).unwrap();
    doc.execute(Command::SetGraph {
        id,
        graph: Some(Graph::new("Cobbles")),
    })
    .unwrap();
    assert_eq!(doc.scene().graphs().get(id).unwrap().name(), "Cobbles");
    // The same again records nothing.
    let steps = doc.undo_steps();
    doc.execute(Command::SetGraph {
        id,
        graph: Some(Graph::new("Cobbles")),
    })
    .unwrap();
    assert_eq!(doc.undo_steps(), steps);
    doc.execute(Command::SetGraph { id, graph: None }).unwrap();
    assert_eq!(doc.undo_label(), Some("Delete a material graph"));
    assert!(doc.scene().graphs().is_empty());
    doc.undo().unwrap();
    assert_eq!(doc.scene().graphs().get(id).unwrap().name(), "Cobbles");
    // An unsound graph (no name) is refused.
    assert!(doc.execute(Command::AddGraph(Graph::new(""))).is_err());
}

#[test]
fn a_slider_drag_is_one_undo_step() {
    let mut doc = Document::new();
    let id = doc.scene().graphs().next_id();
    doc.execute(Command::AddGraph(Graph::new("Stone"))).unwrap();
    edit(&mut doc, id, add_box("noise")).unwrap();
    let steps = doc.undo_steps();
    for amount in [0.1, 0.2, 0.3, 0.4] {
        doc.execute_merging(
            Command::EditGraph {
                id,
                edit: GraphEdit::SetSetting {
                    node: BoxId(1),
                    key: "roughness".into(),
                    value: SettingValue::Number(amount),
                },
            },
            7,
        )
        .unwrap();
    }
    assert_eq!(doc.undo_steps(), steps + 1);
    doc.undo().unwrap();
    let graph = doc.scene().graphs().get(id).unwrap();
    assert_eq!(graph.get(BoxId(1)).unwrap().settings.get("roughness"), None);
}

#[test]
fn undo_memory_counts_only_the_graph_that_changed() {
    let mut doc = Document::new();
    let id = doc.scene().graphs().next_id();
    doc.execute(Command::AddGraph(Graph::new("Stone"))).unwrap();
    let before = doc.history_memory();
    for _ in 0..50 {
        edit(&mut doc, id, add_box("noise")).unwrap();
    }
    let per_step = (doc.history_memory() - before) / 50;
    assert!(per_step > 0 && per_step < 64 * 1024, "{per_step}");
}
