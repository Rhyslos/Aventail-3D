//! Animated materials in the Material mode (decision 0277), used through
//! the real interface: the preview's player plays the moving boxes (the
//! live pictures follow), pauses, goes back to the start and seeks;
//! Bake this moment sets what exports bake (one undo step); sliders are
//! marked "change in the game" in Attributes; the export check says
//! both.

use super::tests::{TestRenderer, harness_built_sized, info};
use super::*;
use egui_kittest::kittest::Queryable;
use glam::Vec2;
use matgraph::{BoxId, Graph, GraphEdit, GraphId, PlugRef, Registry};

type H = egui_kittest::Harness<'static, super::tests::State>;

const SCROLL: BoxId = BoxId(2);

/// The Material mode showing Noise → Scroll → Material from height →
/// Material out (or, `still`, Noise → Material from height → Material
/// out), frames a tenth of a second apart.
fn page(still: bool) -> H {
    let mut workbench = Workbench::new(info(), None);
    let registry = Registry::standard();
    let mut graph = Graph::new("Lava");
    let kinds: &[&str] = if still {
        &["noise", "material_from_height", "material_out"]
    } else {
        &["noise", "scroll", "material_from_height", "material_out"]
    };
    for (i, kind) in kinds.iter().enumerate() {
        let edit = GraphEdit::AddBox {
            kind: (*kind).into(),
            at: Vec2::new(i as f32 * 300.0, 0.0),
        };
        graph.apply(registry, &edit).unwrap();
    }
    let wires: &[(u64, &str, u64, &str)] = if still {
        &[(1, "height", 2, "height"), (2, "material", 3, "material")]
    } else {
        &[
            (1, "height", 2, "in"),
            (2, "out", 3, "height"),
            (3, "material", 4, "material"),
        ]
    };
    for (from, out, to, input) in wires {
        let edit = GraphEdit::Connect {
            from: PlugRef::new(BoxId(*from), *out),
            to: PlugRef::new(BoxId(*to), *input),
        };
        graph.apply(registry, &edit).unwrap();
    }
    workbench
        .document_mut()
        .execute(scene::Command::AddGraph(graph))
        .unwrap();
    workbench.mode = Mode::Material;
    let mut harness = harness_built_sized(
        egui_kittest::Harness::builder().with_step_dt(0.1),
        workbench,
        TestRenderer::default(),
        [1440.0, 1800.0],
    );
    harness.run();
    harness.run();
    harness
}

fn graph(harness: &H) -> std::sync::Arc<Graph> {
    let bench = &harness.state().workbench;
    bench
        .document()
        .scene()
        .graphs()
        .get(GraphId(1))
        .unwrap()
        .clone()
}

fn says(harness: &H, words: &str) -> bool {
    harness.query_all_by_label_contains(words).next().is_some()
}

fn click(harness: &mut H, label: &str) {
    harness.get_by_label(label).click();
    harness.run();
}

/// What Scroll's live picture shows now, by its key.
fn scroll_picture(harness: &mut H) -> usize {
    let value = harness
        .state_mut()
        .workbench
        .material()
        .pictures
        .value(SCROLL)
        .cloned()
        .expect("Scroll's picture");
    crate::material::pictures::picture_key(&value)
}

#[test]
fn a_still_graph_has_no_player() {
    let harness = page(true);
    assert!(harness.query_by_label(crate::material::player::PLAY).is_none());
    assert!(!says(&harness, "Exports and the painter bake"));
}

