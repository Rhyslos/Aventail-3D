//! The example scripts all check, run, and do what the manual says.

use super::*;
use crate::output::{Ending, Event, Line};
use crate::run::tests::{names, run_on, snapshot};
use crate::{Choice, Limits, Moment, Run, SettingValue};
use scene::{Change, Document};

fn example(file: &str) -> ScriptFile {
    examples(&names())
        .into_iter()
        .find(|e| e.name == file)
        .unwrap_or_else(|| panic!("no example {file}"))
}

/// Runs the example `file` on the test scene, and gives back what it
/// said and the document once its changes landed.
fn run_example(file: &str, snapshot: crate::Snapshot) -> (Vec<Line>, Document) {
    let mut document =
        Document::from_scene(snapshot.scene.clone(), snapshot.selection.iter().copied());
    let script = example(file);
    let ran = run_on(&script.name, &script.text, snapshot, Limits::default());
    assert_eq!(ran.ending, Ending::Finished, "{file}: {:?}", ran.lines);
    if let Some(landing) = ran.landing {
        document.execute_batch(landing.changes, file).unwrap();
        assert_eq!(document.scene(), &landing.scene);
    }
    (ran.lines, document)
}

fn names_in(document: &Document) -> Vec<String> {
    let scene = document.scene();
    scene
        .walk()
        .into_iter()
        .map(|(id, _)| scene.object(id).unwrap().name.clone())
        .collect()
}

#[test]
fn every_example_checks_reads_its_header_and_names_the_folder() {
    let all = examples(&names());
    assert_eq!(all.len(), 6);
    for file in &all {
        assert!(!file.text.contains("%folder%"), "{}", file.name);
        assert!(file.text.contains("fetch \"aventail/"), "{}", file.name);
        let problems = crate::check(file, &names());
        assert!(problems.is_empty(), "{}: {problems:?}", file.name);
        let header = crate::read_header(&file.text);
        assert!(header.notes.is_empty(), "{}: {:?}", file.name, header.notes);
        assert_eq!(header.api, Some(1), "{}", file.name);
        assert!(header.about.is_some(), "{}", file.name);
        assert_eq!(
            format!("{}.lsc", header.name.as_deref().unwrap_or("")),
            file.name
        );
        let described = crate::commands_of(file, &names(), Limits::default());
        assert!(described.problems.is_empty(), "{}", file.name);
        assert_eq!(described.commands.len(), 1, "{}", file.name);
    }
    // Another scripting folder's name goes in their fetch lines.
    let other = crate::Names {
        folder: "other".into(),
        made_by: "x".into(),
    };
    assert!(
        examples(&other)
            .iter()
            .all(|f| f.text.contains("fetch \"other/"))
    );
}

#[test]
fn rename_by_pattern_renames_the_selection() {
    let (lines, document) = run_example("Rename by pattern.lsc", snapshot().0);
    assert!(names_in(&document).contains(&"Crate".to_string()));
    assert!(lines.contains(&Line::Said("Renamed 1 shapes.".into())));
}

#[test]
fn rename_by_pattern_with_nothing_selected_renames_every_shape_and_adds_a_prefix() {
    let (mut snapshot, _) = snapshot();
    snapshot.selection.clear();
    let script = example("Rename by pattern.lsc");
    let events = Run::start_command(
        script,
        snapshot.clone(),
        Limits::default(),
        names(),
        Choice {
            command: "RenameByPattern".into(),
            values: vec![
                ("find".into(), SettingValue::Text(String::new())),
                ("prefix".into(), SettingValue::Text("SM_".into())),
            ],
        },
    )
    .wait();
    let landing = events
        .into_iter()
        .find_map(|e| match e {
            Event::Land(l) => Some(l),
            _ => None,
        })
        .unwrap();
    let scene = &landing.scene;
    let renamed: Vec<String> = scene
        .walk()
        .into_iter()
        .map(|(id, _)| scene.object(id).unwrap().name.clone())
        .collect();
    assert!(renamed.iter().all(|n| n.starts_with("SM_")), "{renamed:?}");
}

