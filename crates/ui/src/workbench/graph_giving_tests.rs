//! Graph materials on shapes and as texture sets (Phase 12, milestone 6,
//! decision 0262), used through the real interface: Give to the selected
//! shapes makes the cube's material from the graph (made on the model,
//! made again when the graph changes, one undo step, saved), the
//! Attributes' Graph… menu gives and takes back, Export textures… asks
//! for a folder and writes the set, and the export checks say what to
//! know in plain words.

use super::tests::{TestRenderer, harness_built_sized, info, with_a_cube};
use super::*;
use crate::material::export_set::{EXPORT_TEXTURES, GIVE_TO_SELECTED};
use crate::material::giving::given_graph;
use egui_kittest::kittest::Queryable;
use glam::Vec2;
use matgraph::{BoxId, Graph, GraphEdit, GraphId, PlugRef, Registry, SettingValue};
use std::time::{Duration, Instant};

type H = egui_kittest::Harness<'static, super::tests::State>;

/// Colour fill and Noise → Material from height → Material out.
fn stone() -> Graph {
    let registry = Registry::standard();
    let mut graph = Graph::new("Stone");
    for (kind, x) in [
        ("noise", 0.0),
        ("colour_fill", 0.0),
        ("material_from_height", 300.0),
        ("material_out", 600.0),
    ] {
        let edit = GraphEdit::AddBox {
            kind: kind.into(),
            at: Vec2::new(x, 0.0),
        };
        graph.apply(registry, &edit).unwrap();
    }
    for (from, out, to, input) in [
        (1, "height", 3, "height"),
        (2, "colour", 3, "colour"),
        (3, "material", 4, "material"),
    ] {
        let edit = GraphEdit::Connect {
            from: PlugRef::new(BoxId(from), out),
            to: PlugRef::new(BoxId(to), input),
        };
        graph.apply(registry, &edit).unwrap();
    }
    graph
}

/// The Material mode showing `graph`, a cube in the scene, selected, and
/// 64-pixel textures: the graph's id.
fn page(graph: Graph) -> (H, GraphId) {
    let mut workbench = with_a_cube(Workbench::new(info(), None));
    let doc = workbench.document_mut();
    let mut texture = doc.scene().texture();
    texture.size = 64;
    doc.execute(scene::Command::SetTexture(texture)).unwrap();
    let id = doc.scene().graphs().next_id();
    doc.execute(scene::Command::AddGraph(graph)).unwrap();
    let cube = doc.scene().walk()[0].0;
    doc.set_selection([cube]);
    workbench.mode = Mode::Material;
    let mut harness = harness_built_sized(
        egui_kittest::Harness::builder(),
        workbench,
        TestRenderer::default(),
        [1440.0, 2000.0],
    );
    harness.run();
    harness.run();
    (harness, id)
}

fn cube(harness: &H) -> scene::ObjectId {
    harness.state().workbench.document().scene().walk()[0].0
}

/// The cube's first material's texture set, if any.
fn set(harness: &H) -> Option<paint::TextureSet> {
    let scene = harness.state().workbench.document().scene();
    let material = scene.material_list(cube(harness))[0].name.clone();
    scene.paint().set(&material).map(|s| (**s).clone())
}

/// Runs frames until no graph fill is being made (or half a minute
/// passes).
fn wait(harness: &mut H) {
    let start = Instant::now();
    loop {
        harness.step();
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
        if !harness.state().workbench.graph_fills.busy()
            || start.elapsed() > Duration::from_secs(30)
        {
            break;
        }
    }
    harness.run();
}

fn note(harness: &H) -> String {
    harness.state().workbench.note.clone().unwrap_or_default()
}

