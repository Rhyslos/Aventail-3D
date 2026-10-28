//! Paint following the model (decision 0189): moving a shape's UVs
//! moves its paint in the same undo step; undo and redo take both back
//! and forth; changes that don't move UVs leave the paint alone.

use super::brushing_tests::stroke;
use super::paint_layers_tests::painting_the_cube;
use super::tests::camera;
use egui::{Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use scene::Operation;
use std::sync::Arc;

type H = egui_kittest::Harness<'static, super::tests::State>;

fn on_screen(harness: &H, world: Vec3) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let at = camera(harness).project(world, size).unwrap();
    rect.min + egui::vec2(at.x, at.y)
}

fn set(harness: &H) -> Arc<paint::TextureSet> {
    Arc::clone(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .paint()
            .set("Default")
            .unwrap(),
    )
}

/// The middle of the painted texels of the top layer.
fn painted_middle(set: &paint::TextureSet) -> (f32, f32) {
    let image = set.layers[0].image().unwrap();
    let (mut sx, mut sy, mut n) = (0.0, 0.0, 0.0);
    for y in 0..image.size() {
        for x in 0..image.size() {
            if image.pixel(x, y)[3] > 200 {
                sx += x as f32;
                sy += y as f32;
                n += 1.0;
            }
        }
    }
    assert!(n > 0.0, "something is painted");
    (sx / n, sy / n)
}

/// A red dab on the cube's front.
fn paint_a_dab(harness: &mut H) {
    harness.state_mut().workbench.attributes.brush.brush.radius = 15.0;
    harness.state_mut().workbench.attributes.brush.brush.color = [255, 0, 0, 255];
    let at = on_screen(harness, Vec3::new(0.0, -50.0, 50.0));
    stroke(harness, at, at, Modifiers::NONE, PointerButton::Primary);
}

/// Moves every UV of the cube `dx` to the right, as a recorded edit.
fn shift_uvs(harness: &mut H, dx: f32) {
    let id = harness.state().workbench.document().scene().walk()[0].0;
    let faces = harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh
        .faces()
        .map(|(f, _)| f)
        .collect();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Record {
            id,
            operation: Operation::TransformUvs {
                faces,
                points: Vec::new(),
                transform: glam::Affine2::from_translation(glam::Vec2::new(dx, 0.0)),
                anchors: None,
            },
        })
        .unwrap();
    harness.run();
}

#[test]
fn moving_the_uvs_moves_the_paint_in_the_same_undo_step() {
    let mut harness = painting_the_cube();
    paint_a_dab(&mut harness);
    let before = set(&harness);
    let (x, y) = painted_middle(&before);
    let steps = harness.state().workbench.document().undo_steps();
    shift_uvs(&mut harness, 0.1);
    let after = set(&harness);
    assert!(!Arc::ptr_eq(&before, &after), "the paint moved");
    let size = after.size as f32;
    let (nx, ny) = painted_middle(&after);
    // Where it was stays (nothing covers it now); the new place has it.
    let image = after.layers[0].image().unwrap();
    let moved_to = ((x + 0.1 * size).round() as u32, y.round() as u32);
    assert!(
        image.pixel(moved_to.0, moved_to.1)[3] > 200,
        "paint at the new place"
    );
    assert!(nx > x, "{nx} vs {x}");
    assert!((ny - y).abs() < 2.0);
    assert_eq!(
        harness.state().workbench.document().undo_steps(),
        steps + 1,
        "one step for the move and the paint"
    );
    // One undo takes both back; redo brings both again; nothing else.
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    harness.run();
    assert!(Arc::ptr_eq(&set(&harness), &before));
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    harness.state_mut().workbench.document_mut().redo();
    harness.run();
    harness.run();
    assert_eq!(*set(&harness), *after);
    assert_eq!(harness.state().workbench.document().redo_steps(), 0);
}

#[test]
fn changes_that_dont_move_uvs_leave_the_paint_alone() {
    let mut harness = painting_the_cube();
    paint_a_dab(&mut harness);
    let before = set(&harness);
    // Moving the object: its UVs are where they were.
    let id = harness.state().workbench.document().scene().walk()[0].0;
    let mut moved = harness
        .state()
        .workbench
        .document()
        .scene()
        .object(id)
        .unwrap()
        .transform;
    moved.position.x += 30.0;
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetTransforms(vec![(id, moved)]))
        .unwrap();
    harness.run();
    harness.run();
    assert!(Arc::ptr_eq(&set(&harness), &before));
}

/// The colour shown at the place on the cube's surface nearest `at`.
fn colour_at(harness: &H, at: Vec3) -> [u8; 4] {
    let doc = harness.state().workbench.document();
    let id = doc.scene().walk()[0].0;
    let mesh = &doc.scene().evaluated(id).unwrap().mesh;
    let surface = paint::PaintSurface::of(mesh);
    let (i, w) = surface.closest(at, &[0]).unwrap();
    let t = &surface.triangles()[i];
    let uv = t.uvs[0] * w[0] + t.uvs[1] * w[1] + t.uvs[2] * w[2];
    let set = set(harness);
    let size = set.size as f32;
    paint::composite::pixel_at(&set, (uv.x * size) as u32, ((1.0 - uv.y) * size) as u32)
}

/// Records `operation` on the cube, over all its polygons.
fn record(harness: &mut H, operation: impl FnOnce(Vec<mesh::FaceId>) -> Operation) {
    let id = harness.state().workbench.document().scene().walk()[0].0;
    let faces = harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh
        .faces()
        .map(|(f, _)| f)
        .collect();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Record {
            id,
            operation: operation(faces),
        })
        .unwrap();
    harness.run();
    harness.run();
}

#[test]
fn re_unwrapping_and_packing_keep_the_paint_where_it_was_on_the_cube() {
    let mut harness = painting_the_cube();
    paint_a_dab(&mut harness);
    let spot = Vec3::new(0.0, -50.0, 50.0);
    let red = colour_at(&harness, spot);
    assert!(red[0] > 200 && red[3] > 200, "{red:?}");
    // Unwrapped again, a different way.
    record(&mut harness, |faces| Operation::Unwrap {
        faces,
        options: uv::UnwrapOptions::plain(false),
        anchors: None,
    });
    let after = colour_at(&harness, spot);
    assert!(
        after[0] > 200 && after[3] > 200,
        "after unwrapping: {after:?}"
    );
    // Packed into half the space.
    record(&mut harness, |faces| Operation::PackUvs {
        islands: vec![(faces, glam::Affine2::from_scale(glam::Vec2::splat(0.5)))],
    });
    let after = colour_at(&harness, spot);
    assert!(after[0] > 200 && after[3] > 200, "after packing: {after:?}");
    // An unpainted place stays unpainted.
    assert!(colour_at(&harness, Vec3::new(0.0, 50.0, 50.0))[3] < 50);
}

#[test]
fn the_status_bar_says_the_paint_moved() {
    let mut harness = painting_the_cube();
    paint_a_dab(&mut harness);
    harness.state_mut().workbench.note = None;
    shift_uvs(&mut harness, 0.05);
    let note = harness.state().workbench.note.clone().unwrap_or_default();
    assert!(note.contains("paint moved"), "{note}");
}
