//! Tests for moving pivots (decision 0068): the shape and the objects
//! inside stay where they are, undo puts everything back, and what uses
//! the shape's place (bounds, joining, splitting off) follows the offset.

use crate::*;
use glam::{Mat4, Vec3};
use std::f32::consts::FRAC_PI_2;

fn with_a_cube() -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cube(
        Default::default(),
    ))))
    .unwrap();
    let id = doc.selection()[0];
    (doc, id)
}

fn at(x: f32, y: f32, z: f32) -> Transform {
    Transform {
        position: Vec3::new(x, y, z),
        ..Transform::default()
    }
}

fn close(a: Mat4, b: Mat4) -> bool {
    a.abs_diff_eq(b, 1e-3)
}

#[test]
fn moving_a_pivot_leaves_the_shape_where_it_was() {
    let (mut doc, id) = with_a_cube();
    let shape_before = doc.scene().shape_matrix(id);
    let bounds_before = doc.scene().bounds_of([id]).unwrap();
    doc.execute(Command::MovePivots(vec![(id, at(50.0, 0.0, 30.0))]))
        .unwrap();
    let scene = doc.scene();
    assert_eq!(scene.object(id).unwrap().transform, at(50.0, 0.0, 30.0));
    assert!(close(scene.shape_matrix(id), shape_before));
    let bounds = scene.bounds_of([id]).unwrap();
    assert!(bounds.min.abs_diff_eq(bounds_before.min, 1e-3));
    assert!(bounds.max.abs_diff_eq(bounds_before.max, 1e-3));
    // The shape now sits back from the pivot.
    assert!(
        scene
            .object(id)
            .unwrap()
            .offset
            .position
            .abs_diff_eq(Vec3::new(-50.0, 0.0, -30.0), 1e-3)
    );
    assert_eq!(doc.undo_label(), Some("Move pivot"));
    doc.undo().unwrap();
    let object = doc.scene().object(id).unwrap();
    assert_eq!(object.transform, Transform::default());
    assert_eq!(object.offset, Transform::default());
}

#[test]
fn turning_a_pivot_leaves_the_shape_where_it_was() {
    let (mut doc, id) = with_a_cube();
    doc.execute(Command::SetTransforms(vec![(id, at(10.0, 20.0, 0.0))]))
        .unwrap();
    let before = doc.scene().shape_matrix(id);
    let turned = Transform {
        rotation: Vec3::new(FRAC_PI_2, 0.3, 0.0),
        ..at(10.0, 20.0, 0.0)
    };
    doc.execute(Command::MovePivots(vec![(id, turned)]))
        .unwrap();
    assert!(close(doc.scene().shape_matrix(id), before));
    // Moving the object afterwards carries the shape along with the new
    // pivot.
    let moved = Transform {
        position: Vec3::new(110.0, 20.0, 0.0),
        ..turned
    };
    doc.execute(Command::SetTransforms(vec![(id, moved)]))
        .unwrap();
    let shift = Mat4::from_translation(Vec3::new(100.0, 0.0, 0.0));
    assert!(close(doc.scene().shape_matrix(id), shift * before));
}

#[test]
fn objects_inside_stay_where_they_were() {
    let (mut doc, parent) = with_a_cube();
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cube(
        Default::default(),
    ))))
    .unwrap();
    let child = doc.selection()[0];
    doc.execute(Command::SetTransforms(vec![(child, at(300.0, 0.0, 0.0))]))
        .unwrap();
    doc.execute(Command::Move {
        ids: vec![child],
        parent: Some(parent),
        index: 0,
    })
    .unwrap();
    let child_before = doc.scene().shape_matrix(child);
    doc.execute(Command::MovePivots(vec![(
        parent,
        Transform {
            rotation: Vec3::new(0.7, 0.0, 0.0),
            ..at(-40.0, 0.0, 10.0)
        },
    )]))
    .unwrap();
    assert!(close(doc.scene().shape_matrix(child), child_before));
    // One undo step puts the parent, its offset and the child back.
    doc.undo().unwrap();
    assert_eq!(
        doc.scene().object(child).unwrap().transform,
        at(300.0, 0.0, 0.0)
    );
}