#[test]
fn gear_builds_a_closed_gear_of_its_teeth() {
    let (_, document) = run_example("Gear.lsc", snapshot().0);
    let scene = document.scene();
    let gear = scene
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| scene.object(*id).unwrap().name == "Gear")
        .unwrap();
    let mesh = scene.evaluated(gear).unwrap().mesh;
    // 12 teeth, four points each round the edge, top and bottom, and the
    // two middles; three polygons for each point round the edge.
    assert_eq!(mesh.vertex_count(), 2 + 12 * 4 * 2);
    assert_eq!(mesh.face_count(), 12 * 4 * 3);
    // Closed: every edge has a polygon on each side.
    assert!(mesh::remesh::is_closed(&mesh));
}

#[test]
fn check_before_export_warns_by_hand_and_stops_a_heavy_export() {
    let (lines, document) = run_example("Check before export.lsc", snapshot().0);
    // Run by hand it only warns, changing nothing.
    assert!(
        lines.iter().any(
            |l| matches!(l, Line::Warning(w) if w.starts_with("Cube still has a standard name"))
        ),
        "{lines:?}"
    );
    assert_eq!(document.undo_steps(), 0);
    let (mut snapshot, cube) = snapshot();
    snapshot.export = Some(crate::ExportInfo {
        shapes: vec![cube],
        format: fileio::export::ExportFormat::Glb,
    });
    let script = example("Check before export.lsc");
    let events = Run::start_moment(
        script,
        snapshot,
        Limits::default(),
        names(),
        Moment::ExportStarting,
        vec![Choice {
            command: "CheckBeforeExport".into(),
            values: vec![("mostPolygons".into(), SettingValue::Int(2))],
        }],
    )
    .wait();
    let cancelled: Vec<String> = events
        .iter()
        .filter_map(|e| match e {
            Event::Cancelled(why) => Some(why.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(cancelled, ["Cube has more than 2 polygons"]);
}

#[test]
fn export_each_shape_writes_a_file_for_each_selected_shape() {
    let root = std::env::temp_dir().join(format!("script-examples-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let (mut snapshot, _) = snapshot();
    snapshot.access = crate::Access {
        folder: Some(root.clone()),
        read: true,
        write: true,
        chosen: false,
    };
    let (lines, _) = run_example("Export each shape.lsc", snapshot);
    assert!(root.join("exports/Cube.glb").is_file(), "{lines:?}");
    assert!(!root.join("exports/Sphere.glb").exists());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn select_polygons_facing_up_selects_the_top_of_a_cube() {
    let script = example("Select polygons facing up.lsc");
    let (snapshot, cube) = snapshot();
    let ran = run_on(&script.name, &script.text, snapshot, Limits::default());
    assert_eq!(ran.ending, Ending::Finished, "{:?}", ran.lines);
    let landing = ran.landing.unwrap();
    let selected: Vec<_> = landing
        .changes
        .iter()
        .filter_map(|c| match c {
            Change::SelectElements(id, elements) if *id == cube => Some(elements.len()),
            _ => None,
        })
        .collect();
    assert_eq!(selected, [1]);
}

#[test]
fn tidy_names_tidies_by_hand_and_as_the_project_is_saved() {
    let (snapshot, cube) = snapshot();
    let mut document = Document::from_scene(snapshot.scene.clone(), [cube]);
    document
        .execute(scene::Command::Rename {
            id: cube,
            name: "  Big   crate ".into(),
        })
        .unwrap();
    let mut snapshot = snapshot;
    snapshot.scene = document.scene().copy_with_results();
    let (_, tidied) = run_example("Tidy names.lsc", snapshot.clone());
    assert!(names_in(&tidied).contains(&"Big crate".to_string()));
    let script = example("Tidy names.lsc");
    let events = Run::start_moment(
        script,
        snapshot,
        Limits::default(),
        names(),
        Moment::ProjectSaved,
        Vec::new(),
    )
    .wait();
    let landing = events
        .into_iter()
        .find_map(|e| match e {
            Event::Land(l) => Some(l),
            _ => None,
        })
        .unwrap();
    assert_eq!(landing.scene.object(cube).unwrap().name, "Big crate");
}
