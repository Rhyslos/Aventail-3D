//! Tests for exporting and importing in the window: the File menu's
//! entries, the Export window, Export again and imported objects.

use super::tests::{fresh, main_menu};
use super::*;
use crate::workbench::exporting::ExportSection;
use egui_kittest::kittest::{NodeT, Queryable};
use fileio::export::{ExportFormat, ExportPreset, ExportSummary, ExportWhat, FileUnits, UpAxis};

fn file_entry(harness: &mut super::modelling_tests::H, name: &str) {
    main_menu(harness, "File").click();
    harness.run();
    harness.get_by_label(name).click();
    harness.run();
}

/// Picks `name` from the Export window's Made for list.
fn made_for(harness: &mut super::modelling_tests::H, name: &str) {
    harness.get_by_label_contains("Made for:").click();
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
    // A preset's settings are fixed; Custom settings shows them.
    assert!(harness.query_by_label("File type").is_none());
    assert!(
        harness
            .query_by_label("glTF · metres · Y up · triangles")
            .is_some()
    );
    made_for(&mut harness, "Custom settings");
    assert!(harness.query_by_label("File type").is_some());
    harness.get_by_label("OBJ (.obj)").click();
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
    assert!(harness.query_by_label("File type").is_none(), "closed");
    // Custom settings: no preset.
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
    made_for(&mut harness, "Substance Painter");
    let choice = harness.state().workbench.exporting.choice;
    assert_eq!(choice.preset, Some(ExportPreset::SubstancePainter));
    assert_eq!(choice.format, ExportFormat::Obj);
    assert_eq!((choice.units, choice.up), (FileUnits::Metres, UpAxis::Y));
    assert!(choice.triangulate);
    // Substance Painter offers the bake check.
    harness.get_by_label("Check for baking…").click();
    harness.run();
    assert!(harness.state().workbench.baking.open);
    harness.state_mut().workbench.baking.open = false;
    harness.run();
    // Custom settings keeps them, to change; unticking the triangles
    // makes a file no preset makes.
    made_for(&mut harness, "Custom settings");
    let custom = harness.state().workbench.exporting.choice;
    assert_eq!(custom.preset, None);
    assert_eq!(custom.format, ExportFormat::Obj);
    harness.get_by_label("Cut polygons into triangles").click();
    harness.run();
    assert!(!harness.state().workbench.exporting.choice.triangulate);
    // Back to Godot: glTF.
    made_for(&mut harness, "Godot");
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
    // The General section says how to set it up, and the link goes there.
    harness
        .get_all_by_label("Game project")
        .last()
        .unwrap()
        .click();
    harness.run();
    assert_eq!(
        harness.state().workbench.exporting.section,
        ExportSection::GameProject
    );
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
    // The General section says where every save goes.
    harness.get_by_label("General").click();
    harness.run();
    assert!(
        harness
            .query_by_label_contains("Every time you save the project")
            .is_some()
    );
    // File → Export to game project exports there, named after the project.
    // (Esc closes the Export window.)
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!harness.state().workbench.exporting.open);
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
    // Switch Export on every save off, in the Game project section: saving
    // asks for nothing.
    file_entry(&mut harness, "Export…");
    harness
        .get_all_by_label("Game project")
        .next()
        .unwrap()
        .click();
    harness.run();
    assert!(
        harness
            .query_by_label_contains("In the Unity project")
            .is_some()
    );
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

#[test]
fn numbers_and_preset_lines_read_well() {
    use super::exporting::{settings_line, thousands};
    assert_eq!(thousands(0), "0");
    assert_eq!(thousands(999), "999");
    assert_eq!(thousands(1284), "1,284");
    assert_eq!(thousands(1_234_567), "1,234,567");
    assert_eq!(
        settings_line(ExportPreset::Godot),
        "glTF · metres · Y up · triangles"
    );
    assert_eq!(
        settings_line(ExportPreset::SubstancePainter),
        "OBJ · metres · Y up · triangles"
    );
}

