//! Tests for the operation stack: recording edits on top of a base,
//! evaluating them with a per-layer cache, and baking them into a mesh.

use crate::stack::{Rebuilt, StackCache};
use crate::*;
use glam::Vec3;
use mesh::CubeParams;
use std::sync::Arc;

fn cube() -> Primitive {
    Primitive::Cube(Default::default())
}

/// A document with one standard cube (selected), and its id.
fn with_a_cube() -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(cube())))
        .unwrap();
    let id = doc.selection()[0];
    (doc, id)
}

fn lift(points: &[u32], by: f32) -> Operation {
    Operation::MovePoints {
        points: points.iter().map(|n| VertexId::from_raw(*n)).collect(),
        offset: Vec3::Z * by,
        anchors: None,
    }
}

fn point(doc: &Document, id: ObjectId, n: u32) -> Vec3 {
    doc.scene()
        .evaluated(id)
        .unwrap()
        .mesh
        .position(VertexId::from_raw(n))
        .unwrap()
}

fn entry(id: u64, operation: Operation) -> Entry {
    Entry::new(EditId::from_raw(id), operation)
}

fn record(doc: &mut Document, id: ObjectId, operation: Operation) {
    doc.execute(Command::Record { id, operation }).unwrap();
}

// ------------------------------------------------------------ recording ---

#[test]
fn a_new_shape_has_no_edits_and_its_shape_is_its_base() {
    let (doc, id) = with_a_cube();
    let object = doc.scene().object(id).unwrap();
    assert!(object.stack.is_empty());
    let shape = doc.scene().evaluated(id).unwrap();
    assert_eq!(*shape.mesh, cube().mesh());
    assert!(shape.warnings.is_empty());
}

#[test]
fn a_recorded_edit_goes_into_an_edits_layer_and_changes_the_shape() {
    let (mut doc, id) = with_a_cube();
    let base = point(&doc, id, 0);
    record(&mut doc, id, lift(&[0], 25.0));
    let stack = &doc.scene().object(id).unwrap().stack;
    assert_eq!(stack.layers().len(), 1);
    assert_eq!(stack.layers()[0].name, EDITS_LAYER);
    assert!(stack.layers()[0].enabled);
    assert_eq!(stack.operation_count(), 1);
    assert_eq!(point(&doc, id, 0), base + Vec3::Z * 25.0);
    assert_eq!(doc.undo_label(), Some("Move 1 point"));
    // The base itself is untouched.
    assert_eq!(
        doc.scene().object(id).unwrap().kind,
        ObjectKind::Primitive(cube())
    );
}

#[test]
fn later_edits_join_the_same_layer_each_as_its_own_undo_step() {
    let (mut doc, id) = with_a_cube();
    let base = point(&doc, id, 0);
    let steps = doc.undo_steps();
    record(&mut doc, id, lift(&[0], 10.0));
    record(&mut doc, id, lift(&[0, 1], 5.0));
    assert_eq!(doc.undo_steps(), steps + 2);
    let stack = &doc.scene().object(id).unwrap().stack;
    assert_eq!(stack.layers().len(), 1);
    assert_eq!(stack.operation_count(), 2);
    assert_eq!(point(&doc, id, 0), base + Vec3::Z * 15.0);
    assert_eq!(doc.undo_label(), Some("Move 2 points"));

    doc.undo().unwrap();
    assert_eq!(point(&doc, id, 0), base + Vec3::Z * 10.0);
    doc.undo().unwrap();
    assert!(
        doc.scene().object(id).unwrap().stack.is_empty(),
        "the layer goes with its first edit"
    );
    assert_eq!(point(&doc, id, 0), base);
    doc.redo().unwrap();
    doc.redo().unwrap();
    assert_eq!(point(&doc, id, 0), base + Vec3::Z * 15.0);
}

#[test]
fn groups_have_no_shape_to_edit() {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    let before = doc.clone();
    let result = doc.execute(Command::Record {
        id: group,
        operation: lift(&[0], 1.0),
    });
    assert_eq!(result, Err(SceneError::NoShape));
    assert_eq!(doc, before, "nothing changed");
    assert_eq!(doc.scene().evaluated(group), None);
    assert_eq!(
        SceneError::NoShape.to_string(),
        "a group has no shape to edit"
    );
}

#[test]
fn recording_on_an_unknown_object_is_refused() {
    let (mut doc, id) = with_a_cube();
    doc.execute(Command::Delete(vec![id])).unwrap();
    assert_eq!(
        doc.execute(Command::Record {
            id,
            operation: lift(&[0], 1.0)
        }),
        Err(SceneError::UnknownObject(id))
    );
}

// ----------------------------------- the base stays editable (wish W4) ---

#[test]
fn changing_a_primitives_settings_replays_the_edits_on_the_new_shape() {
    let (mut doc, id) = with_a_cube();
    record(&mut doc, id, lift(&[0], 30.0));
    let taller = CubeParams {
        size: Vec3::new(100.0, 100.0, 300.0),
        ..CubeParams::default()
    };
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: Primitive::Cube(taller),
    })
    .unwrap();
    let new_base = Primitive::Cube(taller)
        .mesh()
        .position(VertexId::from_raw(0))
        .unwrap();
    assert_eq!(point(&doc, id, 0), new_base + Vec3::Z * 30.0);
    assert_eq!(doc.scene().object(id).unwrap().stack.operation_count(), 1);
    // Undoing the settings keeps the edit, on the old shape.
    doc.undo().unwrap();
    let old_base = cube().mesh().position(VertexId::from_raw(0)).unwrap();
    assert_eq!(point(&doc, id, 0), old_base + Vec3::Z * 30.0);
}

#[test]
fn edits_whose_points_are_gone_still_apply_to_the_rest_and_warn() {
    let (mut doc, id) = with_a_cube();
    let base = point(&doc, id, 1);
    // A standard cube has 8 points: 500 isn't one of them.
    record(&mut doc, id, lift(&[1, 500, 600], 10.0));
    let shape = doc.scene().evaluated(id).unwrap();
    assert_eq!(
        shape.mesh.position(VertexId::from_raw(1)),
        Some(base + Vec3::Z * 10.0)
    );
    assert_eq!(
        shape.warnings,
        [StackWarning {
            layer: 0,
            operation: 0,
            missing: 2,
            found_again: false,
            too_big: false,
        }]
    );
}

#[test]
fn fewer_segments_can_leave_an_edit_without_its_points() {
    let (mut doc, id) = with_a_cube();
    let busy = CubeParams {
        segments: [4, 4, 4],
        ..CubeParams::default()
    };
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: Primitive::Cube(busy),
    })
    .unwrap();
    let count = doc.scene().evaluated(id).unwrap().mesh.vertex_count() as u32;
    record(&mut doc, id, lift(&[count - 1], 10.0));
    assert!(doc.scene().evaluated(id).unwrap().warnings.is_empty());
    // Back to one segment: that last point no longer exists.
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: cube(),
    })
    .unwrap();
    let shape = doc.scene().evaluated(id).unwrap();
    assert_eq!(shape.warnings.len(), 1);
    assert_eq!(*shape.mesh, cube().mesh(), "nothing else changed");
}

#[test]
fn the_box_around_a_shape_follows_its_edits() {
    let (mut doc, id) = with_a_cube();
    assert_eq!(doc.scene().local_bounds(id), Some(cube().bounds()));
    // Lift the four top corners (z = 100) by 50.
    let top: Vec<u32> = cube()
        .mesh()
        .vertices()
        .filter(|(_, p)| p.z > 50.0)
        .map(|(v, _)| v.get())
        .collect();
    assert_eq!(top.len(), 4);
    record(&mut doc, id, lift(&top, 50.0));
    let bounds = doc.scene().local_bounds(id).unwrap();
    assert_eq!(bounds.max.z, 150.0);
    assert_eq!(doc.scene().bounds().unwrap().max.z, 150.0);
}

// -------------------------------------------------------------- caching ---

#[test]
fn an_unchanged_object_is_not_evaluated_again() {
    let (mut doc, id) = with_a_cube();
    record(&mut doc, id, lift(&[0], 5.0));
    let first = doc.scene().evaluated(id).unwrap();
    assert_eq!(doc.scene().last_rebuilt(id), Some(Rebuilt::Everything));
    let again = doc.scene().evaluated(id).unwrap();
    assert_eq!(doc.scene().last_rebuilt(id), Some(Rebuilt::Nothing));
    assert!(Arc::ptr_eq(&first.mesh, &again.mesh), "the same mesh");
    // Renaming or moving the object doesn't change its shape either.
    doc.execute(Command::Rename {
        id,
        name: "Box".into(),
    })
    .unwrap();
    let renamed = doc.scene().evaluated(id).unwrap();
    assert!(Arc::ptr_eq(&first.mesh, &renamed.mesh));
}

#[test]
fn a_new_edit_reevaluates_only_its_layer_and_a_new_base_everything() {
    let (mut doc, id) = with_a_cube();
    record(&mut doc, id, lift(&[0], 5.0));
    let _ = doc.scene().evaluated(id);
    record(&mut doc, id, lift(&[1], 5.0));
    let _ = doc.scene().evaluated(id);
    assert_eq!(
        doc.scene().last_rebuilt(id),
        Some(Rebuilt::From {
            layer: 0,
            operations: 1
        }),
        "only the new edit is applied"
    );
    let mut wide = CubeParams::default();
    wide.size.x = 250.0;
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: Primitive::Cube(wide),
    })
    .unwrap();
    let _ = doc.scene().evaluated(id);
    assert_eq!(doc.scene().last_rebuilt(id), Some(Rebuilt::Everything));
}

