//! Tests for the Weighted normals modifier and custom normals in the
//! scene (decision 0148).

use crate::*;
use mesh::custom_normals::has_custom_normals;
use std::sync::Arc;

/// A 100 cm cube with every edge bevelled 5 cm in two segments, every
/// edge soft, as a plain shape.
fn bevelled_cube() -> mesh::Mesh {
    let mut cube = mesh::Primitive::Cube(Default::default()).mesh();
    let edges: Vec<EdgeKey> = cube.edges().to_vec();
    mesh::bevel::bevel_edges(
        &mut cube,
        &edges,
        BevelShape {
            width: 5.0,
            segments: 2,
            profile: 1.0,
            options: BevelOptions::default(),
        },
    );
    let all: Vec<EdgeKey> = cube.edges().to_vec();
    for e in all {
        let (a, b) = e.ends();
        cube.set_hard(a, b, false);
    }
    cube
}

/// A document holding the bevelled cube, shaded by its marks (none), with
/// a standard Weighted normals modifier.
fn weighted_cube() -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Mesh(Arc::new(bevelled_cube()))))
        .unwrap();
    let id = doc.selection()[0];
    doc.execute(Command::SetHardness {
        ids: vec![id],
        hardness: Hardness::Marked,
    })
    .unwrap();
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind: weighted(),
    })
    .unwrap();
    (doc, id)
}

fn weighted() -> ModifierKind {
    ModifierKind::WeightedNormals(WeightedNormalsOptions::default())
}

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
    .unwrap();
}

#[test]
fn the_shown_and_exported_shapes_carry_the_normals_and_the_edited_one_doesnt() {
    let (doc, id) = weighted_cube();
    let scene = doc.scene();
    assert!(has_custom_normals(&scene.shown_shape(id).unwrap()));
    assert!(has_custom_normals(&scene.exported_shape(id).unwrap()));
    assert!(!has_custom_normals(&scene.evaluated(id).unwrap().mesh));
    assert_eq!(scene.shown_shape(id), scene.exported_shape(id));
    assert!(scene.modifier_problems(id).is_empty());
    assert_eq!(doc.undo_label(), Some("Add Weighted normals"));
    assert_eq!(
        scene.final_weighted_normals(id, Purpose::Export),
        Some(WeightedNormalsOptions::default())
    );
    assert_eq!(scene.normals_dropped_by(id, 0), None);
    // Only the normals differ from the shape before it.
    let mut plain = (*scene.shown_shape(id).unwrap()).clone();
    mesh::custom_normals::forget_custom_normals(&mut plain);
    assert_eq!(plain, *scene.unmodified_shape(id).unwrap());
}

#[test]
fn a_modifier_after_it_drops_them_and_moving_it_last_brings_them_back() {
    let (mut doc, id) = weighted_cube();
    add(
        &mut doc,
        id,
        ModifierKind::Subdivide(SubdivideOptions::default()),
    );
    let scene = doc.scene();
    assert!(!has_custom_normals(&scene.shown_shape(id).unwrap()));
    assert_eq!(scene.final_weighted_normals(id, Purpose::Export), None);
    assert_eq!(scene.normals_dropped_by(id, 0), Some("Subdivide"));
    doc.execute(Command::MoveModifier { id, from: 0, to: 1 })
        .unwrap();
    let scene = doc.scene();
    assert!(has_custom_normals(&scene.shown_shape(id).unwrap()));
    assert_eq!(scene.normals_dropped_by(id, 1), None);
    // A deformer after it drops them too: its points have moved.
    let mut doc2 = weighted_cube().0;
    let id2 = doc2.scene().walk()[0].0;
    add(
        &mut doc2,
        id2,
        ModifierKind::Decimate(DecimateOptions {
            ratio: 1.0,
            ..Default::default()
        }),
    );
    assert!(!has_custom_normals(&doc2.scene().shown_shape(id2).unwrap()));
    // Switched off, it no longer drops them.
    let mut off = doc2.scene().modifiers(id2)[1].clone();
    off.on = false;
    doc2.execute(Command::ChangeModifier {
        id: id2,
        index: 1,
        modifier: off,
    })
    .unwrap();
    assert!(has_custom_normals(&doc2.scene().shown_shape(id2).unwrap()));
    assert_eq!(doc2.scene().normals_dropped_by(id2, 0), None);
}

