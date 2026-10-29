//! Showing one channel from the Paint tab (decision 0213).

use super::paint_layers_tests::{painting_the_cube, press};
use crate::attributes::channel_view::{CHANNELS_LABEL, ChannelView};
use crate::viewport::PaintPreview;
use egui_kittest::kittest::Queryable;

type H = egui_kittest::Harness<'static, super::tests::State>;

fn preview(harness: &H) -> PaintPreview {
    harness.state().workbench.viewport.options().paint_preview
}

fn with_realism(harness: &mut H) {
    let doc = harness.state_mut().workbench.document_mut();
    let size = doc.scene().texture().size;
    let mut set = paint::TextureSet::new(size).unwrap();
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
fn colour_shows_the_colours_alone_and_all_goes_back() {
    let mut harness = painting_the_cube();
    harness
        .state_mut()
        .workbench
        .viewport
        .options_mut()
        .paint_preview = PaintPreview::Godot;
    harness.run();
    assert!(harness.query_by_label(CHANNELS_LABEL).is_some());
    // Without the realism channels: All and Colour only.
    assert!(harness.query_by_label("Show Rough").is_none());
    press(&mut harness, "Show Colour");
    assert_eq!(preview(&harness), PaintPreview::Unlit);
    assert_eq!(
        harness.state().workbench.attributes.channel_shown,
        ChannelView::Colour
    );
    press(&mut harness, "Show All");
    assert_eq!(preview(&harness), PaintPreview::Godot, "the look before");
}

#[test]
fn the_realism_channels_each_show_on_their_own() {
    let mut harness = painting_the_cube();
    with_realism(&mut harness);
    for (label, shows) in [
        ("Show Rough", PaintPreview::Roughness),
        ("Show Metal", PaintPreview::Metal),
        ("Show Height", PaintPreview::Height),
        ("Show Glow", PaintPreview::Glow),
    ] {
        press(&mut harness, label);
        assert_eq!(preview(&harness), shows, "{label}");
    }
    press(&mut harness, "Show All");
    assert_eq!(preview(&harness), PaintPreview::Lit);
}

#[test]
fn choosing_a_look_in_the_shading_button_shows_all_again() {
    let mut harness = painting_the_cube();
    with_realism(&mut harness);
    press(&mut harness, "Show Height");
    harness
        .state_mut()
        .workbench
        .viewport
        .options_mut()
        .paint_preview = PaintPreview::Toon;
    harness.run();
    harness.run();
    assert_eq!(
        harness.state().workbench.attributes.channel_shown,
        ChannelView::All
    );
}