fn two_layers() -> Stack {
    let mut lower = Layer::new("Lower");
    lower.entries.push(entry(1, lift(&[0], 10.0)));
    let mut upper = Layer::new("Upper");
    upper.entries.push(entry(2, lift(&[0], 1.0)));
    Stack {
        layers: vec![lower, upper],
    }
}

#[test]
fn changing_an_upper_layer_keeps_the_layers_below_it() {
    let mut cache = StackCache::default();
    let id = ObjectId(1);
    let base = ObjectKind::Primitive(cube());
    let context = EvalContext::default();
    let mut stack = two_layers();
    let first = cache.evaluate(id, &base, &stack, &context).unwrap();
    let start = cube().mesh().position(VertexId::from_raw(0)).unwrap();
    assert_eq!(
        first.mesh.position(VertexId::from_raw(0)),
        Some(start + Vec3::Z * 11.0)
    );
    stack.layers[1].entries.push(entry(3, lift(&[0], 100.0)));
    let second = cache.evaluate(id, &base, &stack, &context).unwrap();
    assert_eq!(
        cache.last_rebuilt(id),
        Some(Rebuilt::From {
            layer: 1,
            operations: 1
        }),
        "only the new edit on the top"
    );
    assert_eq!(
        second.mesh.position(VertexId::from_raw(0)),
        Some(start + Vec3::Z * 111.0)
    );
    // Changing the lower layer evaluates both again.
    stack.layers[0].entries.clear();
    let third = cache.evaluate(id, &base, &stack, &context).unwrap();
    assert_eq!(
        cache.last_rebuilt(id),
        Some(Rebuilt::From {
            layer: 0,
            operations: 2
        }),
        "the top layer starts over on the new shape"
    );
    assert_eq!(
        third.mesh.position(VertexId::from_raw(0)),
        Some(start + Vec3::Z * 101.0)
    );
    // Taking the top layer away.
    stack.layers.pop();
    let fourth = cache.evaluate(id, &base, &stack, &context).unwrap();
    assert_eq!(*fourth.mesh, cube().mesh());
}

#[test]
fn a_switched_off_layer_passes_the_shape_through() {
    let mut cache = StackCache::default();
    let base = ObjectKind::Primitive(cube());
    let mut stack = two_layers();
    stack.layers[0].enabled = false;
    stack.layers[1].enabled = false;
    let shape = cache
        .evaluate(ObjectId(1), &base, &stack, &EvalContext::default())
        .unwrap();
    assert_eq!(*shape.mesh, cube().mesh());
    stack.layers[0].enabled = true;
    let shape = cache
        .evaluate(ObjectId(1), &base, &stack, &EvalContext::default())
        .unwrap();
    let start = cube().mesh().position(VertexId::from_raw(0)).unwrap();
    assert_eq!(
        shape.mesh.position(VertexId::from_raw(0)),
        Some(start + Vec3::Z * 10.0)
    );
}

#[test]
fn a_mesh_base_is_used_as_it_is_and_shared() {
    let mut cache = StackCache::default();
    let mesh = Arc::new(cube().mesh());
    let base = ObjectKind::Mesh(Arc::clone(&mesh));
    let shape = cache
        .evaluate(
            ObjectId(3),
            &base,
            &Stack::default(),
            &EvalContext::default(),
        )
        .unwrap();
    assert!(Arc::ptr_eq(&shape.mesh, &mesh), "no copy without edits");
    assert_eq!(
        cache.evaluate(
            ObjectId(4),
            &ObjectKind::Group,
            &Stack::default(),
            &EvalContext::default()
        ),
        None
    );
}

#[test]
fn deleting_and_undoing_brings_the_same_shape_back() {
    let (mut doc, id) = with_a_cube();
    record(&mut doc, id, lift(&[2], 7.0));
    let before = doc.scene().evaluated(id).unwrap();
    doc.execute(Command::Delete(vec![id])).unwrap();
    assert_eq!(doc.scene().evaluated(id), None);
    doc.undo().unwrap();
    assert_eq!(doc.scene().evaluated(id), Some(before));
}

#[test]
fn a_copied_scene_is_equal_and_works_out_the_same_shapes() {
    let (mut doc, id) = with_a_cube();
    record(&mut doc, id, lift(&[2], 7.0));
    let _ = doc.scene().evaluated(id);
    let copy = doc.scene().clone();
    assert_eq!(copy, *doc.scene());
    assert_eq!(copy.evaluated(id), doc.scene().evaluated(id));
}

#[test]
fn the_same_history_gives_the_same_shape() {
    let build = || {
        let (mut doc, id) = with_a_cube();
        record(&mut doc, id, lift(&[0, 3, 5], 12.5));
        record(&mut doc, id, lift(&[3], -4.0));
        doc.scene().evaluated(id).unwrap()
    };
    assert_eq!(build(), build());
}

// --------------------------------------------------------- make editable ---

#[test]
fn make_editable_bakes_settings_and_edits_into_a_mesh() {
    let (mut doc, id) = with_a_cube();
    record(&mut doc, id, lift(&[0], 20.0));
    let shape = doc.scene().evaluated(id).unwrap();
    let command = Command::MakeEditable(vec![id]);
    assert_eq!(command.describe(doc.scene()), "Bake into shape");
    doc.execute(command).unwrap();
    let object = doc.scene().object(id).unwrap();
    assert!(
        object.stack.is_empty(),
        "the edits are part of the mesh now"
    );
    assert_eq!(object.name, "Cube", "the name stays");
    match &object.kind {
        ObjectKind::Mesh(mesh) => assert_eq!(**mesh, *shape.mesh),
        other => panic!("not a mesh: {other:?}"),
    }
    assert_eq!(doc.scene().evaluated(id).unwrap().mesh, shape.mesh);
    assert_eq!(doc.undo_label(), Some("Bake into shape"));
    // One undo brings the settings and the edits back.
    doc.undo().unwrap();
    let object = doc.scene().object(id).unwrap();
    assert_eq!(object.kind, ObjectKind::Primitive(cube()));
    assert_eq!(object.stack.operation_count(), 1);
    doc.redo().unwrap();
    assert!(matches!(
        doc.scene().object(id).unwrap().kind,
        ObjectKind::Mesh(_)
    ));
}

#[test]
fn a_mesh_takes_new_edits_and_can_be_made_editable_again() {
    let (mut doc, id) = with_a_cube();
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    let steps = doc.undo_steps();
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    assert_eq!(doc.undo_steps(), steps, "nothing to bake: no step");
    record(&mut doc, id, lift(&[0], 3.0));
    let shape = doc.scene().evaluated(id).unwrap();
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    assert_eq!(doc.undo_steps(), steps + 2);
    assert!(doc.scene().object(id).unwrap().stack.is_empty());
    assert_eq!(*doc.scene().evaluated(id).unwrap().mesh, *shape.mesh);
}

#[test]
fn make_editable_skips_groups_and_handles_several_shapes() {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(cube())))
        .unwrap();
    let a = doc.selection()[0];
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Sphere.standard(),
    )))
    .unwrap();
    let b = doc.selection()[0];
    doc.execute(Command::Group(vec![a])).unwrap();
    let group = doc.selection()[0];
    let command = Command::MakeEditable(vec![group, a, b]);
    assert_eq!(command.describe(doc.scene()), "Bake 2 objects into shape");
    doc.execute(command).unwrap();
    assert_eq!(doc.scene().object(group).unwrap().kind, ObjectKind::Group);
    for id in [a, b] {
        assert!(matches!(
            doc.scene().object(id).unwrap().kind,
            ObjectKind::Mesh(_)
        ));
    }
    doc.undo().unwrap();
    assert_eq!(
        doc.scene().object(b).unwrap().kind,
        ObjectKind::Primitive(PrimitiveKind::Sphere.standard()),
        "one step for all of them"
    );
}

#[test]
fn settings_cant_be_given_to_a_mesh() {
    let (mut doc, id) = with_a_cube();
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    assert_eq!(
        doc.execute(Command::SetPrimitive {
            ids: vec![id],
            primitive: cube(),
        }),
        Err(SceneError::WrongKind("Cube"))
    );
}

#[test]
fn make_editable_with_nothing_or_unknown_objects() {
    let (mut doc, id) = with_a_cube();
    let steps = doc.undo_steps();
    doc.execute(Command::MakeEditable(Vec::new())).unwrap();
    assert_eq!(doc.undo_steps(), steps);
    let ghost = ObjectId(99);
    assert_eq!(
        doc.execute(Command::MakeEditable(vec![id, ghost])),
        Err(SceneError::UnknownObject(ghost))
    );
    assert_eq!(
        doc.scene().object(id).unwrap().kind,
        ObjectKind::Primitive(cube()),
        "refused as a whole"
    );
}

// ------------------------------------- finding points again (0032) ---

/// Records lifting the cube's top polygon by 50 cm, with anchors, as the
/// move gizmo does.
fn lift_top_with_anchors(doc: &mut Document, id: ObjectId) {
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let top = shape
        .faces()
        .find(|(f, _)| shape.face_normal(*f).z > 0.9)
        .map(|(_, c)| c.to_vec())
        .unwrap();
    let anchors = mesh::Anchors::record(
        &shape,
        mesh::ElementKind::Polygons,
        std::slice::from_ref(&top),
    );
    doc.execute(Command::Record {
        id,
        operation: Operation::MovePoints {
            points: top,
            offset: Vec3::Z * 50.0,
            anchors,
        },
    })
    .unwrap();
}

fn segments(doc: &mut Document, id: ObjectId, n: u32) {
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: Primitive::Cube(CubeParams {
            segments: [n; 3],
            ..CubeParams::default()
        }),
    })
    .unwrap();
}

