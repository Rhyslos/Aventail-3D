//! Tests for the Edges switch (decision 0220): off as standard, ticked by
//! the Points, Edges and Polygons modes, unticked again on leaving them
//! unless changed by hand, and reaching the renderer.

use super::tests::{State, empty};
use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn press(harness: &mut Harness<'static, State>, key: Key) {
    harness.key_press(key);
    harness.run();
}

fn shown(harness: &Harness<'static, State>) -> bool {
    harness.state().workbench.viewport().options().show_edges
}

/// Whether the last frame drawn asked for edges.
fn drawn_with_edges(harness: &Harness<'static, State>) -> bool {
    harness
        .state()
        .renderer
        .drawn
        .last()
        .expect("drawn")
        .show_edges
}

#[test]
fn edges_are_off_until_a_mode_that_edits_parts_ticks_them() {
    let mut harness = empty();
    assert!(!shown(&harness) && !drawn_with_edges(&harness));
    for key in [Key::Num1, Key::Num2, Key::Num3] {
        press(&mut harness, key);
        assert!(shown(&harness), "{key:?} ticks the switch");
    }
    assert!(drawn_with_edges(&harness));
    // Pressed again: back to the Model mode, and the switch off again.
    press(&mut harness, Key::Num3);
    assert_eq!(harness.state().workbench.mode(), Mode::Model);
    assert!(!shown(&harness));
}

#[test]
fn a_switch_changed_by_hand_is_left_as_it_is() {
    let mut harness = empty();
    // Already on: leaving Edges mode keeps it on.
    harness
        .state_mut()
        .workbench
        .viewport_mut()
        .options_mut()
        .show_edges = true;
    press(&mut harness, Key::Num2);
    press(&mut harness, Key::Num2);
    assert_eq!(harness.state().workbench.mode(), Mode::Model);
    assert!(shown(&harness), "was on before");
    // Ticked by the mode, then unticked by hand: it stays off there and
    // after.
    harness
        .state_mut()
        .workbench
        .viewport_mut()
        .options_mut()
        .show_edges = false;
    press(&mut harness, Key::Num2);
    assert!(shown(&harness));
    harness
        .state_mut()
        .workbench
        .viewport_mut()
        .options_mut()
        .show_edges = false;
    harness.run();
    assert!(!shown(&harness), "not ticked again while in the mode");
    // Ticked by hand again in the mode: it's theirs, kept on leaving.
    harness
        .state_mut()
        .workbench
        .viewport_mut()
        .options_mut()
        .show_edges = true;
    harness.run();
    press(&mut harness, Key::Num2);
    assert!(shown(&harness));
}

#[test]
fn a_switch_ticked_by_a_mode_isnt_saved_as_on() {
    let mut harness = empty();
    press(&mut harness, Key::Num2);
    assert!(shown(&harness));
    assert!(!harness.state().workbench.view_options().show_edges);
}

#[test]
fn the_shading_list_has_the_edges_switch() {
    let mut harness = empty();
    harness.get_by_label(crate::viewport::SHADING_LABEL).click();
    harness.run();
    harness
        .get_by_label(crate::viewport::look::EDGES_LABEL)
        .click();
    harness.run();
    assert!(shown(&harness) && harness.state().workbench.view_options().show_edges);
    assert!(drawn_with_edges(&harness));
    // Only two looks are left: Shaded and Wireframe.
    assert_eq!(crate::Shading::ALL.len(), 2);
}

#[test]
fn an_old_shaded_with_wires_setting_opens_as_shaded() {
    // Settings saved before decision 0220 can hold the look it replaced.
    let old: crate::ViewOptions = ron::from_str("(shading: ShadedWithWires)").unwrap();
    assert_eq!(old.shading, crate::Shading::Shaded);
    assert!(!old.show_edges);
    // And the switch is remembered.
    let saved = ron::to_string(&crate::ViewOptions {
        show_edges: true,
        ..Default::default()
    })
    .unwrap();
    let read: crate::ViewOptions = ron::from_str(&saved).unwrap();
    assert!(read.show_edges);
    // Rubbish in the file is refused, not taken for a look.
    assert!(ron::from_str::<crate::ViewOptions>("(shading: Wires)").is_err());
}
