//! Tests for packing in the window (decision 0101).

use super::modelling_tests::{H, note};
use super::tests::fresh;
use crate::commands::CommandId;
use egui_kittest::kittest::Queryable;
use glam::Vec2;
use scene::ObjectId;

fn add_cube(harness: &mut H, at: f32) -> ObjectId {
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
        scene::Primitive::Cube(Default::default()),
    )))
    .unwrap();
    let id = *doc.selection().first().unwrap();
    doc.execute(scene::Command::SetTransforms(vec![(
        id,
        scene::Transform {
            position: glam::Vec3::new(at, 0.0, 0.0),
            ..Default::default()
        },
    )]))
    .unwrap();
    harness.run();
    id
}

/// Each island's box in the texture, for each object.
fn island_boxes(harness: &H, ids: &[ObjectId]) -> Vec<(Vec2, Vec2)> {
    let doc = harness.state().workbench.document();
    let mut out = Vec::new();
    for id in ids {
        let mesh = doc.scene().evaluated(*id).unwrap().mesh;
        let uvs = mesh::uvs::Uvs::of(&mesh).unwrap();
        for island in mesh::uvs::islands(&mesh) {
            let mut min = Vec2::splat(f32::INFINITY);
            let mut max = Vec2::splat(f32::NEG_INFINITY);
            for f in island {
                for k in 0..mesh.face(f).unwrap().len() {
                    let v = uvs.corner(&mesh, f, k).unwrap();
                    min = min.min(v);
                    max = max.max(v);
                }
            }
            out.push((min, max));
        }
    }
    out
}

fn overlap(a: (Vec2, Vec2), b: (Vec2, Vec2)) -> bool {
    a.0.x < b.1.x - 1e-5 && b.0.x < a.1.x - 1e-5 && a.0.y < b.1.y - 1e-5 && b.0.y < a.1.y - 1e-5
}

fn run(harness: &mut H, command: CommandId) {
    let ctx = harness.ctx.clone();
    harness
        .state_mut()
        .workbench
        .run_command(command, &ctx, &mut Vec::new());
    harness.run();
}

#[test]
fn pack_fits_two_objects_into_one_square_without_overlaps_in_one_step() {
    let mut harness = fresh();
    let a = add_cube(&mut harness, -200.0);
    let b = add_cube(&mut harness, 200.0);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([a, b]);
    harness.run();
    let steps = harness.state().workbench.document().undo_steps();
    run(&mut harness, CommandId::Pack);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
    let boxes = island_boxes(&harness, &[a, b]);
    assert_eq!(boxes.len(), 12);
    for (i, x) in boxes.iter().enumerate() {
        assert!(
            x.0.min_element() >= -1e-4 && x.1.max_element() <= 1.0 + 1e-4,
            "{x:?}"
        );
        for y in &boxes[i + 1..] {
            assert!(!overlap(*x, *y), "{x:?} {y:?}");
        }
    }
    assert!(
        note(&harness).contains("Pack 12 pieces"),
        "{}",
        note(&harness)
    );
}

#[test]
fn a_texture_per_object_packs_each_on_its_own() {
    let mut harness = fresh();
    harness.state_mut().workbench.uv_settings.per_object = true;
    let a = add_cube(&mut harness, -200.0);
    let b = add_cube(&mut harness, 200.0);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([a, b]);
    harness.run();
    run(&mut harness, CommandId::Pack);
    // Each fills the square: their islands lie on top of each other.
    let first = island_boxes(&harness, &[a]);
    let second = island_boxes(&harness, &[b]);
    assert!(first.iter().any(|x| second.iter().any(|y| overlap(*x, *y))));
}

#[test]
fn the_uv_menu_has_packing_settings() {
    let mut harness = fresh();
    super::tests::uv_group(&mut harness, super::UV_SETTINGS_MENU);
    harness.get_by_label_contains("Packing").click();
    harness.run();
    for label in [
        super::unwrapping::PADDING_LABEL,
        "Turn pieces to fit",
        "Stack identical pieces",
        "A texture per material",
        "A texture per object",
    ] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    harness.get_by_label("Stack identical pieces").click();
    harness.run();
    assert!(harness.state().workbench.uv_settings().stack);
}

#[test]
fn nothing_selected_says_so() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.run();
    run(&mut harness, CommandId::Pack);
    assert!(note(&harness).contains("select the objects"));
}
