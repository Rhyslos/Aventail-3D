//! Tests for saves and exports done on another thread (decision 0163),
//! as the workbench sees them: the status bar says what is being done,
//! another save, export, open or new project waits, and a save reported
//! late leaves the changes made meanwhile unsaved.

use super::tests::{State, fresh};
use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn actions(harness: &mut Harness<'static, State>) -> Vec<WorkbenchAction> {
    std::mem::take(&mut harness.state_mut().actions)
}

fn path(name: &str) -> PathBuf {
    PathBuf::from(format!("/work/{name}"))
}

/// A harness whose workbench has saved its cube and is saving it again,
/// on another thread.
fn saving() -> Harness<'static, State> {
    let mut harness = fresh();
    harness.state_mut().workbench.saved(path("crate.avtl"));
    harness.run();
    actions(&mut harness);
    harness
        .state_mut()
        .workbench
        .file_work_started(FileWork::Saving(path("crate.avtl")));
    harness.step();
    harness
}

#[test]
fn the_status_bar_says_what_is_being_saved() {
    let harness = saving();
    let workbench = &harness.state().workbench;
    assert_eq!(workbench.note(), Some("Saving crate.avtl…"));
    assert_eq!(
        workbench.file_work(),
        Some(&FileWork::Saving(path("crate.avtl")))
    );
    assert!(harness.query_by_label("Saving crate.avtl…").is_some());
}

#[test]
fn another_save_or_export_waits_and_says_so() {
    let mut harness = saving();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::S);
    harness.step();
    assert!(actions(&mut harness).is_empty(), "no second save");
    let note = harness.state().workbench.note().unwrap().to_string();
    assert!(note.contains("Try again when it's done"), "{note}");
    // Reported: the next save goes.
    harness
        .state_mut()
        .workbench
        .saved_at(path("crate.avtl"), 0, None);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::S);
    harness.step();
    assert!(matches!(
        actions(&mut harness).as_slice(),
        [WorkbenchAction::Save { .. }]
    ));
}

#[test]
fn a_new_project_waits_for_the_save() {
    let mut harness = saving();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::N);
    harness.step();
    // Not replaced: the cube is still there.
    let workbench = &harness.state().workbench;
    assert_eq!(workbench.document().scene().walk().len(), 1);
    assert!(workbench.note().unwrap().contains("Try again"));
}

#[test]
fn a_save_reported_late_keeps_later_changes_unsaved() {
    let mut harness = saving();
    let revision = harness.state().workbench.document().revision();
    harness.get_by_label("Create group").click();
    harness.step();
    harness
        .state_mut()
        .workbench
        .saved_at(path("crate.avtl"), revision, None);
    let workbench = &harness.state().workbench;
    assert!(workbench.has_unsaved_changes());
    assert_eq!(workbench.file_work(), None);
    assert_eq!(
        workbench.note(),
        Some("Saved crate.avtl. Changes made while it was saving aren't in it yet.")
    );
}

#[test]
fn an_export_reported_ends_the_work() {
    let mut harness = fresh();
    let workbench = &mut harness.state_mut().workbench;
    workbench.file_work_started(FileWork::Exporting(path("crate.glb")));
    assert_eq!(workbench.note(), Some("Exporting crate.glb…"));
    workbench.export_failed("the disk is full");
    assert_eq!(workbench.file_work(), None);
    workbench.file_work_started(FileWork::ExportingToGame);
    workbench.target_exported(Err("no folder".into()));
    assert_eq!(workbench.file_work(), None);
    workbench.file_work_started(FileWork::ExportingKit);
    workbench.closing_after_file_work();
    assert!(
        workbench
            .note()
            .unwrap()
            .ends_with("The app closes when it's done.")
    );
    workbench.kit_exported(Err("no folder".into()));
    assert_eq!(workbench.file_work(), None);
}