#[test]
fn hidden_in_the_viewport_it_still_exports() {
    let (mut doc, id) = weighted_cube();
    let mut hidden = doc.scene().modifiers(id)[0].clone();
    hidden.in_viewport = false;
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier: hidden,
    })
    .unwrap();
    let scene = doc.scene();
    assert!(!has_custom_normals(&scene.shown_shape(id).unwrap()));
    assert!(has_custom_normals(&scene.exported_shape(id).unwrap()));
    assert!(
        scene
            .final_weighted_normals(id, Purpose::Viewport)
            .is_none()
    );
    assert!(scene.final_weighted_normals(id, Purpose::Export).is_some());
}

#[test]
fn the_objects_smoothing_decides_the_hard_edges_kept() {
    let (mut doc, id) = weighted_cube();
    let marked = doc.scene().shown_shape(id).unwrap();
    // By angle at 30°: the 45° bevel steps (two segments of a right
    // angle) are hard, so the shading breaks there; worked out again.
    doc.execute(Command::SetHardness {
        ids: vec![id],
        hardness: Hardness::ByAngle(30.0),
    })
    .unwrap();
    let by_angle = doc.scene().shown_shape(id).unwrap();
    assert_ne!(marked, by_angle);
    let input = doc.scene().unmodified_shape(id).unwrap();
    let expected = mesh::weighted_normals::weighted_normals(
        &input,
        &WeightedNormalsOptions::default(),
        Hardness::ByAngle(30.0),
    );
    assert_eq!(*by_angle, expected);
    assert!(
        mesh::hardness::shading_hard_edges(&by_angle, Hardness::Marked).len()
            > mesh::hardness::shading_hard_edges(&marked, Hardness::Marked).len()
    );
}

#[test]
fn bake_into_shape_keeps_them_until_the_shape_is_edited() {
    let (mut doc, id) = weighted_cube();
    let expected = doc.scene().exported_shape(id).unwrap();
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    let object = doc.scene().object(id).unwrap();
    assert!(object.modifiers.is_empty());
    let ObjectKind::Mesh(baked) = &object.kind else {
        panic!("a mesh")
    };
    let baked = Arc::clone(baked);
    assert_eq!(*baked, *expected);
    assert!(has_custom_normals(&doc.scene().shown_shape(id).unwrap()));
    // Marking seams doesn't change the shading: they stay.
    let edge = baked.edges()[0];
    doc.execute(Command::Record {
        id,
        operation: Operation::SetSeams {
            edges: vec![edge],
            seam: true,
            anchors: None,
        },
    })
    .unwrap();
    assert!(has_custom_normals(&doc.scene().evaluated(id).unwrap().mesh));
    // Moving a point does: they go, and the shading is worked out again.
    let (point, _) = baked.vertices().next().unwrap();
    doc.execute(Command::Record {
        id,
        operation: Operation::MovePoints {
            points: vec![point],
            offset: glam::Vec3::Z,
            anchors: None,
        },
    })
    .unwrap();
    assert!(!has_custom_normals(
        &doc.scene().evaluated(id).unwrap().mesh
    ));
    doc.undo();
    assert!(has_custom_normals(&doc.scene().evaluated(id).unwrap().mesh));
    // The live mirror drops them too (its seam shades across).
    doc.execute(Command::SetMirror {
        ids: vec![id],
        mirror: Some(LiveMirror::across_x()),
    })
    .unwrap();
    assert!(!has_custom_normals(&doc.scene().shown_shape(id).unwrap()));
}

#[test]
fn settings_are_brought_within_limits() {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    add(
        &mut doc,
        id,
        ModifierKind::WeightedNormals(WeightedNormalsOptions {
            strength: f32::NAN,
            influence: 9.0,
            ..Default::default()
        }),
    );
    let ModifierKind::WeightedNormals(o) = doc.scene().modifiers(id)[0].kind else {
        panic!("weighted normals")
    };
    assert_eq!(o.strength, 1.0);
    assert_eq!(o.influence, 1.0);
}

#[test]
fn heavy_shapes_are_worked_out_on_another_thread() {
    let light = mesh::Primitive::Cube(Default::default()).mesh();
    assert!(weighted().work(&light, Purpose::Viewport) < MODIFIERS_IN_BACKGROUND_FROM);
    let heavy = mesh::samples::heavy_ball(100_000, 50.0);
    assert!(weighted().work(&heavy, Purpose::Viewport) >= MODIFIERS_IN_BACKGROUND_FROM);
}
