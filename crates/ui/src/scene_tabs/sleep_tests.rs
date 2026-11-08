//! Sleeping tabs (decision 0267), through the real interface without a
//! window: a tab left sleeps after the time chosen (at once with Straight
//! away or when memory is low, never with Never), never while in use or
//! while work it started is under way, wakes exactly as it was left
//! (scene, undo history, selection, view, mode, layout, unsaved changes),
//! shows a moon saying why it sleeps, and is still asked about when
//! closing or quitting.

use super::sleep::{Asleep, SleepReason};
use super::strip::ASLEEP;
use super::tests::{H, add_cube, harness, new_tab, objects, quits, tab};
use super::*;
use crate::palette::Mode;
use crate::viewport::{MemoryFreed, TabSleep, ViewOptions};
use egui::Key;
use egui_kittest::kittest::Queryable;
use std::time::{Duration, Instant};

/// What the stand-in renderer says it freed.
const GPU_FREED: MemoryFreed = MemoryFreed {
    gpu: 300 * 1024 * 1024,
    cpu: 0,
};

/// Asks the tabs to sleep as if it were `later` from now, with memory
/// `low` or not. Returns the tabs whose renderer was put to sleep, and
/// when to look again.
fn sleep_at(harness: &mut H, later: Duration, low: bool) -> (Vec<TabId>, Option<Duration>) {
    let mut slept = Vec::new();
    let wait = harness
        .state_mut()
        .tabs
        .sleep_tabs(Instant::now() + later, low, |id| {
            slept.push(id);
            GPU_FREED
        });
    harness.run();
    (slept, wait)
}

fn set_sleep(harness: &mut H, choice: TabSleep) {
    harness
        .state_mut()
        .tabs
        .active_mut()
        .viewport_mut()
        .options_mut()
        .tab_sleep = choice;
}

/// Everything a tab must wake with, as it was.
#[derive(Debug, PartialEq)]
struct Kept {
    scene: scene::Scene,
    /// How many things its drawing shows.
    drawn: usize,
    undo: usize,
    redo: usize,
    selection: Vec<scene::ObjectId>,
    mode: Mode,
    camera: tools::Camera,
    layout: crate::workbench::LayoutChoice,
    unsaved: bool,
    name: String,
}

fn kept(workbench: &Workbench) -> Kept {
    Kept {
        scene: workbench.document().scene().clone(),
        drawn: workbench.drawn().0,
        undo: workbench.document().undo_steps(),
        redo: workbench.document().redo_steps(),
        selection: workbench.document().selection().to_vec(),
        mode: workbench.mode(),
        camera: *workbench.viewport().camera(),
        layout: workbench.layout_choice(),
        unsaved: workbench.has_unsaved_changes(),
        name: workbench.project_name(),
    }
}

/// A first tab with two cubes (one undone, so there is a redo too), the
/// second selected, in the Edges mode; then a new tab in use.
fn left_with_work() -> (H, TabId, Kept) {
    let mut harness = harness();
    add_cube(&mut harness);
    add_cube(&mut harness);
    add_cube(&mut harness);
    harness.state_mut().tabs.active_mut().document_mut().undo();
    harness.run();
    harness.key_press(Key::Num2);
    harness.run();
    let first = harness.state().tabs.active_id();
    let before = kept(harness.state().tabs.active());
    assert_eq!(before.mode, Mode::Edges);
    assert!(before.unsaved && before.selection.len() == 1);
    new_tab(&mut harness);
    (harness, first, before)
}

#[test]
fn a_tab_left_sleeps_after_two_minutes_and_wakes_exactly_as_left() {
    let (mut harness, first, before) = left_with_work();
    assert_eq!(
        harness.state().tabs.active().view_options().tab_sleep,
        TabSleep::After2Minutes,
        "the standard"
    );
    // A minute on: still awake, due in about another minute.
    let (slept, wait) = sleep_at(&mut harness, Duration::from_secs(60), false);
    assert!(slept.is_empty());
    let wait = wait.expect("a tab will be due");
    assert!(wait > Duration::from_secs(50) && wait <= Duration::from_secs(60));
    assert!(!harness.state().tabs.is_asleep(first));
    // Two minutes on: asleep, its renderer and workbench let go.
    let (slept, wait) = sleep_at(&mut harness, Duration::from_secs(121), false);
    assert_eq!(slept, [first]);
    assert_eq!(wait, None, "nothing else to put to sleep");
    let tabs = &harness.state().tabs;
    assert!(tabs.is_asleep(first));
    assert!(!tabs.is_asleep(tabs.active_id()), "never the tab in use");
    assert!(tabs.get(first).unwrap().holds_nothing_worked_out());
    assert_eq!(tabs.freed_by_sleep(first).unwrap().gpu, GPU_FREED.gpu);
    // Nothing is lost while it sleeps (only the drawing is let go of).
    let asleep = kept(tabs.get(first).unwrap());
    assert_eq!(asleep.drawn, 0);
    assert_eq!(
        Kept {
            drawn: before.drawn,
            ..asleep
        },
        before
    );
    assert!(tabs.has_unsaved_changes());
    // Asleep already: not put to sleep again.
    let (slept, _) = sleep_at(&mut harness, Duration::from_secs(600), false);
    assert!(slept.is_empty());
    // Clicked, it wakes as it was left, and draws again.
    tab(&harness, "Untitled").click();
    harness.run();
    let tabs = &harness.state().tabs;
    assert_eq!(tabs.active_id(), first);
    assert!(!tabs.is_asleep(first));
    assert!(tabs.freed_by_sleep(first).is_none());
    // Including its drawing, worked out again.
    assert!(before.drawn > 0);
    assert_eq!(kept(tabs.active()), before);
    // Its undo history works: everything can be undone, then redone.
    let document = harness.state_mut().tabs.active_mut().document_mut();
    while document.undo_steps() > 0 {
        document.undo();
    }
    assert_eq!(document.redo_steps(), before.undo + before.redo);
    harness.run();
    assert_eq!(objects(&harness), 0);
}

