//! Fixes from the texture painter review (2026-10-30), room and smaller
//! ones: the paint previews offered, the realism switch whatever the
//! tool, and the Realistic start's preview.

use super::paint_layers_tests::{painting_the_cube, press};
use crate::viewport::PaintPreview;

type H = egui_kittest::Harness<'static, super::tests::State>;

fn realism_on(harness: &mut H) {
    let doc = harness.state_mut().workbench.document_mut();
    let mut set = doc
        .scene()
        .paint()
        .set("Default")
        .map(|s| (**s).clone())
        .unwrap_or_else(|| paint::TextureSet::new(64).unwrap());
    set.realism = true;
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    harness.run();
    harness.run();
}

#[test]
fn the_channel_previews_are_offered_only_with_realism() {
    let mut harness = painting_the_cube();
    assert!(!harness.state().workbench.viewport.realism);
    realism_on(&mut harness);
    assert!(harness.state().workbench.viewport.realism);
    assert!(PaintPreview::Roughness.is_channel());
    assert!(!PaintPreview::Realistic.is_channel());
}

#[test]
fn the_realism_switch_is_noticed_with_the_move_tool() {
    // A review: with the select or move tool chosen, the shelf and Paint
    // tab missed the realism switch.
    let mut harness = painting_the_cube();
    harness.state_mut().workbench.tool = crate::transforming::Tool::Move;
    harness.run();
    realism_on(&mut harness);
    assert!(harness.state().workbench.attributes.realism);
}

#[test]
fn the_realistic_start_shows_the_realistic_preview() {
    let mut harness = painting_the_cube();
    press(&mut harness, "Realistic");
    harness.run();
    assert_eq!(
        harness.state().workbench.viewport.options().paint_preview,
        PaintPreview::Realistic
    );
}
