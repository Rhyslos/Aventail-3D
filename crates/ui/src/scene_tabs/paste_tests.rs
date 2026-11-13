//! Copy and paste between tabs (decision 0268), through the real
//! interface without a window: objects copied in one tab paste in another
//! with their materials, paint, graphs and boxes of your own, without
//! clashing with what that project has, undo there in one step, leave the
//! tab they came from untouched, still paste once that tab sleeps or
//! closes, explain Alt + V, and save and open again.

use super::tests::{H, harness, new_tab, objects, tab};
use super::*;
use egui::{Key, Modifiers};
use matgraph::{BoxId, Graph, GraphEdit, GraphId, OwnBox};
use paint::{GraphFill, Image, Layer, TextureSet};
use scene::{Command, Material, ObjectKind, PrimitiveKind};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// An own box called `name` holding a Levels box set to `low`.
fn own_box(name: &str, low: f32) -> OwnBox {
    let registry = matgraph::Registry::standard();
    let mut g = Graph::new("inside");
    g.apply(
        registry,
        &GraphEdit::AddBox {
            kind: "levels".into(),
            at: glam::Vec2::ZERO,
        },
    )
    .unwrap();
    g.apply(
        registry,
        &GraphEdit::SetSetting {
            node: BoxId(1),
            key: "low".into(),
            value: matgraph::SettingValue::Number(low),
        },
    )
    .unwrap();
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

/// Puts `own` in the tab in use's project with a graph `graph` using
/// it, and a cube with material `material` painted `rgba`, the graph's
/// fill on top. The cube is selected. Returns the graph's id.
fn painted_cube(harness: &mut H, own: &OwnBox, graph: &str, material: &str, rgba: [u8; 4]) {
    let doc = harness.state_mut().tabs.active_mut().document_mut();
    doc.execute(Command::SetOwnBox {
        id: own.id.clone(),
        own: Some(own.clone()),
    })
    .unwrap();
    let registry = doc.scene().graphs().registry().clone();
    let mut g = Graph::new(graph);
    for (kind, x) in [("material_out", 0.0), (own.id.as_str(), -300.0)] {
        g.apply(
            &registry,
            &GraphEdit::AddBox {
                kind: kind.into(),
                at: glam::Vec2::new(x, 0.0),
            },
        )
        .unwrap();
    }
    let id = doc.scene().graphs().next_id();
    doc.execute(Command::AddGraph(g)).unwrap();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let cube = doc.selection()[0];
    doc.execute(Command::SetMaterials {
        id: cube,
        materials: vec![Material {
            name: material.into(),
            color: [0.5, 0.5, 0.5],
        }],
    })
    .unwrap();
    let mut image = Image::new(64).unwrap();
    image.set_pixel(0, 0, rgba);
    let mut set = TextureSet::new(64).unwrap();
    set.layers.push(Layer::paint("Paint", image));
    let mut fill = Layer::fill("Stone", [0, 0, 0, 255]);
    fill.graph = Some(GraphFill::of(id));
    set.layers.push(fill);
    doc.execute(Command::SetTextureSet {
        material: material.into(),
        set: Some(set),
    })
    .unwrap();
    doc.set_selection([cube]);
    settle(harness);
}

/// Runs frames until no graph fill is being made in the tab in use (or
/// half a minute passes): making one joins the step that needed it.
fn settle(harness: &mut H) {
    let start = Instant::now();
    loop {
        harness.step();
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
        if !harness.state().tabs.active().graph_fills_busy()
            || start.elapsed() > Duration::from_secs(30)
        {
            break;
        }
    }
    harness.step();
}

fn press(harness: &mut H, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    settle(harness);
}

fn note(harness: &H) -> String {
    harness
        .state()
        .tabs
        .active()
        .note()
        .unwrap_or_default()
        .to_string()
}

fn scene_of(harness: &H) -> scene::Scene {
    harness.state().tabs.active().document().scene().clone()
}

/// The graph material `material`'s graph fill runs, in `scene`.
fn fill_graph(scene: &scene::Scene, material: &str) -> (GraphId, Arc<Graph>) {
    let id = scene
        .paint()
        .set(material)
        .unwrap()
        .layers
        .iter()
        .find_map(|l| l.graph.as_ref().map(|g| g.graph))
        .unwrap();
    (id, scene.graphs().get(id).unwrap().clone())
}

/// A first tab with a painted cube copied (Ctrl + C), then a second tab
/// in use. Returns the first tab, its scene and the own box.
fn copied_in_first_tab() -> (H, TabId, scene::Scene, OwnBox) {
    let mut harness = harness();
    let rocky = own_box("Rocky", 0.2);
    painted_cube(&mut harness, &rocky, "Cobbles", "Stone", [9, 8, 7, 255]);
    press(&mut harness, Modifiers::COMMAND, Key::C);
    assert!(
        note(&harness).starts_with("Copied 1 object"),
        "{}",
        note(&harness)
    );
    let first = harness.state().tabs.active_id();
    let scene = scene_of(&harness);
    new_tab(&mut harness);
    (harness, first, scene, rocky)
}

#[test]
fn objects_pasted_in_another_tab_bring_their_paint_graphs_and_own_boxes() {
    let (mut harness, first, first_scene, rocky) = copied_in_first_tab();
    press(&mut harness, Modifiers::COMMAND, Key::V);
    let scene = scene_of(&harness);
    assert_eq!(scene.children(None).len(), 1);
    let pasted = harness.state().tabs.active().document().selection();
    let object = scene.object(pasted[0]).unwrap();
    assert_eq!(object.materials[0].name, "Stone");
    let set = scene.paint().set("Stone").unwrap();
    assert_eq!(
        set.layers[0],
        first_scene.paint().set("Stone").unwrap().layers[0]
    );
    let (_, graph) = fill_graph(&scene, "Stone");
    assert_eq!(graph.name(), "Cobbles");
    assert!(scene.graphs().own(&rocky.id).is_some());
    let words = note(&harness);
    assert!(words.contains("Pasted 1 object from Untitled."), "{words}");
    assert!(words.contains("paint, material graphs"), "{words}");
    // The tab copied from is untouched.
    assert_eq!(
        harness.state().tabs.get(first).unwrap().document().scene(),
        &first_scene
    );
}

#[test]
fn a_paste_fits_in_without_clashing_and_undoes_in_one_step() {
    let (mut harness, first, first_scene, rocky) = copied_in_first_tab();
    // The second project has its own Cobbles graph, its own Rocky with
    // the same id but another inside, and its own Stone painted otherwise.
    let mut other = own_box("Rocky", 0.9);
    other.id.clone_from(&rocky.id);
    painted_cube(&mut harness, &other, "Cobbles", "Stone", [1, 1, 1, 255]);
    let before = scene_of(&harness);
    let steps = harness.state().tabs.active().document().undo_steps();
    press(&mut harness, Modifiers::COMMAND, Key::V);
    let scene = scene_of(&harness);
    // What the project had is as it was.
    assert_eq!(
        scene.graphs().own(&rocky.id),
        before.graphs().own(&rocky.id)
    );
    assert_eq!(scene.paint().set("Stone"), before.paint().set("Stone"));
    // What came in has its own ids and names.
    let pasted = harness.state().tabs.active().document().selection();
    let object = scene.object(pasted[0]).unwrap();
    assert_eq!(object.materials[0].name, "Stone 2");
    let (id, graph) = fill_graph(&scene, "Stone 2");
    assert!(before.graphs().get(id).is_none(), "a new graph id");
    assert_eq!(graph.name(), "Cobbles 2");
    let kind = graph
        .boxes()
        .map(|(_, b)| b.kind.clone())
        .find(|k| matgraph::own::is_own(k))
        .unwrap();
    assert_ne!(kind, rocky.id);
    assert_eq!(scene.graphs().own(&kind).unwrap().inner, rocky.inner);
    assert!(
        note(&harness).contains("Stone is Stone 2 here"),
        "{}",
        note(&harness)
    );
    // One undo step, in this tab only.
    assert_eq!(
        harness.state().tabs.active().document().undo_steps(),
        steps + 1
    );
    press(&mut harness, Modifiers::COMMAND, Key::Z);
    let undone = scene_of(&harness);
    assert_eq!(undone.walk(), before.walk());
    assert_eq!(undone.paint(), before.paint());
    assert_eq!(undone.graphs().len(), before.graphs().len());
    assert_eq!(undone.graphs().own_count(), before.graphs().own_count());
    assert_eq!(
        harness.state().tabs.get(first).unwrap().document().scene(),
        &first_scene
    );
}

#[test]
fn alt_v_in_another_tab_pastes_plain_copies_and_says_why() {
    let (mut harness, _, _, _) = copied_in_first_tab();
    press(&mut harness, Modifiers::ALT, Key::V);
    assert_eq!(objects(&harness), 1);
    let scene = scene_of(&harness);
    let id = scene.children(None)[0];
    assert!(!matches!(
        scene.object(id).unwrap().kind,
        ObjectKind::Instance(_)
    ));
    let words = note(&harness);
    assert!(words.contains("can't stay linked"), "{words}");
}

#[test]
fn pasting_works_after_the_tab_copied_from_sleeps_or_closes() {
    let (mut harness, first, _, _) = copied_in_first_tab();
    harness
        .state_mut()
        .tabs
        .sleep_tabs(Instant::now() + Duration::from_secs(600), false, |_| {
            crate::viewport::MemoryFreed::default()
        });
    settle(&mut harness);
    assert!(harness.state().tabs.is_asleep(first));
    press(&mut harness, Modifiers::COMMAND, Key::V);
    assert_eq!(objects(&harness), 1);
    // The first tab closed (its changes thrown away): still pastes.
    harness.state_mut().tabs.remove(first);
    settle(&mut harness);
    assert_eq!(harness.state().tabs.len(), 1);
    press(&mut harness, Modifiers::COMMAND, Key::V);
    assert_eq!(objects(&harness), 2);
    let scene = scene_of(&harness);
    assert_eq!(scene.graphs().len(), 1, "the same graph is reused");
    assert_eq!(scene.paint().sets().count(), 1);
}

#[test]
fn pasting_in_the_same_tab_still_places_copies_after_their_originals() {
    let mut harness = harness();
    let rocky = own_box("Rocky", 0.2);
    painted_cube(&mut harness, &rocky, "Cobbles", "Stone", [9, 8, 7, 255]);
    let graphs = scene_of(&harness).graphs().len();
    press(&mut harness, Modifiers::COMMAND, Key::C);
    press(&mut harness, Modifiers::COMMAND, Key::V);
    assert_eq!(objects(&harness), 2);
    let scene = scene_of(&harness);
    assert_eq!(scene.graphs().len(), graphs, "nothing brought in");
    assert!(note(&harness).starts_with("Pasted copies"));
    // Alt + V in the same tab: instances, as before.
    press(&mut harness, Modifiers::ALT, Key::V);
    let scene = scene_of(&harness);
    assert!(
        scene
            .children(None)
            .iter()
            .any(|id| matches!(scene.object(*id).unwrap().kind, ObjectKind::Instance(_)))
    );
}

#[test]
fn copied_objects_paste_into_the_next_project_in_the_same_tab() {
    let mut harness = harness();
    let rocky = own_box("Rocky", 0.2);
    painted_cube(&mut harness, &rocky, "Cobbles", "Stone", [9, 8, 7, 255]);
    press(&mut harness, Modifiers::COMMAND, Key::C);
    // Another project opened in this tab, in its place.
    harness.state_mut().tabs.active_mut().opened_noting(
        scene::Document::new(),
        std::path::PathBuf::from("other.project"),
        &[],
    );
    settle(&mut harness);
    assert_eq!(objects(&harness), 0);
    press(&mut harness, Modifiers::COMMAND, Key::V);
    assert_eq!(objects(&harness), 1);
    assert_eq!(scene_of(&harness).graphs().len(), 1);
}

#[test]
fn a_project_with_pasted_objects_saves_and_opens_again() {
    let (mut harness, _, _, _) = copied_in_first_tab();
    press(&mut harness, Modifiers::COMMAND, Key::V);
    let document = harness.state().tabs.active().document();
    let bytes = fileio::project::to_bytes(document).unwrap();
    let opened = fileio::project::from_bytes(&bytes).unwrap();
    let (a, b) = (document.scene(), opened.scene());
    assert_eq!(a.walk(), b.walk());
    for (id, _) in a.walk() {
        assert_eq!(a.object(id), b.object(id));
    }
    assert_eq!(a.paint(), b.paint());
    assert_eq!(
        a.graphs().iter().collect::<Vec<_>>(),
        b.graphs().iter().collect::<Vec<_>>()
    );
    assert_eq!(a.graphs().own_count(), b.graphs().own_count());
}

#[test]
fn the_copy_is_handed_to_every_tab() {
    let (harness, first, _, _) = copied_in_first_tab();
    // The second tab has what was copied; the first knows it came from
    // itself (a paste there is a plain Duplicate).
    let tabs = &harness.state().tabs;
    assert!(tabs.active().has_copied_objects());
    assert!(tabs.get(first).unwrap().has_copied_objects());
    let _ = tab(&harness, "Untitled");
}
