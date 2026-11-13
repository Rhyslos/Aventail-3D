//! Blend seams (decision 0293): key 9 and the tool row's button, a click
//! on a seam marking its line on a new Seam blend layer, Shift + click
//! marking more or unmarking, Every seam that shows, and the layer made
//! from the paint under it on another thread, joining the change's undo
//! step, made again only when what it is made from changes.

use super::brushing_tests::stroke;
use super::modelling_tests::on_screen;
use super::paint_layers_tests::{painting_the_cube, set};
use super::tests::{State, camera};
use crate::attributes::PaintTool;
use crate::paint_layers::seam_rows::SeamAsk;
use egui::{Modifiers, PointerButton};
use glam::Vec3;
use paint::seams::{Seam, seams_of};
use paint::{Brush, Image, Layer, PaintSurface, SeamBlend, StrokeCanvas, TextureSet};
use scene::Command;
use std::time::{Duration, Instant};

type H = egui_kittest::Harness<'static, State>;

/// Each side of the cube (its two triangles) a colour of its own.
const COLOURS: [[u8; 4]; 6] = [
    [220, 30, 30, 255],
    [30, 200, 40, 255],
    [30, 40, 220, 255],
    [230, 220, 40, 255],
    [200, 40, 200, 255],
    [40, 210, 210, 255],
];

/// The cube's surface, as the brush sees it.
pub(super) fn cube_surface() -> PaintSurface {
    PaintSurface::of(&scene::PrimitiveKind::Cube.standard().mesh())
}

/// Painting the cube with Blend seams chosen (9), each of its sides
/// painted its own colour on a 128-pixel texture.
pub(super) fn blending_the_cube() -> H {
    let mut harness = painting_the_cube();
    let doc = harness.state_mut().workbench.document_mut();
    let mut texture = doc.scene().texture();
    texture.size = 128;
    doc.execute(Command::SetTexture(texture)).unwrap();
    let surface = cube_surface();
    let mut canvas = StrokeCanvas::new(Image::new(128).unwrap());
    for (side, colour) in COLOURS.iter().enumerate() {
        let brush = Brush {
            color: *colour,
            ..Brush::default()
        };
        paint::fill::fill_triangles(&mut canvas, &surface, &[side * 2, side * 2 + 1], &brush);
    }
    let mut set = TextureSet::new(128).unwrap();
    set.layers
        .push(Layer::paint("Paint", canvas.image().clone()));
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(Command::SetTextureSet {
            material: "Default".into(),
            set: Some(set),
        })
        .unwrap();
    harness.key_press(egui::Key::Num9);
    harness.run();
    harness
}

/// The cube's seams that face the camera, nearest the middle of the
/// view first.
pub(super) fn seams_in_view(harness: &H) -> Vec<Seam> {
    let surface = cube_surface();
    let camera = camera(harness);
    let mut seams: Vec<Seam> = seams_of(&surface, &[0])
        .into_iter()
        .filter(|s| {
            let middle = (s.ends[0] + s.ends[1]) / 2.0;
            s.facing(&surface).iter().any(|n| {
                n.dot(middle - camera.position) < -0.2 * (middle - camera.position).length()
            })
        })
        .collect();
    let towards = |s: &Seam| {
        let middle = (s.ends[0] + s.ends[1]) / 2.0;
        middle.distance(Vec3::new(0.0, -50.0, 50.0))
    };
    seams.sort_by(|a, b| towards(a).total_cmp(&towards(b)));
    seams
}

/// Clicks at `at` (with `modifiers`), frame by frame: a Seam blend
/// layer being made keeps asking for frames, so `Harness::run` can't be
/// used.
pub(super) fn click(harness: &mut H, at: egui::Pos2, modifiers: Modifiers) {
    harness.event(egui::Event::ModifiersChanged(modifiers));
    harness.event(egui::Event::PointerMoved(at));
    harness.step();
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers,
        });
        harness.step();
    }
    harness.event(egui::Event::ModifiersChanged(Modifiers::NONE));
    harness.step();
    harness.step();
}

/// Clicks the middle of `seam` (with `modifiers`).
fn click_seam(harness: &mut H, seam: &Seam, modifiers: Modifiers) {
    let at = on_screen(harness, (seam.ends[0] + seam.ends[1]) / 2.0);
    click(harness, at, modifiers);
}

