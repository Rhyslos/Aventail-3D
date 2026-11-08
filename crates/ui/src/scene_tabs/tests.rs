//! Workflow tests for the scene tabs (decision 0266), run through the
//! real interface without a window: new tabs, switching (each tab keeps
//! its own scene, undo, selection and mode), closing with and without
//! unsaved changes, quitting with changes in several tabs, dragging and
//! middle-clicking, New and Open both ways, and where the tabs sit.

use super::quitting::{CLOSE_ALL, GO_THROUGH, QUIT_TITLE};
use super::strip::{NEW_TAB, close_label};
use super::*;
use crate::palette::Mode;
use crate::viewport::{NoRenderer, TabPlacement, ViewOptions};
use crate::workbench::AppInfo;
use egui::{Event, Key, Modifiers, PointerButton};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

type H = Harness<'static, State>;

struct State {
    tabs: SceneTabs,
    actions: Vec<(TabId, WorkbenchAction)>,
    renderer: NoRenderer,
}

fn info() -> AppInfo {
    AppInfo {
        name: "Test App".into(),
        version: "9.9.9".into(),
        build: None,
        manual_url: "https://example.com/manual".into(),
    }
}

/// A window with one tab, its project empty and untouched.
fn harness() -> H {
    let mut workbench = Workbench::new(info(), None);
    // Tests run frames until the window rests (decision 0231).
    workbench.viewport_mut().options_mut().steady_frames = crate::viewport::SteadyFrames::Off;
    let mut harness = Harness::builder()
        .with_size([1440.0, 900.0])
        .build_ui_state(
            |ui, state: &mut State| {
                let actions = state.tabs.show(ui, &mut state.renderer);
                state.actions.extend(actions);
            },
            State {
                tabs: SceneTabs::new(workbench),
                actions: Vec::new(),
                renderer: NoRenderer::default(),
            },
        );
    harness.run();
    harness
}

