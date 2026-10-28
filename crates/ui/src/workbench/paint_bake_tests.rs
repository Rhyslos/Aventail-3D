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

/// The cube, painting, with its maps baked at 128 px.
fn baked() -> H {
    let mut harness = painting_the_cube();
    {
        let doc = harness.state_mut().workbench.document_mut();
        let mut texture = doc.scene().texture();
        texture.size = 128;
        doc.execute(scene::Command::SetTexture(texture)).unwrap();
    }
    harness.state_mut().workbench.paint_layers.bakes.quality =
        crate::paint_layers::bakes::BakeQuality::Draft;
    harness.run();
    harness.get_by_label(BAKE_LABEL).click();
    wait_for_bakes(&mut harness);
    harness
}

#[test]
fn a_map_clicked_shows_on_the_model_as_a_guide_until_clicked_again() {
    let mut harness = baked();
    let revision = harness.state().workbench.bakes.guides.revision;
    harness.state_mut().workbench.paint_layers.bakes.toggled = Some(BakeKind::Occlusion);
    harness.run();
    let bakes = &harness.state().workbench.bakes;
    assert!(bakes.guides.revision > revision);
    let guide = bakes.guides.by_material.get("Default").expect("a guide");
    assert_eq!(guide.blend, paint::Blend::Multiply);
    assert!(
        harness
            .query_by_label_contains("Shown on the model: Occlusion")
            .is_some()
    );
    // The same map again: no guide.
    harness.state_mut().workbench.paint_layers.bakes.toggled = Some(BakeKind::Occlusion);
    harness.run();
    assert!(
        harness
            .state()
            .workbench
            .bakes
            .guides
            .by_material
            .is_empty()
    );
}

#[test]
fn a_map_becomes_a_layer_or_the_chosen_layers_mask_in_one_step() {
    let mut harness = baked();
    harness.state_mut().workbench.paint_layers.bakes.toggled = Some(BakeKind::Curvature);
    harness.run();
    // The panel has more rows above the Bakes section now (starting
    // layouts and presets): bring the button into view first.
    harness
        .get_by_label(crate::paint_layers::bakes::ADD_LAYER_LABEL)
        .scroll_to_me();
    harness.run();
    harness
        .get_by_label(crate::paint_layers::bakes::ADD_LAYER_LABEL)
        .click();
    harness.run();
    let set = |h: &H| {
        h.state()
            .workbench
            .document()
            .scene()
            .paint()
            .set("Default")
            .map(|s| (**s).clone())
            .expect("a texture set")
    };
    let with_layer = set(&harness);
    let top = with_layer.layers.last().unwrap();
    assert_eq!(top.name, "Curvature");
    assert_eq!(top.blend, paint::Blend::Overlay);
    assert_eq!(top.image().unwrap().size(), 128);
    // A texture set made since the bake doesn't make the maps out of date
    // (regression: they were, once the set existed).
    assert!(harness.query_by_label_contains("Baked at 128 px").is_some());
    // As the chosen layer's mask: the new layer is chosen. (The panel is
    // longer now, its button out of the test window's sight: asked
    // directly.)
    assert!(
        harness
            .query_by_label(crate::paint_layers::bakes::AS_MASK_LABEL)
            .is_some()
    );
    harness.state_mut().workbench.paint_layers.bakes.as_mask = true;
    harness.run();
    let masked = set(&harness);
    let mask = masked.layers.last().unwrap().mask.as_ref().expect("a mask");
    assert_eq!(&mask.image, top.image().unwrap());
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert!(set(&harness).layers.last().unwrap().mask.is_none());
}
