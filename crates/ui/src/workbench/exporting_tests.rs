//! Tests for exporting and importing in the window: the File menu's
//! entries, the Export window, Export again and imported objects.

use super::tests::{fresh, main_menu};
use super::*;
use egui_kittest::kittest::{NodeT, Queryable};
use fileio::export::{ExportFormat, ExportPreset, ExportSummary, ExportWhat, FileUnits, UpAxis};

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
    // Godot's settings (the standard) cut triangles; the click unticked it.
    assert!(!options.triangulate);
    assert_eq!(options.what, ExportWhat::Everything);
    // Automatic hard edges is on as standard: shading by angle.
    assert!(matches!(options.hardness, mesh::Hardness::ByAngle(_)));
    assert!(harness.query_by_label("Kind of file").is_none(), "closed");
    // Changing the settings by hand left the Godot preset.
    assert_eq!(request.preset, None);
}

#[test]
fn presets_choose_the_settings_and_changing_one_leaves_them() {
    let mut harness = fresh();
    file_entry(&mut harness, "Export…");
    // Godot is the standard.
    assert_eq!(
        harness.state().workbench.exporting.choice.preset,
        Some(ExportPreset::Godot)
    );
    harness.get_by_label("Substance Painter").click();
    harness.run();
    let choice = harness.state().workbench.exporting.choice;
    assert_eq!(choice.preset, Some(ExportPreset::SubstancePainter));
    assert_eq!(choice.format, ExportFormat::Obj);
    assert_eq!((choice.units, choice.up), (FileUnits::Metres, UpAxis::Y));
    assert!(choice.triangulate);
    // Untick the triangles: your own settings now.
    harness.get_by_label("Cut polygons into triangles").click();
    harness.run();
    assert_eq!(harness.state().workbench.exporting.choice.preset, None);
    // Back to Godot: glTF.
    harness.get_by_label("Godot").click();
    harness.run();
    let choice = harness.state().workbench.exporting.choice;
    assert_eq!(choice.format, ExportFormat::Glb);
    harness.get_all_by_label("Export…").last().unwrap().click();
    harness.run();
    let request = export_requests(&harness).pop().unwrap();
    assert_eq!(request.preset, Some(ExportPreset::Godot));
    assert_eq!(request.format, ExportFormat::Glb);
    // The status bar says who it was for.
    harness.state_mut().workbench.exported(
        &request,
        "/work/crate.glb".into(),
        ExportSummary {
            objects: 2,
            triangles: 24,
            materials: 3,
        },
    );
    assert!(
        harness
            .state()
            .workbench
            .note
            .as_deref()
            .unwrap()
            .starts_with("Exported 2 objects for Godot (24 triangles, 3 materials) to crate.glb")
    );
}

