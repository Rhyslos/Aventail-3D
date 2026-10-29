//! The setting that puts the paint tools in the left bar (decision
//! 0213): floating along the viewport's bottom is the standard.

use super::paint_layers_tests::{painting_the_cube, press};
use crate::paint_tools::{COLUMN_LABEL, TOOLS_LABEL};
use egui_kittest::kittest::Queryable;
use paint::BrushKind;

#[test]
fn the_tools_float_as_standard_and_move_to_the_left_bar_when_set() {
    let mut harness = painting_the_cube();
    assert!(
        !harness
            .state()
            .workbench
            .viewport
            .options()
            .paint_tools_left
    );
    assert!(harness.query_by_label(TOOLS_LABEL).is_some());
    assert!(harness.query_by_label(COLUMN_LABEL).is_none());
    harness
        .state_mut()
        .workbench
        .viewport
        .options_mut()
        .paint_tools_left = true;
    harness.run();
    harness.run();
    assert!(harness.query_by_label(TOOLS_LABEL).is_none());
    assert!(harness.query_by_label(COLUMN_LABEL).is_some());
    press(&mut harness, "Smudge (5)");
    assert_eq!(
        harness.state().workbench.attributes.brush.brush.kind,
        BrushKind::Smudge
    );
    // Not in the Model mode.
    harness.state_mut().workbench.mode = crate::palette::Mode::Model;
    harness.run();
    harness.run();
    assert!(harness.query_by_label(COLUMN_LABEL).is_none());
}

#[test]
fn the_choice_is_remembered_with_the_view_options() {
    let options = crate::viewport::ViewOptions {
        paint_tools_left: true,
        ..Default::default()
    };
    let text = ron::to_string(&options).unwrap();
    let back: crate::viewport::ViewOptions = ron::from_str(&text).unwrap();
    assert!(back.paint_tools_left);
    // Options saved before the setting: floating, the standard.
    let old: crate::viewport::ViewOptions = ron::from_str("()").unwrap();
    assert!(!old.paint_tools_left);
}
