//! Tests for backup copies in the interface: when one is asked for, the
//! question after a crash and its answers, and the Autosave settings.
//! (Writing and finding the copies is tested in `fileio` and `app`.)

use super::tests::{State, empty, fresh, info};
use super::*;
use crate::{RecoveryCopy, RecoveryOffer};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use std::time::{Duration, SystemTime};

fn actions(harness: &mut Harness<'static, State>) -> Vec<WorkbenchAction> {
    std::mem::take(&mut harness.state_mut().actions)
}

/// Runs a frame at `seconds` on the window's clock.
fn at(harness: &mut Harness<'static, State>, seconds: f64) {
    harness.input_mut().time = Some(seconds);
    harness.step();
}

fn autosaves(actions: &[WorkbenchAction]) -> Vec<usize> {
    actions
        .iter()
        .filter_map(|a| match a {
            WorkbenchAction::Autosave { keep, .. } => Some(*keep),
            _ => None,
        })
        .collect()
}

/// An offer of two copies, saved 3 and 10 minutes ago.
fn offer(project: Option<&str>) -> RecoveryOffer {
    let now = SystemTime::now();
    RecoveryOffer {
        folder: PathBuf::from("/backups/session-1"),
        project: project.map(PathBuf::from),
        copies: vec![
            RecoveryCopy {
                path: PathBuf::from("/backups/session-1/copy-2.avtl"),
                saved: now - Duration::from_secs(3 * 60 + 5),
            },
            RecoveryCopy {
                path: PathBuf::from("/backups/session-1/copy-1.avtl"),
                saved: now - Duration::from_secs(10 * 60 + 5),
            },
        ],
    }
}

fn asking(project: Option<&str>) -> Harness<'static, State> {
    let mut harness = empty();
    harness
        .state_mut()
        .workbench
        .offer_recovery(vec![offer(project)]);
    harness.run();
    assert!(harness.state().workbench.is_asking_to_recover());
    assert!(
        harness
            .query_by_label(crate::autosave::RECOVERY_TITLE)
            .is_some()
    );
    harness
}

// ---------------------------------------------------------------- timer ---

/// A harness whose scene was changed (a group added) at 100 s.
fn changed_at_100() -> Harness<'static, State> {
    let mut harness = fresh();
    at(&mut harness, 50.0);
    harness.get_by_label("Create group").click();
    at(&mut harness, 100.0);
    actions(&mut harness);
    harness
}

#[test]
fn changed_work_is_backed_up_on_the_clock_with_the_copies_to_keep() {
    // A scene as opened needs no backup, however long it waits.
    let mut quiet = fresh();
    at(&mut quiet, 10_000.0);
    assert!(autosaves(&actions(&mut quiet)).is_empty());

    let mut harness = changed_at_100();
    at(&mut harness, 200.0);
    assert!(autosaves(&actions(&mut harness)).is_empty(), "not yet");
    at(&mut harness, 5.0 * 60.0 + 1.0);
    let found = actions(&mut harness);
    assert_eq!(autosaves(&found), [5]);
    // The copy holds the scene: the cube and the group.
    let Some(WorkbenchAction::Autosave { document, .. }) = found.first() else {
        panic!("no copy");
    };
    assert_eq!(document.scene().children(None).len(), 2);
    // Only one is asked for until it is written.
    at(&mut harness, 1_000.0);
    assert!(autosaves(&actions(&mut harness)).is_empty());
    harness.state_mut().workbench.autosaved();
    at(&mut harness, 2_000.0);
    assert!(autosaves(&actions(&mut harness)).is_empty(), "nothing new");
}

#[test]
fn saving_and_backups_are_separate() {
    // Ctrl + S doesn't reset the backup clock: the change still gets
    // its backup copy on time.
    let mut harness = changed_at_100();
    harness
        .state_mut()
        .workbench
        .saved(PathBuf::from("/work/a.avtl"));
    at(&mut harness, 5.0 * 60.0 + 1.0);
    assert_eq!(autosaves(&actions(&mut harness)), [5]);
}

#[test]
fn switched_off_backups_ask_for_nothing() {
    let mut off = super::tests::harness_for(super::tests::with_a_cube(
        Workbench::new(info(), None).with_autosave(AutosaveSettings {
            on: false,
            ..AutosaveSettings::default()
        }),
    ));
    at(&mut off, 50.0);
    off.get_by_label("Create group").click();
    at(&mut off, 100.0);
    at(&mut off, 10_000.0);
    assert!(autosaves(&actions(&mut off)).is_empty());
}

#[test]
fn a_failed_backup_is_reported() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .autosave_failed("the disk is full");
    assert_eq!(
        harness.state().workbench.note(),
        Some("Couldn't save a backup copy: the disk is full.")
    );
}

// ------------------------------------------------------------ recovery ---