#[test]
fn what_goes_out_follows_the_scene_and_the_selection() {
    let mut harness = fresh();
    file_entry(&mut harness, "Export…");
    // One cube: 12 triangles, one material.
    assert!(harness.query_by_label("12").is_some());
    assert!(harness.query_by_label("triangles").is_some());
    // Selected only, with nothing selected, stays on everything shown.
    harness.get_by_label("Selected only").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.exporting.choice.what,
        ExportWhat::Everything
    );
    // Hide the cube: nothing to export, and Export… can't be pressed.
    let id = harness.state().workbench.document().scene().walk()[0].0;
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetVisible {
            ids: vec![id],
            visible: false,
        })
        .unwrap();
    harness.run();
    assert!(
        harness
            .query_by_label_contains("Nothing to export")
            .is_some()
    );
    assert!(
        harness
            .get_all_by_label("Export…")
            .last()
            .unwrap()
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn solis_is_in_made_for_and_gets_plain_gltf() {
    // Decision 0122: Solis is one more preset; Godot stays the first.
    let mut harness = fresh();
    file_entry(&mut harness, "Export…");
    assert_eq!(
        harness.state().workbench.exporting.choice.preset,
        Some(ExportPreset::Godot)
    );
    made_for(&mut harness, "Solis");
    assert!(
        harness
            .query_by_label("glTF · metres · Y up · triangles")
            .is_some()
    );
    harness.get_all_by_label("Export…").last().unwrap().click();
    harness.run();
    let request = export_requests(&harness).pop().unwrap();
    assert_eq!(request.preset, Some(ExportPreset::Solis));
    assert_eq!(request.format, ExportFormat::Glb);
    assert!(!request.options.godot_names, "Godot's names are Godot's");
}

/// An export the app made of `preset`'s file.
fn exported_for(harness: &mut super::modelling_tests::H, preset: Option<ExportPreset>) {
    let format = preset.map_or(ExportFormat::Obj, |p| p.settings().format);
    let request = ExportRequest {
        preset,
        format,
        options: fileio::export::ExportOptions {
            units: FileUnits::Centimetres,
            up: UpAxis::Z,
            triangulate: false,
            ..fileio::export::ExportOptions::default()
        },
        path: None,
        suggested_name: "Untitled.obj".into(),
    };
    harness.state_mut().workbench.exported(
        &request,
        "/work/crate.obj".into(),
        ExportSummary::default(),
    );
}

#[test]
fn the_last_export_is_what_the_next_launch_starts_with() {
    let mut harness = fresh();
    exported_for(&mut harness, Some(ExportPreset::Solis));
    let settings = harness.state().workbench.export_settings();
    assert_eq!(settings.preset, Some(ExportPreset::Solis));
    // The app keeps them as it keeps its other settings, and the next
    // launch (or any new project in it) starts the Export window there.
    let kept = ron::to_string(&settings).unwrap();
    let back: ExportSettings = ron::from_str(&kept).unwrap();
    assert_eq!(back, settings);
    let mut harness = super::tests::harness_for(super::tests::with_a_cube(
        Workbench::new(super::tests::info(), None).with_export_settings(back),
    ));
    harness.run();
    file_entry(&mut harness, "Export…");
    assert!(harness.query_by_label_contains("Made for: Solis").is_some());
    harness.get_all_by_label("Export…").last().unwrap().click();
    harness.run();
    assert_eq!(
        export_requests(&harness).pop().unwrap().preset,
        Some(ExportPreset::Solis)
    );
}

#[test]
fn custom_settings_are_remembered_too() {
    let mut harness = fresh();
    exported_for(&mut harness, None);
    let choice = harness.state().workbench.export_settings().choice();
    assert_eq!(choice.preset, None);
    assert_eq!(
        (choice.format, choice.units, choice.up, choice.triangulate),
        (ExportFormat::Obj, FileUnits::Centimetres, UpAxis::Z, false)
    );
    // What goes out belongs to the moment: it starts at everything.
    assert_eq!(choice.what, ExportWhat::Everything);
}

#[test]
fn choosing_without_exporting_changes_nothing_remembered() {
    let mut harness = fresh();
    file_entry(&mut harness, "Export…");
    made_for(&mut harness, "Solis");
    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.export_settings(),
        ExportSettings::default()
    );
    assert_eq!(
        ExportSettings::default().preset,
        Some(ExportPreset::Godot),
        "the first launch starts with Godot"
    );
}

