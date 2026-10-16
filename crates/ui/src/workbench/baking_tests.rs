//! Tests for the bake check in the window (decision 0077): File → Check
//! for baking…, its list, Select and Fix.

use super::modelling_tests::{H, cube, last_operation, note, shape};
use super::tests::{fresh, main_menu};
use egui_kittest::kittest::Queryable;
use scene::Operation;

fn open_check(harness: &mut H) {
    main_menu(harness, "File").click();
    harness.run();
    harness.get_by_label("Check for baking…").click();
    harness.run();
}

/// The fresh window's cube with its top flipped (recorded).
fn with_a_flipped_top() -> H {
    let mut harness = fresh();
    let id = cube(&harness);
    let top = shape(&harness)
        .faces()
        .map(|(f, _)| f)
        .max_by(|a, b| {
            let mesh = shape(&harness);
            mesh.face_normal(*a).z.total_cmp(&mesh.face_normal(*b).z)
        })
        .unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Record {
            id,
            operation: Operation::FlipPolygons {
                faces: vec![top],
                anchors: None,
            },
        })
        .unwrap();
    harness.run();
    harness
}

#[test]
fn a_clean_scene_says_so() {
    let mut harness = fresh();
    open_check(&mut harness);
    assert!(
        harness
            .query_by_label("Nothing found that would spoil a bake.")
            .is_some()
    );
}

#[test]
fn a_flipped_polygon_is_listed_selected_and_fixed() {
    let mut harness = with_a_flipped_top();
    open_check(&mut harness);
    assert!(
        harness
            .query_by_label("Cube: 1 polygon facing the wrong way")
            .is_some()
    );
    harness
        .get_by_label("Select Cube: 1 polygon facing the wrong way")
        .click();
    harness.run();
    let workbench = &harness.state().workbench;
    let id = cube(&harness);
    assert_eq!(workbench.mode, crate::palette::Mode::Polygons);
    assert_eq!(workbench.document().selection(), vec![id]);
    assert_eq!(
        workbench
            .document()
            .elements(id)
            .unwrap()
            .polygons()
            .count(),
        1
    );
    harness
        .get_by_label("Fix Cube: 1 polygon facing the wrong way")
        .click();
    harness.run();
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::FixFacing)
    ));
    assert!(note(&harness).contains("Ctrl + Z"));
    // Checked again at once: nothing left.
    assert!(
        harness
            .query_by_label("Nothing found that would spoil a bake.")
            .is_some()
    );
}

#[test]
fn unpaired_low_and_high_objects_are_listed() {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Rename {
            id,
            name: "crate_low".into(),
        })
        .unwrap();
    open_check(&mut harness);
    assert!(
        harness
            .query_by_label("crate_low: no partner called crate_high")
            .is_some()
    );
    // No Fix for it, but Select picks the object.
    assert!(
        harness
            .query_by_label("Fix crate_low: no partner called crate_high")
            .is_none()
    );
    harness
        .get_by_label("Select crate_low: no partner called crate_high")
        .click();
    harness.run();
    assert_eq!(harness.state().workbench.mode, crate::palette::Mode::Model);
}

#[test]
fn the_substance_painter_preset_offers_the_check() {
    let mut harness = fresh();
    main_menu(&harness, "File").click();
    harness.run();
    harness.get_by_label("Export…").click();
    harness.run();
    assert!(harness.query_by_label("Check for baking…").is_none());
    harness.get_by_label_contains("Made for:").click();
    harness.run();
    harness.get_by_label("Substance Painter").click();
    harness.run();
    harness.get_by_label("Check for baking…").click();
    harness.run();
    assert!(harness.state().workbench.baking.open);
}

#[test]
fn points_on_the_same_spot_are_welded_by_fix() {
    let mut harness = fresh();
    // Three triangles, one corner doubled on the same spot.
    let text = "o Patch\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nv 1 1 0\nv 5 5 5\n\
                f 1 2 3\nf 1 5 4\nf 3 5 4\n";
    let import = fileio::obj::read_obj(
        text,
        fileio::export::FileUnits::Metres,
        fileio::export::UpAxis::Y,
    )
    .unwrap();
    harness
        .state_mut()
        .workbench
        .imported(std::path::Path::new("/in/patch.obj"), import);
    harness.run();
    let problems = harness.state_mut().workbench.bake_problems();
    let texts: Vec<String> = problems.iter().map(|p| p.kind.text()).collect();
    assert!(
        texts.contains(&"1 edge with no length".to_string()),
        "{texts:?}"
    );
    for problem in problems
        .iter()
        .filter(|p| p.kind.text().contains("no length"))
    {
        harness.state_mut().workbench.fix_problem(problem);
    }
    harness.run();
    let left: Vec<String> = harness
        .state_mut()
        .workbench
        .bake_problems()
        .iter()
        .map(|p| p.kind.text())
        .collect();
    assert!(!left.iter().any(|t| t.contains("no length")), "{left:?}");
}

#[test]
fn a_fix_that_makes_something_else_worse_says_so() {
    let mut harness = fresh();
    // The saved sample's cube has points on the same spot, among its
    // many edits, that weld into edges shared by three polygons.
    let sample = include_bytes!("../../../fileio/tests/data/sample-v19.avtl");
    let document = fileio::project::from_bytes(sample).unwrap();
    harness
        .state_mut()
        .workbench
        .opened(document, "/w/crate.avtl".into());
    harness.run();
    let problems = harness.state_mut().workbench.bake_problems();
    let weld = problems
        .iter()
        .find(|p| p.name == "Cube" && p.kind.text().contains("no length"))
        .unwrap()
        .clone();
    harness.state_mut().workbench.fix_problem(&weld);
    let text = note(&harness);
    assert!(
        text.contains("but now:") && text.contains("shared by three or more polygons"),
        "{text}"
    );
}
