//! The live engine preview (decision 0212): with the game project
//! folder's live switch on, a change to the painting goes out once it
//! has rested, not mid-stroke and not again while unchanged.

use super::WorkbenchAction;
use super::paint_layers_tests::{painting_the_cube, press};
use std::time::Duration;

type H = egui_kittest::Harness<'static, super::tests::State>;

fn live_exports(harness: &H) -> usize {
    harness
        .state()
        .actions
        .iter()
        .filter(|a| matches!(a, WorkbenchAction::ExportToTarget(_)))
        .count()
}

fn set_target(harness: &mut H, live: bool) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetExportTarget(Some(scene::ExportTarget {
            folder: std::env::temp_dir().display().to_string(),
            on_save: false,
            live,
        })))
        .unwrap();
}

#[test]
fn painting_goes_out_once_it_rests_and_only_when_it_changed() {
    let mut harness = painting_the_cube();
    harness.state_mut().workbench.live.delay = Some(Duration::ZERO);
    set_target(&mut harness, false);
    press(&mut harness, "+ Fill");
    harness.state_mut().actions.clear();
    harness.run();
    assert_eq!(live_exports(&harness), 0, "off: nothing goes out");
    // On: the painting goes out.
    set_target(&mut harness, true);
    harness.run();
    harness.run();
    assert_eq!(live_exports(&harness), 1);
    // Unchanged: not again.
    harness.run();
    harness.run();
    assert_eq!(live_exports(&harness), 1);
    // A change goes out again.
    press(&mut harness, "+ Fill");
    harness.run();
    assert_eq!(live_exports(&harness), 2);
}

#[test]
fn a_change_waits_for_the_painting_to_rest() {
    let mut harness = painting_the_cube();
    harness.state_mut().workbench.live.delay = Some(Duration::from_secs(3600));
    set_target(&mut harness, true);
    press(&mut harness, "+ Fill");
    harness.run();
    assert_eq!(live_exports(&harness), 0, "not before it rests");
}

#[test]
fn the_export_window_has_the_live_switch() {
    let mut harness = painting_the_cube();
    set_target(&mut harness, false);
    harness.state_mut().workbench.exporting.open = true;
    harness.state_mut().workbench.exporting.section = super::exporting::ExportSection::GameProject;
    harness.run();
    press(&mut harness, super::exporting::LIVE_LABEL);
    let target = harness
        .state()
        .workbench
        .document()
        .scene()
        .export_target()
        .unwrap()
        .clone();
    assert!(target.live);
}
