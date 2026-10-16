//! Tests for the live mirror (decision 0088): an object's shape shown with
//! mirrored copies across world planes, welded where they meet.

use crate::*;
use glam::Vec3;

/// A document with the standard cube (100 cm, around its pivot) moved
/// `x` along X, selected nothing, and its id.
fn cube_at(x: f32) -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    doc.execute(Command::SetTransforms(vec![(
        id,
        Transform {
            position: Vec3::new(x, 0.0, 0.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    (doc, id)
}

fn mirror_on(doc: &mut Document, id: ObjectId, mirror: LiveMirror) {
    doc.execute(Command::SetMirror {
        ids: vec![id],
        mirror: Some(mirror),
    })
    .unwrap();
}

#[test]
fn a_mirror_is_set_undone_and_refused_when_broken() {
    let (mut doc, id) = cube_at(0.0);
    mirror_on(&mut doc, id, LiveMirror::across_x());
    assert_eq!(doc.undo_label(), Some("Mirror"));
    assert_eq!(
        doc.scene().object(id).unwrap().mirror,
        Some(LiveMirror::across_x())
    );
    doc.undo().unwrap();
    assert_eq!(doc.scene().object(id).unwrap().mirror, None);
    doc.redo().unwrap();
    assert!(doc.scene().object(id).unwrap().mirror.is_some());
    for broken in [
        LiveMirror {
            axes: [false; 3],
            ..LiveMirror::across_x()
        },
        LiveMirror {
            centre: Vec3::NAN,
            ..LiveMirror::across_x()
        },
        LiveMirror {
            weld_distance: -1.0,
            ..LiveMirror::across_x()
        },
        LiveMirror {
            weld_distance: 50.0,
            ..LiveMirror::across_x()
        },
    ] {
        assert!(
            doc.execute(Command::SetMirror {
                ids: vec![id],
                mirror: Some(broken),
            })
            .is_err()
        );
    }
    // A group has no shape: left alone.
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    doc.execute(Command::SetMirror {
        ids: vec![group],
        mirror: Some(LiveMirror::across_x()),
    })
    .unwrap();
    assert_eq!(doc.scene().object(group).unwrap().mirror, None);
    doc.execute(Command::SetMirror {
        ids: vec![id],
        mirror: None,
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Mirror off"));
}

#[test]
fn a_cube_beside_the_middle_gets_a_twin_on_the_other_side() {
    let (mut doc, id) = cube_at(100.0);
    mirror_on(&mut doc, id, LiveMirror::across_x());
    let scene = doc.scene();
    let own = scene.evaluated(id).unwrap().mesh;
    let shown = scene.shown_shape(id).unwrap();
    // Two cubes apart: nothing welds.
    assert_eq!(shown.vertex_count(), 2 * own.vertex_count());
    assert_eq!(shown.face_count(), 2 * own.face_count());
    // Its own points keep their ids; the copy sits at x = -100 in the
    // world, which is -200 in the cube's own space.
    for (v, p) in own.vertices() {
        assert_eq!(shown.position(v), Some(p));
    }
    let lowest = shown.vertices().map(|(_, p)| p.x).fold(f32::MAX, f32::min);
    assert!((lowest + 250.0).abs() < 1e-3, "{lowest}");
    // The box around it takes in the twin.
    let bounds = scene.bounds().unwrap();
    assert!((bounds.min.x + 150.0).abs() < 1e-3, "{bounds:?}");
    assert!((bounds.max.x - 150.0).abs() < 1e-3, "{bounds:?}");
    let report = mesh::MeshReport::new(&shown);
    assert!(!report.has_problems(), "{report:?}");
    assert_eq!(report.pieces, 2);
}

#[test]
fn moving_the_object_moves_its_twin_the_other_way() {
    let (mut doc, id) = cube_at(100.0);
    mirror_on(&mut doc, id, LiveMirror::across_x());
    let twin_x = |doc: &Document| {
        let scene = doc.scene();
        let shown = scene.shown_shape(id).unwrap();
        let matrix = scene.shape_matrix(id);
        shown
            .vertices()
            .map(|(_, p)| matrix.transform_point3(p).x)
            .fold(f32::MAX, f32::min)
    };
    assert!((twin_x(&doc) + 150.0).abs() < 1e-3);
    doc.execute(Command::SetTransforms(vec![(
        id,
        Transform {
            position: Vec3::new(300.0, 0.0, 0.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    assert!((twin_x(&doc) + 350.0).abs() < 1e-3, "{}", twin_x(&doc));
}

#[test]
fn a_cube_across_the_middle_welds_and_turned_objects_mirror_in_the_world() {
    // The cube's left side touches the plane: only the points on it weld.
    let (mut doc, id) = cube_at(50.0);
    mirror_on(&mut doc, id, LiveMirror::across_x());
    let shown = doc.scene().shown_shape(id).unwrap();
    // The cube's left side sits on the plane: its four points weld.
    assert_eq!(shown.vertex_count(), 16 - 4);
    // A cube turned a quarter round still mirrors across the world's X.
    doc.execute(Command::SetTransforms(vec![(
        id,
        Transform {
            position: Vec3::new(100.0, 0.0, 0.0),
            rotation: Vec3::new(std::f32::consts::FRAC_PI_2, 0.0, 0.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    let scene = doc.scene();
    let bounds = scene.bounds().unwrap();
    assert!((bounds.min.x + 150.0).abs() < 1e-2, "{bounds:?}");
    // Two axes: four cubes.
    let (mut doc, id) = cube_at(100.0);
    mirror_on(
        &mut doc,
        id,
        LiveMirror {
            axes: [true, true, false],
            ..LiveMirror::across_x()
        },
    );
    let shown = doc.scene().shown_shape(id).unwrap();
    assert_eq!(shown.face_count(), 4 * 6);
    // A moved mirror: through x = 100, the twin lands at x = 100 too
    // (the cube is on the plane's middle).
    let (mut doc, id) = cube_at(100.0);
    mirror_on(
        &mut doc,
        id,
        LiveMirror {
            centre: Vec3::new(150.0, 0.0, 0.0),
            ..LiveMirror::across_x()
        },
    );
    let bounds = doc.scene().bounds().unwrap();
    assert!((bounds.max.x - 250.0).abs() < 1e-3, "{bounds:?}");
    // Without a mirror the shown shape is the shape itself.
    let (doc, id) = cube_at(0.0);
    let scene = doc.scene();
    assert!(std::sync::Arc::ptr_eq(
        &scene.shown_shape(id).unwrap(),
        &scene.evaluated(id).unwrap().mesh
    ));
}
