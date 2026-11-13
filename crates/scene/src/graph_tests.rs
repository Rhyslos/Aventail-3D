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

/// A graph of Noise into Levels, Levels made into an own box ("Lumpy",
/// Levels' Darkest a slider) as one undo step: the graph's id, the own
/// box's id and the placed box.
fn with_own_box(doc: &mut Document) -> (GraphId, String, BoxId) {
    let id = doc.scene().graphs().next_id();
    doc.execute(Command::AddGraph(Graph::new("Stone"))).unwrap();
    edit(doc, id, add_box("noise")).unwrap();
    edit(doc, id, add_box("levels")).unwrap();
    edit(doc, id, add_box("material_from_height")).unwrap();
    for (from, to) in [((1, "height"), (2, "in")), ((2, "out"), (3, "height"))] {
        edit(
            doc,
            id,
            GraphEdit::Connect {
                from: PlugRef::new(BoxId(from.0), from.1),
                to: PlugRef::new(BoxId(to.0), to.1),
            },
        )
        .unwrap();
    }
    let graph = doc.scene().graphs().get(id).unwrap().clone();
    let registry = doc.scene().graphs().registry().clone();
    let mut proposal = matgraph::own::Proposal::of(&graph, &registry, &[BoxId(2)].into()).unwrap();
    proposal.name = "Lumpy".into();
    proposal.sliders[0].chosen = true;
    let own_id = matgraph::own::fresh_id("Lumpy", |_| false);
    let (own, edits) = proposal
        .make(&graph, &registry, own_id.clone(), None)
        .unwrap();
    let placed = BoxId(graph.next_id());
    let commands = std::iter::once(Command::SetOwnBox {
        id: own_id.clone(),
        own: Some(own),
    })
    .chain(
        edits
            .into_iter()
            .map(|edit| Command::EditGraph { id, edit }),
    );
    doc.execute_all_labelled(commands, "Make your own box", 77)
        .unwrap();
    (id, own_id, placed)
}

#[test]
fn an_own_box_is_made_in_one_step_and_undone() {
    let mut doc = Document::new();
    let (id, own_id, placed) = with_own_box(&mut doc);
    assert_eq!(doc.undo_label(), Some("Make your own box"));
    let graph = doc.scene().graphs().get(id).unwrap().clone();
    assert_eq!(graph.get(placed).unwrap().kind, own_id);
    assert_eq!(graph.box_count(), 3);
    assert!(doc.scene().graphs().registry().get(&own_id).is_some());
    doc.undo().unwrap();
    assert!(doc.scene().graphs().own(&own_id).is_none());
    assert!(doc.scene().graphs().registry().get(&own_id).is_none());
    assert_eq!(
        doc.scene().graphs().get(id).unwrap().first_of("levels"),
        Some(BoxId(2))
    );
    doc.redo().unwrap();
    assert_eq!(doc.scene().graphs().get(id).unwrap(), &graph);
}

#[test]
fn the_inside_of_an_own_box_changes_for_every_placed_box() {
    let mut doc = Document::new();
    let (_, own_id, _) = with_own_box(&mut doc);
    doc.execute(Command::EditOwnBox {
        id: own_id.clone(),
        edit: GraphEdit::SetSetting {
            node: BoxId(2),
            key: "invert".into(),
            value: SettingValue::Switch(true),
        },
    })
    .unwrap();
    let own = doc.scene().graphs().own(&own_id).unwrap().clone();
    assert_eq!(
        own.inner.get(BoxId(2)).unwrap().settings.get("invert"),
        Some(SettingValue::Switch(true))
    );
    // Removing the box behind the slider takes the slider away.
    doc.execute(Command::EditOwnBox {
        id: own_id.clone(),
        edit: GraphEdit::RemoveBoxes(vec![BoxId(2)]),
    })
    .unwrap();
    let own = doc.scene().graphs().own(&own_id).unwrap().clone();
    assert!(own.sliders.is_empty());
    assert!(own.outputs[0].from.is_none());
    doc.undo().unwrap();
    doc.undo().unwrap();
    let own = doc.scene().graphs().own(&own_id).unwrap().clone();
    assert_eq!(own.sliders.len(), 1);
}

