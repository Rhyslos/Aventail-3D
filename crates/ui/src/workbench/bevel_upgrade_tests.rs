//! A bevel made before the improved bevel (decision 0141) keeps its shape
//! until a person changes it; then it is made the improved way.

use super::modelling_tests::{
    bevel_ready, ctrl_wheel, cube, drag_by, edge_middle, last_operation, shape, top_right_edge,
};
use super::tests::State;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use scene::{Command, Operation};

type H = Harness<'static, State>;

fn options(harness: &H) -> scene::BevelOptions {
    match last_operation(harness) {
        Some(Operation::BevelEdges { shape, .. }) => shape.options,
        other => panic!("not a bevel: {other:?}"),
    }
}

/// A bevel just made, then turned into one as an older file holds it.
fn old_bevel() -> H {
    let mut harness = bevel_ready(egui::Key::Num2, top_right_edge);
    let from = edge_middle(&harness);
    drag_by(&mut harness, from, egui::vec2(0.0, -60.0));
    assert!(options(&harness).improved, "new bevels are improved");
    let Some(Operation::BevelEdges {
        edges,
        shape: s,
        stretch,
        anchors,
    }) = last_operation(&harness)
    else {
        panic!("a bevel");
    };
    let id = cube(&harness);
    let doc = harness.state_mut().workbench.document_mut();
    let layers = doc.scene().object(id).unwrap().stack.layers();
    let (layer, index) = (layers.len() - 1, layers.last().unwrap().entries.len() - 1);
    let operation = Operation::BevelEdges {
        edges,
        shape: scene::BevelShape {
            options: scene::BevelOptions {
                improved: false,
                ..s.options
            },
            ..s
        },
        stretch,
        anchors,
    };
    doc.execute(Command::ReplaceOperation {
        id,
        layer,
        index,
        operation,
    })
    .unwrap();
    harness.run();
    assert!(!options(&harness).improved);
    harness
}

#[test]
fn changing_an_old_bevels_settings_improves_it() {
    let mut harness = old_bevel();
    harness
        .get_all_by_label("Even width")
        .last()
        .unwrap()
        .click();
    harness.run();
    let now = options(&harness);
    assert!(now.improved && !now.even, "{now:?}");
    // The next bevel is improved too.
    assert!(harness.state().workbench.attributes.bevel.options.improved);
    let report = mesh::MeshReport::new(&shape(&harness));
    assert!(report.is_closed() && !report.has_problems(), "{report:?}");
}

#[test]
fn ctrl_wheel_on_an_old_bevel_improves_it() {
    let mut harness = old_bevel();
    let middle = edge_middle(&harness);
    ctrl_wheel(&mut harness, middle, true);
    assert!(options(&harness).improved);
}

#[test]
fn the_switch_is_called_stop_at_neighbours() {
    let harness = bevel_ready(egui::Key::Num2, top_right_edge);
    assert!(harness.query_all_by_label("Stop at neighbours").count() >= 1);
    assert_eq!(harness.query_all_by_label("Stop at overlaps").count(), 0);
}
