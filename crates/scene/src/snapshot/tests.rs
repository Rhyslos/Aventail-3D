//! Layout snapshots (decision 0114): taking them is one step to undo,
//! putting one back is one recorded edit per object, and modelling done
//! since is kept.

use super::*;
use crate::{Command, Document, FaceId, Grouping, PrimitiveKind};
use glam::{Affine2, Vec2, Vec3};

fn with_a_cube() -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    (doc, id)
}

fn layout_of(doc: &Document, id: ObjectId) -> UvLayout {
    UvLayout::of(&doc.scene().evaluated(id).unwrap().mesh).unwrap()
}

/// Moves every UV of `id`, as a hand edit.
fn move_uvs(doc: &mut Document, id: ObjectId) {
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let faces = mesh.faces().map(|(f, _)| f).collect();
    doc.execute(Command::Record {
        id,
        operation: Operation::TransformUvs {
            faces,
            points: Vec::new(),
            transform: Affine2::from_scale_angle_translation(Vec2::splat(0.5), 0.3, Vec2::X),
            anchors: None,
        },
    })
    .unwrap();
}

fn take(doc: &mut Document, ids: &[ObjectId], name: &str) {
    let snap = UvSnapshot::take(doc.scene(), ids, name).unwrap();
    let mut list = doc.scene().snapshots().to_vec();
    list.push(snap);
    doc.execute(Command::SetSnapshots(list)).unwrap();
}

fn restore(doc: &mut Document, index: usize) {
    let ops = doc.scene().snapshots()[index].restore(doc.scene());
    for (id, operation) in ops {
        doc.execute_labelled(Command::Record { id, operation }, "Restore", Some(77))
            .unwrap();
    }
}

#[test]
fn a_snapshot_puts_the_layout_back_as_one_step() {
    let (mut doc, id) = with_a_cube();
    let saved = layout_of(&doc, id);
    take(&mut doc, &[id], "First");
    assert_eq!(doc.scene().snapshots().len(), 1);
    assert_eq!(doc.scene().snapshots()[0].layout(id), Some(&saved));
    move_uvs(&mut doc, id);
    let moved = layout_of(&doc, id);
    assert_ne!(moved, saved);
    let steps = doc.undo_steps();
    restore(&mut doc, 0);
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(layout_of(&doc, id), saved);
    doc.undo();
    assert_eq!(layout_of(&doc, id), moved);
    doc.redo();
    assert_eq!(layout_of(&doc, id), saved);
    // The snapshot is still there, unchanged.
    assert_eq!(doc.scene().snapshots()[0].layout(id), Some(&saved));
}

#[test]
fn taking_and_removing_snapshots_are_steps_to_undo() {
    let (mut doc, id) = with_a_cube();
    take(&mut doc, &[id], "A");
    take(&mut doc, &[id], "B");
    assert_eq!(doc.scene().snapshots().len(), 2);
    doc.undo();
    assert_eq!(doc.scene().snapshots().len(), 1);
    doc.redo();
    let mut list = doc.scene().snapshots().to_vec();
    list.remove(0);
    doc.execute(Command::SetSnapshots(list)).unwrap();
    assert_eq!(doc.scene().snapshots()[0].name, "B");
    doc.undo();
    assert_eq!(doc.scene().snapshots()[0].name, "A");
    let steps = doc.undo_steps();
    doc.execute(Command::SetSnapshots(doc.scene().snapshots().to_vec()))
        .unwrap();
    assert_eq!(doc.undo_steps(), steps, "the same again records nothing");
}

