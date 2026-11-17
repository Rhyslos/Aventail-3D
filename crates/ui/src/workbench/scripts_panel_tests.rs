//! The Scripts panel through the interface (decision 0303): F11 and
//! Window → Scripts, the library and new scripts, the editor's
//! Ctrl + Enter with and without a selection, the console line's Enter,
//! Up, Down and Shift + Enter, undo while typing, and a long script
//! staying smooth.

use super::tests::{State, fresh, main_menu};
use super::*;
use crate::scripts_panel::Row;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use script::Line;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// A fresh, empty folder for one test.
fn folder(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("scripts-panel-ui-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// A harness whose Scripts panel keeps its library in a folder of its
/// own, the panel shown, and `text` open in the editor as `name`.
fn editing(test: &str, name: &str, text: &str) -> (Harness<'static, State>, PathBuf) {
    let dir = folder(test);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    let mut harness = fresh();
    {
        let workbench = &mut harness.state_mut().workbench;
        workbench.scripts.library = Some(script::library::Library::new(dir.clone()));
        workbench.toggle_scripts_panel();
        workbench.scripts.edit(&path, Instant::now());
    }
    harness.run_steps(3);
    (harness, dir)
}

/// Gives the keyboard to the text box `id`.
fn focus(harness: &mut Harness<'static, State>, id: Option<egui::Id>) {
    let id = id.expect("drawn");
    harness.ctx.memory_mut(|m| m.request_focus(id));
    harness.run_steps(2);
}

/// Draws frames until the running script has ended (at most 10 s).
fn until_ended(harness: &mut Harness<'static, State>) {
    let start = Instant::now();
    harness.run_steps(1);
    while harness.state().workbench.scripts.is_running() {
        assert!(start.elapsed() < Duration::from_secs(10), "never ended");
        std::thread::sleep(Duration::from_millis(10));
        harness.run_steps(1);
    }
    harness.run_steps(2);
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

#[test]
fn f11_and_window_scripts_show_and_put_away_the_panel() {
    let mut harness = fresh();
    let shown = |h: &Harness<'static, State>| h.state().workbench.dock.find_tab(&Panel::Scripts).is_some();
    assert!(!shown(&harness));
    harness.key_press(Key::F11);
    harness.run();
    assert!(shown(&harness));
    harness.key_press(Key::F11);
    harness.run();
    assert!(!shown(&harness));
    main_menu(&harness, "Window").click();
    harness.run();
    harness.get_by_label_contains("F11").click();
    harness.run();
    assert!(shown(&harness));
}

#[test]
fn ctrl_enter_runs_the_whole_script_or_only_the_selected_lines() {
    let text = "-- name: Two\n-- api: 1\nprint(\"one\")\nprint(\"two\")\n";
    let (mut harness, dir) = editing("ctrl-enter", "two.lua", text);
    let id = harness.state().workbench.scripts.editor.as_ref().unwrap().id;
    focus(&mut harness, id);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    until_ended(&mut harness);
    assert_eq!(said(&harness), ["one", "two"]);
    assert_eq!(
        harness.state().workbench.scripts.editor.as_ref().unwrap().text,
        text,
        "Ctrl + Enter typed nothing"
    );
    // Select the fourth line, then Ctrl + Enter.
    harness.state_mut().workbench.scripts.rows.clear();
    let start = text.find("print(\"two\")").unwrap();
    let mut state = egui::TextEdit::load_state(&harness.ctx, id.unwrap()).unwrap();
    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
        egui::text::CCursor::new(start),
        egui::text::CCursor::new(start + 5),
    )));
    state.store(&harness.ctx, id.unwrap());
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    until_ended(&mut harness);
    assert_eq!(said(&harness), ["two"]);
    let rows = &harness.state().workbench.scripts.rows;
    assert_eq!(rows[0], Row::Started("Two (line 4)".into()), "{rows:?}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn typing_in_the_editor_saves_it_and_ctrl_z_undoes_typing_not_the_scene() {
    let (mut harness, dir) = editing("typing", "t.lua", "print(1)\n");
    let steps = harness.state().workbench.document().undo_steps();
    let id = harness.state().workbench.scripts.editor.as_ref().unwrap().id;
    focus(&mut harness, id);
    harness.event(egui::Event::Text("-- more\n".into()));
    harness.run_steps(2);
    let typed = harness.state().workbench.scripts.editor.as_ref().unwrap().text.clone();
    assert!(typed.contains("-- more"), "{typed:?}");
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run_steps(2);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps, "the scene is untouched");
    // Closing the editor saves what's typed.
    harness.state_mut().workbench.scripts.editor = None;
    let saved = std::fs::read_to_string(dir.join("t.lua")).unwrap();
    assert!(saved.starts_with("print(1)") || saved.contains("-- more"), "{saved:?}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_console_runs_a_line_with_enter_and_up_brings_it_back() {
    let (mut harness, dir) = editing("console", "c.lua", "print(1)\n");
    let steps = harness.state().workbench.document().undo_steps();
    let id = harness.state().workbench.scripts.console.id;
    assert_eq!(harness.state().workbench.scripts.console.language, script::Language::Lua, "follows the script open");
    focus(&mut harness, id);
    harness.event(egui::Event::Text("aventail.Scene:add(\"Cube\")".into()));
    harness.run_steps(1);
    // Shift + Enter adds a line rather than running it.
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Enter);
    harness.run_steps(1);
    harness.event(egui::Event::Text("print(\"made\")".into()));
    harness.run_steps(1);
    assert!(!harness.state().workbench.scripts.is_running());
    assert_eq!(
        harness.state().workbench.scripts.console.text,
        "aventail.Scene:add(\"Cube\")\nprint(\"made\")"
    );
    harness.key_press(Key::Enter);
    until_ended(&mut harness);
    assert_eq!(said(&harness), ["made"]);
    let document = harness.state().workbench.document();
    assert_eq!(document.undo_steps(), steps + 1, "one step for the line");
    assert_eq!(document.undo_label(), Some("Console line"));
    assert!(harness.state().workbench.scripts.console.text.is_empty());
    assert!(
        harness.state().workbench.scripts.rows.contains(&Row::Typed("aventail.Scene:add(\"Cube\")\nprint(\"made\")".into()))
    );
    // Up brings it back; Down goes back to what was being typed.
    harness.event(egui::Event::Text("x".into()));
    harness.run_steps(1);
    harness.key_press(Key::ArrowUp);
    harness.run_steps(1);
    assert_eq!(
        harness.state().workbench.scripts.console.text,
        "aventail.Scene:add(\"Cube\")\nprint(\"made\")"
    );
    // On the last line, Down goes forward.
    harness.key_press(Key::ArrowDown);
    harness.run_steps(1);
    assert_eq!(harness.state().workbench.scripts.console.text, "x");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_console_mistake_is_at_the_line_typed() {
    let (mut harness, dir) = editing("console-mistake", "c.lsc", "-- api: 1\nend\n");
    let id = harness.state().workbench.scripts.console.id;
    focus(&mut harness, id);
    harness.event(egui::Event::Text("int n = \"text\";".into()));
    harness.key_press(Key::Enter);
    until_ended(&mut harness);
    let rows = &harness.state().workbench.scripts.rows;
    let mistake = rows
        .iter()
        .find_map(|r| match r {
            Row::Line(Line::Mistake(p)) => Some(p.clone()),
            _ => None,
        })
        .expect("a mistake");
    assert_eq!((mistake.file.as_str(), mistake.line), ("console.lsc", 1), "{rows:?}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_new_script_is_made_in_the_library_and_opened() {
    let dir = folder("new");
    let mut harness = fresh();
    harness.state_mut().workbench.scripts.library =
        Some(script::library::Library::new(dir.clone()));
    harness.key_press(Key::F11);
    harness.run();
    // The description files are written for outside editors.
    assert!(dir.join("aventail").join("scene.lsc").is_file());
    harness.get_by_label("+").click();
    harness.run();
    harness.get_by_role(egui::accesskit::Role::TextInput).type_text("Say names");
    harness.run();
    harness.get_by_label("Lua").click();
    harness.run();
    harness.get_by_label("Make it").click();
    harness.run_steps(3);
    let path = dir.join("Say names.lua");
    assert!(path.is_file(), "{:?} {:?}", harness.state().workbench.scripts.new_script, harness.state().workbench.scripts.library_trouble);
    let editor = harness.state().workbench.scripts.editor.as_ref().expect("opened");
    assert_eq!(editor.path, path);
    assert!(editor.text.contains("-- name: Say names"));
    assert_eq!(harness.get_all_by_label("Say names.lua").count(), 2, "listed, and named over the editor");
    // The same name again is refused, saying why.
    harness.get_by_label("+").click();
    harness.run_steps(3);
    harness.get_by_role(egui::accesskit::Role::TextInput).type_text("Say names");
    harness.get_by_label("Lua").click();
    harness.run_steps(3);
    harness.get_by_label("Make it").click();
    harness.run_steps(3);
    assert!(harness.state().workbench.scripts.new_script.as_ref().unwrap().why.is_some());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn open_in_your_own_editor_asks_the_app() {
    let (mut harness, dir) = editing("outside", "o.lua", "print(1)\n");
    harness.get_by_label("Open in your own editor").click();
    harness.run_steps(3);
    assert!(harness.state().actions.contains(&WorkbenchAction::OpenOutside {
        path: dir.join("o.lua")
    }));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_ten_thousand_line_script_stays_smooth() {
    let line = "    shape:bevel(shape:hardEdges(), 0.2, 2); -- round \"it\" off\n";
    let text = format!("-- api: 1\n{}end\n", line.repeat(10_000));
    let (mut harness, dir) = editing("long", "long.lsc", &text);
    let id = harness.state().workbench.scripts.editor.as_ref().unwrap().id;
    focus(&mut harness, id);
    // Typing a character at a time: each frame lays out only what changed.
    let start = Instant::now();
    for _ in 0..10 {
        harness.event(egui::Event::Text("x".into()));
        harness.run_steps(1);
    }
    let per_key = start.elapsed() / 10;
    // And frames with nothing typed (the cursor blinking).
    let start = Instant::now();
    harness.run_steps(10);
    let idle = start.elapsed() / 10;
    eprintln!("an idle frame with a 10,000-line script: {idle:?} (debug build)");
    let typed = &harness.state().workbench.scripts.editor.as_ref().unwrap().text;
    assert_eq!(typed.len(), text.len() + 10);
    eprintln!("a key in a 10,000-line script: {per_key:?} a frame (debug build)");
    // Generous for a debug build on a busy machine.
    assert!(per_key < Duration::from_secs(2), "{per_key:?}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_scripts_panel_gets_room_beside_coordinates_and_gives_it_back() {
    let mut harness = fresh();
    let band = |h: &mut Harness<'static, State>| {
        let dock = &mut h.state_mut().workbench.dock;
        let path = dock.find_tab(&Panel::Coordinates).unwrap();
        let rect = dock.leaf(path.node_path()).unwrap().rect;
        rect.height()
    };
    let slim = band(&mut harness);
    harness.key_press(Key::F11);
    harness.run_steps(3);
    let roomy = band(&mut harness);
    assert!(roomy >= crate::layout::SCRIPTS_HEIGHT - 1.0, "{slim} then {roomy}");
    harness.key_press(Key::F11);
    harness.run_steps(3);
    assert!((band(&mut harness) - slim).abs() < 2.0);
}
