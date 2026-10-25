//! Tests for the update bar and setting (decision 0169).

use super::tests::{State, fresh};
use super::updating::{LATER, RESTART_NOW};
use super::{UpdateOffer, WorkbenchAction};
use crate::settings::{CHECK_FOR_UPDATES, UPDATES};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

type H = Harness<'static, State>;

fn offer(build: u32) -> UpdateOffer {
    UpdateOffer {
        build,
        notes_url: format!("https://github.com/Rhyslos/Aventail-3D/releases/tag/build-{build}"),
    }
}

fn bar_shown(harness: &H) -> bool {
    harness.query_by_label(RESTART_NOW).is_some()
}

#[test]
fn no_bar_until_an_update_is_offered() {
    let harness = fresh();
    assert!(!bar_shown(&harness));
}

#[test]
fn restart_now_asks_for_the_install_at_once() {
    let mut harness = fresh();
    harness.state_mut().workbench.offer_update(offer(412));
    harness.run();
    assert!(bar_shown(&harness));
    assert!(
        harness
            .query_by_label_contains("A new version is ready (build 412)")
            .is_some()
    );
    assert!(harness.query_by_label("What's new").is_some());
    harness.get_by_label(RESTART_NOW).click();
    harness.run();
    assert!(
        harness
            .state()
            .actions
            .contains(&WorkbenchAction::InstallUpdate { now: true })
    );
}

#[test]
fn later_puts_the_bar_away_until_a_newer_build() {
    let mut harness = fresh();
    harness.state_mut().workbench.offer_update(offer(412));
    harness.run();
    harness.get_by_label(LATER).click();
    harness.run();
    assert!(
        harness
            .state()
            .actions
            .contains(&WorkbenchAction::InstallUpdate { now: false })
    );
    assert!(!bar_shown(&harness));
    let note = harness.state().workbench.note.clone().unwrap_or_default();
    assert!(note.contains("installs when you close"), "{note}");
    // The same build again (the next look): it stays away.
    harness.state_mut().workbench.offer_update(offer(412));
    harness.run();
    assert!(!bar_shown(&harness));
    // A newer one: offered again.
    harness.state_mut().workbench.offer_update(offer(413));
    harness.run();
    assert!(bar_shown(&harness));
}

#[test]
fn settings_switch_looking_off_and_say_what_it_found() {
    let mut harness = fresh();
    assert!(
        harness.state().workbench.checks_for_updates(),
        "on as standard"
    );
    harness
        .state_mut()
        .workbench
        .set_update_status("Up to date: build 412 is the newest.".into());
    harness.state_mut().workbench.settings.open = true;
    harness.run();
    assert!(harness.query_by_label(crate::text::tr(UPDATES)).is_some());
    assert!(
        harness
            .query_by_label_contains("Up to date: build 412")
            .is_some()
    );
    // Low on the Interface page: scrolled to first.
    harness
        .get_by_label(crate::text::tr(CHECK_FOR_UPDATES))
        .scroll_to_me();
    harness.run();
    harness
        .get_by_label(crate::text::tr(CHECK_FOR_UPDATES))
        .click();
    harness.run();
    assert!(!harness.state().workbench.checks_for_updates());
}
