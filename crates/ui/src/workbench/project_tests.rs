//! Tests for the project file in the interface: the title, File menu,
//! keys, the "Save changes?" question, and reporting back from the app.
//! (Writing and reading the files themselves is tested in `fileio`.)

use super::tests::{State, empty, fresh, harness_for, info, main_menu};
use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn actions(harness: &mut Harness<'static, State>) -> Vec<WorkbenchAction> {
    std::mem::take(&mut harness.state_mut().actions)
}

fn press(harness: &mut Harness<'static, State>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

fn workbench<'h>(harness: &'h Harness<'static, State>) -> &'h Workbench {
    &harness.state().workbench
}

fn path(name: &str) -> PathBuf {
    PathBuf::from(format!("/work/{name}"))
}

/// A harness whose workbench has just "saved" its cube to crate.avtl.
fn saved_cube() -> Harness<'static, State> {
    let mut harness = fresh();
    harness.state_mut().workbench.saved(path("crate.avtl"));
    harness.run();
    actions(&mut harness);
    harness
}

// ------------------------------------------------------- title and state ---

#[test]
fn a_new_project_is_untitled_and_marked_when_changed() {
    let mut harness = empty();
    assert_eq!(workbench(&harness).title(), "Untitled — Test App 9.9.9");
    assert!(!workbench(&harness).has_unsaved_changes());
    harness.get_by_label("Create group").click();
    harness.run();
    assert!(workbench(&harness).has_unsaved_changes());
    assert_eq!(workbench(&harness).title(), "Untitled* — Test App 9.9.9");
}

#[test]
fn a_saved_project_is_named_and_clean_until_changed_again() {
    let mut harness = saved_cube();
    assert_eq!(workbench(&harness).title(), "crate.avtl — Test App 9.9.9");
    assert_eq!(
        workbench(&harness).project_path(),
        Some(path("crate.avtl").as_path())
    );
    assert_eq!(workbench(&harness).note(), Some("Saved crate.avtl."));
    assert_eq!(workbench(&harness).recent_projects(), [path("crate.avtl")]);
    // Undo still works after saving, and makes it changed again.
    press(&mut harness, Modifiers::COMMAND, Key::Z);
    assert!(workbench(&harness).has_unsaved_changes());
}

// --------------------------------------------------------- saving keys ---

#[test]
fn ctrl_s_saves_where_it_was_and_asks_where_the_first_time() {
    let mut harness = fresh();
    press(&mut harness, Modifiers::COMMAND, Key::S);
    assert_eq!(
        actions(&mut harness),
        [WorkbenchAction::Save { path: None }]
    );
    harness.state_mut().workbench.saved(path("crate.avtl"));
    press(&mut harness, Modifiers::COMMAND, Key::S);
    assert_eq!(
        actions(&mut harness),
        [WorkbenchAction::Save {
            path: Some(path("crate.avtl"))
        }]
    );
    // Save As always asks.
    press(&mut harness, Modifiers::COMMAND | Modifiers::SHIFT, Key::S);
    assert_eq!(
        actions(&mut harness),
        [WorkbenchAction::Save { path: None }]
    );
}

#[test]
fn the_file_menu_saves_and_opens() {
    let mut harness = saved_cube();
    for (label, expected) in [
        (
            "Save",
            WorkbenchAction::Save {
                path: Some(path("crate.avtl")),
            },
        ),
        ("Save As…", WorkbenchAction::Save { path: None }),
        ("Open…", WorkbenchAction::Open { path: None }),
    ] {
        main_menu(&harness, "File").click();
        harness.run();
        harness
            .get_all_by_label_contains(label)
            .find(|n| {
                n.accesskit_node()
                    .label()
                    .is_some_and(|l| l.starts_with(label))
            })
            .unwrap_or_else(|| panic!("no {label}"))
            .click();
        harness.run();
        assert_eq!(actions(&mut harness), [expected], "{label}");
    }
}

#[test]
fn a_failed_or_cancelled_save_changes_nothing() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .save_failed("the disk is full");
    harness.run();
    assert_eq!(
        workbench(&harness).note(),
        Some("Couldn't save: the disk is full.")
    );
    assert!(workbench(&harness).has_unsaved_changes());
    assert_eq!(workbench(&harness).project_path(), None);
    harness.state_mut().workbench.save_cancelled();
    assert!(workbench(&harness).has_unsaved_changes());
}

