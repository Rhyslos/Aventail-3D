//! Outlines (decision 0201): switched on in the Attributes panel, with
//! their width shown while on, sent to the renderer, one undo step.

use super::modelling_tests::{H, cube};
use super::tests::fresh;
use crate::attributes::outline_rows::OUTLINE_LABEL;
use egui_kittest::kittest::Queryable;

fn outline(harness: &H) -> Option<scene::Outline> {
    let doc = harness.state().workbench.document();
    doc.scene().object(cube(harness)).unwrap().outline
}

#[test]
fn an_outline_is_switched_on_in_attributes_and_drawn() {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    assert!(harness.query_by_label("Outline width").is_none());
    harness.get_by_label(OUTLINE_LABEL).scroll_to_me();
    harness.run();
    harness.get_by_label(OUTLINE_LABEL).click();
    harness.run();
    harness.run();
    assert_eq!(outline(&harness), Some(scene::Outline::default()));
    assert!(harness.query_by_label("Outline width").is_some());
    // The renderer is asked to draw it.
    let view = harness.state().renderer.drawn.last().unwrap().clone();
    assert_eq!(
        view.scene.items[0].toon_outline,
        Some(scene::Outline::default())
    );
    // One step to undo.
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Change an outline")
    );
    harness.state_mut().workbench.document_mut().undo().unwrap();
    harness.run();
    assert_eq!(outline(&harness), None);
}