/// Runs a few frames (without `Harness::run`, as for [`click`]).
pub(super) fn frames(harness: &mut H) {
    for _ in 0..4 {
        harness.step();
    }
}

/// The top layer's Seam blend.
pub(super) fn top_blend(harness: &H) -> Option<SeamBlend> {
    set(harness).layers.last()?.seam_blend.clone()
}

/// Runs frames until no Seam blend layer is being made (or half a
/// minute passes).
pub(super) fn wait(harness: &mut H) {
    let start = Instant::now();
    loop {
        harness.step();
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
        if !harness.state().workbench.seam_blends.busy()
            || start.elapsed() > Duration::from_secs(30)
        {
            break;
        }
    }
    harness.run();
}

#[test]
fn nine_chooses_blend_seams_and_shows_the_seams_over_the_view() {
    let mut harness = blending_the_cube();
    let workbench = &harness.state().workbench;
    assert_eq!(workbench.attributes.brush.tool, PaintTool::Seams);
    let lines = workbench.viewport.seam_lines().expect("seams shown");
    assert!(!lines.all.is_empty());
    // Another tool's key: the seams go.
    harness.key_press(egui::Key::Num1);
    harness.run();
    assert!(harness.state().workbench.viewport.seam_lines().is_none());
    // The tool row's button chooses it too.
    let mut settings = harness.state().workbench.attributes.brush;
    let mut hands = harness.state().workbench.paint_hands();
    crate::paint_tools::PaintToolButton::Seams.choose(&mut settings, &mut hands);
    assert_eq!(settings.tool, PaintTool::Seams);
    assert!(crate::paint_tools::PaintToolButton::Seams.is_on(&settings, hands));
}

#[test]
fn a_click_marks_the_seam_line_on_a_new_layer_blended_in_one_undo_step() {
    let mut harness = blending_the_cube();
    let steps = harness.state().workbench.document().undo_steps();
    let seam = seams_in_view(&harness)[0];
    click_seam(&mut harness, &seam, Modifiers::NONE);
    let blend = top_blend(&harness).expect("a Seam blend layer on top");
    assert!(!blend.seams.is_empty());
    assert!(blend.seams.iter().any(|s| s.points == seam.points));
    assert_eq!(set(&harness).layers.last().unwrap().name, "Seam blend");
    wait(&mut harness);
    let blend = top_blend(&harness).unwrap();
    assert_ne!(blend.made, 0, "it was made");
    let layer = set(&harness).layers.last().unwrap().clone();
    assert!(layer.image().unwrap().painted_tiles() > 0, "blended texels");
    // Marking and making are one step: one undo takes both away.
    let doc = harness.state_mut().workbench.document_mut();
    assert_eq!(doc.undo_steps(), steps + 1);
    doc.undo().unwrap();
    wait(&mut harness);
    assert!(set(&harness).layers.iter().all(|l| l.seam_blend.is_none()));
}

#[test]
fn shift_click_marks_more_and_unmarks_a_marked_line() {
    let mut harness = blending_the_cube();
    let seams = seams_in_view(&harness);
    click_seam(&mut harness, &seams[0], Modifiers::NONE);
    let first = top_blend(&harness).unwrap().seams;
    // Another line, not on the first.
    let other = seams
        .iter()
        .find(|s| !first.iter().any(|f| f.points == s.points))
        .expect("another seam in view");
    click_seam(&mut harness, other, Modifiers::SHIFT);
    let both = top_blend(&harness).unwrap().seams;
    assert!(both.len() > first.len());
    assert!(first.iter().all(|f| both.contains(f)));
    // A plain click marks in place of them.
    click_seam(&mut harness, other, Modifiers::NONE);
    let only = top_blend(&harness).unwrap().seams;
    assert!(!only.iter().any(|s| first.contains(s)));
    // Shift + click on a marked line unmarks it.
    click_seam(&mut harness, other, Modifiers::SHIFT);
    assert!(top_blend(&harness).unwrap().seams.is_empty());
    // Still one layer.
    let count = set(&harness)
        .layers
        .iter()
        .filter(|l| l.seam_blend.is_some())
        .count();
    assert_eq!(count, 1);
}

