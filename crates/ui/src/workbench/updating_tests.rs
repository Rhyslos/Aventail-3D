//! Tests for the update window and setting (decisions 0169 and 0172).

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

fn window_shown(harness: &H) -> bool {
    harness.query_by_label(RESTART_NOW).is_some()
}

#[test]
fn no_window_until_an_update_is_offered() {
    let harness = fresh();
    assert!(!window_shown(&harness));
}

#[test]
fn restart_now_asks_for_the_install_at_once() {
    let mut harness = fresh();
    harness.state_mut().workbench.offer_update(offer(412));
    harness.run();
    assert!(window_shown(&harness));
    assert!(
        harness
            .query_by_label_contains("A new version is ready")
            .is_some()
    );
    assert!(
        harness
            .query_by_label_contains("Build 412 is downloaded")
            .is_some()
    );
    assert!(harness.query_by_label("What's new in this build").is_some());
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
fn later_puts_the_window_away_until_a_newer_build() {
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
    assert!(!window_shown(&harness));
    let note = harness.state().workbench.note.clone().unwrap_or_default();
    assert!(note.contains("installs when you close"), "{note}");
    // The same build again (the next look): it stays away.
    harness.state_mut().workbench.offer_update(offer(412));
    harness.run();
    assert!(!window_shown(&harness));
    // A newer one: offered again.
    harness.state_mut().workbench.offer_update(offer(413));
    harness.run();
    assert!(window_shown(&harness));
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

#[test]
fn esc_is_later() {
    let mut harness = fresh();
    harness.state_mut().workbench.offer_update(offer(500));
    harness.run();
    assert!(window_shown(&harness));
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!window_shown(&harness));
    assert!(
        harness
            .state()
            .actions
            .contains(&WorkbenchAction::InstallUpdate { now: false })
    );
}

#[test]
fn skip_this_version_remembers_the_build() {
    let mut harness = fresh();
    harness.state_mut().workbench.offer_update(offer(600));
    harness.run();
    harness
        .get_by_label(crate::text::tr(super::SKIP_VERSION))
        .click();
    harness.run();
    harness.get_by_label(LATER).click();
    harness.run();
    assert!(!window_shown(&harness));
    assert_eq!(harness.state().workbench.skipped_update(), Some(600));
    assert!(
        harness
            .state()
            .actions
            .contains(&WorkbenchAction::SkipUpdate)
    );
    assert!(
        !harness
            .state()
            .actions
            .contains(&WorkbenchAction::InstallUpdate { now: false }),
        "a skipped build isn't installed at close"
    );
}

#[test]
fn settings_show_the_skipped_build_and_can_forget_it() {
    let mut harness = fresh();
    harness.state_mut().workbench.skip_update(Some(7));
    harness.state_mut().workbench.settings.open = true;
    harness.run();
    let ask = crate::text::tr(crate::settings::ASK_AGAIN);
    harness.get_by_label(ask).scroll_to_me();
    harness.run();
    assert!(
        harness
            .query_by_label_contains("Build 7 is skipped")
            .is_some()
    );
    harness.get_by_label(ask).click();
    harness.run();
    assert_eq!(harness.state().workbench.skipped_update(), None);
}

#[test]
fn the_splash_screens_question_answers_update_or_open() {
    use crate::splash::{OPEN_WITHOUT, UPDATE_NOW, UpdateAnswer, update_question};
    let run = |click: &'static str, tick_skip: bool| {
        // A harness of our own state: the box and the answer.
        let mut harness = egui_kittest::Harness::builder()
            .with_size([900.0, 600.0])
            .build_ui_state(
                |ui, state: &mut (bool, Option<UpdateAnswer>)| {
                    let ctx = ui.ctx().clone();
                    if let Some(a) = update_question(&ctx, 9, "https://example.com", &mut state.0) {
                        state.1 = Some(a);
                    }
                },
                (false, None),
            );
        harness.run();
        assert!(
            harness
                .query_by_label_contains("Build 9 is available")
                .is_some()
        );
        if tick_skip {
            harness
                .get_by_label(crate::text::tr(super::SKIP_VERSION))
                .click();
            harness.run();
        }
        harness.get_by_label(crate::text::tr(click)).click();
        harness.run();
        (harness.state().0, harness.state().1)
    };
    assert_eq!(run(UPDATE_NOW, false), (false, Some(UpdateAnswer::Update)));
    assert_eq!(run(OPEN_WITHOUT, false), (false, Some(UpdateAnswer::Open)));
    assert_eq!(run(OPEN_WITHOUT, true), (true, Some(UpdateAnswer::Open)));
}