#[test]
fn the_player_plays_the_moving_boxes_and_pauses() {
    let mut harness = page(false);
    assert!(says(&harness, "Exports and the painter bake this moment (0 s)"));
    let still = scroll_picture(&mut harness);
    harness.run();
    assert_eq!(scroll_picture(&mut harness), still, "paused: it stays");
    // While it plays the preview is drawn again and again: step frames.
    harness.get_by_label(crate::material::player::PLAY).click();
    harness.step();
    assert!(harness.state_mut().workbench.material().player.playing);
    for _ in 0..5 {
        harness.step();
    }
    let time = harness.state_mut().workbench.material().player.time;
    assert!(time > 0.2, "the time goes on: {time}");
    assert_ne!(scroll_picture(&mut harness), still, "and Scroll moves");
    // Playing doesn't change the graph: nothing to undo.
    assert_eq!(graph(&harness).moment(), 0.0);
    assert_eq!(harness.state().workbench.document().undo_steps(), 1);
    harness.get_by_label(crate::material::player::PAUSE).click();
    harness.run();
    let paused = harness.state_mut().workbench.material().player.time;
    harness.run();
    harness.run();
    assert_eq!(harness.state_mut().workbench.material().player.time, paused);
    // Back to the start: the first picture again.
    click(&mut harness, crate::material::player::TO_START);
    harness.run();
    assert_eq!(harness.state_mut().workbench.material().player.time, 0.0);
    assert_eq!(scroll_picture(&mut harness), still);
}

#[test]
fn bake_this_moment_sets_what_exports_bake_and_undoes() {
    let mut harness = page(false);
    assert!(harness.query_by_label(crate::material::player::BAKE_MOMENT).is_none());
    harness.state_mut().workbench.material().player.seek(2.5);
    harness.run();
    assert!(says(&harness, "bake the moment at 0 s"));
    click(&mut harness, crate::material::player::BAKE_MOMENT);
    assert_eq!(graph(&harness).moment(), 2.5);
    assert!(says(&harness, "bake this moment (2.5 s)"));
    // The export check says so.
    let registry = Registry::standard();
    let (stops, notes) = crate::material::export_set::checks(registry, &graph(&harness));
    assert!(!stops);
    assert!(
        notes.iter().any(|n| n.contains("Scroll moves over time") && n.contains("at 2.5 s")),
        "{notes:?}"
    );
    let bench = &mut harness.state_mut().workbench;
    bench.document_mut().undo();
    assert_eq!(graph(&harness).moment(), 0.0);
}

#[test]
fn a_graph_opens_paused_at_its_moment() {
    let mut harness = page(false);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::EditGraph {
            id: GraphId(1),
            edit: GraphEdit::SetMoment(4.0),
        })
        .unwrap();
    // Shown afresh (another graph, then back).
    harness.state_mut().workbench.material().show_graph(None);
    harness.run();
    harness
        .state_mut()
        .workbench
        .material()
        .show_graph(Some(GraphId(1)));
    harness.run();
    let player = harness.state_mut().workbench.material().player.clone();
    assert!(!player.playing);
    assert_eq!(player.time, 4.0);
}

#[test]
fn sliders_are_marked_to_change_in_the_game_in_attributes() {
    let mut harness = page(false);
    harness
        .state_mut()
        .workbench
        .material()
        .canvas
        .selected
        .insert(SCROLL);
    harness.run();
    assert!(says(&harness, crate::material::game_rows::IN_GAME));
    let speed = format!("{} Speed", crate::material::game_rows::IN_GAME_SWITCH);
    click(&mut harness, &speed);
    let marked = graph(&harness).get(SCROLL).unwrap().in_game.clone();
    assert!(marked.contains("speed"), "{marked:?}");
    let registry = Registry::standard();
    let (_, notes) = crate::material::export_set::checks(registry, &graph(&harness));
    assert!(
        notes.iter().any(|n| n.contains("Scroll's Speed is marked")),
        "{notes:?}"
    );
    // Undo takes the mark back.
    harness.state_mut().workbench.document_mut().undo();
    assert!(graph(&harness).get(SCROLL).unwrap().in_game.is_empty());
    // Nothing selected: the graph's own rows list the moment.
    harness.state_mut().workbench.material().canvas.selected.clear();
    harness.run();
    assert!(says(&harness, "Some boxes move over time"));
}
