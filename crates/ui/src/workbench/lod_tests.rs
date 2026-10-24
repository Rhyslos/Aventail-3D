//! Tests for levels of detail in the window (decision 0155): the
//! Attributes panel's Levels of detail section, the preview in the
//! viewport, and Make levels of detail in command search.

use super::modelling_tests::{H, cube};
use super::tests::fresh;
use crate::objects::DocRequest;
use egui_kittest::kittest::{NodeT, Queryable};
use mesh::decimate::triangle_count;
use scene::{Lods, ObjectId};

/// Runs the window until no levels are made elsewhere.
fn settle(harness: &mut H) {
    harness.run_steps(1);
    for _ in 0..600 {
        if !harness.state().workbench.drawing.busy() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        harness.run_steps(1);
    }
    harness.run_steps(2);
}

fn selected_cube() -> (H, ObjectId) {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    (harness, id)
}

/// Clicks the widget named `name` with `role` (its row's label has the
/// same name).
fn click(harness: &mut H, name: &str, role: egui::accesskit::Role) {
    for scroll in [true, false] {
        {
            let node = harness
                .get_all_by_label(name)
                .find(|n| n.accesskit_node().role() == role)
                .unwrap_or_else(|| panic!("no {role:?} {name}"));
            if scroll {
                node.scroll_to_me();
            } else {
                node.click();
            }
        }
        if scroll {
            harness.run();
        }
    }
}

/// Clicks the only widget named `name`, scrolled into view first.
fn press(harness: &mut H, name: &str) {
    harness.get_by_label(name).scroll_to_me();
    harness.run();
    harness.get_by_label(name).click();
}

fn lods(harness: &H, id: ObjectId) -> Lods {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .object(id)
        .unwrap()
        .lods
        .clone()
}

#[test]
fn make_levels_shows_each_levels_settings_and_triangles() {
    let (mut harness, id) = selected_cube();
    assert_eq!(harness.query_all_by_label("Level 1").count(), 0);
    click(&mut harness, "Make levels", egui::accesskit::Role::CheckBox);
    settle(&mut harness);
    assert!(lods(&harness, id).on);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Change the levels of detail")
    );
    for level in ["Level 1", "Level 2", "Level 3"] {
        assert!(harness.query_all_by_label(level).count() >= 1, "{level}");
    }
    // Made at once for a light shape, so its counts and the full shape's
    // are shown.
    assert!(
        harness
            .query_by_label_contains("The full shape has 12 triangles")
            .is_some()
    );
    press(&mut harness, "Increase Levels");
    settle(&mut harness);
    assert_eq!(lods(&harness, id).levels.len(), 4);
    assert!(harness.query_all_by_label("Level 4").count() >= 1);
    press(&mut harness, "Decrease Level 1");
    settle(&mut harness);
    assert!((lods(&harness, id).levels[0].ratio - 0.49).abs() < 1e-4);
}

#[test]
fn preview_draws_the_level_and_undo_brings_the_full_shape_back() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Create(scene::ObjectKind::Primitive(
            scene::PrimitiveKind::Sphere.standard(),
        )))
        .unwrap();
    harness.run();
    let id = harness.state().workbench.document().selection()[0];
    let drawn = |harness: &mut H| {
        let workbench = &mut harness.state_mut().workbench;
        let summary = workbench
            .drawing
            .update(&workbench.document, None, &workbench.hidden);
        triangle_count(&summary.drawing.items[0].drawn)
    };
    let full = drawn(&mut harness);
    click(&mut harness, "Make levels", egui::accesskit::Role::CheckBox);
    settle(&mut harness);
    click(&mut harness, "Preview", egui::accesskit::Role::ComboBox);
    harness.run();
    harness
        .get_all_by_label("Level 3")
        .last()
        .expect("the choice")
        .click();
    settle(&mut harness);
    assert_eq!(lods(&harness, id).preview, 3);
    assert!(drawn(&mut harness) < full / 4, "the simplest level");
    harness.state_mut().workbench.document_mut().undo().unwrap();
    settle(&mut harness);
    assert_eq!(drawn(&mut harness), full);
}

#[test]
fn make_levels_of_detail_switches_them_on_for_the_selected_shapes() {
    let (mut harness, id) = selected_cube();
    let mut requests = Vec::new();
    harness.state_mut().workbench.make_lods(&mut requests);
    let [DocRequest::Run(scene::Command::SetLods { ids, lods })] = requests.as_slice() else {
        panic!("one command: {requests:?}");
    };
    assert_eq!(ids, &[id]);
    assert!(lods.on);
    // Nothing selected: a note, no command.
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    let mut requests = Vec::new();
    harness.state_mut().workbench.make_lods(&mut requests);
    assert!(requests.is_empty());
    assert!(
        harness
            .state()
            .workbench
            .note
            .as_deref()
            .is_some_and(|n| n.contains("select a shape first"))
    );
}