#[test]
fn giving_the_material_to_the_selected_cube_makes_it_from_the_graph() {
    let (mut harness, id) = page(stone());
    assert!(set(&harness).is_none(), "nothing painted yet");
    harness.get_by_label(GIVE_TO_SELECTED).click();
    harness.step();
    let given = set(&harness).expect("the cube's material has textures now");
    assert_eq!(given_graph(&given), Some(id));
    assert!(given.realism);
    assert!(
        note(&harness).starts_with("Gave Stone to"),
        "{}",
        note(&harness)
    );
    assert_eq!(
        harness.state().workbench.viewport.options().surface,
        crate::viewport::Surface::Painted,
        "the viewport shows it"
    );
    wait(&mut harness);
    let made = set(&harness).unwrap();
    let fill = made.layers[0].graph.clone().unwrap();
    assert_ne!(fill.made, 0, "made on the cube");
    let paint::Content::Paint(image) = &made.layers[0].content else {
        panic!("a graph fill is paint");
    };
    assert!(
        (0..64).any(|x| image.pixel(x, 32)[3] > 0),
        "the cube's texels are painted"
    );
    assert!(made.layers[0].channels.height.is_some(), "with its height");
    // One undo step takes it all back (the making joined it).
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert!(set(&harness).is_none());
}

#[test]
fn the_material_follows_the_graph_as_it_changes() {
    let (mut harness, id) = page(stone());
    harness.get_by_label(GIVE_TO_SELECTED).click();
    wait(&mut harness);
    let before = set(&harness).unwrap().layers[0].graph.clone().unwrap().made;
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(scene::Command::EditGraph {
        id,
        edit: GraphEdit::SetSetting {
            node: BoxId(2),
            key: "colour".into(),
            value: SettingValue::Colour([0.9, 0.1, 0.1]),
        },
    })
    .unwrap();
    harness.run();
    wait(&mut harness);
    let after = set(&harness).unwrap();
    assert_ne!(after.layers[0].graph.as_ref().unwrap().made, before);
    let paint::Content::Paint(image) = &after.layers[0].content else {
        panic!("paint");
    };
    let reddest = (0..64)
        .flat_map(|y| (0..64).map(move |x| (x, y)))
        .map(|(x, y)| image.pixel(x, y))
        .filter(|p| p[3] > 0)
        .any(|p| p[0] > p[1] + 60);
    assert!(reddest, "the new colour reached the cube");
}

#[test]
fn a_given_material_is_saved_with_the_project() {
    let (mut harness, id) = page(stone());
    harness.get_by_label(GIVE_TO_SELECTED).click();
    wait(&mut harness);
    let doc = harness.state().workbench.document();
    let bytes = fileio::project::to_bytes(doc).unwrap();
    let back = fileio::project::from_bytes(&bytes).unwrap();
    let material = back.scene().material_list(cube(&harness))[0].name.clone();
    let set = back.scene().paint().set(&material).unwrap();
    assert_eq!(given_graph(set), Some(id));
    assert_ne!(set.layers[0].graph.as_ref().unwrap().made, 0, "made, kept");
    assert!(back.scene().graphs().get(id).is_some());
}

#[test]
fn the_attributes_graph_menu_gives_and_takes_back() {
    let (mut harness, id) = page(stone());
    let cube = cube(&harness);
    let workbench = &mut harness.state_mut().workbench;
    workbench.mode = Mode::Object;
    let material = workbench.document().scene().material_list(cube)[0]
        .name
        .clone();
    workbench.attributes.give_graph = Some((cube, material.clone(), Some(id)));
    harness.run();
    assert_eq!(given_graph(&set(&harness).unwrap()), Some(id));
    assert!(
        harness.query_by_label("Made by the graph Stone").is_some(),
        "Attributes says what makes it"
    );
    harness.state_mut().workbench.attributes.give_graph = Some((cube, material, None));
    harness.run();
    assert!(set(&harness).is_none(), "taken back");
    assert!(note(&harness).contains("no longer made by a graph"));
}

#[test]
fn painted_textures_keep_their_paint_over_the_graph() {
    let (mut harness, id) = page(stone());
    let cube = cube(&harness);
    let workbench = &mut harness.state_mut().workbench;
    let material = workbench.document().scene().material_list(cube)[0]
        .name
        .clone();
    let mut painted = paint::TextureSet::new(64).unwrap();
    painted.layers.push(paint::Layer::paint(
        "Scratches",
        paint::Image::new(64).unwrap(),
    ));
    workbench
        .document_mut()
        .execute(scene::Command::SetTextureSet {
            material: material.clone(),
            set: Some(painted),
        })
        .unwrap();
    let given = workbench.give_graph(&[(cube, material)], id);
    assert_eq!(given, vec![crate::material::giving::Given::UnderPaint]);
    let set = set(&harness).unwrap();
    assert_eq!(set.layers.len(), 2);
    assert_eq!(set.layers[1].name, "Scratches", "the paint stays on top");
    assert!(note(&harness).contains("stays on top"));
}

