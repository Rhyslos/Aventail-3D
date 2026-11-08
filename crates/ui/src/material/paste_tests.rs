//! Boxes copied on the Material canvas pasted in another project
//! (decision 0268): the boxes of your own among them come along, the
//! project's own box with the same id keeps it, Alt + V explains that
//! links can't reach another project, and the same project pastes as
//! before.

use super::MaterialEditor;
use glam::Vec2;
use matgraph::{BoxId, Graph, GraphEdit, GraphId, OwnBox};
use scene::{Command, Document};
use std::sync::Arc;

/// An own box called `name` holding a Levels box set to `low`.
fn own_box(name: &str, low: f32) -> OwnBox {
    let registry = matgraph::Registry::standard();
    let mut g = Graph::new("inside");
    for edit in [
        GraphEdit::AddBox {
            kind: "levels".into(),
            at: Vec2::ZERO,
        },
        GraphEdit::SetSetting {
            node: BoxId(1),
            key: "low".into(),
            value: matgraph::SettingValue::Number(low),
        },
    ] {
        g.apply(registry, &edit).unwrap();
    }
    OwnBox {
        id: matgraph::own::fresh_id(name, |_| false),
        name: name.into(),
        about: String::new(),
        picture: None,
        inner: Arc::new(g),
        inputs: Vec::new(),
        outputs: Vec::new(),
        sliders: Vec::new(),
    }
}

/// A project holding `own` and a graph with a Noise box and a box of
/// `own`; returns it and the graph's id.
fn project(own: &OwnBox) -> (Document, GraphId) {
    let mut doc = Document::new();
    doc.execute(Command::SetOwnBox {
        id: own.id.clone(),
        own: Some(own.clone()),
    })
    .unwrap();
    let registry = doc.scene().graphs().registry().clone();
    let mut g = Graph::new("Cobbles");
    for (kind, x) in [("noise", 0.0), (own.id.as_str(), 300.0)] {
        g.apply(
            &registry,
            &GraphEdit::AddBox {
                kind: kind.into(),
                at: Vec2::new(x, 0.0),
            },
        )
        .unwrap();
    }
    let id = doc.scene().graphs().next_id();
    doc.execute(Command::AddGraph(g)).unwrap();
    (doc, id)
}

fn editor(project: u64, graph: GraphId) -> MaterialEditor {
    MaterialEditor {
        project,
        graph: Some(graph),
        ..MaterialEditor::default()
    }
}

/// Copies both boxes of the first project's graph.
fn copied() -> (MaterialEditor, OwnBox) {
    let rocky = own_box("Rocky", 0.2);
    let (doc, graph) = project(&rocky);
    let mut first = editor(1, graph);
    first.canvas.selected = [BoxId(1), BoxId(2)].into();
    let mut note = None;
    first.copy(&doc, false, &mut note);
    assert_eq!(note.as_deref(), Some("Copied 2 boxes."));
    (first, rocky)
}

#[test]
fn boxes_pasted_in_another_project_bring_their_own_boxes() {
    let (first, rocky) = copied();
    // The second project has another Rocky with the same id.
    let mut theirs = own_box("Rocky", 0.9);
    theirs.id.clone_from(&rocky.id);
    let (mut doc, graph) = project(&theirs);
    let mut second = editor(2, graph);
    second.clipboard = first.clipboard.clone();
    let steps = doc.undo_steps();
    let mut note = None;
    second.paste(&mut doc, Vec2::new(0.0, 400.0), false, &mut note);
    let words = note.unwrap();
    assert!(
        words.starts_with("Pasted 2 boxes from another project."),
        "{words}"
    );
    assert!(
        words.contains("1 of your own boxes came with them"),
        "{words}"
    );
    let store = doc.scene().graphs();
    assert_eq!(**store.own(&rocky.id).unwrap(), theirs, "theirs kept");
    let g = store.get(graph).unwrap();
    assert_eq!(g.box_count(), 4);
    let kind = g
        .boxes()
        .map(|(_, b)| b.kind.clone())
        .find(|k| matgraph::own::is_own(k) && *k != rocky.id)
        .expect("a box of the one brought in");
    assert_eq!(store.own(&kind).unwrap().inner, rocky.inner);
    assert_eq!(doc.undo_steps(), steps + 1);
    doc.undo();
    assert_eq!(doc.scene().graphs().own_count(), 1);
    assert_eq!(doc.scene().graphs().get(graph).unwrap().box_count(), 2);
}

#[test]
fn the_same_own_box_is_reused_and_alt_v_explains_links() {
    let (first, rocky) = copied();
    let (mut doc, graph) = project(&rocky);
    let mut second = editor(2, graph);
    second.clipboard = first.clipboard.clone();
    let mut note = None;
    second.paste(&mut doc, Vec2::new(0.0, 400.0), true, &mut note);
    let words = note.unwrap();
    assert!(
        words.contains("Links can't reach into another project"),
        "{words}"
    );
    let store = doc.scene().graphs();
    assert_eq!(store.own_count(), 1, "the same box, reused");
    let placed = store
        .get(graph)
        .unwrap()
        .boxes()
        .filter(|(_, b)| b.kind == rocky.id)
        .count();
    assert_eq!(placed, 2);
}

#[test]
fn in_the_same_project_ctrl_v_still_makes_a_copy_of_its_own() {
    let (mut first, rocky) = copied();
    let (mut doc, graph) = project(&rocky);
    // As if the same project: its number matches.
    first.graph = Some(graph);
    let mut note = None;
    first.paste(&mut doc, Vec2::new(0.0, 400.0), false, &mut note);
    let store = doc.scene().graphs();
    assert_eq!(store.own_count(), 2);
    assert!(store.own_boxes().any(|o| o.name == "Rocky copy"));
}