#[test]
fn a_click_off_the_seams_marks_nothing_and_says_why() {
    let mut harness = blending_the_cube();
    let middle = on_screen(&harness, Vec3::new(0.0, -50.0, 50.0));
    click(&mut harness, middle, Modifiers::NONE);
    assert!(set(&harness).layers.iter().all(|l| l.seam_blend.is_none()));
    assert_eq!(
        harness.state().workbench.note.as_deref(),
        Some(crate::text::tr(super::seam_picking::NO_SEAM_HERE))
    );
    // Nothing was painted either.
    assert_eq!(set(&harness).layers.len(), 1);
}

#[test]
fn every_seam_that_shows_marks_the_mismatched_ones() {
    let mut harness = blending_the_cube();
    harness.state_mut().workbench.attributes.seam_asked = Some(SeamAsk::Showing);
    frames(&mut harness);
    let blend = top_blend(&harness).expect("a layer made for them");
    // Every side is a colour of its own: every seam shows.
    assert_eq!(blend.seams.len(), seams_of(&cube_surface(), &[0]).len());
    // Unmark all.
    harness.state_mut().workbench.attributes.seam_asked = Some(SeamAsk::Unmark);
    frames(&mut harness);
    assert!(top_blend(&harness).unwrap().seams.is_empty());
}

#[test]
fn the_width_remakes_it_and_undo_puts_both_back() {
    let mut harness = blending_the_cube();
    let seam = seams_in_view(&harness)[0];
    click_seam(&mut harness, &seam, Modifiers::NONE);
    wait(&mut harness);
    let before = set(&harness);
    let made = top_blend(&harness).unwrap().made;
    harness.state_mut().workbench.attributes.seam_asked = Some(SeamAsk::Width(20.0));
    frames(&mut harness);
    wait(&mut harness);
    let blend = top_blend(&harness).unwrap();
    assert_eq!(blend.width, 20.0);
    assert_ne!(blend.made, made);
    let wider = set(&harness)
        .layers
        .last()
        .unwrap()
        .image()
        .unwrap()
        .clone();
    let narrower = before.layers.last().unwrap().image().unwrap().clone();
    let count = |image: &Image| {
        let mut n = 0;
        for y in 0..image.size() {
            for x in 0..image.size() {
                n += usize::from(image.pixel(x, y)[3] > 0);
            }
        }
        n
    };
    assert!(count(&wider) > count(&narrower));
    harness.state_mut().workbench.document_mut().undo().unwrap();
    wait(&mut harness);
    assert_eq!(set(&harness), before);
}

#[test]
fn turning_the_view_or_changing_a_layer_above_makes_nothing_again() {
    let mut harness = blending_the_cube();
    let seam = seams_in_view(&harness)[0];
    click_seam(&mut harness, &seam, Modifiers::NONE);
    wait(&mut harness);
    let made = top_blend(&harness).unwrap().made;
    // Orbit: nothing changes in the document, nothing is made.
    let middle = on_screen(&harness, Vec3::new(0.0, -50.0, 50.0));
    stroke(
        &mut harness,
        middle,
        middle + egui::vec2(40.0, 10.0),
        Modifiers::ALT,
        PointerButton::Primary,
    );
    assert!(!harness.state().workbench.seam_blends.busy());
    // A layer added above it: not under it, so not made again.
    {
        let doc = harness.state_mut().workbench.document_mut();
        let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
        set.layers.push(Layer::fill("Tint", [10, 10, 10, 40]));
        doc.execute(Command::SetTextureSet {
            material: "Default".into(),
            set: Some(set),
        })
        .unwrap();
    }
    frames(&mut harness);
    assert!(!harness.state().workbench.seam_blends.busy());
    let blend = set(&harness).layers[1].seam_blend.clone().unwrap();
    assert_eq!(blend.made, made);
    // Paint under it changed: made again.
    {
        let doc = harness.state_mut().workbench.document_mut();
        let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
        if let Some(image) = set.layers[0].image_mut() {
            for y in 0..128 {
                for x in 0..128 {
                    if image.pixel(x, y)[3] > 0 {
                        image.set_pixel(x, y, [128, 128, 128, 255]);
                    }
                }
            }
        }
        doc.execute(Command::SetTextureSet {
            material: "Default".into(),
            set: Some(set),
        })
        .unwrap();
    }
    wait(&mut harness);
    assert_ne!(
        set(&harness).layers[1].seam_blend.clone().unwrap().made,
        made
    );
}