#[test]
fn nothing_selected_says_what_to_do() {
    let (mut harness, id) = page(stone());
    let workbench = &mut harness.state_mut().workbench;
    workbench.document_mut().set_selection([]);
    workbench.give_graph_to_selected(id);
    assert!(note(&harness).starts_with("Select a shape first"));
    assert!(set(&harness).is_none());
}

#[test]
fn export_textures_asks_for_a_folder_and_writes_the_set() {
    let (mut harness, _) = page(stone());
    harness.get_by_label(EXPORT_TEXTURES).click();
    harness.run();
    assert!(
        harness
            .state()
            .actions
            .contains(&WorkbenchAction::ChooseGraphTexturesFolder)
    );
    let folder = std::env::temp_dir().join(format!("graph-textures-ui-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let workbench = &mut harness.state_mut().workbench;
    workbench.material.preview.budget_size = Some(256);
    workbench.export_graph_textures_into(folder.clone());
    let said = workbench.material.export_set.finish().unwrap();
    assert!(
        said.starts_with("Exported Stone's textures (5 files)"),
        "{said}"
    );
    // Godot's (the standard Made for): colour, ORM, normal, height and a
    // material.
    for name in [
        "Stone.png",
        "Stone_ORM.png",
        "Stone_Normal.png",
        "Stone_Height.png",
        "Stone.tres",
    ] {
        assert!(folder.join(name).is_file(), "{name}");
    }
    let colour =
        fileio::image_file::read_image(&std::fs::read(folder.join("Stone.png")).unwrap()).unwrap();
    assert_eq!(
        (colour.width, colour.height),
        (256, 256),
        "the budget's size"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_graph_that_makes_no_material_cant_be_exported_and_says_why() {
    let registry = Registry::standard();
    let mut graph = Graph::new("Unfinished");
    graph
        .apply(
            registry,
            &GraphEdit::AddBox {
                kind: "noise".into(),
                at: Vec2::ZERO,
            },
        )
        .unwrap();
    let (mut harness, _) = page(graph);
    let (stops, checks) = crate::material::export_set::checks(
        registry,
        harness
            .state()
            .workbench
            .material
            .current(harness.state().workbench.document())
            .unwrap(),
    );
    assert!(stops);
    assert!(checks[0].contains("no Material out"), "{checks:?}");
    let folder = std::env::temp_dir().join(format!("graph-textures-none-{}", std::process::id()));
    harness
        .state_mut()
        .workbench
        .export_graph_textures_into(folder.clone());
    assert!(note(&harness).starts_with("Couldn't export Unfinished's textures"));
    assert!(!folder.exists(), "nothing written");
    harness.run();
}

#[test]
fn the_export_window_says_when_a_graph_material_is_still_being_made() {
    let (mut harness, _) = page(stone());
    harness.get_by_label(GIVE_TO_SELECTED).click();
    harness.step();
    let material = harness
        .state()
        .workbench
        .document()
        .scene()
        .material_list(cube(&harness))[0]
        .name
        .clone();
    // Given but not made yet.
    let checks = harness
        .state()
        .workbench
        .graph_export_checks(std::slice::from_ref(&material));
    assert!(
        checks.iter().any(|c| c.contains("still being made")),
        "{checks:?}"
    );
    wait(&mut harness);
    let checks = harness
        .state()
        .workbench
        .graph_export_checks(std::slice::from_ref(&material));
    assert!(checks.is_empty(), "made: nothing to say ({checks:?})");
    // The texture export bakes it: colour, ORM and normal map go out.
    let doc = harness.state().workbench.document();
    let objects = fileio::export::prepare(doc, &fileio::export::ExportOptions::default());
    let textures = fileio::export::textures::make(
        doc,
        &objects,
        fileio::export::TextureFormat::Png,
        8,
        "model",
        fileio::export::ChannelPacking::Standard,
        true,
        false,
    );
    let maps: Vec<_> = textures.iter().map(|t| t.map).collect();
    use fileio::export::TextureMap::*;
    assert_eq!(maps, vec![Colour, Orm, Normal]);
    assert!(textures.iter().all(|t| t.report.size == 64));
}
