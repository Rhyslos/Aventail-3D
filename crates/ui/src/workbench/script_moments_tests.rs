//! Scripts that run by themselves, through the interface (decision 0309):
//! saving and exporting waiting for them, an export stopped, a slow
//! script stopped with the save going on, opening, name order, trust and
//! permissions, the script manager's switches, Turn them off, the example
//! scripts and Sculpt mode.

use super::tests::{State, fresh, main_menu};
use super::*;
use crate::script_sharing::ScriptId;
use crate::scripts_panel::Row;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use scene::{Document, ProjectScript};
use script::{Line, Moment};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// A fresh folder for one test.
fn folder(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("script-moments-ui-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A harness with a library in `dir/library` holding `scripts`, each
/// switched on for the moments given, and the project saved at
/// `dir/crate.avtl` (with the cube).
fn harness_with(dir: &Path, scripts: &[(&str, &str, &[Moment])]) -> Harness<'static, State> {
    let library = dir.join("library");
    std::fs::create_dir_all(&library).unwrap();
    let mut harness = fresh();
    let workbench = &mut harness.state_mut().workbench;
    workbench.scripts.library = Some(script::library::Library::new(library.clone()));
    workbench.file.path = Some(dir.join("crate.avtl"));
    for (file, text, moments) in scripts {
        std::fs::write(library.join(file), text).unwrap();
        for moment in *moments {
            workbench.scripts.settings.set_runs_when(
                &ScriptId::Library((*file).to_string()),
                *moment,
                true,
            );
        }
    }
    harness.run_steps(2);
    harness
}

/// Draws frames until nothing waits for scripts and none runs (at most
/// 10 s).
fn settle(harness: &mut Harness<'static, State>) {
    let start = Instant::now();
    harness.run_steps(1);
    loop {
        let workbench = &harness.state().workbench;
        if !workbench.scripts.is_running() && !workbench.scripts.moments.busy() {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(10), "never ended");
        std::thread::sleep(Duration::from_millis(10));
        harness.run_steps(1);
    }
    harness.run_steps(2);
}

/// Asks to save, as Ctrl + S does.
fn save(harness: &mut Harness<'static, State>) {
    harness.key_press_modifiers(Modifiers::COMMAND, Key::S);
    harness.run_steps(1);
}

fn saves(harness: &Harness<'static, State>) -> usize {
    harness
        .state()
        .actions
        .iter()
        .filter(|a| matches!(a, WorkbenchAction::Save { .. }))
        .count()
}

fn names(harness: &Harness<'static, State>) -> Vec<String> {
    let scene = harness.state().workbench.document.scene();
    scene
        .walk()
        .into_iter()
        .map(|(id, _)| scene.object(id).unwrap().name.clone())
        .collect()
}

fn said(harness: &Harness<'static, State>) -> Vec<String> {
    harness
        .state()
        .workbench
        .scripts
        .rows
        .iter()
        .filter_map(|r| match r {
            Row::Line(Line::Said(s)) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

fn note(harness: &Harness<'static, State>) -> String {
    harness.state().workbench.note.clone().unwrap_or_default()
}

const RENAME_ON_SAVE: &str = "-- name: Tidy
-- api: 1
local T = command('Tidy', {})
function T:on_ProjectSaved()
    print('tidy')
    aventail.Scene:shapes()[1]:setName('Tidied')
end
";

#[test]
fn a_save_waits_for_its_scripts_which_land_first_as_one_step() {
    let dir = folder("save");
    let mut harness = harness_with(&dir, &[("tidy.lua", RENAME_ON_SAVE, &[Moment::ProjectSaved])]);
    let steps = harness.state().workbench.document.undo_steps();
    save(&mut harness);
    settle(&mut harness);
    assert_eq!(saves(&harness), 1);
    assert_eq!(names(&harness), ["Tidied"]);
    let workbench = &harness.state().workbench;
    assert_eq!(workbench.document.undo_steps(), steps + 1);
    assert_eq!(workbench.document.undo_label(), Some("Tidy"));
    // Its output, and the status bar once saved.
    assert_eq!(said(&harness), ["tidy"]);
    harness
        .state_mut()
        .workbench
        .saved(dir.join("crate.avtl"));
    let said = note(&harness);
    assert!(said.contains("Saved crate.avtl."), "{said}");
    assert!(
        said.contains("Ran by itself as the project was saved: Tidy."),
        "{said}"
    );
}

#[test]
fn a_script_switched_off_or_turned_off_doesnt_run_and_the_save_goes_at_once() {
    let dir = folder("off");
    let mut harness = harness_with(&dir, &[("tidy.lua", RENAME_ON_SAVE, &[])]);
    save(&mut harness);
    assert_eq!(saves(&harness), 1);
    assert_eq!(names(&harness), ["Cube"]);
    // Switched on, but the script is turned off in the manager.
    let workbench = &mut harness.state_mut().workbench;
    workbench.scripts.settings.set_runs_when(
        &ScriptId::Library("tidy.lua".into()),
        Moment::ProjectSaved,
        true,
    );
    workbench.scripts.settings.off.push("tidy.lua".into());
    save(&mut harness);
    settle(&mut harness);
    assert_eq!(saves(&harness), 2);
    assert_eq!(names(&harness), ["Cube"]);
}

#[test]
fn scripts_at_a_moment_run_once_each_in_name_order() {
    let dir = folder("order");
    let say = |word: &str| {
        format!(
            "-- name: {word}\n-- api: 1\nlocal C = command('C', {{}})\nfunction C:on_ProjectSaved() print('{word}') end\n"
        )
    };
    let (b, a, c) = (say("Bravo"), say("alpha"), say("Charlie"));
    let mut harness = harness_with(
        &dir,
        &[
            ("1.lua", &b, &[Moment::ProjectSaved]),
            ("2.lua", &a, &[Moment::ProjectSaved]),
            ("3.lua", &c, &[Moment::ProjectOpened]),
        ],
    );
    save(&mut harness);
    settle(&mut harness);
    assert_eq!(said(&harness), ["alpha", "Bravo"]);
    assert_eq!(saves(&harness), 1);
}

const STOP_EXPORT: &str = "-- name: Check
-- api: 1
local C = command('Check', {})
function C:on_ExportStarting()
    for _, s in ipairs(aventail.Export:shapes()) do print(s:name()) end
    aventail.Export:cancel('the cube is too plain')
end
";

/// An export of everything to glTF, as the Export window asks for it.
fn export_action(dir: &Path) -> WorkbenchAction {
    WorkbenchAction::Export(Box::new(ExportRequest {
        preset: None,
        format: fileio::export::ExportFormat::Glb,
        options: fileio::export::ExportOptions::default(),
        path: Some(dir.join("crate.glb")),
        suggested_name: "crate.glb".into(),
    }))
}

#[test]
fn an_export_stopped_by_a_script_says_why_and_isnt_made() {
    let dir = folder("stop");
    let mut harness = harness_with(&dir, &[("check.lua", STOP_EXPORT, &[Moment::ExportStarting])]);
    harness.state_mut().workbench.queued.push(export_action(&dir));
    settle(&mut harness);
    let actions = &harness.state().actions;
    assert!(
        !actions.iter().any(|a| matches!(a, WorkbenchAction::Export(_))),
        "{actions:?}"
    );
    assert_eq!(said(&harness), ["Cube"]);
    let said = note(&harness);
    assert!(
        said.contains("The export didn't go ahead: Check stopped it (the cube is too plain)."),
        "{said}"
    );
    // Without the script, the export goes.
    harness.state_mut().workbench.scripts.settings.moments.clear();
    harness.state_mut().workbench.queued.push(export_action(&dir));
    harness.run_steps(2);
    assert!(
        harness
            .state()
            .actions
            .iter()
            .any(|a| matches!(a, WorkbenchAction::Export(_)))
    );
}

#[test]
fn a_kit_export_and_the_save_s_export_into_the_game_project_wait_too() {
    let dir = folder("kit");
    let mut harness = harness_with(&dir, &[("check.lua", STOP_EXPORT, &[Moment::ExportStarting])]);
    let cube = harness.state().workbench.document.scene().children(None)[0];
    harness
        .state_mut()
        .workbench
        .queued
        .push(WorkbenchAction::ExportKit(Box::new(KitRequest {
            pieces: vec![cube],
            format: fileio::export::ExportFormat::Obj,
            options: Default::default(),
            folder: Some(dir.clone()),
        })));
    settle(&mut harness);
    assert!(
        !harness
            .state()
            .actions
            .iter()
            .any(|a| matches!(a, WorkbenchAction::ExportKit(_)))
    );
    // A project exporting into its game project on every save: the save
    // goes on, without the export.
    let target = scene::ExportTarget {
        live: false,
        folder: dir.display().to_string(),
        on_save: true,
    };
    harness
        .state_mut()
        .workbench
        .document
        .execute(scene::Command::SetExportTarget(Some(target)))
        .unwrap();
    save(&mut harness);
    settle(&mut harness);
    assert_eq!(saves(&harness), 1);
    let workbench = &mut harness.state_mut().workbench;
    assert!(workbench.save_export_request(&dir.join("crate.avtl")).is_none());
    // Once only: the next save exports again.
    assert!(workbench.save_export_request(&dir.join("crate.avtl")).is_some());
}

#[test]
fn a_slow_script_is_stopped_and_the_save_goes_on() {
    let dir = folder("slow");
    let slow = "-- name: Slow\nlocal C = command('C', {})\nfunction C:on_ProjectSaved() while true do end end\n";
    let mut harness = harness_with(&dir, &[("slow.lua", slow, &[Moment::ProjectSaved])]);
    let started = Instant::now();
    save(&mut harness);
    settle(&mut harness);
    assert!(started.elapsed() >= script::MOMENT_LIMIT);
    assert_eq!(saves(&harness), 1);
    let rows = &harness.state().workbench.scripts.rows;
    assert!(
        rows.iter().any(|r| matches!(r, Row::Ended(words, false) if words.contains("after 2 seconds"))),
        "{rows:?}"
    );
    // Its own limit is back for the next run.
    let workbench = &harness.state().workbench;
    assert_eq!(workbench.scripts.limits, workbench.scripts.settings.limits());
}

#[test]
fn opening_a_project_runs_its_moments_scripts_trusted_project_ones_too() {
    let dir = folder("open");
    let opened = "-- name: Hello\n-- api: 1\nlocal C = command('C', {})\nfunction C:on_ProjectOpened() print('opened') end\n";
    let mut harness = harness_with(&dir, &[("hello.lua", opened, &[Moment::ProjectOpened])]);
    let theirs = "-- name: Theirs\n-- api: 1\nlocal C = command('C', {})\nfunction C:on_ProjectOpened() print('theirs') end\n";
    let document = Document::new().with_scripts(vec![ProjectScript {
        name: "theirs.lua".into(),
        text: theirs.into(),
    }]);
    let fingerprint = script::fingerprint(theirs);
    harness.state_mut().workbench.scripts.settings.set_runs_when(
        &ScriptId::Project(fingerprint.clone()),
        Moment::ProjectOpened,
        true,
    );
    harness
        .state_mut()
        .workbench
        .opened(document.clone(), dir.join("crate.avtl"));
    settle(&mut harness);
    // The project's own script isn't trusted, so it doesn't run.
    assert_eq!(said(&harness), ["opened"]);
    assert!(note(&harness).contains("Ran by itself as the project opened: Hello."));
    harness
        .state_mut()
        .workbench
        .scripts
        .settings
        .trust(&fingerprint);
    harness
        .state_mut()
        .workbench
        .opened(document, dir.join("crate.avtl"));
    settle(&mut harness);
    assert_eq!(said(&harness), ["opened", "opened", "theirs"]);
}

#[test]
fn a_script_still_asking_to_reach_files_is_left_out_with_a_note() {
    let dir = folder("asks");
    let asks = "-- name: Writer\n-- asks: read and write the project folder\nlocal C = command('C', {})\nfunction C:on_ProjectSaved() aventail.Project:writeText('a.txt', 'x') end\n";
    let mut harness = harness_with(&dir, &[("writer.lua", asks, &[Moment::ProjectSaved])]);
    save(&mut harness);
    settle(&mut harness);
    assert_eq!(saves(&harness), 1);
    assert!(!dir.join("a.txt").exists());
    let rows = &harness.state().workbench.scripts.rows;
    assert!(
        rows.iter().any(|r| matches!(r, Row::Line(Line::Note(n)) if n.starts_with("Writer didn't run as the project was saved"))),
        "{rows:?}"
    );
    // Allowed, it writes as the project is saved.
    let id = ScriptId::Library("writer.lua".into());
    harness.state_mut().workbench.scripts.settings.approve(
        &id,
        &[script::Ask::WriteProjectFolder],
        None,
    );
    save(&mut harness);
    settle(&mut harness);
    assert!(dir.join("a.txt").is_file());
}

#[test]
fn opening_another_project_forgets_the_save_waiting() {
    let dir = folder("forget");
    let slow = "-- name: Slow\nlocal C = command('C', {})\nfunction C:on_ProjectSaved() while true do end end\n";
    let mut harness = harness_with(&dir, &[("slow.lua", slow, &[Moment::ProjectSaved])]);
    save(&mut harness);
    // Held back while the script runs.
    assert!(harness.state().workbench.scripts.moments.busy());
    assert_eq!(saves(&harness), 0);
    harness
        .state_mut()
        .workbench
        .opened(Document::new(), dir.join("other.avtl"));
    settle(&mut harness);
    // Nothing of the old project's save is handed on.
    assert_eq!(saves(&harness), 0);
}

#[test]
fn the_script_manager_switches_a_script_on_for_the_moments_it_reacts_to() {
    let dir = folder("manager");
    let mut harness = harness_with(&dir, &[("tidy.lua", RENAME_ON_SAVE, &[])]);
    let workbench = &mut harness.state_mut().workbench;
    let names = workbench.scripts.names();
    let limits = workbench.scripts.settings.limits();
    let library = workbench.scripts.library.clone();
    workbench
        .scripts
        .registry
        .refresh(library.as_ref(), &names, limits, Instant::now());
    workbench.scripts.registry.wait();
    workbench.scripts.manager.open = true;
    // The window settles on its size over a few frames.
    harness.run_steps(20);
    harness.get_by_label_contains("Runs by itself when");
    // Only the moment it reacts to has a switch.
    assert!(harness.query_by_label("a project opens").is_none());
    harness.get_by_label("a project is saved").click();
    harness.run_steps(4);
    let settings = &harness.state().workbench.scripts.settings;
    assert!(settings.runs_when(&ScriptId::Library("tidy.lua".into()), Moment::ProjectSaved));
}

#[test]
fn a_projects_scripts_can_be_turned_off_again() {
    let dir = folder("turn-off");
    let mut harness = harness_with(&dir, &[]);
    let text = "-- name: Theirs\nprint(1)\n";
    let document = Document::new().with_scripts(vec![ProjectScript {
        name: "theirs.lua".into(),
        text: text.into(),
    }]);
    harness.state_mut().workbench.scripts.settings.trust(&script::fingerprint(text));
    harness
        .state_mut()
        .workbench
        .opened(document, dir.join("crate.avtl"));
    harness.run_steps(2);
    harness.state_mut().workbench.scripts.manager.open = true;
    harness.run_steps(20);
    harness
        .get_by_label(crate::text::tr(super::script_manager::TURN_THEM_OFF))
        .click();
    harness.run_steps(4);
    let workbench = &harness.state().workbench;
    assert!(!workbench.scripts.settings.is_trusted(&script::fingerprint(text)));
    assert_eq!(workbench.scripts.project.untrusted(&workbench.scripts.settings), 1);
}

#[test]
fn moment_switches_follow_renames_go_with_removals_and_survive_a_restart() {
    let mut settings = crate::ScriptSettings::default();
    let id = ScriptId::Library("a.lua".into());
    settings.set_runs_when(&id, Moment::ProjectSaved, true);
    settings.set_runs_when(&id, Moment::ExportStarting, true);
    settings.set_runs_when(&id, Moment::ProjectSaved, false);
    let text = serde_json::to_string(&settings).unwrap();
    let back: crate::ScriptSettings = serde_json::from_str(&text).unwrap();
    assert_eq!(back.checked(), settings);
    settings.renamed("a.lua", "b.lua");
    let renamed = ScriptId::Library("b.lua".into());
    assert!(settings.runs_when(&renamed, Moment::ExportStarting));
    assert!(!settings.runs_when(&id, Moment::ExportStarting));
    settings.removed("b.lua");
    assert!(settings.moments.is_empty());
    // Switching the last one off leaves nothing behind; a damaged file
    // with twins and empty lists is put right.
    let mut damaged = crate::ScriptSettings::default();
    damaged.moments = vec![
        (renamed.clone(), vec![crate::script_commands::SavedMoment::Saved; 9]),
        (renamed.clone(), vec![crate::script_commands::SavedMoment::Opened]),
        (id, vec![]),
    ];
    let checked = damaged.checked();
    assert_eq!(
        checked.moments,
        [(renamed, vec![crate::script_commands::SavedMoment::Saved])]
    );
}

#[test]
fn an_example_script_is_added_to_the_library_and_opened() {
    let dir = folder("examples");
    let mut harness = harness_with(&dir, &[]);
    main_menu(&harness, "Scripts").click();
    harness.run();
    harness
        .get_by_label_contains(crate::text::tr(super::scripting::EXAMPLES_ENTRY))
        .hover();
    harness.run_steps(8);
    harness.get_by_label("Gear").click();
    harness.run_steps(2);
    assert!(dir.join("library/Gear.lsc").is_file());
    let workbench = &harness.state().workbench;
    let editing = workbench.scripts.editor.as_ref().map(|e| e.path.clone());
    assert_eq!(editing, Some(dir.join("library/Gear.lsc")));
    // A second one is numbered, never over yours.
    let gear = script::examples(&workbench.scripts.names())
        .into_iter()
        .find(|e| e.name == "Gear.lsc")
        .unwrap();
    harness.state_mut().workbench.add_example(&gear);
    assert!(dir.join("library/Gear 2.lsc").is_file());
}

#[test]
fn scripts_see_sculpt_mode() {
    let mut harness = fresh();
    harness.state_mut().workbench.mode = crate::palette::Mode::Sculpt;
    assert_eq!(
        harness.state().workbench.script_snapshot().mode,
        script::Mode::Sculpt
    );
    harness.state_mut().workbench.run_script(script::ScriptFile {
        name: "mode.lua".into(),
        text: "print(aventail.Selection:mode())".into(),
    });
    settle(&mut harness);
    assert!(said(&harness).contains(&"Sculpt".to_string()));
}