#[test]
fn bad_lists_are_refused() {
    let (mut doc, id) = with_a_cube();
    let good = UvSnapshot::take(doc.scene(), &[id], "Good").unwrap();
    let named = |name: &str| UvSnapshot {
        name: name.into(),
        ..good.clone()
    };
    let mut unreal = good.clone();
    let mut layout = (*unreal.objects[0].1).clone();
    layout.faces[0].1[0] = Vec2::new(f32::INFINITY, 0.0);
    unreal.objects[0].1 = Arc::new(layout);
    let too_many: Vec<UvSnapshot> = (0..=MAX_SNAPSHOTS)
        .map(|i| named(&format!("S{i}")))
        .collect();
    for bad in [
        vec![named("")],
        vec![named("  ")],
        vec![named("A\nB")],
        vec![named(&"x".repeat(MAX_SNAPSHOT_NAME + 1))],
        vec![named("Same"), named("Same")],
        vec![unreal],
        too_many,
    ] {
        assert!(!snapshots_are_valid(&bad));
        assert!(doc.execute(Command::SetSnapshots(bad)).is_err());
    }
    assert!(snapshots_are_valid(&[good]));
    assert!(snapshots_are_valid(&[]));
}

#[test]
fn free_names_never_clash() {
    let (doc, id) = with_a_cube();
    let snap = |name: &str| UvSnapshot {
        name: name.into(),
        ..UvSnapshot::take(doc.scene(), &[id], "x").unwrap()
    };
    assert_eq!(free_name(&[], "Snapshot 1"), "Snapshot 1");
    assert_eq!(free_name(&[], "   "), "Snapshot");
    let list = vec![snap("Packed"), snap("Packed 2")];
    assert_eq!(free_name(&list, "Packed"), "Packed 3");
    assert_eq!(free_name(&list, " Unwrapped "), "Unwrapped");
    let long = "y".repeat(200);
    let name = free_name(&[], &long);
    assert!(name_is_valid(&name));
    let list = vec![snap(&name)];
    assert!(name_is_valid(&free_name(&list, &long)));
}

#[test]
fn modelling_done_since_is_kept() {
    let (mut doc, id) = with_a_cube();
    take(&mut doc, &[id], "Before");
    let saved = layout_of(&doc, id);
    move_uvs(&mut doc, id);
    // An extrude makes new polygons the snapshot doesn't know.
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let top: Vec<FaceId> = mesh.faces().map(|(f, _)| f).take(1).collect();
    doc.execute(Command::Record {
        id,
        operation: Operation::ExtrudePolygons {
            faces: top,
            distance: 20.0,
            grouping: Grouping::Together,
            shape: crate::ExtrudeShape::EARLIER,
            stretch: Vec3::ONE,
            anchors: None,
        },
    })
    .unwrap();
    let grown = doc.scene().evaluated(id).unwrap().mesh;
    assert!(grown.face_count() > saved.polygon_count());
    restore(&mut doc, 0);
    let after = layout_of(&doc, id);
    assert_eq!(after.polygon_count(), grown.face_count());
    for (f, uvs) in &saved.faces {
        if let Some((_, now)) = after.faces.iter().find(|(g, _)| g == f)
            && now.len() == uvs.len()
        {
            assert_eq!(now, uvs);
        }
    }
}

#[test]
fn deleted_objects_and_instances_are_left_out() {
    let (mut doc, id) = with_a_cube();
    doc.execute(Command::Duplicate {
        ids: vec![id],
        instances: true,
    })
    .unwrap();
    let instance = doc.selection()[0];
    let snap = UvSnapshot::take(doc.scene(), &[id, instance], "Both").unwrap();
    assert_eq!(snap.objects.len(), 1, "an instance has no UVs of its own");
    assert!(UvSnapshot::take(doc.scene(), &[instance], "Only").is_none());
    assert!(snap.polygon_count() > 0);
    doc.execute(Command::Delete(vec![id])).unwrap();
    assert!(snap.restore(doc.scene()).is_empty());
}

#[test]
fn restoring_is_recorded_and_described() {
    let (doc, id) = with_a_cube();
    let snap = UvSnapshot::take(doc.scene(), &[id], "S").unwrap();
    let ops = snap.restore(doc.scene());
    assert_eq!(ops.len(), 1);
    let (_, op) = &ops[0];
    assert!(op.numbers_are_real());
    assert_eq!(op.describe(), "Restore the UVs of 6 polygons");
    assert!(!crate::mirroring::mirrors(op), "UVs aren't mirrored");
}
