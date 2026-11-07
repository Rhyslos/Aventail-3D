//! Graphs in the painter (Phase 12, milestone 5, decision 0261): a graph
//! fill is made on the model off the interface's thread, follows the
//! model (moss on what faces up), is made again when its graph or its own
//! values change, undoes with the change that asked for it, and says why
//! when its graph can't run. The ··· menu and the shelf add one, and
//! "Open in the graph" opens its graph in the Material mode.

use super::paint_layers_tests::{painting_the_cube, set};
use crate::paint_layers::{GRAPH_FILL_MENU, ShelfAsk};
use crate::palette::Mode;
use egui_kittest::kittest::Queryable;
use glam::Vec2;
use matgraph::{BoxId, Graph, GraphEdit, GraphId, PlugRef, Registry, SettingValue};
use paint::{GraphFill, Layer};
use scene::Command;
use std::time::{Duration, Instant};

type H = egui_kittest::Harness<'static, super::tests::State>;

/// The moss box's colour setting, and the moss's colour.
const MOSS_COLOUR: BoxId = BoxId(4);
const MOSS: [f32; 3] = [0.1, 0.6, 0.1];

/// Grey stone everywhere, moss where the model faces up.
fn moss_on_top() -> Graph {
    let registry = Registry::standard();
    let mut g = Graph::new("Mossy stone");
    for (i, kind) in [
        "colour_fill",
        "noise",
        "material_from_height",
        "colour_fill",
        "material_from_height",
        "model_facing_up",
        "levels",
        "mix_materials",
        "material_out",
    ]
    .into_iter()
    .enumerate()
    {
        g.apply(
            registry,
            &GraphEdit::AddBox {
                kind: kind.into(),
                at: Vec2::new(i as f32 * 220.0, 0.0),
            },
        )
        .unwrap();
    }
    let p = |id: u64, plug: &str| PlugRef::new(BoxId(id), plug);
    for (from, to) in [
        (p(1, "colour"), p(3, "colour")),
        (p(2, "height"), p(3, "height")),
        (p(4, "colour"), p(5, "colour")),
        (p(2, "height"), p(5, "height")),
        (p(6, "up"), p(7, "in")),
        (p(3, "material"), p(8, "bottom")),
        (p(5, "material"), p(8, "top")),
        (p(7, "out"), p(8, "where")),
        (p(8, "material"), p(9, "material")),
    ] {
        g.apply(registry, &GraphEdit::Connect { from, to }).unwrap();
    }
    for (node, value) in [(BoxId(1), [0.5, 0.5, 0.5]), (MOSS_COLOUR, MOSS)] {
        g.apply(
            registry,
            &GraphEdit::SetSetting {
                node,
                key: "colour".into(),
                value: SettingValue::Colour(value),
            },
        )
        .unwrap();
    }
    g
}

/// Painting the cube at 64 pixels, with the mossy stone graph in the
/// project: its id.
fn with_graph(graph: Graph) -> (H, GraphId) {
    let mut harness = painting_the_cube();
    let doc = harness.state_mut().workbench.document_mut();
    let mut texture = doc.scene().texture();
    texture.size = 64;
    doc.execute(Command::SetTexture(texture)).unwrap();
    let id = doc.scene().graphs().next_id();
    doc.execute(Command::AddGraph(graph)).unwrap();
    harness.run();
    (harness, id)
}

/// Adds a graph fill running `graph` on top of the cube's layers, as one
/// undo step.
fn add_graph_fill(harness: &mut H, graph: GraphId) {
    let doc = harness.state_mut().workbench.document_mut();
    let mut set = doc
        .scene()
        .paint()
        .set("Default")
        .map(|s| (**s).clone())
        .unwrap_or_else(|| paint::TextureSet::new(64).unwrap());
    let mut layer = Layer::paint("Moss", paint::Image::new(set.size).unwrap());
    layer.graph = Some(GraphFill::of(graph));
    set.layers.push(layer);
    doc.execute_labelled(
        Command::SetTextureSet {
            material: "Default".into(),
            set: Some(set),
        },
        "Add a graph fill",
        None,
    )
    .unwrap();
}

/// The top layer's graph fill.
fn top_fill(harness: &H) -> GraphFill {
    set(harness).layers.last().unwrap().graph.clone().unwrap()
}