#[test]
fn seams_no_longer_on_the_model_are_counted_and_left_out() {
    let mut harness = blending_the_cube();
    {
        let doc = harness.state_mut().workbench.document_mut();
        let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
        let mut layer = Layer::paint("Seam blend", Image::new(128).unwrap());
        let mut blend = SeamBlend::default();
        blend.mark(
            [paint::SeamId {
                object: 999,
                points: [1, 2],
            }],
            false,
        );
        layer.seam_blend = Some(blend);
        set.layers.push(layer);
        doc.execute(Command::SetTextureSet {
            material: "Default".into(),
            set: Some(set),
        })
        .unwrap();
    }
    wait(&mut harness);
    let workbench = &harness.state().workbench;
    assert_eq!(workbench.attributes.seam_tool.1.missing, 1);
    assert_eq!(workbench.attributes.seam_tool.1.marked, 1);
    let layer = set(&harness).layers.last().unwrap().clone();
    assert_ne!(layer.seam_blend.as_ref().unwrap().made, 0);
    assert_eq!(layer.image().unwrap().painted_tiles(), 0);
}

#[test]
fn with_the_realism_channels_each_channel_is_blended_too() {
    let mut harness = blending_the_cube();
    {
        let doc = harness.state_mut().workbench.document_mut();
        let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
        set.realism = true;
        let colour = set.layers[0].image().unwrap().clone();
        set.layers[0].channels.roughness = Some(paint::ChannelPart::Paint(colour));
        doc.execute(Command::SetTextureSet {
            material: "Default".into(),
            set: Some(set),
        })
        .unwrap();
    }
    frames(&mut harness);
    let seam = seams_in_view(&harness)[0];
    click_seam(&mut harness, &seam, Modifiers::NONE);
    wait(&mut harness);
    let layer = set(&harness).layers.last().unwrap().clone();
    let rough = layer
        .channels
        .get(paint::Channel::Roughness)
        .and_then(|p| p.image())
        .expect("roughness blended");
    assert!(rough.painted_tiles() > 0);
    // Metal isn't painted under it: nothing to blend there.
    let metal = layer
        .channels
        .get(paint::Channel::Metal)
        .and_then(|p| p.image());
    assert!(metal.is_none_or(|m| m.painted_tiles() == 0));
    assert!(set(&harness).is_valid());
}

#[test]
fn the_paint_tab_and_the_layer_list_show_the_seam_blend() {
    use egui_kittest::kittest::Queryable;
    let mut harness = blending_the_cube();
    let showing = crate::paint_layers::seam_rows::SHOWING_LABEL;
    assert!(
        harness.query_by_label(showing).is_some(),
        "in the Paint tab"
    );
    let seam = seams_in_view(&harness)[0];
    click_seam(&mut harness, &seam, Modifiers::NONE);
    wait(&mut harness);
    let layer = set(&harness).layers.last().unwrap().clone();
    assert_eq!(crate::paint_layers::kind_text(&layer), "Seam blend");
    // The chosen layer's rows: its width there as well as in the tab.
    let widths = harness
        .query_all_by_label(crate::paint_layers::seam_rows::WIDTH_LABEL)
        .count();
    assert!(widths >= 1, "{widths}");
    // The panel's Every seam that shows marks the rest.
    harness.state_mut().workbench.paint_layers.seam_asked = Some(("Default".into(), vec![1]));
    frames(&mut harness);
    let all = seams_of(&cube_surface(), &[0]).len();
    assert_eq!(top_blend(&harness).unwrap().seams.len(), all);
}

#[test]
fn a_saved_seam_blend_opens_made_and_is_not_made_again() {
    let mut harness = blending_the_cube();
    let seam = seams_in_view(&harness)[0];
    click_seam(&mut harness, &seam, Modifiers::NONE);
    wait(&mut harness);
    let saved = set(&harness);
    let bytes = fileio::project::to_bytes(harness.state().workbench.document()).unwrap();
    let doc = fileio::project::from_bytes(&bytes).unwrap();
    harness
        .state_mut()
        .workbench
        .opened(doc, std::path::PathBuf::from("seams.avtl"));
    frames(&mut harness);
    assert!(
        !harness.state().workbench.seam_blends.busy(),
        "nothing to make"
    );
    assert_eq!(set(&harness), saved);
    assert!(!harness.state().workbench.has_unsaved_changes());
}
