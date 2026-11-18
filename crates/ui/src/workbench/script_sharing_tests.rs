//! Keeping, sharing and permissions through the interface (decision
//! 0307): a project's scripts off until turned on, the trust bar, Keep
//! with this project as one undo step, the access window before a script
//! reaches files, Install a script… and Export.

use super::tests::{State, fresh, main_menu};
use super::*;
use crate::script_sharing::{ScriptId, Then};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use scene::{Command, Document, ProjectScript};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// A fresh folder for one test.
fn folder(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("script-sharing-ui-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A harness with an empty library in `dir/library`.
fn harness_in(dir: &Path) -> Harness<'static, State> {
    let library = dir.join("library");
    std::fs::create_dir_all(&library).unwrap();
    let mut harness = fresh();
    harness.state_mut().workbench.scripts.library = Some(script::library::Library::new(library));
    harness.run_steps(2);
    harness
}

/// A script making a cube, as a command.
const MAKE_CUBE: &str = "-- name: Make a cube
-- api: 1
local C = command(\"Cube\", { size = 1 })
function C:run()
    aventail.Scene:add(aventail.Primitive.Cube)
end
";

/// A script writing a note into the project's folder.
const WRITE_NOTE: &str = "-- name: Write a note
-- asks: read and write the project folder
aventail.Project:writeText(\"notes/note.txt\", \"hello\")
";

/// Opens a project saved at `dir/crate.avtl` holding `scripts`.
fn open_with(harness: &mut Harness<'static, State>, dir: &Path, scripts: &[(&str, &str)]) {
    let document = Document::new().with_scripts(
        scripts
            .iter()
            .map(|(name, text)| ProjectScript {
                name: (*name).into(),
                text: (*text).into(),
            })
            .collect(),
    );
    harness
        .state_mut()
        .workbench
        .opened(document, dir.join("crate.avtl"));
    harness.run_steps(2);
}

/// Draws frames until no script runs and the project's commands are
/// read (at most 10 s).
fn settle(harness: &mut Harness<'static, State>) {
    let start = Instant::now();
    harness.run_steps(1);
    loop {
        harness.state_mut().workbench.scripts.project.wait();
        harness.run_steps(1);
        if !harness.state().workbench.scripts.is_running() {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(10), "never ended");
        std::thread::sleep(Duration::from_millis(10));
    }
    harness.run_steps(2);
}

fn shapes(harness: &Harness<'static, State>) -> usize {
    harness
        .state()
        .workbench
        .document
        .scene()
        .children(None)
        .len()
}

#[test]
fn a_projects_scripts_are_off_until_turned_on_from_the_bar() {
    let dir = folder("bar");
    let mut harness = harness_in(&dir);
    open_with(&mut harness, &dir, &[("cube.lua", MAKE_CUBE)]);
    settle(&mut harness);
    harness.get_by_label("This project has a script. It won't run until you turn it on.");
    // Not read, so nothing of it ran and it has no commands.
    let workbench = &harness.state().workbench;
    assert!(workbench.scripts.project.entries[0].described.is_none());
    assert!(workbench.scripts.commands().is_empty());
    harness
        .get_by_label(crate::text::tr(super::script_sharing::TURN_THEM_ON))
        .click();
    settle(&mut harness);
    let commands = harness.state().workbench.scripts.commands();
    assert_eq!(commands.len(), 1);
    assert!(commands[0].key.project);
    // Its command runs from the Scripts menu.
    main_menu(&harness, "Scripts").click();
    harness.run();
    harness.get_by_label("Make a cube").click();
    settle(&mut harness);
    assert_eq!(shapes(&harness), 1);
    // Trust is kept on this computer: the same script in another project
    // is on.
    assert!(harness.query_by_label_contains("won't run until").is_none());
}

#[test]
fn not_now_puts_the_bar_away_and_running_an_untrusted_script_is_refused() {
    let dir = folder("not-now");
    let mut harness = harness_in(&dir);
    open_with(
        &mut harness,
        &dir,
        &[("cube.lua", MAKE_CUBE), ("b.lua", "print(1)")],
    );
    harness.get_by_label("Not now").click();
    harness.run_steps(2);
    assert!(harness.query_by_label_contains("won't run until").is_none());
    harness
        .state_mut()
        .workbench
        .scripts
        .run_project_script("cube.lua");
    settle(&mut harness);
    assert_eq!(shapes(&harness), 0);
    assert!(!harness.state().workbench.scripts.is_running());
}

#[test]
fn keeping_a_library_script_with_the_project_is_one_undo_step_and_trusted() {
    let dir = folder("keep");
    let mut harness = harness_in(&dir);
    let path = dir.join("library").join("cube.lua");
    std::fs::write(&path, MAKE_CUBE).unwrap();
    let steps = harness.state().workbench.document.undo_steps();
    harness.state_mut().workbench.keep_with_project(&path);
    settle(&mut harness);
    let workbench = &harness.state().workbench;
    assert_eq!(workbench.document.undo_steps(), steps + 1);
    assert_eq!(workbench.document.scene().scripts()[0].name, "cube.lua");
    assert_eq!(
        workbench
            .scripts
            .project
            .untrusted(&workbench.scripts.settings),
        0
    );
    assert!(harness.query_by_label_contains("won't run until").is_none());
    // Kept again after a change, it's updated, not doubled.
    std::fs::write(&path, format!("{MAKE_CUBE}\n-- changed\n")).unwrap();
    harness.state_mut().workbench.keep_with_project(&path);
    let scripts = harness
        .state()
        .workbench
        .document
        .scene()
        .scripts()
        .to_vec();
    assert_eq!(scripts.len(), 1);
    assert!(scripts[0].text.contains("changed"));
    // Taken out, and Ctrl + Z puts it back.
    harness
        .state_mut()
        .workbench
        .take_out_of_project("cube.lua");
    assert!(
        harness
            .state()
            .workbench
            .document
            .scene()
            .scripts()
            .is_empty()
    );
    harness.state_mut().workbench.document.undo();
    assert_eq!(
        harness.state().workbench.document.scene().scripts().len(),
        1
    );
}

#[test]
fn a_script_asks_before_writing_and_writes_once_allowed() {
    let dir = folder("ask");
    let mut harness = harness_in(&dir);
    open_with(&mut harness, &dir, &[]);
    let library = dir.join("library").join("note.lua");
    std::fs::write(&library, WRITE_NOTE).unwrap();
    let file = script::ScriptFile::read(&library).unwrap();
    harness.state_mut().workbench.start_run(
        file.clone(),
        "Write a note".into(),
        None,
        None,
        crate::script_sharing::Source::Library("note.lua".into()),
    );
    harness.run_steps(3);
    assert!(harness.state().workbench.scripts.asking_access.is_some());
    assert!(!harness.state().workbench.scripts.is_running());
    harness.get_by_label("• read and write the project folder");
    harness
        .get_by_label(crate::text::tr(super::script_sharing::ALLOW_AND_RUN))
        .click();
    settle(&mut harness);
    assert_eq!(
        std::fs::read_to_string(dir.join("notes").join("note.txt")).unwrap(),
        "hello"
    );
    // Allowed once: the next run doesn't ask.
    std::fs::remove_dir_all(dir.join("notes")).unwrap();
    harness.state_mut().workbench.start_run(
        file,
        "Write a note".into(),
        None,
        None,
        crate::script_sharing::Source::Library("note.lua".into()),
    );
    settle(&mut harness);
    assert!(harness.state().workbench.scripts.asking_access.is_none());
    assert!(dir.join("notes").join("note.txt").exists());
}

#[test]
fn dont_run_it_writes_nothing_and_a_bigger_ask_asks_again() {
    let dir = folder("bigger");
    let mut harness = harness_in(&dir);
    open_with(&mut harness, &dir, &[]);
    let id = ScriptId::Library("note.lua".into());
    harness.state_mut().workbench.scripts.settings.approve(
        &id,
        &[script::Ask::ReadProjectFolder],
        None,
    );
    let file = script::ScriptFile {
        name: "note.lua".into(),
        text: WRITE_NOTE.into(),
    };
    harness.state_mut().workbench.start_run(
        file,
        "Write a note".into(),
        None,
        None,
        crate::script_sharing::Source::Library("note.lua".into()),
    );
    harness.run_steps(3);
    let ask = harness
        .state()
        .workbench
        .scripts
        .asking_access
        .clone()
        .unwrap();
    assert!(matches!(ask.then, Then::Run { .. }));
    harness.get_by_label("Don't run it").click();
    settle(&mut harness);
    assert!(harness.state().workbench.scripts.asking_access.is_none());
    assert!(!dir.join("notes").exists());
}

#[test]
fn the_console_line_reaches_no_files() {
    let dir = folder("console");
    let mut harness = harness_in(&dir);
    open_with(&mut harness, &dir, &[]);
    harness.state_mut().workbench.run_script_as(
        script::ScriptFile {
            name: "console.lua".into(),
            text: WRITE_NOTE.into(),
        },
        "Console line".into(),
        None,
    );
    settle(&mut harness);
    assert!(harness.state().workbench.scripts.asking_access.is_none());
    assert!(!dir.join("notes").exists());
}

#[test]
fn installing_shows_the_script_first_and_adds_it_to_the_library() {
    let dir = folder("install");
    let mut harness = harness_in(&dir);
    let given = dir.join("Shared cube.lua");
    std::fs::write(&given, MAKE_CUBE).unwrap();
    harness.state_mut().workbench.install_script_file(&given);
    harness.run_steps(3);
    harness.get_by_label("It asks for nothing outside the project.");
    harness
        .get_by_label(crate::text::tr(super::script_sharing::INSTALL))
        .click();
    harness.run_steps(3);
    assert!(dir.join("library").join("Shared cube.lua").exists());
    // Installing the same again never writes over it.
    harness.state_mut().workbench.install_script_file(&given);
    harness.run_steps(2);
    harness
        .get_by_label(crate::text::tr(super::script_sharing::INSTALL))
        .click();
    harness.run_steps(3);
    assert!(dir.join("library").join("Shared cube 2.lua").exists());
    // Not a script: said, nothing shown.
    let notes = dir.join("notes.txt");
    std::fs::write(&notes, "x").unwrap();
    harness.state_mut().workbench.install_script_file(&notes);
    assert!(harness.state().workbench.scripts.installing.is_none());
}

#[test]
fn scripts_export_from_the_library_and_the_project() {
    let dir = folder("export");
    let mut harness = harness_in(&dir);
    std::fs::write(dir.join("library").join("cube.lua"), MAKE_CUBE).unwrap();
    open_with(&mut harness, &dir, &[("kept.lsc", "end\n")]);
    let workbench = &harness.state().workbench;
    assert_eq!(
        workbench.script_to_export("cube.lua", false).unwrap().text,
        MAKE_CUBE
    );
    assert_eq!(
        workbench.script_to_export("kept.lsc", true).unwrap().text,
        "end\n"
    );
    assert!(workbench.script_to_export("missing.lua", false).is_none());
    harness.state_mut().workbench.queued.clear();
    harness
        .state_mut()
        .workbench
        .scripts
        .requests
        .push(crate::scripts_panel::Request::Export {
            file: "kept.lsc".into(),
            project: true,
        });
    harness.run_steps(1);
}

#[test]
fn copying_a_projects_script_into_the_library_never_writes_over_yours() {
    let dir = folder("copy");
    let mut harness = harness_in(&dir);
    std::fs::write(dir.join("library").join("cube.lua"), "-- mine\n").unwrap();
    open_with(&mut harness, &dir, &[("cube.lua", MAKE_CUBE)]);
    harness.state_mut().workbench.copy_into_library("cube.lua");
    assert_eq!(
        std::fs::read_to_string(dir.join("library").join("cube.lua")).unwrap(),
        "-- mine\n"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("library").join("cube 2.lua")).unwrap(),
        MAKE_CUBE
    );
}

#[test]
fn the_scripts_menu_has_install_and_export() {
    let dir = folder("menu");
    let mut harness = harness_in(&dir);
    main_menu(&harness, "Scripts").click();
    harness.run();
    harness
        .get_by_label(crate::text::tr(super::script_sharing::INSTALL_ENTRY))
        .click();
    harness.run_steps(1);
    assert!(
        harness
            .state()
            .workbench
            .queued
            .iter()
            .chain(harness.state().actions.iter())
            .any(|a| matches!(a, WorkbenchAction::InstallScript { path: None }))
    );
}

#[test]
fn a_kept_script_can_be_looked_at_but_not_run_while_off() {
    let dir = folder("look");
    let mut harness = harness_in(&dir);
    open_with(&mut harness, &dir, &[("cube.lua", MAKE_CUBE)]);
    harness
        .get_by_label(crate::text::tr(super::script_sharing::LOOK_AT_THEM))
        .click();
    harness.run_steps(20);
    let viewing = harness
        .state()
        .workbench
        .scripts
        .viewing
        .as_ref()
        .map(|v| v.file.name.clone());
    assert_eq!(viewing.as_deref(), Some("cube.lua"));
    assert!(!harness.state().workbench.scripts.is_running());
    // Taking it out of the project stops showing it.
    harness
        .state_mut()
        .workbench
        .document
        .execute(Command::SetScripts(Vec::new()))
        .unwrap();
    harness.run_steps(2);
    assert!(harness.state().workbench.scripts.viewing.is_none());
}
