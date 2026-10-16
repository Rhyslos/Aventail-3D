//! Tests for applying objects' placement (decision 0089): position,
//! rotation and scale go back to none while everything looks the same.

use crate::*;
use glam::{Mat4, Vec3};

/// A document with the standard cube placed by `transform`, and its id.
fn cube_placed(transform: Transform) -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    doc.execute(Command::SetTransforms(vec![(id, transform)]))
        .unwrap();
    (doc, id)
}

/// Every point of `id`'s shape, in the world, in order.
fn world_points(doc: &Document, id: ObjectId) -> Vec<Vec3> {
    let scene = doc.scene();
    let matrix = scene.shape_matrix(id);
    scene
        .evaluated(id)
        .unwrap()
        .mesh
        .vertices()
        .map(|(_, p)| matrix.transform_point3(p))
        .collect()
}

fn same_points(a: &[Vec3], b: &[Vec3]) {
    assert_eq!(a.len(), b.len());
    for (p, q) in a.iter().zip(b) {
        assert!(p.distance(*q) < 1e-2, "{p} {q}");
    }
}

fn stretched() -> Transform {
    Transform {
        position: Vec3::new(120.0, -30.0, 10.0),
        rotation: Vec3::new(0.6, 0.2, -0.3),
        scale: Vec3::new(2.0, 0.5, 1.5),
    }
}

#[test]
fn applying_rotation_and_scale_keeps_the_look_and_the_pivot() {
    let (mut doc, id) = cube_placed(stretched());
    let before = world_points(&doc, id);
    doc.execute(Command::ApplyTransforms {
        ids: vec![id],
        parts: ApplyParts::ROTATION_AND_SCALE,
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Apply rotation and scale"));
    let object = doc.scene().object(id).unwrap();
    assert_eq!(object.transform.rotation, Vec3::ZERO);
    assert_eq!(object.transform.scale, Vec3::ONE);
    assert_eq!(object.transform.position, stretched().position);
    same_points(&before, &world_points(&doc, id));
    // It is a primitive still, with the change recorded on top.
    assert!(matches!(object.kind, ObjectKind::Primitive(_)));
    assert!(matches!(
        object.stack.layers()[0].entries.last().unwrap().operation,
        Operation::PlaceShape { .. }
    ));
    // No stretch left: new edits measure as they look.
    assert_eq!(stretch_of(doc.scene().shape_matrix(id)), Vec3::ONE);
    // One undo step takes it all back.
    doc.undo().unwrap();
    assert_eq!(doc.scene().object(id).unwrap().transform, stretched());
    assert!(
        doc.scene().object(id).unwrap().stack.layers().is_empty() || {
            doc.scene().object(id).unwrap().stack.layers()[0]
                .entries
                .is_empty()
        }
    );
    same_points(&before, &world_points(&doc, id));
}

#[test]
fn applying_everything_or_one_part_at_a_time() {
    for parts in [
        ApplyParts::ALL,
        ApplyParts {
            scale: true,
            ..ApplyParts::default()
        },
        ApplyParts {
            rotation: true,
            ..ApplyParts::default()
        },
        ApplyParts {
            position: true,
            ..ApplyParts::default()
        },
    ] {
        let (mut doc, id) = cube_placed(stretched());
        let before = world_points(&doc, id);
        doc.execute(Command::ApplyTransforms {
            ids: vec![id],
            parts,
        })
        .unwrap();
        let t = doc.scene().object(id).unwrap().transform;
        assert_eq!(t, parts.kept(stretched()), "{parts:?}");
        same_points(&before, &world_points(&doc, id));
    }
    assert_eq!(ApplyParts::ALL.name(), "position, rotation and scale");
    assert_eq!(ApplyParts::default().name(), "nothing");
}

#[test]
fn children_stay_put_and_moved_pivots_are_kept_in_the_shape() {
    let (mut doc, parent) = cube_placed(stretched());
    // A child sphere, placed in the parent.
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Sphere.standard(),
    )))
    .unwrap();
    let child = doc.selection()[0];
    doc.execute(Command::Move {
        ids: vec![child],
        parent: Some(parent),
        index: 0,
    })
    .unwrap();
    // The parent's pivot moved off its shape (decision 0068).
    doc.execute(Command::MovePivots(vec![(
        parent,
        Transform {
            position: stretched().position + Vec3::new(40.0, 0.0, 0.0),
            ..stretched()
        },
    )]))
    .unwrap();
    let (shape_before, child_before) = (world_points(&doc, parent), world_points(&doc, child));
    doc.execute(Command::ApplyTransforms {
        ids: vec![parent],
        parts: ApplyParts::ROTATION_AND_SCALE,
    })
    .unwrap();
    same_points(&shape_before, &world_points(&doc, parent));
    same_points(&child_before, &world_points(&doc, child));
    assert_eq!(
        doc.scene().object(parent).unwrap().offset,
        Transform::default()
    );
    // Both selected: the parent first, then the child; still in place.
    doc.execute(Command::ApplyTransforms {
        ids: vec![child, parent],
        parts: ApplyParts::ALL,
    })
    .unwrap();
    same_points(&shape_before, &world_points(&doc, parent));
    same_points(&child_before, &world_points(&doc, child));
    assert_eq!(
        doc.scene().world_matrix(child),
        Mat4::IDENTITY,
        "the child applied too"
    );
}

#[test]
fn a_mirrored_scale_still_faces_out_and_instances_and_groups_are_handled() {
    let (mut doc, id) = cube_placed(Transform {
        scale: Vec3::new(-1.0, 1.0, 1.0),
        ..Transform::default()
    });
    doc.execute(Command::ApplyTransforms {
        ids: vec![id],
        parts: ApplyParts::ALL,
    })
    .unwrap();
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    assert!(mesh::cleanup::faces_facing_wrong(&shape).is_empty());
    // An instance is left alone; its original applies.
    let (mut doc, id) = cube_placed(stretched());
    doc.execute(Command::Duplicate {
        ids: vec![id],
        instances: true,
    })
    .unwrap();
    let instance = doc.selection()[0];
    doc.execute(Command::ApplyTransforms {
        ids: vec![instance],
        parts: ApplyParts::ALL,
    })
    .unwrap();
    assert_eq!(doc.scene().object(instance).unwrap().transform, stretched());
    // Nothing to apply: nothing recorded.
    let (mut doc, id) = cube_placed(Transform::default());
    let steps = doc.undo_steps();
    doc.execute(Command::ApplyTransforms {
        ids: vec![id],
        parts: ApplyParts::ALL,
    })
    .unwrap();
    assert_eq!(doc.undo_steps(), steps);
    // A squashed-flat object can't be applied into a usable shape.
    let (mut doc, id) = cube_placed(Transform {
        scale: Vec3::new(1.0, 0.0, 1.0),
        ..Transform::default()
    });
    assert!(
        doc.execute(Command::ApplyTransforms {
            ids: vec![id],
            parts: ApplyParts::ALL,
        })
        .is_err()
    );
    // A group's children stay put.
    let (mut doc, cube) = cube_placed(Transform::default());
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    doc.execute(Command::SetTransforms(vec![(group, stretched())]))
        .unwrap();
    doc.execute(Command::Move {
        ids: vec![cube],
        parent: Some(group),
        index: 0,
    })
    .unwrap();
    let before = world_points(&doc, cube);
    doc.execute(Command::ApplyTransforms {
        ids: vec![group],
        parts: ApplyParts::ALL,
    })
    .unwrap();
    same_points(&before, &world_points(&doc, cube));
}