#[test]
fn a_glb_follows_godot_whatever_its_greyed_out_settings_say() {
    let choice = ExportChoice {
        preset: None,
        format: ExportFormat::Glb,
        what: ExportWhat::Everything,
        units: FileUnits::Centimetres,
        up: UpAxis::Z,
        triangulate: false,
    };
    assert!(choice.follows(ExportPreset::Godot));
    assert!(!choice.follows(ExportPreset::SubstancePainter));
    let obj = ExportChoice {
        format: ExportFormat::Obj,
        ..choice
    };
    assert!(!obj.follows(ExportPreset::SubstancePainter));
    assert!(
        obj.with_preset(ExportPreset::SubstancePainter)
            .follows(ExportPreset::SubstancePainter)
    );
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
        preset: None,
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
            materials: 1,
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

#[test]
fn a_game_project_folder_is_chosen_used_and_cleared() {
    let mut harness = fresh();
    file_entry(&mut harness, "Export…");
    harness.get_by_label("Choose folder…").click();
    harness.run();
    assert!(
        harness
            .state()
            .actions
            .iter()
            .any(|a| matches!(a, WorkbenchAction::ChooseExportFolder { .. }))
    );
    // The app asks, and the user picks a folder inside a Unity project.
    let root = std::env::temp_dir().join(format!("ui-target-{}", std::process::id()));
    let models = root.join("Assets/Models");
    std::fs::create_dir_all(&models).unwrap();
    std::fs::create_dir_all(root.join("ProjectSettings")).unwrap();
    harness
        .state_mut()
        .workbench
        .export_folder_chosen(models.clone());
    harness.run();
    let workbench = &harness.state().workbench;
    let target = workbench.document().scene().export_target().unwrap();
    assert!(target.on_save, "on once a folder is chosen");
    assert_eq!(workbench.exporting.choice.preset, Some(ExportPreset::Unity));
    assert!(workbench.note.as_deref().unwrap().contains("Unity project"));
    assert!(
        harness
            .query_by_label_contains("In the Unity project")
            .is_some()
    );
    // File → Export to game project exports there, named after the project.
    harness.key_press(egui::Key::Escape);
    harness.run();
    file_entry(&mut harness, "Export to game project");
    let request = harness
        .state()
        .actions
        .iter()
        .find_map(|a| match a {
            WorkbenchAction::ExportToTarget(r) => Some((**r).clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(request.folder, models);
    assert_eq!(request.name, "Untitled");
    assert_eq!(request.preset, ExportPreset::Unity);
    // Saving asks for the same, named after the saved project.
    let saving = harness
        .state_mut()
        .workbench
        .save_export_request(std::path::Path::new("/w/crate.avtl"))
        .unwrap();
    assert_eq!(saving.name, "crate");
    harness.state_mut().workbench.saved_with_export(
        "/w/crate.avtl".into(),
        Some(Ok((
            models.join("crate.glb"),
            ExportSummary {
                objects: 1,
                triangles: 12,
                materials: 1,
            },
        ))),
    );
    assert_eq!(
        harness.state().workbench.note.as_deref(),
        Some("Saved crate.avtl. Exported crate.glb (12 triangles) into Models.")
    );
    // Switch Export on every save off (the window is still open): saving
    // asks for nothing.
    harness.get_by_label("Export on every save").click();
    harness.run();
    assert!(
        harness
            .state_mut()
            .workbench
            .save_export_request(std::path::Path::new("/w/crate.avtl"))
            .is_none()
    );
    harness.get_by_label("Clear").click();
    harness.run();
    assert!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .export_target()
            .is_none()
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_failed_export_into_the_game_project_says_why() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .target_exported(Err("the folder isn't there".into()));
    assert_eq!(
        harness.state().workbench.note.as_deref(),
        Some("Couldn't export into the game project: the folder isn't there.")
    );
    harness.run();
}

#[test]
fn export_kit_asks_for_the_selected_pieces() {
    let mut harness = fresh();
    // Nothing selected: it says so.
    file_entry(&mut harness, "Export kit…");
    assert!(
        harness
            .state()
            .workbench
            .note
            .as_deref()
            .unwrap()
            .contains("select the pieces")
    );
    let id = super::modelling_tests::cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    file_entry(&mut harness, "Export kit…");
    let request = harness
        .state()
        .actions
        .iter()
        .find_map(|a| match a {
            WorkbenchAction::ExportKit(r) => Some((**r).clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(request.pieces, vec![id]);
    assert_eq!(request.folder, None, "no game project folder: the app asks");
    assert_eq!(request.format, ExportFormat::Glb);
    assert!(request.options.godot_names, "Godot is the standard preset");
}

#[test]
fn kit_results_are_summed_up() {
    let mut harness = fresh();
    let summary = ExportSummary {
        objects: 1,
        triangles: 12,
        materials: 1,
    };
    harness.state_mut().workbench.kit_exported(Ok(vec![
        ("Wall".into(), Ok(("/kit/Wall.glb".into(), summary))),
        ("Door".into(), Err("Door.glb is already there".into())),
    ]));
    assert_eq!(
        harness.state().workbench.note.as_deref(),
        Some("Exported 1 of 2 pieces into kit. Door: Door.glb is already there.")
    );
    harness
        .state_mut()
        .workbench
        .kit_exported(Err("the folder isn't there".into()));
    assert_eq!(
        harness.state().workbench.note.as_deref(),
        Some("Couldn't export the kit: the folder isn't there.")
    );
}

#[test]
fn the_attributes_panel_sets_a_shapes_collision() {
    let mut harness = fresh();
    let id = super::modelling_tests::cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness.get_by_label("Collision shape").click();
    harness.run();
    harness.get_by_label("A box").click();
    harness.run();
    let doc = harness.state().workbench.document();
    assert_eq!(
        doc.scene().object(id).unwrap().collision,
        scene::Collision::Box
    );
    assert_eq!(doc.undo_label(), Some("Collision: a box"));
}
