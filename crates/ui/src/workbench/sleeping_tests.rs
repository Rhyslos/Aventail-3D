//! A workbench going to sleep and waking (decision 0267): what it worked
//! out is let go of and worked out again the same, background work stops
//! and starts again, and work the person started keeps the tab awake.

use super::paint_graph_tests::{add_graph_fill, moss_on_top, top_fill, wait, with_graph};
use super::paint_layers_tests::painting_the_cube;
use super::sleeping::StaysAwake;
use super::*;
use egui_kittest::kittest::Queryable;

#[test]
fn asleep_the_drawing_and_paint_mixes_are_let_go_of_and_awake_made_the_same() {
    let mut harness = painting_the_cube();
    // Some paint on the cube's texture.
    let doc = harness.state_mut().workbench.document_mut();
    let mut image = paint::Image::new(64).unwrap();
    for x in 0..64 {
        image.set_pixel(x, x, [200, 40, 40, 255]);
    }
    let mut set = paint::TextureSet::new(64).unwrap();
    set.layers.push(paint::Layer::paint("Lines", image));
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    harness.run();
    let (items, textures) = harness.state().workbench.drawn();
    assert_eq!(items, 1);
    assert!(!textures.is_empty(), "the cube's paint is mixed");
    let document = harness.state().workbench.document().scene().clone();
    let freed = harness.state_mut().workbench.sleep();
    assert!(freed.cpu > 0, "the mixes took memory: {freed:?}");
    let workbench = &harness.state().workbench;
    assert!(workbench.is_asleep());
    assert!(workbench.holds_nothing_worked_out());
    assert_eq!(workbench.drawn().0, 0);
    // Nothing the person made is touched.
    assert_eq!(workbench.document().scene(), &document);
    // Shown again, it wakes and draws what it drew before.
    harness.run();
    let workbench = &harness.state().workbench;
    assert!(!workbench.is_asleep());
    let (items_again, textures_again) = workbench.drawn();
    assert_eq!(items_again, items);
    assert_eq!(textures_again.len(), textures.len());
    for (a, b) in textures.iter().zip(&textures_again) {
        assert_eq!(a.key, b.key);
        assert_eq!(a.image.to_rgba(), b.image.to_rgba(), "the same pixels");
    }
}

#[test]
fn a_graph_fill_under_way_stops_asleep_and_is_made_on_waking() {
    let (mut harness, id) = with_graph(moss_on_top());
    add_graph_fill(&mut harness, id);
    // One frame starts making it; sleeping stops it (unless it was
    // already quick enough to land, which a busy machine can't promise).
    harness.step();
    harness.state_mut().workbench.sleep();
    assert!(!harness.state().workbench.graph_fills.busy(), "stopped");
    assert!(harness.state().workbench.holds_nothing_worked_out());
    let made_asleep = top_fill(&harness).made;
    // Nothing more lands while asleep, however long it waits.
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert_eq!(top_fill(&harness).made, made_asleep);
    // Awake, it is started again (when it was stopped) and made.
    wait(&mut harness);
    assert_ne!(top_fill(&harness).made, 0);
}

#[test]
fn sleeping_twice_or_waking_an_awake_workbench_changes_nothing() {
    let mut harness = painting_the_cube();
    harness.state_mut().workbench.sleep();
    let again = harness.state_mut().workbench.sleep();
    assert_eq!(again.cpu, 0, "nothing left to free");
    harness.state_mut().workbench.wake();
    harness.state_mut().workbench.wake();
    harness.run();
    assert_eq!(harness.state().workbench.drawn().0, 1);
}

#[test]
fn work_the_person_started_keeps_the_tab_awake() {
    let mut harness = painting_the_cube();
    assert_eq!(harness.state().workbench.stays_awake(), None);
    // Saving.
    harness
        .state_mut()
        .workbench
        .file_work_started(FileWork::Saving("project".into()));
    assert_eq!(
        harness.state().workbench.stays_awake(),
        Some(StaysAwake::FileWork)
    );
    harness.state_mut().workbench.file_work_done();
    assert_eq!(harness.state().workbench.stays_awake(), None);
    // Bake maps, at full size and Best: slow enough to catch under way.
    harness.state_mut().workbench.paint_layers.bakes.quality =
        crate::paint_layers::bakes::BakeQuality::Best;
    harness.step();
    harness
        .get_by_label(crate::paint_layers::bakes::BAKE_LABEL)
        .click();
    harness.step();
    harness.step();
    assert!(harness.state().workbench.bakes.busy());
    assert_eq!(
        harness.state().workbench.stays_awake(),
        Some(StaysAwake::BakingMaps)
    );
    harness
        .get_by_label(crate::paint_layers::bakes::BAKE_CANCEL_LABEL)
        .click();
    harness.step();
    harness.step();
    assert_eq!(harness.state().workbench.stays_awake(), None);
}
