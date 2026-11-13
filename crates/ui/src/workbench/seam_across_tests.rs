//! Blending seams where two materials meet (decision 0296): a cube whose
//! top is a second material, each material painted its own colour on its
//! own texture. A click on an edge where they meet marks it on both
//! materials' Seam blend layers in one undo step, both are blended
//! towards the middle, painting one side makes the other's layer again,
//! and Every seam that shows and Unmark all take in both sides.

use super::modelling_tests::on_screen;
use super::seam_blend_tests::{click, frames, wait};
use super::tests::{State, camera};
use crate::paint_layers::seam_rows::SeamAsk;
use egui::Modifiers;
use glam::Vec3;
use paint::seams::{Seam, seams_between, seams_of};
use paint::{Image, Layer, PaintSurface, SeamBlend, TextureSet};
use scene::{Command, ObjectId};

type H = egui_kittest::Harness<'static, State>;

const RED: [u8; 4] = [220, 30, 30, 255];
const BLUE: [u8; 4] = [30, 40, 220, 255];

/// A `size`-pixel texture set with one layer of `colour` all over.
fn painted(size: u32, colour: [u8; 4]) -> TextureSet {
    let mut image = Image::new(size).unwrap();
    for y in 0..size {
        for x in 0..size {
            image.set_pixel(x, y, colour);
        }
    }
    let mut set = TextureSet::new(size).unwrap();
    set.layers.push(Layer::paint("Paint", image));
    set
}

/// The cube in Paint mode with Blend seams chosen, its top given a
/// second material, Metal (slot 1). Default is painted red, Metal blue
/// when `metal_painted`, each on its own texture (textures of different
/// sizes are tested in `paint::seam_blend`).
fn two_material_cube(metal_painted: bool) -> (H, ObjectId) {
    let mut harness = super::paint_layers_tests::painting_the_cube();
    let id = harness.state().workbench.document().selection()[0];
    let doc = harness.state_mut().workbench.document_mut();
    let mut texture = doc.scene().texture();
    texture.size = 128;
    doc.execute(Command::SetTexture(texture)).unwrap();
    let mut metal = scene::Material::standard();
    metal.name = "Metal".into();
    doc.execute(Command::SetMaterials {
        id,
        materials: vec![scene::Material::standard(), metal],
    })
    .unwrap();
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let top = mesh
        .faces()
        .map(|(f, _)| f)
        .max_by(|a, b| mesh.face_normal(*a).z.total_cmp(&mesh.face_normal(*b).z))
        .unwrap();
    doc.execute(Command::Record {
        id,
        operation: scene::Operation::SetMaterial {
            faces: vec![top],
            slot: 1,
            anchors: None,
        },
    })
    .unwrap();
    doc.execute(Command::SetTextureSet {
        material: "Default".into(),
        set: Some(painted(128, RED)),
    })
    .unwrap();
    if metal_painted {
        doc.execute(Command::SetTextureSet {
            material: "Metal".into(),
            set: Some(painted(128, BLUE)),
        })
        .unwrap();
    }
    harness.run();
    harness.key_press(egui::Key::Num9);
    harness.run();
    (harness, id)
}

/// The cube's shape as it is now, as the brush sees it.
fn surface(harness: &H, id: ObjectId) -> PaintSurface {
    let mesh = harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh;
    PaintSurface::of(&mesh)
}

