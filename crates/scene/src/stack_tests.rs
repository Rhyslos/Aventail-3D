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
    assert_eq!(doc.scene().last_rebuilt(id), Some(Rebuilt::From(0)));
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
    lower.operations.push(lift(&[0], 10.0));
    let mut upper = Layer::new("Upper");
    upper.operations.push(lift(&[0], 1.0));
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
    stack.layers[1].operations.push(lift(&[0], 100.0));
    let second = cache.evaluate(id, &base, &stack, &context).unwrap();
    assert_eq!(
        cache.last_rebuilt(id),
        Some(Rebuilt::From(1)),
        "only the top"
    );
    assert_eq!(
        second.mesh.position(VertexId::from_raw(0)),
        Some(start + Vec3::Z * 111.0)
    );
    // Changing the lower layer evaluates both again.
    stack.layers[0].operations.clear();
    let third = cache.evaluate(id, &base, &stack, &context).unwrap();
    assert_eq!(cache.last_rebuilt(id), Some(Rebuilt::From(0)));
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
    assert_eq!(command.describe(doc.scene()), "Make object editable");
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
    assert_eq!(doc.undo_label(), Some("Make object editable"));
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
    assert_eq!(command.describe(doc.scene()), "Make 2 objects editable");
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
        anchors: anchored
            .then(|| mesh::Anchors::record(&mesh, ElementKind::Polygons, &corners))
            .flatten(),
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
        doc.scene().object(id).unwrap().stack.layers()[0].operations,
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
        },
        anchors: mesh::Anchors::record(&mesh, ElementKind::Edges, &ends),
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
    assert_eq!(settings.distance, 10.0);
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
        },
    );
    assert_eq!(doc.undo_label(), Some("Bevel 1 point"));
    let shape = doc.scene().evaluated(id).unwrap();
    assert_eq!(shape.mesh.face_count(), 7);
    assert!(mesh::MeshReport::new(&shape.mesh).is_closed());
    // Moves and turns have no settings to change afterwards.
    assert!(lift(&[0], 1.0).settings().is_none());
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