#[test]
fn a_lifted_top_stays_lifted_with_more_segments() {
    let (mut doc, id) = with_a_cube();
    lift_top_with_anchors(&mut doc, id);
    let shape = doc.scene().evaluated(id).unwrap();
    assert!(shape.warnings.is_empty());
    let tall = |m: &Mesh| {
        m.vertices()
            .filter(|(_, p)| (p.z - 150.0).abs() < 1e-3)
            .count()
    };
    assert_eq!(tall(&shape.mesh), 4);

    // More segments: the points are renumbered, but the whole top, now
    // nine points, is found again and lifted.
    segments(&mut doc, id, 2);
    let shape = doc.scene().evaluated(id).unwrap();
    assert_eq!(tall(&shape.mesh), 9);
    assert!(
        shape.mesh.vertices().all(|(_, p)| p.z < 1e-3 || p.z > 49.0),
        "nothing else moved up to the top: the middle row stays at 50"
    );
    assert_eq!(
        shape.warnings,
        [StackWarning {
            layer: 0,
            operation: 0,
            missing: 0,
            found_again: true,
            too_big: false,
        }]
    );

    // A taller cube keeps the same numbering: no searching needed.
    segments(&mut doc, id, 1);
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: Primitive::Cube(CubeParams {
            size: Vec3::new(100.0, 100.0, 300.0),
            ..CubeParams::default()
        }),
    })
    .unwrap();
    let shape = doc.scene().evaluated(id).unwrap();
    assert!(shape.warnings.is_empty());
    assert_eq!(
        shape
            .mesh
            .vertices()
            .filter(|(_, p)| (p.z - 350.0).abs() < 1e-3)
            .count(),
        4
    );
}

#[test]
fn without_anchors_edits_keep_naming_points_by_id() {
    let (mut doc, id) = with_a_cube();
    doc.execute(Command::Record {
        id,
        operation: lift(&[0], 10.0),
    })
    .unwrap();
    segments(&mut doc, id, 2);
    let shape = doc.scene().evaluated(id).unwrap();
    assert!(shape.warnings.iter().all(|w| !w.found_again));
}

// ------------------------------------------------- extrude and inset ---

/// The cube's polygon facing up.
fn top_face(mesh: &Mesh) -> FaceId {
    mesh.faces()
        .map(|(f, _)| f)
        .max_by(|a, b| mesh.face_normal(*a).z.total_cmp(&mesh.face_normal(*b).z))
        .unwrap()
}

fn extrude_top(doc: &Document, id: ObjectId, distance: f32, anchored: bool) -> Operation {
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let top = top_face(&mesh);
    let corners = vec![mesh.face(top).unwrap().to_vec()];
    Operation::ExtrudePolygons {
        faces: vec![top],
        distance,
        grouping: Grouping::Together,
        shape: crate::ExtrudeShape::EARLIER,
        anchors: anchored
            .then(|| mesh::Anchors::record(&mesh, ElementKind::Polygons, &corners))
            .flatten(),
        stretch: glam::Vec3::ONE,
    }
}

#[test]
fn an_extrude_is_recorded_undone_and_redone() {
    let (mut doc, id) = with_a_cube();
    let op = extrude_top(&doc, id, 30.0, true);
    record(&mut doc, id, op);
    assert_eq!(doc.undo_label(), Some("Extrude 1 polygon"));
    let shape = doc.scene().evaluated(id).unwrap();
    assert_eq!(shape.mesh.face_count(), 10);
    assert!((shape.mesh.bounds().unwrap().max.z - 130.0).abs() < 1e-3);
    assert!(shape.warnings.is_empty());
    doc.undo().unwrap();
    assert_eq!(doc.scene().evaluated(id).unwrap().mesh.face_count(), 6);
    doc.redo().unwrap();
    assert_eq!(doc.scene().evaluated(id).unwrap().mesh.face_count(), 10);
}

#[test]
fn an_extrudes_distance_can_be_changed_afterwards() {
    let (mut doc, id) = with_a_cube();
    let op = extrude_top(&doc, id, 30.0, true);
    record(&mut doc, id, op.clone());
    let changed = op.with_settings(50.0, Grouping::Separate);
    assert_eq!(changed.distance(), Some(50.0));
    assert_eq!(changed.grouping(), Some(Grouping::Separate));
    doc.execute(Command::ReplaceOperation {
        id,
        layer: 0,
        index: 0,
        operation: changed.clone(),
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Change Extrude 1 polygon"));
    let top = doc
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh
        .bounds()
        .unwrap()
        .max
        .z;
    assert!((top - 150.0).abs() < 1e-3);
    assert_eq!(
        doc.scene().object(id).unwrap().stack.layers()[0]
            .entries
            .iter()
            .map(|e| e.operation.clone())
            .collect::<Vec<_>>(),
        vec![changed.clone()]
    );
    // Undo puts the old distance back; the same value again is no step.
    doc.undo().unwrap();
    let top = doc
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh
        .bounds()
        .unwrap()
        .max
        .z;
    assert!((top - 130.0).abs() < 1e-3);
    let steps = doc.undo_steps();
    doc.execute(Command::ReplaceOperation {
        id,
        layer: 0,
        index: 0,
        operation: op,
    })
    .unwrap();
    assert_eq!(doc.undo_steps(), steps);
    // An edit that isn't there is refused.
    assert!(
        doc.execute(Command::ReplaceOperation {
            id,
            layer: 0,
            index: 5,
            operation: changed,
        })
        .is_err()
    );
}

#[test]
fn an_extrude_replays_when_the_cube_gets_more_segments() {
    let (mut doc, id) = with_a_cube();
    let op = extrude_top(&doc, id, 30.0, true);
    record(&mut doc, id, op);
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: Primitive::Cube(CubeParams {
            segments: [2, 2, 2],
            ..Default::default()
        }),
    })
    .unwrap();
    let shape = doc.scene().evaluated(id).unwrap();
    // The top, now four polygons, was found again and extruded as one
    // block: 24 polygons, plus 8 walls.
    assert_eq!(shape.mesh.face_count(), 24 + 8);
    assert!((shape.mesh.bounds().unwrap().max.z - 130.0).abs() < 1e-3);
    assert!(
        shape
            .warnings
            .iter()
            .all(|w| w.missing == 0 && w.found_again)
    );
    assert!(mesh::MeshReport::new(&shape.mesh).is_closed());
}

#[test]
fn inset_and_edge_extrude_are_recorded_too() {
    let (mut doc, id) = with_a_cube();
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let top = top_face(&mesh);
    record(
        &mut doc,
        id,
        Operation::InsetPolygons {
            faces: vec![top],
            distance: 10.0,
            grouping: Grouping::Together,
            anchors: None,
            stretch: glam::Vec3::ONE,
        },
    );
    assert_eq!(doc.undo_label(), Some("Inset 1 polygon"));
    assert_eq!(doc.scene().evaluated(id).unwrap().mesh.face_count(), 10);
    // A cube has no open border: an edge extrude changes nothing.
    let edge = mesh.edges()[0];
    record(
        &mut doc,
        id,
        Operation::ExtrudeEdges {
            edges: vec![edge],
            distance: 10.0,
            grouping: Grouping::Together,
            anchors: None,
            stretch: glam::Vec3::ONE,
        },
    );
    assert_eq!(doc.undo_label(), Some("Extrude 1 edge"));
    assert_eq!(doc.scene().evaluated(id).unwrap().mesh.face_count(), 10);
}

#[test]
fn make_editable_keeps_an_extrude() {
    let (mut doc, id) = with_a_cube();
    let op = extrude_top(&doc, id, 30.0, false);
    record(&mut doc, id, op);
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    let object = doc.scene().object(id).unwrap();
    assert!(object.stack.is_empty());
    let ObjectKind::Mesh(mesh) = &object.kind else {
        panic!("a mesh now");
    };
    assert_eq!(mesh.face_count(), 10);
}

// ---------------------------------------------------------------- bevel ---

/// The cube's edges around its top, as the stack would record them.
fn bevel_top(doc: &Document, id: ObjectId, width: f32, segments: u32) -> Operation {
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let edges: Vec<EdgeKey> = mesh
        .edges()
        .iter()
        .filter(|e| {
            let (a, b) = e.ends();
            mesh.position(a).unwrap().z > 99.0 && mesh.position(b).unwrap().z > 99.0
        })
        .copied()
        .collect();
    let ends: Vec<Vec<VertexId>> = edges
        .iter()
        .map(|e| {
            let (a, b) = e.ends();
            vec![a, b]
        })
        .collect();
    Operation::BevelEdges {
        edges,
        shape: BevelShape {
            width,
            segments,
            profile: 1.0,
            options: mesh::edit::BevelOptions::EARLIER,
        },
        anchors: mesh::Anchors::record(&mesh, ElementKind::Edges, &ends),
        stretch: glam::Vec3::ONE,
    }
}

#[test]
fn a_bevel_is_recorded_changed_and_replayed() {
    let (mut doc, id) = with_a_cube();
    let op = bevel_top(&doc, id, 10.0, 2);
    record(&mut doc, id, op.clone());
    assert_eq!(doc.undo_label(), Some("Bevel 4 edges"));
    let faces = |doc: &Document| doc.scene().evaluated(id).unwrap().mesh.face_count();
    assert_eq!(faces(&doc), 6 + 8);
    // The Last edit: width, rows and profile, no grouping.
    let settings = op.settings().unwrap();
    assert_eq!(settings.distance, Some(10.0));
    assert_eq!(settings.segments, Some(2));
    assert_eq!(settings.profile, Some(1.0));
    assert_eq!(settings.grouping, None);
    let changed = op.with(EditSettings {
        segments: Some(4),
        ..settings
    });
    doc.execute(Command::ReplaceOperation {
        id,
        layer: 0,
        index: 0,
        operation: changed,
    })
    .unwrap();
    assert_eq!(faces(&doc), 6 + 16);
    doc.undo().unwrap();
    assert_eq!(faces(&doc), 6 + 8);
    // More segments on the cube: the top's edges are found again along
    // where they were, and the bevel follows.
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: Primitive::Cube(CubeParams {
            segments: [2, 2, 2],
            ..Default::default()
        }),
    })
    .unwrap();
    let shape = doc.scene().evaluated(id).unwrap();
    assert!(shape.warnings.iter().all(|w| w.missing == 0));
    assert!(mesh::MeshReport::new(&shape.mesh).is_closed());
    assert!(shape.mesh.face_count() > 24);
}