/// Runs frames until no graph fill is being made and none is waiting (or
/// half a minute passes).
fn wait(harness: &mut H) {
    let start = Instant::now();
    loop {
        harness.step();
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
        let fills = &harness.state().workbench.graph_fills;
        if !fills.busy() || start.elapsed() > Duration::from_secs(30) {
            break;
        }
    }
    harness.run();
}

/// How many of the top layer's texels are mossy (green) and stony
/// (grey).
fn moss_and_stone(harness: &H) -> (usize, usize) {
    let set = set(harness);
    let layer = set.layers.last().unwrap();
    let image = layer.image().unwrap();
    let (mut moss, mut stone) = (0, 0);
    for y in 0..image.size() {
        for x in 0..image.size() {
            let [r, g, b, a] = image.pixel(x, y);
            if a == 0 {
                continue;
            }
            if g > r + 40 && g > b + 40 {
                moss += 1;
            } else if r.abs_diff(g) < 12 && g.abs_diff(b) < 12 {
                stone += 1;
            }
        }
    }
    (moss, stone)
}

#[test]
fn a_graph_fill_is_made_on_the_model_and_undoes_with_its_adding() {
    let (mut harness, id) = with_graph(moss_on_top());
    let steps = harness.state().workbench.document().undo_steps();
    add_graph_fill(&mut harness, id);
    assert_eq!(top_fill(&harness).made, 0);
    wait(&mut harness);
    let fill = top_fill(&harness);
    assert_ne!(fill.made, 0, "it was made");
    // Moss on the top face only: one side of six faces up.
    let (moss, stone) = moss_and_stone(&harness);
    assert!(moss > 0, "moss on top");
    assert!(stone > moss * 2, "stone on the sides: {moss} {stone}");
    // The pixels joined the step that added the layer.
    let doc = harness.state_mut().workbench.document_mut();
    assert_eq!(doc.undo_steps(), steps + 1);
    doc.undo();
    harness.run();
    assert!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .paint()
            .set("Default")
            .is_none_or(|s| s.layers.iter().all(|l| l.graph.is_none()))
    );
}

#[test]
fn changing_the_graph_makes_the_layer_again_and_undo_puts_both_back() {
    let (mut harness, id) = with_graph(moss_on_top());
    add_graph_fill(&mut harness, id);
    wait(&mut harness);
    let first = top_fill(&harness).made;
    let before = set(&harness);
    // The moss turns red in the Material mode's graph.
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(Command::EditGraph {
            id,
            edit: GraphEdit::SetSetting {
                node: MOSS_COLOUR,
                key: "colour".into(),
                value: SettingValue::Colour([0.8, 0.1, 0.1]),
            },
        })
        .unwrap();
    wait(&mut harness);
    assert_ne!(top_fill(&harness).made, first);
    assert_eq!(moss_and_stone(&harness).0, 0, "no green moss now");
    // One undo takes back the graph and the layer's pixels together.
    harness.state_mut().workbench.document_mut().undo();
    wait(&mut harness);
    assert_eq!(set(&harness), before);
    assert!(!harness.state().workbench.graph_fills.busy());
}

#[test]
fn a_shown_setting_is_set_the_layers_own_way() {
    let (mut harness, id) = with_graph(moss_on_top());
    {
        let doc = harness.state_mut().workbench.document_mut();
        doc.execute(Command::EditGraph {
            id,
            edit: GraphEdit::ShowInPainter {
                node: MOSS_COLOUR,
                key: "colour".into(),
                shown: true,
            },
        })
        .unwrap();
    }
    add_graph_fill(&mut harness, id);
    wait(&mut harness);
    // The panel shows the shown setting under the layer's settings.
    harness.run();
    assert!(
        harness
            .query_by_label_contains(crate::paint_layers::graph_rows::FOLLOWS_LABEL)
            .is_some()
    );
    assert!(harness.query_by_label_contains("Facing up").is_some());
    // The layer's own colour for the moss: blue.
    let doc = harness.state_mut().workbench.document_mut();
    let mut changed = (**doc.scene().paint().set("Default").unwrap()).clone();
    let layer = changed.layers.last_mut().unwrap();
    layer.graph.as_mut().unwrap().set_value(
        MOSS_COLOUR,
        "colour",
        SettingValue::Colour([0.1, 0.1, 0.9]),
    );
    doc.execute(Command::SetTextureSet {
        material: "Default".into(),
        set: Some(changed),
    })
    .unwrap();
    wait(&mut harness);
    let set = set(&harness);
    let image = set.layers.last().unwrap().image().unwrap().clone();
    let blue = (0..64)
        .flat_map(|y| (0..64).map(move |x| (x, y)))
        .any(|(x, y)| {
            let [r, _, b, a] = image.pixel(x, y);
            a > 0 && b > r + 60
        });
    assert!(blue, "blue moss");
    // The graph itself keeps its green.
    let graph = harness
        .state()
        .workbench
        .document()
        .scene()
        .graphs()
        .get(id)
        .unwrap()
        .clone();
    assert_eq!(
        graph.get(MOSS_COLOUR).unwrap().settings.get("colour"),
        Some(SettingValue::Colour(MOSS))
    );
}

