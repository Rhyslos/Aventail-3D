//! Realism channels (decision 0207): the switch, painting roughness and
//! metal with the colour, erasing them, fill layers' channels, the
//! realistic previews, and what the views get.

use super::brushing_tests::stroke;
use super::paint_layers_tests::{painting_the_cube, press};
use super::tests::camera;
use crate::attributes::PaintTool;
use egui::{Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use paint::{BrushKind, Channel, ChannelPart, TextureSet};

type H = egui_kittest::Harness<'static, super::tests::State>;

fn on_screen(harness: &H, world: Vec3) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let at = camera(harness).project(world, size).unwrap();
    rect.min + egui::vec2(at.x, at.y)
}

fn set(harness: &H) -> TextureSet {
    let doc = harness.state().workbench.document();
    (**doc.scene().paint().set("Default").unwrap()).clone()
}

fn paint_front(harness: &mut H) {
    let from = on_screen(harness, Vec3::new(-20.0, -50.0, 50.0));
    let to = on_screen(harness, Vec3::new(20.0, -50.0, 50.0));
    stroke(harness, from, to, Modifiers::NONE, PointerButton::Primary);
}

/// Paint mode on the cube with the realism channels on.
fn realism_on() -> H {
    let mut harness = painting_the_cube();
    press(&mut harness, crate::paint_layers::REALISM_LABEL);
    harness
}

/// How many texels of the top layer's `channel` are painted.
fn painted(set: &TextureSet, channel: Channel) -> usize {
    let top = set.layers.last().unwrap();
    match top.channels.get(channel) {
        Some(ChannelPart::Paint(image)) => {
            image.to_rgba().chunks_exact(4).filter(|p| p[3] > 0).count()
        }
        _ => 0,
    }
}

#[test]
fn the_switch_turns_the_channels_on_in_one_step() {
    let mut harness = realism_on();
    assert!(set(&harness).realism);
    assert!(set(&harness).layers.is_empty(), "kept with no layers");
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_label(), Some("Turn the realism channels on"));
    assert!(
        harness.query_by_label("Roughness").is_some(),
        "the brush rows"
    );
    harness.state_mut().workbench.document_mut().undo().unwrap();
    harness.run();
    let doc = harness.state().workbench.document();
    assert!(doc.scene().paint().set("Default").is_none());
}

#[test]
fn a_stroke_paints_roughness_and_metal_with_the_colour() {
    let mut harness = realism_on();
    {
        let channels = &mut harness.state_mut().workbench.attributes.brush.channels;
        channels[0].1 = [40, 40, 40, 255];
    }
    paint_front(&mut harness);
    let after = set(&harness);
    let colour = after
        .layers
        .last()
        .unwrap()
        .image()
        .unwrap()
        .painted_tiles();
    assert!(colour > 0);
    let rough = painted(&after, Channel::Roughness);
    assert!(rough > 0, "roughness painted");
    assert_eq!(rough, painted(&after, Channel::Metal), "the same texels");
    assert_eq!(painted(&after, Channel::Glow), 0, "glow isn't ticked");
    // The value painted.
    let top = after.layers.last().unwrap();
    let Some(ChannelPart::Paint(image)) = top.channels.get(Channel::Roughness) else {
        panic!("painted roughness");
    };
    let solid = image
        .to_rgba()
        .chunks_exact(4)
        .find(|p| p[3] == 255)
        .map(|p| p[0]);
    assert!(solid.is_some_and(|v| v.abs_diff(40) <= 1), "{solid:?}");
    // The views get the packed roughness and metal.
    let drawing = harness.state().renderer.drawn.last().unwrap().scene.clone();
    let maps = drawing.textures[0].realism.as_ref().expect("realism maps");
    assert!(
        maps.surface
            .to_rgba()
            .chunks_exact(4)
            .any(|p| p[0].abs_diff(40) <= 1 && p[1] == 0)
    );
}

#[test]
fn unticked_channels_and_hand_painted_sets_are_left_alone() {
    let mut harness = painting_the_cube();
    paint_front(&mut harness);
    let plain = set(&harness);
    assert!(
        plain.layers.last().unwrap().channels.is_empty(),
        "no realism"
    );
    let mut harness = realism_on();
    harness.state_mut().workbench.attributes.brush.channels = [(false, [0; 4]); 4];
    paint_front(&mut harness);
    assert!(set(&harness).layers.last().unwrap().channels.is_empty());
}