#[test]
fn a_point_bevel_is_recorded() {
    let (mut doc, id) = with_a_cube();
    record(
        &mut doc,
        id,
        Operation::BevelPoints {
            points: vec![VertexId::from_raw(0)],
            width: 10.0,
            anchors: None,
            stretch: glam::Vec3::ONE,
        },
    );
    assert_eq!(doc.undo_label(), Some("Bevel 1 point"));
    let shape = doc.scene().evaluated(id).unwrap();
    assert_eq!(shape.mesh.face_count(), 7);
    assert!(mesh::MeshReport::new(&shape.mesh).is_closed());
    // Moves and turns have no settings to change afterwards.
    assert!(lift(&[0], 1.0).settings().is_none());
}

/// A loop cut across the cube's top front edge (along X), with anchors.
fn loop_cut_top_front(doc: &Document, id: ObjectId, cuts: u32, position: f32) -> Operation {
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let edge = *mesh
        .edges()
        .iter()
        .find(|e| {
            let (a, b) = e.ends();
            let (pa, pb) = (mesh.position(a).unwrap(), mesh.position(b).unwrap());
            pa.y < -49.0 && pb.y < -49.0 && pa.z > 99.0 && pb.z > 99.0
        })
        .unwrap();
    let (a, b) = edge.ends();
    Operation::LoopCut {
        edge,
        cuts,
        position,
        anchors: mesh::Anchors::record(&mesh, ElementKind::Edges, &[vec![a, b]]),
    }
}

/// The X of every point the cube's shape has beyond its first `before`.
fn new_xs(doc: &Document, id: ObjectId, before: usize) -> Vec<f32> {
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    mesh.vertices().skip(before).map(|(_, p)| p.x).collect()
}

#[test]
fn a_loop_cut_is_recorded_changed_and_replayed() {
    let (mut doc, id) = with_a_cube();
    let op = loop_cut_top_front(&doc, id, 1, 0.5);
    record(&mut doc, id, op.clone());
    assert_eq!(doc.undo_label(), Some("Loop cut"));
    let faces = |doc: &Document| doc.scene().evaluated(id).unwrap().mesh.face_count();
    assert_eq!(faces(&doc), 10);
    // The Last edit: cuts and position, nothing else.
    let settings = op.settings().unwrap();
    assert_eq!(settings.cuts, Some(1));
    assert_eq!(settings.position, Some(0.5));
    assert_eq!(settings.distance, None);
    assert_eq!(settings.segments, None);
    let changed = op.with(EditSettings {
        cuts: Some(3),
        ..settings
    });
    assert_eq!(changed.describe(), "Loop cut (3 cuts)");
    doc.execute(Command::ReplaceOperation {
        id,
        layer: 0,
        index: 0,
        operation: changed,
    })
    .unwrap();
    assert_eq!(faces(&doc), 6 + 12);
    doc.undo().unwrap();
    assert_eq!(faces(&doc), 10);
    // A loop cut keeps only what it says.
    assert_eq!(
        crate::memory::operation_size(&op),
        op.anchors().unwrap().memory_size()
    );
}

#[test]
fn a_loop_cut_stays_on_its_side_when_the_points_are_renumbered() {
    let (mut doc, id) = with_a_cube();
    let op = loop_cut_top_front(&doc, id, 1, 0.25);
    let Operation::LoopCut { edge, .. } = &op else {
        unreachable!()
    };
    let low = point(&doc, id, edge.ends().0.get());
    let want = low.x * 0.5;
    record(&mut doc, id, op.clone());
    assert!(new_xs(&doc, id, 8).iter().all(|x| (x - want).abs() < 1e-3));
    // More segments up and across (not along the cut edge): every point
    // is renumbered, and the cut is found again, on the same side.
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: Primitive::Cube(CubeParams {
            segments: [1, 3, 2],
            ..Default::default()
        }),
    })
    .unwrap();
    let shape = doc.scene().evaluated(id).unwrap();
    assert!(
        shape.warnings.iter().all(|w| w.missing == 0),
        "{:?}",
        shape.warnings
    );
    let before = Primitive::Cube(CubeParams {
        segments: [1, 3, 2],
        ..Default::default()
    })
    .mesh()
    .vertex_count();
    let xs = new_xs(&doc, id, before);
    assert!(!xs.is_empty());
    assert!(
        xs.iter().all(|x| (x - want).abs() < 1e-3),
        "{xs:?}, want {want}"
    );
    assert!(mesh::MeshReport::new(&shape.mesh).is_closed());
}

#[test]
fn a_loop_cut_whose_edge_was_split_follows_a_piece_of_it() {
    let (mut doc, id) = with_a_cube();
    let op = loop_cut_top_front(&doc, id, 1, 0.5);
    record(&mut doc, id, op);
    // Two segments along the edge: it is now two edges. The cut is found
    // again across one of them, and the shape stays whole.
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: Primitive::Cube(CubeParams {
            segments: [2, 1, 1],
            ..Default::default()
        }),
    })
    .unwrap();
    let shape = doc.scene().evaluated(id).unwrap();
    assert!(
        shape
            .warnings
            .iter()
            .all(|w| w.found_again && w.missing == 0)
    );
    assert_eq!(shape.mesh.face_count(), 10 + 4);
    let report = mesh::MeshReport::new(&shape.mesh);
    assert!(report.is_closed() && !report.has_problems(), "{report:?}");
    // A position that isn't a number is refused.
    assert!(!loop_cut_top_front(&doc, id, 1, f32::NAN).numbers_are_real());
}

#[test]
fn a_loop_cut_whose_edge_is_gone_is_missing() {
    let (mut doc, id) = with_a_cube();
    let mut op = loop_cut_top_front(&doc, id, 1, 0.5);
    if let Operation::LoopCut { edge, anchors, .. } = &mut op {
        *edge = mesh::EdgeKey::new(VertexId::from_raw(0), VertexId::from_raw(6));
        *anchors = None;
    }
    record(&mut doc, id, op);
    let shape = doc.scene().evaluated(id).unwrap();
    assert_eq!(shape.mesh.face_count(), 6);
    assert_eq!(shape.warnings.len(), 1);
    assert_eq!(shape.warnings[0].missing, 1);
}

/// A slit across the cube's front at 30 cm high, seen from in front.
fn slit_front(through: bool) -> Operation {
    Operation::Slit {
        line: SlitLine {
            points: vec![Vec3::new(-80.0, -50.0, 30.0), Vec3::new(80.0, -50.0, 30.0)],
            eye: Eye::At(Vec3::new(0.0, -500.0, 30.0)),
            through,
        },
    }
}

#[test]
fn a_slit_is_recorded_changed_and_follows_the_shape() {
    let (mut doc, id) = with_a_cube();
    record(&mut doc, id, slit_front(false));
    assert_eq!(doc.undo_label(), Some("Slit"));
    let faces = |doc: &Document| doc.scene().evaluated(id).unwrap().mesh.face_count();
    assert_eq!(faces(&doc), 7);
    // The Last edit: only Cut through.
    let settings = slit_front(false).settings().unwrap();
    assert_eq!(settings.through, Some(false));
    assert_eq!((settings.distance, settings.cuts), (None, None));
    let changed = slit_front(false).with(EditSettings {
        through: Some(true),
        ..settings
    });
    assert_eq!(changed.describe(), "Slit through");
    doc.execute(Command::ReplaceOperation {
        id,
        layer: 0,
        index: 0,
        operation: changed,
    })
    .unwrap();
    assert_eq!(faces(&doc), 10);
    // More height segments: the slit still cuts at 30 cm, through the
    // lower row.
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: Primitive::Cube(CubeParams {
            segments: [1, 1, 2],
            ..Default::default()
        }),
    })
    .unwrap();
    let shape = doc.scene().evaluated(id).unwrap();
    assert_eq!(shape.mesh.face_count(), 10 + 4);
    assert!(shape.warnings.is_empty(), "{:?}", shape.warnings);
    assert!(mesh::MeshReport::new(&shape.mesh).is_closed());
    let low = shape
        .mesh
        .vertices()
        .filter(|(_, p)| (p.z - 30.0).abs() < 1e-3)
        .count();
    assert_eq!(low, 4);
    // Numbers that aren't real are refused, and a slit keeps only its
    // points.
    let mut broken = slit_front(false);
    if let Operation::Slit { line } = &mut broken {
        line.eye = Eye::At(Vec3::NAN);
    }
    assert!(!broken.numbers_are_real());
    let op = slit_front(true);
    assert_eq!(
        crate::memory::operation_size(&op),
        2 * std::mem::size_of::<Vec3>()
    );
    assert!(op.anchors().is_none());
}

