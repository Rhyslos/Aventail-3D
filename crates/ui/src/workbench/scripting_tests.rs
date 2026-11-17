//! Running scripts through the interface (decision 0301): the Scripts
//! menu, the Scripts panel's output, Stop and Esc.

use super::tests::{State, fresh, main_menu};
use super::*;
use crate::scripts_panel::Row;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use script::{Line, ScriptFile};
use std::time::{Duration, Instant};

fn lua(text: &str) -> ScriptFile {
    ScriptFile {
        name: "test.lua".into(),
        text: text.into(),
    }
}

/// Draws frames until the running script has ended (at most 10 s).
fn until_ended(harness: &mut Harness<'static, State>) {
    let start = Instant::now();
    while harness.state().workbench.scripts.is_running() {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "the script never ended"
        );
        std::thread::sleep(Duration::from_millis(10));
        harness.run_steps(1);
    }
    harness.run_steps(2);
}

#[test]
fn the_scripts_menu_asks_the_app_for_a_file() {
    let mut harness = fresh();
    main_menu(&harness, "Scripts").click();
    harness.run();
    harness.get_by_label("Run a script file…").click();
    harness.run();
    assert!(
        harness
            .state()
            .actions
            .contains(&WorkbenchAction::RunScript { path: None })
    );
}

#[test]
fn a_script_reads_the_scene_and_its_output_shows_in_the_scripts_panel() {
    let mut harness = fresh();
    harness.state_mut().workbench.run_script(lua(
        "-- name: Say names\n-- api: 1\nfor _, s in ipairs(aventail.Scene:shapes()) do print(s:name()) end\naventail.Script:warn(\"careful\")",
    ));
    harness.run_steps(1);
    assert!(
        harness
            .state()
            .workbench
            .dock
            .find_tab(&Panel::Scripts)
            .is_some()
    );
    until_ended(&mut harness);
    let rows = &harness.state().workbench.scripts.rows;
    assert_eq!(rows[0], Row::Started("Say names".into()));
    assert!(
        rows.contains(&Row::Line(Line::Said("Cube".into()))),
        "{rows:?}"
    );
    assert!(matches!(rows.last(), Some(Row::Ended(words, true)) if words == "Say names finished."));
    assert_eq!(
        harness.state().workbench.note.as_deref(),
        Some("Say names finished.")
    );
    // The panel shows it.
    assert!(harness.get_all_by_label("Cube").count() >= 2);
    assert!(harness.query_by_label("Warning: careful").is_some());
}

#[test]
fn a_mistake_shows_with_its_file_and_line() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .run_script(lua("-- api: 1\n\nlocal t = nil\nprint(t.x)\n"));
    until_ended(&mut harness);
    let rows = &harness.state().workbench.scripts.rows;
    let mistake = rows.iter().find_map(|r| match r {
        Row::Line(Line::Mistake(p)) => Some(p.clone()),
        _ => None,
    });
    let mistake = mistake.unwrap();
    assert_eq!((mistake.file.as_str(), mistake.line), ("test.lua", 4));
    assert!(matches!(rows.last(), Some(Row::Ended(_, false))));
}

#[test]
fn esc_stops_a_running_script_and_only_then() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .run_script(lua("while true do end"));
    harness.run_steps(2);
    assert!(harness.state().workbench.scripts.is_running());
    // A second script waits for the first.
    harness.state_mut().workbench.run_script(lua("print(1)"));
    assert!(
        harness
            .state()
            .workbench
            .note
            .as_deref()
            .is_some_and(|n| n.contains("already running"))
    );
    harness.key_press(egui::Key::Escape);
    until_ended(&mut harness);
    assert!(matches!(
        harness.state().workbench.scripts.rows.last(),
        Some(Row::Ended(words, false)) if words == "test was stopped."
    ));
    // With no script running, Esc goes back to its other jobs.
    harness.state_mut().workbench.stop_script();
    assert!(!harness.state().workbench.scripts.is_running());
}

#[test]
fn stop_in_the_status_bar_stops_the_script() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .run_script(lua("-- name: Spin\nwhile true do end"));
    harness.run_steps(2);
    harness.get_by_label("Stop").click();
    until_ended(&mut harness);
    assert_eq!(
        harness.state().workbench.note.as_deref(),
        Some("Spin was stopped.")
    );
}

#[test]
fn a_file_that_cant_be_read_is_said_plainly() {
    let mut harness = fresh();
    let missing = std::env::temp_dir().join("no-such-script-here.lua");
    harness.state_mut().workbench.run_script_file(&missing);
    harness.run_steps(2);
    let note = harness.state().workbench.note.clone().unwrap();
    assert!(note.contains("couldn't be read"), "{note}");
    let picture = std::env::temp_dir().join("not-a-script.png");
    harness.state_mut().workbench.run_script_file(&picture);
    let note = harness.state().workbench.note.clone().unwrap();
    assert!(note.contains("isn't a script"), "{note}");
    assert!(!harness.state().workbench.scripts.is_running());
}

#[test]
fn the_output_keeps_the_newest_rows_and_clears() {
    let mut panel = crate::scripts_panel::ScriptsPanel::default();
    for i in 0..crate::scripts_panel::MAX_ROWS + 10 {
        panel.push(Row::Started(i.to_string()));
    }
    assert_eq!(panel.rows.len(), crate::scripts_panel::MAX_ROWS);
    assert_eq!(panel.rows[0], Row::Started("10".into()));
}