// ------------------------------------------------- the unsaved question ---

#[test]
fn quitting_with_unsaved_changes_asks_first() {
    let mut harness = fresh();
    press(&mut harness, Modifiers::COMMAND, Key::Q);
    assert!(actions(&mut harness).is_empty(), "not yet");
    assert!(workbench(&harness).is_asking_to_save());
    assert!(
        harness
            .query_by_label("Save changes to Untitled?")
            .is_some()
    );
    assert!(harness.query_by_label_contains("before closing").is_some());
    // Esc: carry on working.
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert!(!workbench(&harness).is_asking_to_save());
    assert!(actions(&mut harness).is_empty());
    // Ask again, and choose not to save.
    press(&mut harness, Modifiers::COMMAND, Key::Q);
    harness.get_by_label("Don't save").click();
    harness.run();
    assert_eq!(actions(&mut harness), [WorkbenchAction::Quit]);
}

#[test]
fn ctrl_d_answers_dont_save() {
    let mut harness = fresh();
    press(&mut harness, Modifiers::COMMAND, Key::Q);
    press(&mut harness, Modifiers::COMMAND, Key::D);
    assert_eq!(actions(&mut harness), [WorkbenchAction::Quit]);
}

#[test]
fn choosing_save_saves_then_carries_on() {
    let mut harness = fresh();
    press(&mut harness, Modifiers::COMMAND, Key::Q);
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(
        actions(&mut harness),
        [WorkbenchAction::Save { path: None }]
    );
    assert!(!workbench(&harness).is_asking_to_save());
    // The app saves, and the quit follows on the next frame.
    harness.state_mut().workbench.saved(path("crate.avtl"));
    harness.run();
    assert_eq!(actions(&mut harness), [WorkbenchAction::Quit]);
}

#[test]
fn a_cancelled_save_stops_the_quit() {
    let mut harness = fresh();
    press(&mut harness, Modifiers::COMMAND, Key::Q);
    harness.get_by_label("Save").click();
    harness.run();
    actions(&mut harness);
    harness.state_mut().workbench.save_cancelled();
    harness.run();
    harness.run();
    assert!(actions(&mut harness).is_empty(), "still open");
}

#[test]
fn the_close_button_asks_only_with_unsaved_changes() {
    let mut clean = empty();
    assert!(clean.state_mut().workbench.request_close());
    let mut harness = fresh();
    assert!(!harness.state_mut().workbench.request_close());
    harness.run();
    assert!(
        harness
            .query_by_label("Save changes to Untitled?")
            .is_some()
    );
    press(&mut harness, Modifiers::COMMAND, Key::D);
    assert_eq!(actions(&mut harness), [WorkbenchAction::Quit]);
}

#[test]
fn keys_answer_the_question_not_the_scene() {
    let mut harness = fresh();
    let id = harness.state().workbench.document().scene().children(None)[0];
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    press(&mut harness, Modifiers::COMMAND, Key::Q);
    press(&mut harness, Modifiers::NONE, Key::Delete);
    press(&mut harness, Modifiers::NONE, Key::B);
    assert_eq!(
        workbench(&harness).document().scene().len(),
        1,
        "nothing deleted"
    );
    assert!(!workbench(&harness).shape_picker().is_open());
}

// ------------------------------------------------------------ new, open ---

#[test]
fn new_starts_an_empty_project_asking_first_if_needed() {
    let mut harness = saved_cube();
    // Saved: no question.
    press(&mut harness, Modifiers::COMMAND, Key::N);
    assert!(workbench(&harness).document().scene().is_empty());
    assert_eq!(workbench(&harness).project_path(), None);
    assert_eq!(workbench(&harness).title(), "Untitled — Test App 9.9.9");
    // Changed: asked first.
    harness.get_by_label("Create group").click();
    harness.run();
    press(&mut harness, Modifiers::COMMAND, Key::N);
    assert!(
        harness
            .query_by_label_contains("before starting a new project")
            .is_some()
    );
    harness.get_by_label("Don't save").click();
    harness.run();
    assert!(workbench(&harness).document().scene().is_empty());
    assert_eq!(workbench(&harness).document().undo_steps(), 0);
}