#[test]
fn a_straight_extrude_given_a_bend_becomes_a_bent_one() {
    let (mut doc, id) = with_a_cube();
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let top = mesh
        .faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).z > 0.9)
        .unwrap();
    let straight = Operation::ExtrudePolygons {
        faces: vec![top],
        distance: 50.0,
        grouping: Grouping::Together,
        shape: crate::ExtrudeShape::EARLIER,
        anchors: None,
        stretch: glam::Vec3::ONE,
    };
    record(&mut doc, id, straight.clone());
    // A straight extrude shows a bend that doesn't turn.
    let settings = straight.settings().unwrap();
    assert!(!settings.bend.unwrap().bends());
    let bent = straight.with(EditSettings {
        bend: Some(Bend {
            angle: 90.0,
            radius: 0.0,
            ..Bend::default()
        }),
        ..settings
    });
    assert!(matches!(bent, Operation::ExtrudeBent { distance, .. } if distance == 50.0));
    assert_eq!(bent.describe(), "Extrude 1 polygon with a bend");
    let spiral = straight.with(EditSettings {
        bend: Some(Bend {
            angle: 720.0,
            radius: 30.0,
            ..Bend::default()
        }),
        ..settings
    });
    assert_eq!(spiral.describe(), "Extrude 1 polygon with a spiral");
    doc.execute(Command::ReplaceOperation {
        id,
        layer: 0,
        index: 0,
        operation: bent.clone(),
    })
    .unwrap();
    let shape = doc.scene().evaluated(id).unwrap();
    assert!(mesh::MeshReport::new(&shape.mesh).is_closed());
    assert_eq!(shape.mesh.face_count(), 6 + 4);
    // Its own settings change it; numbers that aren't real are refused.
    let wider = bent.with(EditSettings {
        bend: Some(Bend {
            angle: 90.0,
            radius: 40.0,
            segments: 5,
            direction: 0.0,
            ..Bend::default()
        }),
        ..bent.settings().unwrap()
    });
    let Operation::ExtrudeBent { bend, .. } = &wider else {
        panic!("still bent");
    };
    assert_eq!(bend.segments, 5);
    let mut broken = wider.clone();
    if let Operation::ExtrudeBent { bend, .. } = &mut broken {
        bend.radius = f32::NAN;
    }
    assert!(!broken.numbers_are_real());
    assert_eq!(
        crate::memory::operation_size(&wider),
        std::mem::size_of::<FaceId>()
    );
}

#[test]
fn a_bend_keeps_the_stretch_it_is_given() {
    let (mut doc, id) = with_a_cube();
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let top = mesh
        .faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).z > 0.9)
        .unwrap();
    let straight = Operation::ExtrudePolygons {
        faces: vec![top],
        distance: 30.0,
        grouping: Grouping::Together,
        shape: crate::ExtrudeShape::default(),
        anchors: None,
        stretch: glam::Vec3::ONE,
    };
    let turn = Bend {
        angle: 90.0,
        radius: 20.0,
        segments: 4,
        ..Bend::default()
    };
    let stretch = glam::Vec3::new(0.75, 0.75, 1.5);
    // A straight extrude given a bend and a stretch takes both.
    let bent = straight.with(EditSettings {
        bend: Some(turn),
        stretch: Some(stretch),
        ..straight.settings().unwrap()
    });
    let Operation::ExtrudeBent { stretch: kept, .. } = &bent else {
        panic!("bent");
    };
    assert_eq!(*kept, stretch);
    assert_eq!(bent.settings().unwrap().stretch, Some(stretch));
    // Without a stretch it keeps what it has; a broken one counts as none.
    let same = bent.with(EditSettings {
        stretch: None,
        ..bent.settings().unwrap()
    });
    assert_eq!(same, bent);
    let broken = bent.with(EditSettings {
        stretch: Some(glam::Vec3::new(f32::NAN, 1.0, 1.0)),
        ..bent.settings().unwrap()
    });
    assert!(matches!(broken, Operation::ExtrudeBent { stretch, .. } if stretch == glam::Vec3::ONE));
    // The shape is the stretched bend's.
    record(&mut doc, id, bent);
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let mut expected = (*mesh).clone();
    mesh::edit::extrude_bent_stretched(
        &mut expected,
        &[top],
        30.0,
        Grouping::Together,
        turn,
        stretch,
    );
    assert_eq!(shape.vertex_count(), expected.vertex_count());
    for (v, p) in expected.vertices() {
        assert!(shape.position(v).unwrap().distance(p) < 1e-3, "{v:?}");
    }
    let mut plain = (*mesh).clone();
    mesh::edit::extrude_bent(&mut plain, &[top], 30.0, Grouping::Together, turn);
    assert!(
        plain
            .vertices()
            .any(|(v, p)| shape.position(v).unwrap().distance(p) > 1.0)
    );
}

#[test]
fn a_stretch_comes_from_the_objects_scale() {
    use glam::{Mat4, Quat, Vec3};
    assert_eq!(crate::stretch_of(Mat4::IDENTITY), Vec3::ONE);
    let even = Mat4::from_scale_rotation_translation(
        Vec3::splat(3.0),
        Quat::from_rotation_z(0.7),
        Vec3::X,
    );
    assert_eq!(crate::stretch_of(even), Vec3::ONE);
    let tall = Mat4::from_scale_rotation_translation(
        Vec3::new(1.0, 1.0, 2.0),
        Quat::from_rotation_x(1.1),
        Vec3::new(4.0, 5.0, 6.0),
    );
    assert!((crate::stretch_of(tall) - Vec3::new(0.75, 0.75, 1.5)).length() < 1e-5);
    for broken in [
        Mat4::ZERO,
        Mat4::from_scale(Vec3::new(f32::NAN, 1.0, 1.0)),
        Mat4::from_scale(Vec3::new(1e-9, 1.0, 1.0)),
    ] {
        assert_eq!(crate::stretch_of(broken), Vec3::ONE);
    }
}

#[test]
fn connect_and_weld_are_recorded_and_follow_the_shape() {
    let (mut doc, id) = with_a_cube();
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let upright: Vec<mesh::EdgeKey> = mesh
        .edges()
        .iter()
        .copied()
        .filter(|e| {
            let (a, b) = e.ends();
            (mesh.position(a).unwrap().z - mesh.position(b).unwrap().z).abs() > 1.0
        })
        .collect();
    let ends: Vec<Vec<VertexId>> = upright
        .iter()
        .map(|e| {
            let (a, b) = e.ends();
            vec![a, b]
        })
        .collect();
    let connect = Operation::Connect {
        points: Vec::new(),
        edges: upright.clone(),
        cuts: 1,
        anchors: mesh::Anchors::record(&mesh, ElementKind::Edges, &ends),
    };
    record(&mut doc, id, connect.clone());
    assert_eq!(doc.undo_label(), Some("Connect 4 edges"));
    let faces = |doc: &Document| doc.scene().evaluated(id).unwrap().mesh.face_count();
    assert_eq!(faces(&doc), 10);
    // Cuts is its Last edit.
    let more = connect.with(EditSettings {
        cuts: Some(3),
        ..connect.settings().unwrap()
    });
    doc.execute(Command::ReplaceOperation {
        id,
        layer: 0,
        index: 0,
        operation: more,
    })
    .unwrap();
    assert_eq!(faces(&doc), 6 + 12);
    // Weld the top's corners into one: a roof.
    let top: Vec<VertexId> = mesh
        .vertices()
        .filter(|(_, p)| p.z > 99.0)
        .map(|(v, _)| v)
        .collect();
    let places: Vec<Vec<VertexId>> = top.iter().map(|v| vec![*v]).collect();
    record(
        &mut doc,
        id,
        Operation::Weld {
            points: top.clone(),
            target: None,
            anchors: mesh::Anchors::record(&mesh, ElementKind::Points, &places),
        },
    );
    assert_eq!(doc.undo_label(), Some("Weld 4 points"));
    let shape = doc.scene().evaluated(id).unwrap();
    assert!(mesh::MeshReport::new(&shape.mesh).is_closed());
    assert!(shape.warnings.is_empty(), "{:?}", shape.warnings);
    assert!(
        Operation::Weld {
            points: top,
            target: None,
            anchors: None
        }
        .settings()
        .is_none()
    );
    // A bridge's rows and twist are its Last edit.
    let bridge = Operation::Bridge {
        edges: Vec::new(),
        faces: vec![FaceId::from_raw(0), FaceId::from_raw(1)],
        segments: 2,
        twist: 0,
        anchors: None,
    };
    assert_eq!(bridge.describe(), "Bridge 2 polygons");
    let turned = bridge.with(EditSettings {
        bridge: Some((4, 1)),
        ..bridge.settings().unwrap()
    });
    assert!(matches!(
        turned,
        Operation::Bridge {
            segments: 4,
            twist: 1,
            ..
        }
    ));
}

#[test]
fn clean_up_edits_are_recorded() {
    let (mut doc, id) = with_a_cube();
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let top = mesh
        .faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).z > 0.9)
        .unwrap();
    let corners = vec![mesh.face(top).unwrap().to_vec()];
    record(
        &mut doc,
        id,
        Operation::DeleteElements {
            points: Vec::new(),
            edges: Vec::new(),
            faces: vec![top],
            anchors: mesh::Anchors::record(&mesh, ElementKind::Polygons, &corners),
        },
    );
    assert_eq!(doc.undo_label(), Some("Delete 1 polygon"));
    let report = |doc: &Document| mesh::MeshReport::new(&doc.scene().evaluated(id).unwrap().mesh);
    assert_eq!(report(&doc).border_edges, 4);
    record(
        &mut doc,
        id,
        Operation::FillHoles {
            points: Vec::new(),
            anchors: None,
        },
    );
    assert_eq!(doc.undo_label(), Some("Fill every hole"));
    assert!(report(&doc).is_closed());
    record(
        &mut doc,
        id,
        Operation::FlipPolygons {
            faces: vec![FaceId::from_raw(0)],
            anchors: None,
        },
    );
    assert_eq!(report(&doc).flipped_edges, 4);
    record(&mut doc, id, Operation::FixFacing);
    assert_eq!(report(&doc).flipped_edges, 0);
    assert_eq!(doc.undo_label(), Some("Fix normals"));
    // More segments afterwards: the deleted polygon is found again.
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: Primitive::Cube(CubeParams {
            segments: [1, 1, 2],
            ..Default::default()
        }),
    })
    .unwrap();
    let shape = doc.scene().evaluated(id).unwrap();
    assert!(mesh::MeshReport::new(&shape.mesh).is_closed());
    assert!(Operation::FixFacing.settings().is_none());
}