/// The edges where Default meets Metal that face the camera, nearest the
/// middle of the view first.
fn meetings_in_view(harness: &H, id: ObjectId) -> Vec<Seam> {
    let surface = surface(harness, id);
    let camera = camera(harness);
    let mut seams: Vec<Seam> = seams_between(&surface, &[0], &[1])
        .into_iter()
        .filter(|s| {
            let middle = (s.ends[0] + s.ends[1]) / 2.0;
            s.facing(&surface).iter().all(|n| {
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

/// `material`'s texture set.
fn set_of(harness: &H, material: &str) -> Option<TextureSet> {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .paint()
        .set(material)
        .map(|s| (**s).clone())
}

/// `material`'s top layer's Seam blend.
fn blend_of(harness: &H, material: &str) -> Option<SeamBlend> {
    set_of(harness, material)?.layers.last()?.seam_blend.clone()
}

/// `material`'s top layer, which should be its Seam blend layer.
fn blend_layer(harness: &H, material: &str) -> Layer {
    set_of(harness, material)
        .and_then(|s| s.layers.last().cloned())
        .expect("a layer on top")
}

/// Clicks the middle of `seam`, a little to one side so the hit is on a
/// triangle (with `modifiers`).
fn click_seam(harness: &mut H, seam: &Seam, modifiers: Modifiers) {
    let at = on_screen(harness, (seam.ends[0] + seam.ends[1]) / 2.0);
    click(harness, at, modifiers);
}

#[test]
fn the_edges_where_two_materials_meet_show_with_the_seams() {
    let (harness, id) = two_material_cube(true);
    let lines = harness
        .state()
        .workbench
        .viewport
        .seam_lines()
        .expect("seams shown");
    let surface = surface(&harness, id);
    let own = seams_of(&surface, &[0]).len();
    let between = seams_between(&surface, &[0], &[1]).len();
    assert_eq!(between, 4, "the top meets its four sides");
    assert_eq!(lines.all.len(), own + between);
}

#[test]
fn a_click_where_two_materials_meet_blends_both_sides_in_one_undo_step() {
    let (mut harness, id) = two_material_cube(true);
    let steps = harness.state().workbench.document().undo_steps();
    let seam = meetings_in_view(&harness, id)[0];
    click_seam(&mut harness, &seam, Modifiers::NONE);
    let note = harness.state().workbench.note.clone().unwrap_or_default();
    assert!(note.contains("Default") && note.contains("Metal"), "{note}");
    for material in ["Default", "Metal"] {
        let blend = blend_of(&harness, material).expect("a Seam blend layer on each");
        assert!(blend.seams.iter().any(|s| s.points == seam.points));
    }
    wait(&mut harness);
    frames(&mut harness);
    wait(&mut harness);
    for material in ["Default", "Metal"] {
        let layer = blend_layer(&harness, material);
        assert_ne!(
            layer.seam_blend.as_ref().unwrap().made,
            0,
            "{material} made"
        );
        let image = layer.image().unwrap();
        assert!(image.painted_tiles() > 0, "{material} blended");
        // The blend holds the other side's colour.
        let mut seen = false;
        for y in 0..image.size() {
            for x in 0..image.size() {
                let p = image.pixel(x, y);
                if p[3] > 0 {
                    seen = true;
                    let towards = if material == "Default" { BLUE } else { RED };
                    assert_eq!(&p[..3], &towards[..3], "{material} at {x}, {y}");
                }
            }
        }
        assert!(seen);
    }
    // Made from each other's paint, not each other's blend: nothing more
    // to make.
    frames(&mut harness);
    assert!(!harness.state().workbench.seam_blends.busy());
    // Marking and both blends are one step.
    let doc = harness.state_mut().workbench.document_mut();
    assert_eq!(doc.undo_steps(), steps + 1);
    doc.undo().unwrap();
    wait(&mut harness);
    for material in ["Default", "Metal"] {
        let set = set_of(&harness, material).unwrap();
        assert!(
            set.layers.iter().all(|l| l.seam_blend.is_none()),
            "{material}"
        );
    }
}

#[test]
fn shift_click_unmarks_both_sides_and_a_plain_click_keeps_the_others_marks() {
    let (mut harness, id) = two_material_cube(true);
    let seams = meetings_in_view(&harness, id);
    assert!(seams.len() >= 2, "two meetings in view");
    click_seam(&mut harness, &seams[0], Modifiers::NONE);
    click_seam(&mut harness, &seams[1], Modifiers::SHIFT);
    for material in ["Default", "Metal"] {
        let marked = blend_of(&harness, material).unwrap().seams;
        assert!(
            marked.iter().any(|s| s.points == seams[1].points),
            "{material}"
        );
    }
    click_seam(&mut harness, &seams[1], Modifiers::SHIFT);
    for material in ["Default", "Metal"] {
        let marked = blend_of(&harness, material).unwrap().seams;
        assert!(
            !marked.iter().any(|s| s.points == seams[1].points),
            "{material}"
        );
        assert!(
            marked.iter().any(|s| s.points == seams[0].points),
            "{material}"
        );
    }
    let note = harness.state().workbench.note.clone().unwrap_or_default();
    assert!(note.contains("Unmarked"), "{note}");
}

#[test]
fn painting_one_side_makes_the_other_sides_layer_again() {
    let (mut harness, id) = two_material_cube(true);
    let seam = meetings_in_view(&harness, id)[0];
    click_seam(&mut harness, &seam, Modifiers::NONE);
    wait(&mut harness);
    frames(&mut harness);
    wait(&mut harness);
    let made = blend_of(&harness, "Default").unwrap().made;
    // Metal's paint changes: Default's layer reads it, so it is made again.
    let mut metal = set_of(&harness, "Metal").unwrap();
    let green = painted(metal.size, [30, 200, 40, 255]);
    metal.layers[0] = green.layers[0].clone();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(Command::SetTextureSet {
            material: "Metal".into(),
            set: Some(metal),
        })
        .unwrap();
    wait(&mut harness);
    frames(&mut harness);
    wait(&mut harness);
    let after = blend_of(&harness, "Default").unwrap().made;
    assert_ne!(after, made, "made again");
    let image = blend_layer(&harness, "Default").image().unwrap().clone();
    let mut greens = 0;
    for y in 0..image.size() {
        for x in 0..image.size() {
            let p = image.pixel(x, y);
            greens += usize::from(p[3] > 0 && p[1] > 150);
        }
    }
    assert!(greens > 0, "now blended towards green");
    frames(&mut harness);
    assert!(!harness.state().workbench.seam_blends.busy(), "settled");
}

#[test]
fn every_seam_that_shows_and_unmark_all_take_in_both_materials() {
    let (mut harness, id) = two_material_cube(true);
    harness.state_mut().workbench.attributes.seam_asked = Some(SeamAsk::Showing);
    frames(&mut harness);
    // Default is one colour all over: only where it meets Metal shows.
    let surface = surface(&harness, id);
    let between: Vec<[u32; 2]> = seams_between(&surface, &[0], &[1])
        .iter()
        .map(|s| s.points)
        .collect();
    for material in ["Default", "Metal"] {
        let marked: Vec<[u32; 2]> = blend_of(&harness, material)
            .unwrap()
            .seams
            .iter()
            .map(|s| s.points)
            .collect();
        assert_eq!(marked, between, "{material}");
    }
    let note = harness.state().workbench.note.clone().unwrap_or_default();
    assert!(note.contains("another material"), "{note}");
    // Unmark all, from Default: Metal's side is unmarked too.
    harness.state_mut().workbench.attributes.seam_asked = Some(SeamAsk::Unmark);
    frames(&mut harness);
    for material in ["Default", "Metal"] {
        assert!(
            blend_of(&harness, material).unwrap().seams.is_empty(),
            "{material}"
        );
    }
}

#[test]
fn pointing_where_two_materials_meet_names_them() {
    let (mut harness, id) = two_material_cube(true);
    let seam = meetings_in_view(&harness, id)[0];
    let at = on_screen(&harness, (seam.ends[0] + seam.ends[1]) / 2.0);
    harness.event(egui::Event::PointerMoved(at));
    frames(&mut harness);
    let note = harness.state().workbench.note.clone().unwrap_or_default();
    assert!(note.contains("meets"), "{note}");
    let lines = harness.state().workbench.viewport.seam_lines().unwrap();
    assert!(!lines.hovered.is_empty());
}

#[test]
fn an_unpainted_other_material_says_to_paint_it_first() {
    let (mut harness, id) = two_material_cube(false);
    let seam = meetings_in_view(&harness, id)[0];
    click_seam(&mut harness, &seam, Modifiers::NONE);
    let note = harness.state().workbench.note.clone().unwrap_or_default();
    assert!(note.contains("Nothing is painted on Metal"), "{note}");
    let default = set_of(&harness, "Default").unwrap();
    assert!(default.layers.iter().all(|l| l.seam_blend.is_none()));
    assert!(set_of(&harness, "Metal").is_none());
}