#[test]
fn open_asks_the_app_once_changes_are_dealt_with() {
    let mut harness = saved_cube();
    press(&mut harness, Modifiers::COMMAND, Key::O);
    assert_eq!(
        actions(&mut harness),
        [WorkbenchAction::Open { path: None }]
    );
    harness.get_by_label("Create group").click();
    harness.run();
    press(&mut harness, Modifiers::COMMAND, Key::O);
    assert!(actions(&mut harness).is_empty());
    assert!(
        harness
            .query_by_label_contains("before opening another project")
            .is_some()
    );
    press(&mut harness, Modifiers::COMMAND, Key::D);
    assert_eq!(
        actions(&mut harness),
        [WorkbenchAction::Open { path: None }]
    );
}

#[test]
fn an_opened_project_replaces_the_scene_with_a_fresh_history() {
    let mut harness = fresh();
    harness.state_mut().workbench.open_settings();
    let mut opened = scene::Document::new();
    for _ in 0..3 {
        opened
            .execute(scene::Command::Create(scene::ObjectKind::Primitive(
                scene::PrimitiveKind::Sphere.standard(),
            )))
            .unwrap();
    }
    let opened = scene::Document::from_scene(opened.scene().clone(), []);
    let workbench_now = &mut harness.state_mut().workbench;
    workbench_now.opened(opened, path("spheres.avtl"));
    harness.run();
    let wb = workbench(&harness);
    assert_eq!(wb.document().scene().len(), 3);
    assert_eq!(wb.document().undo_steps(), 0);
    assert!(!wb.has_unsaved_changes());
    assert_eq!(wb.title(), "spheres.avtl — Test App 9.9.9");
    assert_eq!(wb.note(), Some("Opened spheres.avtl."));
    assert_eq!(
        wb.document().history_limit(),
        wb.undo_memory().limit(),
        "the undo memory setting carries over"
    );
    // The viewport draws the new scene.
    let drawn = harness
        .state()
        .renderer
        .drawn
        .last()
        .unwrap()
        .scene
        .items
        .len();
    assert_eq!(drawn, 3);
}

#[test]
fn a_project_that_wont_open_is_explained_and_a_missing_one_forgotten() {
    let mut harness = harness_for(
        Workbench::new(info(), None)
            .with_recent_projects(vec![path("gone.avtl"), path("other.avtl")]),
    );
    harness.run();
    let gone = path("gone.avtl");
    harness
        .state_mut()
        .workbench
        .open_failed(&gone, "it isn't a project file");
    harness.run();
    assert_eq!(
        workbench(&harness).note(),
        Some("Couldn't open gone.avtl: it isn't a project file.")
    );
    assert_eq!(workbench(&harness).recent_projects(), [path("other.avtl")]);
}

// ---------------------------------------------------------- open recent ---

#[test]
fn open_recent_lists_projects_and_opens_one() {
    let mut harness = harness_for(
        Workbench::new(info(), None)
            .with_recent_projects(vec![path("crate.avtl"), path("barrel.avtl")]),
    );
    harness.run();
    main_menu(&harness, "File").click();
    harness.run();
    harness.get_by_label_contains("Open recent").click();
    harness.run();
    harness.get_by_label("barrel.avtl  (work)").click();
    harness.run();
    assert_eq!(
        actions(&mut harness),
        [WorkbenchAction::Open {
            path: Some(path("barrel.avtl"))
        }]
    );
}

#[test]
fn the_recent_list_can_be_cleared_and_is_greyed_when_empty() {
    let mut harness =
        harness_for(Workbench::new(info(), None).with_recent_projects(vec![path("crate.avtl")]));
    harness.run();
    main_menu(&harness, "File").click();
    harness.run();
    harness.get_by_label_contains("Open recent").click();
    harness.run();
    harness.get_by_label("Clear the list").click();
    harness.run();
    assert!(workbench(&harness).recent_projects().is_empty());
    main_menu(&harness, "File").click();
    harness.run();
    assert!(
        harness
            .get_by_label_contains("Open recent")
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn at_most_eight_recent_projects_are_kept() {
    let many = (0..20).map(|n| path(&format!("{n}.avtl"))).collect();
    let workbench = Workbench::new(info(), None).with_recent_projects(many);
    assert_eq!(workbench.recent_projects().len(), crate::RECENT_COUNT);
}