#[test]
fn edits_holding_numbers_that_arent_real_are_refused() {
    // A regression test from the whole-app review (L2): such an edit
    // could be saved but not opened again.
    let (mut doc, id) = with_a_cube();
    let steps = doc.undo_steps();
    for operation in [
        lift(&[0], f32::NAN),
        Operation::TransformPoints {
            points: vec![VertexId::from_raw(0)],
            matrix: glam::Mat4::from_scale(Vec3::INFINITY),
            anchors: None,
        },
        Operation::BevelPoints {
            points: vec![VertexId::from_raw(0)],
            width: f32::NAN,
            anchors: None,
            stretch: glam::Vec3::ONE,
        },
    ] {
        assert!(!operation.numbers_are_real());
        assert_eq!(
            doc.execute(Command::Record { id, operation }),
            Err(SceneError::NotANumber)
        );
    }
    assert_eq!(doc.undo_steps(), steps);
    record(&mut doc, id, lift(&[0], 5.0));
    assert_eq!(
        doc.execute(Command::ReplaceOperation {
            id,
            layer: 0,
            index: 0,
            operation: lift(&[0], f32::INFINITY),
        }),
        Err(SceneError::NotANumber)
    );
}

// ---------------------------------------------------------- checkpoints ---

fn one_layer(operations: Vec<Operation>) -> Stack {
    let mut layer = Layer::new("Edits");
    layer.entries = operations
        .into_iter()
        .enumerate()
        .map(|(i, op)| entry(i as u64 + 1, op))
        .collect();
    Stack {
        layers: vec![layer],
    }
}