#[test]
fn the_eraser_takes_the_channels_away_too() {
    let mut harness = realism_on();
    paint_front(&mut harness);
    let before = painted(&set(&harness), Channel::Metal);
    harness.state_mut().workbench.attributes.brush.brush.kind = BrushKind::Erase;
    harness.state_mut().workbench.attributes.brush.brush.radius = 80.0;
    paint_front(&mut harness);
    let after = set(&harness);
    let solid_metal = match after.layers.last().unwrap().channels.get(Channel::Metal) {
        Some(ChannelPart::Paint(image)) => image
            .to_rgba()
            .chunks_exact(4)
            .filter(|p| p[3] == 255)
            .count(),
        _ => 0,
    };
    assert!(solid_metal < before, "{solid_metal} of {before}");
}

#[test]
fn the_fill_tool_fills_the_channels() {
    let mut harness = realism_on();
    let brush = &mut harness.state_mut().workbench.attributes.brush;
    brush.tool = PaintTool::Fill;
    brush.channels[2] = (true, [255, 0, 0, 255]);
    let at = on_screen(&harness, Vec3::new(0.0, -50.0, 50.0));
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let after = set(&harness);
    assert!(painted(&after, Channel::Glow) > 0);
    assert_eq!(
        painted(&after, Channel::Glow),
        painted(&after, Channel::Roughness)
    );
}

#[test]
fn a_fill_layers_channel_reaches_the_views() {
    let mut harness = realism_on();
    press(&mut harness, "+ Fill");
    {
        let mut changed = set(&harness);
        changed.layers[0].channels.metal = Some(ChannelPart::Fill([255; 4]));
        harness
            .state_mut()
            .workbench
            .document_mut()
            .execute(scene::Command::SetTextureSet {
                material: "Default".into(),
                set: Some(changed),
            })
            .unwrap();
    }
    harness.run();
    let drawing = harness.state().renderer.drawn.last().unwrap().scene.clone();
    let maps = drawing.textures[0].realism.as_ref().unwrap();
    assert_eq!(maps.surface.pixel(10, 10), [204, 255, 128, 255]);
}

#[test]
fn the_realistic_start_turns_the_channels_on() {
    let mut harness = painting_the_cube();
    press(&mut harness, "Realistic");
    let started = set(&harness);
    assert!(started.realism);
    assert!(matches!(
        started.layers[0].channels.roughness,
        Some(ChannelPart::Fill(_))
    ));
}

#[test]
fn the_realistic_preview_has_a_light_to_turn() {
    let mut harness = realism_on();
    let options = harness.state_mut().workbench.viewport.options_mut();
    options.paint_preview = crate::PaintPreview::Realistic;
    options.light_turn = 90.0;
    harness.run();
    let view = harness.state().renderer.drawn.last().unwrap();
    assert_eq!(view.paint_preview, crate::PaintPreview::Realistic);
    assert_eq!(view.light_turn, 90.0);
    assert!(
        crate::PaintPreview::ALL
            .iter()
            .all(|p| !p.explanation().is_empty())
    );
}

#[test]
fn the_realism_masks_add_dirt_wear_and_rust_in_one_step_each() {
    let mut harness = realism_on();
    for (button, name, metal) in [
        ("+ Dirt", "Dirt", 0),
        ("+ Wear", "Worn metal", 255),
        ("+ Rust", "Rust", 0),
    ] {
        press(&mut harness, button);
        let after = set(&harness);
        let top = after.layers.last().unwrap();
        assert_eq!(top.name, name);
        assert_eq!(
            top.channels.metal,
            Some(ChannelPart::Fill([metal, metal, metal, 255]))
        );
        let mask = top.mask.as_ref().unwrap();
        assert!(mask.smart.is_some());
        assert!(mask.image.painted_tiles() > 0, "{name}'s mask is made");
    }
    // Hand-painted sets don't offer them (in the ⋯ menu, decision 0227).
    let mut harness = painting_the_cube();
    press(&mut harness, crate::paint_layers::MORE_LABEL);
    assert!(harness.query_by_label("+ Wear").is_none());
    assert!(
        harness.query_by_label("+ Gradient").is_some(),
        "the menu is open"
    );
}

