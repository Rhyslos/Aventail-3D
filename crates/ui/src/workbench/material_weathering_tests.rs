//! The weathering set in the Material mode (decision 0288), used through
//! the real interface: Dirt found in the box search off a material's
//! dropped wire, wired and pictured in one undo step; Edge wear added from
//! "usually comes next"; and a graph using one counted as following the
//! model. No keys or mouse actions of their own.

use super::material_mode_tests::{NOISE, box_at, click, drag, empty_spot, graph, page, plug, undo};
use egui::Modifiers;
use egui_kittest::kittest::Queryable;
use matgraph::{BoxId, GraphEdit, GraphId, PlugRef, Value};
use scene::Command;

type H = egui_kittest::Harness<'static, super::tests::State>;

/// Material from height, added after [`page`]'s five.
const MATERIAL: BoxId = BoxId(6);
/// The weathering box found in the search.
const WEATHER: BoxId = BoxId(7);

fn edit(harness: &mut H, edit: GraphEdit) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(Command::EditGraph {
            id: GraphId(1),
            edit,
        })
        .unwrap();
    harness.run();
}

fn picture(harness: &mut H, id: BoxId) -> Option<Value> {
    let bench = &mut harness.state_mut().workbench;
    let pictures = &bench.material().pictures;
    assert!(!pictures.waiting());
    pictures.value(id).cloned()
}

/// [`page`] with Noise made into a material, and `word` typed into the
/// box search opened by dropping that material's wire on the canvas.
fn searched(word: &str) -> H {
    let mut harness = page();
    edit(
        &mut harness,
        GraphEdit::AddBox {
            kind: "material_from_height".into(),
            at: glam::Vec2::new(500.0, 0.0),
        },
    );
    edit(
        &mut harness,
        GraphEdit::Connect {
            from: PlugRef::new(NOISE, "height"),
            to: PlugRef::new(MATERIAL, "height"),
        },
    );
    let from = plug(&mut harness, MATERIAL, "material", true);
    let spot = empty_spot(&mut harness);
    drag(&mut harness, from, spot, Modifiers::NONE);
    for c in word.chars() {
        harness.event(egui::Event::Text(c.to_string()));
        harness.run();
    }
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness
}

#[test]
fn dirt_comes_from_the_search_off_a_material_and_weathers_it() {
    let mut harness = searched("dirt");
    let g = graph(&harness);
    assert_eq!(g.get(WEATHER).unwrap().kind, "dirt");
    assert_eq!(
        g.wire_into(&PlugRef::new(WEATHER, "material"))
            .unwrap()
            .from
            .node,
        MATERIAL
    );
    // Its picture is the weathered material.
    assert!(matches!(
        picture(&mut harness, WEATHER),
        Some(Value::Material(_))
    ));
    // The graph now follows the model in the painter.
    let registry = harness
        .state()
        .workbench
        .document()
        .scene()
        .graphs()
        .registry()
        .clone();
    assert!(matgraph::model::uses_model(&registry, &g));
    undo(&mut harness);
    assert_eq!(graph(&harness).box_count(), 6, "one step");
}

#[test]
fn water_streaks_are_found_by_what_they_do() {
    let harness = searched("washed");
    assert_eq!(graph(&harness).get(WEATHER).unwrap().kind, "water_streaks");
}

#[test]
fn edge_wear_is_usually_next_after_material_from_height() {
    let mut harness = page();
    edit(
        &mut harness,
        GraphEdit::AddBox {
            kind: "material_from_height".into(),
            at: glam::Vec2::new(500.0, 0.0),
        },
    );
    let (made, _) = box_at(&mut harness, MATERIAL);
    click(
        &mut harness,
        made.left_top() + egui::vec2(20.0, 6.0),
        Modifiers::NONE,
    );
    harness.get_by_label("Edge wear").scroll_to_me();
    harness.run();
    harness.get_by_label("Edge wear").click();
    harness.run();
    let g = graph(&harness);
    assert_eq!(g.get(WEATHER).unwrap().kind, "edge_wear");
    assert!(g.wire_into(&PlugRef::new(WEATHER, "material")).is_some());
}