fn applied(cache: &StackCache, id: ObjectId) -> usize {
    match cache.last_rebuilt(id) {
        Some(Rebuilt::From { operations, .. }) => operations,
        Some(Rebuilt::Nothing) => 0,
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn changing_the_last_edit_replays_only_that_edit() {
    let mut cache = StackCache::default();
    let id = ObjectId(1);
    let base = ObjectKind::Primitive(cube());
    let context = EvalContext::default();
    let mut ops: Vec<Operation> = (0..20).map(|i| lift(&[i % 8], 1.0)).collect();
    let _ = cache.evaluate(id, &base, &one_layer(ops.clone()), &context);
    // A drag: the last edit changes every frame.
    for frame in 1..5 {
        *ops.last_mut().unwrap() = lift(&[3], 1.0 + frame as f32);
        let now = cache
            .evaluate(id, &base, &one_layer(ops.clone()), &context)
            .unwrap();
        assert_eq!(applied(&cache, id), 1, "frame {frame}");
        let fresh = StackCache::default()
            .evaluate(id, &base, &one_layer(ops.clone()), &context)
            .unwrap();
        assert_eq!(now, fresh, "the same shape as working it all out");
    }
}

#[test]
fn a_new_edit_after_opening_and_then_its_drag_apply_one_each() {
    let mut cache = StackCache::default();
    let id = ObjectId(1);
    let base = ObjectKind::Primitive(cube());
    let context = EvalContext::default();
    let mut ops: Vec<Operation> = (0..10).map(|i| lift(&[i % 8], 1.0)).collect();
    let _ = cache.evaluate(id, &base, &one_layer(ops.clone()), &context);
    ops.push(lift(&[0], 1.0));
    let _ = cache.evaluate(id, &base, &one_layer(ops.clone()), &context);
    assert_eq!(applied(&cache, id), 1, "the new edit");
    *ops.last_mut().unwrap() = lift(&[0], 7.0);
    let _ = cache.evaluate(id, &base, &one_layer(ops.clone()), &context);
    assert_eq!(applied(&cache, id), 1, "dragging it");
}

#[test]
fn changing_an_earlier_edit_replays_from_the_start() {
    let mut cache = StackCache::default();
    let id = ObjectId(1);
    let base = ObjectKind::Primitive(cube());
    let context = EvalContext::default();
    let mut ops: Vec<Operation> = (0..6).map(|i| lift(&[i], 1.0)).collect();
    let _ = cache.evaluate(id, &base, &one_layer(ops.clone()), &context);
    ops[2] = lift(&[2], 9.0);
    let now = cache
        .evaluate(id, &base, &one_layer(ops.clone()), &context)
        .unwrap();
    assert_eq!(applied(&cache, id), 6);
    let fresh = StackCache::default()
        .evaluate(id, &base, &one_layer(ops), &context)
        .unwrap();
    assert_eq!(now, fresh);
}

#[test]
fn undo_uses_the_checkpoint_and_redo_applies_one() {
    let mut cache = StackCache::default();
    let id = ObjectId(1);
    let base = ObjectKind::Primitive(cube());
    let context = EvalContext::default();
    let ops: Vec<Operation> = (0..6).map(|i| lift(&[i], 1.0)).collect();
    let _ = cache.evaluate(id, &base, &one_layer(ops.clone()), &context);
    let undone = cache
        .evaluate(id, &base, &one_layer(ops[..5].to_vec()), &context)
        .unwrap();
    assert_eq!(applied(&cache, id), 0, "the shape before the last edit");
    let fresh = StackCache::default()
        .evaluate(id, &base, &one_layer(ops[..5].to_vec()), &context)
        .unwrap();
    assert_eq!(undone, fresh);
    let _ = cache.evaluate(id, &base, &one_layer(ops.clone()), &context);
    assert_eq!(applied(&cache, id), 1);
    // Undoing twice in a row has no checkpoint for the second: all again.
    let _ = cache.evaluate(id, &base, &one_layer(ops[..5].to_vec()), &context);
    let _ = cache.evaluate(id, &base, &one_layer(ops[..4].to_vec()), &context);
    assert_eq!(applied(&cache, id), 4);
}

#[test]
fn checkpoints_keep_warnings_right() {
    let mut cache = StackCache::default();
    let id = ObjectId(1);
    let base = ObjectKind::Primitive(cube());
    let context = EvalContext::default();
    // Point 999 doesn't exist: a warning for the first and the last edit.
    let mut ops = vec![lift(&[999], 1.0), lift(&[0], 1.0), lift(&[999], 1.0)];
    let first = cache
        .evaluate(id, &base, &one_layer(ops.clone()), &context)
        .unwrap();
    assert_eq!(first.warnings.len(), 2);
    ops[2] = lift(&[1], 1.0);
    let second = cache
        .evaluate(id, &base, &one_layer(ops.clone()), &context)
        .unwrap();
    assert_eq!(applied(&cache, id), 1);
    assert_eq!(second.warnings.len(), 1, "the last edit's warning is gone");
    assert_eq!(second.warnings[0].operation, 0);
}

#[test]
fn switching_a_layer_off_and_on_gives_the_same_shape() {
    let mut cache = StackCache::default();
    let id = ObjectId(1);
    let base = ObjectKind::Primitive(cube());
    let context = EvalContext::default();
    let mut stack = one_layer((0..4).map(|i| lift(&[i], 1.0)).collect());
    let on = cache.evaluate(id, &base, &stack, &context).unwrap();
    stack.layers[0].enabled = false;
    let off = cache.evaluate(id, &base, &stack, &context).unwrap();
    assert_eq!(*off.mesh, cube().mesh());
    stack.layers[0].enabled = true;
    let again = cache.evaluate(id, &base, &stack, &context).unwrap();
    assert_eq!(on, again);
}

// ------------------------------------- edit ids and switches (0044) ---

fn entries(doc: &Document, id: ObjectId) -> Vec<Entry> {
    doc.scene().object(id).unwrap().stack.layers()[0]
        .entries
        .clone()
}

#[test]
fn every_edit_gets_an_id_that_stays_with_it() {
    let (mut doc, id) = with_a_cube();
    record(&mut doc, id, lift(&[0], 5.0));
    record(&mut doc, id, lift(&[1], 5.0));
    let ids: Vec<u64> = entries(&doc, id).iter().map(|e| e.id.get()).collect();
    assert_eq!(ids, [1, 2]);
    assert!(entries(&doc, id).iter().all(|e| e.enabled && !e.pinned));
    // Undo and redo bring the same edit back, id and all.
    doc.undo().unwrap();
    doc.redo().unwrap();
    assert_eq!(entries(&doc, id)[1].id, EditId::from_raw(2));
    // Changing an edit's settings keeps its id and switches.
    doc.execute(Command::SetEditPinned {
        id,
        edit: EditId::from_raw(1),
        pinned: true,
    })
    .unwrap();
    doc.execute(Command::ReplaceOperation {
        id,
        layer: 0,
        index: 0,
        operation: lift(&[0], 9.0),
    })
    .unwrap();
    let first = &entries(&doc, id)[0];
    assert_eq!((first.id, first.pinned), (EditId::from_raw(1), true));
    assert_eq!(first.operation, lift(&[0], 9.0));
    assert_eq!(
        doc.scene()
            .object(id)
            .unwrap()
            .stack
            .entry(EditId::from_raw(2))
            .map(|(l, i, _)| (l, i)),
        Some((0, 1))
    );
}

#[test]
fn a_switched_off_edit_is_skipped_and_can_come_back() {
    let (mut doc, id) = with_a_cube();
    let start = point(&doc, id, 0);
    record(&mut doc, id, lift(&[0], 5.0));
    record(&mut doc, id, lift(&[0], 1.0));
    let off = |enabled| Command::SetEditEnabled {
        id,
        edit: EditId::from_raw(1),
        enabled,
    };
    doc.execute(off(false)).unwrap();
    assert_eq!(point(&doc, id, 0), start + Vec3::Z, "only the second edit");
    assert_eq!(doc.undo_label(), Some("Switch edit off"));
    let steps = doc.undo_steps();
    doc.execute(off(false)).unwrap();
    assert_eq!(doc.undo_steps(), steps, "already off: no step");
    doc.undo().unwrap();
    assert_eq!(point(&doc, id, 0), start + Vec3::Z * 6.0);
    doc.redo().unwrap();
    doc.execute(off(true)).unwrap();
    assert_eq!(point(&doc, id, 0), start + Vec3::Z * 6.0);
    // Unknown edits and objects are refused.
    assert!(
        doc.execute(Command::SetEditEnabled {
            id,
            edit: EditId::from_raw(99),
            enabled: false,
        })
        .is_err()
    );
    assert_eq!(
        doc.execute(Command::SetEditPinned {
            id: ObjectId(999),
            edit: EditId::from_raw(1),
            pinned: true,
        }),
        Err(SceneError::UnknownObject(ObjectId(999)))
    );
    // Make editable leaves switched-off edits out, as they are shown.
    doc.execute(off(false)).unwrap();
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    assert_eq!(point(&doc, id, 0), start + Vec3::Z);
}

#[test]
fn a_saved_scene_with_two_edits_sharing_an_id_is_refused() {
    let mut object = Object::new(ObjectKind::Primitive(cube()));
    let mut layer = Layer::new(EDITS_LAYER);
    layer.entries = vec![entry(3, lift(&[0], 1.0)), entry(3, lift(&[1], 1.0))];
    object.stack = Stack::from_layers(vec![layer]);
    assert!(Scene::from_objects([(ObjectId(1), None, object)], 2).is_err());
}

/// Records a move of `points` (by id) of `id`'s current shape by `offset`,
/// with anchors, as the gizmo does.
fn move_with_anchors(doc: &mut Document, id: ObjectId, points: Vec<VertexId>, offset: Vec3) {
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let elements: Vec<Vec<VertexId>> = points.iter().map(|v| vec![*v]).collect();
    let anchors = mesh::Anchors::record(&shape, mesh::ElementKind::Points, &elements);
    doc.execute(Command::Record {
        id,
        operation: Operation::MovePoints {
            points,
            offset,
            anchors,
        },
    })
    .unwrap();
}

#[test]
fn changing_an_earlier_edit_keeps_later_edits_on_their_points() {
    // Extrude the top twice, move the middle ring out, then make the first
    // extrude longer: the ring moved, but it is the same ring, so the move
    // still applies to it (review A2).
    let (mut doc, id) = with_a_cube();
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let top = shape
        .faces()
        .find(|(f, _)| shape.face_normal(*f).z > 0.9)
        .unwrap()
        .0;
    let extrude = |distance| Operation::ExtrudePolygons {
        faces: vec![top],
        distance,
        grouping: Grouping::Together,
        shape: crate::ExtrudeShape::EARLIER,
        anchors: None,
        stretch: glam::Vec3::ONE,
    };
    record(&mut doc, id, extrude(50.0));
    record(&mut doc, id, extrude(50.0));
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let ring: Vec<VertexId> = shape
        .vertices()
        .filter(|(_, p)| (p.z - 150.0).abs() < 1e-3)
        .map(|(v, _)| v)
        .collect();
    assert_eq!(ring.len(), 4);
    move_with_anchors(&mut doc, id, ring.clone(), Vec3::X * 20.0);
    doc.execute(Command::ReplaceOperation {
        id,
        layer: 0,
        index: 0,
        operation: extrude(80.0),
    })
    .unwrap();
    let shape = doc.scene().evaluated(id).unwrap();
    for v in &ring {
        let p = shape.mesh.position(*v).unwrap();
        assert!((p.z - 180.0).abs() < 1e-3, "the ring rose with the extrude");
        assert!(
            (p.x - 70.0).abs() < 1e-3 || (p.x + 30.0).abs() < 1e-3,
            "and is still moved 20 along X: {p}"
        );
    }
    assert!(shape.warnings.is_empty(), "nothing had to be searched for");
}

// ------------------------------------------------ limits (decision 0045) ---

#[test]
fn an_edit_that_would_make_the_shape_too_large_is_skipped_with_a_warning() {
    let mut doc = Document::new();
    let dense = Primitive::Cube(CubeParams {
        segments: [256; 3],
        ..CubeParams::default()
    });
    doc.execute(Command::Create(ObjectKind::Primitive(dense)))
        .unwrap();
    let id = doc.selection()[0];
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let edges: Vec<mesh::EdgeKey> = shape.edges().to_vec();
    // Every edge, 64 rows: about 50 million polygons. Skipped.
    record(
        &mut doc,
        id,
        Operation::BevelEdges {
            edges,
            shape: BevelShape {
                width: 0.1,
                segments: 64,
                profile: 1.0,
                options: mesh::edit::BevelOptions::EARLIER,
            },
            anchors: None,
            stretch: glam::Vec3::ONE,
        },
    );
    let after = doc.scene().evaluated(id).unwrap();
    assert_eq!(
        after.mesh.face_count(),
        shape.face_count(),
        "nothing changed"
    );
    assert_eq!(
        after.warnings,
        [StackWarning {
            layer: 0,
            operation: 0,
            missing: 0,
            found_again: false,
            too_big: true,
        }]
    );
    // A sensible edit after it still applies.
    record(&mut doc, id, lift(&[0], 5.0));
    assert_eq!(doc.scene().evaluated(id).unwrap().warnings.len(), 1);
}

#[test]
fn hard_and_soft_edges_are_recorded() {
    let (mut doc, id) = with_a_cube();
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    assert_eq!(mesh.hard_edge_count(), 12);
    let edges: Vec<EdgeKey> = mesh.edges()[..4].to_vec();
    let ends: Vec<Vec<VertexId>> = edges
        .iter()
        .map(|e| {
            let (a, b) = e.ends();
            vec![a, b]
        })
        .collect();
    record(
        &mut doc,
        id,
        Operation::SetHardEdges {
            edges,
            hard: false,
            anchors: mesh::Anchors::record(&mesh, ElementKind::Edges, &ends),
        },
    );
    assert_eq!(doc.undo_label(), Some("Soften 4 edges"));
    let hard = |doc: &Document| doc.scene().evaluated(id).unwrap().mesh.hard_edge_count();
    assert_eq!(hard(&doc), 8);
    // By angle: every corner of a cube is sharper than 30°.
    let by_angle = Operation::HardEdgesByAngle { angle: 30.0 };
    assert_eq!(by_angle.settings().unwrap().angle, Some(30.0));
    record(&mut doc, id, by_angle.clone());
    assert_eq!(hard(&doc), 12);
    assert_eq!(doc.undo_label(), Some("Hard edges by angle"));
    // A wider angle than a cube's corners softens them all; out of range
    // is brought within 0–180°.
    let wide = by_angle.with(EditSettings {
        angle: Some(500.0),
        ..EditSettings::default()
    });
    assert_eq!(wide.settings().unwrap().angle, Some(180.0));
    assert!(!Operation::HardEdgesByAngle { angle: f32::NAN }.numbers_are_real());
    // A taller cube afterwards: the edits still apply.
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: Primitive::Cube(CubeParams {
            size: glam::Vec3::new(100.0, 100.0, 300.0),
            ..Default::default()
        }),
    })
    .unwrap();
    let shape = doc.scene().evaluated(id).unwrap();
    assert!(shape.warnings.is_empty());
    assert_eq!(shape.mesh.hard_edge_count(), 12);
}

#[test]
fn an_inset_on_a_stretched_object_is_even_as_seen() {
    // The cube stretched to twice its width along X (decision 0082): an
    // inset of 10 leaves a border 10 wide on every side as seen, not 20
    // along X and 10 along Y.
    let (mut doc, id) = with_a_cube();
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let top = mesh
        .faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).z > 0.9)
        .unwrap();
    let stretch = Vec3::new(2.0, 1.0, 1.0) / (4.0 / 3.0);
    record(
        &mut doc,
        id,
        Operation::InsetPolygons {
            faces: vec![top],
            distance: 10.0,
            grouping: Grouping::Together,
            stretch,
            anchors: None,
        },
    );
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let across = |m: &Mesh, f: FaceId| {
        let seen: Vec<Vec3> = m
            .face(f)
            .unwrap()
            .iter()
            .map(|v| m.position(*v).unwrap() * stretch)
            .collect();
        let size = |pick: fn(Vec3) -> f32| {
            let most = seen.iter().map(|p| pick(*p)).fold(f32::MIN, f32::max);
            let least = seen.iter().map(|p| pick(*p)).fold(f32::MAX, f32::min);
            most - least
        };
        (size(|p| p.x), size(|p| p.y))
    };
    let (outer_x, outer_y) = across(&mesh, top);
    let (inner_x, inner_y) = across(&shape, top);
    assert!(
        (outer_x - inner_x - 20.0).abs() < 1e-2,
        "{outer_x} {inner_x}"
    );
    assert!(
        (outer_y - inner_y - 20.0).abs() < 1e-2,
        "{outer_y} {inner_y}"
    );
}