#[test]
fn the_question_lists_the_copies_newest_first() {
    let harness = asking(Some("/work/crate.avtl"));
    assert!(harness.query_by_label_contains("crate.avtl").is_some());
    assert!(
        harness
            .query_by_label("Saved 3 minutes ago (newest)")
            .is_some()
    );
    assert!(harness.query_by_label("Saved 10 minutes ago").is_some());
    assert!(harness.query_by_label("Open as saved").is_some());
}

#[test]
fn restore_brings_back_the_chosen_copy() {
    let mut harness = asking(Some("/work/crate.avtl"));
    harness.get_by_label("Saved 10 minutes ago").click();
    harness.run();
    harness.get_by_label("Restore").click();
    harness.run();
    assert_eq!(
        actions(&mut harness),
        [WorkbenchAction::Restore {
            folder: PathBuf::from("/backups/session-1"),
            copy: PathBuf::from("/backups/session-1/copy-1.avtl"),
            project: Some(PathBuf::from("/work/crate.avtl")),
        }]
    );
    assert!(!harness.state().workbench.is_asking_to_recover());
}

#[test]
fn enter_restores_the_newest() {
    let mut harness = asking(None);
    harness.key_press(Key::Enter);
    harness.run();
    let found = actions(&mut harness);
    assert!(matches!(
        &found[..],
        [WorkbenchAction::Restore { copy, project: None, .. }]
            if copy.ends_with("copy-2.avtl")
    ));
}

#[test]
fn open_as_saved_opens_the_project_and_lets_the_copies_go() {
    let mut harness = asking(Some("/work/crate.avtl"));
    harness.get_by_label("Open as saved").click();
    harness.run();
    assert_eq!(
        actions(&mut harness),
        [
            WorkbenchAction::Open {
                path: Some(PathBuf::from("/work/crate.avtl"))
            },
            WorkbenchAction::DiscardBackups {
                folder: PathBuf::from("/backups/session-1")
            },
        ]
    );
}

#[test]
fn an_unsaved_scene_has_no_open_as_saved_and_discard_lets_go() {
    let mut harness = asking(None);
    assert!(harness.query_by_label("Open as saved").is_none());
    assert!(
        harness
            .query_by_label_contains("an unsaved scene")
            .is_some()
    );
    harness.get_by_label("Discard").click();
    harness.run();
    assert_eq!(
        actions(&mut harness),
        [WorkbenchAction::DiscardBackups {
            folder: PathBuf::from("/backups/session-1")
        }]
    );
}

#[test]
fn two_leftovers_are_asked_about_one_after_the_other() {
    let mut harness = empty();
    let mut second = offer(None);
    second.folder = PathBuf::from("/backups/session-2");
    harness.state_mut().workbench.offer_recovery(vec![
        offer(None),
        second,
        RecoveryOffer {
            folder: PathBuf::from("/backups/empty"),
            project: None,
            copies: Vec::new(),
        },
    ]);
    harness.run();
    harness.get_by_label("Discard").click();
    harness.run();
    assert!(harness.state().workbench.is_asking_to_recover());
    harness.get_by_label("Discard").click();
    harness.run();
    assert!(
        !harness.state().workbench.is_asking_to_recover(),
        "the empty one isn't asked about"
    );
    assert_eq!(actions(&mut harness).len(), 2);
}

#[test]
fn restored_work_is_unsaved_named_and_not_backed_up_again_straight_away() {
    let mut harness = empty();
    let mut document = Document::new();
    document
        .execute(scene::Command::Create(scene::ObjectKind::Primitive(
            scene::PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
    harness.state_mut().workbench.recovered(
        document,
        Some(PathBuf::from("/work/crate.avtl")),
        "3 minutes ago",
    );
    at(&mut harness, 0.0);
    let workbench = &harness.state().workbench;
    assert!(workbench.has_unsaved_changes());
    assert_eq!(
        workbench.project_path(),
        Some(Path::new("/work/crate.avtl"))
    );
    assert_eq!(workbench.title(), "● crate.avtl (unsaved) — Test App 9.9.9");
    assert_eq!(
        workbench.note(),
        Some("Restored your work from 3 minutes ago. Save it to keep it.")
    );
    at(&mut harness, 10_000.0);
    assert!(autosaves(&actions(&mut harness)).is_empty());
    harness
        .state_mut()
        .workbench
        .recovery_failed("it is damaged");
    assert_eq!(
        harness.state().workbench.note(),
        Some("Couldn't restore the backup copy: it is damaged.")
    );
}

// ------------------------------------------------------------ settings ---

#[test]
fn settings_change_how_backups_are_made_and_show_where_they_go() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .set_autosave_folder(Some(PathBuf::from("/data/backups")));
    crate::theme::apply(&harness.ctx);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Comma);
    harness.run();
    harness.get_by_label("Settings section: Autosave").click();
    harness.run();
    assert!(harness.query_by_label_contains("/data/backups").is_some());
    harness
        .get_by_label("Save backup copies in the background")
        .click();
    harness.run();
    assert!(!harness.state().workbench.autosave_settings().on);
    harness.get_by_label("Restore standard").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.autosave_settings(),
        AutosaveSettings::default()
    );
}