#[test]
fn a_graph_that_cant_run_says_why_and_keeps_its_layer() {
    // No Material out: nothing to lay.
    let (mut harness, id) = with_graph(Graph::new("Unfinished"));
    add_graph_fill(&mut harness, id);
    wait(&mut harness);
    assert_eq!(top_fill(&harness).made, 0);
    harness.run();
    assert!(harness.query_by_label_contains("Material out").is_some());
    // It isn't tried again and again.
    harness.run();
    assert!(!harness.state().workbench.graph_fills.busy());
}

#[test]
fn the_menu_and_the_shelf_add_a_graph_fill_and_open_in_the_graph_opens_it() {
    let (mut harness, id) = with_graph(moss_on_top());
    // ··· then + Graph fill, a menu of the project's graphs.
    harness
        .get_by_label(crate::paint_layers::MORE_LABEL)
        .click();
    harness.run();
    harness.get_by_label_contains(GRAPH_FILL_MENU).hover();
    harness.run();
    harness.get_by_label_contains(GRAPH_FILL_MENU).click();
    harness.run();
    harness.get_by_label("Mossy stone").click();
    harness.step();
    harness.step();
    let fill = top_fill(&harness);
    assert_eq!(fill.graph, id);
    assert_eq!(set(&harness).layers.last().unwrap().name, "Mossy stone");
    wait(&mut harness);
    // From the shelf too.
    {
        let state = harness.state_mut();
        let ctx = egui::Context::default();
        let workbench = &mut state.workbench;
        let mut note = None;
        workbench.paint_layers.apply_shelf(
            &ctx,
            &mut workbench.document,
            ShelfAsk::Graph(id),
            &mut note,
        );
    }
    wait(&mut harness);
    let set_now = set(&harness);
    let graph_layers = set_now
        .listed(true)
        .iter()
        .filter(|(p, _)| set_now.layer(p).unwrap().graph.is_some())
        .count();
    assert_eq!(graph_layers, 2);
    wait(&mut harness);
    // Open in the graph: the Material mode, on that graph.
    harness
        .get_by_label(crate::paint_layers::graph_rows::OPEN_GRAPH_LABEL)
        .click();
    harness.run();
    assert_eq!(harness.state().workbench.mode, Mode::Material);
    assert_eq!(harness.state().workbench.material.graph, Some(id));
}

#[test]
fn the_graph_fill_mixes_into_what_the_views_and_export_show() {
    let (mut harness, id) = with_graph(moss_on_top());
    add_graph_fill(&mut harness, id);
    wait(&mut harness);
    let set = set(&harness);
    let composite = paint::Composite::of(&set).unwrap();
    let mixed = composite.image();
    let mossy = (0..64)
        .flat_map(|y| (0..64).map(move |x| (x, y)))
        .filter(|(x, y)| {
            let [r, g, _, a] = mixed.pixel(*x, *y);
            a > 0 && g > r + 40
        })
        .count();
    assert!(mossy > 0);
}

#[test]
fn a_graph_fill_follows_the_texture_size() {
    let (mut harness, id) = with_graph(moss_on_top());
    add_graph_fill(&mut harness, id);
    wait(&mut harness);
    let first = top_fill(&harness).made;
    let doc = harness.state_mut().workbench.document_mut();
    let mut texture = doc.scene().texture();
    texture.size = 128;
    doc.execute(Command::SetTexture(texture)).unwrap();
    wait(&mut harness);
    wait(&mut harness);
    let set = set(&harness);
    assert_eq!(set.size, 128);
    assert_ne!(top_fill(&harness).made, first);
    assert_eq!(set.layers.last().unwrap().image().unwrap().size(), 128);
}