#[test]
fn a_bevelled_corner_on_a_stretched_object_is_even_as_seen() {
    let (mut doc, id) = with_a_cube();
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let (corner, at) = mesh
        .vertices()
        .max_by(|a, b| (a.1.x + a.1.y + a.1.z).total_cmp(&(b.1.x + b.1.y + b.1.z)))
        .unwrap();
    let stretch = Vec3::new(0.5, 1.0, 1.5);
    record(
        &mut doc,
        id,
        Operation::BevelPoints {
            points: vec![corner],
            width: 8.0,
            stretch,
            anchors: None,
        },
    );
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    // The new points sit 8 from the corner as seen, one along each edge.
    let seen_corner = at * stretch;
    let near: Vec<f32> = shape
        .vertices()
        .map(|(_, p)| (p * stretch).distance(seen_corner))
        .filter(|d| *d < 30.0 && *d > 1e-3)
        .collect();
    assert_eq!(near.len(), 3, "{near:?}");
    for d in near {
        assert!((d - 8.0).abs() < 1e-2, "{d}");
    }
}

#[test]
fn every_measuring_edit_keeps_a_stretch_and_others_dont() {
    let s = Vec3::new(0.5, 1.0, 1.5);
    let face = FaceId::from_raw(0);
    let inset = Operation::InsetPolygons {
        faces: vec![face],
        distance: 1.0,
        grouping: Grouping::Together,
        stretch: Vec3::ONE,
        anchors: None,
    };
    assert!(inset.keeps_stretch());
    let stretched = inset.with_stretch(s);
    assert_eq!(stretched.stretch(), s);
    assert_eq!(stretched.settings().unwrap().stretch, Some(s));
    // Changing a setting keeps it, unless a new one comes with it.
    let wider = stretched.with(EditSettings {
        distance: Some(3.0),
        stretch: None,
        ..EditSettings::default()
    });
    assert_eq!(wider.stretch(), s);
    let fresh = stretched.with(EditSettings {
        stretch: Some(Vec3::ONE),
        ..EditSettings::default()
    });
    assert_eq!(fresh.stretch(), Vec3::ONE);
    // A broken stretch counts as none; a mirrored edit keeps its edit's.
    assert_eq!(
        inset.with_stretch(Vec3::new(f32::NAN, 1.0, 1.0)).stretch(),
        Vec3::ONE
    );
    let mirrored = Operation::Mirrored {
        op: Box::new(stretched.clone()),
        mirror: mesh::symmetry::Mirror {
            axes: [true, false, false],
            centre: Vec3::ZERO,
            clip: false,
        },
    };
    assert_eq!(mirrored.stretch(), s);
    assert_eq!(mirrored.with_stretch(Vec3::ONE).stretch(), Vec3::ONE);
    // A straight extrude bent and straightened again keeps it.
    let straight = Operation::ExtrudePolygons {
        faces: vec![face],
        distance: 5.0,
        grouping: Grouping::Together,
        shape: crate::ExtrudeShape::default(),
        stretch: s,
        anchors: None,
    };
    let bent = straight.with(EditSettings {
        bend: Some(Bend {
            angle: 30.0,
            ..Bend::default()
        }),
        ..straight.settings().unwrap()
    });
    assert!(matches!(bent, Operation::ExtrudeBent { stretch, .. } if stretch == s));
    let back = bent.with(EditSettings {
        bend: Some(Bend::default()),
        ..bent.settings().unwrap()
    });
    assert!(matches!(back, Operation::ExtrudePolygons { stretch, .. } if stretch == s));
    // Moves, cuts and the rest don't keep one.
    for other in [lift(&[0], 1.0), Operation::FixFacing] {
        assert!(!other.keeps_stretch());
        assert_eq!(other.with_stretch(s), other);
        assert_eq!(other.stretch(), Vec3::ONE);
    }
    // A stretch that isn't a real number makes the edit refused.
    let mut broken = stretched;
    if let Operation::InsetPolygons { stretch, .. } = &mut broken {
        *stretch = Vec3::new(f32::INFINITY, 1.0, 1.0);
    }
    assert!(!broken.numbers_are_real());
}

#[test]
fn a_groove_is_recorded_named_and_changed_afterwards() {
    let (mut doc, id) = with_a_cube();
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let top: Vec<mesh::EdgeKey> = mesh
        .edges()
        .iter()
        .filter(|e| {
            let (a, b) = e.ends();
            mesh.position(a).unwrap().z > 99.0 && mesh.position(b).unwrap().z > 99.0
        })
        .copied()
        .collect();
    let groove = Operation::Groove {
        edges: top,
        shape: crate::GrooveShape {
            width: 4.0,
            depth: -2.0,
            sides: crate::GrooveSides::Straight,
        },
        stretch: Vec3::ONE,
        anchors: None,
    };
    assert_eq!(groove.describe(), "Groove along 4 edges");
    assert!(groove.keeps_stretch());
    record(&mut doc, id, groove.clone());
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let r = mesh::MeshReport::new(&shape);
    assert!(r.is_closed() && !r.has_problems(), "{r:?}");
    assert!(shape.face_count() > mesh.face_count());
    // Its settings: the depth as the distance, and the whole shape.
    let settings = groove.settings().unwrap();
    assert_eq!(settings.distance, Some(-2.0));
    let ridge = groove.with(EditSettings {
        distance: Some(3.0),
        ..settings
    });
    assert_eq!(ridge.describe(), "Ridge along 4 edges");
    let sloped = ridge.with(EditSettings {
        groove: Some(crate::GrooveShape {
            sides: crate::GrooveSides::Sloped,
            ..crate::GrooveShape::default()
        }),
        distance: None,
        ..EditSettings::default()
    });
    let Operation::Groove { shape: s, .. } = &sloped else {
        panic!("a groove");
    };
    assert_eq!(s.sides, crate::GrooveSides::Sloped);
    let mut broken = sloped.clone();
    if let Operation::Groove { shape, .. } = &mut broken {
        shape.width = f32::NAN;
    }
    assert!(!broken.numbers_are_real());
}

// ------------------------------------------------- stable element names ---

#[test]
fn points_on_the_same_spot_are_told_apart_by_their_names() {
    // A cube; a bevel along a bottom edge; the top pushed out by nothing
    // (a cap right on top of the old top); then the cap's points lifted.
    let (mut doc, id) = with_a_cube();
    let shape = |doc: &Document| doc.scene().evaluated(id).unwrap().mesh;
    let mesh = shape(&doc);
    let bottom: Vec<VertexId> = mesh
        .edges()
        .iter()
        .find(|e| {
            let (a, b) = e.ends();
            let (pa, pb) = (mesh.position(a).unwrap(), mesh.position(b).unwrap());
            pa.z < 1.0 && pb.z < 1.0 && pa.y < -49.0 && pb.y < -49.0
        })
        .map(|e| vec![e.ends().0, e.ends().1])
        .unwrap();
    let bevel = |segments| Operation::BevelEdges {
        edges: vec![EdgeKey::new(bottom[0], bottom[1])],
        shape: BevelShape {
            width: 10.0,
            segments,
            profile: 0.0,
            options: mesh::edit::BevelOptions::EARLIER,
        },
        anchors: mesh::Anchors::record(&mesh, ElementKind::Edges, std::slice::from_ref(&bottom)),
        stretch: Vec3::ONE,
    };
    record(&mut doc, id, bevel(1));
    let mesh = shape(&doc);
    let top = mesh
        .faces()
        .find(|(f, _)| mesh.face_normal(*f).z > 0.9)
        .map(|(f, c)| (f, c.to_vec()))
        .unwrap();
    record(
        &mut doc,
        id,
        Operation::ExtrudePolygons {
            faces: vec![top.0],
            distance: 0.0,
            grouping: Grouping::Together,
            shape: crate::ExtrudeShape::EARLIER,
            anchors: mesh::Anchors::record(
                &mesh,
                ElementKind::Polygons,
                std::slice::from_ref(&top.1),
            ),
            stretch: Vec3::ONE,
        },
    );
    // The cap: the top polygon, now on the extrude's new points.
    let mesh = shape(&doc);
    let cap: Vec<VertexId> = mesh.face(top.0).unwrap().to_vec();
    assert!(cap.iter().all(|v| !top.1.contains(v)), "new points");
    assert!(cap.iter().all(|v| mesh.trusted_point_name(*v).is_some()));
    let places: Vec<Vec<VertexId>> = cap.iter().map(|v| vec![*v]).collect();
    record(
        &mut doc,
        id,
        Operation::MovePoints {
            points: cap.clone(),
            offset: Vec3::Z * 50.0,
            anchors: mesh::Anchors::record(&mesh, ElementKind::Points, &places),
        },
    );
    let lifted_right = |doc: &Document| {
        let mesh = shape(doc);
        // The top polygon is lifted, and the old top's points stay put:
        // still four points at the old height.
        let lifted = mesh
            .face(top.0)
            .unwrap()
            .iter()
            .all(|v| (mesh.position(*v).unwrap().z - 150.0).abs() < 1e-3);
        let stayed = mesh
            .vertices()
            .filter(|(_, p)| (p.z - 100.0).abs() < 1e-3)
            .count();
        lifted && stayed == 4
    };
    assert!(lifted_right(&doc));
    // More bevel rows renumber every point made after the bevel: the
    // lift finds its points again by place, where two points sit on each
    // spot, and its names tell which.
    doc.execute(Command::ReplaceOperation {
        id,
        layer: 0,
        index: 0,
        operation: bevel(3),
    })
    .unwrap();
    assert!(lifted_right(&doc), "the cap is lifted, not the old top");
}