#[test]
fn straight_away_sleeps_at_once_and_never_keeps_every_tab_awake() {
    let (mut harness, first, _) = left_with_work();
    set_sleep(&mut harness, TabSleep::Never);
    // Never: not even when memory runs low.
    let (slept, wait) = sleep_at(&mut harness, Duration::from_secs(3600), true);
    assert!(slept.is_empty() && wait.is_none());
    set_sleep(&mut harness, TabSleep::StraightAway);
    let (slept, _) = sleep_at(&mut harness, Duration::ZERO, false);
    assert_eq!(slept, [first]);
}

#[test]
fn low_memory_puts_every_tab_left_to_sleep_at_once() {
    let (mut harness, first, _) = left_with_work();
    new_tab(&mut harness);
    let second = harness.state().tabs.ids()[1];
    set_sleep(&mut harness, TabSleep::After30Minutes);
    let (slept, wait) = sleep_at(&mut harness, Duration::ZERO, false);
    assert!(slept.is_empty());
    assert!(wait.unwrap() > Duration::from_secs(29 * 60));
    let (mut slept, _) = sleep_at(&mut harness, Duration::ZERO, true);
    slept.sort_unstable();
    assert_eq!(slept, [first, second]);
    let tabs = &harness.state().tabs;
    let asleep = tabs.tabs.iter().find(|t| t.id == first).unwrap().asleep;
    assert_eq!(asleep.unwrap().reason, SleepReason::LowMemory);
}

#[test]
fn a_tab_whose_save_is_under_way_stays_awake_until_it_is_done() {
    let (mut harness, first, _) = left_with_work();
    harness
        .state_mut()
        .tabs
        .get_mut(first)
        .unwrap()
        .file_work_started(crate::FileWork::Saving("project".into()));
    let (slept, wait) = sleep_at(&mut harness, Duration::from_secs(600), true);
    assert!(slept.is_empty(), "a save under way isn't stopped");
    assert!(
        wait.unwrap() <= Duration::from_secs(2),
        "looked at again soon"
    );
    harness
        .state_mut()
        .tabs
        .get_mut(first)
        .unwrap()
        .end_file_work();
    let (slept, _) = sleep_at(&mut harness, Duration::from_secs(600), false);
    assert_eq!(slept, [first]);
}

#[test]
fn a_sleeping_tab_shows_a_moon_saying_why_and_that_nothing_is_lost() {
    let (mut harness, first, _) = left_with_work();
    assert!(harness.query_by_label(ASLEEP).is_none());
    sleep_at(&mut harness, Duration::from_secs(121), false);
    // One moon, on the first tab (left of the second).
    let moon = harness.get_by_label(ASLEEP).rect();
    assert!(moon.right() < tab(&harness, "Untitled 2").rect().left());
    let asleep = harness
        .state()
        .tabs
        .tabs
        .iter()
        .find(|t| t.id == first)
        .unwrap()
        .asleep
        .unwrap();
    let words = asleep.explanation(true);
    assert!(words.contains("more than 2 minutes ago"), "{words}");
    assert!(words.contains("300.0 MB"), "{words}");
    assert!(words.contains("unsaved changes are kept safe"), "{words}");
    // Woken, the moon goes.
    tab(&harness, "Untitled").click();
    harness.run();
    assert!(harness.query_by_label(ASLEEP).is_none());
}

#[test]
fn the_moon_says_why_for_every_reason() {
    let at = |reason| Asleep {
        reason,
        since: Instant::now(),
        freed: MemoryFreed {
            gpu: 2 * 1024 * 1024 * 1024,
            cpu: 0,
        },
    };
    let straight = at(SleepReason::Left(TabSleep::StraightAway)).explanation(false);
    assert!(
        straight.contains("You left it for another tab"),
        "{straight}"
    );
    assert!(straight.contains("2.00 GB"), "{straight}");
    assert!(straight.contains("Nothing is lost"), "{straight}");
    let later = at(SleepReason::Left(TabSleep::After30Minutes)).explanation(false);
    assert!(later.contains("30 minutes"), "{later}");
    let low = at(SleepReason::LowMemory).explanation(true);
    assert!(low.contains("low on memory"), "{low}");
    assert!(low.contains("unsaved changes"), "{low}");
}

