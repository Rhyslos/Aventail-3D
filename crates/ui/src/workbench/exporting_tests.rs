//! Tests for exporting and importing in the window: the File menu's
//! entries, the Export window, Export again and imported objects.

use super::tests::{fresh, main_menu};
use super::*;
use egui_kittest::kittest::{NodeT, Queryable};
use fileio::export::{ExportFormat, ExportSummary, ExportWhat, FileUnits, UpAxis};

fn file_entry(harness: &mut super::modelling_tests::H, name: &str) {
    main_menu(harness, "File").click();
    harness.run();
    harness.get_by_label(name).click();
    harness.run();
}

fn export_requests(harness: &super::modelling_tests::H) -> Vec<ExportRequest> {
    harness
        .state()
        .actions
        .iter()
        .filter_map(|a| match a {
            WorkbenchAction::Export(request) => Some((**request).clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_export_window_asks_the_app_to_export_as_chosen() {
    let mut harness = fresh();
    file_entry(&mut harness, "Export…");
    assert!(harness.query_by_label("Kind of file").is_some());
    harness.get_by_label_contains("OBJ (.obj)").click();
    harness.run();
    harness.get_by_label("Centimetres").click();
    harness.run();
    harness.get_by_label("Z up").click();
    harness.run();
    harness.get_by_label("Cut polygons into triangles").click();
    harness.run();
    // Nothing selected: the selection can't be picked.
    harness.get_all_by_label("Export…").last().unwrap().click();
    harness.run();
    let requests = export_requests(&harness);
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(request.format, ExportFormat::Obj);
    assert_eq!(request.path, None, "the app asks where");
    assert_eq!(request.suggested_name, "Untitled.obj");
    let options = request.options;
    assert_eq!(
        (options.units, options.up),
        (FileUnits::Centimetres, UpAxis::Z)
    );
    assert!(options.triangulate);
    assert_eq!(options.what, ExportWhat::Everything);
    // Automatic hard edges is on as standard: shading by angle.
    assert!(matches!(options.hardness, mesh::Hardness::ByAngle(_)));
    assert!(harness.query_by_label("Kind of file").is_none(), "closed");
}

#[test]
fn gltf_is_the_standard_and_fixes_its_own_space() {
    let mut harness = fresh();
    file_entry(&mut harness, "Export…");
    harness.get_all_by_label("Export…").last().unwrap().click();
    harness.run();
    let request = export_requests(&harness).pop().unwrap();
    assert_eq!(request.format, ExportFormat::Glb);
    assert_eq!(request.suggested_name, "Untitled.glb");
}

#[test]
fn export_again_repeats_the_last_export_to_the_same_file() {
    let mut harness = fresh();
    main_menu(&harness, "File").click();
    harness.run();
    assert!(
        harness
            .get_by_label("Export again")
            .accesskit_node()
            .is_disabled()
    );
    harness.key_press(egui::Key::Escape);
    harness.run();
    let request = ExportRequest {
        format: ExportFormat::Obj,
        options: fileio::export::ExportOptions::default(),
        path: None,
        suggested_name: "Untitled.obj".into(),
    };
    harness.state_mut().workbench.exported(
        &request,
        "/work/crate.obj".into(),
        ExportSummary {
            objects: 1,
            triangles: 12,
        },
    );
    assert!(
        harness
            .state()
            .workbench
            .note
            .as_deref()
            .unwrap()
            .starts_with("Exported 1 object (12 triangles) to crate.obj")
    );
    file_entry(&mut harness, "Export again");
    let requests = export_requests(&harness);
    assert_eq!(
        requests.last().unwrap().path.as_deref(),
        Some(std::path::Path::new("/work/crate.obj"))
    );
}

#[test]
fn imported_objects_join_the_scene_as_one_step() {
    let mut harness = fresh();
    let text = "o Crate\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\nf 1 3 9\n";
    let import = fileio::obj::read_obj(text, FileUnits::Metres, UpAxis::Y).unwrap();
    let steps = harness.state().workbench.document().undo_steps();
    harness
        .state_mut()
        .workbench
        .imported(std::path::Path::new("/in/crate.obj"), import);
    harness.run();
    let workbench = &harness.state().workbench;
    let doc = workbench.document();
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.undo_label(), Some("Import crate.obj"));
    let id = doc.selection()[0];
    assert_eq!(doc.scene().object(id).unwrap().name, "Crate");
    assert!(matches!(
        doc.scene().object(id).unwrap().kind,
        scene::ObjectKind::Mesh(_)
    ));
    let note = workbench.note.as_deref().unwrap();
    assert!(
        note.starts_with("Imported 1 object from crate.obj. 1 broken polygon was left out."),
        "{note}"
    );
    // Asking to import asks the app.
    file_entry(&mut harness, "Import…");
    assert!(
        harness
            .state()
            .actions
            .iter()
            .any(|a| matches!(a, WorkbenchAction::Import { path: None }))
    );
}
