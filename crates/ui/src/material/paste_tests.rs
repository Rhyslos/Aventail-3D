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
        tags: Vec::new(),
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

/// A palette called `name` of `colours`.
fn palette(name: &str, colours: &[[u8; 3]]) -> paint::palette::Palette {
    paint::palette::Palette {
        name: name.into(),
        colours: colours.to_vec(),
    }
}

/// A project with palettes Moss and Unused and a graph holding a Palette
/// map set to Moss; returns it and the graph's id.
fn palette_project() -> (Document, GraphId) {
    let mut doc = Document::new();
    doc.execute(Command::SetPalettes(vec![
        palette("Moss", &[[40, 90, 30], [120, 170, 60]]),
        palette("Unused", &[[1, 1, 1]]),
    ]))
    .unwrap();
    let registry = doc.scene().graphs().registry().clone();
    let mut g = Graph::new("Mossy");
    for edit in [
        GraphEdit::AddBox {
            kind: "palette_map".into(),
            at: Vec2::ZERO,
        },
        GraphEdit::SetSetting {
            node: BoxId(1),
            key: "palette".into(),
            value: matgraph::SettingValue::Palette("Moss".into()),
        },
    ] {
        g.apply(&registry, &edit).unwrap();
    }
    let id = doc.scene().graphs().next_id();
    doc.execute(Command::AddGraph(g)).unwrap();
    (doc, id)
}

#[test]
fn boxes_pasted_in_another_project_bring_the_palettes_they_name() {
    let (doc, graph) = palette_project();
    let mut first = editor(1, graph);
    first.canvas.selected = [BoxId(1)].into();
    let mut note = None;
    first.copy(&doc, false, &mut note);
    // Another project with a palette of its own.
    let (mut other, theirs) = project(&own_box("Rocky", 0.2));
    other
        .execute(Command::SetPalettes(vec![palette("Mine", &[[5, 5, 5]])]))
        .unwrap();
    let mut second = editor(2, theirs);
    second.clipboard = first.clipboard.clone();
    let steps = other.undo_steps();
    let mut note = None;
    second.paste(&mut other, Vec2::new(0.0, 400.0), false, &mut note);
    let words = note.unwrap();
    assert!(words.contains("1 colour palettes they use came"), "{words}");
    let names: Vec<&str> = other
        .scene()
        .paint()
        .palettes()
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(names, ["Mine", "Moss"], "only the palette named came");
    assert_eq!(other.undo_steps(), steps + 1, "one undo step");
    // The pasted box runs with it.
    let registry = other.scene().graphs().registry().clone();
    let g = other.scene().graphs().get(theirs).unwrap().clone();
    let pasted = g
        .boxes()
        .find(|(_, b)| b.kind == "palette_map")
        .map(|(id, _)| id)
        .unwrap();
    let out = matgraph::Evaluator::with_threads(1).run(
        &registry,
        &g,
        matgraph::Size::new(16).unwrap(),
        &[pasted],
    );
    // Nothing is plugged in, so it asks for an input, not a palette.
    let why = out.outcome(pasted).unwrap().as_ref().unwrap_err();
    assert!(!why.reason.contains("Moss"), "{}", why.reason);
    other.undo();
    assert_eq!(other.scene().paint().palettes().len(), 1);
    // A project with its own Moss keeps it, and says so.
    let (mut third, graph3) = project(&own_box("Rocky", 0.2));
    let theirs_moss = palette("Moss", &[[0, 0, 255]]);
    third
        .execute(Command::SetPalettes(vec![theirs_moss.clone()]))
        .unwrap();
    let mut third_editor = editor(3, graph3);
    third_editor.clipboard = first.clipboard.clone();
    let mut note = None;
    third_editor.paste(&mut third, Vec2::new(0.0, 400.0), false, &mut note);
    assert!(note.unwrap().contains("are its own"));
    assert_eq!(third.scene().paint().palettes(), [theirs_moss]);
}

#[test]
fn the_palette_fixes_choose_one_or_add_the_starters_in_one_step() {
    let (mut doc, graph) = palette_project();
    let mut editor = editor(1, graph);
    let context = super::note::FixContext {
        node: Some(BoxId(1)),
        ..Default::default()
    };
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
    let mut note = None;
    editor.carry_out_fix(
        &mut doc,
        &matgraph::Fix::UsePalette {
            key: "palette".into(),
            name: "Unused".into(),
        },
        &context,
        rect,
        &mut note,
    );
    assert_eq!(
        doc.scene()
            .graphs()
            .get(graph)
            .unwrap()
            .get(BoxId(1))
            .unwrap()
            .settings
            .get("palette"),
        Some(matgraph::SettingValue::Palette("Unused".into()))
    );
    assert!(note.unwrap().starts_with("Done: Use the palette Unused"));
    // A project without palettes gets the painter's starter palettes.
    doc.execute(Command::SetPalettes(Vec::new())).unwrap();
    let steps = doc.undo_steps();
    let mut note = None;
    editor.carry_out_fix(
        &mut doc,
        &matgraph::Fix::AddPalettes,
        &context,
        rect,
        &mut note,
    );
    assert_eq!(
        doc.scene().paint().palettes(),
        paint::palette::starters().as_slice()
    );
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(
        doc.scene().graphs().registry().palettes().len(),
        paint::palette::starters().len()
    );
    assert!(note.unwrap().starts_with("Done:"));
}