#[test]
fn own_boxes_refuse_what_would_break_them() {
    let mut doc = Document::new();
    let (id, own_id, placed) = with_own_box(&mut doc);
    let refused = |doc: &mut Document, command: Command, words: &str| {
        let why = doc.execute(command).unwrap_err();
        assert!(why.to_string().contains(words), "{why}");
    };
    // Itself inside itself, Material out and From the model inside.
    for (kind, words) in [
        (own_id.as_str(), "inside itself"),
        ("material_out", "material comes"),
        ("model_curvature", "model's own maps"),
    ] {
        refused(
            &mut doc,
            Command::EditOwnBox {
                id: own_id.clone(),
                edit: add_box(kind),
            },
            words,
        );
    }
    // Deleting it while a graph uses it.
    refused(
        &mut doc,
        Command::SetOwnBox {
            id: own_id.clone(),
            own: None,
        },
        "still used in Stone",
    );
    // Changing its plugs while placed.
    let mut changed = (**doc.scene().graphs().own(&own_id).unwrap()).clone();
    changed.outputs[0].kind = matgraph::ValueKind::Shape;
    changed.outputs[0].from = None;
    refused(
        &mut doc,
        Command::SetOwnBox {
            id: own_id.clone(),
            own: Some(changed),
        },
        "plugs stay",
    );
    // Its name can change.
    let mut renamed = (**doc.scene().graphs().own(&own_id).unwrap()).clone();
    renamed.name = "Bumpy".into();
    doc.execute(Command::SetOwnBox {
        id: own_id.clone(),
        own: Some(renamed),
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Change your own box"));
    // Once no graph uses it, it can be deleted.
    edit(&mut doc, id, GraphEdit::RemoveBoxes(vec![placed])).unwrap();
    doc.execute(Command::SetOwnBox {
        id: own_id.clone(),
        own: None,
    })
    .unwrap();
    assert!(doc.scene().graphs().own(&own_id).is_none());
    doc.undo().unwrap();
    assert!(doc.scene().graphs().own(&own_id).is_some());
}

#[test]
fn a_stroke_on_a_big_painting_holds_only_the_tiles_it_changed() {
    use matgraph::Painting;
    use std::sync::Arc;
    let mut doc = Document::new();
    let id = doc.scene().graphs().next_id();
    doc.execute(Command::AddGraph(Graph::new("Moss"))).unwrap();
    edit(&mut doc, id, add_box("painted")).unwrap();
    // A whole 2048 painting painted: 16 MB.
    let full = Painting::from_rgba(2048, &vec![200u8; 2048 * 2048 * 4]).unwrap();
    edit(
        &mut doc,
        id,
        GraphEdit::SetPainting {
            node: BoxId(1),
            painting: Some(Arc::new(full.clone())),
        },
    )
    .unwrap();
    let before = doc.history_memory();
    // One stroke changes one tile.
    let mut tiles = full.tiles().to_vec();
    tiles[0] = Some(Arc::new(vec![9u8; matgraph::painting::PAINT_TILE_BYTES]));
    let stroke = Painting::from_tiles(2048, tiles).unwrap();
    edit(
        &mut doc,
        id,
        GraphEdit::SetPainting {
            node: BoxId(1),
            painting: Some(Arc::new(stroke)),
        },
    )
    .unwrap();
    let step = doc.history_memory() - before;
    assert!(step < 100_000, "a one-tile stroke holds {step} bytes");
    assert_eq!(doc.undo_label(), Some("Paint on a Painted box"));
    doc.undo().unwrap();
    let painted = doc
        .scene()
        .graphs()
        .get(id)
        .unwrap()
        .get(BoxId(1))
        .unwrap()
        .painting
        .clone();
    assert_eq!(painted.as_deref(), Some(&full));
}
