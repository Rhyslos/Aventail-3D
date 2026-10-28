//! Baking the texture set's maps from the Paint layers panel (decision
//! 0192): Bake maps bakes them on another thread, the panel shows them,
//! and says when they are out of date.

use super::paint_layers_tests::painting_the_cube;
use crate::paint_layers::bakes::BAKE_LABEL;
use bake::BakeKind;
use egui_kittest::kittest::Queryable;
use std::time::{Duration, Instant};

type H = egui_kittest::Harness<'static, super::tests::State>;

/// Runs frames until the maps are baked (or a minute passes).
fn wait_for_bakes(harness: &mut H) {
    let start = Instant::now();
    let done = |h: &H| {
        let bakes = &h.state().workbench.bakes;
        !bakes.busy() && bakes.map("Default", BakeKind::Occlusion).is_some()
    };
    while !done(harness) && start.elapsed() < Duration::from_secs(60) {
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }
    harness.step();
    harness.step();
}

#[test]
fn bake_maps_bakes_every_map_and_says_when_they_are_out_of_date() {
    let mut harness = painting_the_cube();
    // A small texture and a quick bake, for the test.
    {
        let doc = harness.state_mut().workbench.document_mut();
        let mut texture = doc.scene().texture();
        texture.size = 128;
        doc.execute(scene::Command::SetTexture(texture)).unwrap();
    }
    harness.state_mut().workbench.paint_layers.bakes.quality =
        crate::paint_layers::bakes::BakeQuality::Draft;
    harness.run();
    assert!(harness.query_by_label_contains("Not baked yet").is_some());
    harness.get_by_label(BAKE_LABEL).click();
    wait_for_bakes(&mut harness);
    let bakes = &harness.state().workbench.bakes;
    for kind in BakeKind::ALL {
        let map = bakes.map("Default", kind).expect("baked");
        assert_eq!(map.size(), 128);
        assert!(!map.is_empty(), "{kind:?}");
    }
    assert!(harness.query_by_label_contains("Baked at 128 px").is_some());
    assert_eq!(harness.state().workbench.paint_layers.bakes.shown.len(), 6);
    // A bigger texture: the maps are out of date.
    {
        let doc = harness.state_mut().workbench.document_mut();
        let mut texture = doc.scene().texture();
        texture.size = 256;
        doc.execute(scene::Command::SetTexture(texture)).unwrap();
    }
    harness.run();
    assert!(harness.query_by_label_contains("Out of date").is_some());
}