#[test]
fn a_kit_export_is_remembered_like_any_other() {
    let mut harness = fresh();
    file_entry(&mut harness, "Export…");
    made_for(&mut harness, "Solis");
    harness.get_by_label("Cancel").click();
    harness.run();
    let id = super::modelling_tests::cube(&harness);
    let workbench = &mut harness.state_mut().workbench;
    workbench.document_mut().set_selection([id]);
    assert!(workbench.kit_request().is_some());
    assert_eq!(
        workbench.export_settings().preset,
        Some(ExportPreset::Solis)
    );
}

#[test]
fn damaged_remembered_settings_fall_back_safely() {
    // Nothing, or a part, gives the standard for the rest.
    let empty: ExportSettings = ron::from_str("()").unwrap();
    assert_eq!(empty, ExportSettings::default());
    let part: ExportSettings = ron::from_str("(preset: Some(Solis))").unwrap();
    assert_eq!(part.choice().preset, Some(ExportPreset::Solis));
    // A preset's own settings win over settings that disagree with it.
    let mixed = ExportSettings {
        preset: Some(ExportPreset::SubstancePainter),
        format: ExportFormat::Glb,
        units: FileUnits::Centimetres,
        up: UpAxis::Z,
        triangulate: false,
    };
    let choice = mixed.choice();
    assert_eq!(choice.format, ExportFormat::Obj);
    assert!(choice.follows(ExportPreset::SubstancePainter));
    // A preset the app doesn't know is refused, so the app starts with
    // the standard settings rather than a guess.
    assert!(ron::from_str::<ExportSettings>("(preset: Some(Blender))").is_err());
    // Anything at all is either read or refused, never a crash.
    for text in [
        "",
        "(",
        "((((",
        "(format: 7)",
        "(preset: None, up: X)",
        "\u{0}",
    ] {
        let _ = ron::from_str::<ExportSettings>(text);
    }
}

#[test]
fn a_folder_no_engine_claims_uses_made_fors_engine() {
    // A plain folder, in no engine's project, gets the engine Made for names.
    let root = std::env::temp_dir().join(format!("ui-solis-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let mut harness = fresh();
    file_entry(&mut harness, "Export…");
    made_for(&mut harness, "Solis");
    harness
        .state_mut()
        .workbench
        .export_folder_chosen(root.clone());
    harness.run();
    let workbench = &mut harness.state_mut().workbench;
    assert!(
        workbench
            .note
            .as_deref()
            .unwrap()
            .contains("with Solis's settings")
    );
    assert_eq!(
        workbench.target_request(None).unwrap().preset,
        ExportPreset::Solis
    );
    // Made for a texturing program: the folder still gets an engine's.
    workbench.exporting.choice = workbench
        .exporting
        .choice
        .with_preset(ExportPreset::SubstancePainter);
    assert_eq!(
        workbench.target_request(None).unwrap().preset,
        ExportPreset::Godot
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_solis_project_is_recognised_and_switches_made_for() {
    let root = std::env::temp_dir().join(format!("ui-solis-project-{}", std::process::id()));
    let models = root.join("models");
    std::fs::create_dir_all(&models).unwrap();
    std::fs::write(root.join("project.solis"), "").unwrap();
    let mut harness = fresh();
    file_entry(&mut harness, "Export…");
    assert_eq!(
        harness.state().workbench.exporting.choice.preset,
        Some(ExportPreset::Godot)
    );
    harness
        .state_mut()
        .workbench
        .export_folder_chosen(models.clone());
    harness.run();
    let workbench = &mut harness.state_mut().workbench;
    let note = workbench.note.clone().unwrap();
    assert!(note.contains("Solis project"), "{note}");
    assert_eq!(workbench.exporting.choice.preset, Some(ExportPreset::Solis));
    // The project's engine wins over Made for.
    workbench.exporting.choice = workbench.exporting.choice.with_preset(ExportPreset::Unity);
    assert_eq!(
        workbench.target_request(None).unwrap().preset,
        ExportPreset::Solis
    );
    let _ = std::fs::remove_dir_all(root);
}