#[test]
fn quitting_with_a_sleeping_unsaved_tab_still_asks_and_wakes_it() {
    let (mut harness, first, before) = left_with_work();
    sleep_at(&mut harness, Duration::from_secs(121), false);
    assert!(harness.state().tabs.is_asleep(first));
    assert!(!harness.state_mut().tabs.request_close());
    harness.run();
    // Its own question, in its own tab, which is awake again.
    let tabs = &harness.state().tabs;
    assert_eq!(tabs.active_id(), first);
    assert!(!tabs.is_asleep(first));
    assert_eq!(kept(tabs.active()), before);
    harness.get_by_label_contains("before closing.");
    assert_eq!(quits(&harness), 0);
    harness.get_by_label("Don't save").click();
    harness.run();
    assert_eq!(quits(&harness), 1);
}

#[test]
fn closing_a_sleeping_tab_with_changes_asks_first() {
    let (mut harness, first, _) = left_with_work();
    sleep_at(&mut harness, Duration::from_secs(121), false);
    harness.state_mut().tabs.close_tab(first);
    harness.run();
    assert_eq!(harness.state().tabs.len(), 2, "asked, not closed");
    assert_eq!(harness.state().tabs.active_id(), first);
    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(harness.state().tabs.len(), 2);
    assert!(harness.state().tabs.active().has_unsaved_changes());
}

/// Picks `choice` in Settings' Tabs sleep list.
fn choose_sleep(harness: &mut H, choice: &str) {
    let label = crate::settings::TABS_SLEEP;
    harness
        .get_by_label_contains(&format!("{label}: "))
        .scroll_to_me();
    harness.run();
    harness.get_by_label_contains(&format!("{label}: ")).click();
    harness.run();
    harness.get_by_label(choice).click();
    harness.run();
}

#[test]
fn settings_choose_when_tabs_sleep() {
    let mut harness = harness();
    let keys = crate::SETTINGS_SHORTCUT;
    harness.key_press_modifiers(keys.modifiers, keys.logical_key);
    harness.run();
    choose_sleep(&mut harness, "Straight away");
    let options = harness.state().tabs.active().view_options();
    assert_eq!(options.tab_sleep, TabSleep::StraightAway);
    // Shared by every tab: a new tab has the choice too.
    choose_sleep(&mut harness, "After 30 minutes");
    let options = harness.state().tabs.active().view_options();
    assert_eq!(options.tab_sleep, TabSleep::After30Minutes);
    harness.get_by_label("Restore standard").click();
    harness.run();
    let options = harness.state().tabs.active().view_options();
    assert_eq!(options.tab_sleep, TabSleep::After2Minutes);
    harness
        .state_mut()
        .tabs
        .active_mut()
        .viewport_mut()
        .options_mut()
        .tab_sleep = TabSleep::Never;
    harness.state_mut().tabs.add_tab();
    harness.run();
    assert_eq!(
        harness.state().tabs.active().view_options().tab_sleep,
        TabSleep::Never
    );
}

#[test]
fn the_choice_is_remembered_with_the_view_options() {
    for choice in TabSleep::ALL {
        let options = ViewOptions {
            tab_sleep: choice,
            ..Default::default()
        };
        let text = ron::to_string(&options).unwrap();
        let back: ViewOptions = ron::from_str(&text).unwrap();
        assert_eq!(back.tab_sleep, choice);
    }
    // Options saved before sleeping tabs: the standard.
    let old: ViewOptions = ron::from_str("(open_in_new_tab: false)").unwrap();
    assert_eq!(old.tab_sleep, TabSleep::After2Minutes);
    assert!(ron::from_str::<ViewOptions>("(tab_sleep: Sometimes)").is_err());
    assert!(ron::from_str::<ViewOptions>("(tab_sleep: 2)").is_err());
    // Only Never never sleeps, and the delays grow.
    let delays: Vec<_> = TabSleep::ALL.iter().map(|c| c.delay()).collect();
    assert_eq!(delays[0], Some(Duration::ZERO));
    assert!(delays.windows(2).take(3).all(|w| w[0] < w[1]));
    assert_eq!(delays[4], None);
}

#[test]
fn random_saved_sleep_choices_never_panic() {
    // A fuzz test: whatever the settings file holds, reading the view
    // options gives options or an error, never a crash.
    let pieces = [
        "(",
        ")",
        ",",
        ":",
        "tab_sleep",
        "StraightAway",
        "After2Minutes",
        "After10Minutes",
        "After30Minutes",
        "Never",
        "Some(",
        "\"",
        "1",
        " ",
        "After",
    ];
    let mut seed: u64 = 0x1234_5678_9abc_def1;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..3000 {
        let mut text = String::new();
        for _ in 0..(next() % 12) {
            text.push_str(pieces[(next() % pieces.len() as u64) as usize]);
        }
        if let Ok(options) = ron::from_str::<ViewOptions>(&text) {
            assert!(TabSleep::ALL.contains(&options.tab_sleep), "{text}");
        }
    }
}
