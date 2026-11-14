//! Tests for command search in the window (decision 0126).

use super::super::tests::fresh;
use crate::command_search::SEARCH_LABEL;
use egui::{Key, Modifiers};
use egui_kittest::kittest::Queryable;

#[test]
fn space_opens_the_search_typing_finds_and_enter_runs() {
    let mut harness = fresh();
    harness.key_press(Key::Space);
    harness.run();
    assert!(harness.state().workbench.is_search_open());
    assert!(harness.query_by_label_contains(SEARCH_LABEL).is_some());
    harness.get_by_label(SEARCH_LABEL).type_text("cube");
    harness.run();
    let objects = harness.state().workbench.document().scene().len();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(!harness.state().workbench.is_search_open());
    assert_eq!(
        harness.state().workbench.document().scene().len(),
        objects + 1,
        "a cube was made"
    );
}

#[test]
fn esc_closes_it_and_space_while_typing_types_a_space() {
    let mut harness = fresh();
    harness.key_press(Key::Space);
    harness.run();
    harness.get_by_label(SEARCH_LABEL).type_text("loop cut");
    harness.run();
    assert_eq!(harness.state().workbench.search.query, "loop cut");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().workbench.is_search_open());
    // Nothing else happened: the selection is as it was.
    harness.key_press_modifiers(Modifiers::NONE, Key::Escape);
    harness.run();
}

#[test]
fn every_kind_of_entry_is_found_with_its_key() {
    let harness = fresh();
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let find = |name: &str| entries.iter().find(|e| e.name == name).cloned();
    assert!(find("Extrude").unwrap().keys.is_some());
    assert!(find("Export…").is_some());
    assert!(find("User manual").is_some());
    assert!(find("Points mode").unwrap().keys.is_some());
    assert!(find("Cube").is_some());
    // Extrude can't be used with nothing selected, and says why.
    assert!(find("Extrude").unwrap().not_now.is_some());
}

#[test]
fn the_search_runs_menu_entries_too() {
    let mut harness = fresh();
    harness.key_press(Key::Space);
    harness.run();
    harness.get_by_label(SEARCH_LABEL).type_text("settings");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().workbench.is_settings_open());
}

#[test]
fn help_has_the_manual_search_and_a_developer_menu_and_view_has_show_and_hide() {
    let mut harness = fresh();
    super::super::tests::main_menu(&harness, "Help").click();
    harness.run();
    assert!(
        harness
            .query_by_label_contains(super::super::DEVELOPER_MENU)
            .is_some()
    );
    assert!(
        harness.query_by_label("Add a heavy test shape").is_none(),
        "tucked away"
    );
    harness.get_by_label(super::super::MANUAL_ENTRY).click();
    harness.run();
    assert!(
        harness
            .state()
            .workbench
            .note
            .as_deref()
            .is_some_and(|n| n.contains("manual"))
    );
    super::super::tests::main_menu(&harness, "Help").click();
    harness.run();
    harness
        .get_by_label_contains(super::super::SEARCH_ENTRY)
        .click();
    harness.run();
    assert!(harness.state().workbench.is_search_open());
    harness.key_press(Key::Escape);
    harness.run();
    super::super::tests::main_menu(&harness, "View").click();
    harness.run();
    assert!(
        harness
            .query_by_label_contains(super::super::SHOW_HIDE_MENU)
            .is_some()
    );
}

fn info_with(file: Option<&str>) -> crate::AppInfo {
    crate::AppInfo {
        manual_file: file.map(std::path::PathBuf::from),
        manual_url: "https://example.com/docs/manual/SUMMARY.md".into(),
        ..super::super::tests::info()
    }
}

#[test]
fn without_an_installed_guide_the_manual_is_read_online() {
    let info = info_with(None);
    assert_eq!(
        super::manual_address(&info),
        "https://example.com/docs/manual/SUMMARY.md"
    );
    assert_eq!(
        super::glossary_address(&info),
        "https://example.com/docs/manual/concepts.md"
    );
}

#[test]
fn the_installed_guide_opens_from_its_file() {
    let info = info_with(Some("/opt/My App/user-guide/index.html"));
    assert_eq!(
        super::manual_address(&info),
        "file:///opt/My%20App/user-guide/index.html"
    );
    assert_eq!(
        super::glossary_address(&info),
        "file:///opt/My%20App/user-guide/index.html#concepts"
    );
}

#[test]
fn windows_paths_become_file_addresses() {
    let url = super::file_url(std::path::Path::new(
        r"C:\Users\Ann\AppData\Local\Programs\App 3D\user-guide\index.html",
    ));
    assert_eq!(
        url,
        "file:///C:/Users/Ann/AppData/Local/Programs/App%203D/user-guide/index.html"
    );
    let long = super::file_url(std::path::Path::new(r"\\?\D:\Apps\index.html"));
    assert_eq!(long, "file:///D:/Apps/index.html");
    let share = super::file_url(std::path::Path::new(r"\\?\UNC\server\share\index.html"));
    assert_eq!(share, "file:////server/share/index.html");
    // Letters outside English are written as UTF-8 bytes.
    assert!(
        super::file_url(std::path::Path::new("/tmp/Größe/a.html"))
            .ends_with("Gr%C3%B6%C3%9Fe/a.html")
    );
}
