//! Scripts as commands through the interface (decision 0305): run from
//! the Scripts menu, command search, the command palette and a key of
//! their own; settings in Attributes running the command again as one
//! undo step; Ctrl + wheel for a count; keys refused when taken; the
//! script manager's rename and remove; what's kept shared by every tab.

use super::tests::{State, fresh, main_menu};
use super::*;
use crate::script_commands::{CommandKey, KeyAsk, SavedKey};
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use script::SettingValue;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// A command making `n` cubes, which fails when `n` is 2.
const CUBES: &str = "-- name: Make cubes
-- about: Makes some cubes.
-- api: 1
-- count: n
local C = command(\"Cubes\", { n = 1, size = 0.5 })
function C:run()
    if self.n == 2 then error(\"two is not allowed\") end
    for i = 1, self.n do
        aventail.Scene:add(aventail.Primitive.Cube)
    end
end
";

/// A harness whose library holds `files` (name, text), read.
fn with_library(test: &str, files: &[(&str, &str)]) -> (Harness<'static, State>, PathBuf) {
    let dir =
        std::env::temp_dir().join(format!("script-commands-ui-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (name, text) in files {
        std::fs::write(dir.join(name), text).unwrap();
    }
    let mut harness = fresh();
    harness.state_mut().workbench.scripts.library =
        Some(script::library::Library::new(dir.clone()));
    read_library(&mut harness);
    (harness, dir)
}

/// Draws frames until the library's commands are read.
fn read_library(harness: &mut Harness<'static, State>) {
    harness.run_steps(1);
    harness.state_mut().workbench.scripts.registry.wait();
    harness.run_steps(2);
}

/// Draws frames until no script runs (at most 10 s).
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

fn shapes(harness: &Harness<'static, State>) -> usize {
    harness
        .state()
        .workbench
        .document
        .scene()
        .children(None)
        .len()
}

fn cubes_key() -> CommandKey {
    CommandKey {
        file: "cubes.lua".into(),
        command: Some("Cubes".into()),
    }
}

fn n_of(harness: &Harness<'static, State>) -> Option<SettingValue> {
    let view = harness.state().workbench.script_view()?;
    view.values
        .into_iter()
        .find(|(name, _)| name == "n")
        .map(|(_, v)| v)
}

/// Changes the live command's `n`, as Attributes would.
fn set_n(harness: &mut Harness<'static, State>, n: i64) {
    let view = harness.state().workbench.script_view().expect("live");
    let values = view
        .values
        .into_iter()
        .map(|(name, v)| {
            if name == "n" {
                (name, SettingValue::Int(n))
            } else {
                (name, v)
            }
        })
        .collect();
    harness.state_mut().workbench.attributes.script_asked = Some(values);
    harness.run_steps(1);
    until_ended(harness);
}

#[test]
fn a_command_runs_from_the_scripts_menu_and_its_settings_show() {
    let (mut harness, _) = with_library("menu", &[("cubes.lua", CUBES)]);
    main_menu(&harness, "Scripts").click();
    harness.run();
    harness.get_by_label("Make cubes").click();
    until_ended(&mut harness);
    assert_eq!(shapes(&harness), 2);
    assert_eq!(
        harness.state().workbench.document.undo_label(),
        Some("Make cubes")
    );
    assert_eq!(n_of(&harness), Some(SettingValue::Int(1)));
    // Attributes shows the command's settings by their names in words.
    harness.run();
    harness.get_by_label("Make cubes");
    harness.get_by_label("Size");
}

#[test]
fn changing_settings_runs_again_as_one_undo_step() {
    let (mut harness, _) = with_library("again", &[("cubes.lua", CUBES)]);
    harness
        .state_mut()
        .workbench
        .run_script_command(&cubes_key());
    until_ended(&mut harness);
    let steps = harness.state().workbench.document.undo_steps();
    for n in [3, 4, 5, 3] {
        set_n(&mut harness, n);
        assert_eq!(shapes(&harness), 1 + n as usize);
        assert_eq!(harness.state().workbench.document.undo_steps(), steps);
    }
    assert_eq!(n_of(&harness), Some(SettingValue::Int(3)));
    harness.state_mut().workbench.document.undo();
    assert_eq!(shapes(&harness), 1, "one Ctrl + Z takes it all back");
    // The settings are kept for the next run.
    harness
        .state_mut()
        .workbench
        .run_script_command(&cubes_key());
    until_ended(&mut harness);
    assert_eq!(shapes(&harness), 4);
}

#[test]
fn a_failing_run_with_new_settings_brings_the_last_good_one_back() {
    let (mut harness, _) = with_library("failing", &[("cubes.lua", CUBES)]);
    harness
        .state_mut()
        .workbench
        .run_script_command(&cubes_key());
    until_ended(&mut harness);
    set_n(&mut harness, 2);
    assert_eq!(shapes(&harness), 2, "the run with n = 1 is back");
    assert_eq!(n_of(&harness), Some(SettingValue::Int(1)));
    assert!(
        harness
            .state()
            .workbench
            .note
            .as_deref()
            .unwrap_or("")
            .contains("last ones are back")
    );
}

#[test]
fn settings_leave_attributes_once_something_else_happens() {
    let (mut harness, _) = with_library("leave", &[("cubes.lua", CUBES)]);
    harness
        .state_mut()
        .workbench
        .run_script_command(&cubes_key());
    until_ended(&mut harness);
    assert!(harness.state().workbench.script_view().is_some());
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.run_steps(2);
    assert!(harness.state().workbench.script_view().is_none());
}

#[test]
fn ctrl_wheel_changes_the_count() {
    let (mut harness, _) = with_library("wheel", &[("cubes.lua", CUBES)]);
    harness
        .state_mut()
        .workbench
        .run_script_command(&cubes_key());
    until_ended(&mut harness);
    assert!(harness.state().workbench.script_count_live());
    assert!(harness.state_mut().workbench.script_wheel(2));
    until_ended(&mut harness);
    assert_eq!(n_of(&harness), Some(SettingValue::Int(3)));
    assert_eq!(shapes(&harness), 4);
    assert!(harness.state_mut().workbench.script_wheel(-10));
    until_ended(&mut harness);
    assert_eq!(n_of(&harness), Some(SettingValue::Int(1)), "never below 1");
}

#[test]
fn a_key_already_taken_is_refused_and_a_free_one_runs_the_command() {
    let (mut harness, _) = with_library("keys", &[("cubes.lua", CUBES)]);
    let ask = KeyAsk {
        key: cubes_key(),
        name: "Make cubes".into(),
        refused: None,
    };
    let ctx = harness.ctx.clone();
    let workbench = &mut harness.state_mut().workbench;
    workbench.scripts.asking_key = Some(ask.clone());
    workbench.give_key(&ask, SavedKey::from_press(Key::S, Modifiers::COMMAND), &ctx);
    let refused = workbench
        .scripts
        .asking_key
        .as_ref()
        .unwrap()
        .refused
        .clone()
        .unwrap();
    assert!(refused.contains("Save"), "{refused}");
    workbench.give_key(&ask, SavedKey::from_press(Key::F11, Modifiers::NONE), &ctx);
    assert!(workbench.scripts.settings.key_of(&cubes_key()).is_none());
    workbench.give_key(&ask, SavedKey::from_press(Key::F6, Modifiers::NONE), &ctx);
    assert!(workbench.scripts.asking_key.is_none());
    assert!(workbench.scripts.settings.key_of(&cubes_key()).is_some());
    // Another command can't have it too.
    let other = KeyAsk {
        key: CommandKey {
            file: "x.lua".into(),
            command: None,
        },
        name: "X".into(),
        refused: None,
    };
    workbench.scripts.asking_key = Some(other.clone());
    workbench.give_key(&other, SavedKey::from_press(Key::F6, Modifiers::NONE), &ctx);
    let refused = workbench
        .scripts
        .asking_key
        .as_ref()
        .unwrap()
        .refused
        .clone()
        .unwrap();
    assert!(refused.contains("Make cubes"), "{refused}");
    workbench.scripts.asking_key = None;
    harness.run();
    harness.key_press(Key::F6);
    until_ended(&mut harness);
    assert_eq!(shapes(&harness), 2);
}

#[test]
fn the_key_window_takes_a_press() {
    let (mut harness, _) = with_library("key-window", &[("cubes.lua", CUBES)]);
    harness.state_mut().workbench.scripts.asking_key = Some(KeyAsk {
        key: cubes_key(),
        name: "Make cubes".into(),
        refused: None,
    });
    harness.run();
    harness.get_by_label_contains("Press the keys");
    harness.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::K);
    harness.run();
    let workbench = &harness.state().workbench;
    assert!(workbench.scripts.asking_key.is_none());
    let key = workbench.scripts.settings.key_of(&cubes_key()).unwrap();
    assert!(key.ctrl && key.shift && !key.alt);
}

#[test]
fn script_commands_are_in_command_search_and_on_the_palette() {
    let (mut harness, _) = with_library("search", &[("cubes.lua", CUBES)]);
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let found = entries
        .iter()
        .find(|e| e.name == "Make cubes")
        .expect("listed");
    assert_eq!(found.place, "Scripts");
    assert_eq!(found.help, "Makes some cubes.");
    harness
        .state_mut()
        .workbench
        .scripts
        .settings
        .set_on_palette(&cubes_key(), true);
    harness.run();
    harness.get_by_label("Make cubes").click();
    until_ended(&mut harness);
    assert_eq!(shapes(&harness), 2);
}

#[test]
fn the_library_list_offers_the_palette_and_a_key() {
    let (mut harness, dir) = with_library("list", &[("cubes.lua", CUBES)]);
    harness.state_mut().workbench.toggle_scripts_panel();
    harness.run();
    harness.get_by_label("cubes.lua").click_secondary();
    harness.run();
    harness
        .get_by_label(crate::scripts_panel::ADD_TO_PALETTE)
        .click();
    harness.run();
    assert!(
        harness
            .state()
            .workbench
            .scripts
            .settings
            .on_palette(&cubes_key())
    );
    harness.get_by_label("cubes.lua").click_secondary();
    harness.run();
    harness.get_by_label(crate::scripts_panel::OFF_PALETTE);
    harness.get_by_label(crate::scripts_panel::GIVE_KEY).click();
    harness.run();
    assert!(harness.state().workbench.scripts.asking_key.is_some());
    let _ = dir;
}

#[test]
fn a_script_with_no_commands_runs_whole_and_one_turned_off_leaves_the_menus() {
    let (mut harness, _) = with_library(
        "whole",
        &[
            ("hello.lua", "-- name: Say hello\nprint(\"hello\")\n"),
            ("cubes.lua", CUBES),
        ],
    );
    let hello = CommandKey {
        file: "hello.lua".into(),
        command: None,
    };
    harness.state_mut().workbench.run_script_command(&hello);
    until_ended(&mut harness);
    assert!(harness.state().workbench.scripts.rows.iter().any(
        |r| matches!(r, crate::scripts_panel::Row::Line(script::Line::Said(s)) if s == "hello")
    ));
    harness
        .state_mut()
        .workbench
        .scripts
        .settings
        .off
        .push("cubes.lua".into());
    let names: Vec<String> = harness
        .state()
        .workbench
        .scripts
        .commands()
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(names, ["Say hello"]);
}

#[test]
fn renaming_and_removing_in_the_manager_keep_keys_with_the_script() {
    let (mut harness, dir) = with_library("manager", &[("cubes.lua", CUBES)]);
    let workbench = &mut harness.state_mut().workbench;
    workbench
        .scripts
        .settings
        .keys
        .push((cubes_key(), SavedKey::from_press(Key::F6, Modifiers::NONE)));
    workbench.rename_script(&dir.join("cubes.lua"), "More cubes");
    assert!(dir.join("More cubes.lua").exists());
    let moved = CommandKey {
        file: "More cubes.lua".into(),
        command: Some("Cubes".into()),
    };
    assert!(workbench.scripts.settings.key_of(&moved).is_some());
    workbench.rename_script(&dir.join("More cubes.lua"), "bad/name");
    assert!(workbench.scripts.manager.renaming.is_none() || dir.join("More cubes.lua").exists());
    workbench.remove_script(&dir.join("More cubes.lua"), "More cubes.lua");
    assert!(!dir.join("More cubes.lua").exists());
    assert!(workbench.scripts.settings.keys.is_empty());
    read_library(&mut harness);
    assert!(harness.state().workbench.scripts.commands().is_empty());
}

#[test]
fn the_manager_opens_from_the_scripts_menu() {
    let (mut harness, _) = with_library("open-manager", &[("cubes.lua", CUBES)]);
    main_menu(&harness, "Scripts").click();
    harness.run();
    harness.get_by_label(scripting::MANAGER_ENTRY).click();
    harness.run();
    assert!(harness.state().workbench.scripts.manager.open);
    // Again without the menu in the way.
    let (mut harness, _) = with_library("open-manager-2", &[("cubes.lua", CUBES)]);
    harness.state_mut().workbench.scripts.manager.open = true;
    // The window settles on its size over a few frames.
    harness.run_steps(20);
    harness.get_by_label_contains("Makes some cubes.");
    harness.get_by_label("Remove…").click();
    harness.run_steps(4);
    assert!(
        harness.state().workbench.scripts.manager.removing.is_some(),
        "{:?}",
        harness.state().workbench.scripts.manager
    );
    harness.run_steps(20);
    harness.get_by_label("Keep it").click();
    harness.run_steps(4);
    assert!(harness.state().workbench.scripts.manager.removing.is_none());
}

#[test]
fn what_is_set_up_for_scripts_is_shared_by_tabs_and_kept() {
    let (mut harness, _) = with_library("shared", &[("cubes.lua", CUBES)]);
    harness
        .state_mut()
        .workbench
        .scripts
        .settings
        .set_on_palette(&cubes_key(), true);
    harness.state_mut().workbench.scripts.settings.time_seconds = 42;
    let mut other = Workbench::new(super::tests::info(), None);
    other.take_shared_from(&harness.state().workbench);
    assert!(other.scripts.settings.on_palette(&cubes_key()));
    assert_eq!(other.scripts.commands().len(), 1);
    let saved = harness.state().workbench.script_settings();
    let restored = Workbench::new(super::tests::info(), None).with_script_settings(saved.clone());
    assert_eq!(restored.script_settings(), saved);
    assert_eq!(restored.scripts.limits.time, Duration::from_secs(42));
}

#[test]
fn settings_scripts_section_reads_times() {
    use crate::settings::parse_seconds;
    assert_eq!(parse_seconds("90"), Some(90.0));
    assert_eq!(parse_seconds("2 min"), Some(120.0));
    assert_eq!(parse_seconds("2m"), Some(120.0));
    assert_eq!(parse_seconds("15 s"), Some(15.0));
    assert_eq!(parse_seconds("soon"), None);
}