#[test]
fn a_material_layer_is_made_and_changes_in_one_step() {
    // Hand-painted: colour only.
    let mut harness = painting_the_cube();
    press(&mut harness, "+ Material");
    let made = set(&harness);
    let top = made.layers.last().unwrap();
    assert!(top.material.is_some());
    assert!(
        top.image().unwrap().painted_tiles() > 0,
        "made on the shape"
    );
    assert!(top.channels.is_empty(), "no channels without realism");
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Add a material layer")
    );
    // With the realism channels: roughness and metal made too.
    let mut harness = realism_on();
    press(&mut harness, "+ Material");
    let made = set(&harness);
    let top = made.layers.last().unwrap();
    assert!(matches!(top.channels.metal, Some(ChannelPart::Paint(_))));
    assert!(painted(&made, Channel::Roughness) > 0);
    // A new pattern: made again, one step.
    let before = top.image().unwrap().clone();
    press(&mut harness, "New pattern");
    let after = set(&harness);
    assert_ne!(after.layers.last().unwrap().image().unwrap(), &before);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Change a material layer")
    );
    // Brushes don't paint into it: a stroke goes on a new layer.
    let count = after.layers.len();
    paint_front(&mut harness);
    assert_eq!(set(&harness).layers.len(), count + 1);
}

#[test]
fn height_is_painted_and_reaches_the_views() {
    let mut harness = realism_on();
    assert!(
        harness.query_all_by_label("Height").count() > 0,
        "the brush's height row"
    );
    {
        let channels = &mut harness.state_mut().workbench.attributes.brush.channels;
        *channels = [(false, [0; 4]); 4];
        channels[3] = (true, [230, 230, 230, 255]);
    }
    paint_front(&mut harness);
    let after = set(&harness);
    assert!(painted(&after, Channel::Height) > 0);
    assert_eq!(painted(&after, Channel::Roughness), 0, "only height ticked");
    // The views get it in the surface map's blue, level elsewhere.
    let drawing = harness.state().renderer.drawn.last().unwrap().scene.clone();
    let maps = drawing.textures[0].realism.as_ref().unwrap();
    let blues: Vec<u8> = maps
        .surface
        .to_rgba()
        .chunks_exact(4)
        .map(|p| p[2])
        .collect();
    assert!(blues.iter().any(|b| b.abs_diff(230) <= 1));
    assert!(blues.contains(&128));
}

/// How many texels of the top layer's colour are painted.
fn colour_painted(set: &TextureSet) -> usize {
    set.layers
        .last()
        .and_then(|l| l.image())
        .map_or(0, |image| {
            image.to_rgba().chunks_exact(4).filter(|p| p[3] > 0).count()
        })
}

#[test]
fn colour_unticked_paints_height_alone() {
    // A review found height couldn't be painted without colour too.
    let mut harness = realism_on();
    assert!(
        harness
            .query_all_by_label(crate::attributes::brush_rows::COLOUR_SWITCH)
            .count()
            > 0,
        "the Colour switch in Lays down"
    );
    {
        let brush = &mut harness.state_mut().workbench.attributes.brush;
        brush.colour_on = false;
        brush.channels = [(false, [0; 4]); 4];
        brush.channels[3] = (true, [230, 230, 230, 255]);
    }
    paint_front(&mut harness);
    let after = set(&harness);
    assert!(painted(&after, Channel::Height) > 0, "the bump is painted");
    assert_eq!(colour_painted(&after), 0, "no colour laid");
    // Painted again (a texture size change), it still lays no colour.
    let stroke = &after.layers.last().unwrap().strokes[0];
    assert_eq!(stroke.color[3], 0);
}

#[test]
fn colour_ticked_or_realism_off_paints_colour_as_ever() {
    let mut harness = realism_on();
    paint_front(&mut harness);
    assert!(colour_painted(&set(&harness)) > 0);
    // Unticked but the realism channels off: the switch isn't shown, so it
    // mustn't stop the brush painting.
    let mut harness = painting_the_cube();
    harness.state_mut().workbench.attributes.brush.colour_on = false;
    paint_front(&mut harness);
    assert!(colour_painted(&set(&harness)) > 0);
}

#[test]
fn a_fill_layer_can_leave_the_colour_alone() {
    let mut harness = realism_on();
    press(&mut harness, "+ Fill");
    press(&mut harness, crate::paint_layers::FILL_COLOUR_SWITCH);
    let after = set(&harness);
    match after.layers[0].content {
        paint::Content::Fill(colour) => assert_eq!(colour[3], 0, "colour off"),
        _ => panic!("a fill layer"),
    }
    // Ticked again: its colour is back, as it was.
    press(&mut harness, crate::paint_layers::FILL_COLOUR_SWITCH);
    match set(&harness).layers[0].content {
        paint::Content::Fill(colour) => assert_eq!(colour, [128, 128, 128, 255]),
        _ => panic!("a fill layer"),
    }
}