#[test]
fn a_pivot_squashed_flat_or_not_a_number_is_refused() {
    let (mut doc, id) = with_a_cube();
    let flat = Transform {
        scale: Vec3::new(1.0, 0.0, 1.0),
        ..Transform::default()
    };
    assert!(doc.execute(Command::MovePivots(vec![(id, flat)])).is_err());
    let nan = at(f32::NAN, 0.0, 0.0);
    assert!(doc.execute(Command::MovePivots(vec![(id, nan)])).is_err());
    assert_eq!(doc.scene().object(id).unwrap().offset, Transform::default());
}

#[test]
fn moving_a_pivot_to_where_it_is_is_no_step() {
    let (mut doc, id) = with_a_cube();
    let steps = doc.undo_steps();
    doc.execute(Command::MovePivots(vec![(id, Transform::default())]))
        .unwrap();
    assert_eq!(doc.undo_steps(), steps);
}

#[test]
fn joining_and_copying_keep_shapes_in_place_after_a_pivot_move() {
    let (mut doc, a) = with_a_cube();
    doc.execute(Command::MovePivots(vec![(a, at(0.0, 0.0, 500.0))]))
        .unwrap();
    // A copy has its own pivot and offset, as made.
    doc.execute(Command::Duplicate {
        ids: vec![a],
        instances: false,
    })
    .unwrap();
    let b = doc.selection()[0];
    assert!(close(
        doc.scene().shape_matrix(b),
        doc.scene().shape_matrix(a)
    ));
    doc.execute(Command::SetTransforms(vec![(b, at(300.0, 0.0, 500.0))]))
        .unwrap();
    let bounds = doc.scene().bounds_of([a, b]).unwrap();
    doc.execute(Command::Join(vec![a, b])).unwrap();
    let joined = doc.scene().bounds_of([a]).unwrap();
    assert!(joined.min.abs_diff_eq(bounds.min, 1e-3));
    assert!(joined.max.abs_diff_eq(bounds.max, 1e-3));
}

#[test]
fn moving_pivots_merges_into_one_step_while_dragged() {
    let (mut doc, id) = with_a_cube();
    for x in 1..=5 {
        doc.execute_merging(
            Command::MovePivots(vec![(id, at(x as f32 * 10.0, 0.0, 0.0))]),
            7,
        )
        .unwrap();
    }
    assert!(doc.undo().is_some());
    assert!(doc.undo().is_some(), "the cube's creation");
    assert!(doc.undo().is_none());
}

// ------------------------------------------------------- project units ---

#[test]
fn a_project_starts_in_metres_and_its_units_change_as_a_step() {
    let mut doc = Document::new();
    assert_eq!(doc.scene().units(), Units::Metres);
    doc.execute(Command::SetUnits(Units::Centimetres)).unwrap();
    assert_eq!(doc.scene().units(), Units::Centimetres);
    assert_eq!(doc.undo_label(), Some("Change units to centimetres"));
    doc.undo().unwrap();
    assert_eq!(doc.scene().units(), Units::Metres);
    doc.redo().unwrap();
    assert_eq!(doc.scene().units(), Units::Centimetres);
    // The same units again: no step.
    let steps = doc.undo_steps();
    doc.execute(Command::SetUnits(Units::Centimetres)).unwrap();
    assert_eq!(doc.undo_steps(), steps);
}

#[test]
fn a_new_or_opened_scene_can_start_in_any_units() {
    let scene = Scene::new().with_units(Units::Centimetres);
    let doc = Document::from_scene(scene, []);
    assert_eq!(doc.scene().units(), Units::Centimetres);
    assert_eq!(doc.undo_steps(), 0);
}