/// Adds a cube to the tab in use (an unsaved change), selected.
fn add_cube(harness: &mut H) {
    let document = harness.state_mut().tabs.active_mut().document_mut();
    document
        .execute(scene::Command::Create(scene::ObjectKind::Primitive(
            scene::PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
    let id = *document.scene().children(None).last().unwrap();
    document.set_selection([id]);
    harness.run();
}

fn objects(harness: &H) -> usize {
    harness
        .state()
        .tabs
        .active()
        .document()
        .scene()
        .children(None)
        .len()
}

/// The tab called `name` on the row: the highest widget with that name.
fn tab<'h>(harness: &'h H, name: &'h str) -> egui_kittest::Node<'h> {
    harness
        .get_all_by_label(name)
        .min_by(|a, b| a.rect().min.y.total_cmp(&b.rect().min.y))
        .unwrap_or_else(|| panic!("no tab {name}"))
}

fn names(harness: &H) -> Vec<String> {
    harness
        .state()
        .tabs
        .tabs
        .iter()
        .map(|t| t.workbench.project_name())
        .collect()
}

fn new_tab(harness: &mut H) {
    harness.get_by_label(NEW_TAB).click();
    harness.run();
}

fn quits(harness: &H) -> usize {
    harness
        .state()
        .actions
        .iter()
        .filter(|(_, a)| *a == WorkbenchAction::Quit)
        .count()
}

/// Drags the tab `name` sideways to `x`, a few steps at a time.
fn drag_tab(harness: &mut H, name: &str, x: f32) {
    let start = tab(harness, name).rect().center();
    let by = x - start.x;
    harness.event(Event::PointerMoved(start));
    harness.step();
    harness.event(Event::PointerButton {
        pos: start,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    for i in 1..=8 {
        harness.event(Event::PointerMoved(
            start + egui::vec2(by * i as f32 / 8.0, 0.0),
        ));
        harness.step();
    }
    harness.event(Event::PointerButton {
        pos: start + egui::vec2(by, 0.0),
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
}

#[test]
fn plus_opens_a_new_empty_tab_after_the_one_in_use() {
    let mut harness = harness();
    add_cube(&mut harness);
    new_tab(&mut harness);
    let tabs = &harness.state().tabs;
    assert_eq!(tabs.len(), 2);
    assert_eq!(tabs.active_id(), tabs.ids()[1]);
    assert_eq!(objects(&harness), 0);
    assert!(harness.state().tabs.active().is_untouched());
    assert_eq!(names(&harness), ["Untitled", "Untitled 2"]);
    // The app never sees the tab's own business: only the first tab's
    // backup copy, made as it is left (its cube isn't in one yet).
    assert!(
        harness
            .state()
            .actions
            .iter()
            .all(|(_, a)| matches!(a, WorkbenchAction::Autosave { .. }))
    );
}

#[test]
fn switching_keeps_each_tabs_scene_undo_selection_and_mode() {
    let mut harness = harness();
    add_cube(&mut harness);
    harness.key_press(Key::Num2);
    harness.run();
    assert_eq!(harness.state().tabs.active().mode(), Mode::Edges);
    new_tab(&mut harness);
    // The new tab: its own empty scene, nothing to undo, the Model mode.
    let fresh = harness.state().tabs.active();
    assert_eq!(fresh.mode(), Mode::Model);
    assert_eq!(fresh.document().undo_steps(), 0);
    assert!(fresh.document().selection().is_empty());
    add_cube(&mut harness);
    add_cube(&mut harness);
    assert_eq!(objects(&harness), 2);
    // Back to the first: as it was left.
    tab(&harness, "Untitled").click();
    harness.run();
    let first = harness.state().tabs.active();
    assert_eq!(first.document().scene().children(None).len(), 1);
    assert_eq!(first.document().undo_steps(), 1);
    assert_eq!(first.document().selection().len(), 1);
    assert_eq!(first.mode(), Mode::Edges);
    // Undo here touches only this tab.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(objects(&harness), 0);
    tab(&harness, "Untitled 2").click();
    harness.run();
    assert_eq!(objects(&harness), 2);
    assert_eq!(harness.state().tabs.active().mode(), Mode::Model);
}

#[test]
fn settings_are_shared_between_tabs() {
    let mut harness = harness();
    new_tab(&mut harness);
    harness
        .state_mut()
        .tabs
        .active_mut()
        .set_accent(crate::theme::Accent::Blue);
    harness
        .state_mut()
        .tabs
        .active_mut()
        .viewport_mut()
        .options_mut()
        .four_views = true;
    tab(&harness, "Untitled").click();
    harness.run();
    let first = harness.state().tabs.active();
    assert_eq!(first.accent(), crate::theme::Accent::Blue);
    assert!(first.view_options().four_views);
}

#[test]
fn a_tab_without_changes_closes_at_once() {
    let mut harness = harness();
    new_tab(&mut harness);
    let second = harness.state().tabs.active_id();
    harness.get_by_label(&close_label("Untitled 2")).click();
    harness.run();
    assert_eq!(harness.state().tabs.len(), 1);
    assert_eq!(harness.state_mut().tabs.take_closed(), [second]);
    assert!(harness.query_by_label_contains("Save changes").is_none());
}

#[test]
fn middle_click_closes_a_tab() {
    let mut harness = harness();
    new_tab(&mut harness);
    new_tab(&mut harness);
    tab(&harness, "Untitled 2").click_button(PointerButton::Middle);
    harness.run();
    assert_eq!(names(&harness), ["Untitled", "Untitled 3"]);
    // The tab in use stays in use.
    assert_eq!(harness.state().tabs.active().project_name(), "Untitled 3");
}

#[test]
fn a_tab_with_changes_asks_first() {
    let mut harness = harness();
    new_tab(&mut harness);
    add_cube(&mut harness);
    tab(&harness, "Untitled").click();
    harness.run();
    // × on the tab with changes: it comes to the front and asks.
    harness.get_by_label(&close_label("Untitled 2")).click();
    harness.run();
    assert_eq!(harness.state().tabs.active().project_name(), "Untitled 2");
    harness.get_by_label_contains("before closing its tab");
    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(harness.state().tabs.len(), 2, "cancel keeps it");
    // Again, and this time don't save.
    harness.get_by_label(&close_label("Untitled 2")).click();
    harness.run();
    harness.get_by_label("Don't save").click();
    harness.run();
    assert_eq!(names(&harness), ["Untitled"]);
    // Save asks the app to save, and the tab closes once it has.
    new_tab(&mut harness);
    add_cube(&mut harness);
    let id = harness.state().tabs.active_id();
    harness.get_by_label(&close_label("Untitled 2")).click();
    harness.run();
    harness.get_by_label("Save").click();
    harness.run();
    assert!(
        harness
            .state()
            .actions
            .contains(&(id, WorkbenchAction::Save { path: None }))
    );
    assert_eq!(harness.state().tabs.len(), 2, "waits for the save");
    harness
        .state_mut()
        .tabs
        .get_mut(id)
        .unwrap()
        .saved(std::path::PathBuf::from("/work/crate.avtl"));
    harness.run();
    assert_eq!(names(&harness), ["Untitled"]);
}

#[test]
fn closing_the_last_tab_leaves_a_new_project() {
    let mut harness = harness();
    let first = harness.state().tabs.active_id();
    harness.get_by_label(&close_label("Untitled")).click();
    harness.run();
    assert_eq!(harness.state().tabs.len(), 1);
    assert_ne!(harness.state().tabs.active_id(), first);
    assert!(harness.state().tabs.active().is_untouched());
}

#[test]
fn a_tab_being_saved_waits_to_close() {
    let mut harness = harness();
    new_tab(&mut harness);
    let path = std::path::PathBuf::from("/work/crate.avtl");
    harness
        .state_mut()
        .tabs
        .active_mut()
        .file_work_started(crate::FileWork::Saving(path));
    // The status bar's spinner keeps drawing: a few frames at a time.
    harness.run_steps(4);
    harness.get_by_label(&close_label("Untitled 2")).click();
    harness.run_steps(4);
    assert_eq!(harness.state().tabs.len(), 2);
    let note = harness.state().tabs.active().note().unwrap_or_default();
    assert!(note.contains("Close its tab once it's done"), "{note}");
}

#[test]
fn quitting_lists_every_tab_with_changes_and_asks_about_each() {
    let mut harness = harness();
    add_cube(&mut harness);
    new_tab(&mut harness);
    new_tab(&mut harness);
    add_cube(&mut harness);
    assert!(!harness.state_mut().tabs.request_close());
    harness.run();
    harness.get_by_label(QUIT_TITLE);
    harness.get_by_label("• Untitled");
    harness.get_by_label("• Untitled 3");
    assert!(harness.query_by_label("• Untitled 2").is_none(), "saved");
    harness.get_by_label(GO_THROUGH).click();
    harness.run();
    // The first with changes comes to the front, asking as closing does.
    assert_eq!(harness.state().tabs.active().project_name(), "Untitled");
    harness.get_by_label("Don't save").click();
    harness.run();
    assert_eq!(harness.state().tabs.active().project_name(), "Untitled 3");
    assert_eq!(quits(&harness), 0);
    harness.get_by_label("Don't save").click();
    harness.run();
    assert_eq!(quits(&harness), 1);
}

#[test]
fn cancelling_any_question_keeps_the_app_open() {
    let mut harness = harness();
    add_cube(&mut harness);
    new_tab(&mut harness);
    add_cube(&mut harness);
    // Cancel in the list.
    assert!(!harness.state_mut().tabs.request_close());
    harness.run();
    harness.get_by_label("Cancel").click();
    harness.run();
    assert!(harness.query_by_label(QUIT_TITLE).is_none());
    // Cancel while going through them: the tabs stay, nothing quits.
    assert!(!harness.state_mut().tabs.request_close());
    harness.run();
    harness.get_by_label(GO_THROUGH).click();
    harness.run();
    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(harness.state().tabs.len(), 2);
    assert_eq!(quits(&harness), 0);
    assert!(harness.state().tabs.quitting.is_none());
    // Close without saving any.
    assert!(!harness.state_mut().tabs.request_close());
    harness.run();
    harness.get_by_label(CLOSE_ALL).click();
    harness.run();
    assert_eq!(quits(&harness), 1);
}

#[test]
fn quitting_with_changes_in_one_tab_asks_there_and_with_none_at_once() {
    let mut harness = harness();
    new_tab(&mut harness);
    assert!(harness.state_mut().tabs.request_close(), "nothing unsaved");
    add_cube(&mut harness);
    tab(&harness, "Untitled").click();
    harness.run();
    // File → Quit: no list for one, its own question in its own tab.
    harness.get_by_label("File").click();
    harness.run();
    harness.get_by_label("Quit").click();
    harness.run();
    assert!(harness.query_by_label(QUIT_TITLE).is_none());
    assert_eq!(harness.state().tabs.active().project_name(), "Untitled 2");
    // Asked as closing the app is, not just the tab.
    harness.get_by_label_contains("before closing.");
    harness.get_by_label("Don't save").click();
    harness.run();
    assert_eq!(quits(&harness), 1);
}

#[test]
fn dragging_a_tab_moves_it_along_the_row() {
    let mut harness = harness();
    new_tab(&mut harness);
    new_tab(&mut harness);
    let order = harness.state().tabs.ids();
    // The first tab dragged past the other two.
    let end = tab(&harness, "Untitled 3").rect().right() - 4.0;
    drag_tab(&mut harness, "Untitled", end);
    let moved = harness.state().tabs.ids();
    assert_eq!(moved, [order[1], order[2], order[0]]);
    // Dragging a tab makes it the tab in use.
    assert_eq!(harness.state().tabs.active_id(), order[0]);
    // And back to the front.
    let start = tab(&harness, "Untitled 2").rect().left() + 4.0;
    drag_tab(&mut harness, "Untitled", start);
    assert_eq!(harness.state().tabs.ids(), order);
    // A click without a drag moves nothing.
    tab(&harness, "Untitled 3").click();
    harness.run();
    assert_eq!(harness.state().tabs.ids(), order);
}

#[test]
fn new_opens_a_tab_unless_this_one_is_untouched() {
    let mut harness = harness();
    // Untouched: New uses this tab.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::N);
    harness.run();
    assert_eq!(harness.state().tabs.len(), 1);
    add_cube(&mut harness);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::N);
    harness.run();
    assert_eq!(harness.state().tabs.len(), 2);
    assert!(harness.query_by_label_contains("Save changes").is_none());
    // File → New too.
    add_cube(&mut harness);
    harness.get_by_label("File").click();
    harness.run();
    harness
        .get_all_by_label_contains("New")
        .find(|n| {
            n.accesskit_node()
                .label()
                .is_some_and(|l| l.starts_with("New") && !l.starts_with(NEW_TAB))
        })
        .expect("File → New")
        .click();
    harness.run();
    assert_eq!(harness.state().tabs.len(), 3);
}

#[test]
fn with_the_setting_off_new_replaces_the_project_after_asking() {
    let mut harness = harness();
    harness
        .state_mut()
        .tabs
        .active_mut()
        .viewport_mut()
        .options_mut()
        .open_in_new_tab = false;
    add_cube(&mut harness);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::N);
    harness.run();
    assert_eq!(harness.state().tabs.len(), 1);
    harness.get_by_label("Don't save").click();
    harness.run();
    assert_eq!(harness.state().tabs.len(), 1);
    assert_eq!(objects(&harness), 0);
}

#[test]
fn open_asks_the_app_for_a_new_tab_or_this_one() {
    let mut harness = harness();
    // Untouched: opened here, as before tabs.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::O);
    harness.run();
    let id = harness.state().tabs.active_id();
    assert_eq!(
        harness.state().actions,
        [(id, WorkbenchAction::Open { path: None })]
    );
    harness.state_mut().actions.clear();
    add_cube(&mut harness);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::O);
    harness.run();
    assert_eq!(
        harness.state().actions,
        [(id, WorkbenchAction::OpenInNewTab { path: None })]
    );
    assert!(harness.query_by_label_contains("Save changes").is_none());
    // The setting off: asked about the changes, then opened here.
    harness.state_mut().actions.clear();
    harness
        .state_mut()
        .tabs
        .active_mut()
        .viewport_mut()
        .options_mut()
        .open_in_new_tab = false;
    harness.key_press_modifiers(Modifiers::COMMAND, Key::O);
    harness.run();
    harness.get_by_label("Don't save").click();
    harness.run();
    assert_eq!(
        harness.state().actions,
        [(id, WorkbenchAction::Open { path: None })]
    );
}

#[test]
fn an_opened_project_gets_a_tab_and_is_found_again() {
    let mut harness = harness();
    let path = std::path::PathBuf::from("/work/crate.avtl");
    // Into the untouched tab.
    let first = harness.state().tabs.active_id();
    let tabs = &mut harness.state_mut().tabs;
    let opened = tabs.opened_in_new_tab(scene::Document::new(), path.clone(), &[]);
    assert_eq!(opened, first);
    assert_eq!(tabs.len(), 1);
    assert_eq!(tabs.tab_with_project(&path), Some(first));
    // Another, beside it.
    let other = std::path::PathBuf::from("/work/barrel.avtl");
    let second = tabs.opened_in_new_tab(scene::Document::new(), other.clone(), &[]);
    assert_ne!(second, first);
    assert_eq!(tabs.len(), 2);
    assert_eq!(tabs.active().project_name(), "barrel.avtl");
    assert_eq!(tabs.tab_with_project(&other), Some(second));
    assert!(
        tabs.tab_with_project(std::path::Path::new("/nowhere.avtl"))
            .is_none()
    );
    harness.run();
    tab(&harness, "crate.avtl").click();
    harness.run();
    assert_eq!(harness.state().tabs.title(), "crate.avtl — Test App 9.9.9");
}

#[test]
fn the_window_title_and_the_dot_follow_the_tab_in_use() {
    let mut harness = harness();
    new_tab(&mut harness);
    add_cube(&mut harness);
    assert!(harness.state().tabs.title().starts_with("● Untitled 2"));
    // The dot on the tab with changes only.
    assert_eq!(harness.get_all_by_label("Unsaved changes").count(), 1);
    tab(&harness, "Untitled").click();
    harness.run();
    assert!(harness.state().tabs.title().starts_with("Untitled —"));
    assert_eq!(harness.state().tabs.unsaved_names(), ["Untitled 2"]);
}

#[test]
fn leaving_a_tab_backs_up_its_changes() {
    let mut harness = harness();
    let first = harness.state().tabs.active_id();
    add_cube(&mut harness);
    new_tab(&mut harness);
    harness.run();
    let backups: Vec<_> = harness
        .state()
        .actions
        .iter()
        .filter(|(_, a)| matches!(a, WorkbenchAction::Autosave { .. }))
        .collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(backups[0].0, first);
}

#[test]
fn the_tabs_sit_in_the_menu_bar_or_on_their_own_row() {
    let mut harness = harness();
    new_tab(&mut harness);
    let file = harness.get_by_label("File").rect();
    let help = harness.get_by_label("Help").rect();
    let standard = harness.get_by_label("Standard").rect();
    let first = tab(&harness, "Untitled").rect();
    // F, the standard: on the menu bar, between the menus and the layouts.
    assert!((first.center().y - file.center().y).abs() < 2.0);
    assert!(first.left() > help.right());
    assert!(harness.get_by_label(NEW_TAB).rect().right() < standard.left());
    // E: a row of their own above the menu bar.
    harness
        .state_mut()
        .tabs
        .active_mut()
        .viewport_mut()
        .options_mut()
        .tab_placement = TabPlacement::OwnRow;
    harness.run();
    let file = harness.get_by_label("File").rect();
    let first = tab(&harness, "Untitled").rect();
    assert!(first.bottom() <= file.top(), "{first:?} above {file:?}");
}

#[test]
fn many_tabs_shrink_and_then_scroll() {
    let mut harness = harness();
    for _ in 0..30 {
        new_tab(&mut harness);
    }
    assert_eq!(harness.state().tabs.len(), 31);
    let widths: Vec<f32> = ["Untitled", "Untitled 2"]
        .iter()
        .map(|n| tab(&harness, n).rect().width())
        .collect();
    assert!(
        widths.iter().all(|w| *w >= strip::TAB_MIN - 0.5),
        "{widths:?}"
    );
    // The tab in use (the newest) is in sight, left of the layouts.
    let standard = harness.get_by_label("Standard").rect();
    let newest = tab(&harness, "Untitled 31").rect();
    assert!(newest.right() <= standard.left(), "{newest:?}");
    assert!(harness.get_by_label(NEW_TAB).rect().right() < standard.left());
    // The mouse wheel over the row scrolls it back towards the first.
    harness.event(Event::PointerMoved(newest.center()));
    harness.run();
    for _ in 0..10 {
        harness.event(Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 3.0),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        });
        // Smooth scrolling keeps drawing: a few frames at a time.
        harness.run_steps(6);
    }
    let scrolled = tab(&harness, "Untitled 31").rect();
    assert!(scrolled.left() > newest.left() + 50.0, "{scrolled:?}");
}

#[test]
fn settings_choose_where_the_tabs_sit_and_what_new_does() {
    let mut harness = harness();
    let keys = crate::SETTINGS_SHORTCUT;
    harness.key_press_modifiers(keys.modifiers, keys.logical_key);
    harness.run();
    harness.get_by_label(crate::settings::PROJECTS_HEADING);
    harness.get_by_label("On their own row").click();
    harness.run();
    let options = harness.state().tabs.active().view_options();
    assert_eq!(options.tab_placement, TabPlacement::OwnRow);
    harness
        .get_by_label(crate::settings::OPEN_IN_NEW_TAB)
        .scroll_to_me();
    harness.run();
    harness
        .get_by_label(crate::settings::OPEN_IN_NEW_TAB)
        .click();
    harness.run();
    assert!(!harness.state().tabs.active().view_options().open_in_new_tab);
    harness.get_by_label("Restore standard").click();
    harness.run();
    let options = harness.state().tabs.active().view_options();
    assert_eq!(options.tab_placement, TabPlacement::InMenuBar);
    assert!(options.open_in_new_tab);
}

#[test]
fn the_choices_are_remembered_with_the_view_options() {
    for placement in TabPlacement::ALL {
        for open_in_new_tab in [false, true] {
            let options = ViewOptions {
                tab_placement: placement,
                open_in_new_tab,
                ..Default::default()
            };
            let text = ron::to_string(&options).unwrap();
            let back: ViewOptions = ron::from_str(&text).unwrap();
            assert_eq!(back.tab_placement, placement);
            assert_eq!(back.open_in_new_tab, open_in_new_tab);
        }
    }
    // Options saved before tabs: the standards.
    let old: ViewOptions = ron::from_str("(layout_switch: SmallTabs)").unwrap();
    assert_eq!(old.tab_placement, TabPlacement::InMenuBar);
    assert!(old.open_in_new_tab);
    // A value no version wrote is refused, never guessed at.
    assert!(ron::from_str::<ViewOptions>("(tab_placement: Sideways)").is_err());
    assert!(ron::from_str::<ViewOptions>("(open_in_new_tab: 3)").is_err());
}

#[test]
fn random_saved_options_never_panic() {
    // A fuzz test: whatever the settings file holds, reading the view
    // options gives options or an error, never a crash.
    let pieces = [
        "(",
        ")",
        ",",
        ":",
        "tab_placement",
        "open_in_new_tab",
        "InMenuBar",
        "OwnRow",
        "true",
        "false",
        "\"",
        "Some(",
        "[",
        "]",
        "-1",
        "1e99",
        "Own",
        " ",
    ];
    let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..3000 {
        let mut text = String::new();
        for _ in 0..(next() % 16) {
            text.push_str(pieces[(next() % pieces.len() as u64) as usize]);
        }
        if let Ok(options) = ron::from_str::<ViewOptions>(&text) {
            assert!(TabPlacement::ALL.contains(&options.tab_placement), "{text}");
        }
    }
}

#[test]
fn moving_and_switching_to_tabs_that_are_gone_does_nothing() {
    let mut harness = harness();
    let tabs = &mut harness.state_mut().tabs;
    assert!(!tabs.switch_to(99));
    tabs.move_tab(99, 0);
    tabs.close_tab(99);
    assert_eq!(tabs.len(), 1);
    // Moving past the end puts it last.
    tabs.add_tab();
    let ids = tabs.ids();
    tabs.move_tab(ids[0], 50);
    assert_eq!(tabs.ids(), [ids[1], ids[0]]);
    assert!(tabs.get(99).is_none());
    assert_eq!(
        tabs.get_or_active_mut(Some(99)).project_name(),
        "Untitled 2"
    );
}
